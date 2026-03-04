import { useState } from 'react';
import { X, Plus, Trash2, GitBranch } from 'lucide-react';
import type { Project } from '@/types/generated';
import { useUpdateProject, useDeleteProject, useProjectRepositories, useRemoveProjectRepository } from '@/hooks/use-projects';
import { RepositoryAddDialog } from './RepositoryAddDialog';

interface ProjectDetailPanelProps {
    project: Project;
    onClose: () => void;
}

export function ProjectDetailPanel({ project, onClose }: ProjectDetailPanelProps) {
    const [isEditing, setIsEditing] = useState(false);
    const [editName, setEditName] = useState(project.name);
    const [editDescription, setEditDescription] = useState(project.description ?? '');
    const [addRepoOpen, setAddRepoOpen] = useState(false);

    const updateProject = useUpdateProject();
    const deleteProject = useDeleteProject();
    const { data: repos } = useProjectRepositories(project.id);
    const removeRepo = useRemoveProjectRepository();

    function handleSave() {
        updateProject.mutate(
            {
                id: project.id,
                name: editName.trim() || undefined,
                description: editDescription.trim() || undefined,
            },
            { onSuccess: () => setIsEditing(false) },
        );
    }

    function handleDelete() {
        deleteProject.mutate(project.id, { onSuccess: onClose });
    }

    return (
        <div className="w-96 border-l border-gray-800 bg-gray-900 h-full overflow-y-auto flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-gray-800">
                <h2 className="text-lg font-semibold text-gray-100 truncate">
                    {project.name}
                </h2>
                <button onClick={onClose} className="text-gray-400 hover:text-gray-200 p-1 rounded hover:bg-gray-800">
                    <X className="w-4 h-4" />
                </button>
            </div>

            <div className="p-4 space-y-4 flex-1">
                {isEditing ? (
                    <div className="space-y-3">
                        <div>
                            <label className="block text-sm font-medium text-gray-300 mb-1">Name</label>
                            <input
                                value={editName}
                                onChange={e => setEditName(e.target.value)}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium text-gray-300 mb-1">Description</label>
                            <textarea
                                value={editDescription}
                                onChange={e => setEditDescription(e.target.value)}
                                rows={3}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
                            />
                        </div>
                        <div className="flex gap-2">
                            <button
                                onClick={handleSave}
                                disabled={updateProject.isPending}
                                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-500 disabled:opacity-50"
                            >
                                Save
                            </button>
                            <button
                                onClick={() => { setIsEditing(false); setEditName(project.name); setEditDescription(project.description ?? ''); }}
                                className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700"
                            >
                                Cancel
                            </button>
                        </div>
                    </div>
                ) : (
                    <div>
                        {project.description && (
                            <p className="text-sm text-gray-400 mb-3">{project.description}</p>
                        )}
                        <div className="flex gap-2">
                            <button
                                onClick={() => setIsEditing(true)}
                                className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700"
                            >
                                Edit
                            </button>
                            <button
                                onClick={handleDelete}
                                disabled={deleteProject.isPending}
                                className="px-3 py-1.5 text-sm text-red-400 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 disabled:opacity-50"
                            >
                                Delete
                            </button>
                        </div>
                    </div>
                )}

                <div className="border-t border-gray-800 pt-4">
                    <div className="flex items-center justify-between mb-3">
                        <h3 className="text-sm font-medium text-gray-300">Repositories</h3>
                        <button
                            onClick={() => setAddRepoOpen(true)}
                            className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-800 text-gray-300 rounded hover:bg-gray-700"
                        >
                            <Plus className="w-3 h-3" />
                            Add
                        </button>
                    </div>

                    {repos && repos.length === 0 && (
                        <p className="text-sm text-gray-500">No repositories added yet.</p>
                    )}

                    {repos && repos.length > 0 && (
                        <div className="space-y-2">
                            {repos.map(repo => (
                                <div key={repo.id} className="flex items-start gap-2 p-2 bg-gray-800 rounded-md group">
                                    <GitBranch className="w-4 h-4 text-gray-500 mt-0.5 shrink-0" />
                                    <div className="flex-1 min-w-0">
                                        <p className="text-sm text-gray-200 truncate">
                                            {repo.display_name || repo.local_path.split(/[\\/]/).pop()}
                                        </p>
                                        <p className="text-xs text-gray-500 truncate font-mono" title={repo.local_path}>
                                            {repo.local_path}
                                        </p>
                                        <span className="text-xs px-1.5 py-0.5 rounded bg-gray-700 text-gray-400 mt-1 inline-block">
                                            {repo.role}
                                        </span>
                                    </div>
                                    <button
                                        onClick={() => removeRepo.mutate({ projectId: project.id, repoId: repo.id })}
                                        disabled={removeRepo.isPending}
                                        className="text-gray-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity p-1"
                                        title="Remove repository"
                                    >
                                        <Trash2 className="w-3.5 h-3.5" />
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>

            <RepositoryAddDialog
                isOpen={addRepoOpen}
                onClose={() => setAddRepoOpen(false)}
                projectId={project.id}
            />
        </div>
    );
}
