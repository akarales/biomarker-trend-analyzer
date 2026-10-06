//! Adapter between stored observations and the pure drift engine: the only
//! place that turns `Observation`s into `SeriesPoint`s and calls `analyze`.

use biomarker_drift::{DriftReport, SeriesPoint};
use biomarker_ingest::Observation;

/// Drift report for one patient+biomarker series (sorted by `taken_at`).
/// `None` for an empty series. The unit is taken from the latest reading.
pub fn report(
    series: &[Observation],
    code: &str,
    window_days: i64,
    now: i64,
) -> Option<DriftReport> {
    let unit = &series.last()?.unit;
    let points: Vec<SeriesPoint> = series
        .iter()
        .map(|o| SeriesPoint {
            t: o.taken_at.and_utc().timestamp(),
            v: o.value,
        })
        .collect();
    Some(biomarker_drift::analyze(
        &points,
        code,
        unit,
        window_days,
        now,
    ))
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

    #[test]
    fn empty_series_has_no_report() {
        assert!(report(&[], "LDL", 90, 0).is_none());
    }

    #[test]
    fn report_uses_latest_unit_and_every_point() {
        let series = [
            obs(1, 100.0, "mg/dL"),
            obs(2, 101.0, "mg/dL"),
            obs(3, 2.6, "mmol/L"),
        ];
        let now = series[2].taken_at.and_utc().timestamp();
        let r = report(&series, "LDL", 90, now).expect("report");
        assert_eq!(r.unit, "mmol/L");
        assert_eq!(r.series_len, 3);
        assert_eq!(r.latest, Some(2.6));
    }
}
