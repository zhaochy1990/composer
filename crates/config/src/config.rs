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
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let mut config = match config_path {
            Some(path) if path.exists() => {
                tracing::info!("Loading config from {}", path.display());
                Self::from_file(path)?
            }
            Some(path) => {
                tracing::debug!("Config file not found at {}, using defaults", path.display());
                Self::default()
            }
            None => {
                // Try the default location
                match crate::paths::config_file_path() {
                    Ok(default_path) if default_path.exists() => {
                        tracing::info!("Loading config from {}", default_path.display());
                        Self::from_file(&default_path)?
                    }
                    _ => {
                        tracing::debug!("No config file found, using defaults");
                        Self::default()
                    }
                }
            }
        };

        // Apply env var overrides
        config.apply_env_overrides();
        config
            .log_active_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("COMPOSER_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                self.server.port = port;
            }
        }

        if let Ok(val) = std::env::var("COMPOSER_BIND_ADDRESS") {
            self.server.bind_address = val;
        }

        if let Ok(val) = std::env::var("DATABASE_URL") {
            self.database.url_pattern = val;
        }

        if let Ok(val) = std::env::var("RUST_LOG") {
            self.logging.level = val;
        }

        if let Ok(val) = std::env::var("CORS_ORIGINS") {
            self.cors.origins = val.split(',').map(|s| s.trim().to_string()).collect();
        }
    }

    fn log_active_overrides(&self) {
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
        if self.database.url_pattern != defaults.database.url_pattern {
            tracing::info!(
                "Config override: database.url_pattern = {}",
                self.database.url_pattern
            );
        }
        if self.logging.level != defaults.logging.level {
            tracing::info!("Config override: logging.level = {}", self.logging.level);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_current_behavior() {
        let config = ComposerConfig::default();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.bind_address, "127.0.0.1");
        assert_eq!(config.database.url_pattern, "sqlite:composer.db?mode=rwc");
        assert_eq!(config.logging.level, "composer=debug,tower_http=debug");
        assert_eq!(config.cors.origins.len(), 4);
        assert!(config.cors.origins.contains(&"http://localhost:5173".to_string()));
    }

    #[test]
    fn missing_file_returns_defaults() {
        let config = ComposerConfig::load(Some(Path::new("/nonexistent/config.toml"))).unwrap();
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn partial_toml_merges_with_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
port = 8080
"#,
        )
        .unwrap();

        let config = ComposerConfig::from_file(&path).unwrap();
        assert_eq!(config.server.port, 8080);
        // Other fields keep defaults
        assert_eq!(config.server.bind_address, "127.0.0.1");
        assert_eq!(config.database.url_pattern, "sqlite:composer.db?mode=rwc");
        assert_eq!(config.logging.level, "composer=debug,tower_http=debug");
    }

    #[test]
    fn full_toml_parses_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
port = 9000
bind_address = "0.0.0.0"

[database]
url_pattern = "sqlite:custom.db?mode=rwc"

[logging]
level = "info"
log_to_file = true

[cors]
origins = ["http://example.com"]
"#,
        )
        .unwrap();

        let config = ComposerConfig::from_file(&path).unwrap();
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.server.bind_address, "0.0.0.0");
        assert_eq!(config.database.url_pattern, "sqlite:custom.db?mode=rwc");
        assert_eq!(config.logging.level, "info");
        assert!(config.logging.log_to_file);
        assert_eq!(config.cors.origins, vec!["http://example.com"]);
    }

    #[test]
    fn env_vars_override_toml_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
port = 8080

[database]
url_pattern = "sqlite:from_file.db?mode=rwc"
"#,
        )
        .unwrap();

        // Set env vars
        std::env::set_var("DATABASE_URL", "sqlite:from_env.db?mode=rwc");
        std::env::set_var("COMPOSER_PORT", "4000");

        let config = ComposerConfig::load(Some(&path)).unwrap();

        // Env vars win
        assert_eq!(config.database.url_pattern, "sqlite:from_env.db?mode=rwc");
        assert_eq!(config.server.port, 4000);

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("COMPOSER_PORT");
    }

    #[test]
    fn cors_origins_env_override() {
        std::env::set_var("CORS_ORIGINS", "http://a.com, http://b.com");

        let config = ComposerConfig::load(None).unwrap();
        assert_eq!(
            config.cors.origins,
            vec!["http://a.com", "http://b.com"]
        );

        std::env::remove_var("CORS_ORIGINS");
    }

    #[test]
    fn invalid_port_env_var_ignored() {
        // Clean up any leftover from parallel tests
        std::env::remove_var("COMPOSER_PORT");

        std::env::set_var("COMPOSER_PORT", "not_a_number");
        let mut config = ComposerConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.server.port, 3000); // default preserved
        std::env::remove_var("COMPOSER_PORT");
    }
}
