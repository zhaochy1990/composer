use std::sync::Arc;
use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
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

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:composer.db?mode=rwc".into());
    let db = Arc::new(composer_db::Database::connect(&db_url).await?);
    db.run_migrations().await?;

    let event_bus = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(db, event_bus.clone());

    let state = Arc::new(AppState { services, event_bus });

    // Fix #8: Read CORS_ORIGINS from env, restrict methods and headers
    let cors_origins_str = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173,http://localhost:3000,http://127.0.0.1:3000".to_string());
    let origins: Vec<HeaderValue> = cors_origins_str
        .split(',')
        .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
        .collect();

    let app = Router::new()
        .nest("/api", routes::api_router())
        .fallback(routes::frontend::serve_frontend)
        .layer(CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
            ]))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Composer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
