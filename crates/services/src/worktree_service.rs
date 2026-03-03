use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use composer_api_types::*;
use composer_db::Database;

#[derive(Clone)]
pub struct WorktreeService {
    db: Arc<Database>,
    creation_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl WorktreeService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            creation_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_for_session(
        &self,
        repo_path: &str,
        agent_name: &str,
        agent_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Worktree> {
        let short_id = &session_id[..8.min(session_id.len())];
        let worktree_name = format!("{}-{}", agent_name, short_id);

        // Acquire per-name lock to prevent concurrent creation of same worktree
        let lock = {
            let mut locks = self.creation_locks.lock().await;
            locks
                .entry(worktree_name.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        let info = composer_git::worktree::create_worktree(
            std::path::Path::new(repo_path),
            &worktree_name,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create worktree: {}", e))?;

        let worktree = composer_db::models::worktree::create(
            &self.db.pool,
            agent_id,
            session_id,
            repo_path,
            &info.worktree_path.to_string_lossy(),
            &info.branch_name,
        )
        .await?;

        Ok(worktree)
    }

    pub async fn cleanup(&self, worktree_id: &str) -> anyhow::Result<()> {
        let wt = composer_db::models::worktree::find_by_id(&self.db.pool, worktree_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Worktree not found"))?;

        composer_git::worktree::remove_worktree(
            std::path::Path::new(&wt.repo_path),
            std::path::Path::new(&wt.worktree_path),
            &wt.branch_name,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to remove worktree: {}", e))?;

        composer_db::models::worktree::update_status(
            &self.db.pool,
            worktree_id,
            &WorktreeStatus::Deleted,
        )
        .await?;

        Ok(())
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Worktree>> {
        composer_db::models::worktree::list_all(&self.db.pool).await
    }
}
