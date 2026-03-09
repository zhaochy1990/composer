import { useState } from 'react';
import { X } from 'lucide-react';
import type { WorkflowStepDefinition } from '@/types/generated';
import { useCreateWorkflow } from '@/hooks/use-workflows';

const DEFAULT_STEPS: WorkflowStepDefinition[] = [
    {
        id: 'plan',
        step_type: 'agentic',
        name: 'Plan',
        depends_on: [],
        session_mode: 'new',
        prompt_template: '{{task}}\n\nInvestigate the existing codebase and create a detailed implementation plan. Do NOT implement yet. Only output the plan.{{rejection}}',
    },
    {
        id: 'review_plan',
        step_type: 'human_gate',
        name: 'Review Plan',
        depends_on: ['plan'],
        on_approve: 'implement',
        on_reject: 'plan',
    },
    {
        id: 'implement',
        step_type: 'agentic',
        name: 'Implement',
        depends_on: ['review_plan'],
        session_mode: 'resume',
        prompt_template: '{{task}}\n\nThe plan has been approved. Implement it now. After implementation, run build, lint, and tests. Fix any failures. Then create a PR.\n\nApproved plan:\n{{step:plan}}',
    },
];

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
                    steps: DEFAULT_STEPS,
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
                <div className="bg-bg-surface border border-border-primary rounded-lg shadow-xl w-[400px] pointer-events-auto">
                    <div className="flex items-center justify-between px-4 py-3 border-b border-border-primary">
                        <h2 className="text-sm font-semibold text-text-primary">New Workflow</h2>
                        <button onClick={onClose} className="text-text-muted hover:text-text-primary p-1 rounded hover:bg-bg-elevated">
                            <X className="w-4 h-4" />
                        </button>
                    </div>
                    <form onSubmit={handleSubmit} className="p-4 space-y-4">
                        <div>
                            <label htmlFor="wf-name" className="block text-sm font-medium text-text-secondary mb-1">
                                Workflow Name <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="wf-name"
                                value={name}
                                onChange={e => setName(e.target.value)}
                                placeholder="e.g., Bug Fix, Refactoring"
                                required
                                autoFocus
                                className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                            <p className="text-xs text-text-muted mt-1">
                                A starter workflow with Plan, Review, and Implement steps will be created. You can customize it after.
                            </p>
                        </div>
                        <div className="flex justify-end gap-2">
                            <button type="button" onClick={onClose}
                                className="px-3 py-1.5 text-sm text-text-secondary bg-bg-elevated border border-border-secondary rounded-md hover:bg-bg-interactive">
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
