//! Personal set point and personalised reference interval (prRI).
//!
//! Method: Coşkun A, Sandberg S, et al., "Personalized reference intervals
//! in laboratory medicine: a new model based on within-subject biological
//! variation", Clin Chem 2021;67:374–384 — a prediction interval for the
//! next result from n earlier steady-state results and the population
//! CVI/CVA. Implemented on the log scale (log-normal variant, consistent
//! with the asymmetric RCV): ln-mean ± z·σ·√(1 + 1/n).
//!
//! Steady state: the baseline is the EARLIEST run of consistent results
//! (3 to 10, each inside the prRI of the ones before it), never including
//! the result being judged. A sustained change therefore stays visible
//! until a clinician re-baselines it — it does not become "the new normal"
//! because a window slid over it (v1 defect D1).

use crate::fmt;
use crate::model::{Baseline, Point, Rule, Severity, Signal};
use crate::stats::{Z95, Z99};

pub const MIN_RESULTS: usize = 3;
pub const MAX_RESULTS: usize = 10;

pub const SOURCE: &str = "personalised reference interval: Coşkun A et al., Clin Chem 2021;67:374–384 \
                          (log-normal prediction interval from earlier steady-state results, population CVI/CVA)";

/// Baseline on the log scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogBaseline {
    pub mean: f64,
    /// σ of one result (biological + analytical), log scale
    pub sigma: f64,
    pub n: usize,
    /// index one past the last baseline point
    pub end: usize,
    pub from: i64,
    pub to: i64,
}

impl LogBaseline {
    /// σ of the prediction for a new result: σ·√(1 + 1/n).
    pub fn prediction_sigma(&self) -> f64 {
        self.sigma * (1.0 + 1.0 / self.n as f64).sqrt()
    }

    pub fn z(&self, value: f64) -> f64 {
        (value.ln() - self.mean) / self.prediction_sigma()
    }

    pub fn interval(&self, z: f64) -> (f64, f64) {
        let w = z * self.prediction_sigma();
        ((self.mean - w).exp(), (self.mean + w).exp())
    }

    pub fn set_point(&self) -> f64 {
        self.mean.exp()
    }

    pub fn view(&self) -> Baseline {
        let (prri_low, prri_high) = self.interval(Z95);
        Baseline {
            n: self.n,
            from: self.from,
            to: self.to,
            set_point: self.set_point(),
            prri_low,
            prri_high,
            level: 0.95,
        }
    }
}

/// Earliest steady-state run among all results except the latest.
pub fn steady_state(points: &[Point], sigma: f64) -> Option<LogBaseline> {
    let candidates = points.get(..points.len().checked_sub(1)?)?;
    if candidates.len() < MIN_RESULTS || candidates.iter().any(|p| p.v <= 0.0) {
        return None;
    }
    let logs: Vec<f64> = candidates.iter().map(|p| p.v.ln()).collect();
    let make = |n: usize| LogBaseline {
        mean: logs[..n].iter().sum::<f64>() / n as f64,
        sigma,
        n,
        end: n,
        from: candidates[0].t,
        to: candidates[n - 1].t,
    };
    let mut base = make(MIN_RESULTS);
    while base.n < MAX_RESULTS && base.n < candidates.len() {
        if base.z(candidates[base.n].v).abs() > Z95 {
            break;
        }
        base = make(base.n + 1);
    }
    Some(base)
}

/// prRI signal for the latest result (watch outside 95 %, alert outside 99 %).
pub fn signal(
    base: &LogBaseline,
    latest: Point,
    unit: &str,
    display: &str,
    cvi: f64,
    cva: f64,
) -> Option<Signal> {
    let z = base.z(latest.v);
    let severity = match z.abs() {
        a if a > Z99 => Severity::Alert,
        a if a > Z95 => Severity::Watch,
        _ => return None,
    };
    let (low, high) = base.interval(Z95);
    let (side, limit) = if z > 0.0 {
        ("above", high)
    } else {
        ("below", low)
    };
    Some(Signal {
        rule: Rule::Prri,
        severity,
        t: latest.t,
        value: latest.v,
        threshold: Some(limit),
        explanation: format!(
            "{display} {} {unit} is {side} this patient's personal reference interval {}–{} {unit} \
             (95 % prediction from {} results {} → {}, set point {} {unit}; CVI {}, CVA {}).",
            fmt::value(latest.v),
            fmt::value(low),
            fmt::value(high),
            base.n,
            fmt::date(base.from),
            fmt::date(base.to),
            fmt::value(base.set_point()),
            fmt::cv(cvi),
            fmt::cv(cva),
        ),
        source: SOURCE.into(),
    })
}

#[cfg(test)]
mod tests {
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
    fn needs_three_earlier_results() {
        assert!(steady_state(&pts(&[5.6, 5.6, 5.7]), 0.02).is_none());
        let b = steady_state(&pts(&[5.6, 5.6, 5.7, 5.6]), 0.02).expect("baseline");
        assert_eq!(b.n, 3);
    }

    #[test]
    fn excludes_the_latest_and_stops_at_ten() {
        let b = steady_state(&pts(&[5.6; 30]), 0.02).expect("baseline");
        assert_eq!(b.n, MAX_RESULTS);
        assert!((b.set_point() - 5.6).abs() < 1e-9);
    }

    #[test]
    fn stops_at_the_first_inconsistent_result() {
        let mut v = vec![5.6; 5];
        v.extend([7.0; 10]);
        let b = steady_state(&pts(&v), 0.02).expect("baseline");
        assert_eq!(b.n, 5);
        assert!(b.z(7.0) > Z99);
    }

    #[test]
    fn interval_widens_with_fewer_results() {
        let three = steady_state(&pts(&[5.6; 4]), 0.02).expect("b");
        let ten = steady_state(&pts(&[5.6; 11]), 0.02).expect("b");
        let w = |b: &LogBaseline| b.interval(Z95).1 - b.interval(Z95).0;
        assert!(w(&three) > w(&ten));
    }
}
