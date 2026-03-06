import { Workflow as WorkflowIcon } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition } from '@/types/generated';

const STEP_TYPE_LABELS: Record<string, string> = {
    agentic: 'Agent',
    human_gate: 'Review',
};

const STEP_TYPE_COLORS: Record<string, string> = {
    agentic: 'bg-blue-900/40 text-blue-300 border-blue-800',
    human_gate: 'bg-yellow-900/40 text-yellow-300 border-yellow-800',
};

// Differentiate agentic steps by session_mode for better visual variety
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
}

export function WorkflowCard({ workflow, onClick }: WorkflowCardProps) {
    return (
        <button
            type="button"
            onClick={onClick}
            className="w-full text-left p-4 bg-gray-900 border border-gray-800 rounded-lg hover:border-gray-700 transition-colors"
        >
            <div className="flex items-center gap-2 mb-3">
                <WorkflowIcon className="w-4 h-4 text-purple-400" />
                <h3 className="text-sm font-semibold text-gray-100">{workflow.name}</h3>
                <span className="text-xs text-gray-500 ml-auto">
                    {workflow.definition.steps.length} steps
                </span>
            </div>
            <div className="flex flex-wrap gap-1.5">
                {workflow.definition.steps.map((step, i) => (
                    <span
                        key={i}
                        className={`text-xs px-1.5 py-0.5 rounded border ${getStepColor(step)}`}
                    >
                        {step.name || STEP_TYPE_LABELS[step.step_type] || step.step_type}
                    </span>
                ))}
            </div>
        </button>
    );
}
