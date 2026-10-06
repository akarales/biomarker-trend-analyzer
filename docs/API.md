# API Reference

Base URL: `http://localhost:8003`.

## Health

```bash
curl localhost:8003/health
```

```json
{ "status": "ok", "store": "memory", "version": "0.1.0" }
```

## Upload observations (CSV)

```bash
curl -X POST localhost:8003/api/v1/observations \
  -H 'content-type: text/csv' \
  --data-binary $'patient_id,code,value,unit,taken_at,source
dave,HBA1C,7.2,%,2026-09-01,upload'
```

```json
{ "inserted": 1, "duplicates": 0, "patients": 1, "biomarkers": ["HBA1C"] }
```

Uploads are idempotent: a row whose `(patient_id, code, taken_at)` is
already stored (or repeated in the same file) is skipped and counted in
`duplicates` — posting the same file twice returns `inserted: 0`.

Enforced CSV schema (exact header): `patient_id,code,value,unit,taken_at,source`.
`taken_at` accepts `YYYY-MM-DD` or full ISO datetimes.

## Patient listing

```bash
curl localhost:8003/api/v1/patients
```

```json
{ "patients": [ { "patient_id": "alice", "biomarkers": 4, "observations": 192 } ] }
```

## Analysis parameters

Both analysis endpoints accept:

| Param | Default | Meaning |
|-------|---------|---------|
| `as_of` | latest result of each series | `YYYY-MM-DD` (end of that day, UTC) or RFC 3339; results after it are excluded. The wall clock is never used, so a report is reproducible. |
| `window_days` | `APP_WINDOW_DAYS` (365) | lookback for the trend detector only (7–3650). The personal baseline is the earliest steady state, independent of the window. |

Invalid values → `400 bad_request`.

## Drift summary

```bash
curl 'localhost:8003/api/v1/patients/alice/summary?window_days=90'
```

```json
{ "patient_id": "alice", "as_of": null, "window_days": 90, "reports": [ DriftReport, … ] }
```

## DriftReport

Method and sources: [ARCHITECTURE.md § Drift engine](ARCHITECTURE.md#drift-engine-cratesdrift).
All times are epoch seconds; all values are in `unit` (the analyte's
canonical UCUM unit). Example (alice HbA1c, abridged):

```json
{
  "code": "HBA1C",
  "analyte": { "code": "HBA1C", "loinc": "4548-4", "display": "Hemoglobin A1c",
               "cvi": 0.012, "cva": 0.015, "cvi_source": "…", "cva_source": "…", "reviewed": "2026-10-06" },
  "unit": "%", "as_of": 1796083200, "window_days": 365,
  "points": [ { "t": 1767657600, "v": 5.59 }, … ],
  "excluded": { "after_as_of": 0, "unit_unknown": 0 },
  "latest": { "t": 1796083200, "v": 7.02 },
  "baseline": { "n": 10, "from": 1767657600, "to": 1773100800, "set_point": 5.59,
                "prri_low": 5.38, "prri_high": 5.82, "level": 0.95 },
  "population": { "low": 4.0, "high": 5.6, "source": "ADA Standards of Care …" },
  "thresholds": [ { "value": 6.5, "direction": "above", "severity": "alert",
                    "label": "diabetes range (ADA ≥ 6.5 %)", "source": "…" }, … ],
  "rcv": { "up": 0.055, "down": -0.052 },
  "rcv_jumps": [ { "from_t": 1788825600, "to_t": 1789430400, "change": 0.246 } ],
  "ewma": { "value": 6.91, "low": 5.49, "high": 5.70 },
  "change_point": { "t": 1789430400, "detected_t": 1790035200, "before": 5.59, "after": 7.01, "change": 0.253 },
  "trend": { "n": 48, "from": 1767657600, "slope_per_day": 0.00071, "ci_low_per_day": 0.00024,
             "ci_high_per_day": 0.0043, "change_per_year": 0.046, "tau": 0.38, "p_value": 0.00015,
             "direction": "rising" },
  "signals": [ {
    "rule": "prri", "severity": "alert", "t": 1796083200, "value": 7.02, "threshold": 5.82,
    "explanation": "Hemoglobin A1c 7.02 % is above this patient's personal reference interval 5.38–5.82 % (95 % prediction from 10 results 2026-01-06 → 2026-03-10, set point 5.59 %; CVI 1.2 %, CVA 1.5 %).",
    "source": "personalised reference interval: Coşkun A et al., Clin Chem 2021;67:374–384 (…)"
  }, … ],
  "not_assessed": [],
  "status": "alert"
}
```

- `rule` ∈ `prri | rcv | shift | ewma | trend | threshold | population`;
  `severity` ∈ `info | watch | alert`; signals are sorted worst first.
- `status` = worst signal (`info` → `normal`); `normal` means no rule
  fired — check `not_assessed` for what could not be evaluated.
- `analyte`, `baseline`, `population`, `rcv`, `ewma`, `change_point`,
  `trend`, `latest` are `null` when not applicable.

## Full series (chart data)

```bash
curl 'localhost:8003/api/v1/patients/alice/biomarkers/HBA1C?as_of=2026-08-31'
```

`observations` as stored (original unit, `taken_at` RFC 3339 UTC) + the
same `report`; the chart draws `report.points`, the prRI band from
`report.baseline`.

## Errors

Every error body is `{"error": "<message>", "code": "<stable code>"}` —
clients branch on `code`. Every response carries `x-request-id` (a
well-formed incoming id is echoed, otherwise one is generated); the same id
is on the server's log lines for that request.

| Status | `code` | Meaning |
|--------|--------|---------|
| `400` | `bad_request` | valid header but zero data rows ("no observations") |
| `404` | `not_found` | unknown patient / biomarker |
| `422` | `invalid_csv` | schema mismatch (header), bad value, bad date (all name the row and column) |
| `500` | `internal` | store failure (e.g. Postgres down) — details only in the server log |
