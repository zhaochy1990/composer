import { useState } from 'react';
import { ChevronRight, ChevronDown, GitPullRequest, Workflow } from 'lucide-react';
import type { Task } from '@/types/generated';
import { shortId, formatTime, extractPrId } from '@/lib/utils';
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
                className="flex items-center justify-between px-4 py-2 bg-bg-surface/50 border-b border-border-primary cursor-pointer select-none"
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
                        <ChevronRight className="w-4 h-4 text-text-muted" />
                    ) : (
                        <ChevronDown className="w-4 h-4 text-text-muted" />
                    )}
                    <span className="text-sm font-semibold text-text-muted uppercase">
                        {title}
                    </span>
                    <span className="text-xs text-text-muted bg-bg-elevated px-1.5 py-0.5 rounded">
                        {tasks.length}
                    </span>
                </div>
            </div>
            {!collapsed && (
                <div>
                    {tasks.length === 0 ? (
                        <div className="px-4 py-3 text-sm text-text-muted">
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
                                    className={`px-4 py-2.5 cursor-pointer border-b border-border-primary/50 transition-colors ${
                                        isSelected
                                            ? 'bg-bg-elevated border-l-2 border-l-blue-500'
                                            : 'hover:bg-bg-elevated border-l-2 border-l-transparent'
                                    }`}
                                >
                                    <div className="flex items-start gap-1">
                                        {task.simple_id && (
                                            <span className="font-mono text-xs text-text-muted mt-0.5 shrink-0">
                                                {task.simple_id}
                                            </span>
                                        )}
                                        <span className="text-sm text-text-primary line-clamp-1">{task.title}</span>
                                    </div>
                                    <div className="flex items-center gap-1.5 mt-1.5 flex-wrap">
                                        <span
                                            className={`inline-flex items-center text-[10px] px-1.5 py-0.5 rounded border ${priority.className}`}
                                        >
                                            {priority.label}
                                        </span>
                                        {task.assigned_agent_id && (
                                            <span className="inline-flex items-center text-[10px] px-1.5 py-0.5 rounded bg-purple-100 text-purple-800 border border-purple-300 dark:bg-purple-900/50 dark:text-purple-300 dark:border-purple-700">
                                                {agentNameMap?.[task.assigned_agent_id] ?? shortId(task.assigned_agent_id)}
                                            </span>
                                        )}
                                        {task.project_id && (
                                            <span className="inline-flex items-center text-[10px] px-1.5 py-0.5 rounded bg-teal-100 text-teal-800 border border-teal-300 dark:bg-teal-900/50 dark:text-teal-300 dark:border-teal-700">
                                                {projectNameMap?.[task.project_id] ?? shortId(task.project_id)}
                                            </span>
                                        )}
                                        {task.pr_urls.map((url) => (
                                            <a
                                                key={url}
                                                href={url}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                onClick={(e) => e.stopPropagation()}
                                                className="inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded bg-green-100 text-green-800 border border-green-300 dark:bg-green-900/50 dark:text-green-300 dark:border-green-700 hover:bg-green-200 dark:hover:bg-green-900/70 transition-colors"
                                            >
                                                <GitPullRequest className="w-2.5 h-2.5" />
                                                {extractPrId(url)}
                                            </a>
                                        ))}
                                        {task.current_step_name && (
                                            <span className={`inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded border ${
                                                task.current_step_status === 'running'
                                                    ? 'bg-indigo-900/50 text-indigo-300 border-indigo-700'
                                                    : task.current_step_status === 'waiting_for_human'
                                                    ? 'bg-amber-900/50 text-amber-300 border-amber-700'
                                                    : task.current_step_status === 'failed'
                                                    ? 'bg-red-900/50 text-red-300 border-red-700'
                                                    : 'bg-cyan-900/50 text-cyan-300 border-cyan-700'
                                            }`}>
                                                <Workflow className="w-2.5 h-2.5" />
                                                {task.current_step_name}
                                            </span>
                                        )}
                                        {task.status === 'done' && task.completed_at && (
                                            <span className="text-[10px] text-text-muted">
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
