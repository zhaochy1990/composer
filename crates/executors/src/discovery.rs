use composer_api_types::AgentType;
use tokio::process::Command;

pub struct DiscoveredAgent {
    pub name: String,
    pub agent_type: AgentType,
    pub executable_path: String,
    pub is_authenticated: bool,
}

/// Find the Claude Code CLI executable.
fn find_claude_cli() -> Option<std::path::PathBuf> {
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
    None
}

/// Discover installed coding agents on this machine.
pub async fn discover_agents() -> Vec<DiscoveredAgent> {
    let mut agents = Vec::new();

    // Prefer the native `claude` CLI; fall back to npx
    let claude_path = find_claude_cli().or_else(|| {
        let npx = if cfg!(target_os = "windows") {
            which::which("npx.cmd").or_else(|_| which::which("npx"))
        } else {
            which::which("npx")
        };
        npx.ok()
    });

    if let Some(path) = claude_path {
        agents.push(DiscoveredAgent {
            name: "Claude Code".to_string(),
            agent_type: AgentType::ClaudeCode,
            executable_path: path.to_string_lossy().to_string(),
            is_authenticated: check_claude_auth().await,
        });
    }

    agents
}

/// Check if Claude Code is authenticated by running `claude auth status`
/// and parsing the JSON output.
async fn check_claude_auth() -> bool {
    // Fast path: check API key env var
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        return true;
    }

    // Run `claude auth status` to check OAuth / claude.ai login
    let cli_path = match find_claude_cli() {
        Some(p) => p,
        None => return false,
    };

    let result = Command::new(&cli_path)
        .args(["auth", "status"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON: {"loggedIn": true, ...}
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
                return json.get("loggedIn").and_then(|v| v.as_bool()).unwrap_or(false);
            }
            false
        }
        _ => false,
    }
}
