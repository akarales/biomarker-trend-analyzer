//! `analyze`: normalise → baseline → detectors → signals → status.

use crate::detectors::{baseline, control, limits, rcv, trend};
use crate::model::{
    AnalysisInput, AnalyteRef, DriftReport, Excluded, NotAssessed, Point, Rule, Severity, Signal,
    Status,
};
use crate::profiles::{self, AnalyteProfile};
use crate::stats::Z95;

fn not(rule: Rule, reason: impl Into<String>) -> NotAssessed {
    NotAssessed {
        rule,
        reason: reason.into(),
    }
}

/// Analyse one patient-biomarker series. Pure and deterministic: the same
/// input gives the same report whatever day it runs.
pub fn analyze(input: &AnalysisInput) -> DriftReport {
    let profile = profiles::lookup(input.code);
    let mut readings: Vec<_> = input
        .readings
        .iter()
        .filter(|r| r.value.is_finite())
        .collect();
    readings.sort_by_key(|r| r.t);
    let as_of = input.as_of.or_else(|| readings.last().map(|r| r.t));
    let mut excluded = Excluded::default();
    let in_range: Vec<_> = readings
        .into_iter()
        .filter(|r| {
            let keep = as_of.is_none_or(|a| r.t <= a);
            excluded.after_as_of += usize::from(!keep);
            keep
        })
        .collect();
    let unit = profile
        .map(|p| p.unit.to_string())
        .or_else(|| in_range.last().map(|r| r.unit.clone()))
        .unwrap_or_default();
    let points: Vec<Point> = in_range
        .iter()
        .filter_map(|r| {
            let v = match profile {
                Some(p) => p.to_canonical(r.value, &r.unit),
                None => (r.unit == unit).then_some(r.value),
            };
            excluded.unit_unknown += usize::from(v.is_none());
            v.map(|v| Point { t: r.t, v })
        })
        .collect();

    let mut report = DriftReport {
        code: input.code.to_string(),
        analyte: profile.map(|p| AnalyteRef {
            code: p.code,
            loinc: p.loinc[0],
            display: p.display,
            cvi: p.cvi,
            cva: p.cva,
            cvi_source: p.cvi_source,
            cva_source: p.cva_source,
            reviewed: profiles::REVIEWED,
        }),
        unit,
        as_of,
        window_days: input.trend_window_days,
        latest: points.last().copied(),
        population: profile.and_then(|p| p.population.clone()),
        thresholds: profile.map(|p| p.thresholds.to_vec()).unwrap_or_default(),
        points,
        excluded,
        baseline: None,
        rcv: None,
        rcv_jumps: Vec::new(),
        ewma: None,
        change_point: None,
        trend: None,
        signals: Vec::new(),
        not_assessed: Vec::new(),
        status: Status::Normal,
    };

    let Some(latest) = report.latest else {
        let all = [
            Rule::Prri,
            Rule::Rcv,
            Rule::Shift,
            Rule::Ewma,
            Rule::Trend,
            Rule::Threshold,
            Rule::Population,
        ];
        report.not_assessed = all
            .into_iter()
            .map(|r| not(r, "no results on or before the as-of date"))
            .collect();
        return report;
    };
    match profile {
        Some(p) => personal(&mut report, p, latest),
        None => {
            let reason = format!(
                "no analyte profile (biological variation, limits) for code {}",
                input.code
            );
            for rule in [
                Rule::Prri,
                Rule::Rcv,
                Rule::Shift,
                Rule::Ewma,
                Rule::Threshold,
                Rule::Population,
            ] {
                report.not_assessed.push(not(rule, reason.clone()));
            }
        }
    }
    trend_section(
        &mut report,
        profile,
        latest,
        as_of.unwrap_or(latest.t),
        input.trend_window_days,
    );

    report
        .signals
        .sort_by_key(|s| std::cmp::Reverse(s.severity));
    report.status = match report.signals.iter().map(|s| s.severity).max() {
        Some(Severity::Alert) => Status::Alert,
        Some(Severity::Watch) => Status::Watch,
        _ => Status::Normal,
    };
    report
}

/// Detectors that need biological variation and/or population limits.
fn personal(report: &mut DriftReport, p: &'static AnalyteProfile, latest: Point) {
    let unit = report.unit.clone();
    let points = &report.points;
    let sigma = p.sigma_log();
    let mut signals: Vec<Signal> = Vec::new();
    let mut skipped: Vec<NotAssessed> = Vec::new();

    if points.iter().any(|x| x.v <= 0.0) {
        for rule in [Rule::Prri, Rule::Rcv, Rule::Shift, Rule::Ewma] {
            skipped.push(not(
                rule,
                "non-positive values cannot be assessed on the log scale",
            ));
        }
    } else {
        match baseline::steady_state(points, sigma) {
            Some(base) => {
                signals.extend(baseline::signal(
                    &base, latest, &unit, p.display, p.cvi, p.cva,
                ));
                report.baseline = Some(base.view());
                report.change_point = control::change_point(points, &base);
                if let Some(cp) = &report.change_point {
                    signals.extend(control::shift_signal(cp, &base, latest.t, &unit, p.display));
                }
                match control::ewma(points, &base) {
                    Some((view, breach)) => {
                        if let Some(side) = breach {
                            signals.push(control::ewma_signal(
                                &view, side, latest.t, &unit, p.display,
                            ));
                        }
                        report.ewma = Some(view);
                    }
                    None => skipped.push(not(Rule::Ewma, "no results after the baseline")),
                }
            }
            None => {
                let earlier = points.len() - 1;
                let reason = format!(
                    "needs ≥ {} earlier results to estimate a personal set point (has {earlier})",
                    baseline::MIN_RESULTS
                );
                for rule in [Rule::Prri, Rule::Shift, Rule::Ewma] {
                    skipped.push(not(rule, reason.clone()));
                }
            }
        }
        report.rcv = Some(rcv::rcv(sigma, Z95));
        report.rcv_jumps = rcv::jumps(points, sigma);
        match points.len().checked_sub(2).map(|i| points[i]) {
            Some(previous) => {
                signals.extend(rcv::signal(previous, latest, sigma, &unit, p.display))
            }
            None => skipped.push(not(Rule::Rcv, "needs 2 results")),
        }
    }

    if p.thresholds.is_empty() {
        skipped.push(not(
            Rule::Threshold,
            "no clinical decision thresholds in the analyte profile",
        ));
    }
    let thresholds = limits::threshold_signals(p, latest, &unit);
    match limits::population_signal(p, latest, &unit, &thresholds) {
        Some(s) => signals.push(s),
        None if p.population.is_none() => skipped.push(not(
            Rule::Population,
            "no single population interval (targets are risk-based)",
        )),
        None => {}
    }
    signals.extend(thresholds);
    report.signals.extend(signals);
    report.not_assessed.extend(skipped);
}

fn trend_section(
    report: &mut DriftReport,
    profile: Option<&'static AnalyteProfile>,
    latest: Point,
    as_of: i64,
    window_days: i64,
) {
    match trend::trend(&report.points, as_of, window_days) {
        Ok(t) => {
            report.signals.extend(trend::signal(
                &t,
                latest,
                profile.map(|p| p.cvi),
                &report.unit,
                profile.map_or(report.code.as_str(), |p| p.display),
            ));
            report.trend = Some(t);
        }
        Err(n) => report.not_assessed.push(not(
            Rule::Trend,
            format!(
                "needs ≥ {} results in the last {window_days} days (has {n})",
                trend::MIN_RESULTS
            ),
        )),
    }
}
