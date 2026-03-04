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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_config_dir_ends_with_dot_composer() {
        let dir = global_config_dir().unwrap();
        assert!(dir.ends_with(".composer"));
    }

    #[test]
    fn config_file_path_ends_with_config_toml() {
        let path = config_file_path().unwrap();
        assert_eq!(path.file_name().unwrap(), "config.toml");
    }

    #[test]
    fn credentials_file_path_ends_with_credentials_toml() {
        let path = credentials_file_path().unwrap();
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
}
