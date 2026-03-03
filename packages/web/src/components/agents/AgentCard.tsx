import { X } from 'lucide-react';
import type { Agent, AgentStatus, AuthStatus } from '@/types/generated';
import { useDeleteAgent, useAgentHealth } from '@/hooks/use-agents';

const statusColors: Record<AgentStatus, string> = {
    idle: 'bg-green-500',
    busy: 'bg-yellow-500',
    error: 'bg-red-500',
    offline: 'bg-gray-500',
};

const statusLabels: Record<AgentStatus, string> = {
    idle: 'Idle',
    busy: 'Busy',
    error: 'Error',
    offline: 'Offline',
};

function authBadgeClass(authStatus: AuthStatus): string {
    switch (authStatus) {
        case 'authenticated':
            return 'bg-green-900 text-green-300';
        case 'unauthenticated':
            return 'bg-red-900 text-red-300';
        default:
            return 'bg-yellow-900 text-yellow-300';
    }
}

function authBadgeLabel(authStatus: AuthStatus): string {
    switch (authStatus) {
        case 'authenticated':
            return 'Authenticated';
        case 'unauthenticated':
            return 'Unauthenticated';
        default:
            return 'Unknown';
    }
}

interface AgentCardProps {
    agent: Agent;
}

export function AgentCard({ agent }: AgentCardProps) {
    const deleteAgent = useDeleteAgent();
    const { data: health } = useAgentHealth(agent.id);

    // Derive auth status from health check if available, else use agent record
    const currentAuth: AuthStatus = health
        ? (health.is_authenticated ? 'authenticated' : 'unauthenticated')
        : agent.auth_status;

    return (
        <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 relative group">
            <button
                onClick={() => deleteAgent.mutate(agent.id)}
                disabled={deleteAgent.isPending}
                className="absolute top-3 right-3 text-gray-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                title="Delete agent"
            >
                <X className="w-4 h-4" />
            </button>

            <div className="flex items-center gap-2 mb-2">
                <span
                    className={`w-2 h-2 rounded-full inline-block ${statusColors[agent.status]}`}
                    title={statusLabels[agent.status]}
                />
                <h3 className="font-bold text-gray-100 truncate">{agent.name}</h3>
            </div>

            <div className="flex items-center gap-2 mb-3">
                <span className="text-xs px-2 py-0.5 rounded bg-gray-800 text-gray-300">
                    {agent.agent_type}
                </span>
                <span className={`text-xs px-2 py-0.5 rounded ${authBadgeClass(currentAuth)}`}>
                    {authBadgeLabel(currentAuth)}
                </span>
                {health && !health.is_installed && (
                    <span className="text-xs px-2 py-0.5 rounded bg-red-900 text-red-300">
                        Not Installed
                    </span>
                )}
            </div>

            <div className="flex items-center justify-between text-xs text-gray-500">
                <span>{statusLabels[agent.status]}</span>
                {agent.last_heartbeat && (
                    <span title={agent.last_heartbeat}>
                        Last seen {new Date(agent.last_heartbeat).toLocaleTimeString()}
                    </span>
                )}
            </div>

            {agent.executable_path && (
                <p className="text-xs text-gray-600 mt-2 truncate" title={agent.executable_path}>
                    {agent.executable_path}
                </p>
            )}
        </div>
    );
}
