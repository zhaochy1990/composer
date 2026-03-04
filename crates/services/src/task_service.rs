use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use crate::event_bus::EventBus;

#[derive(Clone)]
pub struct TaskService {
    db: Arc<Database>,
    event_bus: EventBus,
}

impl TaskService {
    pub fn new(db: Arc<Database>, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    pub async fn create(&self, req: CreateTaskRequest) -> anyhow::Result<Task> {
        let task = composer_db::models::task::create(
            &self.db.pool,
            &req.title,
            req.description.as_deref(),
            req.priority,
            req.status.as_ref(),
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
        let task = composer_db::models::task::update(
            &self.db.pool,
            id,
            req.title.as_deref(),
            req.description.as_deref(),
            req.priority,
            req.status.as_ref(),
            req.position,
        )
        .await?;
        self.event_bus.broadcast(WsEvent::TaskUpdated(task.clone()));
        Ok(task)
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid: uuid::Uuid = id.parse()?;
        composer_db::models::task::delete(&self.db.pool, id).await?;
        self.event_bus
            .broadcast(WsEvent::TaskDeleted { task_id: uuid });
        Ok(())
    }

    pub async fn assign_agent(&self, task_id: &str, agent_id: &str) -> anyhow::Result<Task> {
        composer_db::models::task::update_assigned_agent(&self.db.pool, task_id, Some(agent_id))
            .await?;
        let task = composer_db::models::task::find_by_id(&self.db.pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        self.event_bus.broadcast(WsEvent::TaskUpdated(task.clone()));
        Ok(task)
    }

    pub async fn move_task(&self, id: &str, req: MoveTaskRequest) -> anyhow::Result<Task> {
        let old_task = composer_db::models::task::find_by_id(&self.db.pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        let from_status = old_task.status.clone();
        composer_db::models::task::update_status(&self.db.pool, id, &req.status).await?;
        if let Some(pos) = req.position {
            composer_db::models::task::update(&self.db.pool, id, None, None, None, None, Some(pos))
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
}
