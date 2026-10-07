//! Shared application state. Cheap to clone: Arc'd store/config + handle clients.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::config::Config;
use crate::store::Store;

/// A hung model provider must not hold a request forever (streams set a
/// longer per-request timeout).
const LLM_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const LLM_TOTAL_TIMEOUT: Duration = Duration::from_secs(60);
/// At most this many model calls in flight; extra requests queue.
pub const EXPLAIN_CONCURRENCY: usize = 2;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    /// Model calls in flight (streams hold a permit until they finish).
    pub explain_permits: Arc<Semaphore>,
}

impl AppState {
    pub fn new(store: Arc<Store>, config: Config) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(LLM_CONNECT_TIMEOUT)
            .timeout(LLM_TOTAL_TIMEOUT)
            .user_agent(concat!(
                "biomarker-trend-analyzer/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("static reqwest client configuration is valid");
        Self {
            store,
            config: Arc::new(config),
            http,
            explain_permits: Arc::new(Semaphore::new(EXPLAIN_CONCURRENCY)),
        }
    }
}
