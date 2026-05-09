use crate::adapters::AdapterRegistry;
use composer_api_types::AgentType;

pub struct DiscoveredAgent {
    pub name: String,
    pub agent_type: AgentType,
    pub executable_path: String,
    pub is_authenticated: bool,
}

/// Discover installed coding agents on this machine by checking all
/// registered CLI adapters for available executables.
pub async fn discover_agents(registry: &AdapterRegistry) -> Vec<DiscoveredAgent> {
    let mut agents = Vec::new();

    for adapter in registry.all() {
        if let Some(path) = adapter.find_executable() {
            agents.push(DiscoveredAgent {
                name: adapter.display_name().to_string(),
                agent_type: adapter.agent_type(),
                executable_path: path.to_string_lossy().to_string(),
                is_authenticated: adapter.check_auth().await,
            });
        }
    }

    agents
}
