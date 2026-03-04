use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum GitWorktreeError {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("worktree already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct WorktreeInfo {
    pub worktree_path: PathBuf,
    pub branch_name: String,
}

/// Create a new git worktree with a new branch.
/// Worktree dir: <repo_path>/.composer/worktrees/<name>/
/// Branch name: composer/<name>
pub async fn create_worktree(
    repo_path: &Path,
    name: &str,
    base_branch: Option<&str>,
) -> Result<WorktreeInfo, GitWorktreeError> {
    let worktree_dir = repo_path.join(".composer").join("worktrees").join(name);
    let branch_name = format!("composer/{}", name);

    if worktree_dir.exists() {
        return Err(GitWorktreeError::AlreadyExists(worktree_dir));
    }

    tokio::fs::create_dir_all(
        worktree_dir.parent().ok_or_else(|| {
            GitWorktreeError::CommandFailed("worktree path has no parent directory".to_string())
        })?,
    )
    .await?;

    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.args(["worktree", "add"]);
    cmd.arg(&worktree_dir);
    cmd.args(["-b", &branch_name]);

    if let Some(base) = base_branch {
        cmd.arg(base);
    }

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitWorktreeError::CommandFailed(stderr.to_string()));
    }

    Ok(WorktreeInfo {
        worktree_path: worktree_dir,
        branch_name,
    })
}

/// Remove a git worktree and its branch.
pub async fn remove_worktree(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
) -> Result<(), GitWorktreeError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "remove", "--force"])
        .arg(worktree_path)
        .output()
        .await?;

    if !output.status.success() {
        if worktree_path.exists() {
            tokio::fs::remove_dir_all(worktree_path).await?;
        }
    }

    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "prune"])
        .output()
        .await;

    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["branch", "-D", branch_name])
        .output()
        .await;

    Ok(())
}

/// Parse `git worktree list --porcelain` output into WorktreeInfo entries.
pub fn parse_porcelain(output: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch.to_string());
        } else if line.is_empty() {
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push(WorktreeInfo {
                    worktree_path: path,
                    branch_name: branch,
                });
            }
        }
    }

    // Handle last entry if no trailing newline
    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        worktrees.push(WorktreeInfo {
            worktree_path: path,
            branch_name: branch,
        });
    }

    worktrees
}

/// List all active worktrees by parsing `git worktree list --porcelain`.
pub async fn list_worktrees(
    repo_path: &Path,
) -> Result<Vec<WorktreeInfo>, GitWorktreeError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitWorktreeError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_porcelain(&stdout))
}
