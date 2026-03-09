import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}

/**
 * Format a duration between two ISO date strings (or from start to now).
 * Returns a human-readable string like "2m 34s" or "1h 12m".
 */
export function formatDuration(startedAt?: string, completedAt?: string): string {
    if (!startedAt) return '--';
    const start = new Date(startedAt).getTime();
    const end = completedAt ? new Date(completedAt).getTime() : Date.now();
    const diffMs = Math.max(0, end - start);
    const totalSeconds = Math.floor(diffMs / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${seconds}s`;
    return `${seconds}s`;
}

/**
 * Format an ISO date string to a short relative or absolute time.
 */
export function formatTime(isoString?: string): string {
    if (!isoString) return '--';
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    if (diffMs < 0) return 'just now';
    const diffMinutes = Math.floor(diffMs / 60000);

    if (diffMinutes < 1) return 'just now';
    if (diffMinutes < 60) return `${diffMinutes}m ago`;
    const diffHours = Math.floor(diffMinutes / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/**
 * Shorten a UUID to its first 8 characters for display.
 */
export function shortId(id: string): string {
    return id.slice(0, 8);
}

/**
 * Extract a PR number from a URL (GitHub, Azure DevOps, GitLab).
 * Returns e.g. "#13" or falls back to "PR" if no number found.
 */
export function extractPrId(url: string): string {
    const match = url.match(/\/(?:pull|pullrequest|merge_requests)\/(\d+)/);
    return match ? `#${match[1]}` : 'PR';
}
