//! biomarker-drift — personal-baseline drift detection the way laboratory
//! medicine does it, as pure functions.
//!
//! For one patient's series of one analyte:
//!
//! 1. **Normalise** every reading to the analyte's canonical UCUM unit
//!    (unknown units are excluded and counted), drop readings after
//!    `as_of` (default: the latest reading — never the wall clock).
//! 2. **Personal baseline**: the earliest steady-state run of 3–10
//!    results gives a set point and a personalised reference interval
//!    (prRI, Coşkun et al. 2021) from population CVI/CVA.
//! 3. **Detectors**, each emitting explainable [`Signal`]s:
//!    prRI (latest vs the personal interval), RCV (latest vs previous),
//!    CUSUM shift with change point, EWMA, Mann–Kendall + Sen trend,
//!    clinical thresholds, population interval.
//! 4. **Status** = the worst signal. "normal" means no rule fired, not
//!    "healthy"; `not_assessed` lists what could not be checked and why.
//!
//! Biological-variation data, limits and their sources live in
//! [`profiles`]. No I/O, no clock, serde is the only dependency
//! (enforced by `tests/purity.rs`).

mod detectors;
mod engine;
mod fmt;
mod model;
pub mod profiles;
mod stats;

pub use engine::analyze;
pub use model::*;
pub use profiles::{AnalyteProfile, Direction, PopulationRange, Threshold, lookup};
