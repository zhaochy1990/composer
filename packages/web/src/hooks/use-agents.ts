import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import { logger } from '@/lib/logger';
import type { Agent, AgentHealth } from '@/types/generated';

export function useAgents() {
    return useQuery({
        queryKey: ['agents'],
        queryFn: () => apiFetch<Agent[]>('/agents'),
    });
}

export function useCreateAgent() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: { name: string; agent_type: string }) =>
            apiFetch<Agent>('/agents', {
                method: 'POST',
                body: JSON.stringify(data),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
        onError: (error: Error) => logger.error('Failed to create agent', { error: error.message }),
    });
}

export function useDeleteAgent() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/agents/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
        onError: (error: Error) => logger.error('Failed to delete agent', { error: error.message }),
    });
}

export function useDiscoverAgents() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: () => apiFetch<Agent[]>('/agents/discover', { method: 'POST' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['agents'] }),
        onError: (error: Error) => logger.error('Failed to discover agents', { error: error.message }),
    });
}

export function useAgentHealth(agentId: string) {
    return useQuery({
        queryKey: ['agents', agentId, 'health'],
        queryFn: () => apiFetch<AgentHealth>(`/agents/${agentId}/health`),
        enabled: !!agentId,
        refetchInterval: 30_000,
    });
}
