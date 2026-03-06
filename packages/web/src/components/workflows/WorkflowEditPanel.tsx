import { useState, useEffect } from 'react';
import { X, Plus, Trash2, ChevronDown, ChevronRight, RotateCcw } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition, WorkflowStepType, SessionMode } from '@/types/generated';
import { useUpdateWorkflow, useDeleteWorkflow } from '@/hooks/use-workflows';

const STEP_TYPES: { value: WorkflowStepType; label: string }[] = [
    { value: 'agentic', label: 'Agentic' },
    { value: 'human_gate', label: 'Human Gate' },
];

const SESSION_MODES: { value: SessionMode; label: string }[] = [
    { value: 'new', label: 'New Session' },
    { value: 'resume', label: 'Resume Main Session' },
    { value: 'separate', label: 'Separate Session' },
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
        // Validate steps
        for (let i = 0; i < steps.length; i++) {
            const step = steps[i];
            if (step.step_type === 'agentic' && !step.prompt_template?.trim()) {
                return;
            }
            if (step.loop_back_to != null) {
                if (step.loop_back_to < 0 || step.loop_back_to >= i) {
                    return;
                }
            }
        }


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
        setSteps([...steps, { step_type: 'agentic', name: '', session_mode: 'resume' }]);
        setExpandedStep(steps.length);
    }

    function removeStep(index: number) {
        const newSteps = steps.filter((_, i) => i !== index).map((step, newIdx) => {
            if (step.loop_back_to == null) return step;

            let newTarget = step.loop_back_to;

            // Target was the removed step — clear
            if (newTarget === index) {
                return { ...step, loop_back_to: undefined, max_retries: undefined };
            }
            // Target was after the removed step — decrement
            if (newTarget > index) {
                newTarget = newTarget - 1;
            }
            // Target must still be a preceding step
            if (newTarget >= newIdx) {
                return { ...step, loop_back_to: undefined, max_retries: undefined };
            }

            return { ...step, loop_back_to: newTarget };
        });
        setSteps(newSteps);
        setExpandedStep(null);
    }

    function updateStep(index: number, updates: Partial<WorkflowStepDefinition>) {
        setSteps(steps.map((s, i) => i === index ? { ...s, ...updates } : s));
    }

    return (
        <div className="bg-gray-900 h-full overflow-y-auto flex flex-col">
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-gray-800">
                <h2 className="text-lg font-semibold text-gray-100 truncate">Edit Workflow</h2>
                <button onClick={onClose} className="text-gray-400 hover:text-gray-200 p-1 rounded hover:bg-gray-800">
                    <X className="w-4 h-4" />
                </button>
            </div>

            <div className="p-6 space-y-4 flex-1 max-w-3xl">
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
                                        {step.loop_back_to != null && (
                                            <span
                                                className="flex items-center gap-0.5 text-xs text-orange-400 shrink-0"
                                                title={`Loops back to step ${step.loop_back_to + 1}`}
                                            >
                                                <RotateCcw className="w-3 h-3" />
                                                <span className="font-mono">{step.loop_back_to + 1}</span>
                                            </span>
                                        )}
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
                                        <div className="px-3 pb-3 pt-1 border-t border-gray-700 space-y-2">
                                            {step.step_type === 'agentic' && (
                                                <div>
                                                    <label className="block text-xs text-gray-400 mb-1">Session Mode</label>
                                                    <select
                                                        value={step.session_mode ?? 'resume'}
                                                        onChange={e => updateStep(index, { session_mode: e.target.value as SessionMode })}
                                                        className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                    >
                                                        {SESSION_MODES.map(m => (
                                                            <option key={m.value} value={m.value}>{m.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            )}
                                            {step.step_type === 'agentic' && (
                                                <div>
                                                    <label className="block text-xs text-gray-400 mb-1">
                                                        Prompt Template <span className="text-red-400">*</span>
                                                    </label>
                                                    <textarea
                                                        value={step.prompt_template ?? ''}
                                                        onChange={e => updateStep(index, {
                                                            prompt_template: e.target.value || undefined,
                                                        })}
                                                        placeholder="Required. Use {{task}} for task context, {{step_N}} for prior step output, {{rejection}} for rejection feedback."
                                                        rows={4}
                                                        className="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-2 text-xs text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 resize-none font-mono"
                                                    />
                                                </div>
                                            )}

                                            {/* Loop Back To */}
                                            {index > 0 && (
                                                <div>
                                                    <label className="block text-xs text-gray-400 mb-1">
                                                        Loop Back To
                                                        <span className="text-gray-600 ml-1">(optional)</span>
                                                    </label>
                                                    <select
                                                        value={step.loop_back_to ?? ''}
                                                        onChange={e => {
                                                            const val = e.target.value;
                                                            if (val === '') {
                                                                updateStep(index, { loop_back_to: undefined, max_retries: undefined });
                                                            } else {
                                                                updateStep(index, { loop_back_to: Number(val) });
                                                            }
                                                        }}
                                                        className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                    >
                                                        <option value="">None (proceed to next step)</option>
                                                        {steps.slice(0, index).map((precedingStep, i) => (
                                                            <option key={i} value={i}>
                                                                Step {i + 1}: {precedingStep.name || precedingStep.step_type}
                                                            </option>
                                                        ))}
                                                    </select>
                                                    <p className="text-xs text-gray-600 mt-0.5">
                                                        After this step completes, loop back to the selected step for review-fix cycles.
                                                    </p>
                                                </div>
                                            )}

                                            {/* Max Retries (only when loop_back_to is set) */}
                                            {step.loop_back_to != null && (
                                                <div>
                                                    <label className="block text-xs text-gray-400 mb-1">
                                                        Max Retries
                                                        <span className="text-gray-600 ml-1">(optional)</span>
                                                    </label>
                                                    <input
                                                        type="number"
                                                        min={1}
                                                        max={10}
                                                        value={step.max_retries ?? ''}
                                                        onChange={e => {
                                                            const val = e.target.value;
                                                            updateStep(index, {
                                                                max_retries: val === '' ? undefined : Number(val),
                                                            });
                                                        }}
                                                        placeholder="Unlimited"
                                                        className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500"
                                                    />
                                                    <p className="text-xs text-gray-600 mt-0.5">
                                                        Number of times to repeat the loop before moving forward. Leave empty for unlimited.
                                                    </p>
                                                </div>
                                            )}
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
                        disabled={!name.trim() || steps.length === 0 || updateWorkflow.isPending || steps.some(s => s.step_type === 'agentic' && !s.prompt_template?.trim())}
                        className="px-3 py-1.5 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {updateWorkflow.isPending ? 'Saving...' : 'Save'}
                    </button>
                </div>
            </div>
        </div>
    );
}
