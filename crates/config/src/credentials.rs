use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Credentials configuration, loaded from `~/.composer/credentials.toml`.
///
/// Precedence: env var > credentials file > None.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CredentialsConfig {
    pub anthropic_api_key: Option<String>,
}

impl CredentialsConfig {
    /// Load credentials from a TOML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read credentials file: {}", path.display()))?;
        let creds: CredentialsConfig =
            toml::from_str(&contents).context("Failed to parse credentials.toml")?;
        Ok(creds)
    }

    /// Load credentials with full precedence: env vars > file > None.
    ///
    /// Note: this method does NOT emit tracing calls because it may run
    /// before the tracing subscriber is initialized. Call [`log_summary`]
    /// after initializing the subscriber.
    pub fn load(creds_path: Option<&Path>) -> Result<Self> {
        Self::load_with_env(creds_path, |key| std::env::var(key).ok())
    }

    /// Load credentials using a custom env lookup function.
    ///
    /// Testable core — avoids `std::env::set_var` in parallel tests.
    pub fn load_with_env(
        creds_path: Option<&Path>,
        env_lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        let mut creds = match creds_path {
            Some(path) if path.exists() => {
                check_file_permissions(path);
                Self::from_file(path)?
            }
            Some(_) => Self::default(),
            None => match crate::paths::credentials_file_path() {
                Ok(default_path) if default_path.exists() => {
                    check_file_permissions(&default_path);
                    Self::from_file(&default_path)?
                }
                _ => Self::default(),
            },
        };

        creds.apply_env_overrides(&env_lookup);
        Ok(creds)
    }

    /// Log a summary of loaded credentials. Call this **after**
    /// initializing the tracing subscriber.
    pub fn log_summary(&self) {
        if self.anthropic_api_key.is_some() {
            tracing::debug!("Anthropic API key loaded");
        }
    }

    fn apply_env_overrides(&mut self, env_lookup: &impl Fn(&str) -> Option<String>) {
        if let Some(val) = env_lookup("ANTHROPIC_API_KEY") {
            self.anthropic_api_key = Some(val);
        }
    }

    /// Inject loaded credentials into the process environment so downstream
    /// code (e.g. `discovery.rs`) can read them via `std::env::var`.
    pub fn inject_into_env(&self) {
        if let Some(ref key) = self.anthropic_api_key {
            // Only set if not already present (env var has highest precedence
            // and was already read, but this covers the file-only case).
            if std::env::var("ANTHROPIC_API_KEY").is_err() {
                std::env::set_var("ANTHROPIC_API_KEY", key);
            }
        }
    }
}

/// Check file permissions on Unix systems. Warn if credentials file is
/// world-readable.
#[cfg(unix)]
fn check_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            tracing::warn!(
                "Credentials file {} has overly permissive permissions ({:o}). \
                 Consider running: chmod 600 {}",
                path.display(),
                mode & 0o777,
                path.display()
            );
        }
    }
}

#[cfg(not(unix))]
fn check_file_permissions(_path: &Path) {
    // Permission checks are Unix-only for now.
}
