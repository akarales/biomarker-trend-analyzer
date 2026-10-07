//! Monotone trend over the lookback window: Sen's slope with its 95 % CI
//! and the Mann–Kendall test (Mann 1945, Kendall 1975; Sen 1968; Gilbert
//! 1987, ch. 16). A trend is reported as rising/falling only when MK
//! p < 0.05 (v1 used a fixed slope cut-off with no test — defect D6), and
//! signalled only when its size exceeds the within-subject variation per
//! year (|Sen slope · 365.25 / median| ≥ CVI).

use crate::fmt::{self, DAY_SECONDS};
use crate::model::{Point, Rule, Severity, Signal, Trend, TrendDirection};
use crate::stats;

pub const MIN_RESULTS: usize = 4;
pub const DAYS_PER_YEAR: f64 = 365.25;
const ALPHA: f64 = 0.05;
pub const SOURCE: &str =
    "Mann–Kendall trend test + Sen slope with distribution-free 95 % CI (Gilbert 1987, ch. 16)";

pub fn trend(points: &[Point], as_of: i64, window_days: i64) -> Result<Trend, usize> {
    let from = as_of - window_days * DAY_SECONDS;
    let window: Vec<Point> = points.iter().copied().filter(|p| p.t >= from).collect();
    if window.len() < MIN_RESULTS {
        return Err(window.len());
    }
    let values: Vec<f64> = window.iter().map(|p| p.v).collect();
    let days: Vec<f64> = window
        .iter()
        .map(|p| (p.t - window[0].t) as f64 / DAY_SECONDS as f64)
        .collect();
    let mk = stats::mann_kendall(&values).ok_or(window.len())?;
    let sen = stats::sen_slope(&days, &values, mk.var_s).ok_or(window.len())?;
    let median = stats::median(&values).unwrap_or(0.0);
    let change_per_year = if median.abs() > f64::EPSILON {
        sen.slope * DAYS_PER_YEAR / median
    } else {
        0.0
    };
    let direction = match (mk.p_value < ALPHA, mk.s) {
        (true, s) if s > 0 => TrendDirection::Rising,
        (true, s) if s < 0 => TrendDirection::Falling,
        _ => TrendDirection::Flat,
    };
    Ok(Trend {
        n: window.len(),
        from: window[0].t,
        slope_per_day: sen.slope,
        ci_low_per_day: sen.ci_low,
        ci_high_per_day: sen.ci_high,
        change_per_year,
        tau: mk.tau,
        p_value: mk.p_value,
        direction,
    })
}

/// `cvi = None` (no profile): significance alone decides.
pub fn signal(
    t: &Trend,
    latest: Point,
    cvi: Option<f64>,
    unit: &str,
    display: &str,
) -> Option<Signal> {
    if t.direction == TrendDirection::Flat || cvi.is_some_and(|c| t.change_per_year.abs() < c) {
        return None;
    }
    let p = if t.p_value < 0.001 {
        "p < 0.001".to_string()
    } else {
        format!("p = {:.3}", t.p_value)
    };
    Some(Signal {
        rule: Rule::Trend,
        severity: Severity::Watch,
        t: latest.t,
        value: latest.v,
        threshold: cvi,
        explanation: format!(
            "{display} is {} about {} per year (Sen slope {} {unit}/year, 95 % CI {} to {}; Mann–Kendall τ {:.2}, {p}; \
             {} results since {}){}.",
            if t.direction == TrendDirection::Rising {
                "rising"
            } else {
                "falling"
            },
            fmt::pct(t.change_per_year),
            fmt::value(t.slope_per_day * DAYS_PER_YEAR),
            fmt::value(t.ci_low_per_day * DAYS_PER_YEAR),
            fmt::value(t.ci_high_per_day * DAYS_PER_YEAR),
            t.tau,
            t.n,
            fmt::date(t.from),
            cvi.map(|c| format!(", more than the within-subject variation of {}", fmt::cv(c)))
                .unwrap_or_default(),
        ),
        source: SOURCE.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: i64, start: f64, per_day: f64) -> Vec<Point> {
        (0..n)
            .map(|i| Point {
                t: i * 7 * DAY_SECONDS,
                v: start + per_day * (i * 7) as f64,
            })
            .collect()
    }

    #[test]
    fn needs_four_results_in_the_window() {
        let p = line(10, 100.0, 0.1);
        assert_eq!(trend(&p, p[9].t, 14), Err(3));
    }

    #[test]
    fn detects_a_clinically_sized_rise() {
        let p = line(48, 107.0, 0.058);
        let t = trend(&p, p[47].t, 365).expect("trend");
        assert_eq!(t.direction, TrendDirection::Rising);
        assert!((t.slope_per_day - 0.058).abs() < 1e-9);
        assert!(t.change_per_year > 0.15);
        assert!(signal(&t, p[47], Some(0.078), "mg/dL", "LDL").is_some());
    }

    #[test]
    fn small_but_significant_drift_is_not_signalled() {
        // significant yet only ~1 %/yr against a CVI of 4.4 %
        let p = line(48, 1.0, 0.00003);
        let t = trend(&p, p[47].t, 365).expect("trend");
        assert_eq!(t.direction, TrendDirection::Rising);
        assert!(signal(&t, p[47], Some(0.044), "mg/dL", "Creatinine").is_none());
    }
}
