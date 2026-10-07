//! Patient triage from drift reports: the worst status, how many
//! biomarkers alert or are on watch, and the single most severe signal —
//! what a clinician scans first. (Review state joins this in M6:
//! "worst *unreviewed* signal".)

use std::cmp::Reverse;

use biomarker_drift::{DriftReport, Rule, Severity, Status};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopSignal {
    pub code: String,
    pub display: String,
    pub rule: Rule,
    pub severity: Severity,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Triage {
    pub status: Status,
    /// biomarkers whose status is alert / watch
    pub alerts: usize,
    pub watches: usize,
    pub top_signal: Option<TopSignal>,
}

pub fn triage(reports: &[DriftReport]) -> Triage {
    let count = |s: Status| reports.iter().filter(|r| r.status == s).count();
    let top_signal = reports
        .iter()
        .flat_map(|r| r.signals.iter().map(move |s| (r, s)))
        .filter(|(_, s)| s.severity > Severity::Info)
        // max_by_key keeps the LAST maximum: reverse the order so the first wins
        .rev()
        .max_by_key(|(_, s)| s.severity)
        .map(|(r, s)| TopSignal {
            code: r.code.clone(),
            display: r
                .analyte
                .as_ref()
                .map_or_else(|| r.code.clone(), |a| a.display.to_string()),
            rule: s.rule,
            severity: s.severity,
            explanation: s.explanation.clone(),
        });
    Triage {
        status: reports
            .iter()
            .map(|r| r.status)
            .max()
            .unwrap_or(Status::Normal),
        alerts: count(Status::Alert),
        watches: count(Status::Watch),
        top_signal,
    }
}

/// Worst first: status, then alert count, then watch count, then id.
pub fn sort_key(
    t: &Triage,
    patient_id: &str,
) -> (Reverse<Status>, Reverse<usize>, Reverse<usize>, String) {
    (
        Reverse(t.status),
        Reverse(t.alerts),
        Reverse(t.watches),
        patient_id.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use biomarker_drift::{AnalysisInput, Reading, analyze};

    use super::*;

    fn report(code: &str, values: &[f64], unit: &str) -> DriftReport {
        let readings: Vec<Reading> = values
            .iter()
            .enumerate()
            .map(|(i, &value)| Reading {
                t: i as i64 * 7 * 86_400,
                value,
                unit: unit.into(),
            })
            .collect();
        analyze(&AnalysisInput {
            code,
            readings: &readings,
            as_of: None,
            trend_window_days: 365,
        })
    }

    #[test]
    fn worst_status_counts_and_top_signal() {
        let mut step = vec![5.6; 10];
        step.push(7.0);
        let reports = [
            report("CREAT", &[1.0; 12], "mg/dL"),
            report("HBA1C", &step, "%"),
            report("TSH", &[5.8; 12], "mIU/L"),
        ];
        let t = triage(&reports);
        assert_eq!(t.status, Status::Alert);
        assert_eq!((t.alerts, t.watches), (1, 1));
        let top = t.top_signal.expect("top signal");
        assert_eq!(
            (top.code.as_str(), top.severity),
            ("HBA1C", Severity::Alert)
        );
        assert_eq!(top.display, "Hemoglobin A1c");
    }

    #[test]
    fn info_only_is_normal_with_no_top_signal() {
        let t = triage(&[report("CREAT", &[1.45; 12], "mg/dL")]);
        assert_eq!(t.status, Status::Normal);
        assert!(
            t.top_signal.is_none(),
            "population info does not headline triage"
        );
        assert!(triage(&[]).top_signal.is_none());
    }

    #[test]
    fn sort_is_worst_first() {
        let normal = triage(&[report("CREAT", &[1.0; 12], "mg/dL")]);
        let watch = triage(&[report("TSH", &[5.8; 12], "mIU/L")]);
        let mut rows = [("b", &normal), ("a", &watch), ("c", &normal)];
        rows.sort_by_key(|(id, t)| sort_key(t, id));
        assert_eq!(rows.map(|(id, _)| id), ["a", "b", "c"]);
    }
}
