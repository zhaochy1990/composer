use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;

#[derive(Clone)]
pub struct WorktreeService {
    db: Arc<Database>,
}

impl WorktreeService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn create_for_session(
        &self,
        repo_path: &str,
        agent_name: &str,
        agent_id: &str,
        session_id: &str,
    ) -> anyhow::Result<Worktree> {
        let short_id = &session_id[..8.min(session_id.len())];
        // Sanitize agent_name: only keep [a-zA-Z0-9_-]
        let safe_name: String = agent_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_lowercase();
        let safe_name = if safe_name.is_empty() { "agent".to_string() } else { safe_name };
        let worktree_name = format!("{}-{}", safe_name, short_id);

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
