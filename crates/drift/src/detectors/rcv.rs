//! Reference change value: is the change between two consecutive results
//! bigger than analytical + within-subject variation? Log-normal,
//! asymmetric form (Fokkema MR et al., Clin Chem 2006;52:1602–1603; as used
//! by EuBIVAS): RCV = exp(±z·√2·σ) − 1 with σ² = ln(1+CVA²) + ln(1+CVI²).

use crate::fmt;
use crate::model::{Jump, Point, Rcv, Rule, Severity, Signal};
use crate::stats::{Z95, Z99};

pub const SOURCE: &str = "reference change value, log-normal asymmetric: Fokkema MR et al., \
                          Clin Chem 2006;52:1602–1603 (z 1.96 two-sided, population CVI/CVA)";

pub fn rcv(sigma: f64, z: f64) -> Rcv {
    let w = z * std::f64::consts::SQRT_2 * sigma;
    Rcv {
        up: w.exp() - 1.0,
        down: (-w).exp() - 1.0,
    }
}

fn exceeds(change: f64, limit: Rcv) -> bool {
    change > limit.up || change < limit.down
}

/// Every consecutive pair whose change exceeds the 95 % RCV.
pub fn jumps(points: &[Point], sigma: f64) -> Vec<Jump> {
    let limit = rcv(sigma, Z95);
    points
        .windows(2)
        .filter(|w| w[0].v > 0.0)
        .map(|w| Jump {
            from_t: w[0].t,
            to_t: w[1].t,
            change: w[1].v / w[0].v - 1.0,
        })
        .filter(|j| exceeds(j.change, limit))
        .collect()
}

/// Signal for the latest pair (watch beyond 95 %, alert beyond 99 %).
pub fn signal(
    previous: Point,
    latest: Point,
    sigma: f64,
    unit: &str,
    display: &str,
) -> Option<Signal> {
    if previous.v <= 0.0 {
        return None;
    }
    let change = latest.v / previous.v - 1.0;
    let r95 = rcv(sigma, Z95);
    let severity = if exceeds(change, rcv(sigma, Z99)) {
        Severity::Alert
    } else if exceeds(change, r95) {
        Severity::Watch
    } else {
        return None;
    };
    let limit = if change > 0.0 { r95.up } else { r95.down };
    Some(Signal {
        rule: Rule::Rcv,
        severity,
        t: latest.t,
        value: latest.v,
        threshold: Some(limit),
        explanation: format!(
            "{display} changed {} from {} {unit} ({}) to {} {unit} ({}); the reference change value is {} / {} — \
             larger than expected from analytical and within-subject variation.",
            fmt::pct(change),
            fmt::value(previous.v),
            fmt::date(previous.t),
            fmt::value(latest.v),
            fmt::date(latest.t),
            fmt::pct(r95.up),
            fmt::pct(r95.down),
        ),
        source: SOURCE.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asymmetric_and_matches_the_worked_example() {
        // creatinine CVI 4.4 %, CVA 1.1 % → ≈ +13.4 % / −11.8 % (research.md)
        let s = ((1.0f64 + 0.044 * 0.044).ln() + (1.0f64 + 0.011 * 0.011).ln()).sqrt();
        let r = rcv(s, Z95);
        assert!((r.up - 0.134).abs() < 0.003, "{}", r.up);
        assert!((r.down + 0.118).abs() < 0.003, "{}", r.down);
        assert!(r.up > -r.down);
    }

    #[test]
    fn flags_only_changes_beyond_the_limit() {
        let p = |t, v| Point { t, v };
        assert!(signal(p(0, 1.0), p(1, 1.10), 0.048, "mg/dL", "Creatinine").is_none());
        let s = signal(p(0, 1.0), p(1, 1.30), 0.048, "mg/dL", "Creatinine").expect("signal");
        assert_eq!(s.severity, Severity::Alert);
        assert_eq!(
            jumps(&[p(0, 1.0), p(1, 1.3), p(2, 1.0), p(3, 1.02)], 0.048).len(),
            2
        );
    }
}
