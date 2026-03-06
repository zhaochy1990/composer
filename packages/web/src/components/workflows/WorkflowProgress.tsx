import { useState, useEffect } from 'react';
import { Check, Circle, Clock, AlertTriangle, X, ThumbsUp, ThumbsDown, Loader2, RotateCcw, SkipForward, Ban } from 'lucide-react';
import type { WorkflowRun, WorkflowStepType, WorkflowStepStatus, Workflow, WorkflowStepDefinition } from '@/types/generated';
import { useWorkflowStepOutputs, useSubmitWorkflowDecision, useResumeWorkflowRun } from '@/hooks/use-workflows';

interface WorkflowProgressProps {
    workflowRun: WorkflowRun;
    workflow: Workflow;
    onPlanContent?: (content: string | null) => void;
}

const STEP_TYPE_LABELS: Record<WorkflowStepType, string> = {
    agentic: 'Agent',
    human_gate: 'Review',
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
        case 'skipped':
            return <Ban className="w-4 h-4 text-gray-500" />;
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
        skipped: 'bg-gray-800 text-gray-500 border-gray-700',
    };

    return (
        <span className={`text-xs px-1.5 py-0.5 rounded border ${colors[status]}`}>
            {status.replace(/_/g, ' ')}
        </span>
    );
}

function getStepName(step: WorkflowStepDefinition): string {
    return step.name || STEP_TYPE_LABELS[step.step_type] || step.id;
}

export function WorkflowProgress({ workflowRun, workflow, onPlanContent }: WorkflowProgressProps) {
    const { data: stepOutputs } = useWorkflowStepOutputs(workflowRun.id, workflowRun.status);
    const submitDecision = useSubmitWorkflowDecision();
    const resumeRun = useResumeWorkflowRun();
    const [comments, setComments] = useState<Record<string, string>>({});

    const steps = workflow.definition.steps;
    const isWaitingForHuman = workflowRun.status === 'paused';
    const isFailed = workflowRun.status === 'failed';

    // Find human gate steps waiting for decisions
    const humanGateSteps = isWaitingForHuman && stepOutputs
        ? stepOutputs.filter(s => s.status === 'waiting_for_human')
        : [];

    // Find steps that failed due to retry exhaustion
    const retryExhaustedSteps = isWaitingForHuman && stepOutputs
        ? stepOutputs.filter(s => s.status === 'failed' && s.output === 'Max retries exceeded')
        : [];

    // Check if paused due to server crash (paused but no human gate and no retry exhaustion)
    const isPausedForRecovery = isWaitingForHuman && humanGateSteps.length === 0 && retryExhaustedSteps.length === 0;
    const canResume = isPausedForRecovery || isFailed;

    // Expose plan content to parent when at a human gate
    useEffect(() => {
        if (!onPlanContent) return;
        if (humanGateSteps.length > 0 && stepOutputs) {
            const planStep = workflow.definition.steps.find(s => s.step_type === 'agentic');
            if (planStep) {
                const planOutputs = stepOutputs
                    .filter(o => o.step_id === planStep.id && o.status === 'completed');
                const planOutput = planOutputs.length > 0 ? planOutputs[planOutputs.length - 1] : null;
                onPlanContent(planOutput?.output ?? null);
            } else {
                onPlanContent(null);
            }
        } else {
            onPlanContent(null);
        }
        return () => onPlanContent(null);
    }, [humanGateSteps.length, stepOutputs, onPlanContent]);

    function handleDecision(stepId: string, approved: boolean) {
        submitDecision.mutate({
            runId: workflowRun.id,
            stepId,
            approved,
            comments: comments[stepId]?.trim() || undefined,
        }, {
            onSuccess: () => setComments(prev => ({ ...prev, [stepId]: '' })),
        });
    }

    function handleRetryResume(stepId: string, action: 'continue_loop' | 'skip_to_next') {
        resumeRun.mutate({
            runId: workflowRun.id,
            step_id: stepId,
            action,
        });
    }

    // Build a map of step_id -> latest output
    const latestOutputMap = new Map<string, (typeof stepOutputs extends (infer T)[] | undefined ? T : never)>();
    if (stepOutputs) {
        for (const output of stepOutputs) {
            const existing = latestOutputMap.get(output.step_id);
            if (!existing || output.attempt > existing.attempt) {
                latestOutputMap.set(output.step_id, output);
            }
        }
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
                {steps.map((step) => {
                    const latestOutput = latestOutputMap.get(step.id);
                    const status: WorkflowStepStatus = latestOutput?.status ?? 'pending';
                    const isSkipped = status === 'skipped';

                    return (
                        <div
                            key={step.id}
                            className={`flex items-start gap-3 px-3 py-2 rounded-md ${
                                status === 'running' || status === 'waiting_for_human' ? 'bg-gray-800' : ''
                            } ${isSkipped ? 'opacity-50' : ''}`}
                        >
                            <div className="mt-0.5">
                                <StepStatusIcon status={status} />
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                    <span className={`text-sm font-medium ${
                                        isSkipped ? 'text-gray-500 line-through' :
                                        status === 'running' || status === 'waiting_for_human' ? 'text-gray-100' : 'text-gray-400'
                                    }`}>
                                        {getStepName(step)}
                                    </span>
                                    <span className="text-xs text-gray-600">
                                        {STEP_TYPE_LABELS[step.step_type]}
                                    </span>
                                    {latestOutput && latestOutput.attempt > 1 && (
                                        <span className="text-xs text-gray-500">
                                            (attempt {latestOutput.attempt})
                                        </span>
                                    )}
                                    {step.loop_back_to != null && (
                                        <span className="text-xs text-gray-500 flex items-center gap-0.5" title={`Loops back to ${step.loop_back_to}`}>
                                            <RotateCcw className="w-3 h-3" />
                                        </span>
                                    )}
                                </div>
                                {step.step_type === 'human_gate' && step.on_approve && (
                                    <p className="text-xs text-gray-600 mt-0.5">
                                        approve → {step.on_approve}
                                        {step.on_reject && ` | reject → ${step.on_reject}`}
                                    </p>
                                )}
                                {latestOutput?.output && status !== 'skipped' && (
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
                        onClick={() => resumeRun.mutate({ runId: workflowRun.id })}
                        disabled={resumeRun.isPending}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-orange-600 text-white hover:bg-orange-500 transition-colors disabled:opacity-50"
                    >
                        <RotateCcw className="w-3.5 h-3.5" />
                        {resumeRun.isPending ? 'Resuming...' : 'Resume Workflow'}
                    </button>
                </div>
            )}

            {/* Retry exhaustion decision UI */}
            {retryExhaustedSteps.map(step => (
                <div key={step.id} className="border border-orange-700 rounded-lg p-4 bg-orange-900/20 space-y-3">
                    <div className="flex items-center gap-2">
                        <AlertTriangle className="w-4 h-4 text-orange-400" />
                        <span className="text-sm font-medium text-orange-300">
                            Retries exhausted: {steps.find(s => s.id === step.step_id)?.name ?? step.step_id}
                        </span>
                    </div>
                    <p className="text-xs text-gray-400">
                        Automated review loop exceeded max retries. What would you like to do?
                    </p>
                    <div className="flex items-center gap-3">
                        <button
                            type="button"
                            onClick={() => handleRetryResume(step.step_id, 'continue_loop')}
                            disabled={resumeRun.isPending}
                            className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-blue-600 text-white hover:bg-blue-500 transition-colors disabled:opacity-50"
                        >
                            <RotateCcw className="w-3.5 h-3.5" />
                            Continue iterating
                        </button>
                        <button
                            type="button"
                            onClick={() => handleRetryResume(step.step_id, 'skip_to_next')}
                            disabled={resumeRun.isPending}
                            className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-gray-700 text-gray-200 border border-gray-600 hover:bg-gray-600 transition-colors disabled:opacity-50"
                        >
                            <SkipForward className="w-3.5 h-3.5" />
                            Stop loop & advance
                        </button>
                    </div>
                </div>
            ))}

            {/* Human gate decision UI — one card per waiting gate */}
            {humanGateSteps.map(gateOutput => {
                const gateDef = steps.find(s => s.id === gateOutput.step_id);
                const stepComments = comments[gateOutput.step_id] ?? '';

                return (
                    <div key={gateOutput.id} className="border border-yellow-700 rounded-lg p-4 bg-yellow-900/20 space-y-3">
                        <div className="flex items-center gap-2">
                            <Clock className="w-4 h-4 text-yellow-400" />
                            <span className="text-sm font-medium text-yellow-300">
                                Your review is needed: {gateDef?.name ?? gateOutput.step_id}
                            </span>
                        </div>

                        <textarea
                            value={stepComments}
                            onChange={(e) => setComments(prev => ({ ...prev, [gateOutput.step_id]: e.target.value }))}
                            placeholder="Optional comments or feedback..."
                            rows={3}
                            className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-yellow-500 focus:ring-1 focus:ring-yellow-500 resize-none"
                        />

                        <div className="flex items-center gap-3">
                            <button
                                type="button"
                                onClick={() => handleDecision(gateOutput.step_id, true)}
                                disabled={submitDecision.isPending}
                                className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-green-600 text-white hover:bg-green-500 transition-colors disabled:opacity-50"
                            >
                                <ThumbsUp className="w-3.5 h-3.5" />
                                {submitDecision.isPending ? 'Submitting...' : `Approve${gateDef?.on_approve ? ` → ${gateDef.on_approve}` : ''}`}
                            </button>
                            {gateDef?.on_reject && (
                                <button
                                    type="button"
                                    onClick={() => handleDecision(gateOutput.step_id, false)}
                                    disabled={submitDecision.isPending || !stepComments.trim()}
                                    className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    <ThumbsDown className="w-3.5 h-3.5" />
                                    Reject → {gateDef.on_reject}
                                </button>
                            )}
                            {gateDef?.on_reject && !stepComments.trim() && (
                                <span className="text-xs text-gray-500">Add comments to reject</span>
                            )}
                        </div>
                    </div>
                );
            })}
        </div>
    );
}
