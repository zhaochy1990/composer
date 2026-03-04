import { useState, useEffect, useRef, useMemo } from 'react';
import { X, Trash2, Plus, Square, Play } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';
import { useUpdateTask, useDeleteTask } from '@/hooks/use-tasks';
import { useTaskSessions } from '@/hooks/use-task-sessions';
import { useSession, useInterruptSession, useResumeSession } from '@/hooks/use-sessions';
import { useAgents } from '@/hooks/use-agents';
import { SessionCreateDialog } from '@/components/sessions/SessionCreateDialog';
import { SessionOutput } from '@/components/sessions/SessionOutput';
import { StatusBadge } from '@/components/sessions/StatusBadge';
import { shortId, formatDuration, formatTime } from '@/lib/utils';

interface TaskDetailPanelProps {
    task: Task;
    onClose: () => void;
}

export function TaskDetailPanel({ task, onClose }: TaskDetailPanelProps) {
    // --- Task edit form state ---
    const [title, setTitle] = useState(task.title);
    const [description, setDescription] = useState(task.description ?? '');
    const [priority, setPriority] = useState(task.priority);
    const [status, setStatus] = useState<TaskStatus>(task.status);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

    useEffect(() => {
        setTitle(task.title);
        setDescription(task.description ?? '');
        setPriority(task.priority);
        setStatus(task.status);
        setShowDeleteConfirm(false);
    }, [task.id, task.updated_at]);

    const updateTask = useUpdateTask();
    const deleteTask = useDeleteTask();

    // --- Sessions ---
    const { data: sessions } = useTaskSessions(task.id);
    const { data: agents } = useAgents();
    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);

    const interruptMutation = useInterruptSession();
    const resumeMutation = useResumeSession();

    // Build agent name map
    const agentNameMap = useMemo(() => {
        const map: Record<string, string> = {};
        if (agents) {
            for (const agent of agents) {
                map[agent.id] = agent.name;
            }
        }
        return map;
    }, [agents]);

    // Sort sessions: running first, then by created_at descending
    const sortedSessions = useMemo(() => {
        const list = [...(sessions ?? [])];
        const statusOrder: Record<string, number> = {
            running: 0, paused: 1, created: 2, failed: 3, completed: 4,
        };
        list.sort((a, b) => {
            const orderA = statusOrder[a.status] ?? 5;
            const orderB = statusOrder[b.status] ?? 5;
            if (orderA !== orderB) return orderA - orderB;
            return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        });
        return list;
    }, [sessions]);

    // Auto-select active session (once, on first data load)
    const hasAutoSelected = useRef(false);
    useEffect(() => {
        if (hasAutoSelected.current) return;
        const active = sortedSessions.find(s => s.status === 'running' || s.status === 'paused');
        if (active) {
            setSelectedSessionId(active.id);
            hasAutoSelected.current = true;
        }
    }, [sortedSessions]);

    const { data: selectedSession, isLoading: selectedSessionLoading } = useSession(selectedSessionId ?? undefined);

    // --- Handlers ---
    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!title.trim()) return;

        updateTask.mutate(
            {
                id: task.id,
                title: title.trim(),
                description: description.trim() || undefined,
                priority,
                status,
            },
            { onSuccess: () => onClose() },
        );
    }

    function handleDelete() {
        deleteTask.mutate(task.id, { onSuccess: () => onClose() });
    }

    const isRunning = selectedSession?.status === 'running';
    const isPaused = selectedSession?.status === 'paused';

    return (
        <>
            {/* Backdrop — ignore clicks while session dialog is open */}
            <div
                className="fixed inset-0 bg-black/40 z-40"
                onMouseDown={(e) => { if (!createDialogOpen && e.target === e.currentTarget) onClose(); }}
            />

            {/* Panel */}
            <div className="fixed inset-y-0 right-0 w-[720px] max-w-full z-50 bg-gray-900 border-l border-gray-700 shadow-2xl flex flex-col overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
                    <h2 className="text-lg font-semibold text-gray-100">Edit Task</h2>
                    <button
                        type="button"
                        onClick={onClose}
                        className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                    >
                        <X className="w-4 h-4" />
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto">
                    {/* Section 1: Task Edit Form */}
                    <form onSubmit={handleSubmit} className="px-6 py-4 border-b border-gray-800">
                        <div className="space-y-4">
                            <div>
                                <label htmlFor="edit-title" className="block text-sm font-medium text-gray-300 mb-1">
                                    Title <span className="text-red-400">*</span>
                                </label>
                                <input
                                    id="edit-title"
                                    type="text"
                                    value={title}
                                    onChange={e => setTitle(e.target.value)}
                                    placeholder="Task title"
                                    required
                                    className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                />
                            </div>

                            <div>
                                <label htmlFor="edit-description" className="block text-sm font-medium text-gray-300 mb-1">
                                    Description
                                </label>
                                <textarea
                                    id="edit-description"
                                    value={description}
                                    onChange={e => setDescription(e.target.value)}
                                    placeholder="Optional description"
                                    rows={3}
                                    className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
                                />
                            </div>

                            <div className="grid grid-cols-2 gap-4">
                                <div>
                                    <label htmlFor="edit-priority" className="block text-sm font-medium text-gray-300 mb-1">
                                        Priority
                                    </label>
                                    <select
                                        id="edit-priority"
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

                                <div>
                                    <label htmlFor="edit-status" className="block text-sm font-medium text-gray-300 mb-1">
                                        Status
                                    </label>
                                    <select
                                        id="edit-status"
                                        value={status}
                                        onChange={e => setStatus(e.target.value as TaskStatus)}
                                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                    >
                                        <option value="backlog">Backlog</option>
                                        <option value="in_progress">In Progress</option>
                                        <option value="waiting">Waiting</option>
                                        <option value="done">Done</option>
                                    </select>
                                </div>
                            </div>

                            <div className="text-xs text-gray-500">
                                Created {new Date(task.created_at).toLocaleString()}
                                {task.updated_at !== task.created_at && (
                                    <> &middot; Updated {new Date(task.updated_at).toLocaleString()}</>
                                )}
                            </div>

                            <div className="flex items-center justify-between pt-2">
                                {!showDeleteConfirm ? (
                                    <button
                                        type="button"
                                        onClick={() => setShowDeleteConfirm(true)}
                                        className="flex items-center gap-1 px-3 py-2 text-sm text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-md transition-colors"
                                    >
                                        <Trash2 className="w-4 h-4" />
                                        Delete
                                    </button>
                                ) : (
                                    <div className="flex items-center gap-2">
                                        <span className="text-sm text-red-400">Delete this task?</span>
                                        <button
                                            type="button"
                                            onClick={handleDelete}
                                            disabled={deleteTask.isPending}
                                            className="px-3 py-1 text-sm text-white bg-red-600 rounded-md hover:bg-red-500 transition-colors disabled:opacity-50"
                                        >
                                            {deleteTask.isPending ? 'Deleting...' : 'Yes'}
                                        </button>
                                        <button
                                            type="button"
                                            onClick={() => setShowDeleteConfirm(false)}
                                            className="px-3 py-1 text-sm text-gray-300 bg-gray-800 rounded-md hover:bg-gray-700 transition-colors"
                                        >
                                            No
                                        </button>
                                    </div>
                                )}

                                <div className="flex gap-2">
                                    <button
                                        type="button"
                                        onClick={onClose}
                                        className="px-4 py-2 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 transition-colors"
                                    >
                                        Cancel
                                    </button>
                                    <button
                                        type="submit"
                                        disabled={!title.trim() || updateTask.isPending}
                                        className="px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        {updateTask.isPending ? 'Saving...' : 'Save'}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </form>

                    {/* Section 2: Sessions */}
                    <div className="px-6 py-4 border-b border-gray-800">
                        <div className="flex items-center justify-between mb-3">
                            <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">Sessions</h3>
                            <button
                                type="button"
                                onClick={() => setCreateDialogOpen(true)}
                                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-500 transition-colors"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                Run Session
                            </button>
                        </div>

                        {sortedSessions.length === 0 ? (
                            <p className="text-sm text-gray-500 py-4 text-center">
                                No sessions yet — click Run Session to start one
                            </p>
                        ) : (
                            <div className="flex flex-col gap-1">
                                {sortedSessions.map((session) => (
                                    <button
                                        key={session.id}
                                        type="button"
                                        onClick={() => setSelectedSessionId(session.id)}
                                        className={`flex items-center gap-3 px-3 py-2 rounded-md text-sm text-left transition-colors ${
                                            selectedSessionId === session.id
                                                ? 'bg-gray-700 text-gray-100'
                                                : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
                                        }`}
                                    >
                                        <span className="font-mono text-xs">{shortId(session.id)}</span>
                                        <StatusBadge status={session.status} />
                                        <span className="truncate">{agentNameMap[session.agent_id] ?? shortId(session.agent_id)}</span>
                                        <span className="ml-auto text-xs text-gray-500">
                                            {formatDuration(session.started_at, session.completed_at)}
                                        </span>
                                        <span className="text-xs text-gray-600">{formatTime(session.created_at)}</span>
                                    </button>
                                ))}
                            </div>
                        )}
                    </div>

                    {/* Section 3: Session Output */}
                    {selectedSessionId && selectedSessionLoading && (
                        <div className="px-6 py-8 text-center">
                            <p className="text-sm text-gray-500">Loading session...</p>
                        </div>
                    )}
                    {selectedSessionId && selectedSession && (
                        <div className="px-6 py-4 flex flex-col gap-3">
                            {/* Session action buttons */}
                            <div className="flex items-center gap-2">
                                {isRunning && (
                                    <button
                                        type="button"
                                        onClick={() => interruptMutation.mutate(selectedSession.id)}
                                        disabled={interruptMutation.isPending}
                                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60 transition-colors disabled:opacity-50"
                                    >
                                        <Square className="w-3.5 h-3.5" />
                                        {interruptMutation.isPending ? 'Interrupting...' : 'Interrupt'}
                                    </button>
                                )}
                                {isPaused && (
                                    <button
                                        type="button"
                                        onClick={() => resumeMutation.mutate({ id: selectedSession.id })}
                                        disabled={resumeMutation.isPending}
                                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-green-900/40 text-green-300 border border-green-700 hover:bg-green-900/60 transition-colors disabled:opacity-50"
                                    >
                                        <Play className="w-3.5 h-3.5" />
                                        {resumeMutation.isPending ? 'Resuming...' : 'Resume'}
                                    </button>
                                )}
                            </div>

                            {/* Prompt */}
                            {selectedSession.prompt && (
                                <div>
                                    <p className="text-xs font-semibold text-gray-500 uppercase mb-1">Prompt</p>
                                    <p className="text-sm text-gray-300 whitespace-pre-wrap">{selectedSession.prompt}</p>
                                </div>
                            )}

                            {/* Result summary */}
                            {selectedSession.result_summary && (
                                <div>
                                    <p className="text-xs font-semibold text-gray-500 uppercase mb-1">Result</p>
                                    <p className="text-sm text-gray-300 whitespace-pre-wrap">{selectedSession.result_summary}</p>
                                </div>
                            )}

                            {/* Output */}
                            <div>
                                <p className="text-xs font-semibold text-gray-500 uppercase mb-2">Output</p>
                                <div className="h-[300px]">
                                    <SessionOutput sessionId={selectedSessionId} />
                                </div>
                            </div>
                        </div>
                    )}
                    {!selectedSessionId && sortedSessions.length > 0 && (
                        <div className="px-6 py-8 text-center">
                            <p className="text-sm text-gray-500">Select a session above to view its output</p>
                        </div>
                    )}
                </div>
            </div>

            {/* Session create dialog */}
            <SessionCreateDialog
                open={createDialogOpen}
                onOpenChange={setCreateDialogOpen}
                taskId={task.id}
                onSessionCreated={(id) => setSelectedSessionId(id)}
            />
        </>
    );
}
