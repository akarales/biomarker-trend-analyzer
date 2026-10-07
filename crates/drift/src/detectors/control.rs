//! Control charts after the baseline, on standardised log results
//! z = (ln x − set point) / σ, with σ from CVI/CVA (never the series' own
//! spread, which made v1's limits depend on how noisy the window was).
//!
//! - Tabular CUSUM (Page 1954; Montgomery, Introduction to Statistical
//!   Quality Control, ch. 9): k = 0.5, h = 5. The change point is the
//!   first result after the last time the statistic was zero.
//! - EWMA (Roberts 1959; Montgomery ch. 9): λ = 0.2, L = 3 with the exact
//!   time-varying σ_EWMA = √(λ/(2−λ)·(1−(1−λ)^{2i})) — v1 used σ itself,
//!   making its limits ~2.4× too wide (defect D3).
//!
//! Each z is clipped to ±4 before charting (a Huber-type robust variant):
//! one erroneous result adds at most 4 − k = 3.5 < h to the CUSUM, so it
//! cannot raise a shift on its own; a real step needs two results.
//!
//! Both charts count results, not days — lab sampling is irregular, so the
//! change point is reported as the date of a result, not an interpolation.

use super::baseline::LogBaseline;
use crate::fmt;
use crate::model::{ChangePoint, Ewma, Point, Rule, Severity, Signal};

pub const CUSUM_K: f64 = 0.5;
pub const CUSUM_H: f64 = 5.0;
pub const EWMA_LAMBDA: f64 = 0.2;
pub const EWMA_L: f64 = 3.0;
const Z_CLIP: f64 = 4.0;

pub const SHIFT_SOURCE: &str = "tabular CUSUM (k 0.5σ, h 5σ; Montgomery, Statistical Quality Control, ch. 9) on \
                                log results standardised by population CVI/CVA";
pub const EWMA_SOURCE: &str = "EWMA (λ 0.2, L 3, exact time-varying limits; Montgomery, Statistical Quality \
                               Control, ch. 9) on log results standardised by population CVI/CVA";

fn z(base: &LogBaseline, v: f64) -> f64 {
    ((v.ln() - base.mean) / base.sigma).clamp(-Z_CLIP, Z_CLIP)
}

/// First CUSUM alarm after the baseline, with its change point.
pub fn change_point(points: &[Point], base: &LogBaseline) -> Option<ChangePoint> {
    let (mut hi, mut lo) = (0.0f64, 0.0f64);
    let (mut start_hi, mut start_lo) = (base.end, base.end);
    for i in base.end..points.len() {
        let zi = z(base, points[i].v);
        hi = (hi + zi - CUSUM_K).max(0.0);
        lo = (lo - zi - CUSUM_K).max(0.0);
        if hi == 0.0 {
            start_hi = i + 1;
        }
        if lo == 0.0 {
            start_lo = i + 1;
        }
        let start = if hi > CUSUM_H {
            start_hi
        } else if lo > CUSUM_H {
            start_lo
        } else {
            continue;
        };
        let run = &points[start..];
        let after = (run.iter().map(|p| p.v.ln()).sum::<f64>() / run.len() as f64).exp();
        let before = base.set_point();
        return Some(ChangePoint {
            t: points[start].t,
            detected_t: points[i].t,
            before,
            after,
            change: after / before - 1.0,
        });
    }
    None
}

/// The shift matters when the level since the change point is outside the
/// prRI; a spike that reverted averages back inside and stays quiet.
pub fn shift_signal(
    cp: &ChangePoint,
    base: &LogBaseline,
    latest_t: i64,
    unit: &str,
    display: &str,
) -> Option<Signal> {
    let (low, high) = base.interval(crate::stats::Z95);
    if (low..=high).contains(&cp.after) {
        return None;
    }
    Some(Signal {
        rule: Rule::Shift,
        severity: Severity::Watch,
        t: latest_t,
        value: cp.after,
        threshold: Some(if cp.after > high { high } else { low }),
        explanation: format!(
            "Sustained shift: since {} {display} averages {} {unit} ({} vs the set point {} {unit}), outside the \
             personal reference interval; CUSUM crossed its limit on {}.",
            fmt::date(cp.t),
            fmt::value(cp.after),
            fmt::pct(cp.change),
            fmt::value(cp.before),
            fmt::date(cp.detected_t),
        ),
        source: SHIFT_SOURCE.into(),
    })
}

/// Which side of its control limits the EWMA ended on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breach {
    Above,
    Below,
}

/// EWMA at the latest result and its breach, if any. The decision uses the
/// clipped (robust) statistic; the reported value is the unclipped smoothed
/// level, so the explanation never understates the real results.
pub fn ewma(points: &[Point], base: &LogBaseline) -> Option<(Ewma, Option<Breach>)> {
    let after = points.get(base.end..).filter(|s| !s.is_empty())?;
    let (mut robust, mut level) = (0.0, 0.0);
    for p in after {
        robust = EWMA_LAMBDA * z(base, p.v) + (1.0 - EWMA_LAMBDA) * robust;
        level = EWMA_LAMBDA * (p.v.ln() - base.mean) / base.sigma + (1.0 - EWMA_LAMBDA) * level;
    }
    let i = after.len() as i32;
    let width = EWMA_L
        * (EWMA_LAMBDA / (2.0 - EWMA_LAMBDA) * (1.0 - (1.0 - EWMA_LAMBDA).powi(2 * i))).sqrt();
    let to_value = |zv: f64| (base.mean + zv * base.sigma).exp();
    let view = Ewma {
        value: to_value(level),
        low: to_value(-width),
        high: to_value(width),
    };
    let breach = match robust {
        r if r > width => Some(Breach::Above),
        r if r < -width => Some(Breach::Below),
        _ => None,
    };
    Some((view, breach))
}

pub fn ewma_signal(
    view: &Ewma,
    breach: Breach,
    latest_t: i64,
    unit: &str,
    display: &str,
) -> Signal {
    let above = breach == Breach::Above;
    Signal {
        rule: Rule::Ewma,
        severity: Severity::Watch,
        t: latest_t,
        value: view.value,
        threshold: Some(if above { view.high } else { view.low }),
        explanation: format!(
            "The smoothed {display} level (EWMA {} {unit}) is {} its control limits {}–{} {unit}: a persistent \
             change has accumulated over recent results.",
            fmt::value(view.value),
            if above { "above" } else { "below" },
            fmt::value(view.low),
            fmt::value(view.high),
        ),
        source: EWMA_SOURCE.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::baseline::steady_state;
    use super::*;

    fn pts(values: &[f64]) -> Vec<Point> {
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| Point {
                t: i as i64 * 86_400,
                v,
            })
            .collect()
    }

    #[test]
    fn step_gives_a_change_point_at_the_step() {
        let mut v = vec![5.6; 12];
        v.extend([7.0; 6]);
        let p = pts(&v);
        let base = steady_state(&p, 0.0192).expect("baseline");
        let cp = change_point(&p, &base).expect("change point");
        assert_eq!(cp.t, p[12].t, "change point = first shifted result");
        assert_eq!(cp.detected_t, p[13].t, "needs two results (clipped z)");
        assert!((cp.change - 0.25).abs() < 0.01);
    }

    #[test]
    fn a_single_outlier_cannot_alarm() {
        let mut v = vec![1.0; 12];
        v.push(1.6);
        v.extend([1.0; 3]);
        let p = pts(&v);
        let base = steady_state(&p, 0.048).expect("baseline");
        assert!(change_point(&p, &base).is_none());
        let (_, breach) = ewma(&p, &base).expect("ewma");
        assert_eq!(breach, None);
    }

    #[test]
    fn ewma_limits_use_the_exact_sigma() {
        // after one result: L·√(λ/(2−λ)·(1−(1−λ)²)) = 3·√(0.2/1.8·0.36) = 0.6σ
        let p = pts(&[100.0, 100.0, 100.0, 100.0]);
        let base = steady_state(&p, 0.05).expect("baseline");
        let (view, _) = ewma(&p, &base).expect("ewma");
        let width = (view.high / 100.0).ln() / 0.05;
        assert!((width - 0.6).abs() < 1e-9, "{width}");
    }
}
