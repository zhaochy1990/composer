import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus, Save } from 'lucide-react';
import { useAddProjectInstruction, useUpdateProjectInstruction } from '@/hooks/use-projects';
import type { ProjectInstruction } from '@/types/generated';

interface InstructionAddDialogProps {
    isOpen: boolean;
    onClose: () => void;
    projectId: string;
    instruction?: ProjectInstruction;
}

export function InstructionAddDialog({ isOpen, onClose, projectId, instruction }: InstructionAddDialogProps) {
    const [title, setTitle] = useState('');
    const [content, setContent] = useState('');
    const addInstruction = useAddProjectInstruction();
    const updateInstruction = useUpdateProjectInstruction();

    const isEditing = !!instruction;

    useEffect(() => {
        if (instruction) {
            setTitle(instruction.title);
            setContent(instruction.content);
        } else {
            setTitle('');
            setContent('');
        }
    }, [instruction, isOpen]);

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!title.trim() || !content.trim()) return;

        if (isEditing) {
            updateInstruction.mutate(
                {
                    projectId,
                    instructionId: instruction!.id,
                    title: title.trim(),
                    content: content.trim(),
                },
                {
                    onSuccess: () => {
                        resetAndClose();
                    },
                },
            );
        } else {
            addInstruction.mutate(
                {
                    projectId,
                    title: title.trim(),
                    content: content.trim(),
                },
                {
                    onSuccess: () => {
                        resetAndClose();
                    },
                },
            );
        }
    }

    function resetAndClose() {
        setTitle('');
        setContent('');
        onClose();
    }

    const isPending = addInstruction.isPending || updateInstruction.isPending;

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => { if (!open) resetAndClose(); }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-[520px] bg-bg-surface border border-border-primary rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-text-primary">
                            {isEditing ? 'Edit Instruction' : 'Add Instruction'}
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
                            <label htmlFor="instr-title" className="block text-sm font-medium text-text-secondary mb-1">
                                Title <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="instr-title"
                                type="text"
                                value={title}
                                onChange={e => setTitle(e.target.value)}
                                placeholder="e.g., Coding Standards, Architecture Guidelines"
                                required
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>

                        <div>
                            <label htmlFor="instr-content" className="block text-sm font-medium text-text-secondary mb-1">
                                Content <span className="text-red-400">*</span>
                            </label>
                            <textarea
                                id="instr-content"
                                value={content}
                                onChange={e => setContent(e.target.value)}
                                placeholder="Instructions for the coding agents..."
                                required
                                rows={8}
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-y"
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
                                disabled={!title.trim() || !content.trim() || isPending}
                                className="flex items-center gap-1.5 px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {isEditing ? <Save className="w-3.5 h-3.5" /> : <Plus className="w-3.5 h-3.5" />}
                                {isPending ? 'Saving...' : isEditing ? 'Save' : 'Add Instruction'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
