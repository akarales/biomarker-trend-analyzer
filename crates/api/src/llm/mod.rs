//! LLM layer (ported from app #1, drug-interaction-visualizer): a
//! schema-constrained "explain this drift" draft for the reviewing
//! clinician, via Ollama (local, shared) or the Anthropic Messages API,
//! with a deterministic stub so demos and tests never need a GPU or network.
//!
//! - `prompts` — system prompt (grounding rules: computed signals win)
//! - `schema`  — JSON schema and strict reply parsing
//! - `anthropic`, `ollama`, `stub` — one backend each
//! - `models`  — chooser data and choice validation
//! - `stream`  — streamed replies (Anthropic SSE, Ollama NDJSON, stub)
//! - `extract` — incremental field extraction from partial JSON

mod anthropic;
pub mod extract;
mod models;
mod ollama;
mod prompts;
mod schema;
mod stream;
mod stub;

use biomarker_drift::DriftReport;
use serde::{Deserialize, Serialize};

pub use models::{ModelOption, ProviderStatus, list_models, validate_choice};
pub use prompts::SYSTEM_PROMPT;
pub use schema::parse_explanation;
pub use stream::{TextStream, explain_stream};

use crate::config::{LlmConfig as Config, LlmProvider};
use crate::error::ApiError;

/// The model's draft. `status` is the model's echo of the computed status;
/// the server overwrites it with the engine's (see `assert_status`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explanation {
    /// what changed, in 2–4 sentences
    pub summary: String,
    /// possible explanations to consider (incl. pre-analytical, analytical
    /// and biological variation) — considerations, not a diagnosis
    pub interpretation: String,
    /// follow-up for the clinician to consider (no orders, no doses)
    pub follow_up: String,
    /// what this analysis cannot say (not assessed, missing context)
    pub limitations: String,
    pub status: String,
}

impl Explanation {
    /// The engine's status always wins. Returns whether the model disagreed.
    pub fn assert_status(&mut self, computed: &str) -> bool {
        let overridden = self.status != computed;
        self.status = computed.to_string();
        overridden
    }
}

/// Generate a draft with an explicit provider + model (validated by the
/// caller against [`list_models`]).
pub async fn explain(
    http: &reqwest::Client,
    config: &Config,
    provider: LlmProvider,
    model: &str,
    report: &DriftReport,
    context: &str,
) -> Result<Explanation, ApiError> {
    if provider == LlmProvider::Stub {
        return Ok(stub::explanation(report));
    }
    let schema = schema::schema();
    let raw = match provider {
        LlmProvider::Ollama => {
            ollama::chat(http, config, model, SYSTEM_PROMPT, &schema, context).await?
        }
        _ => anthropic::messages(http, config, model, SYSTEM_PROMPT, &schema, context).await?,
    };
    schema::parse_explanation(&raw)
}

/// Upstream failures are logged with detail; clients get a generic message.
fn upstream(provider: &str, err: impl std::fmt::Display) -> ApiError {
    tracing::error!(provider, error = %err, "llm upstream failure");
    ApiError::LlmUpstream(format!("{provider} request failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_status_wins() {
        let mut e = Explanation {
            summary: "s".into(),
            interpretation: "i".into(),
            follow_up: "f".into(),
            limitations: "l".into(),
            status: "normal".into(),
        };
        assert!(e.assert_status("alert"));
        assert_eq!(e.status, "alert");
        assert!(!e.assert_status("alert"));
    }
}
