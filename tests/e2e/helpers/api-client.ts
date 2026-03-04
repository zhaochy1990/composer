const API_BASE = 'http://localhost:3000/api';

interface Task {
  id: string;
  title: string;
  description: string | null;
  priority: number;
  status: string;
  assigned_agent_id: string | null;
  project_id: string | null;
  auto_approve: boolean;
}

interface Agent {
  id: string;
  name: string;
  agent_type: string;
  status: string;
}

interface Session {
  id: string;
  agent_id: string;
  status: string;
}

interface Project {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

interface ProjectRepository {
  id: string;
  project_id: string;
  local_path: string;
  remote_url: string | null;
  role: string;
  display_name: string | null;
}

export class ApiClient {
  private async fetch<T>(path: string, options?: RequestInit): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`, {
      headers: { 'Content-Type': 'application/json', ...options?.headers },
      ...options,
    });
    if (!res.ok) {
      throw new Error(`API ${options?.method ?? 'GET'} ${path}: ${res.status} ${res.statusText}`);
    }
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text);
  }

  // --- Tasks ---

  async createTask(data: { title: string; description?: string; priority?: number; status?: string; project_id?: string; assigned_agent_id?: string }): Promise<Task> {
    return this.fetch<Task>('/tasks', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listTasks(): Promise<Task[]> {
    return this.fetch<Task[]>('/tasks');
  }

  async getTask(id: string): Promise<Task> {
    return this.fetch<Task>(`/tasks/${id}`);
  }

  async updateTask(id: string, data: Partial<{ title: string; description: string; priority: number; status: string; position: number; project_id: string; assigned_agent_id: string }>): Promise<Task> {
    return this.fetch<Task>(`/tasks/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async startTask(id: string): Promise<{ task: Task; session: Session }> {
    return this.fetch<{ task: Task; session: Session }>(`/tasks/${id}/start`, {
      method: 'POST',
    });
  }

  async moveTask(id: string, status: string, position?: number): Promise<Task> {
    return this.fetch<Task>(`/tasks/${id}/move`, {
      method: 'POST',
      body: JSON.stringify({ status, position }),
    });
  }

  async deleteTask(id: string): Promise<void> {
    return this.fetch<void>(`/tasks/${id}`, { method: 'DELETE' });
  }

  // --- Agents ---

  async createAgent(data: { name: string; agent_type?: string }): Promise<Agent> {
    return this.fetch<Agent>('/agents', {
      method: 'POST',
      body: JSON.stringify({ agent_type: 'claude_code', ...data }),
    });
  }

  async listAgents(): Promise<Agent[]> {
    return this.fetch<Agent[]>('/agents');
  }

  async getAgent(id: string): Promise<Agent> {
    return this.fetch<Agent>(`/agents/${id}`);
  }

  async deleteAgent(id: string): Promise<void> {
    return this.fetch<void>(`/agents/${id}`, { method: 'DELETE' });
  }

  // --- Sessions ---

  async createSession(data: { agent_id: string; task_id: string; prompt: string; repo_path: string; auto_approve?: boolean }): Promise<Session> {
    return this.fetch<Session>('/sessions', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listSessions(): Promise<Session[]> {
    return this.fetch<Session[]>('/sessions');
  }

  async listTaskSessions(taskId: string): Promise<Session[]> {
    return this.fetch<Session[]>(`/tasks/${taskId}/sessions`);
  }

  // --- Projects ---

  async createProject(data: { name: string; description?: string }): Promise<Project> {
    return this.fetch<Project>('/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listProjects(): Promise<Project[]> {
    return this.fetch<Project[]>('/projects');
  }

  async getProject(id: string): Promise<Project> {
    return this.fetch<Project>(`/projects/${id}`);
  }

  async updateProject(id: string, data: { name?: string; description?: string }): Promise<Project> {
    return this.fetch<Project>(`/projects/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async deleteProject(id: string): Promise<void> {
    return this.fetch<void>(`/projects/${id}`, { method: 'DELETE' });
  }

  async addProjectRepository(
    projectId: string,
    data: { local_path: string; remote_url?: string; role?: string; display_name?: string },
  ): Promise<ProjectRepository> {
    return this.fetch<ProjectRepository>(`/projects/${projectId}/repositories`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listProjectRepositories(projectId: string): Promise<ProjectRepository[]> {
    return this.fetch<ProjectRepository[]>(`/projects/${projectId}/repositories`);
  }

  async removeProjectRepository(projectId: string, repoId: string): Promise<void> {
    return this.fetch<void>(`/projects/${projectId}/repositories/${repoId}`, { method: 'DELETE' });
  }

  async listProjectTasks(projectId: string): Promise<Task[]> {
    return this.fetch<Task[]>(`/projects/${projectId}/tasks`);
  }

  // --- Cleanup ---

  async resetAllData(): Promise<void> {
    const [tasks, agents, projects] = await Promise.all([
      this.listTasks(),
      this.listAgents(),
      this.listProjects(),
    ]);
    await Promise.all([
      ...tasks.map((t) => this.deleteTask(t.id)),
      ...agents.map((a) => this.deleteAgent(a.id)),
      ...projects.map((p) => this.deleteProject(p.id)),
    ]);
  }

  // --- Health ---

  async waitForHealthy(timeoutMs = 30_000): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      try {
        const res = await fetch(`${API_BASE}/health`);
        if (res.ok) return;
      } catch {
        // server not ready yet
      }
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error(`Server not healthy after ${timeoutMs}ms`);
  }
}
