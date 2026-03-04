import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus } from 'lucide-react';
import type { TaskStatus } from '@/types/generated';
import { useCreateTask } from '@/hooks/use-tasks';
import { useProjects } from '@/hooks/use-projects';
import { useAgents } from '@/hooks/use-agents';

interface TaskCreateDialogProps {
    isOpen: boolean;
    onClose: () => void;
    defaultStatus: TaskStatus;
}

export function TaskCreateDialog({ isOpen, onClose, defaultStatus }: TaskCreateDialogProps) {
    const [title, setTitle] = useState('');
    const [description, setDescription] = useState('');
    const [priority, setPriority] = useState(0);
    const [projectId, setProjectId] = useState('');
    const [assignedAgentId, setAssignedAgentId] = useState('');

    const createTask = useCreateTask();
    const { data: projects } = useProjects();
    const { data: agents } = useAgents();

    // Default to first available agent (Claude Code)
    useEffect(() => {
        if (agents?.length && !assignedAgentId) {
            setAssignedAgentId(agents[0].id);
        }
    }, [agents, assignedAgentId]);

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!title.trim()) return;

        createTask.mutate(
            {
                title: title.trim(),
                description: description.trim() || undefined,
                priority,
                status: defaultStatus,
                project_id: projectId || undefined,
                assigned_agent_id: assignedAgentId || undefined,
            },
            {
                onSuccess: () => {
                    setTitle('');
                    setDescription('');
                    setPriority(0);
                    setProjectId('');
                    setAssignedAgentId(agents?.[0]?.id ?? '');
                    onClose();
                },
            },
        );
    }

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => { if (!open) { setTitle(''); setDescription(''); setPriority(0); setProjectId(''); setAssignedAgentId(agents?.[0]?.id ?? ''); onClose(); } }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-[480px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-gray-100">
                            New Task
                        </Dialog.Title>
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </Dialog.Close>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div>
                            <label htmlFor="create-title" className="block text-sm font-medium text-gray-300 mb-1">
                                Title <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="create-title"
                                type="text"
                                value={title}
                                onChange={e => setTitle(e.target.value)}
                                placeholder="Task title"
                                required
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>

                        <div>
                            <label htmlFor="create-description" className="block text-sm font-medium text-gray-300 mb-1">
                                Description
                            </label>
                            <textarea
                                id="create-description"
                                value={description}
                                onChange={e => setDescription(e.target.value)}
                                placeholder="Optional description"
                                rows={3}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
                            />
                        </div>

                        <div>
                            <label htmlFor="create-priority" className="block text-sm font-medium text-gray-300 mb-1">
                                Priority
                            </label>
                            <select
                                id="create-priority"
                                value={priority}
                                onChange={e => setPriority(Number(e.target.value))}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            >
                                <option value={0}>None</option>
                                <option value={1}>Low</option>
                                <option value={2}>Medium</option>
                                <option value={3}>High</option>
                            </select>
                        </div>

                        {projects && projects.length > 0 && (
                            <div>
                                <label htmlFor="create-project" className="block text-sm font-medium text-gray-300 mb-1">
                                    Project
                                </label>
                                <select
                                    id="create-project"
                                    value={projectId}
                                    onChange={e => setProjectId(e.target.value)}
                                    className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                >
                                    <option value="">No project</option>
                                    {projects.map(p => (
                                        <option key={p.id} value={p.id}>{p.name}</option>
                                    ))}
                                </select>
                            </div>
                        )}

                        <div>
                            <label htmlFor="create-agent" className="block text-sm font-medium text-gray-300 mb-1">
                                Agent
                            </label>
                            <select
                                id="create-agent"
                                value={assignedAgentId}
                                onChange={e => setAssignedAgentId(e.target.value)}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            >
                                <option value="">None</option>
                                {agents?.map(agent => (
                                    <option key={agent.id} value={agent.id}>{agent.name}</option>
                                ))}
                            </select>
                        </div>

                        <p className="text-xs text-gray-500">
                            Task will be created in <span className="font-medium text-gray-400">{formatStatus(defaultStatus)}</span>
                        </p>

                        <div className="flex justify-end gap-2 pt-2">
                            <Dialog.Close asChild>
                                <button
                                    type="button"
                                    className="px-4 py-2 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 transition-colors"
                                >
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                type="submit"
                                disabled={!title.trim() || createTask.isPending}
                                className="flex items-center gap-1.5 px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                {createTask.isPending ? 'Creating...' : 'Create Task'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}

function formatStatus(status: TaskStatus): string {
    const labels: Record<TaskStatus, string> = {
        backlog: 'Backlog',
        in_progress: 'In Progress',
        waiting: 'Waiting',
        done: 'Done',
    };
    return labels[status];
}
