//! Deterministic offline backend, built only from the computed report —
//! demos and tests never need a GPU or network, and the stub can never
//! say anything the engine did not compute.

use biomarker_drift::{DriftReport, Severity};

use super::Explanation;

pub fn explanation(report: &DriftReport) -> Explanation {
    let name = report
        .analyte
        .as_ref()
        .map_or(report.code.as_str(), |a| a.display);
    let flagged: Vec<&str> = report
        .signals
        .iter()
        .filter(|s| s.severity > Severity::Info)
        .map(|s| s.explanation.as_str())
        .collect();
    let summary = if flagged.is_empty() {
        format!(
            "No drift rule fired for {name}. Offline stub: this text restates the computed signals; choose an Ollama or Claude model for a generated explanation."
        )
    } else {
        format!(
            "{} (Offline stub: this text restates the computed signals; choose an Ollama or Claude model for a generated explanation.)",
            flagged.join(" ")
        )
    };
    let not_assessed: Vec<String> = report
        .not_assessed
        .iter()
        .map(|n| format!("{:?}: {}", n.rule, n.reason).to_lowercase())
        .collect();
    Explanation {
        summary,
        interpretation: "Consider first whether the change exceeds analytical and within-subject biological variation (see the reference change value and the personal reference interval), then pre-analytical factors, then clinical causes. Stub mode: no model call was made.".into(),
        follow_up: "Consider confirming with a repeat measurement before acting, and reviewing medications and intercurrent illness. Considerations for clinical judgement, not orders.".into(),
        limitations: format!(
            "Medications, symptoms and history are not in this record; no signal does not mean healthy.{}",
            if not_assessed.is_empty() { String::new() } else { format!(" Not assessed: {}.", not_assessed.join("; ")) }
        ),
        status: format!("{:?}", report.status).to_lowercase(),
    }
}
