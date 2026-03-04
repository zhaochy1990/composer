import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { formatDuration, formatTime, shortId, cn } from '../utils';

describe('formatDuration', () => {
    it('returns -- when no start', () => {
        expect(formatDuration()).toBe('--');
        expect(formatDuration(undefined)).toBe('--');
    });

    it('formats minutes and seconds', () => {
        const start = new Date('2024-01-01T00:00:00Z').toISOString();
        const end = new Date('2024-01-01T00:02:34Z').toISOString();
        expect(formatDuration(start, end)).toBe('2m 34s');
    });

    it('formats hours and minutes', () => {
        const start = new Date('2024-01-01T00:00:00Z').toISOString();
        const end = new Date('2024-01-01T01:12:00Z').toISOString();
        expect(formatDuration(start, end)).toBe('1h 12m');
    });

    it('formats zero seconds', () => {
        const start = new Date('2024-01-01T00:00:00Z').toISOString();
        const end = new Date('2024-01-01T00:00:00Z').toISOString();
        expect(formatDuration(start, end)).toBe('0s');
    });

    it('uses now when no end time (running)', () => {
        const fiveSecondsAgo = new Date(Date.now() - 5000).toISOString();
        const result = formatDuration(fiveSecondsAgo);
        // Should be approximately 5s
        expect(result).toMatch(/^\d+s$/);
    });
});

describe('formatTime', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2024-06-15T12:00:00Z'));
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('returns -- for undefined', () => {
        expect(formatTime()).toBe('--');
        expect(formatTime(undefined)).toBe('--');
    });

    it('returns just now for < 1 min', () => {
        const now = new Date('2024-06-15T12:00:00Z').toISOString();
        expect(formatTime(now)).toBe('just now');
    });

    it('returns minutes ago', () => {
        const fiveMinAgo = new Date('2024-06-15T11:55:00Z').toISOString();
        expect(formatTime(fiveMinAgo)).toBe('5m ago');
    });

    it('returns hours ago', () => {
        const threeHoursAgo = new Date('2024-06-15T09:00:00Z').toISOString();
        expect(formatTime(threeHoursAgo)).toBe('3h ago');
    });

    it('returns formatted date for > 24h', () => {
        const twoDaysAgo = new Date('2024-06-13T12:00:00Z').toISOString();
        const result = formatTime(twoDaysAgo);
        // Should contain date info, not relative time
        expect(result).not.toContain('ago');
    });
});

describe('shortId', () => {
    it('returns first 8 chars', () => {
        expect(shortId('abcdefgh-1234-5678-9abc-def012345678')).toBe('abcdefgh');
    });
});

describe('cn', () => {
    it('merges class names', () => {
        const result = cn('px-2', 'py-1');
        expect(result).toContain('px-2');
        expect(result).toContain('py-1');
    });

    it('handles conditional classes', () => {
        const result = cn('base', false && 'hidden', 'extra');
        expect(result).toContain('base');
        expect(result).toContain('extra');
        expect(result).not.toContain('hidden');
    });
});
