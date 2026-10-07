#!/usr/bin/env python3
"""Select the committed demo subset from a Synthea FHIR R4 export.

Usage (stdlib only, run with `uv run --no-project`):

    select_subset.py <synthea fhir dir> <out bundle.json>           # the reviewed selection
    select_subset.py <synthea fhir dir> <out bundle.json> --candidates   # every plausible patient

What is kept (per selected patient):
  - a minimal Patient: pseudonymous id, administrative gender, birth YEAR only
    (no names, addresses, identifiers, exact dates of birth)
  - final lab Observations for the profiled analytes, rounded to the
    precision a laboratory reports, one result per analyte per timestamp

Plausibility rules (Synthea is a population simulator, not a lab
simulator; some generated series are physiologically impossible and
would mislead a clinical audience). A failing series is dropped WHOLE,
individual results are never edited or removed:
  - HbA1c 4.0-15 %, LDL-C 20-400 mg/dL, creatinine 0.3-15 mg/dL, TSH 0.01-100 mIU/L
  - at least 3 distinct values (real assays never repeat one value for years)
  - creatinine: max/min <= 3, and not a median > 1.5 mg/dL while the same
    record's eGFR median is >= 60 (Synthea's creatinine and eGFR disagree)
  - at least 4 results
"""

from __future__ import annotations

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

LOINC = "http://loinc.org"
UCUM = "http://unitsofmeasure.org"

# LOINC -> (analyte, preferred LOINC, display, UCUM unit, decimals)
ANALYTES = {
    "4548-4": ("HBA1C", "4548-4", "Hemoglobin A1c/Hemoglobin.total in Blood", "%", 1),
    "18262-6": ("LDL", "18262-6", "Cholesterol in LDL [Mass/volume] in Serum or Plasma by Direct assay", "mg/dL", 0),
    "13457-7": ("LDL", "13457-7", "Cholesterol in LDL [Mass/volume] in Serum or Plasma by calculation", "mg/dL", 0),
    "2160-0": ("CREAT", "2160-0", "Creatinine [Mass/volume] in Serum or Plasma", "mg/dL", 2),
    "38483-4": ("CREAT", "38483-4", "Creatinine [Mass/volume] in Blood", "mg/dL", 2),
    "3016-3": ("TSH", "3016-3", "Thyrotropin [Units/volume] in Serum or Plasma", "m[IU]/L", 2),
}
BOUNDS = {"HBA1C": (4.0, 15.0), "LDL": (20.0, 400.0), "CREAT": (0.3, 15.0), "TSH": (0.01, 100.0)}
EGFR = "33914-3"
MIN_RESULTS = 4

# Reviewed selection: Synthea patient id (stable for the pinned version,
# seed and reference date) -> (pseudonym, why it is in the demo).
SELECTION: dict[str, tuple[str, str]] = {
    "92efb9b4-0aaa-c3ac-9923-9b287e88a434": ("SYN-01", "type 2 diabetes: HbA1c rising from 6.1 % into the diabetes range"),
    "6910e0f5-2e16-59b5-a809-53d5372a9d01": ("SYN-02", "hyperlipidaemia on a statin: LDL-C rising 53 → 150 mg/dL"),
    "98e6533e-f89c-6389-0731-57bcb3347c15": ("SYN-03", "hyperlipidaemia: LDL-C falling 135 → 96 mg/dL after statin start"),
    "48a874bf-7a1c-04af-af74-b1beb910deab": ("SYN-04", "CKD stage 3: dense creatinine series (~2.0 mg/dL), personally stable"),
    "340266db-ef1a-5aa4-9a6a-1159a0b0d778": ("SYN-05", "CKD stage 3 + diabetes: stable creatinine above the population interval"),
    "983a4fab-bdac-ef0c-0615-552dadffd5fb": ("SYN-06", "prediabetes: HbA1c steady around 6.2 %"),
}


def load(path: Path) -> tuple[dict, dict[str, list[tuple[str, float, str]]], list[float]]:
    bundle = json.loads(path.read_text())
    patient: dict = {}
    series: dict[str, list[tuple[str, float, str]]] = defaultdict(list)
    egfr: list[float] = []
    for entry in bundle.get("entry", []):
        r = entry.get("resource", {})
        kind = r.get("resourceType")
        if kind == "Patient":
            patient = r
        if kind != "Observation" or r.get("status") not in ("final", "amended", "corrected"):
            continue
        q = r.get("valueQuantity")
        if not q or "value" not in q:
            continue
        for c in r.get("code", {}).get("coding", []):
            if c.get("system") != LOINC:
                continue
            if c.get("code") in ANALYTES:
                series[c["code"]].append((r["effectiveDateTime"], float(q["value"]), c["code"]))
            elif c.get("code") == EGFR and q.get("code") in ("mL/min/{1.73_m2}", "mL/min"):
                egfr.append(float(q["value"]))
    by_analyte: dict[str, list[tuple[str, float, str]]] = defaultdict(list)
    for loinc, rows in series.items():
        by_analyte[ANALYTES[loinc][0]].extend(rows)
    return patient, by_analyte, egfr


def plausible(analyte: str, rows: list[tuple[str, float, str]], egfr: list[float]) -> str | None:
    """Reason the whole series is rejected, or None."""
    values = [v for _, v, _ in rows]
    lo, hi = BOUNDS[analyte]
    if len(rows) < MIN_RESULTS:
        return f"fewer than {MIN_RESULTS} results"
    if any(v < lo or v > hi for v in values):
        return f"value outside {lo}-{hi}"
    if len({round(v, 3) for v in values}) < 3:
        return "fewer than 3 distinct values"
    if analyte == "CREAT":
        if max(values) / min(values) > 3:
            return "creatinine range > 3x"
        if egfr and statistics.median(egfr) >= 60 and statistics.median(values) > 1.5:
            return "creatinine inconsistent with eGFR"
    return None


def dedupe(rows: list[tuple[str, float, str]]) -> list[tuple[str, float, str]]:
    """One result per timestamp (Synthea can emit blood + serum codes together)."""
    seen: dict[str, tuple[str, float, str]] = {}
    for row in sorted(rows):
        seen.setdefault(row[0], row)
    return sorted(seen.values())


def patient_resource(pid: str, patient: dict) -> dict:
    return {
        "resourceType": "Patient",
        "id": pid,
        "meta": {"tag": [{"system": "urn:biomarker-trend-analyzer:data", "code": "synthetic"}]},
        "gender": patient.get("gender", "unknown"),
        "birthDate": patient.get("birthDate", "")[:4],
    }


def observation_resource(pid: str, n: int, row: tuple[str, float, str]) -> dict:
    when, value, loinc = row
    _, code, display, unit, decimals = ANALYTES[loinc]
    return {
        "resourceType": "Observation",
        "id": f"{pid}-obs-{n:04d}",
        "status": "final",
        "category": [{"coding": [{"system": "http://terminology.hl7.org/CodeSystem/observation-category", "code": "laboratory"}]}],
        "code": {"coding": [{"system": LOINC, "code": code, "display": display}]},
        "subject": {"reference": f"Patient/{pid}"},
        "effectiveDateTime": when,
        "valueQuantity": {"value": round(value, decimals), "unit": unit, "system": UCUM, "code": unit},
    }


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    candidates = "--candidates" in sys.argv[3:]
    entries: list[dict] = []
    report: list[str] = []
    for path in sorted(src.glob("*.json")):
        patient, by_analyte, egfr = load(path)
        sid = patient.get("id", "")
        if not sid or patient.get("deceasedDateTime"):
            continue
        if candidates:
            pid, why = f"C-{sid[:8]}", "candidate"
        elif sid in SELECTION:
            pid, why = SELECTION[sid]
        else:
            continue
        kept: dict[str, list[tuple[str, float, str]]] = {}
        for analyte, rows in sorted(by_analyte.items()):
            reason = plausible(analyte, dedupe(rows), egfr)
            if reason:
                report.append(f"{pid} {analyte}: dropped ({reason})")
            else:
                kept[analyte] = dedupe(rows)
        if not kept or (candidates and len(kept) < 2):
            continue
        entries.append({"fullUrl": f"urn:biomarker-trend-analyzer:Patient/{pid}", "resource": patient_resource(pid, patient)})
        n = 0
        for analyte in sorted(kept):
            for row in kept[analyte]:
                n += 1
                entries.append({"resource": observation_resource(pid, n, row)})
        report.append(f"{pid} kept {', '.join(f'{a}×{len(r)}' for a, r in sorted(kept.items()))} — {why}")
    if not candidates and len({e['resource']['id'] for e in entries if e['resource']['resourceType'] == 'Patient'}) != len(SELECTION):
        print("selection mismatch: not every selected Synthea patient was found", file=sys.stderr)
        return 1
    bundle = {"resourceType": "Bundle", "type": "collection", "entry": entries}
    out.write_text(json.dumps(bundle, indent=1, ensure_ascii=False) + "\n")
    print("\n".join(report))
    print(f"wrote {out} ({sum(1 for e in entries if e['resource']['resourceType'] == 'Observation')} observations)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
