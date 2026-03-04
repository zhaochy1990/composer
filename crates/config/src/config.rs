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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Helper: build an env lookup from key-value pairs.
    fn mock_env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    /// Helper: empty env (no overrides).
    fn empty_env() -> impl Fn(&str) -> Option<String> {
        |_| None
    }

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
        let config =
            ComposerConfig::load_with_env(Some(Path::new("/nonexistent/config.toml")), empty_env())
                .unwrap();
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

        let env = mock_env(&[
            ("DATABASE_URL", "sqlite:from_env.db?mode=rwc"),
            ("COMPOSER_PORT", "4000"),
        ]);
        let config = ComposerConfig::load_with_env(Some(&path), env).unwrap();

        assert_eq!(config.database.url_pattern, "sqlite:from_env.db?mode=rwc");
        assert_eq!(config.server.port, 4000);
    }

    #[test]
    fn cors_origins_env_override() {
        let env = mock_env(&[("CORS_ORIGINS", "http://a.com, http://b.com")]);
        let config = ComposerConfig::load_with_env(None, env).unwrap();
        assert_eq!(config.cors.origins, vec!["http://a.com", "http://b.com"]);
    }

    #[test]
    fn invalid_port_env_var_ignored() {
        let env = mock_env(&[("COMPOSER_PORT", "not_a_number")]);
        let config = ComposerConfig::load_with_env(None, env).unwrap();
        assert_eq!(config.server.port, 3000); // default preserved
    }

    #[test]
    fn bind_address_env_override() {
        let env = mock_env(&[("COMPOSER_BIND_ADDRESS", "0.0.0.0")]);
        let config = ComposerConfig::load_with_env(None, env).unwrap();
        assert_eq!(config.server.bind_address, "0.0.0.0");
    }

    #[test]
    fn rust_log_env_override() {
        let env = mock_env(&[("RUST_LOG", "info")]);
        let config = ComposerConfig::load_with_env(None, env).unwrap();
        assert_eq!(config.logging.level, "info");
    }
}
