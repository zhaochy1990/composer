import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus } from 'lucide-react';
import { useAgents } from '@/hooks/use-agents';
import { useCreateSession } from '@/hooks/use-sessions';

interface SessionCreateDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    taskId: string;
    onSessionCreated?: (sessionId: string) => void;
}

export function SessionCreateDialog({
    open,
    onOpenChange,
    taskId,
    onSessionCreated,
}: SessionCreateDialogProps) {
    const { data: agents, isLoading: agentsLoading } = useAgents();
    const createSession = useCreateSession();

    const [agentId, setAgentId] = useState('');
    const [prompt, setPrompt] = useState('');
    const [repoPath, setRepoPath] = useState('');
    const [autoApprove, setAutoApprove] = useState(true);

    const resetForm = () => {
        setAgentId('');
        setPrompt('');
        setRepoPath('');
        setAutoApprove(true);
    };

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (!agentId || !prompt.trim()) return;

        if (!repoPath.trim()) return;

        createSession.mutate(
            {
                agent_id: agentId,
                task_id: taskId,
                prompt: prompt.trim(),
                repo_path: repoPath.trim(),
                auto_approve: autoApprove,
            },
            {
                onSuccess: (session) => {
                    resetForm();
                    onOpenChange(false);
                    onSessionCreated?.(session.id);
                },
            },
        );
    };

    const availableAgents = agents?.filter(
        (a) => a.status !== 'offline',
    ) ?? [];

    return (
        <Dialog.Root open={open} onOpenChange={(o) => { if (!o) resetForm(); onOpenChange(o); }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-lg bg-bg-surface border border-border-primary rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-5">
                        <Dialog.Title className="text-lg font-semibold text-text-primary">
                            New Session
                        </Dialog.Title>
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="text-text-muted hover:text-text-secondary transition-colors p-1 rounded hover:bg-bg-elevated"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </Dialog.Close>
                    </div>

                    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
                        {/* Agent select */}
                        <div className="flex flex-col gap-1.5">
                            <label
                                htmlFor="session-agent"
                                className="text-sm font-medium text-text-muted"
                            >
                                Agent
                            </label>
                            <select
                                id="session-agent"
                                value={agentId}
                                onChange={(e) => setAgentId(e.target.value)}
                                required
                                className="bg-bg-elevated border border-border-primary rounded-md px-3 py-2 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-blue-600 focus:border-transparent"
                            >
                                <option value="">
                                    {agentsLoading
                                        ? 'Loading agents...'
                                        : 'Select an agent'}
                                </option>
                                {availableAgents.map((agent) => (
                                    <option key={agent.id} value={agent.id}>
                                        {agent.name} ({agent.agent_type})
                                        {agent.status === 'busy'
                                            ? ' - busy'
                                            : ''}
                                    </option>
                                ))}
                            </select>
                            {!agentsLoading && availableAgents.length === 0 && (
                                <p className="text-xs text-yellow-500">
                                    No agents available. Register or discover
                                    agents first.
                                </p>
                            )}
                        </div>

                        {/* Prompt */}
                        <div className="flex flex-col gap-1.5">
                            <label
                                htmlFor="session-prompt"
                                className="text-sm font-medium text-text-muted"
                            >
                                Prompt
                            </label>
                            <textarea
                                id="session-prompt"
                                value={prompt}
                                onChange={(e) => setPrompt(e.target.value)}
                                required
                                rows={4}
                                placeholder="Describe the task for the agent..."
                                className="bg-bg-elevated border border-border-primary rounded-md px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-blue-600 focus:border-transparent resize-y min-h-[80px]"
                            />
                        </div>

                        {/* Repo path */}
                        <div className="flex flex-col gap-1.5">
                            <label
                                htmlFor="session-repo"
                                className="text-sm font-medium text-text-muted"
                            >
                                Repository Path
                            </label>
                            <input
                                id="session-repo"
                                type="text"
                                value={repoPath}
                                onChange={(e) => setRepoPath(e.target.value)}
                                placeholder="/path/to/repo"
                                required
                                className="bg-bg-elevated border border-border-primary rounded-md px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-blue-600 focus:border-transparent font-mono"
                            />
                        </div>

                        {/* Auto-approve */}
                        <label className="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                checked={autoApprove}
                                onChange={(e) =>
                                    setAutoApprove(e.target.checked)
                                }
                                className="w-4 h-4 rounded border-border-secondary bg-bg-elevated text-blue-600 focus:ring-blue-600 focus:ring-offset-0"
                            />
                            <span className="text-sm text-text-secondary">
                                Auto-approve tool usage
                            </span>
                        </label>

                        {/* Error message */}
                        {createSession.isError && (
                            <p className="text-sm text-red-400">
                                Failed to create session:{' '}
                                {createSession.error?.message ?? 'Unknown error'}
                            </p>
                        )}

                        {/* Actions */}
                        <div className="flex items-center justify-end gap-2 mt-2">
                            <Dialog.Close asChild>
                                <button
                                    type="button"
                                    className="px-4 py-2 rounded-md text-sm font-medium text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors"
                                >
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                type="submit"
                                disabled={
                                    createSession.isPending ||
                                    !agentId ||
                                    !prompt.trim() ||
                                    !repoPath.trim()
                                }
                                className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                {createSession.isPending
                                    ? 'Creating...'
                                    : 'Create Session'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
