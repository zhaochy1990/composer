import { useEffect } from 'react';
import { Check, Circle, Clock, AlertTriangle, X, Loader2, RotateCcw, SkipForward, Ban, MessageCircle, CheckCircle2 } from 'lucide-react';
import type { WorkflowRun, WorkflowStepType, WorkflowStepStatus, WorkflowStepOutput, Workflow, WorkflowStepDefinition } from '@/types/generated';
import { useWorkflowStepOutputs, useResumeWorkflowRun } from '@/hooks/use-workflows';
import { useCompleteSession } from '@/hooks/use-sessions';
import type { ReviewPanelData } from './WorkflowReviewSidePanel';

interface WorkflowProgressProps {
    workflowRun: WorkflowRun;
    workflow: Workflow;
    onReviewData?: (data: ReviewPanelData | null) => void;
    /** Called when an interactive step is running, with its session_id (or null when not). */
    onInteractiveSession?: (sessionId: string | null) => void;
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

export function WorkflowProgress({ workflowRun, workflow, onReviewData, onInteractiveSession }: WorkflowProgressProps) {
    const { data: stepOutputs } = useWorkflowStepOutputs(workflowRun.id, workflowRun.status);
    const resumeRun = useResumeWorkflowRun();
    const completeSession = useCompleteSession();

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

    // Expose review data to parent when at a human gate
    useEffect(() => {
        if (!onReviewData) return;
        if (humanGateSteps.length > 0 && stepOutputs) {
            // Find the agentic step immediately preceding the active human gate
            const activeGateId = humanGateSteps[0].step_id;
            const gateIndex = workflow.definition.steps.findIndex(s => s.id === activeGateId);
            let precedingStep: WorkflowStepDefinition | undefined;
            for (let i = gateIndex - 1; i >= 0; i--) {
                if (workflow.definition.steps[i].step_type === 'agentic') {
                    precedingStep = workflow.definition.steps[i];
                    break;
                }
            }
            let content = '';
            if (precedingStep) {
                const outputs = stepOutputs
                    .filter(o => o.step_id === precedingStep!.id && o.status === 'completed');
                const latestOutput = outputs.length > 0 ? outputs[outputs.length - 1] : null;
                content = latestOutput?.output ?? '';
            }
            onReviewData({
                content,
                humanGateSteps,
                steps: workflow.definition.steps,
                workflowRunId: workflowRun.id,
            });
        } else {
            onReviewData(null);
        }
        // No cleanup — avoids flicker from cleanup nulling then effect re-setting
    }, [humanGateSteps.length, stepOutputs, onReviewData, workflow, workflowRun.id]);

    // Detect interactive running step and expose its session_id to parent
    const interactiveRunningStep = steps.find(step => {
        if (!step.interactive) return false;
        const output = stepOutputs?.find(o => o.step_id === step.id && o.status === 'running');
        return !!output;
    });
    const interactiveSessionId = interactiveRunningStep && stepOutputs
        ? stepOutputs.find(o => o.step_id === interactiveRunningStep.id && o.status === 'running')?.session_id ?? null
        : null;

    useEffect(() => {
        onInteractiveSession?.(interactiveSessionId);
    }, [interactiveSessionId, onInteractiveSession]);

    function handleRetryResume(stepId: string, action: 'continue_loop' | 'skip_to_next') {
        resumeRun.mutate({
            runId: workflowRun.id,
            step_id: stepId,
            action,
        });
    }

    // Build a map of step_id -> latest output
    const latestOutputMap = new Map<string, WorkflowStepOutput>();
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
                                    {step.interactive && (
                                        <span className="text-xs text-purple-400 flex items-center gap-0.5" title="Interactive — you can send messages during this step">
                                            <MessageCircle className="w-3 h-3" />
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
                                {step.interactive && status === 'running' && latestOutput?.session_id && (
                                    <button
                                        type="button"
                                        onClick={() => completeSession.mutate(latestOutput.session_id!)}
                                        disabled={completeSession.isPending}
                                        className="mt-1.5 flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-purple-900/40 text-purple-300 border border-purple-700 hover:bg-purple-900/60 transition-colors disabled:opacity-50"
                                    >
                                        <CheckCircle2 className="w-3 h-3" />
                                        {completeSession.isPending ? 'Completing...' : 'Complete Step'}
                                    </button>
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

        </div>
    );
}
