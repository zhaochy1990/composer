import { GripVertical, GitPullRequest, Link2 } from 'lucide-react';
import type { Task } from '@/types/generated';
import { shortId, formatTime, extractPrId } from '@/lib/utils';
import { priorityConfig } from './priority-config';

interface TaskCardProps {
    task: Task;
    onClick: (task: Task) => void;
    agentNameMap?: Record<string, string>;
    projectNameMap?: Record<string, string>;
}

export function TaskCard({ task, onClick, agentNameMap, projectNameMap }: TaskCardProps) {
    const priority = priorityConfig[task.priority] ?? priorityConfig[0];

    return (
        <div
            role="button"
            tabIndex={0}
            onClick={() => onClick(task)}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick(task); }}
            className="w-full text-left bg-bg-elevated border border-border-primary rounded-md p-3 hover:border-border-secondary transition-colors cursor-pointer group"
        >
            <div className="flex items-start gap-2">
                <GripVertical className="w-4 h-4 text-text-muted mt-0.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" />
                <div className="flex-1 min-w-0">
                    {task.simple_id && (
                        <span className="font-mono text-xs text-text-muted mr-1.5">{task.simple_id}</span>
                    )}
                    <p className="font-semibold text-text-primary text-sm leading-snug">
                        {task.title}
                    </p>
                    {task.description && (
                        <p className="text-sm text-text-muted mt-1 line-clamp-2">
                            {task.description}
                        </p>
                    )}
                    <div className="flex items-center gap-2 mt-2 flex-wrap">
                        <span
                            className={`inline-flex items-center text-xs px-1.5 py-0.5 rounded border ${priority.className}`}
                        >
                            {priority.label}
                        </span>
                        {task.assigned_agent_id && (
                            <span className="inline-flex items-center text-xs px-1.5 py-0.5 rounded bg-purple-100 text-purple-800 border border-purple-300 dark:bg-purple-900/50 dark:text-purple-300 dark:border-purple-700">
                                {agentNameMap?.[task.assigned_agent_id] ?? shortId(task.assigned_agent_id)}
                            </span>
                        )}
                        {task.project_id && (
                            <span className="inline-flex items-center text-xs px-1.5 py-0.5 rounded bg-teal-100 text-teal-800 border border-teal-300 dark:bg-teal-900/50 dark:text-teal-300 dark:border-teal-700">
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
                                className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-green-100 text-green-800 border border-green-300 dark:bg-green-900/50 dark:text-green-300 dark:border-green-700 hover:bg-green-200 dark:hover:bg-green-900/70 transition-colors"
                            >
                                <GitPullRequest className="w-3 h-3" />
                                {extractPrId(url)}
                            </a>
                        ))}
                        {task.related_task_ids.length > 0 && (
                            <span className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-blue-100 text-blue-800 border border-blue-300 dark:bg-blue-900/50 dark:text-blue-300 dark:border-blue-700">
                                <Link2 className="w-3 h-3" />
                                {task.related_task_ids.length} related
                            </span>
                        )}
                        {task.status === 'done' && task.completed_at && (
                            <span className="text-[10px] text-text-muted">
                                Completed {formatTime(task.completed_at)}
                            </span>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
