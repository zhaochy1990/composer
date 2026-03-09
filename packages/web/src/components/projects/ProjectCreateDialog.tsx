import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus } from 'lucide-react';
import { useCreateProject } from '@/hooks/use-projects';

interface ProjectCreateDialogProps {
    isOpen: boolean;
    onClose: () => void;
}

export function ProjectCreateDialog({ isOpen, onClose }: ProjectCreateDialogProps) {
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');
    const createProject = useCreateProject();

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!name.trim()) return;

        createProject.mutate(
            {
                name: name.trim(),
                description: description.trim() || undefined,
            },
            {
                onSuccess: () => {
                    setName('');
                    setDescription('');
                    onClose();
                },
            },
        );
    }

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => { if (!open) { setName(''); setDescription(''); onClose(); } }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-[480px] bg-bg-surface border border-border-primary rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-text-primary">
                            New Project
                        </Dialog.Title>
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="text-text-muted hover:text-text-primary transition-colors p-1 rounded hover:bg-bg-elevated"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </Dialog.Close>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div>
                            <label htmlFor="project-name" className="block text-sm font-medium text-text-secondary mb-1">
                                Name <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="project-name"
                                type="text"
                                value={name}
                                onChange={e => setName(e.target.value)}
                                placeholder="Project name"
                                required
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>

                        <div>
                            <label htmlFor="project-description" className="block text-sm font-medium text-text-secondary mb-1">
                                Description
                            </label>
                            <textarea
                                id="project-description"
                                value={description}
                                onChange={e => setDescription(e.target.value)}
                                placeholder="Optional description"
                                rows={3}
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
                            />
                        </div>

                        <div className="flex justify-end gap-2 pt-2">
                            <Dialog.Close asChild>
                                <button
                                    type="button"
                                    className="px-4 py-2 text-sm text-text-secondary bg-bg-elevated border border-border-secondary rounded-md hover:bg-bg-interactive transition-colors"
                                >
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                type="submit"
                                disabled={!name.trim() || createProject.isPending}
                                className="flex items-center gap-1.5 px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                {createProject.isPending ? 'Creating...' : 'Create Project'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
