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
            className="bg-gray-900 border border-gray-800 rounded-lg p-4 text-left hover:border-gray-600 transition-colors w-full"
        >
            <div className="flex items-center gap-2 mb-2">
                <FolderGit2 className="w-4 h-4 text-blue-400" />
                <h3 className="font-bold text-gray-100 truncate">{project.name}</h3>
            </div>

            {project.description && (
                <p className="text-sm text-gray-400 mb-3 line-clamp-2">{project.description}</p>
            )}

            <div className="flex items-center gap-2 text-xs text-gray-500">
                <span className="px-2 py-0.5 rounded bg-gray-800 text-gray-300">
                    {repoCount} {repoCount === 1 ? 'repo' : 'repos'}
                </span>
                <span>Created {new Date(project.created_at).toLocaleDateString()}</span>
            </div>
        </button>
    );
}
