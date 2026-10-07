//! Anthropic Messages API with JSON-schema structured output.

use serde::Deserialize;
use serde_json::json;

use super::upstream;
use crate::config::LlmConfig as Config;
use crate::error::ApiError;

pub const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Room for both the clinician and the patient section in one response.
pub const ANTHROPIC_MAX_TOKENS: u32 = 2048;

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

pub async fn messages(
    http: &reqwest::Client,
    config: &Config,
    model: &str,
    system: &str,
    schema: &serde_json::Value,
    context: &str,
) -> Result<String, ApiError> {
    let key = config
        .anthropic_api_key
        .as_deref()
        .ok_or_else(|| ApiError::LlmUpstream("anthropic api key not configured".into()))?;
    let body = json!({
        "model": model,
        "max_tokens": ANTHROPIC_MAX_TOKENS,
        // no `temperature`: current Claude models reject it; determinism comes
        // from the JSON-schema constraint and the grounded prompt instead
        "system": system,
        "messages": [{"role": "user", "content": context}],
        "output_config": {
            "format": {"type": "json_schema", "schema": schema}
        }
    });
    let response = http
        .post(format!("{}/v1/messages", config.anthropic_url))
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| upstream("anthropic", e))?;
    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(upstream("anthropic", format!("{status}: {detail}")));
    }
    let parsed: AnthropicResponse = response
        .json()
        .await
        .map_err(|e| upstream("anthropic", e))?;
    parsed
        .content
        .into_iter()
        .find(|block| block.kind == "text")
        .map(|block| block.text)
        .ok_or_else(|| upstream("anthropic", "response had no text block"))
}
