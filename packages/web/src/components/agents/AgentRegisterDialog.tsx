import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X } from 'lucide-react';
import { useCreateAgent } from '@/hooks/use-agents';

interface AgentRegisterDialogProps {
    open: boolean;
    onClose: () => void;
}

export function AgentRegisterDialog({ open, onClose }: AgentRegisterDialogProps) {
    const [name, setName] = useState('');
    const createAgent = useCreateAgent();

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!name.trim()) return;

        createAgent.mutate(
            { name: name.trim(), agent_type: 'claude_code' },
            {
                onSuccess: () => {
                    setName('');
                    onClose();
                },
            },
        );
    }

    return (
        <Dialog.Root open={open} onOpenChange={(o) => { if (!o) { setName(''); onClose(); } }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-md bg-bg-surface border border-border-primary rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-bold text-text-primary">
                            Register Agent
                        </Dialog.Title>
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="text-text-muted hover:text-text-primary transition-colors p-1 rounded hover:bg-bg-elevated"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </Dialog.Close>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div>
                            <label htmlFor="agent-name" className="block text-sm text-text-muted mb-1">
                                Name
                            </label>
                            <input
                                id="agent-name"
                                type="text"
                                value={name}
                                onChange={e => setName(e.target.value)}
                                placeholder="e.g. Agent 1"
                                required
                                className="w-full bg-bg-elevated border border-border-primary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-blue-600 focus:border-transparent"
                            />
                        </div>

                        <div>
                            <label className="block text-sm text-text-muted mb-1">Agent Type</label>
                            <div className="w-full bg-bg-elevated border border-border-primary rounded-md px-3 py-2 text-sm text-text-muted">
                                Claude Code
                            </div>
                        </div>

                        {createAgent.isError && (
                            <p className="text-sm text-red-400">
                                Failed to register agent. Please try again.
                            </p>
                        )}

                        <div className="flex justify-end gap-2 pt-2">
                            <Dialog.Close asChild>
                                <button
                                    type="button"
                                    className="px-4 py-2 text-sm text-text-muted hover:text-text-primary rounded-md hover:bg-bg-elevated transition-colors"
                                >
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                type="submit"
                                disabled={createAgent.isPending || !name.trim()}
                                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                            >
                                {createAgent.isPending ? 'Registering...' : 'Register'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
