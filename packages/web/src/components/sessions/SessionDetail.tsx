import { ArrowLeft, Square, Play, Clock, Terminal, Bot, FileText, FolderGit2 } from 'lucide-react';
import { useSession, useInterruptSession, useResumeSession } from '@/hooks/use-sessions';
import { SessionOutput } from './SessionOutput';
import { StatusBadge } from './StatusBadge';
import { shortId, formatDuration, formatTime } from '@/lib/utils';

interface SessionDetailProps {
    sessionId: string;
    onBack: () => void;
}

export function SessionDetail({ sessionId, onBack }: SessionDetailProps) {
    const { data: session, isLoading, error } = useSession(sessionId);
    const interruptMutation = useInterruptSession();
    const resumeMutation = useResumeSession();

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-full">
                <p className="text-text-muted">Loading session...</p>
            </div>
        );
    }

    if (error || !session) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-4">
                <p className="text-red-400">
                    Failed to load session{error ? `: ${error.message}` : ''}
                </p>
                <button
                    type="button"
                    onClick={onBack}
                    className="text-sm text-text-muted hover:text-text-primary transition-colors"
                >
                    Back to sessions
                </button>
            </div>
        );
    }

    const isRunning = session.status === 'running';
    const isPaused = session.status === 'paused';
    const duration = formatDuration(session.started_at, session.completed_at);

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="border-b border-border-primary px-6 py-4">
                <div className="flex items-center gap-3 mb-3">
                    <button
                        type="button"
                        onClick={onBack}
                        className="text-text-muted hover:text-text-primary transition-colors p-1 rounded hover:bg-bg-elevated"
                        title="Back to sessions"
                    >
                        <ArrowLeft className="w-4 h-4" />
                    </button>
                    <div className="flex items-center gap-2">
                        <Terminal className="w-4 h-4 text-text-muted" />
                        <span className="font-mono text-sm text-text-muted">
                            {shortId(session.id)}
                        </span>
                    </div>
                    <StatusBadge status={session.status} />
                </div>

                <div className="flex items-center justify-between">
                    {/* Metadata row */}
                    <div className="flex items-center gap-4 text-sm text-text-muted">
                        <span className="flex items-center gap-1.5">
                            <Bot className="w-3.5 h-3.5" />
                            {shortId(session.agent_id)}
                        </span>
                        <span className="flex items-center gap-1.5">
                            <Clock className="w-3.5 h-3.5" />
                            {duration}
                        </span>
                        <span className="text-text-muted">
                            Started {formatTime(session.started_at ?? session.created_at)}
                        </span>
                        {session.task_id && (
                            <span className="flex items-center gap-1.5">
                                <FileText className="w-3.5 h-3.5" />
                                Task {shortId(session.task_id)}
                            </span>
                        )}
                        {session.worktree_id && (
                            <span className="flex items-center gap-1.5">
                                <FolderGit2 className="w-3.5 h-3.5" />
                                {shortId(session.worktree_id)}
                            </span>
                        )}
                    </div>

                    {/* Action buttons */}
                    <div className="flex items-center gap-2">
                        {isRunning && (
                            <button
                                type="button"
                                onClick={() =>
                                    interruptMutation.mutate(session.id)
                                }
                                disabled={interruptMutation.isPending}
                                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60 transition-colors disabled:opacity-50"
                            >
                                <Square className="w-3.5 h-3.5" />
                                {interruptMutation.isPending
                                    ? 'Interrupting...'
                                    : 'Interrupt'}
                            </button>
                        )}
                        {isPaused && (
                            <button
                                type="button"
                                onClick={() =>
                                    resumeMutation.mutate({ id: session.id })
                                }
                                disabled={resumeMutation.isPending}
                                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium bg-green-900/40 text-green-300 border border-green-700 hover:bg-green-900/60 transition-colors disabled:opacity-50"
                            >
                                <Play className="w-3.5 h-3.5" />
                                {resumeMutation.isPending
                                    ? 'Resuming...'
                                    : 'Resume'}
                            </button>
                        )}
                    </div>
                </div>
            </div>

            {/* Prompt section */}
            {session.prompt && (
                <div className="px-6 py-3 border-b border-border-primary">
                    <p className="text-xs font-semibold text-text-muted uppercase mb-1">
                        Prompt
                    </p>
                    <p className="text-sm text-text-secondary whitespace-pre-wrap">
                        {session.prompt}
                    </p>
                </div>
            )}

            {/* Result summary */}
            {session.result_summary && (
                <div className="px-6 py-3 border-b border-border-primary">
                    <p className="text-xs font-semibold text-text-muted uppercase mb-1">
                        Result
                    </p>
                    <p className="text-sm text-text-secondary whitespace-pre-wrap">
                        {session.result_summary}
                    </p>
                </div>
            )}

            {/* Live output */}
            <div className="flex-1 overflow-hidden px-6 py-4">
                <p className="text-xs font-semibold text-text-muted uppercase mb-2">
                    Output
                </p>
                <SessionOutput sessionId={session.id} claudeSessionId={session.resume_session_id} />
            </div>
        </div>
    );
}
