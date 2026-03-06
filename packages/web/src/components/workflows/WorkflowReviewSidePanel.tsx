import { useState, useEffect } from 'react';
import { FileText, X, ThumbsUp, ThumbsDown } from 'lucide-react';
import type { WorkflowStepOutput, WorkflowStepDefinition } from '@/types/generated';
import { MarkdownContent } from '@/components/sessions/MarkdownContent';
import { useSubmitWorkflowDecision } from '@/hooks/use-workflows';

export interface ReviewPanelData {
    content: string;
    humanGateSteps: WorkflowStepOutput[];
    steps: WorkflowStepDefinition[];
    workflowRunId: string;
}

interface WorkflowReviewSidePanelProps {
    data: ReviewPanelData;
    onClose: () => void;
}

export function WorkflowReviewSidePanel({ data, onClose }: WorkflowReviewSidePanelProps) {
    const { content, humanGateSteps, steps, workflowRunId } = data;
    const submitDecision = useSubmitWorkflowDecision();
    const [comments, setComments] = useState<Record<string, string>>({});

    // Reset comments when the workflow run changes
    useEffect(() => {
        setComments({});
    }, [workflowRunId]);

    function handleDecision(stepId: string, approved: boolean) {
        submitDecision.mutate({
            runId: workflowRunId,
            stepId,
            approved,
            comments: comments[stepId]?.trim() || undefined,
        }, {
            onSuccess: () => setComments(prev => ({ ...prev, [stepId]: '' })),
        });
    }

    // Find gate definition for display names
    function getGateName(stepId: string): string {
        const step = steps.find(s => s.id === stepId);
        return step?.name || 'Review';
    }

    function getGateDef(stepId: string): WorkflowStepDefinition | undefined {
        return steps.find(s => s.id === stepId);
    }

    const activeGateName = humanGateSteps.length > 0
        ? getGateName(humanGateSteps[0].step_id)
        : 'Review';

    return (
        <div className="w-[500px] shrink-0 h-full bg-gray-900 border-r border-gray-700 flex flex-col overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-3 border-b border-gray-800 shrink-0">
                <div className="flex items-center gap-2">
                    <FileText className="w-4 h-4 text-blue-400" />
                    <h3 className="text-sm font-semibold text-gray-200">{activeGateName}</h3>
                </div>
                <button
                    type="button"
                    onClick={onClose}
                    aria-label="Close review panel"
                    className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                >
                    <X className="w-4 h-4" />
                </button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto px-6 py-4 min-h-0">
                {content ? (
                    <div className="prose prose-invert prose-sm max-w-none leading-relaxed">
                        <MarkdownContent content={content} />
                    </div>
                ) : (
                    <p className="text-sm text-gray-500 text-center py-8">No content to review</p>
                )}
            </div>

            {/* Decision footer — one section per waiting gate */}
            {humanGateSteps.map(gateOutput => {
                const gateDef = getGateDef(gateOutput.step_id);
                const stepComments = comments[gateOutput.step_id] ?? '';

                return (
                    <div key={gateOutput.id} className="shrink-0 border-t border-gray-800 px-6 py-4 space-y-3">
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
