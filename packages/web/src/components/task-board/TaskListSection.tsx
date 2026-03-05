import { useState } from 'react';
import { ChevronRight, ChevronDown, Plus } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';

interface TaskListSectionProps {
    title: string;
    status: TaskStatus;
    tasks: Task[];
    onEditTask: (task: Task) => void;
    onCreateTask: (status: TaskStatus) => void;
    defaultCollapsed?: boolean;
}

export function TaskListSection({
    title,
    status,
    tasks,
    onEditTask,
    onCreateTask,
    defaultCollapsed = false,
}: TaskListSectionProps) {
    const [collapsed, setCollapsed] = useState(defaultCollapsed);

    return (
        <div>
            <div
                className="flex items-center justify-between px-4 py-2 bg-gray-900/50 border-b border-gray-800 cursor-pointer select-none"
                role="button"
                tabIndex={0}
                aria-expanded={!collapsed}
                onClick={() => setCollapsed(!collapsed)}
                onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        setCollapsed(!collapsed);
                    }
                }}
            >
                <div className="flex items-center gap-2">
                    {collapsed ? (
                        <ChevronRight className="w-4 h-4 text-gray-500" />
                    ) : (
                        <ChevronDown className="w-4 h-4 text-gray-500" />
                    )}
                    <span className="text-sm font-semibold text-gray-400 uppercase">
                        {title}
                    </span>
                    <span className="text-xs text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
                        {tasks.length}
                    </span>
                </div>
                <button
                    type="button"
                    onClick={(e) => {
                        e.stopPropagation();
                        onCreateTask(status);
                    }}
                    className="text-gray-500 hover:text-gray-300 transition-colors"
                    title={`Add task to ${title}`}
                >
                    <Plus className="w-4 h-4" />
                </button>
            </div>
            {!collapsed && (
                <div>
                    {tasks.length === 0 ? (
                        <div className="px-4 py-3 text-sm text-gray-600">
                            No tasks
                        </div>
                    ) : (
                        tasks.map((task) => (
                            <div
                                key={task.id}
                                role="button"
                                tabIndex={0}
                                onClick={() => onEditTask(task)}
                                onKeyDown={(e) => {
                                    if (e.key === 'Enter' || e.key === ' ') {
                                        e.preventDefault();
                                        onEditTask(task);
                                    }
                                }}
                                className="px-4 py-2 hover:bg-gray-800 cursor-pointer border-b border-gray-800/50 transition-colors"
                            >
                                {task.simple_id && (
                                    <span className="font-mono text-xs text-gray-500 mr-2">
                                        {task.simple_id}
                                    </span>
                                )}
                                <span className="text-sm text-gray-100">{task.title}</span>
                            </div>
                        ))
                    )}
                </div>
            )}
        </div>
    );
}
