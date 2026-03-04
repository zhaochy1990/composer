import { describe, it, expect, vi } from 'vitest';
import { screen } from '@testing-library/react';
import { renderWithProviders } from '@/test/test-utils';
import { AgentCard } from '../AgentCard';
import type { Agent } from '@/types/generated';

// Mock the hooks that AgentCard uses
vi.mock('@/hooks/use-agents', () => ({
    useDeleteAgent: () => ({ mutate: vi.fn(), isPending: false }),
    useAgentHealth: () => ({ data: null }),
}));

function makeAgent(overrides: Partial<Agent> = {}): Agent {
    return {
        id: 'agent-aaaa-bbbb-cccc-dddddddddddd',
        name: 'Test Agent',
        agent_type: 'claude_code',
        executable_path: null,
        status: 'idle',
        auth_status: 'unknown',
        last_heartbeat: null,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        ...overrides,
    };
}

describe('AgentCard', () => {
    it('renders agent name', () => {
        renderWithProviders(<AgentCard agent={makeAgent()} />);
        expect(screen.getByText('Test Agent')).toBeInTheDocument();
    });

    it('renders agent type badge', () => {
        renderWithProviders(<AgentCard agent={makeAgent()} />);
        expect(screen.getByText('claude_code')).toBeInTheDocument();
    });

    it('shows auth status badge', () => {
        renderWithProviders(<AgentCard agent={makeAgent({ auth_status: 'authenticated' })} />);
        expect(screen.getByText('Authenticated')).toBeInTheDocument();
    });

    it('shows status label', () => {
        renderWithProviders(<AgentCard agent={makeAgent({ status: 'busy' })} />);
        expect(screen.getByText('Busy')).toBeInTheDocument();
    });

    it('shows executable path when present', () => {
        renderWithProviders(
            <AgentCard agent={makeAgent({ executable_path: '/usr/bin/claude' })} />,
        );
        expect(screen.getByText('/usr/bin/claude')).toBeInTheDocument();
    });
});
