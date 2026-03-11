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
import { ProjectFilter } from './ProjectFilter';
import { TaskSearchInput } from './TaskSearchInput';

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
    const [projectFilter, setProjectFilter] = useState<string[]>([]);
    const [showNoProject, setShowNoProject] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
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
            let filtered: Task[] = tasks;
            if (searchQuery.trim()) {
                const q = searchQuery.trim().toLowerCase();
                filtered = filtered.filter(t =>
                    t.title.toLowerCase().includes(q) ||
                    (t.description && t.description.toLowerCase().includes(q)) ||
                    t.simple_id.toLowerCase().includes(q)
                );
            }
            if (priorityFilter.length > 0) {
                filtered = filtered.filter(t => priorityFilter.includes(t.priority));
            }
            if (projectFilter.length > 0 || showNoProject) {
                filtered = filtered.filter(t => {
                    if (!t.project_id) return showNoProject;
                    return projectFilter.includes(t.project_id);
                });
            }
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
    }, [tasks, searchQuery, priorityFilter, projectFilter, showNoProject]);

    const filteredCount = useMemo(() =>
        Object.values(tasksByStatus).reduce((sum, arr) => sum + arr.length, 0),
        [tasksByStatus]
    );

    function handleEditTask(task: Task) {
        setEditingTask(task);
    }

    function handleCloneSuccess(newTask: Task) {
        setEditingTask(newTask);
    }

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-border-primary">
                <div>
                    <h1 className="text-lg font-semibold text-text-primary">Task Board</h1>
                    <p className="text-sm text-text-muted">
                        {tasks
                            ? filteredCount < tasks.length
                                ? `${filteredCount} of ${tasks.length} task${tasks.length !== 1 ? 's' : ''}`
                                : `${tasks.length} task${tasks.length !== 1 ? 's' : ''}`
                            : 'Loading...'}
                    </p>
                </div>
                <div className="flex items-center gap-3">
                    <TaskSearchInput value={searchQuery} onChange={setSearchQuery} />
                    <PriorityFilter selected={priorityFilter} onChange={setPriorityFilter} />
                    {projects && projects.length > 0 && (
                        <ProjectFilter
                            projects={projects}
                            selected={projectFilter}
                            includeNoProject={showNoProject}
                            onChange={(sel, noProj) => { setProjectFilter(sel); setShowNoProject(noProj); }}
                        />
                    )}
                    <div className="flex rounded-md border border-border-primary overflow-hidden">
                        <button
                            type="button"
                            onClick={() => setViewMode('list')}
                            aria-pressed={viewMode === 'list'}
                            className={`flex items-center px-2.5 py-1.5 transition-colors ${viewMode === 'list' ? 'bg-bg-interactive text-text-primary' : 'text-text-muted hover:text-text-secondary'}`}
                            title="List view"
                        >
                            <LayoutList className="w-4 h-4" />
                        </button>
                        <button
                            type="button"
                            onClick={() => setViewMode('kanban')}
                            aria-pressed={viewMode === 'kanban'}
                            className={`flex items-center px-2.5 py-1.5 transition-colors ${viewMode === 'kanban' ? 'bg-bg-interactive text-text-primary' : 'text-text-muted hover:text-text-secondary'}`}
                            title="Kanban view"
                        >
                            <Kanban className="w-4 h-4" />
                        </button>
                    </div>
                    <button
                        type="button"
                        onClick={() => refetch()}
                        className="flex items-center gap-1 px-3 py-1.5 text-sm text-text-muted hover:text-text-primary bg-bg-elevated border border-border-primary rounded-md hover:bg-bg-interactive transition-colors"
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
                        <p className="text-text-muted text-sm">Loading tasks...</p>
                    </div>
                )}

                {isError && (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-center">
                            <p className="text-red-400 text-sm mb-2">Failed to load tasks</p>
                            <p className="text-text-muted text-xs mb-3">{(error as Error)?.message}</p>
                            <button
                                type="button"
                                onClick={() => refetch()}
                                className="px-3 py-1.5 text-sm text-text-secondary bg-bg-elevated border border-border-primary rounded-md hover:bg-bg-interactive transition-colors"
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
                        onCloneSuccess={handleCloneSuccess}
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
                    onCloneSuccess={handleCloneSuccess}
                />
            )}
        </div>
    );
}
