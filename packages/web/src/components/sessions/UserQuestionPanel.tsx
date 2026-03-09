import { useState } from 'react';
import { Send } from 'lucide-react';
import { useAnswerQuestion } from '@/hooks/use-sessions';
import { useUserQuestionStore, type PendingQuestion } from '@/stores/user-question-store';

interface QuestionOption {
    label: string;
    description?: string;
}

interface QuestionItem {
    question: string;
    header?: string;
    options: QuestionOption[];
    multiSelect?: boolean;
}

interface UserQuestionPanelProps {
    pendingQuestion: PendingQuestion;
}

export function UserQuestionPanel({ pendingQuestion }: UserQuestionPanelProps) {
    const answerMutation = useAnswerQuestion();
    const clearQuestion = useUserQuestionStore((s) => s.clear);

    // Parse questions from the AskUserQuestion payload
    const questionItems: QuestionItem[] = (() => {
        const q = pendingQuestion.questions as { questions?: QuestionItem[] };
        return q?.questions ?? [];
    })();

    // Track selected answers per question
    const [answers, setAnswers] = useState<Record<string, string>>({});
    const [otherTexts, setOtherTexts] = useState<Record<string, string>>({});

    function handleSelect(question: string, value: string) {
        setAnswers((prev) => ({ ...prev, [question]: value }));
    }

    function handleSubmit() {
        // Build final answers, substituting "Other" with freeform text
        const finalAnswers: Record<string, string> = {};
        for (const item of questionItems) {
            const selected = answers[item.question];
            if (selected === '__other__') {
                finalAnswers[item.question] = otherTexts[item.question] ?? '';
            } else if (selected) {
                finalAnswers[item.question] = selected;
            }
        }

        answerMutation.mutate(
            {
                id: pendingQuestion.sessionId,
                requestId: pendingQuestion.requestId,
                answers: finalAnswers,
            },
            {
                onSuccess: () => {
                    clearQuestion(pendingQuestion.sessionId);
                },
            },
        );
    }

    const allAnswered = questionItems.every((item) => {
        const selected = answers[item.question];
        if (!selected) return false;
        if (selected === '__other__' && !otherTexts[item.question]?.trim()) return false;
        return true;
    });

    return (
        <div className="space-y-4">
            {questionItems.map((item) => (
                <div key={item.question} className="space-y-2">
                    <div className="flex items-center gap-2">
                        {item.header && (
                            <span className="px-2 py-0.5 rounded text-xs font-medium bg-purple-900/50 text-purple-300 border border-purple-700">
                                {item.header}
                            </span>
                        )}
                        <span className="text-sm font-medium text-text-primary">{item.question}</span>
                    </div>
                    <div className="space-y-1.5 pl-1">
                        {item.options.map((opt) => (
                            <label
                                key={opt.label}
                                className={`flex items-start gap-2.5 px-3 py-2 rounded-md cursor-pointer border transition-colors ${
                                    answers[item.question] === opt.label
                                        ? 'border-purple-600 bg-purple-900/30'
                                        : 'border-border-primary hover:border-border-secondary bg-bg-elevated/50'
                                }`}
                            >
                                <input
                                    type="radio"
                                    name={item.question}
                                    value={opt.label}
                                    checked={answers[item.question] === opt.label}
                                    onChange={() => handleSelect(item.question, opt.label)}
                                    className="mt-0.5 accent-purple-500"
                                />
                                <div>
                                    <span className="text-sm text-text-primary">{opt.label}</span>
                                    {opt.description && (
                                        <p className="text-xs text-text-muted mt-0.5">{opt.description}</p>
                                    )}
                                </div>
                            </label>
                        ))}
                        {/* Other option */}
                        <label
                            className={`flex items-start gap-2.5 px-3 py-2 rounded-md cursor-pointer border transition-colors ${
                                answers[item.question] === '__other__'
                                    ? 'border-purple-600 bg-purple-900/30'
                                    : 'border-border-primary hover:border-border-secondary bg-bg-elevated/50'
                            }`}
                        >
                            <input
                                type="radio"
                                name={item.question}
                                value="__other__"
                                checked={answers[item.question] === '__other__'}
                                onChange={() => handleSelect(item.question, '__other__')}
                                className="mt-0.5 accent-purple-500"
                            />
                            <div className="flex-1">
                                <span className="text-sm text-text-primary">Other</span>
                                {answers[item.question] === '__other__' && (
                                    <input
                                        type="text"
                                        value={otherTexts[item.question] ?? ''}
                                        onChange={(e) =>
                                            setOtherTexts((prev) => ({
                                                ...prev,
                                                [item.question]: e.target.value,
                                            }))
                                        }
                                        placeholder="Type your answer..."
                                        className="mt-1.5 w-full bg-bg-elevated border border-border-secondary rounded-md px-2 py-1 text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-purple-500 focus:ring-1 focus:ring-purple-500"
                                        autoFocus
                                    />
                                )}
                            </div>
                        </label>
                    </div>
                </div>
            ))}
            <button
                type="button"
                onClick={handleSubmit}
                disabled={!allAnswered || answerMutation.isPending}
                className="flex items-center gap-1.5 px-4 py-2 rounded-md text-sm font-medium bg-purple-700 text-white hover:bg-purple-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
                <Send className="w-3.5 h-3.5" />
                {answerMutation.isPending ? 'Submitting...' : 'Submit Answer'}
            </button>
        </div>
    );
}
