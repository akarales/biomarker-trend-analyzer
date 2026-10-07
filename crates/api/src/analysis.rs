//! Adapter between stored observations and the pure drift engine: the only
//! place that turns `Observation`s into `Reading`s and calls `analyze`,
//! plus parsing of the analysis query parameters.

use biomarker_drift::{AnalysisInput, DriftReport, Reading};
use biomarker_ingest::Observation;
use chrono::{DateTime, NaiveDate};
use serde::Deserialize;

use crate::error::ApiError;

pub const MIN_WINDOW_DAYS: i64 = 7;
pub const MAX_WINDOW_DAYS: i64 = 3_650;

/// `?as_of=YYYY-MM-DD|RFC 3339&window_days=N` on summary and series.
/// Strings so malformed values get our `{error, code}` body, not axum's
/// plain-text rejection.
#[derive(Debug, Default, Deserialize)]
pub struct AnalysisParams {
    pub as_of: Option<String>,
    pub window_days: Option<String>,
}

/// Validated analysis options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// epoch seconds; None = the latest result of each series
    pub as_of: Option<i64>,
    pub window_days: i64,
}

impl AnalysisParams {
    pub fn options(&self, default_window: i64) -> Result<Options, ApiError> {
        let window_days = match self.window_days.as_deref() {
            None | Some("") => default_window,
            Some(raw) => raw.trim().parse().unwrap_or(-1),
        };
        if !(MIN_WINDOW_DAYS..=MAX_WINDOW_DAYS).contains(&window_days) {
            return Err(ApiError::BadRequest(format!(
                "window_days must be between {MIN_WINDOW_DAYS} and {MAX_WINDOW_DAYS}"
            )));
        }
        let as_of = self
            .as_of
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(parse_as_of)
            .transpose()?;
        Ok(Options { as_of, window_days })
    }
}

/// A date means "end of that day (UTC)", so results taken that day count.
fn parse_as_of(raw: &str) -> Result<i64, ApiError> {
    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        let end = date.and_hms_opt(23, 59, 59).expect("valid time of day");
        return Ok(end.and_utc().timestamp());
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.timestamp())
        .map_err(|_| {
            ApiError::BadRequest("as_of must be YYYY-MM-DD or an RFC 3339 timestamp".into())
        })
}

/// Drift report for one patient+biomarker series. `None` for an empty series.
pub fn report(series: &[Observation], code: &str, options: Options) -> Option<DriftReport> {
    if series.is_empty() {
        return None;
    }
    let readings: Vec<Reading> = series
        .iter()
        .map(|o| Reading {
            t: o.taken_at.and_utc().timestamp(),
            value: o.value,
            unit: o.unit.clone(),
        })
        .collect();
    Some(biomarker_drift::analyze(&AnalysisInput {
        code,
        readings: &readings,
        as_of: options.as_of,
        trend_window_days: options.window_days,
    }))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn obs(day: u32, value: f64, unit: &str) -> Observation {
        Observation {
            patient_id: "p".into(),
            code: "LDL".into(),
            value,
            unit: unit.into(),
            taken_at: NaiveDate::from_ymd_opt(2026, 1, day)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .expect("valid date"),
            source: "test".into(),
        }
    }

    const ALL: Options = Options {
        as_of: None,
        window_days: 365,
    };

    #[test]
    fn empty_series_has_no_report() {
        assert!(report(&[], "LDL", ALL).is_none());
    }

    #[test]
    fn units_are_normalised_by_the_engine() {
        let series = [
            obs(1, 100.0, "mg/dL"),
            obs(2, 101.0, "mg/dL"),
            obs(3, 2.6, "mmol/L"),
        ];
        let r = report(&series, "LDL", ALL).expect("report");
        assert_eq!(r.unit, "mg/dL");
        assert_eq!(r.points.len(), 3);
        assert!((r.latest.expect("latest").v - 100.5).abs() < 0.1);
    }

    #[test]
    fn as_of_date_includes_that_whole_day() {
        let params = AnalysisParams {
            as_of: Some("2026-01-02".into()),
            window_days: None,
        };
        let options = params.options(365).expect("valid");
        let r = report(
            &[
                obs(1, 100.0, "mg/dL"),
                obs(2, 101.0, "mg/dL"),
                obs(3, 99.0, "mg/dL"),
            ],
            "LDL",
            options,
        )
        .expect("report");
        assert_eq!(r.points.len(), 2);
        assert_eq!(r.excluded.after_as_of, 1);
    }

    #[test]
    fn invalid_params_are_rejected() {
        for w in ["1", "abc", "99999"] {
            let bad = AnalysisParams {
                as_of: None,
                window_days: Some(w.into()),
            };
            assert!(
                matches!(bad.options(365), Err(ApiError::BadRequest(_))),
                "{w}"
            );
        }
        let bad_date = AnalysisParams {
            as_of: Some("yesterday".into()),
            window_days: None,
        };
        assert!(matches!(
            bad_date.options(365),
            Err(ApiError::BadRequest(_))
        ));
        let rfc = AnalysisParams {
            as_of: Some("2026-01-02T12:00:00Z".into()),
            window_days: Some("90".into()),
        };
        assert_eq!(rfc.options(365).expect("valid").window_days, 90);
    }
}
