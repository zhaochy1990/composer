import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import { logger } from '@/lib/logger';
import type { Project, ProjectInstruction, ProjectRepository } from '@/types/generated';

export function useProjects() {
    return useQuery({
        queryKey: ['projects'],
        queryFn: () => apiFetch<Project[]>('/projects'),
    });
}

export function useProject(id: string) {
    return useQuery({
        queryKey: ['projects', id],
        queryFn: () => apiFetch<Project>(`/projects/${id}`),
        enabled: !!id,
    });
}

export function useCreateProject() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: { name: string; description?: string }) =>
            apiFetch<Project>('/projects', { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['projects'] }),
        onError: (error: Error) => logger.error('Failed to create project', { error: error.message }),
    });
}

export function useUpdateProject() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, ...data }: { id: string; name?: string; description?: string }) =>
            apiFetch<Project>(`/projects/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['projects'] }),
        onError: (error: Error) => logger.error('Failed to update project', { error: error.message }),
    });
}

export function useDeleteProject() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/projects/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['projects'] }),
        onError: (error: Error) => logger.error('Failed to delete project', { error: error.message }),
    });
}

export function useProjectRepositories(projectId: string) {
    return useQuery({
        queryKey: ['projects', projectId, 'repositories'],
        queryFn: () => apiFetch<ProjectRepository[]>(`/projects/${projectId}/repositories`),
        enabled: !!projectId,
    });
}

export function useAddProjectRepository() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ projectId, ...data }: { projectId: string; local_path: string; remote_url?: string; role?: string; display_name?: string }) =>
            apiFetch<ProjectRepository>(`/projects/${projectId}/repositories`, { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: (_data, variables) =>
            queryClient.invalidateQueries({ queryKey: ['projects', variables.projectId, 'repositories'] }),
        onError: (error: Error) => logger.error('Failed to add repository', { error: error.message }),
    });
}

export function useRemoveProjectRepository() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ projectId, repoId }: { projectId: string; repoId: string }) =>
            apiFetch<void>(`/projects/${projectId}/repositories/${repoId}`, { method: 'DELETE' }),
        onSuccess: (_data, variables) =>
            queryClient.invalidateQueries({ queryKey: ['projects', variables.projectId, 'repositories'] }),
        onError: (error: Error) => logger.error('Failed to remove repository', { error: error.message }),
    });
}

export function useProjectInstructions(projectId: string) {
    return useQuery({
        queryKey: ['projects', projectId, 'instructions'],
        queryFn: () => apiFetch<ProjectInstruction[]>(`/projects/${projectId}/instructions`),
        enabled: !!projectId,
    });
}

export function useAddProjectInstruction() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ projectId, ...data }: { projectId: string; title: string; content: string; sort_order?: number }) =>
            apiFetch<ProjectInstruction>(`/projects/${projectId}/instructions`, { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: (_data, variables) =>
            queryClient.invalidateQueries({ queryKey: ['projects', variables.projectId, 'instructions'] }),
        onError: (error: Error) => logger.error('Failed to add instruction', { error: error.message }),
    });
}

export function useUpdateProjectInstruction() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ projectId, instructionId, ...data }: { projectId: string; instructionId: string; title?: string; content?: string; sort_order?: number }) =>
            apiFetch<ProjectInstruction>(`/projects/${projectId}/instructions/${instructionId}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: (_data, variables) =>
            queryClient.invalidateQueries({ queryKey: ['projects', variables.projectId, 'instructions'] }),
        onError: (error: Error) => logger.error('Failed to update instruction', { error: error.message }),
    });
}

export function useRemoveProjectInstruction() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ projectId, instructionId }: { projectId: string; instructionId: string }) =>
            apiFetch<void>(`/projects/${projectId}/instructions/${instructionId}`, { method: 'DELETE' }),
        onSuccess: (_data, variables) =>
            queryClient.invalidateQueries({ queryKey: ['projects', variables.projectId, 'instructions'] }),
        onError: (error: Error) => logger.error('Failed to remove instruction', { error: error.message }),
    });
}
