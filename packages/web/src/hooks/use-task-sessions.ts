import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import type { Session } from '@/types/generated';

export function useTaskSessions(taskId: string | undefined) {
    return useQuery({
        queryKey: ['tasks', taskId, 'sessions'],
        queryFn: () => apiFetch<Session[]>(`/tasks/${taskId}/sessions`),
        enabled: !!taskId,
        refetchInterval: 10_000,
    });
}
