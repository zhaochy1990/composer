use axum::{
    extract::Query,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use composer_api_types::{BrowseResponse, DirEntry};
use crate::AppState;
use crate::error::ServiceError;
use serde::Deserialize;

#[derive(Deserialize)]
struct BrowseParams {
    path: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/filesystem/browse", get(browse))
}

async fn browse(Query(params): Query<BrowseParams>) -> Result<Json<BrowseResponse>, ServiceError> {
    let target = match params.path {
        Some(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => dirs::home_dir()
            .ok_or_else(|| ServiceError::Internal(anyhow::anyhow!("Could not determine home directory")))?,
    };

    let target = target
        .canonicalize()
        .map_err(|e| ServiceError::BadRequest(format!("Invalid path: {}", e)))?;

    if !target.is_dir() {
        return Err(ServiceError::BadRequest(format!(
            "Not a directory: {}",
            target.display()
        )));
    }

    let parent = target.parent().map(|p| p.to_string_lossy().to_string());

    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&target)
        .await
        .map_err(|e| ServiceError::BadRequest(format!("Cannot read directory: {}", e)))?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
        ServiceError::Internal(anyhow::anyhow!("Error reading entry: {}", e))
    })? {
        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue, // skip entries we can't stat
        };
        if !metadata.is_dir() {
            continue; // only return directories
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden directories
        if name.starts_with('.') {
            continue;
        }
        entries.push(DirEntry {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: true,
        });
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(Json(BrowseResponse {
        current_path: target.to_string_lossy().to_string(),
        parent,
        entries,
    }))
}
