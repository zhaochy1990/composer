import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { apiFetch } from '../api';

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

beforeEach(() => {
    mockFetch.mockReset();
});

afterEach(() => {
    vi.restoreAllMocks();
});

describe('apiFetch', () => {
    it('GET success returns parsed JSON', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            status: 200,
            text: () => Promise.resolve(JSON.stringify([{ id: '1', title: 'Test' }])),
        });

        const result = await apiFetch('/tasks');
        expect(result).toEqual([{ id: '1', title: 'Test' }]);
        expect(mockFetch).toHaveBeenCalledWith('/api/tasks', expect.objectContaining({
            headers: expect.objectContaining({}),
        }));
    });

    it('POST sets Content-Type when body present', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            status: 200,
            text: () => Promise.resolve(JSON.stringify({ id: '1' })),
        });

        await apiFetch('/tasks', {
            method: 'POST',
            body: JSON.stringify({ title: 'New' }),
        });

        const [, opts] = mockFetch.mock.calls[0];
        expect(opts.headers['Content-Type']).toBe('application/json');
    });

    it('does not set Content-Type without body', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            status: 200,
            text: () => Promise.resolve(JSON.stringify([])),
        });

        await apiFetch('/tasks');

        const [, opts] = mockFetch.mock.calls[0];
        expect(opts.headers['Content-Type']).toBeUndefined();
    });

    it('throws on 404 with error message', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: false,
            status: 404,
            statusText: 'Not Found',
            json: () => Promise.resolve({ error: 'Task not found' }),
        });

        await expect(apiFetch('/tasks/123')).rejects.toThrow('Task not found');
    });

    it('throws on 500 with status text fallback', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error',
            json: () => Promise.reject(new Error('not json')),
        });

        await expect(apiFetch('/tasks')).rejects.toThrow('500 Internal Server Error');
    });

    it('handles 204 No Content', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            status: 204,
            text: () => Promise.resolve(''),
        });

        const result = await apiFetch('/tasks/1', { method: 'DELETE' });
        expect(result).toBeUndefined();
    });

    it('handles empty response body', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: true,
            status: 200,
            text: () => Promise.resolve(''),
        });

        const result = await apiFetch('/empty');
        expect(result).toBeUndefined();
    });

    it('handles error body parsing failure gracefully', async () => {
        mockFetch.mockResolvedValueOnce({
            ok: false,
            status: 400,
            statusText: 'Bad Request',
            json: () => Promise.reject(new Error('parse error')),
        });

        await expect(apiFetch('/tasks')).rejects.toThrow('400 Bad Request');
    });
});
