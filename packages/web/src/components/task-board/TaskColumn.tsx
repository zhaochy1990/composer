import { Plus } from 'lucide-react';
import type { Task, TaskStatus } from '@/types/generated';
import { TaskCard } from './TaskCard';

interface TaskColumnProps {
    status: TaskStatus;
    title: string;
    tasks: Task[];
    onCreateTask: (status: TaskStatus) => void;
    onEditTask: (task: Task) => void;
    agentNameMap?: Record<string, string>;
    projectNameMap?: Record<string, string>;
    onStartTask?: (taskId: string) => void;
    startingTaskId?: string | null;
}

export function TaskColumn({ status, title, tasks, onCreateTask, onEditTask, agentNameMap, projectNameMap, onStartTask, startingTaskId }: TaskColumnProps) {
    return (
        <div className="flex-1 min-w-[280px] flex flex-col">
            <div className="flex items-center justify-between mb-3 px-1">
                <div className="flex items-center gap-2">
                    <h2 className="text-sm font-semibold text-gray-400 uppercase">
                        {title}
                    </h2>
                    <span className="text-xs text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded-full">
                        {tasks.length}
                    </span>
                </div>
                <button
                    type="button"
                    onClick={() => onCreateTask(status)}
                    className="text-gray-500 hover:text-gray-300 transition-colors p-1 rounded hover:bg-gray-800"
                    title={`Add task to ${title}`}
                >
                    <Plus className="w-4 h-4" />
                </button>
            </div>
            <div className="flex-1 bg-gray-900/50 rounded-lg p-2 space-y-2 overflow-y-auto">
                {tasks.length === 0 ? (
                    <p className="text-xs text-gray-600 p-4 text-center">No tasks</p>
                ) : (
                    tasks.map(task => (
                        <TaskCard key={task.id} task={task} onClick={onEditTask} agentNameMap={agentNameMap} projectNameMap={projectNameMap} onStart={onStartTask} startingTaskId={startingTaskId} />
                    ))
                )}
            </div>
        </div>
    );
}
