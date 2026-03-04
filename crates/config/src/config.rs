use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level configuration for Composer.
///
/// Precedence: env var > `~/.composer/config.toml` > defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ComposerConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub log_to_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    pub origins: Vec<String>,
}

// ── Defaults (match current hardcoded values) ────────────────────────

impl Default for ComposerConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            bind_address: "127.0.0.1".into(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url_pattern: "sqlite:composer.db?mode=rwc".into(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "composer=debug,tower_http=debug".into(),
            log_to_file: false,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            origins: vec![
                "http://localhost:5173".into(),
                "http://127.0.0.1:5173".into(),
                "http://localhost:3000".into(),
                "http://127.0.0.1:3000".into(),
            ],
        }
    }
}

// ── Loading ──────────────────────────────────────────────────────────

impl ComposerConfig {
    /// Load config from a TOML file, falling back to defaults for missing fields.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: ComposerConfig =
            toml::from_str(&contents).context("Failed to parse config.toml")?;
        Ok(config)
    }

    /// Load config with full precedence: env vars > file > defaults.
    ///
    /// If the config file doesn't exist, starts from defaults.
    /// Note: this method does NOT emit tracing calls because it runs before
    /// the tracing subscriber is initialized. Call [`log_summary`] after
    /// initializing the subscriber to log the loaded configuration.
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        Self::load_with_env(config_path, |key| std::env::var(key).ok())
    }

    /// Load config using a custom env lookup function.
    ///
    /// This is the testable core — tests can supply a controlled environment
    /// without touching `std::env::set_var` (which is unsafe in parallel tests).
    pub fn load_with_env(
        config_path: Option<&Path>,
        env_lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        let mut config = match config_path {
            Some(path) if path.exists() => Self::from_file(path)?,
            Some(_) => Self::default(),
            None => {
                match crate::paths::config_file_path() {
                    Ok(default_path) if default_path.exists() => Self::from_file(&default_path)?,
                    _ => Self::default(),
                }
            }
        };

        config.apply_env_overrides(&env_lookup);
        config.resolve_database_path();
        Ok(config)
    }

    /// Log a summary of the active configuration. Call this **after**
    /// initializing the tracing subscriber so messages are not dropped.
    pub fn log_summary(&self) {
        let defaults = Self::default();
        if self.server.port != defaults.server.port {
            tracing::info!("Config override: server.port = {}", self.server.port);
        }
        if self.server.bind_address != defaults.server.bind_address {
            tracing::info!(
                "Config override: server.bind_address = {}",
                self.server.bind_address
            );
        }
        tracing::info!("Database URL: {}", self.database.url_pattern);
        if self.logging.level != defaults.logging.level {
            tracing::info!("Config override: logging.level = {}", self.logging.level);
        }
    }

    /// If the database URL is still the default relative path, resolve it
    /// to `~/.composer/data/composer.db` so the DB lives in a stable location.
    fn resolve_database_path(&mut self) {
        let default_pattern = DatabaseConfig::default().url_pattern;
        if self.database.url_pattern == default_pattern {
            if let Ok(data_dir) = crate::paths::data_dir() {
                let db_path = data_dir.join("composer.db");
                self.database.url_pattern =
                    format!("sqlite:{}?mode=rwc", db_path.display());
            }
        }
    }

    /// Apply environment variable overrides using the given lookup function.
    fn apply_env_overrides(&mut self, env_lookup: &impl Fn(&str) -> Option<String>) {
        if let Some(val) = env_lookup("COMPOSER_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                self.server.port = port;
            }
        }

        if let Some(val) = env_lookup("COMPOSER_BIND_ADDRESS") {
            self.server.bind_address = val;
        }

        if let Some(val) = env_lookup("DATABASE_URL") {
            self.database.url_pattern = val;
        }

        if let Some(val) = env_lookup("RUST_LOG") {
            self.logging.level = val;
        }

        if let Some(val) = env_lookup("CORS_ORIGINS") {
            self.cors.origins = val.split(',').map(|s| s.trim().to_string()).collect();
        }
    }
}
