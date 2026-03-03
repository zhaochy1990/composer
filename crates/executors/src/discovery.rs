use composer_api_types::AgentType;

pub struct DiscoveredAgent {
    pub name: String,
    pub agent_type: AgentType,
    pub executable_path: String,
    pub is_authenticated: bool,
}

/// Discover installed coding agents on this machine.
pub async fn discover_agents() -> Vec<DiscoveredAgent> {
    let mut agents = Vec::new();

    let npx_result = if cfg!(target_os = "windows") {
        which::which("npx.cmd").or_else(|_| which::which("npx"))
    } else {
        which::which("npx")
    };

    if let Ok(npx_path) = npx_result {
        agents.push(DiscoveredAgent {
            name: "Claude Code".to_string(),
            agent_type: AgentType::ClaudeCode,
            executable_path: npx_path.to_string_lossy().to_string(),
            is_authenticated: check_claude_auth().await,
        });
    }

    agents
}

/// Check if Claude Code is authenticated.
async fn check_claude_auth() -> bool {
    std::env::var("ANTHROPIC_API_KEY").is_ok()
}
