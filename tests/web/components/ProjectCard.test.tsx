import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProjectCard } from '@/components/projects/ProjectCard';
import type { Project } from '@/types/generated';

function makeProject(overrides: Partial<Project> = {}): Project {
    return {
        id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
        name: 'Test Project',
        description: undefined,
        created_at: '2024-01-15T00:00:00Z',
        updated_at: '2024-01-15T00:00:00Z',
        ...overrides,
    };
}

describe('ProjectCard', () => {
    it('renders project name', () => {
        render(<ProjectCard project={makeProject()} repoCount={0} onClick={() => {}} />);
        expect(screen.getByText('Test Project')).toBeInTheDocument();
    });

    it('renders description when present', () => {
        render(
            <ProjectCard
                project={makeProject({ description: 'A great project' })}
                repoCount={0}
                onClick={() => {}}
            />,
        );
        expect(screen.getByText('A great project')).toBeInTheDocument();
    });

    it('does not render description when absent', () => {
        render(<ProjectCard project={makeProject()} repoCount={0} onClick={() => {}} />);
        expect(screen.queryByText('A great project')).not.toBeInTheDocument();
    });

    it('shows singular "repo" for count 1', () => {
        render(<ProjectCard project={makeProject()} repoCount={1} onClick={() => {}} />);
        expect(screen.getByText('1 repo')).toBeInTheDocument();
    });

    it('shows plural "repos" for count != 1', () => {
        render(<ProjectCard project={makeProject()} repoCount={3} onClick={() => {}} />);
        expect(screen.getByText('3 repos')).toBeInTheDocument();
    });

    it('shows 0 repos', () => {
        render(<ProjectCard project={makeProject()} repoCount={0} onClick={() => {}} />);
        expect(screen.getByText('0 repos')).toBeInTheDocument();
    });

    it('calls onClick when clicked', async () => {
        const handleClick = vi.fn();
        render(<ProjectCard project={makeProject()} repoCount={0} onClick={handleClick} />);
        await userEvent.click(screen.getByRole('button'));
        expect(handleClick).toHaveBeenCalledOnce();
    });
});
