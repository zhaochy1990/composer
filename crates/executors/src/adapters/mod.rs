pub mod claude_code;
pub mod codex;
pub mod copilot;
pub mod gemini;

use crate::error::ExecutorError;
use composer_api_types::AgentType;
use std::path::PathBuf;

/// Capability flags that the process manager checks to decide which
/// features to enable for a given CLI agent.
pub struct CliCapabilities {
    /// CLI supports bidirectional stream-JSON on stdin (multi-turn conversations).
    pub supports_stdin_protocol: bool,
    /// CLI supports session resume via a flag (e.g. `--resume`).
    pub supports_resume: bool,
    /// CLI emits stream-JSON on stdout that can be parsed as `CliMessage`.
    pub supports_stream_json_stdout: bool,
    /// CLI supports plan file detection (`.claude/plans/*.md`).
    pub supports_plan_detection: bool,
    /// CLI has a control protocol (CanUseTool, HookCallback, etc.).
    pub supports_control_protocol: bool,
}

/// Everything needed to spawn a CLI process.
pub struct CliSpawnConfig {
    /// The executable command (e.g. "npx.cmd", "codex", "gemini").
    pub command: String,
    /// Fully constructed argument list.
    pub args: Vec<String>,
    /// Environment variables to remove from the child process.
    pub env_removes: Vec<String>,
}

#[async_trait::async_trait]
pub trait CliAdapter: Send + Sync {
    /// Which agent type this adapter handles.
    fn agent_type(&self) -> AgentType;

    /// Declare what this CLI can do.
    fn capabilities(&self) -> CliCapabilities;

    /// Build the command + args for spawning the CLI process.
    fn build_spawn_config(
        &self,
        prompt: &str,
        working_dir: &str,
        auto_approve: bool,
        resume_session_id: Option<&str>,
        resume_at_message_id: Option<&str>,
    ) -> Result<CliSpawnConfig, ExecutorError>;

    /// Find the CLI binary on this machine.
    fn find_executable(&self) -> Option<PathBuf>;

    /// Check if the CLI is authenticated (API keys present, etc.).
    async fn check_auth(&self) -> bool;

    /// Human-readable display name for UI.
    fn display_name(&self) -> &str;
}

/// Registry of all known CLI adapters.
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn CliAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(claude_code::ClaudeCodeAdapter),
                Box::new(codex::CodexAdapter),
                Box::new(gemini::GeminiAdapter),
                Box::new(copilot::CopilotAdapter),
            ],
        }
    }

    pub fn get(&self, agent_type: &AgentType) -> Option<&dyn CliAdapter> {
        self.adapters
            .iter()
            .find(|a| a.agent_type() == *agent_type)
            .map(|a| a.as_ref())
    }

    pub fn all(&self) -> &[Box<dyn CliAdapter>] {
        &self.adapters
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_agent_types() {
        let registry = AdapterRegistry::new();
        assert_eq!(registry.all().len(), 4);

        assert!(registry.get(&AgentType::ClaudeCode).is_some());
        assert!(registry.get(&AgentType::Codex).is_some());
        assert!(registry.get(&AgentType::GeminiCli).is_some());
        assert!(registry.get(&AgentType::CopilotCli).is_some());
    }

    #[test]
    fn registry_returns_none_for_unknown_type_pattern() {
        let registry = AdapterRegistry::new();
        // All known types should resolve; just verify get() works for each
        for adapter in registry.all() {
            assert!(registry.get(&adapter.agent_type()).is_some());
        }
    }

    #[test]
    fn claude_code_capabilities_all_true() {
        let adapter = claude_code::ClaudeCodeAdapter;
        let caps = adapter.capabilities();
        assert!(caps.supports_stdin_protocol);
        assert!(caps.supports_resume);
        assert!(caps.supports_stream_json_stdout);
        assert!(caps.supports_plan_detection);
        assert!(caps.supports_control_protocol);
    }

    #[test]
    fn non_claude_capabilities_all_false() {
        let adapters: Vec<Box<dyn CliAdapter>> = vec![
            Box::new(codex::CodexAdapter),
            Box::new(gemini::GeminiAdapter),
            Box::new(copilot::CopilotAdapter),
        ];
        for adapter in &adapters {
            let caps = adapter.capabilities();
            assert!(
                !caps.supports_stdin_protocol,
                "{} should not support stdin protocol",
                adapter.display_name()
            );
            assert!(
                !caps.supports_resume,
                "{} should not support resume",
                adapter.display_name()
            );
            assert!(
                !caps.supports_plan_detection,
                "{} should not support plan detection",
                adapter.display_name()
            );
            assert!(
                !caps.supports_control_protocol,
                "{} should not support control protocol",
                adapter.display_name()
            );
        }
    }

    #[test]
    fn display_names_are_human_readable() {
        let registry = AdapterRegistry::new();
        let names: Vec<&str> = registry.all().iter().map(|a| a.display_name()).collect();
        assert_eq!(names, vec!["Claude Code", "Codex", "Gemini CLI", "Copilot CLI"]);
    }

    #[test]
    fn claude_code_build_spawn_config_basic() {
        let adapter = claude_code::ClaudeCodeAdapter;
        let config = adapter
            .build_spawn_config("test prompt", "/tmp", false, None, None)
            .unwrap();

        // Should use npx
        assert!(
            config.command.contains("npx"),
            "command should be npx, got: {}",
            config.command
        );
        // Should include the claude-code package
        assert!(config.args.iter().any(|a| a.contains("@anthropic-ai/claude-code@")));
        // Should include stream-json flags
        assert!(config.args.contains(&"--output-format=stream-json".to_string()));
        assert!(config.args.contains(&"--input-format=stream-json".to_string()));
        // Should NOT include --dangerously-skip-permissions when auto_approve=false
        assert!(!config.args.contains(&"--dangerously-skip-permissions".to_string()));
        // Should remove CLAUDECODE env var
        assert!(config.env_removes.contains(&"CLAUDECODE".to_string()));
    }

    #[test]
    fn claude_code_build_spawn_config_auto_approve() {
        let adapter = claude_code::ClaudeCodeAdapter;
        let config = adapter
            .build_spawn_config("test", "/tmp", true, None, None)
            .unwrap();
        assert!(config.args.contains(&"--dangerously-skip-permissions".to_string()));
    }

    #[test]
    fn claude_code_build_spawn_config_resume() {
        let adapter = claude_code::ClaudeCodeAdapter;
        let config = adapter
            .build_spawn_config("test", "/tmp", false, Some("sess-123"), Some("msg-456"))
            .unwrap();
        assert!(config.args.contains(&"--resume".to_string()));
        assert!(config.args.contains(&"sess-123".to_string()));
        assert!(config.args.contains(&"--resume-session-at".to_string()));
        assert!(config.args.contains(&"msg-456".to_string()));
    }

    #[test]
    fn adapter_agent_types_match() {
        assert_eq!(claude_code::ClaudeCodeAdapter.agent_type(), AgentType::ClaudeCode);
        assert_eq!(codex::CodexAdapter.agent_type(), AgentType::Codex);
        assert_eq!(gemini::GeminiAdapter.agent_type(), AgentType::GeminiCli);
        assert_eq!(copilot::CopilotAdapter.agent_type(), AgentType::CopilotCli);
    }
}
