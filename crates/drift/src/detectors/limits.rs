//! Population context: clinical decision thresholds and the population
//! reference interval. "Personal drift" and "clinical category" are
//! different questions (v1 had only the first — defect D9).

use crate::fmt;
use crate::model::{Point, Rule, Severity, Signal};
use crate::profiles::{AnalyteProfile, Direction};

/// Most severe threshold crossed by the latest result, per direction.
pub fn threshold_signals(profile: &AnalyteProfile, latest: Point, unit: &str) -> Vec<Signal> {
    [Direction::Above, Direction::Below]
        .into_iter()
        .filter_map(|dir| {
            profile
                .thresholds
                .iter()
                .filter(|t| t.direction == dir)
                .filter(|t| match dir {
                    Direction::Above => latest.v >= t.value,
                    Direction::Below => latest.v < t.value,
                })
                .max_by_key(|t| t.severity)
        })
        .map(|t| Signal {
            rule: Rule::Threshold,
            severity: t.severity,
            t: latest.t,
            value: latest.v,
            threshold: Some(t.value),
            explanation: format!(
                "{} {} {unit} on {}: {}.",
                profile.display,
                fmt::value(latest.v),
                fmt::date(latest.t),
                t.label,
            ),
            source: t.source.into(),
        })
        .collect()
}

/// Outside the population interval — context (`info`): the clinical weight
/// of a result comes from the cited thresholds, and the interval is only
/// mentioned when no threshold already speaks for that side.
pub fn population_signal(
    profile: &AnalyteProfile,
    latest: Point,
    unit: &str,
    thresholds: &[Signal],
) -> Option<Signal> {
    let range = profile.population.as_ref()?;
    let (side, limit, dir) = if latest.v > range.high {
        ("above", range.high, Direction::Above)
    } else if latest.v < range.low {
        ("below", range.low, Direction::Below)
    } else {
        return None;
    };
    // a threshold signal is on the "above" side exactly when value ≥ limit
    let covered = thresholds.iter().any(|s| {
        s.threshold
            .is_some_and(|limit| (s.value >= limit) == (dir == Direction::Above))
    });
    if covered {
        return None;
    }
    Some(Signal {
        rule: Rule::Population,
        severity: Severity::Info,
        t: latest.t,
        value: latest.v,
        threshold: Some(limit),
        explanation: format!(
            "{} {} {unit} is {side} the population reference interval {}–{} {unit}.",
            profile.display,
            fmt::value(latest.v),
            fmt::value(range.low),
            fmt::value(range.high),
        ),
        source: range.source.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::lookup;

    fn at(v: f64) -> Point {
        Point { t: 0, v }
    }

    #[test]
    fn hba1c_picks_the_most_severe_band() {
        let p = lookup("HBA1C").expect("profile");
        let s = threshold_signals(p, at(7.0), "%");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].severity, Severity::Alert);
        assert_eq!(
            threshold_signals(p, at(6.0), "%")[0].severity,
            Severity::Watch
        );
        assert!(threshold_signals(p, at(5.5), "%").is_empty());
    }

    #[test]
    fn population_defers_to_a_threshold_on_the_same_side() {
        let p = lookup("HBA1C").expect("profile");
        let t = threshold_signals(p, at(6.0), "%");
        assert!(population_signal(p, at(6.0), "%", &t).is_none());
        let tsh = lookup("TSH").expect("profile");
        let sub = threshold_signals(tsh, at(6.0), "m[IU]/L");
        assert_eq!(sub[0].severity, Severity::Watch, "subclinical range");
        assert!(population_signal(tsh, at(6.0), "m[IU]/L", &sub).is_none());
        let creat = lookup("CREAT").expect("profile");
        let none = threshold_signals(creat, at(1.5), "mg/dL");
        assert!(none.is_empty());
        let s = population_signal(creat, at(1.5), "mg/dL", &none).expect("population");
        assert_eq!((s.rule, s.severity), (Rule::Population, Severity::Info));
        assert!(population_signal(creat, at(1.0), "mg/dL", &none).is_none());
    }
}
