import { useEffect } from 'react';
import type { Task, TaskStatus } from '@/types/generated';
import { TaskListSection } from './TaskListSection';
import { TaskDetailPanel } from './TaskDetailPanel';
import { LayoutList } from 'lucide-react';

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
    selectedTask: Task | null;
    onCloseTask: () => void;
    agentNameMap: Record<string, string>;
    projectNameMap: Record<string, string>;
}

export function TaskListView({
    tasksByStatus,
    onEditTask,
    selectedTask,
    onCloseTask,
    agentNameMap,
    projectNameMap,
}: TaskListViewProps) {
    // Escape key deselects the current task
    useEffect(() => {
        function handleKeyDown(e: KeyboardEvent) {
            if (e.key === 'Escape' && selectedTask) {
                onCloseTask();
            }
        }
        document.addEventListener('keydown', handleKeyDown);
        return () => document.removeEventListener('keydown', handleKeyDown);
    }, [selectedTask, onCloseTask]);

    return (
        <div className="flex h-full">
            {/* Left panel — task list */}
            <div className="w-[380px] shrink-0 border-r border-gray-800 overflow-y-auto">
                <div className="border-b border-gray-800">
                    {sections.map((section) => (
                        <TaskListSection
                            key={section.status}
                            title={section.title}
                            tasks={tasksByStatus[section.status]}
                            onEditTask={onEditTask}
                            defaultCollapsed={section.status === 'done'}
                            agentNameMap={agentNameMap}
                            projectNameMap={projectNameMap}
                            selectedTaskId={selectedTask?.id}
                        />
                    ))}
                </div>
            </div>

            {/* Right panel — task detail (always visible) */}
            <div className="flex-1 min-w-0 overflow-hidden">
                {selectedTask ? (
                    <TaskDetailPanel
                        key={selectedTask.id}
                        task={selectedTask}
                        onClose={onCloseTask}
                        inline
                    />
                ) : (
                    <div className="flex flex-col items-center justify-center h-full text-gray-500">
                        <LayoutList className="w-10 h-10 mb-3 text-gray-600" />
                        <p className="text-sm">Select a task to view details</p>
                    </div>
                )}
            </div>
        </div>
    );
}
