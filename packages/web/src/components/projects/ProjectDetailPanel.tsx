import { useState } from 'react';
import { X, Plus, Trash2, Pencil, GitBranch, BookOpen } from 'lucide-react';
import type { Project, ProjectInstruction } from '@/types/generated';
import { useUpdateProject, useDeleteProject, useProjectRepositories, useRemoveProjectRepository, useProjectInstructions, useRemoveProjectInstruction } from '@/hooks/use-projects';
import { RepositoryAddDialog } from './RepositoryAddDialog';
import { InstructionAddDialog } from './InstructionAddDialog';

interface ProjectDetailPanelProps {
    project: Project;
    onClose: () => void;
}

export function ProjectDetailPanel({ project, onClose }: ProjectDetailPanelProps) {
    const [isEditing, setIsEditing] = useState(false);
    const [editName, setEditName] = useState(project.name);
    const [editDescription, setEditDescription] = useState(project.description ?? '');
    const [addRepoOpen, setAddRepoOpen] = useState(false);
    const [addInstrOpen, setAddInstrOpen] = useState(false);
    const [editingInstruction, setEditingInstruction] = useState<ProjectInstruction | undefined>();

    const updateProject = useUpdateProject();
    const deleteProject = useDeleteProject();
    const { data: repos } = useProjectRepositories(project.id);
    const removeRepo = useRemoveProjectRepository();
    const { data: instructions } = useProjectInstructions(project.id);
    const removeInstruction = useRemoveProjectInstruction();

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
        <div className="w-96 border-l border-border-primary bg-bg-surface h-full overflow-y-auto flex flex-col">
            <div className="flex items-center justify-between p-4 border-b border-border-primary">
                <h2 className="text-lg font-semibold text-text-primary truncate">
                    {project.name}
                </h2>
                <button onClick={onClose} className="text-text-muted hover:text-text-primary p-1 rounded hover:bg-bg-elevated">
                    <X className="w-4 h-4" />
                </button>
            </div>

            <div className="p-4 space-y-4 flex-1">
                {isEditing ? (
                    <div className="space-y-3">
                        <div>
                            <label className="block text-sm font-medium text-text-secondary mb-1">Name</label>
                            <input
                                value={editName}
                                onChange={e => setEditName(e.target.value)}
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium text-text-secondary mb-1">Description</label>
                            <textarea
                                value={editDescription}
                                onChange={e => setEditDescription(e.target.value)}
                                rows={3}
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
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
                                className="px-3 py-1.5 text-sm text-text-secondary bg-bg-elevated border border-border-secondary rounded-md hover:bg-bg-interactive"
                            >
                                Cancel
                            </button>
                        </div>
                    </div>
                ) : (
                    <div>
                        {project.description && (
                            <p className="text-sm text-text-muted mb-3">{project.description}</p>
                        )}
                        <div className="flex gap-2">
                            <button
                                onClick={() => setIsEditing(true)}
                                className="px-3 py-1.5 text-sm text-text-secondary bg-bg-elevated border border-border-secondary rounded-md hover:bg-bg-interactive"
                            >
                                Edit
                            </button>
                            <button
                                onClick={handleDelete}
                                disabled={deleteProject.isPending}
                                className="px-3 py-1.5 text-sm text-red-400 bg-bg-elevated border border-border-secondary rounded-md hover:bg-bg-interactive disabled:opacity-50"
                            >
                                Delete
                            </button>
                        </div>
                    </div>
                )}

                <div className="border-t border-border-primary pt-4">
                    <div className="flex items-center justify-between mb-3">
                        <h3 className="text-sm font-medium text-text-secondary">Repositories</h3>
                        <button
                            onClick={() => setAddRepoOpen(true)}
                            className="flex items-center gap-1 px-2 py-1 text-xs bg-bg-elevated text-text-secondary rounded hover:bg-bg-interactive"
                        >
                            <Plus className="w-3 h-3" />
                            Add
                        </button>
                    </div>

                    {repos && repos.length === 0 && (
                        <p className="text-sm text-text-muted">No repositories added yet.</p>
                    )}

                    {repos && repos.length > 0 && (
                        <div className="space-y-2">
                            {repos.map(repo => (
                                <div key={repo.id} className="flex items-start gap-2 p-2 bg-bg-elevated rounded-md group">
                                    <GitBranch className="w-4 h-4 text-text-muted mt-0.5 shrink-0" />
                                    <div className="flex-1 min-w-0">
                                        <p className="text-sm text-text-primary truncate">
                                            {repo.display_name || repo.local_path.split(/[\\/]/).pop()}
                                        </p>
                                        <p className="text-xs text-text-muted truncate font-mono" title={repo.local_path}>
                                            {repo.local_path}
                                        </p>
                                        <span className="text-xs px-1.5 py-0.5 rounded bg-bg-interactive text-text-muted mt-1 inline-block">
                                            {repo.role}
                                        </span>
                                    </div>
                                    <button
                                        onClick={() => removeRepo.mutate({ projectId: project.id, repoId: repo.id })}
                                        disabled={removeRepo.isPending}
                                        className="text-text-muted hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity p-1"
                                        title="Remove repository"
                                    >
                                        <Trash2 className="w-3.5 h-3.5" />
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>

                <div className="border-t border-border-primary pt-4">
                    <div className="flex items-center justify-between mb-3">
                        <h3 className="text-sm font-medium text-text-secondary">Instructions</h3>
                        <button
                            onClick={() => { setEditingInstruction(undefined); setAddInstrOpen(true); }}
                            className="flex items-center gap-1 px-2 py-1 text-xs bg-bg-elevated text-text-secondary rounded hover:bg-bg-interactive"
                        >
                            <Plus className="w-3 h-3" />
                            Add
                        </button>
                    </div>

                    {instructions && instructions.length === 0 && (
                        <p className="text-sm text-text-muted">No instructions added yet.</p>
                    )}

                    {instructions && instructions.length > 0 && (
                        <div className="space-y-2">
                            {instructions.map(instr => (
                                <div key={instr.id} className="p-2 bg-bg-elevated rounded-md group">
                                    <div className="flex items-start gap-2">
                                        <BookOpen className="w-4 h-4 text-text-muted mt-0.5 shrink-0" />
                                        <div className="flex-1 min-w-0">
                                            <p className="text-sm text-text-primary font-medium">{instr.title}</p>
                                            <p className="text-xs text-text-muted mt-1 line-clamp-2">{instr.content}</p>
                                        </div>
                                        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                            <button
                                                onClick={() => { setEditingInstruction(instr); setAddInstrOpen(true); }}
                                                className="text-text-muted hover:text-blue-400 p-1"
                                                title="Edit instruction"
                                            >
                                                <Pencil className="w-3.5 h-3.5" />
                                            </button>
                                            <button
                                                onClick={() => removeInstruction.mutate({ projectId: project.id, instructionId: instr.id })}
                                                disabled={removeInstruction.isPending}
                                                className="text-text-muted hover:text-red-400 p-1"
                                                title="Remove instruction"
                                            >
                                                <Trash2 className="w-3.5 h-3.5" />
                                            </button>
                                        </div>
                                    </div>
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
            <InstructionAddDialog
                isOpen={addInstrOpen}
                onClose={() => { setAddInstrOpen(false); setEditingInstruction(undefined); }}
                projectId={project.id}
                instruction={editingInstruction}
            />
        </div>
    );
}
