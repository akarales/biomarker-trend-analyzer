//! Input and output types of the drift engine (the report is the API's
//! wire format — mirrored by `frontend/src/api/schemas.ts`).

use serde::{Deserialize, Serialize};

use crate::profiles::{PopulationRange, Threshold};

/// One lab result as stored (any unit the profile can convert).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Reading {
    /// epoch seconds when the specimen was taken
    pub t: i64,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisInput<'a> {
    /// short code (`HBA1C`) or LOINC (`4548-4`)
    pub code: &'a str,
    pub readings: &'a [Reading],
    /// evaluate as of this instant (epoch seconds); later readings are
    /// excluded. `None` = the latest reading. Never the wall clock.
    pub as_of: Option<i64>,
    /// lookback for the trend detector (days before `as_of`)
    pub trend_window_days: i64,
}

/// A normalised point (canonical unit).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub t: i64,
    pub v: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Watch,
    Alert,
}

/// Worst signal wins; `Info` signals leave the status `Normal`.
/// "normal" means *no rule fired*, not "healthy" — see `not_assessed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Normal,
    Watch,
    Alert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rule {
    /// latest result outside the personalised reference interval
    Prri,
    /// change between the last two results exceeds the reference change value
    Rcv,
    /// sustained level shift (CUSUM) with an estimated change point
    Shift,
    /// EWMA outside its control limits
    Ewma,
    /// statistically significant, clinically sized monotone trend
    Trend,
    /// clinical decision threshold crossed
    Threshold,
    /// outside the population reference interval
    Population,
}

/// One explainable finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Signal {
    pub rule: Rule,
    pub severity: Severity,
    /// time of the observation the signal is about
    pub t: i64,
    pub value: f64,
    /// the limit that was crossed (canonical unit or fraction for rcv)
    pub threshold: Option<f64>,
    pub explanation: String,
    /// where the method or limit comes from
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NotAssessed {
    pub rule: Rule,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalyteRef {
    pub code: &'static str,
    pub loinc: &'static str,
    pub display: &'static str,
    pub cvi: f64,
    pub cva: f64,
    pub cvi_source: &'static str,
    pub cva_source: &'static str,
    pub reviewed: &'static str,
}

/// Personal set point + personalised reference interval (prRI).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Baseline {
    /// results the set point was estimated from (earliest steady state)
    pub n: usize,
    pub from: i64,
    pub to: i64,
    /// geometric mean of the baseline results (homeostatic set point)
    pub set_point: f64,
    pub prri_low: f64,
    pub prri_high: f64,
    /// coverage of the interval (0.95)
    pub level: f64,
}

/// Reference change value as fractions (asymmetric, log-normal, 95 %).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rcv {
    pub up: f64,
    pub down: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Jump {
    pub from_t: i64,
    pub to_t: i64,
    /// relative change (fraction)
    pub change: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Ewma {
    pub value: f64,
    pub low: f64,
    pub high: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ChangePoint {
    /// first observation of the shifted run
    pub t: i64,
    /// CUSUM crossed its decision limit here
    pub detected_t: i64,
    pub before: f64,
    pub after: f64,
    pub change: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Rising,
    Falling,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Trend {
    pub n: usize,
    pub from: i64,
    pub slope_per_day: f64,
    pub ci_low_per_day: f64,
    pub ci_high_per_day: f64,
    /// Sen slope relative to the window median, per year (fraction)
    pub change_per_year: f64,
    pub tau: f64,
    pub p_value: f64,
    /// `Flat` unless Mann–Kendall p < 0.05
    pub direction: TrendDirection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Excluded {
    /// readings after `as_of`
    pub after_as_of: usize,
    /// readings in a unit the profile cannot convert
    pub unit_unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DriftReport {
    pub code: String,
    pub analyte: Option<AnalyteRef>,
    /// canonical unit of `points` and every value below
    pub unit: String,
    pub as_of: Option<i64>,
    pub window_days: i64,
    /// the analysed, normalised series (sorted, ≤ as_of)
    pub points: Vec<Point>,
    pub excluded: Excluded,
    pub latest: Option<Point>,
    pub baseline: Option<Baseline>,
    pub population: Option<PopulationRange>,
    pub thresholds: Vec<Threshold>,
    pub rcv: Option<Rcv>,
    /// consecutive changes beyond the RCV (chart markers)
    pub rcv_jumps: Vec<Jump>,
    pub ewma: Option<Ewma>,
    pub change_point: Option<ChangePoint>,
    pub trend: Option<Trend>,
    pub signals: Vec<Signal>,
    pub not_assessed: Vec<NotAssessed>,
    pub status: Status,
}
