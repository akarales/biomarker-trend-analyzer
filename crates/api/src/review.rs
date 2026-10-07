//! Clinician review of drift signals, folded from the append-only events.
//!
//! - A signal is `(patient, code, rule, t)` — `t` is the result it fired
//!   on, so a NEW result raises a new, unreviewed signal even if the rule is
//!   the same: a review never silences future drift.
//! - State = the latest acknowledge / dismiss / reopen event (reopen →
//!   unreviewed); annotate adds a note and never changes the state.
//! - Snapshots are computed here from the server's own report at decision
//!   time — the client only names the signal.

use biomarker_drift::{DriftReport, Severity, Signal};
use serde::Serialize;
use serde_json::{Value, json};

use crate::store::{ReviewAction, ReviewEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewState {
    Unreviewed,
    Acknowledged,
    Dismissed,
}

/// Review status of one signal of a report (same order as `report.signals`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalReview {
    pub rule: String,
    pub t: i64,
    pub state: ReviewState,
    /// the event that set the state (absent when never decided / reopened)
    pub decided_by: Option<String>,
    pub decided_at: Option<i64>,
    pub reason: Option<String>,
    pub notes: usize,
}

pub fn rule_name(signal: &Signal) -> String {
    serde_json::to_value(signal.rule)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn matches(e: &ReviewEvent, code: &str, rule: &str, t: i64) -> bool {
    e.code == code && e.rule == rule && e.signal_t == t
}

/// Fold the events (oldest first) of one signal into its review status.
pub fn signal_review(code: &str, signal: &Signal, events: &[ReviewEvent]) -> SignalReview {
    let rule = rule_name(signal);
    let mut review = SignalReview {
        rule: rule.clone(),
        t: signal.t,
        state: ReviewState::Unreviewed,
        decided_by: None,
        decided_at: None,
        reason: None,
        notes: 0,
    };
    for e in events.iter().filter(|e| matches(e, code, &rule, signal.t)) {
        let state = match e.action {
            ReviewAction::Annotate => {
                review.notes += 1;
                continue;
            }
            ReviewAction::Acknowledge => ReviewState::Acknowledged,
            ReviewAction::Dismiss => ReviewState::Dismissed,
            ReviewAction::Reopen => ReviewState::Unreviewed,
        };
        review.state = state;
        let decided = state != ReviewState::Unreviewed;
        review.decided_by = decided.then(|| e.actor.clone());
        review.decided_at = decided.then_some(e.created_at);
        review.reason = if decided { e.reason.clone() } else { None };
    }
    review
}

pub fn report_reviews(report: &DriftReport, events: &[ReviewEvent]) -> Vec<SignalReview> {
    report
        .signals
        .iter()
        .map(|s| signal_review(&report.code, s, events))
        .collect()
}

/// Watch/alert signals still awaiting review (info never needs one).
pub fn unreviewed<'a>(report: &'a DriftReport, events: &[ReviewEvent]) -> Vec<&'a Signal> {
    report
        .signals
        .iter()
        .filter(|s| s.severity > Severity::Info)
        .filter(|s| signal_review(&report.code, s, events).state == ReviewState::Unreviewed)
        .collect()
}

/// The signal and its context as the clinician saw it (stored with the event).
pub fn snapshot(report: &DriftReport, signal: &Signal) -> Value {
    json!({
        "signal": signal,
        "status": report.status,
        "unit": report.unit,
        "latest": report.latest,
        "baseline": report.baseline,
        "as_of": report.as_of,
        "window_days": report.window_days,
        "analyte_reviewed": report.analyte.as_ref().map(|a| a.reviewed),
        "engine": concat!("biomarker-drift ", env!("CARGO_PKG_VERSION")),
    })
}

#[cfg(test)]
mod tests {
    use biomarker_drift::{AnalysisInput, Reading, analyze};

    use super::*;

    pub(crate) fn alert_report() -> DriftReport {
        let mut readings: Vec<Reading> = (0..8)
            .map(|i| Reading {
                t: i * 30 * 86_400,
                value: 5.6,
                unit: "%".into(),
            })
            .collect();
        readings.push(Reading {
            t: 8 * 30 * 86_400,
            value: 7.1,
            unit: "%".into(),
        });
        analyze(&AnalysisInput {
            code: "HBA1C",
            readings: &readings,
            as_of: None,
            trend_window_days: 365,
        })
    }

    fn event(
        id: i64,
        rule: &str,
        t: i64,
        action: ReviewAction,
        reason: Option<&str>,
    ) -> ReviewEvent {
        ReviewEvent {
            id,
            patient_id: "p".into(),
            code: "HBA1C".into(),
            rule: rule.into(),
            signal_t: t,
            action,
            reason: reason.map(str::to_string),
            actor: "demo-clinician".into(),
            snapshot: json!({}),
            created_at: 1_000 + id,
        }
    }

    #[test]
    fn the_latest_decision_wins_and_notes_do_not_change_state() {
        let report = alert_report();
        let prri = report
            .signals
            .iter()
            .find(|s| rule_name(s) == "prri")
            .expect("prri signal");
        let t = prri.t;
        let mut events = vec![event(1, "prri", t, ReviewAction::Acknowledge, None)];
        assert_eq!(
            signal_review("HBA1C", prri, &events).state,
            ReviewState::Acknowledged
        );
        events.push(event(
            2,
            "prri",
            t,
            ReviewAction::Annotate,
            Some("repeat booked"),
        ));
        let r = signal_review("HBA1C", prri, &events);
        assert_eq!(
            (r.state, r.notes, r.decided_at),
            (ReviewState::Acknowledged, 1, Some(1_001))
        );
        events.push(event(
            3,
            "prri",
            t,
            ReviewAction::Dismiss,
            Some("haemolysed sample"),
        ));
        let r = signal_review("HBA1C", prri, &events);
        assert_eq!(
            (r.state, r.reason.as_deref()),
            (ReviewState::Dismissed, Some("haemolysed sample"))
        );
        events.push(event(
            4,
            "prri",
            t,
            ReviewAction::Reopen,
            Some("new context"),
        ));
        let r = signal_review("HBA1C", prri, &events);
        assert_eq!(
            (r.state, r.reason, r.decided_by),
            (ReviewState::Unreviewed, None, None)
        );
    }

    #[test]
    fn a_review_never_covers_a_new_result_or_another_rule() {
        let report = alert_report();
        let prri = report
            .signals
            .iter()
            .find(|s| rule_name(s) == "prri")
            .expect("prri");
        let older = vec![event(
            1,
            "prri",
            prri.t - 86_400,
            ReviewAction::Dismiss,
            Some("old"),
        )];
        assert_eq!(
            signal_review("HBA1C", prri, &older).state,
            ReviewState::Unreviewed
        );
        let other_rule = vec![event(
            1,
            "threshold",
            prri.t,
            ReviewAction::Dismiss,
            Some("x"),
        )];
        assert_eq!(
            signal_review("HBA1C", prri, &other_rule).state,
            ReviewState::Unreviewed
        );
    }

    #[test]
    fn unreviewed_skips_info_and_decided_signals() {
        let report = alert_report();
        let open = unreviewed(&report, &[]);
        assert!(open.iter().all(|s| s.severity > Severity::Info));
        let all: Vec<ReviewEvent> = open
            .iter()
            .enumerate()
            .map(|(i, s)| {
                event(
                    i as i64 + 1,
                    &rule_name(s),
                    s.t,
                    ReviewAction::Acknowledge,
                    None,
                )
            })
            .collect();
        assert!(unreviewed(&report, &all).is_empty());
        let snap = snapshot(&report, open[0]);
        assert_eq!(snap["status"], "alert");
        assert!(snap["signal"]["explanation"].is_string() && snap["engine"].is_string());
    }
}
