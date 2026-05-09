use super::{CliAdapter, CliCapabilities, CliSpawnConfig};
use crate::error::ExecutorError;
use composer_api_types::AgentType;
use std::path::PathBuf;

pub struct CodexAdapter;

#[async_trait::async_trait]
impl CliAdapter for CodexAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::Codex
    }

    fn capabilities(&self) -> CliCapabilities {
        CliCapabilities {
            supports_stdin_protocol: false,
            supports_resume: false,
            supports_stream_json_stdout: false,
            supports_plan_detection: false,
            supports_control_protocol: false,
        }
    }

    fn build_spawn_config(
        &self,
        prompt: &str,
        _working_dir: &str,
        auto_approve: bool,
        _resume_session_id: Option<&str>,
        _resume_at_message_id: Option<&str>,
    ) -> Result<CliSpawnConfig, ExecutorError> {
        let binary = self
            .find_executable()
            .ok_or_else(|| ExecutorError::SpawnFailed("Codex CLI not found".into()))?;
        let mut args = vec![];

        if auto_approve {
            args.push("--full-auto".to_string());
        }

        args.push("-q".to_string());
        args.push(prompt.to_string());

        Ok(CliSpawnConfig {
            command: binary.to_string_lossy().to_string(),
            args,
            env_removes: vec![],
        })
    }

    fn find_executable(&self) -> Option<PathBuf> {
        let candidates = if cfg!(target_os = "windows") {
            vec!["codex.exe", "codex.cmd", "codex"]
        } else {
            vec!["codex"]
        };
        for name in candidates {
            if let Ok(path) = which::which(name) {
                return Some(path);
            }
        }
        None
    }

    async fn check_auth(&self) -> bool {
        std::env::var("OPENAI_API_KEY").is_ok()
    }

    fn display_name(&self) -> &str {
        "Codex"
    }
}
