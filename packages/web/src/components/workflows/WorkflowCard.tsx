import { Workflow as WorkflowIcon, Copy, Lock } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition } from '@/types/generated';
import { useCloneWorkflow } from '@/hooks/use-workflows';

const STEP_TYPE_LABELS: Record<string, string> = {
    agentic: 'Agent',
    human_gate: 'Review',
};

const STEP_TYPE_COLORS: Record<string, string> = {
    agentic: 'bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/40 dark:text-blue-300 dark:border-blue-800',
    human_gate: 'bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/40 dark:text-yellow-300 dark:border-yellow-800',
};

const SESSION_MODE_COLORS: Record<string, string> = {
    new: 'bg-purple-100 text-purple-800 border-purple-300 dark:bg-purple-900/40 dark:text-purple-300 dark:border-purple-800',
    resume: 'bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/40 dark:text-blue-300 dark:border-blue-800',
    separate: 'bg-cyan-100 text-cyan-800 border-cyan-300 dark:bg-cyan-900/40 dark:text-cyan-300 dark:border-cyan-800',
};

function getStepColor(step: WorkflowStepDefinition): string {
    if (step.step_type === 'agentic' && step.session_mode) {
        return SESSION_MODE_COLORS[step.session_mode] ?? STEP_TYPE_COLORS.agentic;
    }
    return STEP_TYPE_COLORS[step.step_type] ?? 'bg-bg-interactive text-text-muted';
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
                className={`w-full text-left px-4 py-2.5 border-b border-border-primary hover:bg-bg-elevated transition-colors ${
                    isSelected ? 'bg-bg-elevated border-l-2 border-l-blue-500' : ''
                }`}
            >
                <div className="flex items-center gap-2">
                    <WorkflowIcon className="w-3.5 h-3.5 text-purple-400 shrink-0" />
                    <span className="text-sm font-medium text-text-primary truncate">{workflow.name}</span>
                    {workflow.is_template && (
                        <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-purple-900/30 text-purple-400 border border-purple-800/50 shrink-0">
                            <Lock className="w-2.5 h-2.5" />
                            Built-in
                        </span>
                    )}
                    <span className="text-xs text-text-muted ml-auto shrink-0">
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
            className="w-full text-left p-4 bg-bg-surface border border-border-primary rounded-lg hover:border-border-primary transition-colors"
        >
            <div className="flex items-center gap-2 mb-3">
                <WorkflowIcon className="w-4 h-4 text-purple-400" />
                <h3 className="text-sm font-semibold text-text-primary">{workflow.name}</h3>
                {workflow.is_template && (
                    <span className="flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-purple-900/40 text-purple-300 border border-purple-800">
                        <Lock className="w-3 h-3" />
                        Template
                    </span>
                )}
                <span className="text-xs text-text-muted ml-auto">
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
                        className="flex items-center gap-1 px-3 py-1.5 text-xs bg-bg-elevated text-text-secondary rounded hover:bg-bg-interactive border border-border-primary transition-colors disabled:opacity-50"
                    >
                        <Copy className="w-3 h-3" />
                        {cloneWorkflow.isPending ? 'Cloning...' : 'Clone to edit'}
                    </button>
                </div>
            )}
        </button>
    );
}
