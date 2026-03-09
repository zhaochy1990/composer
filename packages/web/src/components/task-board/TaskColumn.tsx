import type { Task } from '@/types/generated';
import { TaskCard } from './TaskCard';

interface TaskColumnProps {
    title: string;
    tasks: Task[];
    onEditTask: (task: Task) => void;
    agentNameMap?: Record<string, string>;
    projectNameMap?: Record<string, string>;
}

export function TaskColumn({ title, tasks, onEditTask, agentNameMap, projectNameMap }: TaskColumnProps) {
    return (
        <div className="flex-1 min-w-[280px] flex flex-col">
            <div className="flex items-center gap-2 mb-3 px-1">
                <h2 className="text-sm font-semibold text-text-muted uppercase">
                    {title}
                </h2>
                <span className="text-xs text-text-muted bg-bg-elevated px-1.5 py-0.5 rounded-full">
                    {tasks.length}
                </span>
            </div>
            <div className="flex-1 bg-bg-surface/50 rounded-lg p-2 space-y-2 overflow-y-auto">
                {tasks.length === 0 ? (
                    <p className="text-xs text-text-muted p-4 text-center">No tasks</p>
                ) : (
                    tasks.map(task => (
                        <TaskCard key={task.id} task={task} onClick={onEditTask} agentNameMap={agentNameMap} projectNameMap={projectNameMap} />
                    ))
                )}
            </div>
        </div>
    );
}
