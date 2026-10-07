//! The model's input for "explain this drift": the computed report as
//! plain text. Only analyte data goes in — no patient identifier (the
//! pseudonym stays server-side), no free text from uploads.

use biomarker_drift::fmt::{self, date, pct, value};
use biomarker_drift::{Direction, DriftReport};

/// Results quoted to the model (newest last).
const RECENT_RESULTS: usize = 12;

pub const DISCLAIMER: &str = "AI-generated draft, grounded in the computed \
signals. The computed status and signals are authoritative; this text is not. \
Synthetic demo data — not medical advice; clinician review required.";

fn wire<T: serde::Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn context(report: &DriftReport) -> String {
    let unit = &fmt::unit(&report.unit);
    let mut out = Vec::new();
    match &report.analyte {
        Some(a) => out.push(format!(
            "Analyte: {} (LOINC {}), unit {unit}",
            a.display, a.loinc
        )),
        None => out.push(format!(
            "Analyte: {} (no analyte profile), unit {unit}",
            report.code
        )),
    }
    let as_of = report.as_of.map_or("latest result".to_string(), date);
    out.push(format!(
        "Analysis as of {as_of}; trend window {} days.",
        report.window_days
    ));
    out.push(format!(
        "Computed status (authoritative): {}",
        wire(report.status).to_uppercase()
    ));
    if let Some(l) = &report.latest {
        out.push(format!(
            "Latest result: {} {unit} on {}",
            value(l.v),
            date(l.t)
        ));
    }
    if let Some(b) = &report.baseline {
        out.push(format!(
            "Personal baseline: set point {} {unit} from {} results {} to {}; personal reference interval {}–{} {unit} ({:.0} % prediction interval)",
            value(b.set_point), b.n, date(b.from), date(b.to), value(b.prri_low), value(b.prri_high), b.level * 100.0
        ));
    }
    if let Some(a) = &report.analyte {
        out.push(format!("Within-subject biological variation (CVI) {}; analytical imprecision (CVA) {} (assumed).", pct_plain(a.cvi), pct_plain(a.cva)));
    }
    if let Some(p) = &report.population {
        out.push(format!(
            "Population reference interval: {}–{} {unit} ({})",
            value(p.low),
            value(p.high),
            p.source
        ));
    }
    for t in &report.thresholds {
        let op = if t.direction == Direction::Above {
            "≥"
        } else {
            "<"
        };
        out.push(format!(
            "Clinical threshold: {op} {} {unit} = {} ({})",
            value(t.value),
            t.label,
            t.source
        ));
    }
    if report.signals.is_empty() {
        out.push("Signals: none — no rule fired (this does not mean healthy).".into());
    } else {
        out.push("Signals (computed, authoritative — do not re-grade):".into());
        for s in &report.signals {
            out.push(format!(
                "- [{}] {}: {} (source: {})",
                wire(s.severity).to_uppercase(),
                wire(s.rule),
                s.explanation,
                s.source
            ));
        }
    }
    for n in &report.not_assessed {
        out.push(format!("Not assessed — {}: {}", wire(n.rule), n.reason));
    }
    if let Some(t) = &report.trend {
        out.push(format!(
            "Trend: {} ({} per year), Mann–Kendall p = {:.3}",
            wire(t.direction),
            pct(t.change_per_year),
            t.p_value
        ));
    }
    let start = report.points.len().saturating_sub(RECENT_RESULTS);
    let recent: Vec<String> = report.points[start..]
        .iter()
        .map(|p| format!("{} {}", date(p.t), value(p.v)))
        .collect();
    out.push(format!(
        "Recent results ({unit}, oldest to newest): {}",
        recent.join("; ")
    ));
    out.push("Task: explain this drift for the reviewing clinician.".into());
    out.join("\n")
}

fn pct_plain(fraction: f64) -> String {
    format!("{:.1} %", fraction * 100.0)
}

#[cfg(test)]
mod tests {
    use biomarker_drift::{AnalysisInput, Reading, analyze};

    use super::*;

    fn report(values: &[f64]) -> DriftReport {
        let readings: Vec<Reading> = values
            .iter()
            .enumerate()
            .map(|(i, &value)| Reading {
                t: 1_767_225_600 + i as i64 * 30 * 86_400,
                value,
                unit: "%".into(),
            })
            .collect();
        analyze(&AnalysisInput {
            code: "HBA1C",
            readings: &readings,
            as_of: None,
            trend_window_days: 365,
        })
    }

    #[test]
    fn context_carries_the_computed_facts() {
        let mut values = vec![5.6, 5.5, 5.6, 5.7, 5.6, 5.5];
        values.push(7.0);
        let text = context(&report(&values));
        for needle in [
            "Analyte: Hemoglobin A1c (LOINC 4548-4), unit %",
            "Computed status (authoritative): ALERT",
            "Latest result: 7.00 %",
            "personal reference interval",
            "Clinical threshold: ≥ 6.50 % = diabetes range",
            "- [ALERT] prri:",
            "Recent results (%, oldest to newest): 2026-01-01 5.60;",
            "Task: explain this drift",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
    }

    #[test]
    fn says_when_nothing_fired_and_quotes_only_recent_results() {
        let text = context(&report(&[5.4; 20]));
        assert!(text.contains("Signals: none — no rule fired (this does not mean healthy)."));
        let recent = text
            .lines()
            .find(|l| l.starts_with("Recent results"))
            .expect("recent line");
        assert_eq!(recent.matches(';').count(), RECENT_RESULTS - 1);
    }
}
