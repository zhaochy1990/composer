import type { Task, TaskStatus } from '@/types/generated';
import { TaskListSection } from './TaskListSection';

// Ordered by urgency: Waiting (needs human action) first, then active work, then backlog, done last
const sections: { status: TaskStatus; title: string }[] = [
    { status: 'waiting', title: 'Waiting' },
    { status: 'in_progress', title: 'In Progress' },
    { status: 'backlog', title: 'Backlog' },
    { status: 'done', title: 'Done' },
];

interface TaskListViewProps {
    tasksByStatus: Record<TaskStatus, Task[]>;
    onEditTask: (task: Task) => void;
}

export function TaskListView({ tasksByStatus, onEditTask }: TaskListViewProps) {
    return (
        <div className="h-full overflow-y-auto p-6">
            <div className="max-w-4xl mx-auto border border-gray-800 rounded-lg overflow-hidden">
                {sections.map((section) => (
                    <TaskListSection
                        key={section.status}
                        title={section.title}
                        tasks={tasksByStatus[section.status]}
                        onEditTask={onEditTask}
                        defaultCollapsed={section.status === 'done'}
                    />
                ))}
            </div>
        </div>
    );
}
