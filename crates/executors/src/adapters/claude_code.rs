use super::{CliAdapter, CliCapabilities, CliSpawnConfig};
use crate::error::ExecutorError;
use crate::process_manager::CLAUDE_CODE_VERSION;
use composer_api_types::AgentType;
use std::path::PathBuf;

pub struct ClaudeCodeAdapter;

#[async_trait::async_trait]
impl CliAdapter for ClaudeCodeAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn capabilities(&self) -> CliCapabilities {
        CliCapabilities {
            supports_stdin_protocol: true,
            supports_resume: true,
            supports_stream_json_stdout: true,
            supports_plan_detection: true,
            supports_control_protocol: true,
        }
    }

    // NOTE: Claude Code's spawn path is handled directly in
    // AgentProcessManager::spawn() (not via spawn_generic), so this
    // is only a reference implementation kept in sync for completeness
    // and for potential future unification. If you change CLI flags,
    // update spawn() in process_manager.rs too.
    fn build_spawn_config(
        &self,
        _prompt: &str,
        _working_dir: &str,
        auto_approve: bool,
        resume_session_id: Option<&str>,
        resume_at_message_id: Option<&str>,
    ) -> Result<CliSpawnConfig, ExecutorError> {
        let npx_cmd = if cfg!(target_os = "windows") {
            "npx.cmd"
        } else {
            "npx"
        };
        let mut args = vec![
            "-y".to_string(),
            format!("@anthropic-ai/claude-code@{}", CLAUDE_CODE_VERSION),
            "--verbose".to_string(),
            "--output-format=stream-json".to_string(),
            "--input-format=stream-json".to_string(),
            "--include-partial-messages".to_string(),
            "--replay-user-messages".to_string(),
        ];

        if auto_approve {
            args.push("--dangerously-skip-permissions".to_string());
        }

        if let Some(resume_id) = resume_session_id {
            args.push("--resume".to_string());
            args.push(resume_id.to_string());
            if let Some(msg_id) = resume_at_message_id {
                args.push("--resume-session-at".to_string());
                args.push(msg_id.to_string());
            }
        }

        Ok(CliSpawnConfig {
            command: npx_cmd.to_string(),
            args,
            env_removes: vec!["CLAUDECODE".to_string()],
        })
    }

    fn find_executable(&self) -> Option<PathBuf> {
        let candidates = if cfg!(target_os = "windows") {
            vec!["claude.exe", "claude.cmd", "claude"]
        } else {
            vec!["claude"]
        };
        for name in candidates {
            if let Ok(path) = which::which(name) {
                return Some(path);
            }
        }
        // Fallback to npx (Claude Code can be run via npx)
        if cfg!(target_os = "windows") {
            which::which("npx.cmd")
                .or_else(|_| which::which("npx"))
                .ok()
        } else {
            which::which("npx").ok()
        }
    }

    async fn check_auth(&self) -> bool {
        // Fast path: check API key env var
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return true;
        }

        // Run `claude auth status` to check OAuth / claude.ai login
        let cli_path = match self.find_executable() {
            Some(p) => p,
            None => return false,
        };

        // Only run auth check if the native CLI was found (not npx fallback)
        let is_native = cli_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("claude"))
            .unwrap_or(false);

        if !is_native {
            return false;
        }

        let result = tokio::process::Command::new(&cli_path)
            .args(["auth", "status"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
                    return json
                        .get("loggedIn")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                }
                false
            }
            _ => false,
        }
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }
}
