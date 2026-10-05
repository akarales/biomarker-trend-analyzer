//! biomarker-drift — personal-baseline drift detection, pure functions.
//!
//! The core concept behind longitudinal patient monitoring:
//! compute a **personal** baseline for each
//! biomarker series (not population reference ranges), then flag drift:
//!
//! - Baseline: median + MAD over the lookback window (robust to lab
//!   outliers; MAD scaled to σ by 1.4826)
//! - z-score of the latest reading against that baseline
//! - EWMA (α = 0.3) control value with its own z
//! - Theil–Sen slope (median of pairwise slopes — robust linear trend)
//!   expressed per day
//! - Status: Normal / Watch (|z| ≥ 2) / Alert (|z| ≥ 3 or EWMA breach)
//!
//! No I/O, no clock access — callers pass timestamps (epoch seconds) and
//! "now". Everything is unit-testable without infrastructure.

use serde::{Deserialize, Serialize};

const DAY_SECONDS: i64 = 86_400;
const MAD_TO_SIGMA: f64 = 1.4826;
const EWMA_ALPHA: f64 = 0.3;
const Z_WATCH: f64 = 2.0;
const Z_ALERT: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeriesPoint {
    /// Epoch seconds when the reading was taken.
    pub t: i64,
    /// Measured value (in the biomarker's unit).
    pub v: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Rising,
    Falling,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Normal,
    Watch,
    Alert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub median: f64,
    /// MAD scaled to a σ estimate; floored so flat series stay usable.
    pub robust_std: f64,
    pub n: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriftReport {
    pub code: String,
    pub unit: String,
    pub baseline: Baseline,
    pub latest: Option<f64>,
    /// z-score of the latest value against the personal baseline.
    pub latest_z: Option<f64>,
    /// EWMA over the window and its z-score.
    pub ewma: Option<f64>,
    pub ewma_z: Option<f64>,
    /// Theil–Sen slope per day over the window.
    pub slope_per_day: Option<f64>,
    pub trend: Option<TrendDirection>,
    /// Window points with |z| ≥ 3.
    pub anomalies: Vec<SeriesPoint>,
    pub status: Status,
    pub window_days: i64,
    pub series_len: usize,
}

/// Personal baseline: median + robust σ over the lookback window.
/// Falls back to the whole (sorted) series when the window is empty.
pub fn baseline(window: &[SeriesPoint]) -> Baseline {
    let mut values: Vec<f64> = window.iter().map(|p| p.v).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n == 0 {
        return Baseline {
            median: 0.0,
            robust_std: 1.0,
            n: 0,
        };
    }
    let median = median_sorted(&values);
    let deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
    let mad = median_unsorted(&deviations);
    let robust_std = (MAD_TO_SIGMA * mad).max(1e-9);
    Baseline {
        median,
        robust_std,
        n,
    }
}

fn median_sorted(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

fn median_unsorted(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    median_sorted(&sorted)
}

fn z_score(value: f64, base: &Baseline) -> f64 {
    (value - base.median) / base.robust_std
}

/// Exponentially weighted moving average over the window.
fn ewma(window: &[SeriesPoint]) -> Option<f64> {
    let mut acc: Option<f64> = None;
    for point in window {
        acc = Some(match acc {
            None => point.v,
            Some(prev) => EWMA_ALPHA * point.v + (1.0 - EWMA_ALPHA) * prev,
        });
    }
    acc
}

/// Theil–Sen slope per day: median of pairwise (Δv / Δt) over the window.
/// Robust to outliers; O(n²) is fine for demo-scale windows (≤ a few
/// hundred points).
pub fn theil_sen_slope_per_day(window: &[SeriesPoint]) -> Option<f64> {
    let mut slopes: Vec<f64> = Vec::new();
    for (i, a) in window.iter().enumerate() {
        for b in window.iter().skip(i + 1) {
            let dt_days = (b.t - a.t) as f64 / DAY_SECONDS as f64;
            if dt_days.abs() < 1e-9 {
                continue;
            }
            let slope = (b.v - a.v) / dt_days;
            if slope.is_finite() {
                slopes.push(slope);
            }
        }
    }
    if slopes.is_empty() {
        return None;
    }
    Some(median_unsorted(&slopes))
}

fn trend_from_slope(slope_per_day: f64, median: f64) -> TrendDirection {
    // Relative slope threshold: 10% of the baseline per year.
    let per_year = slope_per_day * 365.0;
    if median.abs() < 1e-9 {
        return if per_year.abs() < 1e-9 {
            TrendDirection::Flat
        } else if per_year > 0.0 {
            TrendDirection::Rising
        } else {
            TrendDirection::Falling
        };
    }
    let relative = per_year.abs() / median.abs();
    if relative < 0.10 {
        TrendDirection::Flat
    } else if per_year > 0.0 {
        TrendDirection::Rising
    } else {
        TrendDirection::Falling
    }
}

/// Analyze one patient-biomarker series.
/// `points` may be unordered; they are sorted by time internally.
/// `code`/`unit` are passed through for API shaping.
pub fn analyze(
    points: &[SeriesPoint],
    code: &str,
    unit: &str,
    window_days: i64,
    now: i64,
) -> DriftReport {
    let mut sorted: Vec<SeriesPoint> = points.to_vec();
    sorted.sort_by_key(|p| p.t);

    let window_start = now - window_days * DAY_SECONDS;
    let window: Vec<SeriesPoint> = sorted
        .iter()
        .filter(|p| p.t >= window_start)
        .copied()
        .collect();
    // Fall back to the whole series when the window caught nothing.
    let window: Vec<SeriesPoint> = if window.is_empty() {
        sorted.clone()
    } else {
        window
    };
    let base = baseline(&window);

    let latest = sorted.last().map(|p| p.v);
    let latest_z = latest.map(|v| z_score(v, &base));
    let ewma_value = ewma(&window);
    let ewma_z = ewma_value.map(|v| z_score(v, &base));
    let slope = theil_sen_slope_per_day(&window);
    let trend = slope.map(|s| trend_from_slope(s, base.median));

    let anomalies: Vec<SeriesPoint> = window
        .iter()
        .filter(|p| z_score(p.v, &base).abs() >= Z_ALERT)
        .copied()
        .collect();

    let status = match (latest_z, ewma_z) {
        (Some(z), _) if z.abs() >= Z_ALERT => Status::Alert,
        (Some(z), Some(ez)) if z.abs() >= Z_WATCH || ez.abs() >= Z_WATCH => Status::Watch,
        (Some(z), None) if z.abs() >= Z_WATCH => Status::Watch,
        _ if !anomalies.is_empty() => Status::Watch,
        _ => Status::Normal,
    };

    DriftReport {
        code: code.to_string(),
        unit: unit.to_string(),
        baseline: base,
        latest,
        latest_z,
        ewma: ewma_value,
        ewma_z,
        slope_per_day: slope,
        trend,
        anomalies,
        status,
        window_days,
        series_len: sorted.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(values: &[(i64, f64)]) -> Vec<SeriesPoint> {
        values.iter().map(|&(t, v)| SeriesPoint { t, v }).collect()
    }

    fn flat(n: i64, value: f64) -> Vec<(i64, f64)> {
        (0..n)
            .map(|i| (i * DAY_SECONDS, value + ((i * 7) % 3) as f64 * 1e-6))
            .collect()
    }

    #[test]
    fn baseline_is_robust_to_outliers() {
        let mut values = flat(30, 100.0);
        values.push((30 * DAY_SECONDS, 400.0)); // gross outlier
        let report = analyze(&series(&values), "LDL", "mg/dL", 365, 31 * DAY_SECONDS);
        assert_eq!(report.baseline.n, 31);
        assert!(
            (report.baseline.median - 100.0).abs() < 0.01,
            "median ignores the outlier"
        );
        assert!(report.latest_z.unwrap() > Z_ALERT, "outlier z must be huge");
        assert_eq!(report.status, Status::Alert);
    }

    #[test]
    fn stable_series_is_normal() {
        let report = analyze(&series(&flat(40, 5.0)), "HBA1C", "%", 365, 40 * DAY_SECONDS);
        assert_eq!(report.status, Status::Normal);
        assert!(report.anomalies.is_empty());
        assert_eq!(report.trend, Some(TrendDirection::Flat));
        assert!((report.ewma.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn step_up_drift_is_flagged() {
        let mut values = flat(30, 6.0);
        for i in 30..34 {
            values.push((i * DAY_SECONDS, 6.9)); // sustained step up
        }
        let report = analyze(&series(&values), "HBA1C", "%", 365, 34 * DAY_SECONDS);
        assert!(report.latest_z.unwrap() >= Z_ALERT);
        assert_eq!(report.status, Status::Alert);
        // Deliberately NOT asserting a Rising trend here: Theil–Sen is
        // robust to steps by design (the median pairwise slope stays flat),
        // so step changes are caught by the z-score/EWMA detectors, while
        // gradual drift is caught by the slope. Two detectors, two shapes.
    }

    #[test]
    fn gradual_rise_flags_trend_not_alert() {
        // +0.1 per day from 50 → over 60 days = +6 (12%/yr of 50)
        let values: Vec<(i64, f64)> = (0..60)
            .map(|i| (i * DAY_SECONDS, 50.0 + 0.1 * i as f64))
            .collect();
        let report = analyze(&series(&values), "LDL", "mg/dL", 365, 60 * DAY_SECONDS);
        assert_eq!(report.trend, Some(TrendDirection::Rising));
        assert!((report.slope_per_day.unwrap() - 0.1).abs() < 0.02);
        // A gradual consistent rise tracked within the window stays watchful, not alerting
        assert_ne!(report.status, Status::Alert);
    }

    #[test]
    fn falling_trend_detected() {
        let values: Vec<(i64, f64)> = (0..40)
            .map(|i| (i * DAY_SECONDS, 10.0 - 0.2 * i as f64))
            .collect();
        let report = analyze(&series(&values), "TSH", "mIU/L", 365, 40 * DAY_SECONDS);
        assert_eq!(report.trend, Some(TrendDirection::Falling));
    }

    #[test]
    fn theil_sen_ignores_spike() {
        let mut values: Vec<(i64, f64)> = (0..20)
            .map(|i| (i * DAY_SECONDS, 20.0 + 0.05 * i as f64))
            .collect();
        values.push((20 * DAY_SECONDS, 900.0));
        let slope = theil_sen_slope_per_day(&series(&values)).unwrap();
        assert!(
            (slope - 0.05).abs() < 0.02,
            "median-of-pairs rejects the spike, got {slope}"
        );
    }

    #[test]
    fn empty_series_is_normal_not_panicking() {
        let report = analyze(&[], "LDL", "mg/dL", 90, 0);
        assert_eq!(report.series_len, 0);
        assert_eq!(report.status, Status::Normal);
        assert!(report.latest.is_none());
    }

    #[test]
    fn unordered_input_is_sorted() {
        let values = [(20 * DAY_SECONDS, 7.0), (0, 5.0), (10 * DAY_SECONDS, 6.0)];
        let report = analyze(&series(&values), "HBA1C", "%", 365, 20 * DAY_SECONDS);
        assert_eq!(report.latest, Some(7.0));
    }
}
