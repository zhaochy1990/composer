import { useState } from 'react';
import { ChevronRight, ChevronDown, Terminal, FileText, Search, Globe } from 'lucide-react';
import type { ParsedMessage } from '@/lib/parse-claude-message';
import { MarkdownContent } from './MarkdownContent';

const toolIcons: Record<string, typeof Terminal> = {
    Bash: Terminal,
    Read: FileText,
    Write: FileText,
    Edit: FileText,
    MultiEdit: FileText,
    Grep: Search,
    Glob: Search,
    WebSearch: Globe,
    WebFetch: Globe,
};

interface MessageEntryProps {
    message: ParsedMessage;
}

export function MessageEntry({ message }: MessageEntryProps) {
    switch (message.kind) {
        case 'assistant_text':
            return (
                <div className="py-1 text-text-primary leading-relaxed">
                    <MarkdownContent content={message.text} />
                </div>
            );

        case 'thinking':
            return <ThinkingBlock text={message.text} />;

        case 'tool_use':
            return <ToolUseBlock tool={message.tool} summary={message.summary} input={message.input} />;

        case 'tool_result':
            return (
                <div className={`py-0.5 pl-4 border-l-2 text-xs whitespace-pre-wrap break-all ${
                    message.isError ? 'border-red-700 text-red-400' : 'border-border-primary text-text-muted'
                }`}>
                    {message.content.slice(0, 2000)}
                    {message.content.length > 2000 && '... (truncated)'}
                </div>
            );

        case 'user_text':
            return (
                <div className={`py-1 whitespace-pre-wrap break-words ${
                    message.isSubagent ? 'text-blue-400' : 'text-green-400'
                }`}>
                    <span className={`mr-1 ${message.isSubagent ? 'text-blue-600' : 'text-green-600'}`}>
                        {message.isSubagent ? '\u2192' : '>'}
                    </span>
                    {message.text}
                </div>
            );

        case 'system':
            return (
                <div className="py-0.5 text-blue-400/60 text-xs italic">
                    {message.text}
                </div>
            );

        case 'result':
            return (
                <div className={`py-1 text-xs font-medium ${
                    message.isError ? 'text-red-400' : 'text-yellow-400'
                }`}>
                    {message.isError ? 'Error: ' : 'Result: '}{message.summary.slice(0, 500)}
                </div>
            );

        case 'raw':
            return (
                <div className="py-0.5 text-text-muted whitespace-pre-wrap break-all text-xs">
                    {message.text}
                </div>
            );

        default:
            return null;
    }
}

function ThinkingBlock({ text }: { text: string }) {
    const [expanded, setExpanded] = useState(false);
    const preview = text.slice(0, 100) + (text.length > 100 ? '...' : '');

    return (
        <div className="py-0.5">
            <button
                type="button"
                onClick={() => setExpanded(!expanded)}
                className="flex items-center gap-1 text-xs text-purple-400/70 hover:text-purple-300 transition-colors"
            >
                {expanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
                <span className="italic">thinking{!expanded && `: ${preview}`}</span>
            </button>
            {expanded && (
                <div className="pl-4 pt-1 text-purple-300/50 text-xs whitespace-pre-wrap break-words">
                    {text}
                </div>
            )}
        </div>
    );
}

function ToolUseBlock({ tool, summary, input }: { tool: string; summary: string; input: Record<string, unknown> }) {
    const [expanded, setExpanded] = useState(false);
    const Icon = toolIcons[tool] ?? Terminal;

    return (
        <div className="py-1">
            <button
                type="button"
                onClick={() => setExpanded(!expanded)}
                className="flex items-center gap-1.5 text-xs text-cyan-400 hover:text-cyan-300 transition-colors group"
            >
                {expanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
                <Icon className="w-3.5 h-3.5" />
                <span className="font-medium">{tool}</span>
                {!expanded && (
                    <span className="text-cyan-400/60 font-mono truncate max-w-[400px]">{summary}</span>
                )}
            </button>
            {expanded && (
                <div className="pl-6 pt-1">
                    {tool === 'Bash' && typeof input.command === 'string' ? (
                        <pre className="text-xs text-text-secondary bg-bg-surface rounded px-2 py-1 overflow-x-auto">{input.command}</pre>
                    ) : tool === 'Edit' && typeof input.file_path === 'string' ? (
                        <div className="text-xs space-y-1">
                            <div className="text-text-muted">{input.file_path as string}</div>
                            {typeof input.old_string === 'string' && (
                                <pre className="text-red-400/70 bg-red-950/30 rounded px-2 py-1 overflow-x-auto">- {(input.old_string as string).slice(0, 500)}</pre>
                            )}
                            {typeof input.new_string === 'string' && (
                                <pre className="text-green-400/70 bg-green-950/30 rounded px-2 py-1 overflow-x-auto">+ {(input.new_string as string).slice(0, 500)}</pre>
                            )}
                        </div>
                    ) : (
                        <pre className="text-xs text-text-muted bg-bg-surface rounded px-2 py-1 overflow-x-auto max-h-40 overflow-y-auto">
                            {JSON.stringify(input, null, 2)}
                        </pre>
                    )}
                </div>
            )}
        </div>
    );
}
