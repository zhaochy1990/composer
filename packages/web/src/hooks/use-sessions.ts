import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import { logger } from '@/lib/logger';
import type { Session, SessionLog, CreateSessionRequest } from '@/types/generated';

export function useSession(id: string | undefined) {
    return useQuery({
        queryKey: ['sessions', id],
        queryFn: () => apiFetch<Session>(`/sessions/${id}`),
        enabled: !!id,
        // Fix #27: Stop polling for terminal session states
        refetchInterval: (query) => {
            const status = query.state.data?.status;
            if (status === 'completed' || status === 'failed') return false;
            return 5_000;
        },
    });
}

export function useCreateSession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: CreateSessionRequest) =>
            apiFetch<Session>('/sessions', {
                method: 'POST',
                body: JSON.stringify(data),
            }),
        onSuccess: (_session, variables) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['tasks', variables.task_id, 'sessions'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
        },
        onError: (error: Error) => logger.error('Failed to create session', { error: error.message }),
    });
}

export function useInterruptSession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<Session>(`/sessions/${id}/interrupt`, { method: 'POST' }),
        onSuccess: (_data, id) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['sessions', id] });
        },
        onError: (error: Error) => logger.error('Failed to interrupt session', { error: error.message }),
    });
}

export function useResumeSession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, prompt, continueChat }: { id: string; prompt?: string; continueChat?: boolean }) =>
            apiFetch<Session>(`/sessions/${id}/resume`, {
                method: 'POST',
                body: JSON.stringify({ prompt, continue_chat: continueChat }),
            }),
        onSuccess: (_data, { id }) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['sessions', id] });
        },
        onError: (error: Error) => logger.error('Failed to resume session', { error: error.message }),
    });
}

export function useSendSessionInput() {
    return useMutation({
        mutationFn: ({ id, message }: { id: string; message: string }) =>
            apiFetch<void>(`/sessions/${id}/input`, {
                method: 'POST',
                body: JSON.stringify({ message }),
            }),
        onError: (error: Error) => logger.error('Failed to send session input', { error: error.message }),
    });
}

export function useCompleteSession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<Session>(`/sessions/${id}/complete`, { method: 'POST' }),
        onSuccess: (_data, id) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['sessions', id] });
        },
        onError: (error: Error) => logger.error('Failed to complete session', { error: error.message }),
    });
}

export function useRetrySession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, prompt }: { id: string; prompt?: string }) =>
            apiFetch<Session>(`/sessions/${id}/retry`, {
                method: 'POST',
                body: JSON.stringify({ prompt }),
            }),
        onSuccess: (_data, { id }) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['sessions', id] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
        },
        onError: (error: Error) => logger.error('Failed to retry session', { error: error.message }),
    });
}

export function useSessionLogs(id: string | undefined) {
    return useQuery({
        queryKey: ['sessions', id, 'logs'],
        queryFn: () => apiFetch<SessionLog[]>(`/sessions/${id}/logs?limit=5000`),
        enabled: !!id,
    });
}
