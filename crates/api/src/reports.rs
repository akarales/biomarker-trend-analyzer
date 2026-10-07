//! A patient's analyses, the one place that loads series + demographics
//! and runs the engine (summary, triage, series, reviews and explanations
//! all go through here, so they always agree).
//!
//! Derived series: when a patient has creatinine results AND a recorded
//! sex (female/male) and birth year, an `EGFR` series is derived per
//! result (2021 CKD-EPI, `biomarker_drift::egfr`) and analysed like any
//! other analyte (prRI, RCV, trend, KDIGO categories). Without them the
//! creatinine report says why kidney-function staging was not assessed.

use std::collections::BTreeMap;

use biomarker_drift::egfr::{self, EGFR_CODE, NotDerived};
use biomarker_drift::{AnalysisInput, DriftReport, NotAssessed, Reading, Rule};
use biomarker_ingest::Observation;
use serde::Serialize;

use crate::analysis::{self, Options};
use crate::error::ApiError;
use crate::state::AppState;
use crate::store::Demographics;

const CREAT: &str = "CREAT";

/// How a derived series was computed (shown next to it).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Derived {
    pub from: &'static str,
    pub method: &'static str,
    pub gender: Option<String>,
    pub birth_year: Option<i32>,
}

/// One analysed series: the stored inputs, the report, and the derivation
/// when the series is computed rather than measured.
pub struct SeriesReport {
    pub observations: Vec<Observation>,
    pub report: DriftReport,
    pub derived: Option<Derived>,
}

fn readings(series: &[Observation]) -> Vec<Reading> {
    series
        .iter()
        .map(|o| Reading {
            t: o.taken_at.and_utc().timestamp(),
            value: o.value,
            unit: o.unit.clone(),
        })
        .collect()
}

/// eGFR report from creatinine + demographics, or why it cannot be derived.
pub fn egfr_report(
    creatinine: &[Observation],
    demographics: Option<&Demographics>,
    options: Options,
) -> Result<(DriftReport, Derived), NotDerived> {
    let demographics = demographics.ok_or(NotDerived::NoDemographics)?;
    let derived = egfr::derive(
        &readings(creatinine),
        demographics.sex(),
        demographics.birth_year,
    )?;
    let report = biomarker_drift::analyze(&AnalysisInput {
        code: EGFR_CODE,
        readings: &derived,
        as_of: options.as_of,
        trend_window_days: options.window_days,
    });
    Ok((
        report,
        Derived {
            from: CREAT,
            method: egfr::METHOD,
            gender: demographics.gender.clone(),
            birth_year: demographics.birth_year,
        },
    ))
}

/// Creatinine has no thresholds of its own: point to the eGFR staging, or
/// say why it could not be done.
fn annotate_creatinine(report: &mut DriftReport, egfr: Result<(), &NotDerived>) {
    let reason = match egfr {
        Ok(()) => {
            "kidney-function categories (KDIGO) are assessed on the derived eGFR series".to_string()
        }
        Err(why) => format!("kidney-function staging not assessed: {}", why.reason()),
    };
    report.not_assessed.retain(|n| n.rule != Rule::Threshold);
    report.not_assessed.push(NotAssessed {
        rule: Rule::Threshold,
        reason,
    });
}

/// All analyses of one patient.
pub struct PatientAnalysis {
    /// stored codes in name order, then derived series
    pub reports: Vec<DriftReport>,
    /// how each derived series was computed, by code
    pub derived: BTreeMap<String, Derived>,
    pub demographics: Option<Demographics>,
}

/// Every report of a patient, derived series included.
pub async fn patient_reports(
    state: &AppState,
    patient_id: &str,
    options: Options,
) -> Result<PatientAnalysis, ApiError> {
    let mut out = Vec::new();
    let mut creatinine = None;
    for code in state.store.patient_codes(patient_id).await? {
        let series = state.store.series(patient_id, &code).await?;
        if let Some(report) = analysis::report(&series, &code, options) {
            out.push(report);
            if code == CREAT {
                creatinine = Some(series);
            }
        }
    }
    let demographics = state.store.demographics(patient_id).await?;
    let mut derived = BTreeMap::new();
    if let Some(series) = creatinine {
        let egfr = egfr_report(&series, demographics.as_ref(), options);
        if let Some(creat) = out.iter_mut().find(|r| r.code == CREAT) {
            annotate_creatinine(creat, egfr.as_ref().map(|_| ()));
        }
        if let Ok((report, how)) = egfr {
            derived.insert(report.code.clone(), how);
            out.push(report);
        }
    }
    Ok(PatientAnalysis {
        reports: out,
        derived,
        demographics,
    })
}

/// One series (measured or derived) with its report.
pub async fn series_report(
    state: &AppState,
    patient_id: &str,
    code: &str,
    options: Options,
) -> Result<SeriesReport, ApiError> {
    let not_found = || ApiError::NotFound(format!("no observations for {patient_id} / {code}"));
    if code == EGFR_CODE {
        let observations = state.store.series(patient_id, CREAT).await?;
        let demographics = state.store.demographics(patient_id).await?;
        let (report, derived) =
            egfr_report(&observations, demographics.as_ref(), options).map_err(|_| not_found())?;
        return Ok(SeriesReport {
            observations,
            report,
            derived: Some(derived),
        });
    }
    let observations = state.store.series(patient_id, code).await?;
    let mut report = analysis::report(&observations, code, options).ok_or_else(not_found)?;
    if code == CREAT {
        let demographics = state.store.demographics(patient_id).await?;
        let egfr = egfr_report(&observations, demographics.as_ref(), options);
        annotate_creatinine(&mut report, egfr.as_ref().map(|_| ()));
    }
    Ok(SeriesReport {
        observations,
        report,
        derived: None,
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn creat(year: i32, value: f64) -> Observation {
        Observation {
            patient_id: "p".into(),
            code: CREAT.into(),
            value,
            unit: "mg/dL".into(),
            taken_at: NaiveDate::from_ymd_opt(year, 3, 1)
                .and_then(|d| d.and_hms_opt(8, 0, 0))
                .expect("date"),
            source: "test".into(),
        }
    }

    const ALL: Options = Options {
        as_of: None,
        window_days: 3650,
    };

    #[test]
    fn egfr_follows_creatinine_and_flags_kdigo_categories() {
        let series: Vec<Observation> = (2018..2027)
            .map(|y| creat(y, 1.0 + 0.12 * f64::from(y - 2018)))
            .collect();
        let demo = Demographics {
            patient_id: "p".into(),
            gender: Some("male".into()),
            birth_year: Some(1957),
        };
        let (report, derived) = egfr_report(&series, Some(&demo), ALL).expect("derived");
        assert_eq!(
            (report.code.as_str(), report.unit.as_str()),
            ("EGFR", "mL/min/{1.73_m2}")
        );
        assert_eq!(report.points.len(), series.len());
        let latest = report.latest.expect("latest").v;
        // male, 69 years, Scr 1.96 mg/dL
        assert!((latest - egfr::ckd_epi_2021(1.96, 69.0, egfr::Sex::Male)).abs() < 1e-6);
        let threshold = report
            .signals
            .iter()
            .find(|s| s.rule == Rule::Threshold)
            .expect("KDIGO signal");
        assert!(
            threshold.explanation.contains("KDIGO G3b"),
            "{}",
            threshold.explanation
        );
        assert_eq!(derived.from, "CREAT");
        assert!(derived.method.contains("2021 CKD-EPI"));
    }

    #[test]
    fn missing_or_non_binary_demographics_are_explained_on_creatinine() {
        let series = vec![creat(2025, 1.0), creat(2026, 1.1)];
        assert!(matches!(
            egfr_report(&series, None, ALL),
            Err(NotDerived::NoDemographics)
        ));
        let other = Demographics {
            patient_id: "p".into(),
            gender: Some("other".into()),
            birth_year: Some(1970),
        };
        let err = egfr_report(&series, Some(&other), ALL).expect_err("not derived");
        let mut report = analysis::report(&series, CREAT, ALL).expect("creat report");
        annotate_creatinine(&mut report, Err(&err));
        let note = report
            .not_assessed
            .iter()
            .find(|n| n.rule == Rule::Threshold)
            .expect("note");
        assert!(note.reason.contains("female or male"), "{}", note.reason);
        assert_eq!(
            report
                .not_assessed
                .iter()
                .filter(|n| n.rule == Rule::Threshold)
                .count(),
            1
        );
    }
}
