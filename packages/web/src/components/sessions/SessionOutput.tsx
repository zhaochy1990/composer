import { useEffect, useRef, useMemo } from 'react';
import { useSessionOutputStore, type SessionLogEntry } from '@/stores/session-output-store';
import { useSessionLogs } from '@/hooks/use-sessions';
import { parseClaudeMessage } from '@/lib/parse-claude-message';
import { MessageEntry } from './MessageEntry';
import { cn } from '@/lib/utils';

const EMPTY_OUTPUT: SessionLogEntry[] = [];

interface SessionOutputProps {
    sessionId: string;
}

export function SessionOutput({ sessionId }: SessionOutputProps) {
    const output = useSessionOutputStore(
        (state) => state.outputs[sessionId] ?? EMPTY_OUTPUT,
    );
    const hydrate = useSessionOutputStore((state) => state.hydrate);
    const prepend = useSessionOutputStore((state) => state.prepend);
    const isHydrated = useSessionOutputStore((state) => state.isHydrated(sessionId));

    // Fetch paginated historical logs from DB
    const {
        data: logPages,
        hasNextPage,
        fetchNextPage,
        isFetchingNextPage,
    } = useSessionLogs(!isHydrated ? sessionId : undefined);

    // Hydrate store with first page of historical logs once fetched
    useEffect(() => {
        if (logPages && logPages.pages.length > 0 && !isHydrated) {
            const firstPage = logPages.pages[0];
            if (firstPage.logs.length > 0) {
                hydrate(
                    sessionId,
                    firstPage.logs.map((log) => ({
                        id: log.id,
                        session_id: log.session_id,
                        log_type: log.log_type,
                        content: log.content,
                        seq: log.id,
                    })),
                );
            }
        }
    }, [logPages, sessionId, hydrate, isHydrated]);

    // Auto-fetch all remaining pages after initial hydration
    const fetchingAllRef = useRef(false);
    useEffect(() => {
        if (!isHydrated || !hasNextPage || isFetchingNextPage || fetchingAllRef.current) return;
        fetchingAllRef.current = true;
        fetchNextPage();
    }, [isHydrated, hasNextPage, isFetchingNextPage, fetchNextPage]);

    // Reset the fetching flag when a fetch completes so the next page can be triggered
    useEffect(() => {
        if (!isFetchingNextPage) {
            fetchingAllRef.current = false;
        }
    }, [isFetchingNextPage]);

    // Prepend older pages into the store as they arrive
    const prependedPagesRef = useRef(1); // page 0 was hydrated
    const scrollRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!logPages || !isHydrated) return;
        const pages = logPages.pages;
        while (prependedPagesRef.current < pages.length) {
            const page = pages[prependedPagesRef.current];
            if (page.logs.length > 0) {
                // Preserve scroll position when prepending
                const el = scrollRef.current;
                const prevScrollHeight = el?.scrollHeight ?? 0;

                prepend(
                    sessionId,
                    page.logs.map((log) => ({
                        id: log.id,
                        session_id: log.session_id,
                        log_type: log.log_type,
                        content: log.content,
                        seq: log.id,
                    })),
                );

                // Restore scroll position after DOM update
                if (el) {
                    requestAnimationFrame(() => {
                        const newScrollHeight = el.scrollHeight;
                        el.scrollTop += newScrollHeight - prevScrollHeight;
                    });
                }
            }
            prependedPagesRef.current++;
        }
    }, [logPages, isHydrated, sessionId, prepend]);

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
            parsed: line.log_type === 'stdout' ? parseClaudeMessage(line.content) : null,
        }));
    }, [output]);

    return (
        <div
            ref={scrollRef}
            className="h-full overflow-y-auto bg-gray-950 rounded-lg p-4 font-mono text-sm border border-gray-800"
        >
            {hasNextPage && (
                <div className="text-center text-gray-500 text-xs py-2">
                    Loading older messages...
                </div>
            )}
            {parsedEntries.length === 0 && !hasNextPage && (
                <p className="text-gray-600 italic">
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
                            ) && 'text-gray-300',
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
