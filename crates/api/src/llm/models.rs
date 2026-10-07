//! What the model chooser may offer, and validation of a client's choice.

use serde::Serialize;

use super::ollama;
use crate::config::{ANTHROPIC_MODELS, LlmConfig as Config, LlmProvider};
use crate::error::ApiError;

/// One selectable model in the chooser.
#[derive(Debug, Clone, Serialize)]
pub struct ModelOption {
    pub provider: LlmProvider,
    pub id: String,
    pub label: String,
    /// Ollama only: currently resident in memory (no load / no eviction).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loaded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_gb: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub provider: LlmProvider,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub models: Vec<ModelOption>,
}

/// Every provider and model the chooser may offer, with availability.
pub async fn list_models(http: &reqwest::Client, config: &Config) -> Vec<ProviderStatus> {
    let has_key = config.anthropic_api_key.is_some();
    vec![
        ProviderStatus {
            provider: LlmProvider::Anthropic,
            available: has_key,
            note: (!has_key).then(|| "set ANTHROPIC_API_KEY to enable".to_string()),
            models: ANTHROPIC_MODELS
                .iter()
                .map(|(id, label)| ModelOption {
                    provider: LlmProvider::Anthropic,
                    id: (*id).to_string(),
                    label: (*label).to_string(),
                    loaded: None,
                    size_gb: None,
                })
                .collect(),
        },
        ollama::status(http, config).await,
        ProviderStatus {
            provider: LlmProvider::Stub,
            available: true,
            note: Some("deterministic offline text, no model call".into()),
            models: vec![ModelOption {
                provider: LlmProvider::Stub,
                id: "stub".into(),
                label: "Offline stub".into(),
                loaded: None,
                size_gb: None,
            }],
        },
    ]
}

/// Is `(provider, model)` selectable right now?
pub async fn validate_choice(
    http: &reqwest::Client,
    config: &Config,
    provider: LlmProvider,
    model: &str,
) -> Result<(), ApiError> {
    let ok = match provider {
        LlmProvider::Stub => true,
        LlmProvider::Anthropic => {
            config.anthropic_api_key.is_some()
                && ANTHROPIC_MODELS.iter().any(|(id, _)| *id == model)
        }
        LlmProvider::Ollama => ollama::status(http, config)
            .await
            .models
            .iter()
            .any(|m| m.id == model),
    };
    if ok {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "model `{model}` is not available for provider {provider:?}"
        )))
    }
}
