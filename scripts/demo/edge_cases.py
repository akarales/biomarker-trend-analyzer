#!/usr/bin/env python3
"""Write demo/edge-cases.json: hand-made synthetic FHIR R4 patients for the
situations Synthea does not produce (it emits no usable TSH, and its CKD
creatinine is incoherent). Deterministic (fixed seed), stdlib only:

    uv run --no-project scripts/demo/edge_cases.py demo/edge-cases.json

Every value is invented; noise is drawn at roughly the published
within-subject variation of each analyte.
"""

from __future__ import annotations

import json
import random
import sys
from datetime import date, timedelta
from pathlib import Path

LOINC = "http://loinc.org"
UCUM = "http://unitsofmeasure.org"
CODES = {
    "TSH": ("3016-3", "Thyrotropin [Units/volume] in Serum or Plasma"),
    "CREAT": ("2160-0", "Creatinine [Mass/volume] in Serum or Plasma"),
    "HBA1C": ("4548-4", "Hemoglobin A1c/Hemoglobin.total in Blood"),
    "LDL": ("18262-6", "Cholesterol in LDL [Mass/volume] in Serum or Plasma by Direct assay"),
}
rng = random.Random(20261006)


def noisy(level: float, cv: float, decimals: int) -> float:
    return round(level * (1 + rng.gauss(0, cv)), decimals)


def months_back(end: date, count: int, step_months: int) -> list[date]:
    """`count` dates ending at `end`, every `step_months` (same day of month, ≤ 28)."""
    out = []
    y, m = end.year, end.month
    for _ in range(count):
        out.append(date(y, m, min(end.day, 28)))
        m -= step_months
        while m <= 0:
            m += 12
            y -= 1
    return sorted(out)


class Builder:
    def __init__(self) -> None:
        self.entries: list[dict] = []

    def patient(self, pid: str, gender: str, birth_year: int, scenario: str) -> None:
        self.pid, self.n = pid, 0
        self.entries.append({
            "fullUrl": f"urn:biomarker-trend-analyzer:Patient/{pid}",
            "resource": {
                "resourceType": "Patient",
                "id": pid,
                "meta": {"tag": [{"system": "urn:biomarker-trend-analyzer:data", "code": "synthetic", "display": scenario}]},
                "gender": gender,
                "birthDate": str(birth_year),
            },
        })

    def result(self, analyte: str, when: date, value: float, unit: str, status: str = "final") -> None:
        self.n += 1
        loinc, display = CODES[analyte]
        self.entries.append({"resource": {
            "resourceType": "Observation",
            "id": f"{self.pid}-obs-{self.n:04d}",
            "status": status,
            "category": [{"coding": [{"system": "http://terminology.hl7.org/CodeSystem/observation-category", "code": "laboratory"}]}],
            "code": {"coding": [{"system": LOINC, "code": loinc, "display": display}]},
            "subject": {"reference": f"Patient/{self.pid}"},
            "effectiveDateTime": f"{when.isoformat()}T08:30:00Z",
            "valueQuantity": {"value": value, "unit": unit, "system": UCUM, "code": unit},
        }})


def build() -> dict:
    b = Builder()
    end = date(2026, 9, 14)

    # EDGE-01 — hypothyroidism: TSH stable on levothyroxine for 2 years, then
    # rising into the subclinical range (missed doses / absorption change)
    b.patient("EDGE-01", "female", 1968, "levothyroxine-treated hypothyroidism, TSH rising")
    dates = months_back(end, 10, 3)
    for i, d in enumerate(dates):
        level = 2.0 if i < 8 else (5.6 if i == 8 else 7.4)
        b.result("TSH", d, noisy(level, 0.12, 2), "m[IU]/L")

    # EDGE-02 — CKD progression: monthly creatinine creeping 1.05 → ~1.6 mg/dL
    # over 2 years; the laboratory switched to SI units (µmol/L) for the last
    # 8 results. Tests unit normalisation and the trend detector.
    b.patient("EDGE-02", "male", 1957, "CKD progression with a mid-series unit change")
    dates = months_back(end, 24, 1)
    for i, d in enumerate(dates):
        mg_dl = noisy(1.05 + 0.024 * i, 0.03, 2)
        if i < 16:
            b.result("CREAT", d, mg_dl, "mg/dL")
        else:
            b.result("CREAT", d, round(mg_dl * 88.42), "umol/L")

    # EDGE-03 — erroneous result: stable creatinine with one implausible jump
    # (haemolysed / mislabelled specimen), repeated twice → back to normal.
    # Both RCV jumps stay on the chart; the latest status is quiet and no
    # shift is raised (one result is not a shift).
    b.patient("EDGE-03", "female", 1975, "erroneous outlier, repeated and normal")
    dates = months_back(date(2026, 8, 28), 9, 4)
    for d in dates:
        b.result("CREAT", d, noisy(0.88, 0.03, 2), "mg/dL")
    b.result("CREAT", date(2026, 9, 2), 1.94, "mg/dL")
    b.result("CREAT", date(2026, 9, 9), noisy(0.88, 0.03, 2), "mg/dL")
    b.result("CREAT", date(2026, 9, 16), noisy(0.88, 0.03, 2), "mg/dL")
    # an entered-in-error duplicate that the importer must ignore
    b.result("CREAT", date(2026, 9, 3), 1.94, "mg/dL", status="entered-in-error")

    # EDGE-04 — sparse and gapped history: two HbA1c results (no personal
    # baseline yet) and LDL with a six-year gap between results
    b.patient("EDGE-04", "male", 1981, "sparse HbA1c, gapped LDL")
    b.result("HBA1C", date(2025, 9, 1), 5.9, "%")
    b.result("HBA1C", date(2026, 9, 1), 6.0, "%")
    for d in [date(2017, 5, 2), date(2018, 5, 8), date(2019, 4, 30)]:
        b.result("LDL", d, noisy(118, 0.06, 0), "mg/dL")
    for d in [date(2025, 6, 3), date(2026, 6, 9)]:
        b.result("LDL", d, noisy(124, 0.06, 0), "mg/dL")

    return {"resourceType": "Bundle", "type": "collection", "entry": b.entries}


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    out = Path(sys.argv[1])
    bundle = build()
    out.write_text(json.dumps(bundle, indent=1, ensure_ascii=False) + "\n")
    n = sum(1 for e in bundle["entry"] if e["resource"]["resourceType"] == "Observation")
    print(f"wrote {out} ({n} observations)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
