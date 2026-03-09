use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use crate::event_bus::EventBus;
use crate::session_service::SessionService;

#[derive(Clone)]
pub struct TaskService {
    db: Arc<Database>,
    event_bus: EventBus,
    session_service: SessionService,
}

impl TaskService {
    pub fn new(db: Arc<Database>, event_bus: EventBus, session_service: SessionService) -> Self {
        Self { db, event_bus, session_service }
    }

    pub async fn create(&self, req: CreateTaskRequest) -> anyhow::Result<Task> {
        tracing::info!(title = %req.title, "Creating task");
        let project_id_str = req.project_id.map(|id| id.to_string());
        let assigned_agent_id_str = req.assigned_agent_id.map(|id| id.to_string());
        let workflow_id_str = req.workflow_id.map(|id| id.to_string());

        // Validate related_task_ids: each must exist and be in "done" status
        let related_ids_str: Vec<String> = if let Some(ref ids) = req.related_task_ids {
            let mut validated = Vec::new();
            for id in ids {
                let id_str = id.to_string();
                let target = composer_db::models::task::find_by_id(&self.db.pool, &id_str)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Related task not found: {}", id))?;
                if target.status != TaskStatus::Done {
                    return Err(anyhow::anyhow!(
                        "Related task {} is not in done status (current: {:?})",
                        id,
                        target.status
                    ));
                }
                validated.push(id_str);
            }
            validated
        } else {
            vec![]
        };

        let task = composer_db::models::task::create(
            &self.db.pool,
            &req.title,
            req.description.as_deref(),
            req.priority,
            req.status.as_ref(),
            project_id_str.as_deref(),
            assigned_agent_id_str.as_deref(),
            workflow_id_str.as_deref(),
            &related_ids_str,
        )
        .await?;
        self.event_bus.broadcast(WsEvent::TaskCreated(task.clone()));
        Ok(task)
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Task>> {
        composer_db::models::task::list_all(&self.db.pool).await
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Task>> {
        composer_db::models::task::find_by_id(&self.db.pool, id).await
    }

    pub async fn update(&self, id: &str, req: UpdateTaskRequest) -> anyhow::Result<Task> {
        let project_id_str = req.project_id.map(|id| id.to_string());
        let assigned_agent_id_str = req.assigned_agent_id.map(|id| id.to_string());
        let workflow_id_str = req.workflow_id.map(|id| id.to_string());

        // If project_id is being changed, reassign task_number and simple_id
        if let Some(ref new_pid) = project_id_str {
            let current = composer_db::models::task::find_by_id(&self.db.pool, id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
            let current_pid = current.project_id.map(|id| id.to_string());

            if current_pid.as_deref() != Some(new_pid.as_str()) {
                composer_db::models::task::reassign_project(
                    &self.db.pool,
                    id,
                    Some(new_pid),
                )
                .await?;
            }
        }

        let task = composer_db::models::task::update(
            &self.db.pool,
            id,
            req.title.as_deref(),
            req.description.as_deref(),
            req.priority,
            req.status.as_ref(),
            req.position,
            None, // project_id handled by reassign_project above
            assigned_agent_id_str.as_deref(),
            workflow_id_str.as_deref(),
        )
        .await?;
        self.event_bus.broadcast(WsEvent::TaskUpdated(task.clone()));
        Ok(task)
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        tracing::info!(task_id = %id, "Deleting task");
        let uuid: uuid::Uuid = id.parse()?;
        composer_db::models::task::delete(&self.db.pool, id).await?;
        self.event_bus
            .broadcast(WsEvent::TaskDeleted { task_id: uuid });
        Ok(())
    }

    pub async fn assign_agent(&self, task_id: &str, agent_id: &str) -> anyhow::Result<Task> {
        tracing::info!(task_id = %task_id, agent_id = %agent_id, "Assigning agent to task");
        composer_db::models::task::update_assigned_agent(&self.db.pool, task_id, Some(agent_id))
            .await?;
        let task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        self.event_bus.broadcast(WsEvent::TaskUpdated(task.clone()));
        Ok(task)
    }

    pub async fn move_task(&self, id: &str, req: MoveTaskRequest) -> anyhow::Result<Task> {
        tracing::info!(task_id = %id, to_status = ?req.status, "Moving task");
        let old_task = composer_db::models::task::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        let from_status = old_task.status.clone();
        composer_db::models::task::update_status(&self.db.pool, id, &req.status).await?;
        if let Some(pos) = req.position {
            composer_db::models::task::update(&self.db.pool, id, None, None, None, None, Some(pos), None, None, None)
                .await?;
        }
        let task = composer_db::models::task::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        self.event_bus.broadcast(WsEvent::TaskMoved {
            task_id: task.id,
            from_status,
            to_status: req.status,
        });
        Ok(task)
    }

    pub async fn start_task(&self, task_id: &str) -> anyhow::Result<StartTaskResponse> {
        tracing::info!(task_id = %task_id, "Starting task");
        let task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        // Validate task is in backlog
        if !matches!(task.status, TaskStatus::Backlog) {
            return Err(anyhow::anyhow!("Task must be in backlog to start"));
        }

        let agent_id = task.assigned_agent_id
            .ok_or_else(|| anyhow::anyhow!("Task has no assigned agent"))?;

        // Derive repo_path from the project's primary repository
        let project_id = task.project_id
            .ok_or_else(|| anyhow::anyhow!("Task has no project assigned"))?;
        let repos = composer_db::models::project_repository::list_by_project(
            &self.db.pool,
            &project_id.to_string(),
        ).await?;
        let primary_repo = repos.iter()
            .find(|r| r.role == RepositoryRole::Primary)
            .or_else(|| repos.first())
            .ok_or_else(|| anyhow::anyhow!("Project has no repositories configured"))?;
        let repo_path = primary_repo.local_path.clone();

        // Build prompt from title + description
        // Use " - " separator instead of newlines because Windows npx.cmd
        // cannot handle newlines in batch file arguments
        let base_prompt = if let Some(ref desc) = task.description {
            format!("{} - {}", task.title, desc)
        } else {
            task.title.clone()
        };

        // Prepend project instructions if any exist
        let instructions = composer_db::models::project_instruction::list_by_project(
            &self.db.pool,
            &project_id.to_string(),
        ).await?;
        let prompt = match composer_db::models::project_instruction::format_instructions_block(&instructions) {
            Some(block) => format!("{} - {}", block, base_prompt),
            None => base_prompt,
        };

        let session = self.session_service.create_session(CreateSessionRequest {
            agent_id,
            task_id: task.id,
            prompt,
            repo_path,
            name: None,
            auto_approve: Some(task.auto_approve),
            exit_on_result: false,
        }).await?;

        // Re-fetch task after session creation (status changed to in_progress)
        let updated_task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        tracing::info!(task_id = %task_id, session_id = %session.id, "Task started successfully");

        Ok(StartTaskResponse {
            task: updated_task,
            session,
        })
    }
}
