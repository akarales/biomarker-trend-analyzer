//! The SHARED local Ollama instance: chat + read-only model discovery.
//! Read/infer only — never pulls, deletes or configures models, and sends
//! no options that force a model reload (e.g. `num_ctx`).

use serde::Deserialize;
use serde_json::json;

use super::models::{ModelOption, ProviderStatus};
use super::upstream;
use crate::config::{LlmConfig as Config, LlmProvider};
use crate::error::ApiError;

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

/// Chat with a short `keep_alive` so our model releases memory quickly for
/// the other applications using this Ollama.
pub async fn chat(
    http: &reqwest::Client,
    config: &Config,
    model: &str,
    system: &str,
    schema: &serde_json::Value,
    context: &str,
) -> Result<String, ApiError> {
    let body = json!({
        "model": model,
        "keep_alive": config.ollama_keep_alive,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": context}
        ],
        "format": schema,
        "stream": false,
        "options": {"temperature": 0.1}
    });
    let response: ChatResponse = http
        .post(format!("{}/api/chat", config.ollama_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| upstream("ollama", e))?
        .error_for_status()
        .map_err(|e| upstream("ollama", e))?
        .json()
        .await
        .map_err(|e| upstream("ollama", e))?;
    Ok(response.message.content)
}

#[derive(Debug, Deserialize)]
struct OllamaTags {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
}

/// Read-only discovery: `GET /api/tags` (installed) and `GET /api/ps`
/// (loaded). Never mutates the instance.
pub async fn status(http: &reqwest::Client, config: &Config) -> ProviderStatus {
    let get = |path: &str| {
        http.get(format!("{}{path}", config.ollama_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
    };
    let tags = match get("/api/tags").await {
        Ok(r) => r.json::<OllamaTags>().await.ok(),
        Err(_) => None,
    };
    let Some(tags) = tags else {
        return ProviderStatus {
            provider: LlmProvider::Ollama,
            available: false,
            note: Some(format!("Ollama not reachable at {}", config.ollama_url)),
            models: Vec::new(),
        };
    };
    let loaded: Vec<String> = match get("/api/ps").await {
        Ok(r) => r
            .json::<OllamaTags>()
            .await
            .map(|ps| ps.models.into_iter().map(|m| m.name).collect())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let models = tags
        .models
        .into_iter()
        .filter(|m| match &m.capabilities {
            Some(caps) => caps.iter().any(|c| c == "completion"),
            None => !m.name.contains("embed"),
        })
        .map(|m| ModelOption {
            provider: LlmProvider::Ollama,
            label: m.name.clone(),
            loaded: Some(loaded.contains(&m.name)),
            size_gb: Some((m.size as f64 / 1e9 * 10.0).round() / 10.0),
            id: m.name,
        })
        .collect();
    ProviderStatus {
        provider: LlmProvider::Ollama,
        available: true,
        note: Some("shared instance — models are used as-is, never pulled or changed".into()),
        models,
    }
}
