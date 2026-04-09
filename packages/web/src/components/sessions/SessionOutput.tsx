import { useEffect, useRef, useMemo } from 'react';
import { useSessionOutputStore, type SessionLogEntry } from '@/stores/session-output-store';
import { useSessionLogs } from '@/hooks/use-sessions';
import { parseClaudeMessage } from '@/lib/parse-claude-message';
import { MessageEntry } from './MessageEntry';
import { cn } from '@/lib/utils';

const EMPTY_OUTPUT: SessionLogEntry[] = [];

interface SessionOutputProps {
    sessionId: string;
    claudeSessionId?: string;
}

export function SessionOutput({ sessionId, claudeSessionId }: SessionOutputProps) {
    const output = useSessionOutputStore(
        (state) => state.outputs[sessionId] ?? EMPTY_OUTPUT,
    );
    const hydrate = useSessionOutputStore((state) => state.hydrate);
    const isHydrated = useSessionOutputStore((state) => state.isHydrated(sessionId));

    // Fetch ALL historical logs from DB in a single request
    const { data: logData } = useSessionLogs(sessionId);

    // Hydrate store with all historical logs once fetched
    useEffect(() => {
        if (logData && logData.logs.length > 0 && !isHydrated) {
            hydrate(
                sessionId,
                logData.logs.map((log) => ({
                    id: log.id,
                    session_id: log.session_id,
                    log_type: log.log_type,
                    content: log.content,
                    seq: log.id,
                })),
            );
        }
    }, [logData, sessionId, hydrate, isHydrated]);

    const scrollRef = useRef<HTMLDivElement>(null);
    const shouldAutoScroll = useRef(true);

    // Track whether user has scrolled up (disable auto-scroll if so)
    useEffect(() => {
        const el = scrollRef.current;
        if (!el) return;

        const handleScroll = () => {
            const { scrollTop, scrollHeight, clientHeight } = el;
            // Auto-scroll if within 60px of bottom
            shouldAutoScroll.current = scrollHeight - scrollTop - clientHeight < 60;
        };

        el.addEventListener('scroll', handleScroll);
        return () => el.removeEventListener('scroll', handleScroll);
    }, []);

    // Auto-scroll on new output
    useEffect(() => {
        if (shouldAutoScroll.current && scrollRef.current) {
            scrollRef.current.scrollTo({
                top: scrollRef.current.scrollHeight,
                behavior: 'smooth',
            });
        }
    }, [output.length]);

    // Parse stdout lines into structured messages, memoized to avoid re-parsing
    const parsedEntries = useMemo(() => {
        return output.map((line) => ({
            ...line,
            parsed: line.log_type === 'stdout' ? parseClaudeMessage(line.content, claudeSessionId) : null,
        }));
    }, [output, claudeSessionId]);

    return (
        <div
            ref={scrollRef}
            className="h-full overflow-y-auto bg-bg-app rounded-lg p-4 font-mono text-sm border border-border-primary"
        >
            {parsedEntries.length === 0 && (
                <p className="text-text-muted italic">
                    Waiting for output...
                </p>
            )}
            {parsedEntries.map((line) => {
                // For stdout lines, render parsed messages (skip if empty — suppressed types)
                if (line.parsed) {
                    if (line.parsed.length === 0) return null;
                    return (
                        <div key={line.seq}>
                            {line.parsed.map((msg, i) => (
                                <MessageEntry key={`${line.seq}-${i}`} message={msg} />
                            ))}
                        </div>
                    );
                }

                // For stderr, control, user_input — keep original rendering
                return (
                    <div
                        key={line.seq}
                        className={cn(
                            'py-0.5 whitespace-pre-wrap break-all leading-relaxed',
                            line.log_type === 'stderr' && 'text-red-400',
                            line.log_type === 'control' && 'text-blue-400',
                            line.log_type === 'status' && 'text-yellow-400',
                            line.log_type === 'user_input' && 'text-green-400',
                            !['stderr', 'control', 'status', 'user_input'].includes(
                                line.log_type,
                            ) && 'text-text-secondary',
                        )}
                    >
                        {line.log_type === 'user_input' && <span className="text-green-600 mr-1">&gt;</span>}
                        {line.content}
                    </div>
                );
            })}
        </div>
    );
}
