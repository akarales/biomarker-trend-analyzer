//! Environment configuration — read once, validated, fail-fast.

/// Which model backend serves "explain this drift".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Stub,
    Ollama,
    Anthropic,
}

/// Claude models offered in the model chooser (validated server-side).
pub const ANTHROPIC_MODELS: &[(&str, &str)] = &[
    ("claude-sonnet-5-5", "Claude Sonnet 5.5 — fast, strong"),
    ("claude-haiku-4-5", "Claude Haiku 4.5 — fastest"),
    ("claude-opus-5-5", "Claude Opus 5.5 — most capable"),
];

/// LLM settings (same variables as app #1). Default = offline stub.
#[derive(Clone)]
pub struct LlmConfig {
    pub llm_stub: bool,
    pub llm_provider: LlmProvider,
    pub ollama_url: String,
    pub ollama_model: String,
    /// How long Ollama keeps OUR model loaded after a call — short: the
    /// instance is shared with other applications.
    pub ollama_keep_alive: String,
    pub anthropic_url: String,
    pub anthropic_model: String,
    /// Secret — never logged (manual `Debug`).
    pub anthropic_api_key: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            llm_stub: true,
            llm_provider: LlmProvider::Ollama,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "qwen3:14b".into(),
            ollama_keep_alive: "2m".into(),
            anthropic_url: "https://api.anthropic.com".into(),
            anthropic_model: "claude-sonnet-5-5".into(),
            anthropic_api_key: None,
        }
    }
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("llm_stub", &self.llm_stub)
            .field("llm_provider", &self.llm_provider)
            .field("ollama_url", &self.ollama_url)
            .field("ollama_model", &self.ollama_model)
            .field("ollama_keep_alive", &self.ollama_keep_alive)
            .field("anthropic_url", &self.anthropic_url)
            .field("anthropic_model", &self.anthropic_model)
            .field(
                "anthropic_api_key",
                &self.anthropic_api_key.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl LlmConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let d = Self::default();
        let llm_provider = match env_or("APP_LLM_PROVIDER", "ollama").to_lowercase().as_str() {
            "ollama" => LlmProvider::Ollama,
            "anthropic" => LlmProvider::Anthropic,
            "stub" => LlmProvider::Stub,
            other => {
                return Err(ConfigError(format!(
                    "APP_LLM_PROVIDER must be `ollama`, `anthropic` or `stub`, got `{other}`"
                )));
            }
        };
        let llm_stub = env_or("APP_LLM_STUB", "true").to_lowercase() != "false";
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty());
        if !llm_stub && llm_provider == LlmProvider::Anthropic && anthropic_api_key.is_none() {
            return Err(ConfigError(
                "APP_LLM_PROVIDER=anthropic needs ANTHROPIC_API_KEY (or set APP_LLM_STUB=true)"
                    .into(),
            ));
        }
        Ok(Self {
            llm_stub,
            llm_provider,
            ollama_url: env_or("APP_OLLAMA_URL", &d.ollama_url),
            ollama_model: env_or("APP_OLLAMA_MODEL", &d.ollama_model),
            ollama_keep_alive: env_or("APP_OLLAMA_KEEP_ALIVE", &d.ollama_keep_alive),
            anthropic_url: env_or("APP_ANTHROPIC_URL", &d.anthropic_url),
            anthropic_model: env_or("APP_ANTHROPIC_MODEL", &d.anthropic_model),
            anthropic_api_key,
        })
    }

    /// Provider + model used when a request doesn't choose one.
    pub fn default_target(&self) -> (LlmProvider, String) {
        match (self.llm_stub, self.llm_provider) {
            (true, _) | (_, LlmProvider::Stub) => (LlmProvider::Stub, "stub".to_string()),
            (false, LlmProvider::Ollama) => (LlmProvider::Ollama, self.ollama_model.clone()),
            (false, LlmProvider::Anthropic) => {
                (LlmProvider::Anthropic, self.anthropic_model.clone())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    Memory,
    Postgres,
}

#[derive(Clone)]
pub struct Config {
    pub store: StoreBackend,
    pub database_url: Option<String>,
    pub seed_demo_data: bool,
    /// Demo data to seed: a file, or a directory of `*.json` (FHIR) and
    /// `*.csv` files seeded in name order.
    pub demo_data: std::path::PathBuf,
    pub port: u16,
    /// Trend lookback window in days (the personal baseline is the earliest steady state).
    pub window_days: i64,
    pub llm: LlmConfig,
}

/// Debug never prints the database URL (it carries credentials).
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("store", &self.store)
            .field(
                "database_url",
                &self.database_url.as_ref().map(|_| "<redacted>"),
            )
            .field("seed_demo_data", &self.seed_demo_data)
            .field("demo_data", &self.demo_data)
            .field("port", &self.port)
            .field("window_days", &self.window_days)
            .field("llm", &self.llm)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid configuration: {0}")]
pub struct ConfigError(String);

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let store = match env_or("APP_STORE", "memory").to_lowercase().as_str() {
            "memory" => StoreBackend::Memory,
            "postgres" => StoreBackend::Postgres,
            other => {
                return Err(ConfigError(format!(
                    "APP_STORE must be 'memory' or 'postgres', got {other}"
                )));
            }
        };
        let database_url = std::env::var("APP_DATABASE_URL").ok().or_else(|| {
            if store == StoreBackend::Postgres {
                Some("postgresql://app:app@127.0.0.1:5435/biomarkers".to_string())
            } else {
                None
            }
        });

        let demo_data = std::path::PathBuf::from(env_or("APP_DEMO_DATA", "demo"));

        Ok(Self {
            store,
            database_url,
            seed_demo_data: env_or("APP_SEED_DEMO", "true").to_lowercase() != "false",
            demo_data,
            port: env_or("APP_PORT", "8003")
                .parse()
                .map_err(|_| ConfigError("APP_PORT must be a valid port number".to_string()))?,
            window_days: env_or("APP_WINDOW_DAYS", "1095")
                .parse()
                .map_err(|_| ConfigError("APP_WINDOW_DAYS must be a number of days".to_string()))?,
            llm: LlmConfig::from_env()?,
        })
    }
}
