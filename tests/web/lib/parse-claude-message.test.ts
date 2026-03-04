import { describe, it, expect } from 'vitest';
import { parseClaudeMessage } from '@/lib/parse-claude-message';

describe('parseClaudeMessage', () => {
    it('parses assistant text message', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: {
                role: 'assistant',
                content: [{ type: 'text', text: 'Hello world' }],
            },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{ kind: 'assistant_text', text: 'Hello world' }]);
    });

    it('parses assistant message with multiple text blocks', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: {
                content: [
                    { type: 'text', text: 'First part' },
                    { type: 'text', text: 'Second part' },
                ],
            },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toHaveLength(2);
        expect(result[0]).toEqual({ kind: 'assistant_text', text: 'First part' });
        expect(result[1]).toEqual({ kind: 'assistant_text', text: 'Second part' });
    });

    it('parses assistant message with tool_use block', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: {
                content: [
                    { type: 'tool_use', id: 'toolu_1', name: 'Bash', input: { command: 'ls -la' } },
                ],
            },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toHaveLength(1);
        expect(result[0]).toMatchObject({
            kind: 'tool_use',
            tool: 'Bash',
            summary: 'ls -la',
        });
    });

    it('parses mixed text + tool_use in one assistant message', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: {
                content: [
                    { type: 'text', text: 'Let me check the files' },
                    { type: 'tool_use', name: 'Bash', input: { command: 'ls' } },
                ],
            },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toHaveLength(2);
        expect(result[0].kind).toBe('assistant_text');
        expect(result[1].kind).toBe('tool_use');
    });

    it('parses thinking blocks', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: {
                content: [
                    { type: 'thinking', thinking: 'Let me analyze this...' },
                    { type: 'text', text: 'Here is my answer' },
                ],
            },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toHaveLength(2);
        expect(result[0]).toEqual({ kind: 'thinking', text: 'Let me analyze this...' });
        expect(result[1]).toEqual({ kind: 'assistant_text', text: 'Here is my answer' });
    });

    it('parses user message with string content', () => {
        const raw = JSON.stringify({
            type: 'user',
            message: { role: 'user', content: 'Fix the bug' },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{ kind: 'user_text', text: 'Fix the bug' }]);
    });

    it('parses user message with array content', () => {
        const raw = JSON.stringify({
            type: 'user',
            message: { content: [{ type: 'text', text: 'Hello' }] },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{ kind: 'user_text', text: 'Hello' }]);
    });

    it('parses result message (success)', () => {
        const raw = JSON.stringify({
            type: 'result',
            result: 'Task completed successfully',
            is_error: false,
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{
            kind: 'result',
            summary: 'Task completed successfully',
            isError: false,
        }]);
    });

    it('parses result message (error)', () => {
        const raw = JSON.stringify({
            type: 'result',
            result: 'Something went wrong',
            is_error: true,
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{
            kind: 'result',
            summary: 'Something went wrong',
            isError: true,
        }]);
    });

    it('parses system init message', () => {
        const raw = JSON.stringify({
            type: 'system',
            subtype: 'init',
            model: 'claude-sonnet-4-20250514',
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{
            kind: 'system',
            text: 'Session started (model: claude-sonnet-4-20250514)',
        }]);
    });

    it('parses standalone tool_use event', () => {
        const raw = JSON.stringify({
            type: 'tool_use',
            tool_name: 'Read',
            tool_data: { file_path: '/src/main.rs' },
        });
        const result = parseClaudeMessage(raw);
        expect(result).toMatchObject([{
            kind: 'tool_use',
            tool: 'Read',
            summary: '/src/main.rs',
        }]);
    });

    it('parses tool_result', () => {
        const raw = JSON.stringify({
            type: 'tool_result',
            result: 'file contents here',
            is_error: false,
        });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{
            kind: 'tool_result',
            content: 'file contents here',
            isError: false,
        }]);
    });

    it('returns raw for non-JSON input', () => {
        const result = parseClaudeMessage('not json at all');
        expect(result).toEqual([{ kind: 'raw', text: 'not json at all' }]);
    });

    it('returns raw for JSON without type field', () => {
        const result = parseClaudeMessage('{"data": 42}');
        expect(result).toEqual([{ kind: 'raw', text: '{"data": 42}' }]);
    });

    it('suppresses control_request messages', () => {
        const raw = JSON.stringify({ type: 'control_request', request_id: 'r1', request: {} });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([]);
    });

    it('suppresses control_response messages', () => {
        const raw = JSON.stringify({ type: 'control_response', response: {} });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([]);
    });

    it('suppresses stream_event messages', () => {
        const raw = JSON.stringify({ type: 'stream_event', event: {} });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([]);
    });

    it('returns raw for truly unknown types', () => {
        const raw = JSON.stringify({ type: 'some_future_type', data: 42 });
        const result = parseClaudeMessage(raw);
        expect(result).toEqual([{ kind: 'raw', text: raw }]);
    });

    // --- Tool summary formatting ---

    it('formats Bash tool summary as command', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: { content: [{ type: 'tool_use', name: 'Bash', input: { command: 'npm test' } }] },
        });
        const result = parseClaudeMessage(raw);
        expect(result[0]).toMatchObject({ summary: 'npm test' });
    });

    it('formats Edit tool summary as file path', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: { content: [{ type: 'tool_use', name: 'Edit', input: { file_path: '/src/app.ts', old_string: 'foo', new_string: 'bar' } }] },
        });
        const result = parseClaudeMessage(raw);
        expect(result[0]).toMatchObject({ summary: '/src/app.ts' });
    });

    it('formats Grep tool summary as pattern', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: { content: [{ type: 'tool_use', name: 'Grep', input: { pattern: 'TODO' } }] },
        });
        const result = parseClaudeMessage(raw);
        expect(result[0]).toMatchObject({ summary: 'TODO' });
    });

    it('formats WebSearch tool summary as query', () => {
        const raw = JSON.stringify({
            type: 'assistant',
            message: { content: [{ type: 'tool_use', name: 'WebSearch', input: { query: 'rust tokio tutorial' } }] },
        });
        const result = parseClaudeMessage(raw);
        expect(result[0]).toMatchObject({ summary: 'rust tokio tutorial' });
    });
});
