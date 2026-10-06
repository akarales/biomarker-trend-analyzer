//! Environment configuration — read once, validated, fail-fast.

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
    /// Demo data CSV path (used when seeding).
    pub demo_csv: std::path::PathBuf,
    pub port: u16,
    /// Trend lookback window in days (the personal baseline is the earliest steady state).
    pub window_days: i64,
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
            .field("demo_csv", &self.demo_csv)
            .field("port", &self.port)
            .field("window_days", &self.window_days)
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

        let demo_csv = std::path::PathBuf::from(env_or(
            "APP_DEMO_CSV",
            "crates/api/tests/fixtures/demo_labs.csv",
        ));

        Ok(Self {
            store,
            database_url,
            seed_demo_data: env_or("APP_SEED_DEMO", "true").to_lowercase() != "false",
            demo_csv,
            port: env_or("APP_PORT", "8003")
                .parse()
                .map_err(|_| ConfigError("APP_PORT must be a valid port number".to_string()))?,
            window_days: env_or("APP_WINDOW_DAYS", "365")
                .parse()
                .map_err(|_| ConfigError("APP_WINDOW_DAYS must be a number of days".to_string()))?,
        })
    }
}
