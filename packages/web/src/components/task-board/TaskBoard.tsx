import { useState, useEffect, useMemo } from 'react';
import { Plus, RefreshCw } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';
import { useTasks, useStartTask } from '@/hooks/use-tasks';
import { useAgents } from '@/hooks/use-agents';
import { useProjects } from '@/hooks/use-projects';
import { TaskColumn } from './TaskColumn';
import { TaskCreateDialog } from './TaskCreateDialog';
import { TaskDetailPanel } from './TaskDetailPanel';

const columns: { status: TaskStatus; title: string }[] = [
    { status: 'backlog', title: 'Backlog' },
    { status: 'in_progress', title: 'In Progress' },
    { status: 'waiting', title: 'Waiting' },
    { status: 'done', title: 'Done' },
];

export function TaskBoard() {
    const { data: tasks, isLoading, isError, error, refetch } = useTasks();
    const { data: agents } = useAgents();
    const { data: projects } = useProjects();
    const startTask = useStartTask();

    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const [createDefaultStatus, setCreateDefaultStatus] = useState<TaskStatus>('backlog');
    const [editingTask, setEditingTask] = useState<Task | null>(null);
    const [startingTaskId, setStartingTaskId] = useState<string | null>(null);

    // Keep editingTask in sync with latest query data
    useEffect(() => {
        if (editingTask && tasks) {
            const updated = tasks.find(t => t.id === editingTask.id);
            if (updated && updated.updated_at !== editingTask.updated_at) {
                setEditingTask(updated);
            }
        }
    }, [tasks, editingTask]);

    // Build agent ID → name map for display in task cards
    const agentNameMap = useMemo(() => {
        const map: Record<string, string> = {};
        if (agents) {
            for (const agent of agents) {
                map[agent.id] = agent.name;
            }
        }
        return map;
    }, [agents]);

    // Build project ID → name map for display in task cards
    const projectNameMap = useMemo(() => {
        const map: Record<string, string> = {};
        if (projects) {
            for (const p of projects) {
                map[p.id] = p.name;
            }
        }
        return map;
    }, [projects]);

    const tasksByStatus = useMemo(() => {
        const grouped: Record<TaskStatus, Task[]> = {
            backlog: [],
            in_progress: [],
            waiting: [],
            done: [],
        };
        if (tasks) {
            for (const task of tasks) {
                const bucket = grouped[task.status];
                if (bucket) {
                    bucket.push(task);
                }
            }
            // Sort each column by position, then by creation date
            for (const status of Object.keys(grouped) as TaskStatus[]) {
                grouped[status].sort((a, b) => a.position - b.position || a.created_at.localeCompare(b.created_at));
            }
        }
        return grouped;
    }, [tasks]);

    function handleCreateTask(status: TaskStatus) {
        setCreateDefaultStatus(status);
        setCreateDialogOpen(true);
    }

    function handleEditTask(task: Task) {
        setEditingTask(task);
    }

    function handleStartTask(taskId: string) {
        setStartingTaskId(taskId);
        startTask.mutate(taskId, {
            onSettled: () => setStartingTaskId(null),
        });
    }

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
                <div>
                    <h1 className="text-lg font-semibold text-gray-100">Task Board</h1>
                    <p className="text-sm text-gray-500">
                        {tasks ? `${tasks.length} task${tasks.length !== 1 ? 's' : ''}` : 'Loading...'}
                    </p>
                </div>
                <div className="flex items-center gap-2">
                    <button
                        type="button"
                        onClick={() => refetch()}
                        className="flex items-center gap-1 px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 bg-gray-800 border border-gray-700 rounded-md hover:bg-gray-700 transition-colors"
                        title="Refresh tasks"
                    >
                        <RefreshCw className="w-3.5 h-3.5" />
                    </button>
                    <button
                        type="button"
                        onClick={() => handleCreateTask('backlog')}
                        className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors"
                    >
                        <Plus className="w-4 h-4" />
                        New Task
                    </button>
                </div>
            </div>

            {/* Board content */}
            <div className="flex-1 overflow-x-auto overflow-y-hidden">
                {isLoading && (
                    <div className="flex items-center justify-center h-full">
                        <p className="text-gray-500 text-sm">Loading tasks...</p>
                    </div>
                )}

                {isError && (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-center">
                            <p className="text-red-400 text-sm mb-2">Failed to load tasks</p>
                            <p className="text-gray-500 text-xs mb-3">{(error as Error)?.message}</p>
                            <button
                                type="button"
                                onClick={() => refetch()}
                                className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-700 rounded-md hover:bg-gray-700 transition-colors"
                            >
                                Retry
                            </button>
                        </div>
                    </div>
                )}

                {!isLoading && !isError && (
                    <div className="flex gap-4 h-full p-6">
                        {columns.map(col => (
                            <TaskColumn
                                key={col.status}
                                status={col.status}
                                title={col.title}
                                tasks={tasksByStatus[col.status]}
                                onCreateTask={handleCreateTask}
                                onEditTask={handleEditTask}
                                agentNameMap={agentNameMap}
                                projectNameMap={projectNameMap}
                                onStartTask={handleStartTask}
                                startingTaskId={startingTaskId}
                            />
                        ))}
                    </div>
                )}
            </div>

            {/* Dialogs */}
            <TaskCreateDialog
                isOpen={createDialogOpen}
                onClose={() => setCreateDialogOpen(false)}
                defaultStatus={createDefaultStatus}
            />

            {editingTask && (
                <TaskDetailPanel
                    key={editingTask.id}
                    task={editingTask}
                    onClose={() => setEditingTask(null)}
                />
            )}
        </div>
    );
}
