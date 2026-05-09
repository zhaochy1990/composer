import type { AgentType } from '@/types/generated';

export function getAgentCapabilities(agentType: AgentType) {
    return {
        supportsResume: agentType === 'claude_code',
        supportsInteractiveInput: agentType === 'claude_code',
        supportsPlanDetection: agentType === 'claude_code',
        supportsControlProtocol: agentType === 'claude_code',
    };
}

export const AGENT_TYPE_LABELS: Record<AgentType, string> = {
    claude_code: 'Claude Code',
    codex: 'Codex',
    gemini_cli: 'Gemini CLI',
    copilot_cli: 'Copilot CLI',
};
