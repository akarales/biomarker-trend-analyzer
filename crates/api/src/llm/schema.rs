//! The JSON schema the model must comply with, and strict parsing of its reply.

use serde_json::json;

use super::Explanation;
use crate::error::ApiError;

/// Text fields first so streamed text starts immediately; `status` last.
pub fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "interpretation": {"type": "string"},
            "follow_up": {"type": "string"},
            "limitations": {"type": "string"},
            "status": {"type": "string", "enum": ["normal", "watch", "alert"]}
        },
        "required": ["summary", "interpretation", "follow_up", "limitations", "status"],
        "additionalProperties": false
    })
}

pub fn parse_explanation(raw: &str) -> Result<Explanation, ApiError> {
    serde_json::from_str(raw).map_err(|e| {
        tracing::warn!(error = %e, "model output did not match the explanation schema");
        ApiError::LlmUpstream("model output did not match schema".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_the_complete_shape() {
        let ok = r#"{"summary":"s","interpretation":"i","follow_up":"f","limitations":"l","status":"watch"}"#;
        assert_eq!(parse_explanation(ok).expect("valid").status, "watch");
        assert!(parse_explanation(r#"{"summary":"s"}"#).is_err());
        assert!(parse_explanation("not json").is_err());
        let required = schema()["required"].as_array().expect("required").len();
        assert_eq!(required, 5);
    }
}
