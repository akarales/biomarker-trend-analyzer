//! Environment configuration — read once, validated, fail-fast.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    Memory,
    Postgres,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub store: StoreBackend,
    pub database_url: Option<String>,
    pub seed_demo_data: bool,
    /// Demo data CSV path (used when seeding).
    pub demo_csv: std::path::PathBuf,
    pub port: u16,
    /// Baseline lookback window in days.
    pub window_days: i64,
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
                Some("postgresql://app:app@localhost:5432/biomarkers".to_string())
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
            window_days: env_or("APP_WINDOW_DAYS", "90")
                .parse()
                .map_err(|_| ConfigError("APP_WINDOW_DAYS must be a number of days".to_string()))?,
        })
    }
}
