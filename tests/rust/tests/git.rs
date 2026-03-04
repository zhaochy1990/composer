use composer_git::worktree::{parse_porcelain, GitWorktreeError};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Parser tests (from crates/git/src/worktree.rs — no git required)
// Now tests exercise the real parse_porcelain() function.
// ---------------------------------------------------------------------------

#[test]
fn parse_single_worktree() {
    let output = "worktree /repo/.composer/worktrees/test\nHEAD abc123\nbranch refs/heads/composer/test\n\n";
    let wts = parse_porcelain(output);
    assert_eq!(wts.len(), 1);
    assert_eq!(wts[0].branch_name, "composer/test");
    assert_eq!(wts[0].worktree_path, PathBuf::from("/repo/.composer/worktrees/test"));
}

#[test]
fn parse_multiple_worktrees() {
    let output = "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /repo/.composer/worktrees/wt1\nHEAD def\nbranch refs/heads/composer/wt1\n\n";
    let wts = parse_porcelain(output);
    assert_eq!(wts.len(), 2);
}

#[test]
fn parse_empty_output() {
    let wts = parse_porcelain("");
    assert!(wts.is_empty());
}

#[test]
fn parse_no_trailing_newline() {
    let output = "worktree /repo\nHEAD abc\nbranch refs/heads/main";
    let wts = parse_porcelain(output);
    assert_eq!(wts.len(), 1);
    assert_eq!(wts[0].branch_name, "main");
}

#[test]
fn parse_bare_worktree_skipped() {
    // A bare worktree has no branch line — should be skipped
    let output = "worktree /repo\nHEAD abc\nbare\n\n";
    let wts = parse_porcelain(output);
    assert!(wts.is_empty());
}

#[test]
fn already_exists_error() {
    let err = GitWorktreeError::AlreadyExists(PathBuf::from("/test"));
    assert!(err.to_string().contains("/test"));
}

#[test]
fn command_failed_error() {
    let err = GitWorktreeError::CommandFailed("bad thing".to_string());
    assert!(err.to_string().contains("bad thing"));
}

#[test]
fn worktree_info_branch_naming() {
    // Verify the expected branch naming convention
    let name = "my-feature";
    let branch = format!("composer/{}", name);
    assert_eq!(branch, "composer/my-feature");
}
