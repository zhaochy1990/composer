import { useState } from 'react';
import { Bot, Plus, RefreshCw } from 'lucide-react';
import { useAgents, useDiscoverAgents } from '@/hooks/use-agents';
import { AgentCard } from './AgentCard';
import { AgentRegisterDialog } from './AgentRegisterDialog';

export function AgentPool() {
    const [registerOpen, setRegisterOpen] = useState(false);
    const { data: agents, isLoading, isError } = useAgents();
    const discoverAgents = useDiscoverAgents();

    return (
        <div className="h-full overflow-y-auto p-6">
            <div className="flex items-center justify-between mb-6">
                <h1 className="text-xl font-bold text-text-primary">Agent Pool</h1>
                <div className="flex items-center gap-2">
                    <button
                        onClick={() => discoverAgents.mutate()}
                        disabled={discoverAgents.isPending}
                        className="flex items-center gap-2 px-3 py-2 text-sm bg-bg-elevated text-text-secondary rounded-md hover:bg-bg-interactive disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <RefreshCw
                            className={`w-4 h-4 ${discoverAgents.isPending ? 'animate-spin' : ''}`}
                        />
                        Discover Agents
                    </button>
                    <button
                        onClick={() => setRegisterOpen(true)}
                        className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
                    >
                        <Plus className="w-4 h-4" />
                        Add Agent
                    </button>
                </div>
            </div>

            {isLoading && (
                <div className="flex items-center justify-center h-64">
                    <p className="text-sm text-text-muted">Loading agents...</p>
                </div>
            )}

            {isError && (
                <div className="flex items-center justify-center h-64">
                    <p className="text-sm text-red-400">Failed to load agents.</p>
                </div>
            )}

            {!isLoading && !isError && agents && agents.length === 0 && (
                <div className="flex flex-col items-center justify-center h-64 text-center">
                    <Bot className="w-12 h-12 text-text-muted mb-4" />
                    <p className="text-sm text-text-muted">
                        No agents registered. Click Discover to find installed agents.
                    </p>
                </div>
            )}

            {!isLoading && !isError && agents && agents.length > 0 && (
                <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                    {agents.map(agent => (
                        <AgentCard key={agent.id} agent={agent} />
                    ))}
                </div>
            )}

            <AgentRegisterDialog open={registerOpen} onClose={() => setRegisterOpen(false)} />
        </div>
    );
}
