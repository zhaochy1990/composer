import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge } from '../StatusBadge';
import type { SessionStatus } from '@/types/generated';

describe('StatusBadge', () => {
    const statuses: Array<{ status: SessionStatus; label: string }> = [
        { status: 'created', label: 'Created' },
        { status: 'running', label: 'Running' },
        { status: 'paused', label: 'Paused' },
        { status: 'completed', label: 'Completed' },
        { status: 'failed', label: 'Failed' },
    ];

    statuses.forEach(({ status, label }) => {
        it(`renders ${label} for status ${status}`, () => {
            render(<StatusBadge status={status} />);
            expect(screen.getByText(label)).toBeInTheDocument();
        });
    });

    it('shows pulse animation for running status', () => {
        const { container } = render(<StatusBadge status="running" />);
        const pulseElement = container.querySelector('.animate-ping');
        expect(pulseElement).toBeInTheDocument();
    });

    it('does not show pulse for non-running status', () => {
        const { container } = render(<StatusBadge status="completed" />);
        const pulseElement = container.querySelector('.animate-ping');
        expect(pulseElement).not.toBeInTheDocument();
    });
});
