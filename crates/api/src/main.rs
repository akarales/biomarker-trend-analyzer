//! Binary entry: config, store selection (memory/postgres), optional demo
//! seeding, serve.

use std::sync::Arc;

use biomarker_api::config::Config;
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

    let config = Config::from_env()?;
    let store = Store::connect(config.store, config.database_url.as_deref()).await?;

    if config.seed_demo_data && config.demo_csv.exists() {
        let csv = std::fs::read_to_string(&config.demo_csv)?;
        match biomarker_ingest::parse_csv_bytes(csv.as_bytes()) {
            Ok(observations) => {
                let report = store.insert_observations(&observations).await?;
                tracing::info!(
                    patients = observations.iter().map(|o| o.patient_id.as_str()).collect::<std::collections::BTreeSet<_>>().len(),
                    inserted = report.inserted,
                    duplicates = report.duplicates,
                    demo_csv = %config.demo_csv.display(),
                    "seeded demo data"
                );
            }
            Err(err) => tracing::warn!(%err, "demo csv failed to parse; starting empty"),
        }
    }

    tracing::info!(
        store = store.backend_name(),
        port = config.port,
        "starting biomarker trend analyzer api"
    );

    let addr = format!("0.0.0.0:{}", config.port);
    let state = AppState {
        store: Arc::new(store),
        config: Arc::new(config),
    };
    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
