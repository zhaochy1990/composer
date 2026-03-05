use std::path::Path;
use std::sync::Arc;
use composer_api_types::*;
use composer_db::Database;
use crate::event_bus::EventBus;

#[derive(Clone)]
pub struct ProjectService {
    db: Arc<Database>,
    event_bus: EventBus,
}

impl ProjectService {
    pub fn new(db: Arc<Database>, event_bus: EventBus) -> Self {
        Self { db, event_bus }
    }

    // ---- Project CRUD ----

    pub async fn create(&self, req: CreateProjectRequest) -> anyhow::Result<Project> {
        let project = composer_db::models::project::create(
            &self.db.pool,
            &req.name,
            req.description.as_deref(),
        )
        .await?;
        self.event_bus.broadcast(WsEvent::ProjectCreated(project.clone()));
        Ok(project)
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Project>> {
        composer_db::models::project::list_all(&self.db.pool).await
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<Project>> {
        composer_db::models::project::find_by_id(&self.db.pool, id).await
    }

    pub async fn update(&self, id: &str, req: UpdateProjectRequest) -> anyhow::Result<Project> {
        let project = composer_db::models::project::update(
            &self.db.pool,
            id,
            req.name.as_deref(),
            req.description.as_deref(),
        )
        .await?;
        self.event_bus.broadcast(WsEvent::ProjectUpdated(project.clone()));
        Ok(project)
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let uuid: uuid::Uuid = id.parse()?;
        composer_db::models::project::delete(&self.db.pool, id).await?;
        self.event_bus.broadcast(WsEvent::ProjectDeleted { project_id: uuid });
        Ok(())
    }

    // ---- Repository sub-resource ----

    pub async fn add_repository(
        &self,
        project_id: &str,
        req: AddProjectRepositoryRequest,
    ) -> anyhow::Result<ProjectRepository> {
        // Validate project exists
        composer_db::models::project::find_by_id(&self.db.pool, project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

        // Validate path is absolute and exists
        let path = Path::new(&req.local_path);
        if !path.is_absolute() {
            anyhow::bail!("local_path must be an absolute path");
        }
        if !path.exists() {
            anyhow::bail!("local_path does not exist: {}", req.local_path);
        }
        // Validate it's a git repo
        if !path.join(".git").exists() {
            anyhow::bail!("local_path is not a git repository: {}", req.local_path);
        }

        let repo = composer_db::models::project_repository::create(
            &self.db.pool,
            project_id,
            &req.local_path,
            req.remote_url.as_deref(),
            req.role.as_ref(),
            req.display_name.as_deref(),
        )
        .await?;

        let pid: uuid::Uuid = project_id.parse()?;
        self.event_bus.broadcast(WsEvent::ProjectRepositoryAdded {
            project_id: pid,
            repository: repo.clone(),
        });
        Ok(repo)
    }

    pub async fn list_repositories(&self, project_id: &str) -> anyhow::Result<Vec<ProjectRepository>> {
        composer_db::models::project_repository::list_by_project(&self.db.pool, project_id).await
    }

    pub async fn update_repository(
        &self,
        project_id: &str,
        repo_id: &str,
        req: UpdateProjectRepositoryRequest,
    ) -> anyhow::Result<ProjectRepository> {
        let repo = composer_db::models::project_repository::update(
            &self.db.pool,
            project_id,
            repo_id,
            req.local_path.as_deref(),
            req.remote_url.as_deref(),
            req.role.as_ref(),
            req.display_name.as_deref(),
        )
        .await?;
        Ok(repo)
    }

    pub async fn remove_repository(&self, project_id: &str, repo_id: &str) -> anyhow::Result<()> {
        let pid: uuid::Uuid = project_id.parse()?;
        let rid: uuid::Uuid = repo_id.parse()?;
        composer_db::models::project_repository::delete(&self.db.pool, project_id, repo_id).await?;
        self.event_bus.broadcast(WsEvent::ProjectRepositoryRemoved {
            project_id: pid,
            repository_id: rid,
        });
        Ok(())
    }

    pub async fn list_tasks(&self, project_id: &str) -> anyhow::Result<Vec<Task>> {
        composer_db::models::task::list_by_project(&self.db.pool, project_id).await
    }

    // ---- Instruction sub-resource ----

    pub async fn add_instruction(
        &self,
        project_id: &str,
        req: AddProjectInstructionRequest,
    ) -> anyhow::Result<ProjectInstruction> {
        let title = req.title.trim().to_string();
        let content = req.content.trim().to_string();
        if title.is_empty() {
            anyhow::bail!("Instruction title cannot be empty");
        }
        if content.is_empty() {
            anyhow::bail!("Instruction content cannot be empty");
        }

        composer_db::models::project::find_by_id(&self.db.pool, project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

        let instruction = composer_db::models::project_instruction::create(
            &self.db.pool,
            project_id,
            &title,
            &content,
            req.sort_order,
        )
        .await?;

        let pid: uuid::Uuid = project_id.parse()?;
        self.event_bus.broadcast(WsEvent::ProjectInstructionAdded {
            project_id: pid,
            instruction: instruction.clone(),
        });
        Ok(instruction)
    }

    pub async fn list_instructions(&self, project_id: &str) -> anyhow::Result<Vec<ProjectInstruction>> {
        composer_db::models::project_instruction::list_by_project(&self.db.pool, project_id).await
    }

    pub async fn update_instruction(
        &self,
        project_id: &str,
        instruction_id: &str,
        req: UpdateProjectInstructionRequest,
    ) -> anyhow::Result<ProjectInstruction> {
        if let Some(ref t) = req.title {
            if t.trim().is_empty() {
                anyhow::bail!("Instruction title cannot be empty");
            }
        }
        if let Some(ref c) = req.content {
            if c.trim().is_empty() {
                anyhow::bail!("Instruction content cannot be empty");
            }
        }

        let instruction = composer_db::models::project_instruction::update(
            &self.db.pool,
            project_id,
            instruction_id,
            req.title.as_deref().map(str::trim),
            req.content.as_deref().map(str::trim),
            req.sort_order,
        )
        .await?;

        let pid: uuid::Uuid = project_id.parse()?;
        self.event_bus.broadcast(WsEvent::ProjectInstructionUpdated {
            project_id: pid,
            instruction: instruction.clone(),
        });
        Ok(instruction)
    }

    pub async fn remove_instruction(&self, project_id: &str, instruction_id: &str) -> anyhow::Result<()> {
        let pid: uuid::Uuid = project_id.parse()?;
        let iid: uuid::Uuid = instruction_id.parse()?;
        composer_db::models::project_instruction::delete(&self.db.pool, project_id, instruction_id).await?;
        self.event_bus.broadcast(WsEvent::ProjectInstructionRemoved {
            project_id: pid,
            instruction_id: iid,
        });
        Ok(())
    }
}
