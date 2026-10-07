//! Validation of persisted free text (review reasons and notes), ported
//! from app #1. The app stores synthetic demo data only: text that looks
//! like a patient identifier is rejected — a conservative pattern screen,
//! not a de-identification tool.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::ApiError;

pub const REASON_MIN: usize = 3;
pub const REASON_MAX: usize = 500;

static PHI_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    [
        ("an email address", r"[\w.+-]+@[\w-]+\.[\w.]+"),
        ("a phone number", r"\(?\b\d{3}\)?[\s.-]\d{3}[\s.-]\d{4}\b"),
        ("an ID number", r"\b\d{3}-\d{2}-\d{4}\b|\d{6,}"),
        (
            "an identifier keyword",
            r"(?i)\b(mrn|dob|ssn|date of birth|medical record|patient name)\b",
        ),
    ]
    .into_iter()
    .map(|(what, pattern)| {
        (
            what,
            Regex::new(pattern).expect("static PHI pattern compiles"),
        )
    })
    .collect()
});

/// Reject text that looks like it carries patient identifiers.
pub fn no_identifiers(field: &str, text: &str) -> Result<(), ApiError> {
    match PHI_PATTERNS.iter().find(|(_, re)| re.is_match(text)) {
        Some((what, _)) => Err(ApiError::BadRequest(format!(
            "{field} looks like it contains {what}; this demo stores synthetic data only — \
             describe the clinical reasoning without identifiers"
        ))),
        None => Ok(()),
    }
}

/// A trimmed reason of 3..=500 characters with no identifier patterns.
pub fn reason(raw: &str) -> Result<String, ApiError> {
    let reason = raw.trim();
    let len = reason.chars().count();
    if !(REASON_MIN..=REASON_MAX).contains(&len) {
        return Err(ApiError::BadRequest(format!(
            "reason must be {REASON_MIN}..={REASON_MAX} characters"
        )));
    }
    no_identifiers("reason", reason)?;
    Ok(reason.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_clinical_text_and_trims() {
        assert_eq!(
            reason("  known CKD, stable on review 2026-09  ").expect("ok"),
            "known CKD, stable on review 2026-09"
        );
        assert!(reason("HbA1c 7.0 % after steroid course; repeat in 3 months").is_ok());
    }

    #[test]
    fn rejects_identifiers_and_bad_lengths() {
        for bad in [
            "ok",
            "see MRN 12345678",
            "call 555-123-4567",
            "mail jane@example.org",
            "DOB on file",
        ] {
            assert!(reason(bad).is_err(), "{bad}");
        }
        assert!(reason(&"x".repeat(501)).is_err());
    }
}
