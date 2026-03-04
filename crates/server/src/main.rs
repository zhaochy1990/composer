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
    // ── Load configuration (env vars > ~/.composer/config.toml > defaults) ──
    // Note: loaded BEFORE tracing init so we can read logging.level.
    // Neither load() emits tracing calls; we call log_summary() after init.
    let config = composer_config::ComposerConfig::load(None)?;
    let credentials = composer_config::CredentialsConfig::load(None)?;

    // ── Ensure ~/.composer/ directories exist ──
    if let Err(e) = composer_config::ensure_directories() {
        eprintln!("Warning: could not create ~/.composer/ directories: {e}");
    }

    // ── Logging ──
    if config.logging.log_to_file {
        let log_dir = composer_config::logs_dir().unwrap_or_else(|_| "logs".into());
        let file_appender = tracing_appender::rolling::daily(&log_dir, "composer.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&config.logging.level)),
            )
            .with_writer(non_blocking)
            .init();

        // Now that tracing is live, log config diagnostics
        config.log_summary();
        credentials.log_summary();

        // Keep _guard alive for the duration of main
        run_server(config, credentials, Some(_guard)).await
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&config.logging.level)),
            )
            .init();

        // Now that tracing is live, log config diagnostics
        config.log_summary();
        credentials.log_summary();

        run_server(config, credentials, None::<tracing_appender::non_blocking::WorkerGuard>).await
    }
}

async fn run_server<G: Send + 'static>(
    config: composer_config::ComposerConfig,
    credentials: composer_config::CredentialsConfig,
    _log_guard: Option<G>,
) -> anyhow::Result<()> {
    // ── Inject credentials into env for downstream usage ──
    credentials.inject_into_env();

    // ── Database ──
    let db = Arc::new(composer_db::Database::connect(&config.database.url_pattern).await?);
    db.run_migrations().await?;

    let event_bus = composer_services::event_bus::EventBus::new();
    let services = composer_services::ServiceContainer::new(db, event_bus.clone());

    let state = Arc::new(AppState { services, event_bus });

    // ── CORS ──
    let origins: Vec<HeaderValue> = config
        .cors
        .origins
        .iter()
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

    // ── Bind ──
    let addr = std::net::SocketAddr::new(
        config.server.bind_address.parse()?,
        config.server.port,
    );
    tracing::info!("Composer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
