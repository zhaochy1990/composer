import { useState } from 'react';
import { Plus, Terminal, RefreshCw } from 'lucide-react';
import { useSessions } from '@/hooks/use-sessions';
import { useAgents } from '@/hooks/use-agents';
import type { Session } from '@/types/generated';
import { StatusBadge } from './StatusBadge';
import { SessionDetail } from './SessionDetail';
import { SessionCreateDialog } from './SessionCreateDialog';
import { shortId, formatDuration, formatTime } from '@/lib/utils';

export function SessionList() {
    const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
        null,
    );
    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const {
        data: sessions,
        isLoading,
        error,
        refetch,
        isFetching,
    } = useSessions();
    const { data: agents } = useAgents();

    // Build a lookup of agent ID -> agent name for display
    const agentNameMap = new Map<string, string>();
    if (agents) {
        for (const agent of agents) {
            agentNameMap.set(agent.id, agent.name);
        }
    }

    // Show detail view if a session is selected
    if (selectedSessionId) {
        return (
            <SessionDetail
                sessionId={selectedSessionId}
                onBack={() => setSelectedSessionId(null)}
            />
        );
    }

    // Sort sessions: running first, then by created_at descending
    const sortedSessions = [...(sessions ?? [])].sort((a, b) => {
        const statusOrder: Record<string, number> = {
            running: 0,
            paused: 1,
            created: 2,
            failed: 3,
            completed: 4,
        };
        const orderA = statusOrder[a.status] ?? 5;
        const orderB = statusOrder[b.status] ?? 5;
        if (orderA !== orderB) return orderA - orderB;
        return (
            new Date(b.created_at).getTime() -
            new Date(a.created_at).getTime()
        );
    });

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
                <div className="flex items-center gap-3">
                    <Terminal className="w-5 h-5 text-gray-500" />
                    <h1 className="text-lg font-semibold text-gray-100">
                        Sessions
                    </h1>
                    {sessions && (
                        <span className="text-xs text-gray-500 bg-gray-800 px-2 py-0.5 rounded-full">
                            {sessions.length}
                        </span>
                    )}
                </div>
                <div className="flex items-center gap-2">
                    <button
                        type="button"
                        onClick={() => refetch()}
                        disabled={isFetching}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm text-gray-400 hover:text-gray-200 hover:bg-gray-800 transition-colors disabled:opacity-50"
                        title="Refresh"
                    >
                        <RefreshCw
                            className={`w-3.5 h-3.5 ${isFetching ? 'animate-spin' : ''}`}
                        />
                    </button>
                    <button
                        type="button"
                        onClick={() => setCreateDialogOpen(true)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-500 transition-colors"
                    >
                        <Plus className="w-3.5 h-3.5" />
                        New Session
                    </button>
                </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto">
                {isLoading && (
                    <div className="flex items-center justify-center py-20">
                        <p className="text-gray-500">Loading sessions...</p>
                    </div>
                )}

                {error && (
                    <div className="flex items-center justify-center py-20">
                        <p className="text-red-400">
                            Failed to load sessions: {error.message}
                        </p>
                    </div>
                )}

                {!isLoading && !error && sortedSessions.length === 0 && (
                    <div className="flex flex-col items-center justify-center py-20 gap-3">
                        <Terminal className="w-10 h-10 text-gray-700" />
                        <p className="text-gray-500">No sessions yet</p>
                        <button
                            type="button"
                            onClick={() => setCreateDialogOpen(true)}
                            className="text-sm text-blue-400 hover:text-blue-300 transition-colors"
                        >
                            Create your first session
                        </button>
                    </div>
                )}

                {!isLoading && !error && sortedSessions.length > 0 && (
                    <table className="w-full">
                        <thead>
                            <tr className="border-b border-gray-800 text-left">
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Session
                                </th>
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Agent
                                </th>
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Task
                                </th>
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Status
                                </th>
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Duration
                                </th>
                                <th className="px-6 py-3 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                                    Created
                                </th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-800/50">
                            {sortedSessions.map((session) => (
                                <SessionRow
                                    key={session.id}
                                    session={session}
                                    agentName={agentNameMap.get(session.agent_id)}
                                    onClick={() =>
                                        setSelectedSessionId(session.id)
                                    }
                                />
                            ))}
                        </tbody>
                    </table>
                )}
            </div>

            {/* Create dialog */}
            <SessionCreateDialog
                open={createDialogOpen}
                onOpenChange={setCreateDialogOpen}
            />
        </div>
    );
}

interface SessionRowProps {
    session: Session;
    agentName?: string;
    onClick: () => void;
}

function SessionRow({ session, agentName, onClick }: SessionRowProps) {
    const duration = formatDuration(session.started_at, session.completed_at);
    const created = formatTime(session.created_at);

    return (
        <tr
            onClick={onClick}
            className="hover:bg-gray-800/50 cursor-pointer transition-colors"
        >
            <td className="px-6 py-3">
                <span className="font-mono text-sm text-gray-300">
                    {shortId(session.id)}
                </span>
            </td>
            <td className="px-6 py-3">
                <span className="text-sm text-gray-300">
                    {agentName ?? shortId(session.agent_id)}
                </span>
            </td>
            <td className="px-6 py-3">
                {session.task_id ? (
                    <span className="font-mono text-sm text-gray-400">
                        {shortId(session.task_id)}
                    </span>
                ) : (
                    <span className="text-sm text-gray-600">--</span>
                )}
            </td>
            <td className="px-6 py-3">
                <StatusBadge status={session.status} />
            </td>
            <td className="px-6 py-3">
                <span className="text-sm text-gray-400">{duration}</span>
            </td>
            <td className="px-6 py-3">
                <span className="text-sm text-gray-500">{created}</span>
            </td>
        </tr>
    );
}
