# Demo data — provenance

Everything in this folder is **synthetic**. No real patient's data, no PHI.
The API seeds every `*.json` (FHIR R4 Bundle) and `*.csv` here at startup
(`APP_DEMO_DATA`, default `demo/`); seeding is idempotent.

| File | Patients | Observations | Source |
|------|----------|--------------|--------|
| `synthea-subset.json` | SYN-01 … SYN-06 | 502 | Synthea v4.0.0 (MITRE, Apache-2.0), curated subset |
| `edge-cases.json` | EDGE-01 … EDGE-04 | 54 (1 entered-in-error, skipped on import) | hand-made, `scripts/demo/edge_cases.py` |

## synthea-subset.json

Regenerate (byte-identical output, verified 2026-10-06):

```bash
scripts/synthea/generate_synthea.sh
```

| Setting | Value |
|---------|-------|
| Synthea | v4.0.0 `synthea-with-dependencies.jar`, sha256 `ed43c20ad40ba5c3bc724503a5af032715fe3c491620b766148e7c2361e6ecc1` (verified by the script) |
| Run | seed / clinician seed `20261006`, reference date `2026-10-01`, 40 patients aged 40–80, Massachusetts, 10 years of history, FHIR R4 export only |
| Keep module | `scripts/synthea/keep_conditions.json`: active diabetes, prediabetes, hyperlipidaemia, CKD stage 1–4 or hypothyroidism (SNOMED CT) |
| Selection | `scripts/synthea/select_subset.py`: explicit, reviewed list of 6 living patients (below) |
| Generated | 2026-10-06 |

**What is kept:**
- A minimal Patient: pseudonymous id, gender, birth **year** only. Synthea's synthetic names, addresses, identifiers and exact birth dates are dropped.
- Final lab Observations for the profiled analytes (HbA1c 4548-4, LDL-C 18262-6/13457-7, creatinine 2160-0/38483-4, TSH 3016-3), rounded to laboratory reporting precision, one result per analyte per timestamp.

**Plausibility rules.** Synthea simulates populations, not laboratories, and some of its series are physiologically impossible. Examples from this run: HbA1c 2.4 % for a decade, creatinine 70 mg/dL, negative LDL-C, creatinine 3 mg/dL next to a normal eGFR. A series that fails a rule is dropped **whole**; individual results are never edited or removed:
- value ranges: HbA1c 4–15 %, LDL-C 20–400 mg/dL, creatinine 0.3–15 mg/dL, TSH 0.01–100 mIU/L
- ≥ 4 results and ≥ 3 distinct values (real assays never repeat one value for years)
- creatinine: max/min ≤ 3, and the median is not > 1.5 mg/dL while the same record's eGFR median is ≥ 60

Even after filtering, Synthea's results vary more from visit to visit than real biology. Expect more RCV jumps than a real patient would show.

| Pseudonym | Why it is in the demo | Kept series |
|-----------|----------------------|-------------|
| SYN-01 | type 2 diabetes: HbA1c rising from 6.1 % into the diabetes range | HbA1c, LDL-C, creatinine |
| SYN-02 | hyperlipidaemia on a statin: LDL-C rising 53 → 150 mg/dL | HbA1c, LDL-C |
| SYN-03 | hyperlipidaemia: LDL-C falling 135 → 96 mg/dL after statin start | HbA1c, LDL-C |
| SYN-04 | CKD stage 3: dense creatinine series (~2.0 mg/dL), personally stable | creatinine (232), LDL-C |
| SYN-05 | CKD stage 3 + diabetes: stable creatinine above the population interval | creatinine (87), LDL-C (71) |
| SYN-06 | prediabetes: HbA1c steady around 6.2 % | HbA1c, LDL-C |

## edge-cases.json

These cover situations Synthea does not produce: it emits no usable TSH, and its CKD creatinine is incoherent. The values are invented, with noise at roughly each analyte's within-subject variation, and generated from a fixed seed:

```bash
uv run --no-project scripts/demo/edge_cases.py demo/edge-cases.json
```

| Pseudonym | Scenario | What it exercises |
|-----------|----------|-------------------|
| EDGE-01 | treated hypothyroidism, TSH stable ~2.0 for 2 years then 5.6 → 7.4 mIU/L | prRI, shift, TSH subclinical threshold |
| EDGE-02 | CKD progression: monthly creatinine 1.05 → ~1.6 mg/dL; the last 8 results in µmol/L | UCUM normalisation, Mann–Kendall trend, EWMA |
| EDGE-03 | stable creatinine, one erroneous 1.94 mg/dL result, two normal repeats; plus an `entered-in-error` duplicate | RCV jumps shown, no shift, quiet latest status; importer skips the bad status |
| EDGE-04 | two HbA1c results; LDL-C with a six-year gap | "not assessed" (no personal baseline yet), irregular sampling |

`crates/api/tests/demo_data.rs` pins these behaviours and checks that every
detector fires for at least one demo patient.
