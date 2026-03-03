pub mod event_bus;
pub mod task_service;
pub mod agent_service;
pub mod session_service;
pub mod worktree_service;

use std::sync::Arc;
use composer_db::Database;
use crate::event_bus::EventBus;

pub struct ServiceContainer {
    pub tasks: task_service::TaskService,
    pub agents: agent_service::AgentService,
    pub sessions: session_service::SessionService,
    pub worktrees: worktree_service::WorktreeService,
}

impl ServiceContainer {
    pub fn new(db: Arc<Database>, event_bus: EventBus) -> Self {
        Self {
            tasks: task_service::TaskService::new(db.clone(), event_bus.clone()),
            agents: agent_service::AgentService::new(db.clone(), event_bus.clone()),
            sessions: session_service::SessionService::new(db.clone(), event_bus.clone()),
            worktrees: worktree_service::WorktreeService::new(db.clone()),
        }
    }
}
