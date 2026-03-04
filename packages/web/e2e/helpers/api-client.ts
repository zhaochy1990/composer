const API_BASE = 'http://localhost:3000/api';

interface Task {
  id: string;
  title: string;
  description: string | null;
  priority: number;
  status: string;
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

  async createTask(data: { title: string; description?: string; priority?: number; status?: string }): Promise<Task> {
    return this.fetch<Task>('/tasks', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listTasks(): Promise<Task[]> {
    return this.fetch<Task[]>('/tasks');
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

  // --- Cleanup ---

  async resetAllData(): Promise<void> {
    const [tasks, agents] = await Promise.all([this.listTasks(), this.listAgents()]);
    await Promise.all([
      ...tasks.map((t) => this.deleteTask(t.id)),
      ...agents.map((a) => this.deleteAgent(a.id)),
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
