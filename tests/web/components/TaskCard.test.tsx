import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TaskCard } from '@/components/task-board/TaskCard';
import type { Task } from '@/types/generated';

function makeTask(overrides: Partial<Task> = {}): Task {
    return {
        id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
        title: 'Test Task',
        description: null,
        status: 'backlog',
        priority: 0,
        assigned_agent_id: null,
        project_id: null,
        auto_approve: true,
        position: 1.0,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        ...overrides,
    };
}

describe('TaskCard', () => {
    it('renders task title', () => {
        render(<TaskCard task={makeTask()} onClick={() => {}} />);
        expect(screen.getByText('Test Task')).toBeInTheDocument();
    });

    it('renders description when present', () => {
        render(
            <TaskCard
                task={makeTask({ description: 'A description' })}
                onClick={() => {}}
            />,
        );
        expect(screen.getByText('A description')).toBeInTheDocument();
    });

    it('does not render description when null', () => {
        render(<TaskCard task={makeTask()} onClick={() => {}} />);
        expect(screen.queryByText('A description')).not.toBeInTheDocument();
    });

    it('renders priority badge', () => {
        render(<TaskCard task={makeTask({ priority: 3 })} onClick={() => {}} />);
        expect(screen.getByText('High')).toBeInTheDocument();
    });

    it('renders None for priority 0', () => {
        render(<TaskCard task={makeTask({ priority: 0 })} onClick={() => {}} />);
        expect(screen.getByText('None')).toBeInTheDocument();
    });

    it('calls onClick when clicked', async () => {
        const handleClick = vi.fn();
        const task = makeTask();
        render(<TaskCard task={task} onClick={handleClick} />);
        await userEvent.click(screen.getByRole('button'));
        expect(handleClick).toHaveBeenCalledWith(task);
    });

    it('shows agent name from agentNameMap', () => {
        const task = makeTask({ assigned_agent_id: 'agent-123' });
        render(
            <TaskCard
                task={task}
                onClick={() => {}}
                agentNameMap={{ 'agent-123': 'Claude' }}
            />,
        );
        expect(screen.getByText('Claude')).toBeInTheDocument();
    });

    it('shows project name from projectNameMap', () => {
        const task = makeTask({ project_id: 'proj-456' });
        render(
            <TaskCard
                task={task}
                onClick={() => {}}
                projectNameMap={{ 'proj-456': 'My Project' }}
            />,
        );
        expect(screen.getByText('My Project')).toBeInTheDocument();
    });

    it('does not show project badge when project_id is null', () => {
        render(<TaskCard task={makeTask()} onClick={() => {}} projectNameMap={{ 'proj-456': 'My Project' }} />);
        expect(screen.queryByText('My Project')).not.toBeInTheDocument();
    });
});
