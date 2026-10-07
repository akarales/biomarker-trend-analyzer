//! Estimated GFR from serum creatinine: the race-free 2021 CKD-EPI
//! creatinine equation (Inker LA et al., N Engl J Med 2021;385:1737–1749;
//! NIDDK "eGFR equations for adults"), for adults ≥ 18 years:
//!
//! `eGFR = 142 × min(Scr/κ, 1)^α × max(Scr/κ, 1)^−1.200 × 0.9938^age × 1.012 [female]`
//! with κ = 0.7 (female) / 0.9 (male), α = −0.241 (female) / −0.302 (male),
//! Scr in mg/dL (IDMS-traceable), result in mL/min/1.73 m².
//!
//! Pure: the caller supplies sex and birth year (from the FHIR Patient). Age
//! at each result is `result year − birth year`: the demo stores the birth
//! YEAR only, so age is exact to ±1 year (≤ 0.6 % on eGFR).

use serde::{Deserialize, Serialize};

use crate::fmt;
use crate::model::Reading;
use crate::profiles;

pub const EGFR_CODE: &str = "EGFR";
pub const EGFR_UNIT: &str = "mL/min/{1.73_m2}";
pub const METHOD: &str = "derived from serum creatinine with the race-free 2021 CKD-EPI creatinine equation \
(Inker LA et al., N Engl J Med 2021;385:1737–1749); age = result year − birth year (±1 year); adults ≥ 18 only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sex {
    Female,
    Male,
}

/// The equation itself (Scr in mg/dL, age in years).
pub fn ckd_epi_2021(scr_mg_dl: f64, age_years: f64, sex: Sex) -> f64 {
    let (kappa, alpha, female) = match sex {
        Sex::Female => (0.7, -0.241, 1.012),
        Sex::Male => (0.9, -0.302, 1.0),
    };
    let ratio = scr_mg_dl / kappa;
    142.0
        * ratio.min(1.0).powf(alpha)
        * ratio.max(1.0).powf(-1.200)
        * 0.9938_f64.powf(age_years)
        * female
}

fn year(epoch_seconds: i64) -> i32 {
    fmt::date(epoch_seconds)[..4].parse().unwrap_or(0)
}

/// Why no eGFR series could be derived (shown as "not assessed").
#[derive(Debug, Clone, PartialEq)]
pub enum NotDerived {
    NoDemographics,
    SexNotBinary,
    NoAdultResults,
}

impl NotDerived {
    pub fn reason(&self) -> &'static str {
        match self {
            NotDerived::NoDemographics => {
                "eGFR needs the patient's sex and birth year (none recorded, e.g. CSV-only data)"
            }
            NotDerived::SexNotBinary => {
                "eGFR (CKD-EPI 2021) needs sex recorded as female or male; it is not estimated otherwise"
            }
            NotDerived::NoAdultResults => {
                "eGFR (CKD-EPI 2021) is defined for adults ≥ 18 years only"
            }
        }
    }
}

/// eGFR readings for every creatinine reading in a known unit taken at
/// age ≥ 18 (creatinine is normalised to mg/dL first).
pub fn derive(
    creatinine: &[Reading],
    sex: Option<Sex>,
    birth_year: Option<i32>,
) -> Result<Vec<Reading>, NotDerived> {
    let birth_year = birth_year.ok_or(NotDerived::NoDemographics)?;
    let sex = sex.ok_or(NotDerived::SexNotBinary)?;
    let creat = profiles::lookup("CREAT").expect("creatinine profile exists");
    let out: Vec<Reading> = creatinine
        .iter()
        .filter_map(|r| {
            let age = year(r.t) - birth_year;
            let scr = creat.to_canonical(r.value, &r.unit)?;
            (age >= 18 && scr > 0.0).then(|| Reading {
                t: r.t,
                value: ckd_epi_2021(scr, f64::from(age), sex),
                unit: EGFR_UNIT.to_string(),
            })
        })
        .collect();
    if out.is_empty() {
        Err(NotDerived::NoAdultResults)
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_published_worked_examples() {
        // 62-year-old woman, Scr 1.3 mg/dL → 46.5 (G3a)
        assert!((ckd_epi_2021(1.3, 62.0, Sex::Female) - 46.49).abs() < 0.05);
        // below the kink the α term applies: woman 40 y, Scr 0.6
        let low = ckd_epi_2021(0.6, 40.0, Sex::Female);
        let expected = 142.0 * (0.6_f64 / 0.7).powf(-0.241) * 0.9938_f64.powf(40.0) * 1.012;
        assert!((low - expected).abs() < 1e-9);
        // a man at κ: 142 × 0.9938^50
        assert!((ckd_epi_2021(0.9, 50.0, Sex::Male) - 142.0 * 0.9938_f64.powf(50.0)).abs() < 1e-9);
        // monotone: higher creatinine → lower eGFR
        assert!(ckd_epi_2021(2.0, 60.0, Sex::Male) < ckd_epi_2021(1.2, 60.0, Sex::Male));
    }

    #[test]
    fn derives_per_result_with_unit_normalisation_and_adult_only() {
        let t2026 = 1_767_225_600; // 2026-01-01
        let readings = vec![
            Reading {
                t: t2026,
                value: 1.3,
                unit: "mg/dL".into(),
            },
            Reading {
                t: t2026 + 86_400,
                value: 1.3 * 88.42,
                unit: "umol/L".into(),
            },
            Reading {
                t: t2026 + 2 * 86_400,
                value: 1.0,
                unit: "g/L".into(),
            },
        ];
        let out = derive(&readings, Some(Sex::Female), Some(1964)).expect("derived");
        assert_eq!(out.len(), 2, "unknown unit skipped");
        assert!((out[0].value - 46.49).abs() < 0.05);
        assert!(
            (out[1].value - out[0].value).abs() < 1e-6,
            "µmol/L normalised"
        );
        assert_eq!(out[0].unit, EGFR_UNIT);
        assert_eq!(
            derive(&readings, Some(Sex::Male), Some(2010)),
            Err(NotDerived::NoAdultResults)
        );
        assert_eq!(
            derive(&readings, None, Some(1964)),
            Err(NotDerived::SexNotBinary)
        );
        assert_eq!(
            derive(&readings, Some(Sex::Male), None),
            Err(NotDerived::NoDemographics)
        );
    }
}
