//! Shared helpers for the drift integration tests.
#![allow(dead_code)]

use biomarker_drift::{AnalysisInput, DriftReport, Reading, Rule};

pub const DAY: i64 = 86_400;
/// 2026-01-06T00:00:00Z
pub const START: i64 = 1_767_657_600;

pub fn weekly(values: &[f64], unit: &str) -> Vec<Reading> {
    values
        .iter()
        .enumerate()
        .map(|(i, &value)| Reading {
            t: START + i as i64 * 7 * DAY,
            value,
            unit: unit.into(),
        })
        .collect()
}

/// (week index, value) pairs.
pub fn readings(pairs: &[(i64, f64)], unit: &str) -> Vec<Reading> {
    pairs
        .iter()
        .map(|&(w, value)| Reading {
            t: START + w * 7 * DAY,
            value,
            unit: unit.into(),
        })
        .collect()
}

pub fn input<'a>(
    code: &'a str,
    readings: &'a [Reading],
    as_of: Option<i64>,
    window: i64,
) -> AnalysisInput<'a> {
    AnalysisInput {
        code,
        readings,
        as_of,
        trend_window_days: window,
    }
}

pub fn rules(r: &DriftReport) -> Vec<Rule> {
    r.signals.iter().map(|s| s.rule).collect()
}

/// Deterministic Gaussian noise (xorshift64* + Box–Muller).
pub struct Noise(u64);

impl Noise {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn gauss(&mut self) -> f64 {
        let (u, v) = (self.uniform(), self.uniform());
        (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
    }

    /// `level` with relative noise `cv`, rounded like a lab report.
    pub fn around(&mut self, level: f64, cv: f64) -> f64 {
        (level * (1.0 + cv * self.gauss()) * 1000.0).round() / 1000.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.uniform() * n as f64) as usize % n
    }
}
