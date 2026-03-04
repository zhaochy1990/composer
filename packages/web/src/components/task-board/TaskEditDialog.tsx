import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Trash2 } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';
import { useUpdateTask, useDeleteTask } from '@/hooks/use-tasks';

interface TaskEditDialogProps {
    task: Task | null;
    onClose: () => void;
}

export function TaskEditDialog({ task, onClose }: TaskEditDialogProps) {
    const [title, setTitle] = useState(task?.title ?? '');
    const [description, setDescription] = useState(task?.description ?? '');
    const [priority, setPriority] = useState(task?.priority ?? 0);
    const [status, setStatus] = useState<TaskStatus>(task?.status ?? 'backlog');
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

    // Fix #29: Sync form state when the task prop changes
    useEffect(() => {
        if (task) {
            setTitle(task.title);
            setDescription(task.description ?? '');
            setPriority(task.priority);
            setStatus(task.status);
            setShowDeleteConfirm(false);
        }
    }, [task?.id, task?.updated_at]);

    const updateTask = useUpdateTask();
    const deleteTask = useDeleteTask();

    if (!task) return null;

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!title.trim() || !task) return;

        updateTask.mutate(
            {
                id: task.id,
                title: title.trim(),
                description: description.trim() || undefined,
                priority,
                status,
            },
            {
                onSuccess: () => onClose(),
            },
        );
    }

    function handleDelete() {
        if (!task) return;
        deleteTask.mutate(task.id, {
            onSuccess: () => onClose(),
        });
    }

    return (
        <Dialog.Root open={!!task} onOpenChange={(open) => { if (!open) onClose(); }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-[480px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-gray-100">
                            Edit Task
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
                                    disabled={!title.trim() || updateTask.isPending}
                                    className="px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    {updateTask.isPending ? 'Saving...' : 'Save'}
                                </button>
                            </div>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
