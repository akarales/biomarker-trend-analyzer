//! Analyte profiles: identity (LOINC, UCUM), biological variation (CVI),
//! assumed analytical imprecision (CVA), population reference interval and
//! clinical decision thresholds — each value with its source.
//!
//! REVIEW STATUS: compiled 2026-10-06 by the developer from the cited
//! publications/guidelines. Not clinically validated. CVA values are
//! ASSUMPTIONS for a typical modern analyser — a deployment must replace
//! them with its own laboratory's imprecision. Change values only together
//! with their `source` and the `REVIEWED` date.

use serde::Serialize;

use crate::model::Severity;

pub const REVIEWED: &str = "2026-10-06";

const CVA_ASSUMED: &str =
    "assumed laboratory imprecision (typical modern analyser) — replace with the local CVA";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Above,
    Below,
}

/// A clinical decision limit on the canonical unit (`≥ value` for Above,
/// `< value` for Below).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Threshold {
    pub value: f64,
    pub direction: Direction,
    pub severity: Severity,
    pub label: &'static str,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PopulationRange {
    pub low: f64,
    pub high: f64,
    pub source: &'static str,
}

/// `canonical = value · factor + offset` for readings in `unit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    pub unit: &'static str,
    pub factor: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyteProfile {
    /// short code used by CSV uploads
    pub code: &'static str,
    /// LOINC codes accepted for this analyte (first = preferred)
    pub loinc: &'static [&'static str],
    pub display: &'static str,
    /// canonical UCUM unit all readings are normalised to
    pub unit: &'static str,
    /// spellings of the canonical unit
    pub unit_aliases: &'static [&'static str],
    pub conversions: &'static [Conversion],
    /// within-subject biological variation (fraction)
    pub cvi: f64,
    pub cvi_source: &'static str,
    /// analytical imprecision (fraction)
    pub cva: f64,
    pub cva_source: &'static str,
    pub population: Option<PopulationRange>,
    pub thresholds: &'static [Threshold],
}

const ADA: &str = "American Diabetes Association, Standards of Care in Diabetes — Section 2, \
                   Diagnosis and Classification of Diabetes (HbA1c 5.7–6.4 % prediabetes, ≥ 6.5 % diabetes)";
const ATP3: &str = "NCEP ATP III, Circulation 2002;106:3143–3421 (LDL-C categories)";
const AHA2018: &str = "Grundy SM et al., 2018 AHA/ACC Cholesterol Guideline, Circulation 2019;139:e1082–e1143 \
     (LDL-C ≥ 190 mg/dL: high-intensity statin without risk estimation)";

const KDIGO: &str = "KDIGO 2024 Clinical Practice Guideline for the Evaluation and Management of CKD, \
     Kidney Int 2024;105(4S):S117–S314 (GFR categories G1–G5; CKD needs abnormality > 3 months)";

const GARBER: &str =
    "Garber JR et al., ATA/AACE hypothyroidism guideline, Thyroid 2012;22:1200–1235";
const ROSS: &str = "Ross DS et al., ATA hyperthyroidism guideline, Thyroid 2016;26:1343–1421";

pub static PROFILES: &[AnalyteProfile] = &[
    AnalyteProfile {
        code: "HBA1C",
        loinc: &["4548-4", "17856-6"],
        display: "Hemoglobin A1c",
        unit: "%",
        unit_aliases: &["%", "% hb", "%{hb}", "percent"],
        // IFCC → NGSP master equation: % = 0.09148 · mmol/mol + 2.152
        conversions: &[Conversion {
            unit: "mmol/mol",
            factor: 0.091_48,
            offset: 2.152,
        }],
        cvi: 0.012,
        cvi_source: "Critical appraisal and meta-analysis of BV studies on glycosylated albumin, glucose \
                     and HbA1c, Adv Lab Med 2020, doi:10.1515/almed-2020-0029 (CVI 1.2 %, 0.3–2.5)",
        cva: 0.015,
        cva_source: CVA_ASSUMED,
        population: Some(PopulationRange {
            low: 4.0,
            high: 5.6,
            source: ADA,
        }),
        thresholds: &[
            Threshold {
                value: 5.7,
                direction: Direction::Above,
                severity: Severity::Watch,
                label: "prediabetes range (ADA 5.7–6.4 %)",
                source: ADA,
            },
            Threshold {
                value: 6.5,
                direction: Direction::Above,
                severity: Severity::Alert,
                label: "diabetes range (ADA ≥ 6.5 %)",
                source: ADA,
            },
        ],
    },
    AnalyteProfile {
        code: "LDL",
        loinc: &["13457-7", "18262-6", "2089-1"],
        display: "LDL cholesterol",
        unit: "mg/dL",
        unit_aliases: &["mg/dl"],
        conversions: &[Conversion {
            unit: "mmol/l",
            factor: 38.67,
            offset: 0.0,
        }],
        cvi: 0.078,
        cvi_source: "EFLM Biological Variation Database, LDL cholesterol CVI 7.8 % (as tabulated by Bio-Rad, \
                     EFLM update 2025); EuBIVAS lipids: Aarsand AK et al., Clin Chem 2018;64:1380–1393",
        cva: 0.02,
        cva_source: CVA_ASSUMED,
        // LDL-C targets are risk-based; there is no single population interval
        population: None,
        thresholds: &[
            Threshold {
                value: 130.0,
                direction: Direction::Above,
                severity: Severity::Info,
                label: "borderline high (ATP III 130–159 mg/dL)",
                source: ATP3,
            },
            Threshold {
                value: 160.0,
                direction: Direction::Above,
                severity: Severity::Watch,
                label: "high (ATP III 160–189 mg/dL)",
                source: ATP3,
            },
            Threshold {
                value: 190.0,
                direction: Direction::Above,
                severity: Severity::Alert,
                label: "very high (≥ 190 mg/dL)",
                source: AHA2018,
            },
        ],
    },
    AnalyteProfile {
        code: "TSH",
        loinc: &["3016-3", "11580-8"],
        display: "Thyrotropin (TSH)",
        unit: "m[IU]/L",
        unit_aliases: &[
            "miu/l", "m[iu]/l", "uiu/ml", "µiu/ml", "μiu/ml", "u[iu]/ml", "mu/l",
        ],
        conversions: &[],
        cvi: 0.177,
        cvi_source: "EuBIVAS thyroid biomarkers: Bottani M et al., Clin Chem Lab Med 2022;60(4):523–532 (CVI 17.7 %)",
        cva: 0.03,
        cva_source: CVA_ASSUMED,
        population: Some(PopulationRange {
            low: 0.45,
            high: 4.5,
            source: "Surks MI et al., Subclinical thyroid disease, JAMA 2004;291:228–238 (0.45–4.5 mIU/L)",
        }),
        thresholds: &[
            Threshold {
                value: 4.5,
                direction: Direction::Above,
                severity: Severity::Watch,
                label: "at or above the upper reference limit (4.5 mIU/L) — subclinical hypothyroidism range if FT4 is normal",
                source: GARBER,
            },
            Threshold {
                value: 10.0,
                direction: Direction::Above,
                severity: Severity::Alert,
                label: "TSH ≥ 10 mIU/L — treatment generally recommended",
                source: GARBER,
            },
            Threshold {
                value: 0.45,
                direction: Direction::Below,
                severity: Severity::Watch,
                label: "below the reference interval (< 0.45 mIU/L) — subclinical hyperthyroidism range if FT4/FT3 are normal",
                source: ROSS,
            },
            Threshold {
                value: 0.1,
                direction: Direction::Below,
                severity: Severity::Alert,
                label: "suppressed TSH (< 0.1 mIU/L)",
                source: ROSS,
            },
        ],
    },
    AnalyteProfile {
        code: "CREAT",
        loinc: &["2160-0", "38483-4"],
        display: "Creatinine (serum/plasma)",
        unit: "mg/dL",
        unit_aliases: &["mg/dl"],
        conversions: &[Conversion {
            unit: "umol/l",
            factor: 1.0 / 88.42,
            offset: 0.0,
        }],
        cvi: 0.044,
        cvi_source: "EuBIVAS creatinine (enzymatic): Carobene A et al., Clin Chem 2017;63:1527–1536, PMID 28720681 (CVI 4.4 %)",
        cva: 0.02,
        cva_source: CVA_ASSUMED,
        population: Some(PopulationRange {
            low: 0.59,
            high: 1.35,
            source: "adult interval spanning female 0.59–1.04 and male 0.74–1.35 mg/dL (Mayo Clinic Laboratories); \
                     kidney-function staging is on the derived eGFR (KDIGO categories)",
        }),
        thresholds: &[],
    },
    // Derived from creatinine + sex + age (crate::egfr); never uploaded.
    // Thresholds are ordered mildest first: on a tie in severity the
    // deepest category crossed is reported.
    AnalyteProfile {
        code: "EGFR",
        loinc: &["98979-8"],
        display: "eGFR (CKD-EPI 2021, creatinine)",
        unit: "mL/min/{1.73_m2}",
        unit_aliases: &["mL/min/1.73m2", "mL/min/1.73 m2", "mL/min/{1.73_m2}"],
        conversions: &[],
        // ∂ln eGFR / ∂ln Scr = −1.200 above κ, so CV(eGFR) ≈ 1.2 × CV(Scr)
        cvi: 0.053,
        cvi_source: "derived: 1.2 × creatinine CVI 4.4 % (EuBIVAS, Clin Chem 2017;63:1527–1536) via the \
                     CKD-EPI 2021 exponent −1.200 above κ (an upper bound below κ)",
        cva: 0.024,
        cva_source: "derived: 1.2 × the assumed creatinine CVA 2 % — replace with the local CVA",
        population: None,
        thresholds: &[
            Threshold {
                value: 60.0,
                direction: Direction::Below,
                severity: Severity::Watch,
                label: "KDIGO G3a, mildly to moderately decreased (eGFR 45–59) — CKD if persistent > 3 months",
                source: KDIGO,
            },
            Threshold {
                value: 45.0,
                direction: Direction::Below,
                severity: Severity::Watch,
                label: "KDIGO G3b, moderately to severely decreased (eGFR 30–44)",
                source: KDIGO,
            },
            Threshold {
                value: 30.0,
                direction: Direction::Below,
                severity: Severity::Alert,
                label: "KDIGO G4, severely decreased (eGFR 15–29)",
                source: KDIGO,
            },
            Threshold {
                value: 15.0,
                direction: Direction::Below,
                severity: Severity::Alert,
                label: "KDIGO G5, kidney failure (eGFR < 15)",
                source: KDIGO,
            },
        ],
    },
];

/// Normalise a unit spelling for matching (case, spaces, micro signs).
fn unit_key(unit: &str) -> String {
    unit.trim()
        .to_lowercase()
        .replace(' ', "")
        .replace(['\u{b5}', '\u{3bc}'], "u")
}

impl AnalyteProfile {
    /// Value in the canonical unit, or None for an unknown unit.
    pub fn to_canonical(&self, value: f64, unit: &str) -> Option<f64> {
        let key = unit_key(unit);
        if key == unit_key(self.unit) || self.unit_aliases.iter().any(|a| unit_key(a) == key) {
            return Some(value);
        }
        self.conversions
            .iter()
            .find(|c| unit_key(c.unit) == key)
            .map(|c| value * c.factor + c.offset)
    }

    /// σ of ln(value) from biological + analytical variation (log-normal).
    pub fn sigma_log(&self) -> f64 {
        ((1.0 + self.cvi * self.cvi).ln() + (1.0 + self.cva * self.cva).ln()).sqrt()
    }
}

/// Profile for a short code (`HBA1C`) or a LOINC code (`4548-4`).
pub fn lookup(code: &str) -> Option<&'static AnalyteProfile> {
    let code = code.trim();
    PROFILES
        .iter()
        .find(|p| p.code.eq_ignore_ascii_case(code) || p.loinc.contains(&code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_short_code_or_loinc() {
        assert_eq!(lookup("hba1c").map(|p| p.code), Some("HBA1C"));
        assert_eq!(lookup("2160-0").map(|p| p.code), Some("CREAT"));
        assert!(lookup("ZZZ").is_none());
    }

    #[test]
    fn unit_conversions() {
        let hba1c = lookup("HBA1C").expect("profile");
        // 48 mmol/mol ≈ 6.5 %, 39 mmol/mol ≈ 5.7 % (IFCC/NGSP)
        assert!((hba1c.to_canonical(48.0, "mmol/mol").expect("conv") - 6.54).abs() < 0.02);
        assert!((hba1c.to_canonical(39.0, "mmol/mol").expect("conv") - 5.72).abs() < 0.02);
        assert_eq!(hba1c.to_canonical(6.0, " % "), Some(6.0));
        let creat = lookup("CREAT").expect("profile");
        assert!((creat.to_canonical(88.42, "µmol/L").expect("conv") - 1.0).abs() < 1e-9);
        assert!((creat.to_canonical(88.42, "umol/L").expect("conv") - 1.0).abs() < 1e-9);
        assert!((creat.to_canonical(88.42, "\u{3bc}mol/L").expect("conv") - 1.0).abs() < 1e-9);
        let ldl = lookup("LDL").expect("profile");
        assert!((ldl.to_canonical(2.6, "mmol/L").expect("conv") - 100.5).abs() < 0.1);
        let tsh = lookup("TSH").expect("profile");
        assert_eq!(tsh.to_canonical(2.0, "uIU/mL"), Some(2.0));
        assert_eq!(tsh.to_canonical(2.0, "mg/dL"), None);
    }

    #[test]
    fn every_profile_is_complete_and_cited() {
        for p in PROFILES {
            assert!(
                p.cvi > 0.0 && p.cvi < 1.0 && p.cva > 0.0 && p.cva < 1.0,
                "{}",
                p.code
            );
            assert!(
                !p.cvi_source.is_empty() && !p.cva_source.is_empty(),
                "{}",
                p.code
            );
            assert!(!p.loinc.is_empty(), "{}", p.code);
            if let Some(r) = &p.population {
                assert!(r.low < r.high && !r.source.is_empty(), "{}", p.code);
            }
            for t in p.thresholds {
                assert!(!t.source.is_empty() && !t.label.is_empty(), "{}", p.code);
            }
        }
    }

    #[test]
    fn creatinine_rcv_matches_the_literature_order_of_magnitude() {
        // with CVI 4.4 % and CVA 1.1 % EuBIVAS reports ≈ +13 % / −12 %;
        // our assumed CVA 2 % widens it slightly
        let s = lookup("CREAT").expect("profile").sigma_log();
        let up = (crate::stats::Z95 * std::f64::consts::SQRT_2 * s).exp() - 1.0;
        assert!(up > 0.13 && up < 0.16, "RCV up {up}");
    }
}
