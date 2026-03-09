import { useState, useEffect, useMemo } from 'react';
import { FileText, X, ThumbsUp, ThumbsDown, MessageCircle } from 'lucide-react';
import type { WorkflowStepOutput, WorkflowStepDefinition } from '@/types/generated';
import { MarkdownContent } from '@/components/sessions/MarkdownContent';
import { useSubmitWorkflowDecision } from '@/hooks/use-workflows';
import { parsePrMeta, parseTestResults, buildReviewMarkdown } from '@/lib/parse-step-output';
import { UserQuestionPanel } from '@/components/sessions/UserQuestionPanel';
import type { PendingQuestion } from '@/stores/user-question-store';

export interface ReviewPanelData {
    content: string;
    humanGateSteps: WorkflowStepOutput[];
    steps: WorkflowStepDefinition[];
    workflowRunId: string;
    allStepOutputs: WorkflowStepOutput[];
}

interface WorkflowReviewSidePanelProps {
    data: ReviewPanelData;
    pendingQuestion?: PendingQuestion | null;
    onClose: () => void;
}

export function WorkflowReviewSidePanel({ data, pendingQuestion, onClose }: WorkflowReviewSidePanelProps) {
    const { content, humanGateSteps, steps, workflowRunId, allStepOutputs } = data;
    const submitDecision = useSubmitWorkflowDecision();
    const [comments, setComments] = useState<Record<string, string>>({});

    // Build structured review markdown from step outputs
    const reviewMarkdown = useMemo(() => {
        // Find latest completed auto_review output
        const autoReviewOutput = allStepOutputs
            .filter(o => o.step_id === 'auto_review' && o.status === 'completed')
            .sort((a, b) => a.attempt - b.attempt)
            .pop();

        // Find latest completed implement output
        const implementOutput = allStepOutputs
            .filter(o => o.step_id === 'implement' && o.status === 'completed')
            .sort((a, b) => a.attempt - b.attempt)
            .pop();

        const autoReviewText = autoReviewOutput?.output ?? '';
        const implementText = implementOutput?.output ?? '';

        const prMeta = parsePrMeta(autoReviewText);
        const testResults = parseTestResults(implementText);

        // Use auto_review output as review content if available, otherwise fall back to preceding step content
        const reviewContent = autoReviewText || content;

        const assembled = buildReviewMarkdown(prMeta, testResults, reviewContent);
        return assembled || content;
    }, [allStepOutputs, content]);

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

    const isPlanQuestion = !!pendingQuestion;
    const activeGateName = isPlanQuestion
        ? 'Plan'
        : humanGateSteps.length > 0
            ? getGateName(humanGateSteps[0].step_id)
            : 'Review';

    // Use plan_content from the question event when in plan question mode;
    // for human gates, use structured review markdown
    const displayContent = isPlanQuestion
        ? (pendingQuestion.planContent ?? content)
        : reviewMarkdown;

    return (
        <div className="w-[800px] shrink-0 h-full bg-bg-surface border-r border-border-primary flex flex-col overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-3 border-b border-border-primary shrink-0">
                <div className="flex items-center gap-2">
                    {isPlanQuestion
                        ? <MessageCircle className="w-4 h-4 text-purple-400" />
                        : <FileText className="w-4 h-4 text-blue-400" />
                    }
                    <h3 className="text-sm font-semibold text-text-primary">{activeGateName}</h3>
                </div>
                <button
                    type="button"
                    onClick={onClose}
                    aria-label="Close review panel"
                    className="text-text-muted hover:text-text-primary transition-colors p-1 rounded hover:bg-bg-elevated"
                >
                    <X className="w-4 h-4" />
                </button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto px-8 py-6 min-h-0">
                {displayContent ? (
                    <div className="max-w-[680px] mx-auto">
                        <div className="prose prose-invert max-w-none leading-relaxed">
                            <MarkdownContent content={displayContent} />
                        </div>
                    </div>
                ) : (
                    <p className="text-sm text-text-muted text-center py-8">No content to review</p>
                )}
            </div>

            {/* Question footer — shown during plan step when agent asks a question */}
            {isPlanQuestion && (
                <div className="shrink-0 border-t border-purple-800 px-6 py-4 bg-purple-900/10">
                    <UserQuestionPanel pendingQuestion={pendingQuestion} />
                </div>
            )}

            {/* Decision footer — one section per waiting gate */}
            {!isPlanQuestion && humanGateSteps.map(gateOutput => {
                const gateDef = getGateDef(gateOutput.step_id);
                const stepComments = comments[gateOutput.step_id] ?? '';

                return (
                    <div key={gateOutput.id} className="shrink-0 border-t border-border-primary px-6 py-4 space-y-3">
                        <textarea
                            value={stepComments}
                            onChange={(e) => setComments(prev => ({ ...prev, [gateOutput.step_id]: e.target.value }))}
                            placeholder="Optional comments or feedback..."
                            rows={3}
                            className="w-full bg-bg-elevated border border-border-secondary rounded-md px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-yellow-500 focus:ring-1 focus:ring-yellow-500 resize-none"
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
                                <span className="text-xs text-text-muted">Add comments to reject</span>
                            )}
                        </div>
                    </div>
                );
            })}
        </div>
    );
}
