import { useState, useEffect, useMemo } from 'react';
import { Plus, RefreshCw, LayoutList, Kanban } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';
import { useTasks } from '@/hooks/use-tasks';
import { useAgents } from '@/hooks/use-agents';
import { useProjects } from '@/hooks/use-projects';
import { TaskColumn } from './TaskColumn';
import { TaskListView } from './TaskListView';
import { TaskCreateDialog } from './TaskCreateDialog';
import { TaskDetailPanel } from './TaskDetailPanel';
import { PriorityFilter } from './PriorityFilter';

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

    const [viewMode, setViewMode] = useState<'list' | 'kanban'>(
        () => (localStorage.getItem('taskBoardViewMode') as 'list' | 'kanban') || 'list'
    );
    useEffect(() => { localStorage.setItem('taskBoardViewMode', viewMode); }, [viewMode]);
    const [priorityFilter, setPriorityFilter] = useState<number[]>([]);
    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const [editingTask, setEditingTask] = useState<Task | null>(null);

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
            const filtered = priorityFilter.length > 0
                ? tasks.filter(t => priorityFilter.includes(t.priority))
                : tasks;
            for (const task of filtered) {
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
    }, [tasks, priorityFilter]);

    function handleEditTask(task: Task) {
        setEditingTask(task);
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
                <div className="flex items-center gap-3">
                    <PriorityFilter selected={priorityFilter} onChange={setPriorityFilter} />
                    <div className="flex rounded-md border border-gray-700 overflow-hidden">
                        <button
                            type="button"
                            onClick={() => setViewMode('list')}
                            aria-pressed={viewMode === 'list'}
                            className={`flex items-center px-2.5 py-1.5 transition-colors ${viewMode === 'list' ? 'bg-gray-700 text-gray-100' : 'text-gray-500 hover:text-gray-300'}`}
                            title="List view"
                        >
                            <LayoutList className="w-4 h-4" />
                        </button>
                        <button
                            type="button"
                            onClick={() => setViewMode('kanban')}
                            aria-pressed={viewMode === 'kanban'}
                            className={`flex items-center px-2.5 py-1.5 transition-colors ${viewMode === 'kanban' ? 'bg-gray-700 text-gray-100' : 'text-gray-500 hover:text-gray-300'}`}
                            title="Kanban view"
                        >
                            <Kanban className="w-4 h-4" />
                        </button>
                    </div>
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
                        onClick={() => setCreateDialogOpen(true)}
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

                {!isLoading && !isError && viewMode === 'kanban' && (
                    <div className="flex gap-4 h-full p-6">
                        {columns.map(col => (
                            <TaskColumn
                                key={col.status}
                                title={col.title}
                                tasks={tasksByStatus[col.status]}
                                onEditTask={handleEditTask}
                                agentNameMap={agentNameMap}
                                projectNameMap={projectNameMap}
                            />
                        ))}
                    </div>
                )}

                {!isLoading && !isError && viewMode === 'list' && (
                    <TaskListView
                        tasksByStatus={tasksByStatus}
                        onEditTask={handleEditTask}
                        selectedTask={editingTask}
                        onCloseTask={() => setEditingTask(null)}
                        agentNameMap={agentNameMap}
                        projectNameMap={projectNameMap}
                    />
                )}
            </div>

            {/* Dialogs */}
            <TaskCreateDialog
                isOpen={createDialogOpen}
                onClose={() => setCreateDialogOpen(false)}
                defaultStatus={'backlog'}
            />

            {editingTask && viewMode === 'kanban' && (
                <TaskDetailPanel
                    key={editingTask.id}
                    task={editingTask}
                    onClose={() => setEditingTask(null)}
                />
            )}
        </div>
    );
}
