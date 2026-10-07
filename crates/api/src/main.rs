//! Binary entry: config, store selection (memory/postgres), optional demo
//! seeding, serve.

use std::sync::Arc;

use biomarker_api::config::Config;
use biomarker_api::import;
use biomarker_api::routes;
use biomarker_api::state::AppState;
use biomarker_api::store::Store;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,biomarker_api=debug".into()),
        )
        .init();

    // optional .env (gitignored) for ANTHROPIC_API_KEY & co.
    let _ = dotenvy::dotenv();
    let config = Config::from_env()?;
    let store = Store::connect(config.store, config.database_url.as_deref()).await?;

    if config.seed_demo_data && config.demo_data.exists() {
        // idempotent: a restart re-reads the files and inserts nothing new
        for file in import::seed_files(&config.demo_data)? {
            match import::parse_file(&file) {
                Ok(parsed) => {
                    let report = store.insert_observations(&parsed.observations).await?;
                    store.upsert_demographics(&parsed.patients).await?;
                    tracing::info!(
                        file = %file.display(),
                        format = ?parsed.format,
                        patients = parsed.observations.iter().map(|o| o.patient_id.as_str()).collect::<std::collections::BTreeSet<_>>().len(),
                        inserted = report.inserted,
                        duplicates = report.duplicates,
                        skipped = parsed.skipped_total(),
                        "seeded demo data"
                    );
                }
                Err(err) => {
                    tracing::warn!(file = %file.display(), %err, "demo file failed to parse; skipped")
                }
            }
        }
    }

    tracing::info!(
        store = store.backend_name(),
        port = config.port,
        "starting biomarker trend analyzer api"
    );

    let addr = format!("0.0.0.0:{}", config.port);
    let state = AppState::new(Arc::new(store), config);
    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
