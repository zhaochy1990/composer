use std::path::PathBuf;

use anyhow::{Context, Result};

/// Returns the global config directory: `~/.composer/`
pub fn global_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".composer"))
}

/// Path to the main config file: `~/.composer/config.toml`
pub fn config_file_path() -> Result<PathBuf> {
    Ok(global_config_dir()?.join("config.toml"))
}

/// Path to the credentials file: `~/.composer/credentials.toml`
pub fn credentials_file_path() -> Result<PathBuf> {
    Ok(global_config_dir()?.join("credentials.toml"))
}

/// Path to the logs directory: `~/.composer/logs/`
pub fn logs_dir() -> Result<PathBuf> {
    Ok(global_config_dir()?.join("logs"))
}

/// Path to the data directory: `~/.composer/data/`
pub fn data_dir() -> Result<PathBuf> {
    Ok(global_config_dir()?.join("data"))
}

/// Ensure `~/.composer/` and its subdirectories exist.
pub fn ensure_directories() -> Result<()> {
    let dirs = [global_config_dir()?, logs_dir()?, data_dir()?];
    for dir in &dirs {
        if !dir.exists() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
            tracing::info!("Created directory: {}", dir.display());
        }
    }
    Ok(())
}
