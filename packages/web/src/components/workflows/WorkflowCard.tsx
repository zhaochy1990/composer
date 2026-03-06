import { Workflow as WorkflowIcon, Copy, Lock } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition } from '@/types/generated';
import { useCloneWorkflow } from '@/hooks/use-workflows';

const STEP_TYPE_LABELS: Record<string, string> = {
    agentic: 'Agent',
    human_gate: 'Review',
};

const STEP_TYPE_COLORS: Record<string, string> = {
    agentic: 'bg-blue-900/40 text-blue-300 border-blue-800',
    human_gate: 'bg-yellow-900/40 text-yellow-300 border-yellow-800',
};

const SESSION_MODE_COLORS: Record<string, string> = {
    new: 'bg-purple-900/40 text-purple-300 border-purple-800',
    resume: 'bg-blue-900/40 text-blue-300 border-blue-800',
    separate: 'bg-cyan-900/40 text-cyan-300 border-cyan-800',
};

function getStepColor(step: WorkflowStepDefinition): string {
    if (step.step_type === 'agentic' && step.session_mode) {
        return SESSION_MODE_COLORS[step.session_mode] ?? STEP_TYPE_COLORS.agentic;
    }
    return STEP_TYPE_COLORS[step.step_type] ?? 'bg-gray-700 text-gray-400';
}

interface WorkflowCardProps {
    workflow: Workflow;
    onClick: () => void;
    compact?: boolean;
    isSelected?: boolean;
}

export function WorkflowCard({ workflow, onClick, compact, isSelected }: WorkflowCardProps) {
    const cloneWorkflow = useCloneWorkflow();

    function handleClone(e: React.MouseEvent) {
        e.stopPropagation();
        cloneWorkflow.mutate(workflow.id);
    }

    if (compact) {
        return (
            <button
                type="button"
                onClick={onClick}
                className={`w-full text-left px-4 py-2.5 border-b border-gray-800 hover:bg-gray-800 transition-colors ${
                    isSelected ? 'bg-gray-800 border-l-2 border-l-blue-500' : ''
                }`}
            >
                <div className="flex items-center gap-2">
                    <WorkflowIcon className="w-3.5 h-3.5 text-purple-400 shrink-0" />
                    <span className="text-sm font-medium text-gray-200 truncate">{workflow.name}</span>
                    {workflow.is_template && (
                        <Lock className="w-3 h-3 text-purple-400 shrink-0" />
                    )}
                    <span className="text-xs text-gray-500 ml-auto shrink-0">
                        {workflow.definition.steps.length}
                    </span>
                </div>
            </button>
        );
    }

    return (
        <button
            type="button"
            onClick={onClick}
            className="w-full text-left p-4 bg-gray-900 border border-gray-800 rounded-lg hover:border-gray-700 transition-colors"
        >
            <div className="flex items-center gap-2 mb-3">
                <WorkflowIcon className="w-4 h-4 text-purple-400" />
                <h3 className="text-sm font-semibold text-gray-100">{workflow.name}</h3>
                {workflow.is_template && (
                    <span className="flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-purple-900/40 text-purple-300 border border-purple-800">
                        <Lock className="w-3 h-3" />
                        Template
                    </span>
                )}
                <span className="text-xs text-gray-500 ml-auto">
                    {workflow.definition.steps.length} steps
                </span>
            </div>
            <div className="flex flex-wrap gap-1.5">
                {workflow.definition.steps.map((step) => (
                    <span
                        key={step.id}
                        className={`text-xs px-1.5 py-0.5 rounded border ${getStepColor(step)}`}
                    >
                        {step.name || STEP_TYPE_LABELS[step.step_type] || step.id}
                    </span>
                ))}
            </div>
            {workflow.is_template && (
                <div className="mt-3 flex">
                    <button
                        type="button"
                        onClick={handleClone}
                        disabled={cloneWorkflow.isPending}
                        className="flex items-center gap-1 px-3 py-1.5 text-xs bg-gray-800 text-gray-300 rounded hover:bg-gray-700 border border-gray-700 transition-colors disabled:opacity-50"
                    >
                        <Copy className="w-3 h-3" />
                        {cloneWorkflow.isPending ? 'Cloning...' : 'Clone to edit'}
                    </button>
                </div>
            )}
        </button>
    );
}
