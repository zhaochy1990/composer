/**
 * Parses raw Claude Code stream-json lines into human-readable message entries.
 */

export type ParsedMessage =
    | { kind: 'assistant_text'; text: string }
    | { kind: 'thinking'; text: string }
    | { kind: 'tool_use'; tool: string; summary: string; input: Record<string, unknown> }
    | { kind: 'tool_result'; content: string; isError: boolean }
    | { kind: 'user_text'; text: string; isSubagent: boolean }
    | { kind: 'system'; text: string }
    | { kind: 'result'; summary: string; isError: boolean }
    | { kind: 'raw'; text: string };

/**
 * Format tool input into a one-line summary for display.
 */
function formatToolSummary(tool: string, input: Record<string, unknown>): string {
    switch (tool) {
        case 'Bash':
            return typeof input.command === 'string' ? input.command : JSON.stringify(input);
        case 'Read':
            return typeof input.file_path === 'string' ? input.file_path : JSON.stringify(input);
        case 'Write':
            return typeof input.file_path === 'string' ? input.file_path : JSON.stringify(input);
        case 'Edit':
        case 'MultiEdit':
            return typeof input.file_path === 'string' ? input.file_path : JSON.stringify(input);
        case 'Grep':
            return typeof input.pattern === 'string' ? input.pattern : JSON.stringify(input);
        case 'Glob':
            return typeof input.pattern === 'string' ? input.pattern : JSON.stringify(input);
        case 'WebSearch':
            return typeof input.query === 'string' ? input.query : JSON.stringify(input);
        case 'WebFetch':
            if (typeof input.url === 'string') return input.url;
            if (Array.isArray(input.urls) && input.urls.length > 0) return String(input.urls[0]);
            return JSON.stringify(input);
        case 'TodoWrite':
        case 'TodoRead':
            return 'todo list';
        case 'Agent':
            return typeof input.prompt === 'string'
                ? input.prompt.slice(0, 80) + (input.prompt.length > 80 ? '...' : '')
                : JSON.stringify(input);
        default:
            return Object.keys(input).join(', ') || tool;
    }
}

/**
 * Extract text from a content item (handles both string and object forms).
 */
function extractTextFromContent(content: unknown): string | null {
    if (typeof content === 'string') return content;
    if (Array.isArray(content)) {
        const texts = content
            .filter((item): item is { type: string; text: string } =>
                item && typeof item === 'object' && item.type === 'text' && typeof item.text === 'string',
            )
            .map((item) => item.text);
        return texts.length > 0 ? texts.join('\n') : null;
    }
    return null;
}

/**
 * Parse a raw JSON line from Claude Code's stream-json stdout into structured messages.
 * A single line can produce multiple entries (e.g., assistant text + tool_use blocks).
 */
export function parseClaudeMessage(raw: string, claudeSessionId?: string): ParsedMessage[] {
    let json: Record<string, unknown>;
    try {
        json = JSON.parse(raw);
    } catch {
        return [{ kind: 'raw', text: raw }];
    }

    if (!json || typeof json !== 'object' || !('type' in json)) {
        return [{ kind: 'raw', text: raw }];
    }

    const type = json.type as string;
    const results: ParsedMessage[] = [];

    switch (type) {
        case 'assistant': {
            const message = json.message as { content?: unknown } | undefined;
            const content = message?.content;
            if (Array.isArray(content)) {
                for (const block of content) {
                    if (!block || typeof block !== 'object') continue;
                    const blockType = (block as Record<string, unknown>).type as string;
                    if (blockType === 'text' && typeof (block as Record<string, unknown>).text === 'string') {
                        results.push({ kind: 'assistant_text', text: (block as { text: string }).text });
                    } else if (blockType === 'thinking' && typeof (block as Record<string, unknown>).thinking === 'string') {
                        results.push({ kind: 'thinking', text: (block as { thinking: string }).thinking });
                    } else if (blockType === 'tool_use') {
                        const b = block as { name?: string; input?: Record<string, unknown> };
                        const tool = b.name ?? 'unknown';
                        const input = b.input ?? {};
                        results.push({
                            kind: 'tool_use',
                            tool,
                            summary: formatToolSummary(tool, input),
                            input,
                        });
                    }
                }
            } else {
                const text = extractTextFromContent(content);
                if (text) results.push({ kind: 'assistant_text', text });
            }
            break;
        }

        case 'user': {
            const message = json.message as { content?: unknown } | undefined;
            const text = extractTextFromContent(message?.content);
            if (text) {
                const msgSessionId = json.session_id as string | undefined;
                const isSubagent = !!(
                    claudeSessionId &&
                    msgSessionId &&
                    msgSessionId !== claudeSessionId
                );
                results.push({ kind: 'user_text', text, isSubagent });
            }
            break;
        }

        case 'result': {
            const result = json.result as string | undefined;
            const isError = (json.is_error as boolean) ?? false;
            results.push({
                kind: 'result',
                summary: typeof result === 'string' ? result : (isError ? 'Error' : 'Completed'),
                isError,
            });
            break;
        }

        case 'system': {
            const subtype = json.subtype as string | undefined;
            const model = json.model as string | undefined;
            if (subtype === 'init' && model) {
                results.push({ kind: 'system', text: `Session started (model: ${model})` });
            } else if (subtype) {
                results.push({ kind: 'system', text: `System: ${subtype}` });
            }
            // Skip other system messages (noisy)
            break;
        }

        case 'tool_use': {
            const toolName = json.tool_name as string ?? 'unknown';
            const toolData = json.tool_data as Record<string, unknown> ?? {};
            results.push({
                kind: 'tool_use',
                tool: toolName,
                summary: formatToolSummary(toolName, toolData),
                input: toolData,
            });
            break;
        }

        case 'tool_result': {
            const resultVal = json.result;
            const content = typeof resultVal === 'string'
                ? resultVal
                : resultVal != null ? JSON.stringify(resultVal) : '';
            const isError = (json.is_error as boolean) ?? false;
            results.push({ kind: 'tool_result', content, isError });
            break;
        }

        // Known protocol/internal types — suppress from UI
        case 'control_request':
        case 'control_response':
        case 'control_cancel_request':
        case 'stream_event':
            return [];

        default:
            // Truly unknown type — show as raw so we don't silently lose data
            return [{ kind: 'raw', text: raw }];
    }

    return results;
}
