//! Minimal FHIR R4 import: a `Bundle` (or a single `Observation`) of lab
//! results → `Observation` records. Hand-defined serde structs for only the
//! fields we read (app #3, fhir-r4-explorer, owns the full models).
//!
//! Rules (each skipped resource gets a reason, nothing is coerced):
//! - `status` must be `final`, `amended` or `corrected`
//! - the code must carry a LOINC coding (`http://loinc.org`); the stored
//!   code is the LOINC code
//! - the value must be a `valueQuantity`; the unit is the UCUM `code` when
//!   the system is UCUM, else the human `unit`
//! - `effectiveDateTime` (or `effectiveInstant`) must be a full date or a
//!   date-time with offset; it is stored as naive UTC
//! - `subject` must reference a Patient (`Patient/<id>` or `urn:uuid:<id>`)
//! - non-Observation resources (Patient, Condition, …) are ignored

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::Deserialize;
use serde_json::Value;

use crate::{IngestError, Observation};

pub const LOINC: &str = "http://loinc.org";
pub const UCUM: &str = "http://unitsofmeasure.org";
const ACCEPTED_STATUS: [&str; 3] = ["final", "amended", "corrected"];

#[derive(Debug, Deserialize)]
struct Bundle {
    #[serde(default)]
    entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    resource: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FhirObservation {
    id: Option<String>,
    status: String,
    code: CodeableConcept,
    subject: Option<Reference>,
    effective_date_time: Option<String>,
    effective_instant: Option<String>,
    value_quantity: Option<Quantity>,
}

#[derive(Debug, Deserialize)]
struct CodeableConcept {
    #[serde(default)]
    coding: Vec<Coding>,
}

#[derive(Debug, Deserialize)]
struct Coding {
    system: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Reference {
    reference: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Quantity {
    value: Option<f64>,
    unit: Option<String>,
    system: Option<String>,
    code: Option<String>,
}

/// One Observation that was not imported, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// `Observation/<id>` or `entry[<n>]`
    pub resource: String,
    pub reason: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct FhirImport {
    pub observations: Vec<Observation>,
    pub skipped: Vec<Skipped>,
}

/// Parse a FHIR R4 JSON Bundle (any type) or a single Observation.
pub fn parse_fhir_json(bytes: &[u8]) -> Result<FhirImport, IngestError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|e| IngestError::Fhir(format!("invalid JSON: {e}")))?;
    let resources: Vec<Value> = match root.get("resourceType").and_then(Value::as_str) {
        Some("Bundle") => serde_json::from_value::<Bundle>(root)
            .map_err(|e| IngestError::Fhir(format!("invalid Bundle: {e}")))?
            .entry
            .into_iter()
            .filter_map(|e| e.resource)
            .collect(),
        Some("Observation") => vec![root],
        Some(other) => {
            return Err(IngestError::Fhir(format!(
                "expected a Bundle or an Observation, got {other}"
            )));
        }
        None => return Err(IngestError::Fhir("missing resourceType".into())),
    };

    let mut out = FhirImport::default();
    for (index, resource) in resources.into_iter().enumerate() {
        if resource.get("resourceType").and_then(Value::as_str) != Some("Observation") {
            continue;
        }
        let label = resource.get("id").and_then(Value::as_str).map_or_else(
            || format!("entry[{index}]"),
            |id| format!("Observation/{id}"),
        );
        match serde_json::from_value::<FhirObservation>(resource)
            .map_err(|e| format!("not a valid Observation: {e}"))
            .and_then(convert)
        {
            Ok(obs) => out.observations.push(obs),
            Err(reason) => out.skipped.push(Skipped {
                resource: label,
                reason,
            }),
        }
    }
    Ok(out)
}

fn convert(o: FhirObservation) -> Result<Observation, String> {
    if !ACCEPTED_STATUS.contains(&o.status.as_str()) {
        return Err(format!(
            "status {} is not final/amended/corrected",
            o.status
        ));
    }
    let code = o
        .code
        .coding
        .iter()
        .find(|c| c.system.as_deref() == Some(LOINC))
        .and_then(|c| c.code.clone())
        .ok_or("no LOINC coding")?;
    let quantity = o
        .value_quantity
        .ok_or("no valueQuantity (coded and component results are not imported)")?;
    let value = quantity
        .value
        .filter(|v| v.is_finite())
        .ok_or("valueQuantity without a value")?;
    let unit = match quantity.system.as_deref() {
        Some(UCUM) => quantity.code.or(quantity.unit),
        _ => quantity.unit.or(quantity.code),
    }
    .filter(|u| !u.trim().is_empty())
    .ok_or("valueQuantity without a unit")?;
    let raw_time = o
        .effective_date_time
        .or(o.effective_instant)
        .ok_or("no effectiveDateTime")?;
    let taken_at = parse_fhir_datetime(&raw_time)?;
    let reference = o
        .subject
        .and_then(|s| s.reference)
        .ok_or("no subject reference")?;
    let patient_id =
        patient_id(&reference).ok_or_else(|| format!("subject {reference} is not a Patient"))?;
    let _ = o.id;
    Ok(Observation {
        patient_id,
        code,
        value,
        unit,
        taken_at,
        source: "fhir".into(),
    })
}

/// FHIR dateTime: a full date, or a date-time WITH offset (stored as UTC).
/// Partial dates (`2026`, `2026-05`) are too imprecise to trend.
fn parse_fhir_datetime(raw: &str) -> Result<NaiveDateTime, String> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.naive_utc());
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight"))
        .map_err(|_| {
            format!("effectiveDateTime {raw} is not a full date or a date-time with offset")
        })
}

fn patient_id(reference: &str) -> Option<String> {
    let id = reference
        .strip_prefix("Patient/")
        .or_else(|| reference.strip_prefix("urn:uuid:"))?;
    (!id.is_empty() && !id.contains('/')).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(extra: Value) -> Value {
        let mut base = serde_json::json!({
            "resourceType": "Observation",
            "id": "o1",
            "status": "final",
            "code": { "coding": [
                { "system": "http://snomed.info/sct", "code": "1" },
                { "system": LOINC, "code": "4548-4", "display": "Hemoglobin A1c" }
            ] },
            "subject": { "reference": "Patient/SYN-01" },
            "effectiveDateTime": "2026-03-01T09:30:00+02:00",
            "valueQuantity": { "value": 6.1, "unit": "%", "system": UCUM, "code": "%" }
        });
        if let (Some(b), Some(e)) = (base.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                b.insert(k.clone(), v.clone());
            }
        }
        base
    }

    fn bundle(resources: Vec<Value>) -> Vec<u8> {
        let entries: Vec<Value> = resources
            .into_iter()
            .map(|r| serde_json::json!({ "resource": r }))
            .collect();
        serde_json::to_vec(&serde_json::json!({ "resourceType": "Bundle", "type": "collection", "entry": entries }))
            .expect("json")
    }

    #[test]
    fn imports_a_lab_observation_as_utc() {
        let out =
            parse_fhir_json(&bundle(vec![observation(serde_json::json!({}))])).expect("parse");
        assert!(out.skipped.is_empty());
        let o = &out.observations[0];
        assert_eq!(
            (o.patient_id.as_str(), o.code.as_str(), o.unit.as_str()),
            ("SYN-01", "4548-4", "%")
        );
        assert_eq!(o.value, 6.1);
        assert_eq!(o.taken_at.to_string(), "2026-03-01 07:30:00");
        assert_eq!(o.source, "fhir");
    }

    #[test]
    fn single_observation_and_urn_subjects_work() {
        let o = observation(
            serde_json::json!({ "subject": { "reference": "urn:uuid:abc-123" }, "effectiveDateTime": "2026-03-01" }),
        );
        let out = parse_fhir_json(&serde_json::to_vec(&o).expect("json")).expect("parse");
        assert_eq!(out.observations[0].patient_id, "abc-123");
        assert_eq!(
            out.observations[0].taken_at.to_string(),
            "2026-03-01 00:00:00"
        );
    }

    #[test]
    fn every_unusable_observation_is_skipped_with_a_reason() {
        let cases = [
            (
                serde_json::json!({ "status": "entered-in-error" }),
                "status entered-in-error",
            ),
            (
                serde_json::json!({ "code": { "coding": [{ "system": "http://snomed.info/sct", "code": "1" }] } }),
                "no LOINC coding",
            ),
            (
                serde_json::json!({ "valueQuantity": null, "valueCodeableConcept": { "text": "positive" } }),
                "no valueQuantity",
            ),
            (
                serde_json::json!({ "valueQuantity": { "value": 6.1 } }),
                "without a unit",
            ),
            (
                serde_json::json!({ "effectiveDateTime": "2026-03" }),
                "not a full date",
            ),
            (
                serde_json::json!({ "effectiveDateTime": "2026-03-01T09:30:00" }),
                "not a full date",
            ),
            (
                serde_json::json!({ "subject": { "reference": "Group/g1" } }),
                "is not a Patient",
            ),
            (
                serde_json::json!({ "status": 5 }),
                "not a valid Observation",
            ),
        ];
        let resources: Vec<Value> = cases
            .iter()
            .map(|(extra, _)| observation(extra.clone()))
            .collect();
        let out = parse_fhir_json(&bundle(resources)).expect("parse");
        assert!(out.observations.is_empty());
        assert_eq!(out.skipped.len(), cases.len());
        for ((_, expected), skipped) in cases.iter().zip(&out.skipped) {
            assert!(
                skipped.reason.contains(expected),
                "{} !~ {expected}",
                skipped.reason
            );
            assert_eq!(skipped.resource, "Observation/o1");
        }
    }

    #[test]
    fn other_resources_are_ignored_and_bad_documents_rejected() {
        let patient = serde_json::json!({ "resourceType": "Patient", "id": "SYN-01" });
        let out = parse_fhir_json(&bundle(vec![patient, observation(serde_json::json!({}))]))
            .expect("parse");
        assert_eq!((out.observations.len(), out.skipped.len()), (1, 0));
        for bad in [
            &b"not json"[..],
            br#"{"resourceType":"Patient"}"#,
            br#"{"entry":[]}"#,
        ] {
            assert!(matches!(parse_fhir_json(bad), Err(IngestError::Fhir(_))));
        }
    }
}
