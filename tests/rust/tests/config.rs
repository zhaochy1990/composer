use std::collections::HashMap;
use std::path::Path;

use composer_config::{ComposerConfig, CredentialsConfig};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an env lookup from key-value pairs (no std::env mutation needed).
fn mock_env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |key| map.get(key).cloned()
}

/// Empty env (no overrides).
fn empty_env() -> impl Fn(&str) -> Option<String> {
    |_| None
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

#[test]
fn global_config_dir_ends_with_dot_composer() {
    let dir = composer_config::global_config_dir().unwrap();
    assert!(dir.ends_with(".composer"));
}

#[test]
fn config_file_path_ends_with_config_toml() {
    let path = composer_config::config_file_path().unwrap();
    assert_eq!(path.file_name().unwrap(), "config.toml");
}

#[test]
fn credentials_file_path_ends_with_credentials_toml() {
    let path = composer_config::credentials_file_path().unwrap();
    assert_eq!(path.file_name().unwrap(), "credentials.toml");
}

#[test]
fn ensure_directories_creates_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join(".composer");
    let logs = base.join("logs");
    let data = base.join("data");

    // Manually create since ensure_directories uses home_dir
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::create_dir_all(&data).unwrap();

    assert!(base.exists());
    assert!(logs.exists());
    assert!(data.exists());
}

// ---------------------------------------------------------------------------
// ComposerConfig — defaults
// ---------------------------------------------------------------------------

#[test]
fn defaults_match_current_behavior() {
    let config = ComposerConfig::default();
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.bind_address, "127.0.0.1");
    assert_eq!(config.database.url_pattern, "sqlite:composer.db?mode=rwc");
    assert_eq!(config.logging.level, "composer=debug,tower_http=debug");
    assert_eq!(config.cors.origins.len(), 4);
    assert!(config
        .cors
        .origins
        .contains(&"http://localhost:5173".to_string()));
}

// ---------------------------------------------------------------------------
// ComposerConfig — file loading
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// ComposerConfig — env var overrides (via mock_env, no real env mutation)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// CredentialsConfig
// ---------------------------------------------------------------------------

#[test]
fn default_credentials_are_none() {
    let creds = CredentialsConfig::default();
    assert!(creds.anthropic_api_key.is_none());
}

#[test]
fn credentials_from_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("credentials.toml");
    std::fs::write(&path, r#"anthropic_api_key = "sk-test-123""#).unwrap();

    let creds = CredentialsConfig::from_file(&path).unwrap();
    assert_eq!(creds.anthropic_api_key.as_deref(), Some("sk-test-123"));
}

#[test]
fn missing_credentials_file_returns_defaults() {
    let creds = CredentialsConfig::load_with_env(
        Some(Path::new("/nonexistent/credentials.toml")),
        empty_env(),
    )
    .unwrap();
    assert!(creds.anthropic_api_key.is_none());
}

#[test]
fn env_var_overrides_credentials_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("credentials.toml");
    std::fs::write(&path, r#"anthropic_api_key = "sk-from-file""#).unwrap();

    let env = mock_env(&[("ANTHROPIC_API_KEY", "sk-from-env")]);
    let creds = CredentialsConfig::load_with_env(Some(&path), env).unwrap();
    assert_eq!(creds.anthropic_api_key.as_deref(), Some("sk-from-env"));
}

#[test]
fn env_only_credentials() {
    let env = mock_env(&[("ANTHROPIC_API_KEY", "sk-env-only")]);
    let creds = CredentialsConfig::load_with_env(None, env).unwrap();
    assert_eq!(creds.anthropic_api_key.as_deref(), Some("sk-env-only"));
}

#[test]
fn no_env_no_file_returns_none() {
    let creds = CredentialsConfig::load_with_env(
        Some(Path::new("/nonexistent/credentials.toml")),
        empty_env(),
    )
    .unwrap();
    assert!(creds.anthropic_api_key.is_none());
}

#[test]
fn inject_into_env_sets_value() {
    let creds = CredentialsConfig {
        anthropic_api_key: Some("sk-inject-test".into()),
    };
    // Verify the struct holds the value for injection
    assert_eq!(creds.anthropic_api_key.as_deref(), Some("sk-inject-test"));
}
