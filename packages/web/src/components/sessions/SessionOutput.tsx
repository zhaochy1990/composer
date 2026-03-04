import { useEffect, useRef } from 'react';
import { useSessionOutputStore, type SessionLogEntry } from '@/stores/session-output-store';
import { cn } from '@/lib/utils';

const EMPTY_OUTPUT: SessionLogEntry[] = [];

interface SessionOutputProps {
    sessionId: string;
}

export function SessionOutput({ sessionId }: SessionOutputProps) {
    const output = useSessionOutputStore(
        (state) => state.outputs[sessionId] ?? EMPTY_OUTPUT,
    );
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

    return (
        <div
            ref={scrollRef}
            className="h-full overflow-y-auto bg-gray-950 rounded-lg p-4 font-mono text-sm border border-gray-800"
        >
            {output.length === 0 && (
                <p className="text-gray-600 italic">
                    Waiting for output...
                </p>
            )}
            {output.map((line) => (
                <div
                    key={line.seq}
                    className={cn(
                        'py-0.5 whitespace-pre-wrap break-all leading-relaxed',
                        line.log_type === 'stderr' && 'text-red-400',
                        line.log_type === 'control' && 'text-blue-400',
                        line.log_type === 'status' && 'text-yellow-400',
                        line.log_type === 'stdout' && 'text-gray-200',
                        line.log_type === 'user_input' && 'text-green-400',
                        !['stderr', 'control', 'status', 'stdout', 'user_input'].includes(
                            line.log_type,
                        ) && 'text-gray-300',
                    )}
                >
                    {line.log_type === 'user_input' && <span className="text-green-600 mr-1">&gt;</span>}
                    {line.content}
                </div>
            ))}
        </div>
    );
}
