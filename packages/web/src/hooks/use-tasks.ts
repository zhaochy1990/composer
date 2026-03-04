import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
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
        mutationFn: (data: { title: string; description?: string; priority?: number; status?: string; assigned_agent_id?: string; repo_path?: string }) =>
            apiFetch<Task>('/tasks', { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
    });
}

export function useUpdateTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, ...data }: { id: string; title?: string; description?: string; priority?: number; status?: string; assigned_agent_id?: string; repo_path?: string }) =>
            apiFetch<Task>(`/tasks/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
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
    });
}

export function useDeleteTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/tasks/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
    });
}

export function useAssignTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, agentId }: { id: string; agentId: string }) =>
            apiFetch<Task>(`/tasks/${id}/assign`, { method: 'POST', body: JSON.stringify({ agent_id: agentId }) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
    });
}

export function useMoveTask() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, status, position }: { id: string; status: string; position?: number }) =>
            apiFetch<Task>(`/tasks/${id}/move`, { method: 'POST', body: JSON.stringify({ status, position }) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
    });
}
