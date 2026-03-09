pub mod event_bus;
pub mod task_service;
pub mod agent_service;
pub mod session_service;
pub mod worktree_service;
pub mod project_service;
pub mod workflow_engine;

use std::sync::Arc;
use composer_db::Database;
use composer_executors::process_manager::AgentProcessManager;
use crate::event_bus::EventBus;

pub struct ServiceContainer {
    pub tasks: task_service::TaskService,
    pub agents: agent_service::AgentService,
    pub sessions: session_service::SessionService,
    pub worktrees: worktree_service::WorktreeService,
    pub projects: project_service::ProjectService,
    pub workflows: workflow_engine::WorkflowEngine,
}

impl ServiceContainer {
    pub fn new(db: Arc<Database>, event_bus: EventBus, persist_rx: tokio::sync::mpsc::UnboundedReceiver<composer_api_types::WsEvent>) -> Self {
        let process_manager = Arc::new(AgentProcessManager::new(event_bus.sender(), event_bus.persist_sender()));
        let sessions = session_service::SessionService::new(db.clone(), event_bus.clone(), process_manager.clone(), persist_rx);
        let workflows = workflow_engine::WorkflowEngine::new(db.clone(), event_bus.clone(), sessions.clone());

        // Wire workflow engine into session service (breaks circular dependency)
        sessions.set_workflow_engine(workflows.clone());

        Self {
            tasks: task_service::TaskService::new(db.clone(), event_bus.clone(), sessions.clone()),
            agents: agent_service::AgentService::new(db.clone(), event_bus.clone(), process_manager),
            sessions,
            worktrees: worktree_service::WorktreeService::new(db.clone()),
            projects: project_service::ProjectService::new(db.clone(), event_bus.clone()),
            workflows,
        }
    }
}
