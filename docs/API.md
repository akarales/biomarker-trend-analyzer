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
{ "inserted": 1, "patients": 1, "biomarkers": ["HBA1C"] }
```

Enforced CSV schema (exact header): `patient_id,code,value,unit,taken_at,source`.
`taken_at` accepts `YYYY-MM-DD` or full ISO datetimes.

## Patient listing

```bash
curl localhost:8003/api/v1/patients
```

```json
{ "patients": [ { "patient_id": "alice", "biomarkers": 4, "observations": 192 } ] }
```

## Drift summary

```bash
curl localhost:8003/api/v1/patients/alice/summary
```

One `DriftReport` per biomarker:

```json
{
  "patient_id": "alice", "window_days": 90,
  "reports": [ {
    "code": "HBA1C", "unit": "%",
    "baseline": { "median": 5.62, "robust_std": 0.09, "n": 48 },
    "latest": 6.9, "latest_z": 13.9,
    "ewma": 6.4, "ewma_z": 8.1,
    "slope_per_day": 0.02, "trend": "rising",
    "anomalies": [ { "t": 1764988800, "v": 6.9 } ],
    "status": "watch", "series_len": 48
  } ]
}
```

## Full series (chart data)

```bash
curl localhost:8003/api/v1/patients/alice/biomarkers/HBA1C
```

Observations (chronological) + the same drift report; the frontend draws
the baseline band from `baseline`.

## Errors

| Status | Meaning |
|--------|---------|
| `400` | valid header but zero data rows ("no observations") |
| `404` | unknown patient / biomarker |
| `422` | CSV errors — schema mismatch (header), bad value, bad date (all name the row and column) |
| `500` | store error (postgres down in pg mode) |
