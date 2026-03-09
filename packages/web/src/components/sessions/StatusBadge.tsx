import type { SessionStatus } from '@/types/generated';
import { cn } from '@/lib/utils';

interface StatusBadgeProps {
    status: SessionStatus;
    className?: string;
}

const statusConfig: Record<
    SessionStatus,
    { label: string; baseClass: string; pulse?: boolean }
> = {
    created: {
        label: 'Created',
        baseClass: 'bg-bg-interactive text-text-secondary border-border-secondary',
    },
    running: {
        label: 'Running',
        baseClass: 'bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/70 dark:text-blue-300 dark:border-blue-600',
        pulse: true,
    },
    paused: {
        label: 'Paused',
        baseClass: 'bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/70 dark:text-yellow-300 dark:border-yellow-600',
    },
    completed: {
        label: 'Completed',
        baseClass: 'bg-green-100 text-green-800 border-green-300 dark:bg-green-900/70 dark:text-green-300 dark:border-green-600',
    },
    failed: {
        label: 'Failed',
        baseClass: 'bg-red-100 text-red-800 border-red-300 dark:bg-red-900/70 dark:text-red-300 dark:border-red-600',
    },
};

export function StatusBadge({ status, className }: StatusBadgeProps) {
    const config = statusConfig[status] ?? statusConfig.created;

    return (
        <span
            className={cn(
                'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium border',
                config.baseClass,
                className,
            )}
        >
            {config.pulse && (
                <span className="relative flex h-2 w-2">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-400" />
                </span>
            )}
            {config.label}
        </span>
    );
}
