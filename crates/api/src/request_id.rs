//! `x-request-id` on every response (and in the request's tracing span),
//! so a client-reported error can be found in the logs. A well-formed
//! incoming id is reused; otherwise one is generated.

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

pub const HEADER: &str = "x-request-id";
const MAX_LEN: usize = 64;

static NEXT: AtomicU64 = AtomicU64::new(1);
/// Process start (seconds, hex) — keeps ids distinct across restarts.
static EPOCH: LazyLock<String> = LazyLock::new(|| {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("{secs:x}")
});

fn well_formed(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub async fn request_id(req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| well_formed(s))
        .map_or_else(
            || format!("{}-{}", *EPOCH, NEXT.fetch_add(1, Ordering::Relaxed)),
            str::to_owned,
        );
    let span = tracing::info_span!("request", request_id = %id);
    let mut response = next.run(req).instrument(span).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(HEADER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_short_token_ids_are_reused() {
        assert!(well_formed("abc-123_X"));
        assert!(!well_formed(""));
        assert!(!well_formed("has space"));
        assert!(!well_formed(&"a".repeat(MAX_LEN + 1)));
    }
}
