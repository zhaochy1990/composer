import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useTasks, useCreateTask, useDeleteTask, useMoveTask, useAssignTask } from '../use-tasks';

// Mock apiFetch
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

describe('useTasks', () => {
    it('fetches tasks from /tasks', async () => {
        const tasks = [{ id: '1', title: 'Task 1' }];
        mockApiFetch.mockResolvedValueOnce(tasks);

        const { result } = renderHook(() => useTasks(), { wrapper: createWrapper() });
        await waitFor(() => expect(result.current.isSuccess).toBe(true));

        expect(result.current.data).toEqual(tasks);
        expect(mockApiFetch).toHaveBeenCalledWith('/tasks');
    });
});

describe('useCreateTask', () => {
    it('POSTs to /tasks', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: '2', title: 'New' });

        const { result } = renderHook(() => useCreateTask(), { wrapper: createWrapper() });
        result.current.mutate({ title: 'New' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/tasks', expect.objectContaining({
            method: 'POST',
        }));
    });
});

describe('useDeleteTask', () => {
    it('DELETEs task by id', async () => {
        mockApiFetch.mockResolvedValueOnce(undefined);

        const { result } = renderHook(() => useDeleteTask(), { wrapper: createWrapper() });
        result.current.mutate('task-1');

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/tasks/task-1', expect.objectContaining({
            method: 'DELETE',
        }));
    });
});

describe('useMoveTask', () => {
    it('POSTs move payload', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: '1', status: 'done' });

        const { result } = renderHook(() => useMoveTask(), { wrapper: createWrapper() });
        result.current.mutate({ id: '1', status: 'done', position: 2.0 });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/tasks/1/move', expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('"done"'),
        }));
    });
});

describe('useAssignTask', () => {
    it('POSTs assign payload', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: '1', assigned_agent_id: 'agent-1' });

        const { result } = renderHook(() => useAssignTask(), { wrapper: createWrapper() });
        result.current.mutate({ id: '1', agentId: 'agent-1' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/tasks/1/assign', expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('agent-1'),
        }));
    });
});
