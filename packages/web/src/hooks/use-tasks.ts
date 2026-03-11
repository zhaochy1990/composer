import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import { logger } from '@/lib/logger';
import type { Task, StartTaskResponse } from '@/types/generated';

export function useTasks() {
    return useQuery({
        queryKey: ['tasks'],
        queryFn: () => apiFetch<Task[]>('/tasks'),
    });
}

export function useCreateTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: { title: string; description?: string; priority?: number; status?: string; project_id?: string; assigned_agent_id?: string; workflow_id?: string; related_task_ids?: string[] }) =>
            apiFetch<Task>('/tasks', { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        onError: (error: Error) => logger.error('Failed to create task', { error: error.message }),
    });
}

export function useCloneTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (sourceTask: Task) =>
            apiFetch<Task>('/tasks', {
                method: 'POST',
                body: JSON.stringify({
                    title: `${sourceTask.title} (Copy)`,
                    description: sourceTask.description || undefined,
                    priority: sourceTask.priority,
                    status: 'backlog',
                    project_id: sourceTask.project_id || undefined,
                    assigned_agent_id: sourceTask.assigned_agent_id || undefined,
                    workflow_id: sourceTask.workflow_id || undefined,
                }),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        onError: (error: Error) => logger.error('Failed to clone task', { error: error.message }),
    });
}

export function useUpdateTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, ...data }: { id: string; title?: string; description?: string; priority?: number; status?: string; assigned_agent_id?: string; project_id?: string; workflow_id?: string }) =>
            apiFetch<Task>(`/tasks/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        onError: (error: Error) => logger.error('Failed to update task', { error: error.message }),
    });
}

export function useStartTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (taskId: string) =>
            apiFetch<StartTaskResponse>(`/tasks/${taskId}/start`, { method: 'POST' }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
        onError: (error: Error) => logger.error('Failed to start task', { error: error.message }),
    });
}

export function useDeleteTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/tasks/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        onError: (error: Error) => logger.error('Failed to delete task', { error: error.message }),
    });
}

export function useAssignTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, agentId }: { id: string; agentId: string }) =>
            apiFetch<Task>(`/tasks/${id}/assign`, { method: 'POST', body: JSON.stringify({ agent_id: agentId }) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        onError: (error: Error) => logger.error('Failed to assign task', { error: error.message }),
    });
}

export function useMoveTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, status, position }: { id: string; status: string; position?: number }) =>
            apiFetch<Task>(`/tasks/${id}/move`, { method: 'POST', body: JSON.stringify({ status, position }) }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['agents'] });
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['worktrees'] });
        },
        onError: (error: Error) => logger.error('Failed to move task', { error: error.message }),
    });
}
