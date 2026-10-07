//! Patient triage from drift reports and review events. Ordering and the
//! headline signal use what still needs a clinician — the worst
//! *unreviewed* watch/alert signal — while the computed status stays
//! visible (a review never hides a finding, it only moves it down).

use std::cmp::Reverse;

use biomarker_drift::{DriftReport, Rule, Severity, Status};
use serde::Serialize;

use crate::review;
use crate::store::ReviewEvent;

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
    /// worst computed status (independent of review)
    pub status: Status,
    /// biomarkers whose status is alert / watch
    pub alerts: usize,
    pub watches: usize,
    /// watch/alert signals not yet acknowledged or dismissed
    pub unreviewed: usize,
    /// the most severe unreviewed signal (null when everything is reviewed)
    pub top_signal: Option<TopSignal>,
}

pub fn triage(reports: &[DriftReport], events: &[ReviewEvent]) -> Triage {
    let count = |s: Status| reports.iter().filter(|r| r.status == s).count();
    let open: Vec<(&DriftReport, &biomarker_drift::Signal)> = reports
        .iter()
        .flat_map(|r| {
            review::unreviewed(r, events)
                .into_iter()
                .map(move |s| (r, s))
        })
        .collect();
    let top_signal = open
        .iter()
        // max_by_key keeps the LAST maximum: reverse so the first wins
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
        unreviewed: open.len(),
        top_signal,
    }
}

/// Worst first: unreviewed severity, unreviewed count, computed status,
/// alert count, watch count, id.
#[allow(clippy::type_complexity)]
pub fn sort_key(
    t: &Triage,
    patient_id: &str,
) -> (
    Reverse<Option<Severity>>,
    Reverse<usize>,
    Reverse<Status>,
    Reverse<usize>,
    Reverse<usize>,
    String,
) {
    (
        Reverse(t.top_signal.as_ref().map(|s| s.severity)),
        Reverse(t.unreviewed),
        Reverse(t.status),
        Reverse(t.alerts),
        Reverse(t.watches),
        patient_id.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use biomarker_drift::{AnalysisInput, Reading, analyze};
    use serde_json::json;

    use super::*;
    use crate::store::ReviewAction;

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

    fn step() -> Vec<f64> {
        let mut v = vec![5.6; 10];
        v.push(7.0);
        v
    }

    fn acknowledge_all(r: &DriftReport) -> Vec<ReviewEvent> {
        review::unreviewed(r, &[])
            .iter()
            .enumerate()
            .map(|(i, s)| ReviewEvent {
                id: i as i64 + 1,
                patient_id: "p".into(),
                code: r.code.clone(),
                rule: review::rule_name(s),
                signal_t: s.t,
                action: ReviewAction::Acknowledge,
                reason: None,
                actor: "demo-clinician".into(),
                snapshot: json!({}),
                created_at: 0,
            })
            .collect()
    }

    #[test]
    fn worst_status_counts_and_top_unreviewed_signal() {
        let reports = [
            report("CREAT", &[1.0; 12], "mg/dL"),
            report("HBA1C", &step(), "%"),
            report("TSH", &[5.8; 12], "mIU/L"),
        ];
        let t = triage(&reports, &[]);
        assert_eq!(t.status, Status::Alert);
        assert_eq!((t.alerts, t.watches), (1, 1));
        assert!(t.unreviewed >= 2);
        let top = t.top_signal.expect("top signal");
        assert_eq!(
            (top.code.as_str(), top.severity),
            ("HBA1C", Severity::Alert)
        );
        assert_eq!(top.display, "Hemoglobin A1c");
    }

    #[test]
    fn reviewing_keeps_the_status_but_moves_the_patient_down() {
        let hba1c = report("HBA1C", &step(), "%");
        let reviewed = triage(std::slice::from_ref(&hba1c), &acknowledge_all(&hba1c));
        assert_eq!(
            reviewed.status,
            Status::Alert,
            "a review never hides the finding"
        );
        assert_eq!(
            (reviewed.unreviewed, reviewed.top_signal.is_none()),
            (0, true)
        );

        let watch = triage(&[report("TSH", &[5.8; 12], "mIU/L")], &[]);
        let mut rows = [("a-reviewed-alert", &reviewed), ("b-open-watch", &watch)];
        rows.sort_by_key(|(id, t)| sort_key(t, id));
        assert_eq!(rows.map(|(id, _)| id), ["b-open-watch", "a-reviewed-alert"]);
    }

    #[test]
    fn info_only_is_normal_with_nothing_to_review() {
        let t = triage(&[report("CREAT", &[1.45; 12], "mg/dL")], &[]);
        assert_eq!((t.status, t.unreviewed), (Status::Normal, 0));
        assert!(
            t.top_signal.is_none(),
            "population info does not headline triage"
        );
        assert!(triage(&[], &[]).top_signal.is_none());
    }

    #[test]
    fn sort_is_worst_first_then_id() {
        let normal = triage(&[report("CREAT", &[1.0; 12], "mg/dL")], &[]);
        let watch = triage(&[report("TSH", &[5.8; 12], "mIU/L")], &[]);
        let mut rows = [("b", &normal), ("a", &watch), ("c", &normal)];
        rows.sort_by_key(|(id, t)| sort_key(t, id));
        assert_eq!(rows.map(|(id, _)| id), ["a", "b", "c"]);
    }
}
