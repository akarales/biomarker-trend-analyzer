//! biomarker-ingest — parse lab-observation CSV uploads (polars, schema
//! enforced) into typed `Observation` records.
//!
//! Schema is declared, never inferred (inference reads twice and drifts);
//! malformed cells are rejected with row context, not silently coerced.
//! Eager `CsvReader` for in-memory HTTP uploads; the CLI uses lazy
//! `scan_csv` + `group_by` aggregation for large batch files.

use std::io::Cursor;

use chrono::NaiveDateTime;
use polars::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub patient_id: String,
    /// Biomarker code, LOINC-style short form ("HBA1C", "LDL", "TSH", "CREAT").
    pub code: String,
    pub value: f64,
    pub unit: String,
    pub taken_at: NaiveDateTime,
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("csv error: {0}")]
    Csv(#[from] polars::error::PolarsError),
    #[error("row {row}: {reason}")]
    Row { row: usize, reason: String },
    #[error("missing column: {0}")]
    MissingColumn(String),
    #[error("schema mismatch: expected columns {expected}, got {got}")]
    SchemaMismatch { expected: String, got: String },
}

fn required_schema() -> Schema {
    Schema::from_iter([
        ("patient_id".into(), DataType::String),
        ("code".into(), DataType::String),
        ("value".into(), DataType::Float64),
        ("unit".into(), DataType::String),
        ("taken_at".into(), DataType::String),
        ("source".into(), DataType::String),
    ])
}

fn parse_datetime(raw: &str) -> Result<NaiveDateTime, String> {
    let raw = raw.trim();
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight"))
        })
        .map_err(|_| format!("invalid taken_at: {raw}"))
}

/// Parse an uploaded CSV (bytes) into observations. Rows are validated in
/// order; the first bad row stops ingestion with row context.
pub fn parse_csv_bytes(bytes: &[u8]) -> Result<Vec<Observation>, IngestError> {
    validate_header(bytes)?;

    let options = CsvReadOptions::default()
        .with_has_header(true)
        .with_schema(Some(Arc::new(required_schema())));
    let reader = CsvReader::new(Cursor::new(bytes)).with_options(options);
    let frame = reader.finish()?;
    frame_to_observations(&frame)
}

/// polars applies the schema positionally and renames mismatched headers,
/// so the header row is validated directly for a clean error message.
fn validate_header(bytes: &[u8]) -> Result<(), IngestError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        IngestError::Csv(polars::prelude::PolarsError::InvalidOperation(
            "upload is not valid UTF-8".into(),
        ))
    })?;
    let header = text
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches('\u{feff}');
    let columns: Vec<String> = header.split(',').map(|c| c.trim().to_lowercase()).collect();
    let expected: Vec<String> = EXPECTED_COLUMNS.iter().map(|c| c.to_string()).collect();
    if columns != expected {
        return Err(IngestError::SchemaMismatch {
            expected: EXPECTED_COLUMNS.join(", "),
            got: columns.join(", "),
        });
    }
    Ok(())
}

const EXPECTED_COLUMNS: [&str; 6] = ["patient_id", "code", "value", "unit", "taken_at", "source"];

fn frame_to_observations(frame: &DataFrame) -> Result<Vec<Observation>, IngestError> {
    let len = frame.height();
    let column = |name: &str| -> Result<&Column, IngestError> {
        frame
            .column(name)
            .map_err(|_| IngestError::MissingColumn(name.to_string()))
    };

    let patient = column("patient_id")?.str()?;
    let code = column("code")?.str()?;
    let value = column("value")?.f64()?;
    let unit = column("unit")?.str()?;
    let taken_at = column("taken_at")?.str()?;
    let source = column("source")?.str()?;

    let mut out = Vec::with_capacity(len);
    for row in 0..len {
        // Strict schema: missing columns arrive as nulls; every cell is
        // validated with the column named in the error.
        let get_str = |series: &StringChunked, name: &str| -> Result<String, IngestError> {
            series
                .get(row)
                .map(|v| v.trim().to_string())
                .ok_or_else(|| IngestError::Row {
                    row,
                    reason: format!("missing or null {name}"),
                })
        };
        let raw_value = value.get(row).ok_or_else(|| IngestError::Row {
            row,
            reason: "missing or null value".to_string(),
        })?;
        if !raw_value.is_finite() {
            return Err(IngestError::Row {
                row,
                reason: "non-finite value".to_string(),
            });
        }

        out.push(Observation {
            patient_id: get_str(patient, "patient_id")?,
            code: get_str(code, "code")?.to_uppercase(),
            value: raw_value,
            unit: get_str(unit, "unit")?,
            taken_at: parse_datetime(&get_str(taken_at, "taken_at")?)
                .map_err(|reason| IngestError::Row { row, reason })?,
            source: get_str(source, "source")?,
        });
    }
    Ok(out)
}

/// Per-biomarker aggregate stats over a (large) CSV, computed with the
/// lazy engine: scan → group_by → one collect. Used by the CLI and by the
/// API's upload summary when files get big.
pub fn biomarker_stats(path: &str) -> Result<Vec<BiomarkerStat>, IngestError> {
    let frame = LazyCsvReader::new(PlRefPath::new(path.to_string()))
        .with_has_header(true)
        .with_schema(Some(Arc::new(required_schema())))
        .finish()?
        .group_by([col("code")])
        .agg([
            col("value").count().alias("n"),
            col("value").mean().alias("mean"),
            col("value").min().alias("min"),
            col("value").max().alias("max"),
        ])
        .sort(["code"], SortMultipleOptions::default())
        .collect()?;

    let codes = frame.column("code")?.str()?;
    let counts = frame.column("n")?.u32()?;
    let means = frame.column("mean")?.f64()?;
    let mins = frame.column("min")?.f64()?;
    let maxs = frame.column("max")?.f64()?;

    let mut stats = Vec::with_capacity(frame.height());
    for row in 0..frame.height() {
        stats.push(BiomarkerStat {
            code: codes
                .get(row)
                .ok_or_else(|| IngestError::Row {
                    row,
                    reason: "null code".to_string(),
                })?
                .to_string(),
            n: counts.get(row).unwrap_or_default() as usize,
            mean: means.get(row).unwrap_or_default(),
            min: mins.get(row).unwrap_or_default(),
            max: maxs.get(row).unwrap_or_default(),
        });
    }
    Ok(stats)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BiomarkerStat {
    pub code: String,
    pub n: usize,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
patient_id,code,value,unit,taken_at,source
alice,HBA1C,5.7,%,2026-01-15,labs
alice,LDL,120,mg/dL,2026-01-15,labs
bob,TSH,2.1,mIU/L,2026-02-01T08:30:00,labs
alice,HBA1C,6.9,%,2026-04-20,labs
";

    #[test]
    fn parses_rows_and_normalizes() {
        let observations = parse_csv_bytes(SAMPLE.as_bytes()).expect("valid csv");
        assert_eq!(observations.len(), 4);
        assert_eq!(observations[0].patient_id, "alice");
        assert_eq!(observations[0].code, "HBA1C");
        assert_eq!(observations[2].taken_at.to_string(), "2026-02-01 08:30:00");
        assert_eq!(observations[2].source, "labs");
    }

    #[test]
    fn date_only_form_is_accepted() {
        let observations = parse_csv_bytes(SAMPLE.as_bytes()).expect("valid csv");
        assert_eq!(
            observations[0].taken_at.date(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date")
        );
    }

    #[test]
    fn missing_column_is_reported() {
        let bad = "patient_id,value\nalice,1.0\n";
        let err = parse_csv_bytes(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("schema mismatch"), "got: {err}");
    }

    #[test]
    fn null_or_bad_value_is_rejected_with_row() {
        let bad = "\
patient_id,code,value,unit,taken_at,source
alice,HBA1C,5.7,%,2026-01-15,labs
bob,HBA1C,,%,2026-01-15,labs
";
        let err = parse_csv_bytes(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("row"), "got: {err}");
    }

    #[test]
    fn bad_datetime_is_rejected_with_row() {
        let bad = "\
patient_id,code,value,unit,taken_at,source
alice,HBA1C,5.7,%,not-a-date,labs
";
        let err = parse_csv_bytes(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("taken_at"), "got: {err}");
    }

    #[test]
    fn biomarker_stats_groups_lazily() {
        let dir = std::env::temp_dir().join("biomarker-ingest-tests");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("sample.csv");
        std::fs::write(&path, SAMPLE).expect("write sample");

        let stats = biomarker_stats(path.to_str().expect("utf8 path")).expect("stats");
        assert_eq!(stats.len(), 3, "HBA1C, LDL, TSH");
        let hba1c = stats
            .iter()
            .find(|s| s.code == "HBA1C")
            .expect("HBA1C present");
        assert_eq!(hba1c.n, 2);
        assert!((hba1c.mean - 6.3).abs() < 1e-9);
    }
}
