import { useState, useEffect } from 'react';
import { Check, Circle, Clock, AlertTriangle, X, ThumbsUp, ThumbsDown, Loader2, RotateCcw } from 'lucide-react';
import type { WorkflowRun, WorkflowStepType, WorkflowStepStatus, Workflow } from '@/types/generated';
import { useWorkflowStepOutputs, useSubmitWorkflowDecision, useResumeWorkflowRun } from '@/hooks/use-workflows';

interface WorkflowProgressProps {
    workflowRun: WorkflowRun;
    workflow: Workflow;
    onPlanContent?: (content: string | null) => void;
}

const STEP_TYPE_LABELS: Record<WorkflowStepType, string> = {
    plan: 'Plan',
    human_gate: 'Review',
    implement: 'Implement',
    pr_review: 'PR Review',
    human_review: 'Human Review',
};

function StepStatusIcon({ status }: { status: WorkflowStepStatus }) {
    switch (status) {
        case 'completed':
            return <Check className="w-4 h-4 text-green-400" />;
        case 'running':
            return <Loader2 className="w-4 h-4 text-blue-400 animate-spin" />;
        case 'waiting_for_human':
            return <Clock className="w-4 h-4 text-yellow-400" />;
        case 'rejected':
            return <X className="w-4 h-4 text-red-400" />;
        case 'failed':
            return <AlertTriangle className="w-4 h-4 text-red-400" />;
        default:
            return <Circle className="w-4 h-4 text-gray-600" />;
    }
}

function StepStatusBadge({ status }: { status: WorkflowStepStatus }) {
    const colors: Record<WorkflowStepStatus, string> = {
        pending: 'bg-gray-700 text-gray-400',
        running: 'bg-blue-900/40 text-blue-300 border-blue-700',
        waiting_for_human: 'bg-yellow-900/40 text-yellow-300 border-yellow-700',
        completed: 'bg-green-900/40 text-green-300 border-green-700',
        rejected: 'bg-red-900/40 text-red-300 border-red-700',
        failed: 'bg-red-900/40 text-red-300 border-red-700',
    };

    return (
        <span className={`text-xs px-1.5 py-0.5 rounded border ${colors[status]}`}>
            {status.replace('_', ' ')}
        </span>
    );
}

export function WorkflowProgress({ workflowRun, workflow, onPlanContent }: WorkflowProgressProps) {
    const { data: stepOutputs } = useWorkflowStepOutputs(workflowRun.id);
    const submitDecision = useSubmitWorkflowDecision();
    const resumeRun = useResumeWorkflowRun();
    const [comments, setComments] = useState('');

    const steps = workflow.definition.steps;
    const isWaitingForHuman = workflowRun.status === 'paused';
    const isFailed = workflowRun.status === 'failed';

    // Find the current human gate step that needs a decision
    const currentHumanStep = isWaitingForHuman && stepOutputs
        ? stepOutputs.find(s => s.status === 'waiting_for_human')
        : null;

    // Check if paused due to server crash (paused but no human gate waiting)
    const isPausedForRecovery = isWaitingForHuman && !currentHumanStep;
    const canResume = isPausedForRecovery || isFailed;

    // Expose plan content to parent when at a human gate
    useEffect(() => {
        if (!onPlanContent) return;
        if (currentHumanStep && stepOutputs) {
            const planOutputs = stepOutputs
                .filter(o => o.step_type === 'plan' && o.status === 'completed');
            const planOutput = planOutputs.length > 0 ? planOutputs[planOutputs.length - 1] : null;
            onPlanContent(planOutput?.output ?? null);
        } else {
            onPlanContent(null);
        }
        return () => onPlanContent(null);
    }, [currentHumanStep, stepOutputs, onPlanContent]);

    function handleDecision(approved: boolean) {
        submitDecision.mutate({
            runId: workflowRun.id,
            approved,
            comments: comments.trim() || undefined,
        }, {
            onSuccess: () => setComments(''),
        });
    }

    return (
        <div className="space-y-3">
            {/* Run status header */}
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                    <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">
                        Workflow: {workflow.name}
                    </span>
                    <StepStatusBadge status={
                        workflowRun.status === 'running' ? 'running' :
                        workflowRun.status === 'paused' ? 'waiting_for_human' :
                        workflowRun.status === 'completed' ? 'completed' : 'failed'
                    } />
                </div>
                {workflowRun.iteration_count > 0 && (
                    <span className="text-xs text-gray-500">
                        Iteration {workflowRun.iteration_count + 1}
                    </span>
                )}
            </div>

            {/* Step timeline */}
            <div className="space-y-1">
                {steps.map((step, index) => {
                    // Find the latest output for this step
                    const outputs = (stepOutputs ?? []).filter(o => o.step_index === index);
                    const latestOutput = outputs.length > 0 ? outputs[outputs.length - 1] : null;
                    const status: WorkflowStepStatus = latestOutput?.status ?? (index < workflowRun.current_step_index ? 'completed' : 'pending');
                    const isCurrent = index === workflowRun.current_step_index;

                    return (
                        <div
                            key={index}
                            className={`flex items-start gap-3 px-3 py-2 rounded-md ${
                                isCurrent ? 'bg-gray-800' : ''
                            }`}
                        >
                            <div className="mt-0.5">
                                <StepStatusIcon status={status} />
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                    <span className={`text-sm font-medium ${isCurrent ? 'text-gray-100' : 'text-gray-400'}`}>
                                        {step.name || STEP_TYPE_LABELS[step.step_type]}
                                    </span>
                                    <span className="text-xs text-gray-600">
                                        {STEP_TYPE_LABELS[step.step_type]}
                                    </span>
                                    {latestOutput && latestOutput.attempt > 1 && (
                                        <span className="text-xs text-gray-500">
                                            (attempt {latestOutput.attempt})
                                        </span>
                                    )}
                                </div>
                                {latestOutput?.output && (
                                    <p className="text-xs text-gray-500 mt-1 truncate max-w-lg" title={latestOutput.output}>
                                        {latestOutput.output.slice(0, 200)}
                                    </p>
                                )}
                            </div>
                        </div>
                    );
                })}
            </div>

            {/* Resume after server crash or failure */}
            {canResume && (
                <div className="border border-orange-700 rounded-lg p-4 bg-orange-900/20 space-y-3">
                    <div className="flex items-center gap-2">
                        <AlertTriangle className="w-4 h-4 text-orange-400" />
                        <span className="text-sm font-medium text-orange-300">
                            {isFailed ? 'Workflow step failed' : 'Workflow interrupted (server restart)'}
                        </span>
                    </div>
                    <p className="text-xs text-gray-400">
                        The workflow can be resumed from the current step. The agent session will be resumed with its full context.
                    </p>
                    <button
                        type="button"
                        onClick={() => resumeRun.mutate(workflowRun.id)}
                        disabled={resumeRun.isPending}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-orange-600 text-white hover:bg-orange-500 transition-colors disabled:opacity-50"
                    >
                        <RotateCcw className="w-3.5 h-3.5" />
                        {resumeRun.isPending ? 'Resuming...' : 'Resume Workflow'}
                    </button>
                </div>
            )}

            {/* Human gate decision UI */}
            {currentHumanStep && (
                <div className="border border-yellow-700 rounded-lg p-4 bg-yellow-900/20 space-y-3">
                    <div className="flex items-center gap-2">
                        <Clock className="w-4 h-4 text-yellow-400" />
                        <span className="text-sm font-medium text-yellow-300">
                            Your review is needed
                        </span>
                    </div>

                    <textarea
                        value={comments}
                        onChange={(e) => setComments(e.target.value)}
                        placeholder="Optional comments or feedback..."
                        rows={3}
                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-yellow-500 focus:ring-1 focus:ring-yellow-500 resize-none"
                    />

                    <div className="flex items-center gap-3">
                        <button
                            type="button"
                            onClick={() => handleDecision(true)}
                            disabled={submitDecision.isPending}
                            className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-green-600 text-white hover:bg-green-500 transition-colors disabled:opacity-50"
                        >
                            <ThumbsUp className="w-3.5 h-3.5" />
                            {submitDecision.isPending ? 'Submitting...' : 'Approve'}
                        </button>
                        <button
                            type="button"
                            onClick={() => handleDecision(false)}
                            disabled={submitDecision.isPending || !comments.trim()}
                            className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ThumbsDown className="w-3.5 h-3.5" />
                            Reject
                        </button>
                        {!comments.trim() && (
                            <span className="text-xs text-gray-500">Add comments to reject</span>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}
