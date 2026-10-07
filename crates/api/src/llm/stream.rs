//! Streaming backends: each yields the model's JSON reply as text
//! fragments. Dropping the returned stream drops the reqwest response,
//! which closes the upstream connection (frees the shared Ollama slot /
//! stops Anthropic billing tokens).

use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;

use futures_util::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::json;

use biomarker_drift::DriftReport;

use super::{stub, upstream};
use crate::config::{LlmConfig as Config, LlmProvider};
use crate::error::ApiError;

pub type TextStream = Pin<Box<dyn Stream<Item = Result<String, ApiError>> + Send>>;

/// Stub pacing: visible progress in demos, still fast in tests.
const STUB_CHUNK_CHARS: usize = 24;
const STUB_CHUNK_DELAY: Duration = Duration::from_millis(12);
/// A streamed reply may legitimately take longer than the client-wide 60 s.
const STREAM_TIMEOUT: Duration = Duration::from_secs(180);

/// Start a streamed explanation. Errors before the first byte (bad key,
/// HTTP status) are returned here so the route can answer with a status.
pub async fn explain_stream(
    http: &reqwest::Client,
    config: &Config,
    provider: LlmProvider,
    model: &str,
    report: &DriftReport,
    context: &str,
) -> Result<TextStream, ApiError> {
    let system = super::SYSTEM_PROMPT;
    let schema = super::schema::schema();
    match provider {
        LlmProvider::Stub => Ok(stub_stream(report)),
        LlmProvider::Ollama => {
            let body = json!({
                "model": model,
                "keep_alive": config.ollama_keep_alive,
                "messages": [{"role": "system", "content": system}, {"role": "user", "content": context}],
                "format": schema,
                "stream": true,
                "options": {"temperature": 0.1}
            });
            let response = send(
                http.post(format!("{}/api/chat", config.ollama_url))
                    .json(&body),
                "ollama",
            )
            .await?;
            Ok(Box::pin(
                lines(response).filter_map(|line| async move { ollama_line(line) }),
            ))
        }
        LlmProvider::Anthropic => {
            let key = config
                .anthropic_api_key
                .as_deref()
                .ok_or_else(|| ApiError::LlmUpstream("anthropic api key not configured".into()))?;
            let body = json!({
                "model": model,
                "max_tokens": super::anthropic::ANTHROPIC_MAX_TOKENS,
                "system": system,
                "messages": [{"role": "user", "content": context}],
                "output_config": {"format": {"type": "json_schema", "schema": schema}},
                "stream": true
            });
            let request = http
                .post(format!("{}/v1/messages", config.anthropic_url))
                .header("x-api-key", key)
                .header("anthropic-version", super::anthropic::ANTHROPIC_VERSION)
                .json(&body);
            let response = send(request, "anthropic").await?;
            Ok(Box::pin(
                lines(response).filter_map(|line| async move { anthropic_line(line) }),
            ))
        }
    }
}

async fn send(
    request: reqwest::RequestBuilder,
    provider: &'static str,
) -> Result<reqwest::Response, ApiError> {
    let response = request
        .timeout(STREAM_TIMEOUT)
        .send()
        .await
        .map_err(|e| upstream(provider, e))?;
    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(upstream(provider, format!("{status}: {detail}")));
    }
    Ok(response)
}

/// Response body as complete UTF-8 lines (chunks may split lines and
/// multi-byte characters anywhere).
fn lines(response: reqwest::Response) -> impl Stream<Item = Result<String, reqwest::Error>> + Send {
    let state = (
        response.bytes_stream().boxed(),
        Vec::<u8>::new(),
        VecDeque::<String>::new(),
        false,
    );
    stream::unfold(
        state,
        |(mut body, mut buf, mut ready, mut eof)| async move {
            loop {
                if let Some(line) = ready.pop_front() {
                    return Some((Ok(line), (body, buf, ready, eof)));
                }
                if eof {
                    return None;
                }
                match body.next().await {
                    Some(Ok(bytes)) => {
                        buf.extend_from_slice(&bytes);
                        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                            let line: Vec<u8> = buf.drain(..=pos).collect();
                            ready.push_back(String::from_utf8_lossy(&line).trim_end().to_string());
                        }
                    }
                    Some(Err(e)) => return Some((Err(e), (body, buf, ready, true))),
                    None => {
                        eof = true;
                        if !buf.is_empty() {
                            ready.push_back(
                                String::from_utf8_lossy(&std::mem::take(&mut buf))
                                    .trim_end()
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        },
    )
}

#[derive(Deserialize)]
struct OllamaChunk {
    #[serde(default)]
    message: Option<OllamaMessage>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    #[serde(default)]
    content: String,
}

fn ollama_line(line: Result<String, reqwest::Error>) -> Option<Result<String, ApiError>> {
    let line = match line {
        Ok(l) if l.trim().is_empty() => return None,
        Ok(l) => l,
        Err(e) => return Some(Err(upstream("ollama", e))),
    };
    match serde_json::from_str::<OllamaChunk>(&line) {
        Ok(OllamaChunk { error: Some(e), .. }) => Some(Err(upstream("ollama", e))),
        Ok(OllamaChunk {
            message: Some(m), ..
        }) if !m.content.is_empty() => Some(Ok(m.content)),
        Ok(_) => None,
        Err(e) => Some(Err(upstream("ollama", e))),
    }
}

/// Anthropic SSE: only `data:` lines matter; text arrives in
/// `content_block_delta` / `text_delta`; an `error` event aborts.
fn anthropic_line(line: Result<String, reqwest::Error>) -> Option<Result<String, ApiError>> {
    let line = match line {
        Ok(l) => l,
        Err(e) => return Some(Err(upstream("anthropic", e))),
    };
    let data = line.strip_prefix("data:")?.trim();
    let event: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => return Some(Err(upstream("anthropic", e))),
    };
    match event["type"].as_str() {
        Some("content_block_delta") if event["delta"]["type"] == "text_delta" => {
            event["delta"]["text"].as_str().map(|t| Ok(t.to_string()))
        }
        Some("error") => Some(Err(upstream("anthropic", event["error"].to_string()))),
        _ => None,
    }
}

/// The stub's JSON, served in small timed fragments (exercises the same
/// extractor + validation path as a real model).
fn stub_stream(report: &DriftReport) -> TextStream {
    let doc = json!(stub::explanation(report)).to_string();
    let chars: Vec<char> = doc.chars().collect();
    let chunks: Vec<String> = chars
        .chunks(STUB_CHUNK_CHARS)
        .map(|c| c.iter().collect())
        .collect();
    Box::pin(stream::iter(chunks).then(|chunk| async move {
        tokio::time::sleep(STUB_CHUNK_DELAY).await;
        Ok(chunk)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_sse_yields_only_text_deltas() {
        let ok = |s: &str| Ok::<_, reqwest::Error>(s.to_string());
        assert!(anthropic_line(ok("event: content_block_delta")).is_none());
        assert!(anthropic_line(ok(r#"data: {"type":"message_start","message":{}}"#)).is_none());
        let text = anthropic_line(ok(
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"{\"clin"}}"#,
        ));
        assert_eq!(text.and_then(Result::ok).as_deref(), Some("{\"clin"));
        let err = anthropic_line(ok(
            r#"data: {"type":"error","error":{"type":"overloaded_error"}}"#,
        ));
        assert!(matches!(err, Some(Err(_))));
    }

    #[test]
    fn ollama_ndjson_yields_message_content() {
        let ok = |s: &str| Ok::<_, reqwest::Error>(s.to_string());
        let text = ollama_line(ok(
            r#"{"message":{"role":"assistant","content":"{\"ex"},"done":false}"#,
        ));
        assert_eq!(text.and_then(Result::ok).as_deref(), Some("{\"ex"));
        assert!(ollama_line(ok(r#"{"message":{"content":""},"done":true}"#)).is_none());
        assert!(matches!(
            ollama_line(ok(r#"{"error":"model not found"}"#)),
            Some(Err(_))
        ));
    }

    #[tokio::test]
    async fn stub_stream_reassembles_into_schema_valid_json() {
        let readings: Vec<biomarker_drift::Reading> = (0..8)
            .map(|i| biomarker_drift::Reading {
                t: i * 7 * 86_400,
                value: if i < 7 { 5.6 } else { 7.0 },
                unit: "%".into(),
            })
            .collect();
        let report = biomarker_drift::analyze(&biomarker_drift::AnalysisInput {
            code: "HBA1C",
            readings: &readings,
            as_of: None,
            trend_window_days: 365,
        });
        let text: Vec<String> = stub_stream(&report)
            .map(|r| r.expect("stub never fails"))
            .collect()
            .await;
        assert!(text.len() > 3, "arrives in several fragments");
        let parsed = super::super::schema::parse_explanation(&text.concat()).expect("valid");
        assert_eq!(
            parsed.status, "alert",
            "the stub restates the computed status"
        );
        assert!(parsed.summary.contains("Hemoglobin A1c"));
    }
}
