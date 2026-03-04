import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import {
    useProjects,
    useProject,
    useCreateProject,
    useUpdateProject,
    useDeleteProject,
    useProjectRepositories,
    useAddProjectRepository,
    useRemoveProjectRepository,
} from '@/hooks/use-projects';

vi.mock('@/lib/api', () => ({
    apiFetch: vi.fn(),
}));

import { apiFetch } from '@/lib/api';
const mockApiFetch = vi.mocked(apiFetch);

function createWrapper() {
    const queryClient = new QueryClient({
        defaultOptions: {
            queries: { retry: false, gcTime: 0 },
            mutations: { retry: false },
        },
    });
    return ({ children }: { children: React.ReactNode }) =>
        createElement(QueryClientProvider, { client: queryClient }, children);
}

beforeEach(() => {
    mockApiFetch.mockReset();
});

describe('useProjects', () => {
    it('fetches projects from /projects', async () => {
        const projects = [{ id: '1', name: 'Project 1' }];
        mockApiFetch.mockResolvedValueOnce(projects);

        const { result } = renderHook(() => useProjects(), { wrapper: createWrapper() });
        await waitFor(() => expect(result.current.isSuccess).toBe(true));

        expect(result.current.data).toEqual(projects);
        expect(mockApiFetch).toHaveBeenCalledWith('/projects');
    });
});

describe('useProject', () => {
    it('fetches single project by id', async () => {
        const project = { id: 'p1', name: 'My Project' };
        mockApiFetch.mockResolvedValueOnce(project);

        const { result } = renderHook(() => useProject('p1'), { wrapper: createWrapper() });
        await waitFor(() => expect(result.current.isSuccess).toBe(true));

        expect(result.current.data).toEqual(project);
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1');
    });
});

describe('useCreateProject', () => {
    it('POSTs to /projects', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: '2', name: 'New' });

        const { result } = renderHook(() => useCreateProject(), { wrapper: createWrapper() });
        result.current.mutate({ name: 'New' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/projects', expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('"New"'),
        }));
    });
});

describe('useUpdateProject', () => {
    it('PUTs to /projects/:id', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: 'p1', name: 'Updated' });

        const { result } = renderHook(() => useUpdateProject(), { wrapper: createWrapper() });
        result.current.mutate({ id: 'p1', name: 'Updated' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1', expect.objectContaining({
            method: 'PUT',
        }));
    });
});

describe('useDeleteProject', () => {
    it('DELETEs project by id', async () => {
        mockApiFetch.mockResolvedValueOnce(undefined);

        const { result } = renderHook(() => useDeleteProject(), { wrapper: createWrapper() });
        result.current.mutate('p1');

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1', expect.objectContaining({
            method: 'DELETE',
        }));
    });
});

describe('useProjectRepositories', () => {
    it('fetches repositories for a project', async () => {
        const repos = [{ id: 'r1', project_id: 'p1', local_path: '/tmp' }];
        mockApiFetch.mockResolvedValueOnce(repos);

        const { result } = renderHook(() => useProjectRepositories('p1'), { wrapper: createWrapper() });
        await waitFor(() => expect(result.current.isSuccess).toBe(true));

        expect(result.current.data).toEqual(repos);
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1/repositories');
    });
});

describe('useAddProjectRepository', () => {
    it('POSTs to /projects/:id/repositories', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: 'r1', project_id: 'p1', local_path: '/repo' });

        const { result } = renderHook(() => useAddProjectRepository(), { wrapper: createWrapper() });
        result.current.mutate({ projectId: 'p1', local_path: '/repo' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1/repositories', expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('/repo'),
        }));
    });
});

describe('useRemoveProjectRepository', () => {
    it('DELETEs repository from project', async () => {
        mockApiFetch.mockResolvedValueOnce(undefined);

        const { result } = renderHook(() => useRemoveProjectRepository(), { wrapper: createWrapper() });
        result.current.mutate({ projectId: 'p1', repoId: 'r1' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/projects/p1/repositories/r1', expect.objectContaining({
            method: 'DELETE',
        }));
    });
});
