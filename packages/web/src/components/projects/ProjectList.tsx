import { useState } from 'react';
import { FolderGit2, Plus } from 'lucide-react';
import { useProjects, useProjectRepositories } from '@/hooks/use-projects';
import type { Project } from '@/types/generated';
import { ProjectCard } from './ProjectCard';
import { ProjectCreateDialog } from './ProjectCreateDialog';
import { ProjectDetailPanel } from './ProjectDetailPanel';

function ProjectCardWithRepoCount({ project, onClick }: { project: Project; onClick: () => void }) {
    const { data: repos } = useProjectRepositories(project.id);
    return <ProjectCard project={project} repoCount={repos?.length ?? 0} onClick={onClick} />;
}

export function ProjectList() {
    const [createOpen, setCreateOpen] = useState(false);
    const [selectedProject, setSelectedProject] = useState<Project | null>(null);
    const { data: projects, isLoading, isError } = useProjects();

    return (
        <div className="h-full flex">
            <div className="flex-1 overflow-y-auto p-6">
                <div className="flex items-center justify-between mb-6">
                    <h1 className="text-xl font-bold text-text-primary">Projects</h1>
                    <button
                        onClick={() => setCreateOpen(true)}
                        className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
                    >
                        <Plus className="w-4 h-4" />
                        New Project
                    </button>
                </div>

                {isLoading && (
                    <div className="flex items-center justify-center h-64">
                        <p className="text-sm text-text-muted">Loading projects...</p>
                    </div>
                )}

                {isError && (
                    <div className="flex items-center justify-center h-64">
                        <p className="text-sm text-red-400">Failed to load projects.</p>
                    </div>
                )}

                {!isLoading && !isError && projects && projects.length === 0 && (
                    <div className="flex flex-col items-center justify-center h-64 text-center">
                        <FolderGit2 className="w-12 h-12 text-text-muted mb-4" />
                        <p className="text-sm text-text-muted">
                            No projects yet. Create one to organize your repositories.
                        </p>
                    </div>
                )}

                {!isLoading && !isError && projects && projects.length > 0 && (
                    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                        {projects.map(project => (
                            <ProjectCardWithRepoCount
                                key={project.id}
                                project={project}
                                onClick={() => setSelectedProject(project)}
                            />
                        ))}
                    </div>
                )}

                <ProjectCreateDialog isOpen={createOpen} onClose={() => setCreateOpen(false)} />
            </div>

            {selectedProject && (
                <ProjectDetailPanel
                    project={selectedProject}
                    onClose={() => setSelectedProject(null)}
                />
            )}
        </div>
    );
}
