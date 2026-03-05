import { useState } from 'react';
import { X } from 'lucide-react';
import { useCreateWorkflow } from '@/hooks/use-workflows';

interface WorkflowCreateDialogProps {
    isOpen: boolean;
    onClose: () => void;
}

export function WorkflowCreateDialog({ isOpen, onClose }: WorkflowCreateDialogProps) {
    const [name, setName] = useState('');
    const createWorkflow = useCreateWorkflow();

    if (!isOpen) return null;

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!name.trim()) return;
        createWorkflow.mutate(
            {
                name: name.trim(),
                definition: {
                    steps: [
                        { step_type: 'plan', name: 'Plan' },
                        { step_type: 'human_gate', name: 'Review Plan' },
                        { step_type: 'implement', name: 'Implement' },
                    ],
                },
            },
            {
                onSuccess: () => {
                    setName('');
                    onClose();
                },
            },
        );
    }

    return (
        <>
            <div className="fixed inset-0 bg-black/50 z-50" onClick={onClose} />
            <div className="fixed inset-0 flex items-center justify-center z-50 pointer-events-none">
                <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-xl w-[400px] pointer-events-auto">
                    <div className="flex items-center justify-between px-4 py-3 border-b border-gray-800">
                        <h2 className="text-sm font-semibold text-gray-100">New Workflow</h2>
                        <button onClick={onClose} className="text-gray-400 hover:text-gray-200 p-1 rounded hover:bg-gray-800">
                            <X className="w-4 h-4" />
                        </button>
                    </div>
                    <form onSubmit={handleSubmit} className="p-4 space-y-4">
                        <div>
                            <label htmlFor="wf-name" className="block text-sm font-medium text-gray-300 mb-1">
                                Workflow Name <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="wf-name"
                                value={name}
                                onChange={e => setName(e.target.value)}
                                placeholder="e.g., Bug Fix, Refactoring"
                                required
                                autoFocus
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                            <p className="text-xs text-gray-500 mt-1">
                                A starter workflow with Plan, Review, and Implement steps will be created. You can customize it after.
                            </p>
                        </div>
                        <div className="flex justify-end gap-2">
                            <button type="button" onClick={onClose}
                                className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700">
                                Cancel
                            </button>
                            <button type="submit" disabled={!name.trim() || createWorkflow.isPending}
                                className="px-3 py-1.5 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 disabled:opacity-50">
                                {createWorkflow.isPending ? 'Creating...' : 'Create'}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </>
    );
}
