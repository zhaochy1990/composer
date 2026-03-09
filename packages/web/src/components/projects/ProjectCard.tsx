import { FolderGit2 } from 'lucide-react';
import type { Project } from '@/types/generated';

interface ProjectCardProps {
    project: Project;
    repoCount: number;
    onClick: () => void;
}

export function ProjectCard({ project, repoCount, onClick }: ProjectCardProps) {
    return (
        <button
            onClick={onClick}
            className="bg-bg-surface border border-border-primary rounded-lg p-4 text-left hover:border-border-secondary transition-colors w-full"
        >
            <div className="flex items-center gap-2 mb-2">
                <FolderGit2 className="w-4 h-4 text-blue-400" />
                <h3 className="font-bold text-text-primary truncate">{project.name}</h3>
            </div>

            {project.description && (
                <p className="text-sm text-text-muted mb-3 line-clamp-2">{project.description}</p>
            )}

            <div className="flex items-center gap-2 text-xs text-text-muted">
                <span className="px-2 py-0.5 rounded bg-bg-elevated text-text-secondary">
                    {repoCount} {repoCount === 1 ? 'repo' : 'repos'}
                </span>
                <span>Created {new Date(project.created_at).toLocaleDateString()}</span>
            </div>
        </button>
    );
}
