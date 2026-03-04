use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use composer_server::{build_app, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("composer=debug,tower_http=debug"))
        )
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:composer.db?mode=rwc".into());
    let db = Arc::new(composer_db::Database::connect(&db_url).await?);
    db.run_migrations().await?;

    let event_bus = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(db, event_bus.clone());

    let state = Arc::new(AppState { services, event_bus });
    let app = build_app(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Composer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
