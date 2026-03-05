import { useState, useEffect } from 'react';
import { X, Plus, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition, WorkflowStepType } from '@/types/generated';
import { useUpdateWorkflow, useDeleteWorkflow } from '@/hooks/use-workflows';

const STEP_TYPES: { value: WorkflowStepType; label: string }[] = [
    { value: 'plan', label: 'Plan' },
    { value: 'human_gate', label: 'Human Gate' },
    { value: 'implement', label: 'Implement' },
    { value: 'pr_review', label: 'PR Review' },
    { value: 'human_review', label: 'Human Review' },
    { value: 'complete_pr', label: 'Complete PR' },
];

interface WorkflowEditPanelProps {
    workflow: Workflow;
    onClose: () => void;
}

export function WorkflowEditPanel({ workflow, onClose }: WorkflowEditPanelProps) {
    const [name, setName] = useState(workflow.name);
    const [steps, setSteps] = useState<WorkflowStepDefinition[]>(workflow.definition.steps);
    const [expandedStep, setExpandedStep] = useState<number | null>(null);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

    const updateWorkflow = useUpdateWorkflow();
    const deleteWorkflow = useDeleteWorkflow();

    useEffect(() => {
        setName(workflow.name);
        setSteps(workflow.definition.steps);
        setShowDeleteConfirm(false);
    }, [workflow.id]);

    function handleSave() {
        updateWorkflow.mutate({
            id: workflow.id,
            name: name.trim() || undefined,
            definition: { steps },
        }, {
            onSuccess: onClose,
        });
    }

    function handleDelete() {
        deleteWorkflow.mutate(workflow.id, { onSuccess: onClose });
    }

    function addStep() {
        setSteps([...steps, { step_type: 'implement', name: '' }]);
        setExpandedStep(steps.length);
    }

    function removeStep(index: number) {
        setSteps(steps.filter((_, i) => i !== index));
        setExpandedStep(null);
    }

    function updateStep(index: number, updates: Partial<WorkflowStepDefinition>) {
        setSteps(steps.map((s, i) => i === index ? { ...s, ...updates } : s));
    }

    return (
        <div className="w-[480px] border-l border-gray-800 bg-gray-900 h-full overflow-y-auto flex flex-col">
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-gray-800">
                <h2 className="text-lg font-semibold text-gray-100 truncate">Edit Workflow</h2>
                <button onClick={onClose} className="text-gray-400 hover:text-gray-200 p-1 rounded hover:bg-gray-800">
                    <X className="w-4 h-4" />
                </button>
            </div>

            <div className="p-4 space-y-4 flex-1">
                {/* Name */}
                <div>
                    <label className="block text-sm font-medium text-gray-300 mb-1">Name</label>
                    <input
                        value={name}
                        onChange={e => setName(e.target.value)}
                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                    />
                </div>

                {/* Steps */}
                <div>
                    <div className="flex items-center justify-between mb-2">
                        <label className="block text-sm font-medium text-gray-300">Steps</label>
                        <button
                            onClick={addStep}
                            className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-800 text-gray-300 rounded hover:bg-gray-700"
                        >
                            <Plus className="w-3 h-3" />
                            Add Step
                        </button>
                    </div>

                    {steps.length === 0 && (
                        <p className="text-sm text-gray-500 py-4 text-center">No steps defined. Add one to get started.</p>
                    )}

                    <div className="space-y-1.5">
                        {steps.map((step, index) => {
                            const isExpanded = expandedStep === index;
                            return (
                                <div key={index} className="bg-gray-800 rounded-md border border-gray-700">
                                    {/* Step header */}
                                    <div className="flex items-center gap-2 px-3 py-2">
                                        <span className="text-xs text-gray-500 font-mono w-5">{index + 1}</span>
                                        <button
                                            type="button"
                                            onClick={() => setExpandedStep(isExpanded ? null : index)}
                                            className="text-gray-400 hover:text-gray-200"
                                        >
                                            {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
                                        </button>
                                        <select
                                            value={step.step_type}
                                            onChange={e => updateStep(index, { step_type: e.target.value as WorkflowStepType })}
                                            className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                        >
                                            {STEP_TYPES.map(t => (
                                                <option key={t.value} value={t.value}>{t.label}</option>
                                            ))}
                                        </select>
                                        <input
                                            value={step.name}
                                            onChange={e => updateStep(index, { name: e.target.value })}
                                            placeholder="Step name"
                                            className="flex-1 bg-transparent border-none text-sm text-gray-200 placeholder-gray-500 focus:outline-none"
                                        />
                                        <button
                                            onClick={() => removeStep(index)}
                                            className="text-gray-600 hover:text-red-400 p-1"
                                            title="Remove step"
                                        >
                                            <Trash2 className="w-3.5 h-3.5" />
                                        </button>
                                    </div>

                                    {/* Expanded details */}
                                    {isExpanded && (
                                        <div className="px-3 pb-3 pt-1 border-t border-gray-700">
                                            <label className="block text-xs text-gray-400 mb-1">
                                                Prompt Template (optional)
                                            </label>
                                            <textarea
                                                value={step.prompt_template ?? ''}
                                                onChange={e => updateStep(index, {
                                                    prompt_template: e.target.value || undefined,
                                                })}
                                                placeholder="Custom prompt for this step. Use {{task}} for task context, {{step_N}} for prior step output."
                                                rows={3}
                                                className="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-2 text-xs text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 resize-none font-mono"
                                            />
                                        </div>
                                    )}
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>

            {/* Footer */}
            <div className="p-4 border-t border-gray-800 flex items-center justify-between">
                <div>
                    {!showDeleteConfirm ? (
                        <button
                            onClick={() => setShowDeleteConfirm(true)}
                            className="flex items-center gap-1 px-3 py-1.5 text-sm text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-md transition-colors"
                        >
                            <Trash2 className="w-3.5 h-3.5" />
                            Delete
                        </button>
                    ) : (
                        <div className="flex items-center gap-2">
                            <span className="text-sm text-red-400">Delete?</span>
                            <button onClick={handleDelete} disabled={deleteWorkflow.isPending}
                                className="px-3 py-1 text-sm text-white bg-red-600 rounded-md hover:bg-red-500 disabled:opacity-50">
                                {deleteWorkflow.isPending ? '...' : 'Yes'}
                            </button>
                            <button onClick={() => setShowDeleteConfirm(false)}
                                className="px-3 py-1 text-sm text-gray-300 bg-gray-800 rounded-md hover:bg-gray-700">
                                No
                            </button>
                        </div>
                    )}
                </div>
                <div className="flex gap-2">
                    <button onClick={onClose}
                        className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700">
                        Cancel
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={!name.trim() || steps.length === 0 || updateWorkflow.isPending}
                        className="px-3 py-1.5 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {updateWorkflow.isPending ? 'Saving...' : 'Save'}
                    </button>
                </div>
            </div>
        </div>
    );
}
