import { useState } from 'react';
import { ChevronRight, ChevronDown, GitPullRequest } from 'lucide-react';
import type { Task } from '@/types/generated';
import { shortId, formatTime } from '@/lib/utils';
import { priorityConfig } from './priority-config';

interface TaskListSectionProps {
    title: string;
    tasks: Task[];
    onEditTask: (task: Task) => void;
    defaultCollapsed?: boolean;
    agentNameMap?: Record<string, string>;
    projectNameMap?: Record<string, string>;
    selectedTaskId?: string;
}

export function TaskListSection({
    title,
    tasks,
    onEditTask,
    defaultCollapsed = false,
    agentNameMap,
    projectNameMap,
    selectedTaskId,
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
            </div>
            {!collapsed && (
                <div>
                    {tasks.length === 0 ? (
                        <div className="px-4 py-3 text-sm text-gray-600">
                            No tasks
                        </div>
                    ) : (
                        tasks.map((task) => {
                            const isSelected = task.id === selectedTaskId;
                            const priority = priorityConfig[task.priority] ?? priorityConfig[0];
                            return (
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
                                    className={`px-4 py-2.5 cursor-pointer border-b border-gray-800/50 transition-colors ${
                                        isSelected
                                            ? 'bg-gray-800 border-l-2 border-l-blue-500'
                                            : 'hover:bg-gray-800/60 border-l-2 border-l-transparent'
                                    }`}
                                >
                                    <div className="flex items-start gap-1">
                                        {task.simple_id && (
                                            <span className="font-mono text-xs text-gray-500 mt-0.5 shrink-0">
                                                {task.simple_id}
                                            </span>
                                        )}
                                        <span className="text-sm text-gray-100 line-clamp-1">{task.title}</span>
                                    </div>
                                    <div className="flex items-center gap-1.5 mt-1.5 flex-wrap">
                                        <span
                                            className={`inline-flex items-center text-[10px] px-1.5 py-0.5 rounded border ${priority.className}`}
                                        >
                                            {priority.label}
                                        </span>
                                        {task.assigned_agent_id && (
                                            <span className="inline-flex items-center text-[10px] px-1.5 py-0.5 rounded bg-purple-900/50 text-purple-300 border border-purple-700">
                                                {agentNameMap?.[task.assigned_agent_id] ?? shortId(task.assigned_agent_id)}
                                            </span>
                                        )}
                                        {task.project_id && (
                                            <span className="inline-flex items-center text-[10px] px-1.5 py-0.5 rounded bg-teal-900/50 text-teal-300 border border-teal-700">
                                                {projectNameMap?.[task.project_id] ?? shortId(task.project_id)}
                                            </span>
                                        )}
                                        {task.pr_urls.length > 0 && (
                                            <span className="inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded bg-green-900/50 text-green-300 border border-green-700">
                                                <GitPullRequest className="w-2.5 h-2.5" />
                                                {task.pr_urls.length === 1 ? 'PR' : `${task.pr_urls.length} PRs`}
                                            </span>
                                        )}
                                        {task.status === 'done' && task.completed_at && (
                                            <span className="text-[10px] text-gray-500">
                                                Completed {formatTime(task.completed_at)}
                                            </span>
                                        )}
                                    </div>
                                </div>
                            );
                        })
                    )}
                </div>
            )}
        </div>
    );
}
