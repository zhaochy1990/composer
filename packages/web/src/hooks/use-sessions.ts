import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import type { Session, SessionLog, CreateSessionRequest } from '@/types/generated';

export function useSessions() {
    return useQuery({
        queryKey: ['sessions'],
        queryFn: () => apiFetch<Session[]>('/sessions'),
        refetchInterval: 10_000,
    });
}

export function useSession(id: string | undefined) {
    return useQuery({
        queryKey: ['sessions', id],
        queryFn: () => apiFetch<Session>(`/sessions/${id}`),
        enabled: !!id,
        refetchInterval: 5_000,
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
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
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
    });
}

export function useResumeSession() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, prompt }: { id: string; prompt?: string }) =>
            apiFetch<Session>(`/sessions/${id}/resume`, {
                method: 'POST',
                body: JSON.stringify({ prompt }),
            }),
        onSuccess: (_data, { id }) => {
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
            queryClient.invalidateQueries({ queryKey: ['sessions', id] });
        },
    });
}

export function useSessionLogs(id: string | undefined) {
    return useQuery({
        queryKey: ['sessions', id, 'logs'],
        queryFn: () => apiFetch<SessionLog[]>(`/sessions/${id}/logs`),
        enabled: !!id,
    });
}
