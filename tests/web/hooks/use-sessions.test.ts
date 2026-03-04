import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useRetrySession } from '@/hooks/use-sessions';

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

describe('useRetrySession', () => {
    it('POSTs to /sessions/:id/retry', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: 'sess-1', status: 'running' });

        const { result } = renderHook(() => useRetrySession(), { wrapper: createWrapper() });
        result.current.mutate({ id: 'sess-1' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/sessions/sess-1/retry', expect.objectContaining({
            method: 'POST',
        }));
    });

    it('sends prompt when provided', async () => {
        mockApiFetch.mockResolvedValueOnce({ id: 'sess-2', status: 'running' });

        const { result } = renderHook(() => useRetrySession(), { wrapper: createWrapper() });
        result.current.mutate({ id: 'sess-2', prompt: 'Try again with different approach' });

        await waitFor(() => expect(result.current.isSuccess).toBe(true));
        expect(mockApiFetch).toHaveBeenCalledWith('/sessions/sess-2/retry', expect.objectContaining({
            method: 'POST',
            body: expect.stringContaining('Try again with different approach'),
        }));
    });
});
