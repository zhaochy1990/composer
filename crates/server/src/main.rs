use std::sync::Arc;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod routes;
mod error;

pub struct AppState {
    pub services: composer_services::ServiceContainer,
    pub event_bus: composer_services::event_bus::EventBus,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("composer=debug,tower_http=debug"))
        )
        .init();

    let db = Arc::new(
        composer_db::Database::connect("sqlite:composer.db?mode=rwc").await?
    );
    db.run_migrations().await?;

    let event_bus = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(db, event_bus.clone());

    let state = Arc::new(AppState { services, event_bus });

    let app = Router::new()
        .nest("/api", routes::api_router())
        .fallback(routes::frontend::serve_frontend)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Composer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
