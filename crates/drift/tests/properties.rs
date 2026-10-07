//! Properties that must hold for any series (checked over many seeded
//! random series — no proptest dependency, the crate stays serde-only).

mod common;

use biomarker_drift::{Reading, Rule, Severity, Status, analyze};
use common::{DAY, Noise, input, weekly};

const CASES: u64 = 200;
const CODES: [(&str, &str, f64); 4] = [
    ("HBA1C", "%", 5.8),
    ("LDL", "mg/dL", 120.0),
    ("TSH", "mIU/L", 2.0),
    ("CREAT", "mg/dL", 1.0),
];

/// A random clinical-looking series: noise, sometimes a step, sometimes a trend.
fn random_series(seed: u64) -> (&'static str, Vec<Reading>) {
    let mut n = Noise::new(seed);
    let (code, unit, level) = CODES[n.below(CODES.len())];
    let len = 2 + n.below(40);
    let step_at = n.below(len + 1);
    let step = 1.0 + 0.3 * n.gauss();
    let slope = 0.01 * n.gauss();
    let values: Vec<f64> = (0..len)
        .map(|i| {
            let base = level
                * (1.0 + slope * i as f64)
                * if i >= step_at {
                    step.abs().max(0.2)
                } else {
                    1.0
                };
            n.around(base, 0.04).max(0.01)
        })
        .collect();
    (code, weekly(&values, unit))
}

#[test]
fn status_is_the_worst_signal_and_signals_are_sorted() {
    for seed in 0..CASES {
        let (code, series) = random_series(seed);
        let r = analyze(&input(code, &series, None, 365));
        let worst = r.signals.iter().map(|s| s.severity).max();
        let expected = match worst {
            Some(Severity::Alert) => Status::Alert,
            Some(Severity::Watch) => Status::Watch,
            _ => Status::Normal,
        };
        assert_eq!(r.status, expected, "seed {seed}");
        assert!(
            r.signals.windows(2).all(|w| w[0].severity >= w[1].severity),
            "seed {seed}"
        );
        // every rule is either assessed (possibly silent) or explained as not assessed
        for s in &r.signals {
            assert!(
                !r.not_assessed.iter().any(|n| n.rule == s.rule),
                "seed {seed}: {:?}",
                s.rule
            );
        }
    }
}

#[test]
fn input_order_does_not_matter() {
    for seed in 0..CASES {
        let (code, series) = random_series(seed);
        let mut shuffled = series.clone();
        let mut n = Noise::new(seed + 1_000);
        for i in (1..shuffled.len()).rev() {
            shuffled.swap(i, n.below(i + 1));
        }
        assert_eq!(
            analyze(&input(code, &series, None, 365)),
            analyze(&input(code, &shuffled, None, 365)),
            "seed {seed}"
        );
    }
}

#[test]
fn shifting_every_date_shifts_nothing_else() {
    for seed in 0..CASES {
        let (code, series) = random_series(seed);
        let moved: Vec<Reading> = series
            .iter()
            .map(|r| Reading {
                t: r.t + 1_000 * DAY,
                ..r.clone()
            })
            .collect();
        let a = analyze(&input(code, &series, None, 365));
        let b = analyze(&input(code, &moved, None, 365));
        let key = |r: &biomarker_drift::DriftReport| -> Vec<(Rule, Severity, String)> {
            r.signals
                .iter()
                .map(|s| (s.rule, s.severity, format!("{:.9}", s.value)))
                .collect()
        };
        assert_eq!(key(&a), key(&b), "seed {seed}");
        assert_eq!(a.status, b.status);
    }
}

#[test]
fn personal_detectors_are_scale_invariant() {
    // prRI, RCV, CUSUM, EWMA and the trend work on ratios: multiplying a
    // creatinine series by a constant changes only the population/threshold view
    let personal = |r: &biomarker_drift::DriftReport| -> Vec<(Rule, Severity)> {
        r.signals
            .iter()
            .filter(|s| !matches!(s.rule, Rule::Population | Rule::Threshold))
            .map(|s| (s.rule, s.severity))
            .collect()
    };
    for seed in 0..CASES {
        let (_, series) = random_series(seed);
        let unitless: Vec<Reading> = series
            .iter()
            .map(|r| Reading {
                unit: "mg/dL".into(),
                ..r.clone()
            })
            .collect();
        let scaled: Vec<Reading> = unitless
            .iter()
            .map(|r| Reading {
                value: r.value * 1.37,
                ..r.clone()
            })
            .collect();
        let a = analyze(&input("CREAT", &unitless, None, 365));
        let b = analyze(&input("CREAT", &scaled, None, 365));
        assert_eq!(personal(&a), personal(&b), "seed {seed}");
        assert_eq!(
            a.change_point.map(|c| c.t),
            b.change_point.map(|c| c.t),
            "seed {seed}"
        );
        if let (Some(x), Some(y)) = (a.trend, b.trend) {
            assert_eq!(x.direction, y.direction, "seed {seed}");
            assert!((x.tau - y.tau).abs() < 1e-12);
            assert!((x.change_per_year - y.change_per_year).abs() < 1e-9);
        }
    }
}

#[test]
fn mann_kendall_is_invariant_under_monotone_transforms() {
    for seed in 0..CASES {
        let (code, series) = random_series(seed);
        let cubed: Vec<Reading> = series
            .iter()
            .map(|r| Reading {
                value: r.value.powi(3),
                ..r.clone()
            })
            .collect();
        let (Some(a), Some(b)) = (
            analyze(&input("XYZ", &series, None, 365)).trend,
            analyze(&input("XYZ", &cubed, None, 365)).trend,
        ) else {
            continue;
        };
        assert!((a.tau - b.tau).abs() < 1e-12, "seed {seed} ({code})");
        assert!((a.p_value - b.p_value).abs() < 1e-12);
        assert_eq!(a.direction, b.direction);
    }
}
