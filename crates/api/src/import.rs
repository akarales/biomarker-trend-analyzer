//! Turning an upload (or a demo file) into storable observations: format
//! detection (CSV | FHIR R4 JSON), parsing, mapping LOINC codes to the
//! analyte profile's short code, and grouping skip reasons for the report.
//!
//! FHIR imports keep only analytes with a profile (biological variation,
//! limits); everything else is skipped and counted — a full Synthea or EHR
//! export would otherwise flood the workspace with untracked tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biomarker_ingest::Observation;
use serde::Serialize;

use crate::error::ApiError;
use crate::store::Demographics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Fhir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkipReason {
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub format: Format,
    pub observations: Vec<Observation>,
    pub skipped: Vec<SkipReason>,
    /// FHIR Patient demographics (gender, birth year) for eGFR
    pub patients: Vec<Demographics>,
}

impl Parsed {
    pub fn skipped_total(&self) -> usize {
        self.skipped.iter().map(|s| s.count).sum()
    }
}

/// JSON content types (`application/json`, `application/fhir+json`) or a
/// body that starts with `{` are FHIR; everything else is CSV.
pub fn detect(content_type: Option<&str>, body: &[u8]) -> Format {
    let json_type = content_type.is_some_and(|ct| {
        let ct = ct.to_ascii_lowercase();
        ct.starts_with("application/json") || ct.starts_with("application/fhir+json")
    });
    let json_body = body.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    if json_type || json_body {
        Format::Fhir
    } else {
        Format::Csv
    }
}

/// The analyte profile's short code for a LOINC/short code, if any.
fn profile_code(code: &str) -> Option<&'static str> {
    biomarker_drift::lookup(code).map(|p| p.code)
}

pub fn parse(format: Format, body: &[u8]) -> Result<Parsed, ApiError> {
    match format {
        Format::Csv => {
            let mut observations = biomarker_ingest::parse_csv_bytes(body)
                .map_err(|e| ApiError::Csv(e.to_string()))?;
            for o in &mut observations {
                if let Some(code) = profile_code(&o.code) {
                    o.code = code.to_string();
                }
            }
            Ok(Parsed {
                format,
                observations,
                skipped: Vec::new(),
                patients: Vec::new(),
            })
        }
        Format::Fhir => {
            let import = biomarker_ingest::fhir::parse_fhir_json(body)
                .map_err(|e| ApiError::Fhir(e.to_string()))?;
            let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
            for s in import.skipped {
                *reasons.entry(s.reason).or_default() += 1;
            }
            let mut observations = Vec::with_capacity(import.observations.len());
            for mut o in import.observations {
                match profile_code(&o.code) {
                    Some(code) => {
                        o.code = code.to_string();
                        observations.push(o);
                    }
                    None => {
                        *reasons
                            .entry(format!("no analyte profile for LOINC {}", o.code))
                            .or_default() += 1
                    }
                }
            }
            let mut skipped: Vec<SkipReason> = reasons
                .into_iter()
                .map(|(reason, count)| SkipReason { reason, count })
                .collect();
            skipped.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
            let patients = import
                .patients
                .into_iter()
                .map(|p| Demographics {
                    patient_id: p.patient_id,
                    gender: p.gender,
                    birth_year: p.birth_year,
                })
                .collect();
            Ok(Parsed {
                format,
                observations,
                skipped,
                patients,
            })
        }
    }
}

/// Demo seed files: a single file, or every `*.json` / `*.csv` in a
/// directory (sorted by name, so seeding is deterministic).
pub fn seed_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json" || e == "csv"))
        .collect();
    files.sort();
    Ok(files)
}

pub fn parse_file(path: &Path) -> Result<Parsed, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let format = if path.extension().is_some_and(|e| e == "json") {
        Format::Fhir
    } else {
        Format::Csv
    };
    parse(format, &bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_format() {
        assert_eq!(detect(Some("text/csv"), b"patient_id,code"), Format::Csv);
        assert_eq!(
            detect(Some("application/fhir+json; charset=utf-8"), b"x"),
            Format::Fhir
        );
        assert_eq!(
            detect(None, b"  \n{\"resourceType\":\"Bundle\"}"),
            Format::Fhir
        );
        assert_eq!(detect(None, b"patient_id,code"), Format::Csv);
    }

    #[test]
    fn csv_loinc_codes_map_to_profile_codes() {
        let csv = b"patient_id,code,value,unit,taken_at,source\np,4548-4,6.1,%,2026-01-01,lab\np,XYZ,1,U/L,2026-01-01,lab\n";
        let parsed = parse(Format::Csv, csv).expect("csv");
        let codes: Vec<&str> = parsed
            .observations
            .iter()
            .map(|o| o.code.as_str())
            .collect();
        assert_eq!(codes, ["HBA1C", "XYZ"]);
    }

    #[test]
    fn fhir_keeps_profiled_analytes_and_counts_the_rest() {
        let obs = |loinc: &str, status: &str| {
            serde_json::json!({ "resource": {
                "resourceType": "Observation", "status": status,
                "code": { "coding": [{ "system": "http://loinc.org", "code": loinc }] },
                "subject": { "reference": "Patient/SYN-01" },
                "effectiveDateTime": "2026-01-01",
                "valueQuantity": { "value": 1.0, "unit": "mg/dL", "system": "http://unitsofmeasure.org", "code": "mg/dL" }
            }})
        };
        let bundle = serde_json::json!({ "resourceType": "Bundle", "entry": [
            obs("2160-0", "final"), obs("2345-7", "final"), obs("2345-7", "final"), obs("2160-0", "cancelled")
        ]});
        let parsed =
            parse(Format::Fhir, &serde_json::to_vec(&bundle).expect("json")).expect("fhir");
        assert_eq!(parsed.observations.len(), 1);
        assert_eq!(parsed.observations[0].code, "CREAT");
        assert_eq!(parsed.skipped_total(), 3);
        assert_eq!(
            parsed.skipped[0],
            SkipReason {
                reason: "no analyte profile for LOINC 2345-7".into(),
                count: 2
            }
        );
    }
}
