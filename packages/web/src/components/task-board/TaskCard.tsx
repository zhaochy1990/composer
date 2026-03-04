import { GripVertical, Play } from 'lucide-react';
import type { Task } from '@/types/generated';
import { shortId } from '@/lib/utils';

const priorityConfig: Record<number, { label: string; className: string }> = {
    3: { label: 'High', className: 'bg-red-900/60 text-red-300 border-red-700' },
    2: { label: 'Medium', className: 'bg-yellow-900/60 text-yellow-300 border-yellow-700' },
    1: { label: 'Low', className: 'bg-blue-900/60 text-blue-300 border-blue-700' },
    0: { label: 'None', className: 'bg-gray-800 text-gray-400 border-gray-600' },
};

interface TaskCardProps {
    task: Task;
    onClick: (task: Task) => void;
    agentNameMap?: Record<string, string>;
    onStart?: (taskId: string) => void;
    startingTaskId?: string | null;
}

export function TaskCard({ task, onClick, agentNameMap, onStart, startingTaskId }: TaskCardProps) {
    const priority = priorityConfig[task.priority] ?? priorityConfig[0];
    const canStart = task.status === 'backlog' && !!task.assigned_agent_id && !!task.repo_path;
    const isStarting = startingTaskId === task.id;

    return (
        <div
            role="button"
            tabIndex={0}
            onClick={() => onClick(task)}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick(task); }}
            className="w-full text-left bg-gray-800 border border-gray-700 rounded-md p-3 hover:border-gray-500 transition-colors cursor-pointer group"
        >
            <div className="flex items-start gap-2">
                <GripVertical className="w-4 h-4 text-gray-600 mt-0.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity" />
                <div className="flex-1 min-w-0">
                    <p className="font-semibold text-gray-100 text-sm leading-snug">
                        {task.title}
                    </p>
                    {task.description && (
                        <p className="text-sm text-gray-400 mt-1 line-clamp-2">
                            {task.description}
                        </p>
                    )}
                    <div className="flex items-center gap-2 mt-2">
                        <span
                            className={`inline-flex items-center text-xs px-1.5 py-0.5 rounded border ${priority.className}`}
                        >
                            {priority.label}
                        </span>
                        {task.assigned_agent_id && (
                            <span className="inline-flex items-center text-xs px-1.5 py-0.5 rounded bg-purple-900/50 text-purple-300 border border-purple-700">
                                {agentNameMap?.[task.assigned_agent_id] ?? shortId(task.assigned_agent_id)}
                            </span>
                        )}
                        {canStart && onStart && (
                            <button
                                type="button"
                                onClick={(e) => { e.stopPropagation(); onStart(task.id); }}
                                disabled={isStarting}
                                className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-green-600 text-white hover:bg-green-500 transition-colors disabled:opacity-50"
                            >
                                <Play className="w-3 h-3" />
                                {isStarting ? 'Starting...' : 'Start'}
                            </button>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
