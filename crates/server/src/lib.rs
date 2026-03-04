pub mod routes;
pub mod error;

use std::sync::Arc;
use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pub services: composer_services::ServiceContainer,
    pub event_bus: composer_services::event_bus::EventBus,
}

/// Build the application router with all routes and middleware.
/// Extracted from main() to enable integration testing.
///
/// `cors_origins` is a list of allowed CORS origins (e.g. from `ComposerConfig`).
pub fn build_app(state: Arc<AppState>, cors_origins: &[String]) -> Router {
    let origins: Vec<HeaderValue> = cors_origins
        .iter()
        .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
        .collect();

    Router::new()
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
        .with_state(state)
}
