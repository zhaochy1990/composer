import { logger } from './logger';

const BASE_URL = '/api';

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
    const headers: Record<string, string> = {};
    // Only set Content-Type when there's a body to send
    if (options?.body) {
        headers['Content-Type'] = 'application/json';
    }

    const res = await fetch(`${BASE_URL}${path}`, {
        headers: { ...headers, ...options?.headers },
        ...options,
    });
    if (!res.ok) {
        let message = `${res.status} ${res.statusText}`;
        try {
            const body = await res.json();
            if (body?.error) message = body.error;
        } catch {
            logger.warn('Failed to parse API error body', { path });
        }
        logger.error(`API error: ${message}`, { path, status: res.status });
        throw new Error(message);
    }
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text);
}
