import { useState, useEffect, useMemo } from 'react';
import { X, Plus, Trash2, ChevronDown, ChevronRight, AlertCircle } from 'lucide-react';
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

function validateDag(steps: WorkflowStepDefinition[]): string[] {
    const errors: string[] = [];
    const ids = new Set(steps.map(s => s.id));

    // Check duplicate IDs
    const seen = new Set<string>();
    for (const step of steps) {
        if (!step.id.trim()) {
            errors.push(`Step "${step.name || '(unnamed)'}" has no ID`);
        } else if (seen.has(step.id)) {
            errors.push(`Duplicate step ID: "${step.id}"`);
        }
        seen.add(step.id);
    }

    for (const step of steps) {
        if (step.step_type === 'agentic' && !step.prompt_template?.trim()) {
            errors.push(`Step "${step.id}" is agentic but has no prompt template`);
        }
        if (step.step_type === 'human_gate' && !step.on_approve) {
            errors.push(`HumanGate step "${step.id}" is missing on_approve`);
        }
        for (const dep of step.depends_on) {
            if (!ids.has(dep)) {
                errors.push(`Step "${step.id}" depends on non-existent step "${dep}"`);
            }
        }
        if (step.on_approve && !ids.has(step.on_approve)) {
            errors.push(`Step "${step.id}" on_approve references non-existent step "${step.on_approve}"`);
        }
        if (step.on_reject && !ids.has(step.on_reject)) {
            errors.push(`Step "${step.id}" on_reject references non-existent step "${step.on_reject}"`);
        }
        if (step.loop_back_to && !ids.has(step.loop_back_to)) {
            errors.push(`Step "${step.id}" loop_back_to references non-existent step "${step.loop_back_to}"`);
        }
    }

    // Cycle detection (topological sort on depends_on)
    if (errors.length === 0) {
        const inDegree = new Map<string, number>();
        const adj = new Map<string, string[]>();
        for (const step of steps) {
            inDegree.set(step.id, 0);
            adj.set(step.id, []);
        }
        for (const step of steps) {
            for (const dep of step.depends_on) {
                adj.get(dep)?.push(step.id);
                inDegree.set(step.id, (inDegree.get(step.id) ?? 0) + 1);
            }
        }
        const queue: string[] = [];
        for (const [id, deg] of inDegree) {
            if (deg === 0) queue.push(id);
        }
        let visited = 0;
        while (queue.length > 0) {
            const node = queue.shift()!;
            visited++;
            for (const n of adj.get(node) ?? []) {
                const deg = (inDegree.get(n) ?? 1) - 1;
                inDegree.set(n, deg);
                if (deg === 0) queue.push(n);
            }
        }
        if (visited !== steps.length) {
            errors.push('Workflow definition contains a cycle');
        }
    }

    return errors;
}

export function WorkflowEditPanel({ workflow, onClose }: WorkflowEditPanelProps) {
    const [name, setName] = useState(workflow.name);
    const [steps, setSteps] = useState<WorkflowStepDefinition[]>(workflow.definition.steps);
    const [expandedStep, setExpandedStep] = useState<string | null>(null);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

    const updateWorkflow = useUpdateWorkflow();
    const deleteWorkflow = useDeleteWorkflow();

    useEffect(() => {
        setName(workflow.name);
        setSteps(workflow.definition.steps);
        setShowDeleteConfirm(false);
    }, [workflow.id]);

    const validationErrors = useMemo(() => validateDag(steps), [steps]);
    const hasErrors = validationErrors.length > 0;

    const stepIds = useMemo(() => steps.map(s => s.id), [steps]);

    function handleSave() {
        if (hasErrors) return;
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
        const newId = `step_${Date.now()}`;
        setSteps([...steps, {
            id: newId,
            step_type: 'agentic',
            name: '',
            depends_on: [],
            session_mode: 'resume',
        }]);
        setExpandedStep(newId);
    }

    function removeStep(id: string) {
        setSteps(steps.filter(s => s.id !== id));
        setExpandedStep(null);
    }

    function updateStep(id: string, updates: Partial<WorkflowStepDefinition>) {
        setSteps(steps.map(s => s.id === id ? { ...s, ...updates } : s));
    }

    if (workflow.is_template) {
        return (
            <div className="w-[480px] border-l border-gray-800 bg-gray-900 h-full overflow-y-auto flex flex-col">
                <div className="flex items-center justify-between p-4 border-b border-gray-800">
                    <h2 className="text-lg font-semibold text-gray-100 truncate">View Template</h2>
                    <button onClick={onClose} className="text-gray-400 hover:text-gray-200 p-1 rounded hover:bg-gray-800">
                        <X className="w-4 h-4" />
                    </button>
                </div>
                <div className="p-4 space-y-4 flex-1">
                    <p className="text-sm text-gray-400">This is a read-only template. Clone it to create an editable copy.</p>
                    <div>
                        <label className="block text-sm font-medium text-gray-300 mb-1">Name</label>
                        <p className="text-sm text-gray-100">{workflow.name}</p>
                    </div>
                    <div>
                        <label className="block text-sm font-medium text-gray-300 mb-2">Steps ({steps.length})</label>
                        <div className="space-y-1.5">
                            {steps.map((step) => (
                                <div key={step.id} className="bg-gray-800 rounded-md border border-gray-700 px-3 py-2">
                                    <div className="flex items-center gap-2">
                                        <span className="text-xs text-gray-500 font-mono">{step.id}</span>
                                        <span className="text-sm text-gray-200">{step.name}</span>
                                        <span className="text-xs text-gray-500 ml-auto">{step.step_type}</span>
                                    </div>
                                    {step.depends_on.length > 0 && (
                                        <p className="text-xs text-gray-500 mt-1">depends on: {step.depends_on.join(', ')}</p>
                                    )}
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
                <div className="p-4 border-t border-gray-800 flex justify-end">
                    <button onClick={onClose}
                        className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700">
                        Close
                    </button>
                </div>
            </div>
        );
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

                {/* Validation errors */}
                {hasErrors && (
                    <div className="bg-red-900/20 border border-red-700 rounded-md p-3 space-y-1">
                        {validationErrors.map((err, i) => (
                            <div key={i} className="flex items-start gap-2">
                                <AlertCircle className="w-3.5 h-3.5 text-red-400 mt-0.5 shrink-0" />
                                <span className="text-xs text-red-300">{err}</span>
                            </div>
                        ))}
                    </div>
                )}

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
                        {steps.map((step) => {
                            const isExpanded = expandedStep === step.id;
                            return (
                                <div key={step.id} className="bg-gray-800 rounded-md border border-gray-700">
                                    {/* Step header */}
                                    <div className="flex items-center gap-2 px-3 py-2">
                                        <button
                                            type="button"
                                            onClick={() => setExpandedStep(isExpanded ? null : step.id)}
                                            className="text-gray-400 hover:text-gray-200"
                                        >
                                            {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
                                        </button>
                                        <span className="text-xs text-gray-500 font-mono shrink-0">{step.id}</span>
                                        <select
                                            value={step.step_type}
                                            onChange={e => updateStep(step.id, { step_type: e.target.value as WorkflowStepType })}
                                            className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                        >
                                            {STEP_TYPES.map(t => (
                                                <option key={t.value} value={t.value}>{t.label}</option>
                                            ))}
                                        </select>
                                        <input
                                            value={step.name}
                                            onChange={e => updateStep(step.id, { name: e.target.value })}
                                            placeholder="Step name"
                                            className="flex-1 bg-transparent border-none text-sm text-gray-200 placeholder-gray-500 focus:outline-none min-w-0"
                                        />
                                        <button
                                            onClick={() => removeStep(step.id)}
                                            className="text-gray-600 hover:text-red-400 p-1"
                                            title="Remove step"
                                        >
                                            <Trash2 className="w-3.5 h-3.5" />
                                        </button>
                                    </div>

                                    {/* Expanded details */}
                                    {isExpanded && (
                                        <div className="px-3 pb-3 pt-1 border-t border-gray-700 space-y-2">
                                            {/* Step ID */}
                                            <div>
                                                <label className="block text-xs text-gray-400 mb-1">Step ID</label>
                                                <input
                                                    value={step.id}
                                                    onChange={e => {
                                                        const oldId = step.id;
                                                        const newId = e.target.value.replace(/[^a-zA-Z0-9_]/g, '');
                                                        // Update the step ID and all references
                                                        setSteps(prev => prev.map(s => {
                                                            const updated = { ...s };
                                                            if (s.id === oldId) updated.id = newId;
                                                            updated.depends_on = s.depends_on.map(d => d === oldId ? newId : d);
                                                            if (s.on_approve === oldId) updated.on_approve = newId;
                                                            if (s.on_reject === oldId) updated.on_reject = newId;
                                                            if (s.loop_back_to === oldId) updated.loop_back_to = newId;
                                                            return updated;
                                                        }));
                                                        setExpandedStep(newId);
                                                    }}
                                                    className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 font-mono focus:outline-none focus:border-blue-500"
                                                />
                                            </div>

                                            {/* Depends On */}
                                            <div>
                                                <label className="block text-xs text-gray-400 mb-1">Depends On</label>
                                                <div className="flex flex-wrap gap-1">
                                                    {stepIds.filter(id => id !== step.id).map(id => (
                                                        <label key={id} className="flex items-center gap-1 text-xs text-gray-300">
                                                            <input
                                                                type="checkbox"
                                                                checked={step.depends_on.includes(id)}
                                                                onChange={e => {
                                                                    const deps = e.target.checked
                                                                        ? [...step.depends_on, id]
                                                                        : step.depends_on.filter(d => d !== id);
                                                                    updateStep(step.id, { depends_on: deps });
                                                                }}
                                                                className="rounded"
                                                            />
                                                            {id}
                                                        </label>
                                                    ))}
                                                </div>
                                                {stepIds.filter(id => id !== step.id).length === 0 && (
                                                    <p className="text-xs text-gray-500">No other steps to depend on</p>
                                                )}
                                            </div>

                                            {step.step_type === 'agentic' && (
                                                <>
                                                    <div>
                                                        <label className="block text-xs text-gray-400 mb-1">Session Mode</label>
                                                        <select
                                                            value={step.session_mode ?? 'resume'}
                                                            onChange={e => updateStep(step.id, { session_mode: e.target.value as SessionMode })}
                                                            className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                        >
                                                            {SESSION_MODES.map(m => (
                                                                <option key={m.value} value={m.value}>{m.label}</option>
                                                            ))}
                                                        </select>
                                                    </div>
                                                    <div>
                                                        <label className="block text-xs text-gray-400 mb-1">
                                                            Prompt Template <span className="text-red-400">*</span>
                                                        </label>
                                                        <textarea
                                                            value={step.prompt_template ?? ''}
                                                            onChange={e => updateStep(step.id, {
                                                                prompt_template: e.target.value || undefined,
                                                            })}
                                                            placeholder="Use {{task}} for task context, {{step:step_id}} for prior step output, {{rejection}} for rejection feedback."
                                                            rows={4}
                                                            className={`w-full bg-gray-700 border rounded-md px-3 py-2 text-xs text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 resize-none font-mono ${
                                                                !step.prompt_template?.trim() ? 'border-red-600' : 'border-gray-600'
                                                            }`}
                                                        />
                                                    </div>
                                                    <div className="grid grid-cols-2 gap-2">
                                                        <div>
                                                            <label className="block text-xs text-gray-400 mb-1">Loop Back To</label>
                                                            <select
                                                                value={step.loop_back_to ?? ''}
                                                                onChange={e => updateStep(step.id, {
                                                                    loop_back_to: e.target.value || undefined,
                                                                })}
                                                                className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                            >
                                                                <option value="">None</option>
                                                                {stepIds.filter(id => id !== step.id).map(id => (
                                                                    <option key={id} value={id}>{id}</option>
                                                                ))}
                                                            </select>
                                                        </div>
                                                        <div>
                                                            <label className="block text-xs text-gray-400 mb-1">Max Retries</label>
                                                            <input
                                                                type="number"
                                                                min="1"
                                                                value={step.max_retries ?? ''}
                                                                onChange={e => updateStep(step.id, {
                                                                    max_retries: e.target.value ? parseInt(e.target.value) : undefined,
                                                                })}
                                                                placeholder="No limit"
                                                                className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                            />
                                                        </div>
                                                    </div>
                                                </>
                                            )}

                                            {step.step_type === 'human_gate' && (
                                                <div className="grid grid-cols-2 gap-2">
                                                    <div>
                                                        <label className="block text-xs text-gray-400 mb-1">
                                                            On Approve → <span className="text-red-400">*</span>
                                                        </label>
                                                        <select
                                                            value={step.on_approve ?? ''}
                                                            onChange={e => updateStep(step.id, {
                                                                on_approve: e.target.value || undefined,
                                                            })}
                                                            className={`w-full bg-gray-700 border rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500 ${
                                                                !step.on_approve ? 'border-red-600' : 'border-gray-600'
                                                            }`}
                                                        >
                                                            <option value="">Select...</option>
                                                            {stepIds.filter(id => id !== step.id).map(id => (
                                                                <option key={id} value={id}>{id}</option>
                                                            ))}
                                                        </select>
                                                    </div>
                                                    <div>
                                                        <label className="block text-xs text-gray-400 mb-1">On Reject →</label>
                                                        <select
                                                            value={step.on_reject ?? ''}
                                                            onChange={e => updateStep(step.id, {
                                                                on_reject: e.target.value || undefined,
                                                            })}
                                                            className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-blue-500"
                                                        >
                                                            <option value="">None (no reject)</option>
                                                            {stepIds.filter(id => id !== step.id).map(id => (
                                                                <option key={id} value={id}>{id}</option>
                                                            ))}
                                                        </select>
                                                    </div>
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
                        disabled={!name.trim() || steps.length === 0 || hasErrors || updateWorkflow.isPending}
                        className="px-3 py-1.5 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {updateWorkflow.isPending ? 'Saving...' : 'Save'}
                    </button>
                </div>
            </div>
        </div>
    );
}
