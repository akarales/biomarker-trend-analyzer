//! Golden clinical scenarios, one per detector, plus regressions for the
//! v1 audit findings D1–D9 (docs/V2_updates/CODE_AUDIT.md in the workspace).

mod common;

use biomarker_drift::{Rule, Severity, Status, TrendDirection, analyze};
use common::{DAY, Noise, input, readings, rules, weekly};

#[test]
fn d1_step_into_the_diabetes_range_alerts_whatever_the_window() {
    // alice: HbA1c 5.6 % for 35 weeks, then 7.0 % for 13 weeks
    let mut noise = Noise::new(7);
    let values: Vec<f64> = (0..48)
        .map(|i| noise.around(if i < 35 { 5.6 } else { 7.0 }, 0.005))
        .collect();
    let series = weekly(&values, "%");
    for window in [30, 90, 365] {
        let r = analyze(&input("HBA1C", &series, None, window));
        assert_eq!(r.status, Status::Alert, "window {window}");
        let prri = r
            .signals
            .iter()
            .find(|s| s.rule == Rule::Prri)
            .expect("prRI signal");
        assert_eq!(prri.severity, Severity::Alert);
        let threshold = r
            .signals
            .iter()
            .find(|s| s.rule == Rule::Threshold)
            .expect("threshold");
        assert_eq!(threshold.severity, Severity::Alert);
        assert!(threshold.explanation.contains("diabetes range"));
        let base = r.baseline.as_ref().expect("baseline");
        assert!(
            (base.set_point - 5.6).abs() < 0.05,
            "baseline is the earlier steady state"
        );
        let cp = r.change_point.expect("change point");
        assert_eq!(cp.t, series[35].t, "change point at the first 7.0 % result");
        assert!(rules(&r).contains(&Rule::Shift));
    }
}

#[test]
fn d2_results_after_as_of_are_excluded_and_as_of_is_explicit() {
    let values: Vec<f64> = (0..20).map(|i| 1.0 + 0.001 * f64::from(i % 3)).collect();
    let series = weekly(&values, "mg/dL");
    let as_of = series[11].t;
    let r = analyze(&input("CREAT", &series, Some(as_of), 365));
    assert_eq!(r.excluded.after_as_of, 8);
    assert_eq!(r.points.len(), 12);
    assert_eq!(r.latest.map(|p| p.t), Some(as_of));
    // same report as analysing the truncated series (apart from the count)
    let mut truncated = analyze(&input("CREAT", &series[..12], None, 365));
    truncated.excluded.after_as_of = 8;
    truncated.as_of = Some(as_of);
    assert_eq!(r, truncated);
    // as_of before every result: nothing to assess, no fallback to the whole series
    let empty = analyze(&input("CREAT", &series, Some(series[0].t - DAY), 365));
    assert!(empty.points.is_empty());
    assert_eq!(empty.status, Status::Normal);
    assert_eq!(empty.not_assessed.len(), 7);
}

#[test]
fn d3_small_sustained_shift_is_caught_by_ewma_not_prri() {
    // +1.3 σ shift: no single result is outside the prRI, the EWMA sees it
    let profile = biomarker_drift::lookup("LDL").expect("profile");
    let shift = (1.3 * profile.sigma_log()).exp();
    let mut values = vec![110.0; 10];
    values.extend(std::iter::repeat_n(110.0 * shift, 12));
    let r = analyze(&input("LDL", &weekly(&values, "mg/dL"), None, 90));
    let fired = rules(&r);
    assert!(fired.contains(&Rule::Ewma), "{fired:?}");
    assert!(!fired.contains(&Rule::Prri));
    assert!(r.change_point.is_some(), "CUSUM also locates the change");
    assert_eq!(r.status, Status::Watch);
}

#[test]
fn d4_creatinine_noise_within_biological_variation_is_normal() {
    // bob: creatinine ≈ 0.96 mg/dL with 3 % noise (below CVI 4.4 %)
    let mut noise = Noise::new(42);
    let values: Vec<f64> = (0..48).map(|_| noise.around(0.96, 0.03)).collect();
    let r = analyze(&input("CREAT", &weekly(&values, "mg/dL"), None, 365));
    assert_eq!(r.status, Status::Normal, "{:?}", r.signals);
    assert!(r.signals.is_empty());
}

#[test]
fn d5_sparse_history_says_what_it_could_not_assess() {
    let r = analyze(&input("HBA1C", &weekly(&[5.5, 5.6], "%"), None, 365));
    assert_eq!(r.status, Status::Normal);
    assert!(r.baseline.is_none());
    let prri = r
        .not_assessed
        .iter()
        .find(|n| n.rule == Rule::Prri)
        .expect("prri not assessed");
    assert!(
        prri.reason.contains("≥ 3 earlier results") && prri.reason.contains("(has 1)"),
        "{}",
        prri.reason
    );
    assert!(r.not_assessed.iter().any(|n| n.rule == Rule::Trend));
    // RCV still works with two results
    assert!(!r.not_assessed.iter().any(|n| n.rule == Rule::Rcv));
}

#[test]
fn d6_noise_is_not_a_trend_but_a_real_rise_is() {
    let mut noise = Noise::new(3);
    let flat: Vec<f64> = (0..48).map(|_| noise.around(2.0, 0.10)).collect();
    let r = analyze(&input("TSH", &weekly(&flat, "mIU/L"), None, 365));
    let trend = r.trend.expect("trend");
    assert_eq!(
        trend.direction,
        TrendDirection::Flat,
        "p = {}",
        trend.p_value
    );
    assert!(!rules(&r).contains(&Rule::Trend));

    // bob: LDL 107 → 127 mg/dL over a year
    let mut noise = Noise::new(5);
    let rise: Vec<f64> = (0..48)
        .map(|i| noise.around(107.0 + 0.41 * f64::from(i), 0.02))
        .collect();
    let r = analyze(&input("LDL", &weekly(&rise, "mg/dL"), None, 365));
    let trend = r.trend.expect("trend");
    assert_eq!(trend.direction, TrendDirection::Rising);
    assert!(trend.ci_low_per_day > 0.0);
    // inside the prRI and no single jump: the trend and the smoothed level
    // (EWMA) carry it, both as watch
    let mut fired = rules(&r);
    fired.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(fired, vec![Rule::Ewma, Rule::Trend]);
    assert_eq!(r.status, Status::Watch);
}

#[test]
fn d7_every_signal_is_explained_and_sourced() {
    let mut values = vec![1.0; 12];
    values.extend([1.6, 1.62, 1.65]);
    let r = analyze(&input("CREAT", &weekly(&values, "mg/dL"), None, 365));
    assert!(!r.signals.is_empty());
    for s in &r.signals {
        assert!(s.explanation.len() > 40, "{s:?}");
        assert!(!s.source.is_empty());
        assert!(s.threshold.is_some());
    }
    assert!(
        r.signals.windows(2).all(|w| w[0].severity >= w[1].severity),
        "worst first"
    );
}

#[test]
fn d8_units_are_normalised_and_unknown_units_excluded() {
    // HbA1c reported in % then IFCC mmol/mol at the same level: no jump
    // (35.3 mmol/mol = 5.38 % by the IFCC→NGSP master equation)
    let mut series = weekly(&[5.4; 6], "%");
    series.extend(readings(&[(6, 35.3), (7, 35.3), (8, 35.3)], "mmol/mol"));
    series.extend(readings(&[(9, 5.4)], "mg/dL"));
    let r = analyze(&input("4548-4", &series, None, 365));
    assert_eq!(r.unit, "%");
    assert_eq!(r.excluded.unit_unknown, 1);
    assert_eq!(r.points.len(), 9);
    assert!(
        r.points.iter().all(|p| (p.v - 5.4).abs() < 0.05),
        "{:?}",
        r.points
    );
    assert!(r.rcv_jumps.is_empty());
    assert_eq!(r.status, Status::Normal);
}

#[test]
fn d9_population_context_beside_personal_baseline() {
    // TSH steady at 5.8 mIU/L: personally unremarkable, clinically in the
    // subclinical-hypothyroidism range (cited threshold, watch)
    let r = analyze(&input("TSH", &weekly(&[5.8; 12], "mIU/L"), None, 365));
    assert_eq!(rules(&r), vec![Rule::Threshold]);
    assert!(
        r.signals[0]
            .explanation
            .contains("subclinical hypothyroidism")
    );
    assert_eq!(r.population.as_ref().map(|p| p.high), Some(4.5));
    assert_eq!(r.status, Status::Watch);

    // creatinine steady at 1.45 mg/dL: no cited threshold without eGFR, so
    // the population interval is shown as context (info) without escalating
    let r = analyze(&input("CREAT", &weekly(&[1.45; 12], "mg/dL"), None, 365));
    assert_eq!(rules(&r), vec![Rule::Population]);
    assert_eq!(r.signals[0].severity, Severity::Info);
    assert_eq!(r.status, Status::Normal);
    assert!(r.not_assessed.iter().any(|n| n.rule == Rule::Threshold));
}

#[test]
fn erroneous_outlier_flags_rcv_then_reverts_without_a_shift() {
    let mut values = vec![1.0; 12];
    values.push(1.6);
    let series = weekly(&values, "mg/dL");
    let at_spike = analyze(&input("CREAT", &series, None, 365));
    assert!(rules(&at_spike).contains(&Rule::Rcv));
    assert!(rules(&at_spike).contains(&Rule::Prri));
    assert_eq!(at_spike.status, Status::Alert);

    values.extend([1.0, 1.01, 0.99]);
    let after = analyze(&input("CREAT", &weekly(&values, "mg/dL"), None, 365));
    assert!(after.change_point.is_none(), "one result is not a shift");
    assert!(
        !rules(&after)
            .iter()
            .any(|r| matches!(r, Rule::Prri | Rule::Shift | Rule::Rcv))
    );
    assert_eq!(
        after.rcv_jumps.len(),
        2,
        "spike up and back down stay visible on the chart"
    );
}

#[test]
fn statin_start_lowers_ldl_falling_trend_and_shift() {
    let mut values = vec![165.0; 8];
    values.extend((1..=10).map(|i| 165.0 - 6.0 * f64::from(i)));
    let r = analyze(&input("LDL", &weekly(&values, "mg/dL"), None, 365));
    assert_eq!(r.trend.map(|t| t.direction), Some(TrendDirection::Falling));
    let prri = r
        .signals
        .iter()
        .find(|s| s.rule == Rule::Prri)
        .expect("below prRI");
    assert!(prri.explanation.contains("below"));
    assert!(
        !rules(&r).contains(&Rule::Threshold),
        "105 mg/dL crosses no threshold"
    );
}

#[test]
fn unknown_analyte_runs_only_the_trend() {
    let values: Vec<f64> = (0..10).map(|i| 50.0 + f64::from(i)).collect();
    let r = analyze(&input("XYZ", &weekly(&values, "U/L"), None, 365));
    assert!(r.analyte.is_none());
    assert_eq!(r.unit, "U/L");
    assert_eq!(rules(&r), vec![Rule::Trend]);
    assert_eq!(r.not_assessed.len(), 6);
}

#[test]
fn empty_series_does_not_panic() {
    let r = analyze(&input("LDL", &[], None, 90));
    assert!(r.latest.is_none() && r.points.is_empty());
    assert_eq!(r.status, Status::Normal);
}
