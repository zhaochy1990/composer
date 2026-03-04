import { useEffect, useRef, useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useSessionOutputStore } from '@/stores/session-output-store';
import type { WsEvent } from '@/types/generated';

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected';

const RECONNECT_DELAY_MS = 3000;
const MAX_RECONNECT_DELAY_MS = 30000;

export function useWebSocket() {
    const queryClient = useQueryClient();
    const append = useSessionOutputStore((state) => state.append);
    const [status, setStatus] = useState<ConnectionStatus>('disconnected');
    const wsRef = useRef<WebSocket | null>(null);
    const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const reconnectDelayRef = useRef(RECONNECT_DELAY_MS);
    const mountedRef = useRef(true);

    const handleMessage = useCallback(
        (event: MessageEvent) => {
            let parsed: WsEvent;
            try {
                parsed = JSON.parse(event.data);
            } catch {
                return;
            }

            switch (parsed.type) {
                // Session output streaming
                case 'SessionOutput': {
                    const { session_id, log_type, content } = parsed.payload;
                    append(session_id, {
                        session_id,
                        log_type,
                        content,
                    });
                    break;
                }

                // Session lifecycle events
                case 'SessionStarted': {
                    queryClient.invalidateQueries({ queryKey: ['sessions'] });
                    queryClient.invalidateQueries({
                        queryKey: ['sessions', parsed.payload.session_id],
                    });
                    if (parsed.payload.task_id) {
                        queryClient.invalidateQueries({ queryKey: ['tasks', parsed.payload.task_id, 'sessions'] });
                    }
                    queryClient.invalidateQueries({ queryKey: ['tasks'] });
                    break;
                }
                case 'SessionCompleted':
                case 'SessionFailed':
                case 'SessionPaused': {
                    queryClient.invalidateQueries({ queryKey: ['sessions'] });
                    queryClient.invalidateQueries({
                        queryKey: ['sessions', parsed.payload.session_id],
                    });
                    queryClient.invalidateQueries({
                        predicate: (q) => q.queryKey[0] === 'tasks' && q.queryKey[2] === 'sessions',
                    });
                    queryClient.invalidateQueries({ queryKey: ['tasks'] });
                    break;
                }

                // Task lifecycle events
                case 'TaskCreated':
                case 'TaskUpdated': {
                    queryClient.invalidateQueries({ queryKey: ['tasks'] });
                    break;
                }

                case 'TaskDeleted': {
                    queryClient.invalidateQueries({ queryKey: ['tasks'] });
                    break;
                }

                case 'TaskMoved': {
                    queryClient.invalidateQueries({ queryKey: ['tasks'] });
                    break;
                }

                // Agent lifecycle events
                case 'AgentStatusChanged': {
                    queryClient.invalidateQueries({ queryKey: ['agents'] });
                    queryClient.invalidateQueries({
                        queryKey: ['agents', parsed.payload.agent_id, 'health'],
                    });
                    break;
                }

                case 'AgentHealthUpdated': {
                    queryClient.invalidateQueries({ queryKey: ['agents'] });
                    queryClient.invalidateQueries({
                        queryKey: ['agents', parsed.payload.agent_id, 'health'],
                    });
                    break;
                }

                // Worktree events
                case 'WorktreeCreated':
                case 'WorktreeDeleted': {
                    // No UI currently showing worktree list, but invalidate if added
                    break;
                }

                // Project events
                case 'ProjectCreated':
                case 'ProjectUpdated': {
                    queryClient.invalidateQueries({ queryKey: ['projects'] });
                    break;
                }

                case 'ProjectDeleted': {
                    queryClient.invalidateQueries({ queryKey: ['projects'] });
                    break;
                }

                case 'ProjectRepositoryAdded':
                case 'ProjectRepositoryRemoved': {
                    queryClient.invalidateQueries({
                        queryKey: ['projects', parsed.payload.project_id, 'repositories'],
                    });
                    break;
                }

                default:
                    break;
            }
        },
        [queryClient, append],
    );

    const connect = useCallback(() => {
        if (!mountedRef.current) return;
        if (wsRef.current?.readyState === WebSocket.OPEN || wsRef.current?.readyState === WebSocket.CONNECTING) {
            return;
        }

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${window.location.host}/api/ws`;

        setStatus('connecting');
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onopen = () => {
            if (!mountedRef.current) {
                ws.close();
                return;
            }
            setStatus('connected');
            reconnectDelayRef.current = RECONNECT_DELAY_MS;
        };

        ws.onmessage = handleMessage;

        ws.onclose = () => {
            // Fix #15: Guard against StrictMode double-mount stale closures
            if (wsRef.current !== ws) return;
            if (!mountedRef.current) return;
            setStatus('disconnected');
            wsRef.current = null;

            // Exponential backoff reconnect
            const delay = reconnectDelayRef.current;
            reconnectTimerRef.current = setTimeout(() => {
                // Fix #20: Null the timer ref when it fires
                reconnectTimerRef.current = null;
                connect();
            }, delay);
            reconnectDelayRef.current = Math.min(
                delay * 1.5,
                MAX_RECONNECT_DELAY_MS,
            );
        };

        ws.onerror = () => {
            // onclose will fire after onerror, triggering reconnect
        };
    }, [handleMessage]);

    useEffect(() => {
        mountedRef.current = true;
        connect();

        return () => {
            mountedRef.current = false;
            if (reconnectTimerRef.current) {
                clearTimeout(reconnectTimerRef.current);
                reconnectTimerRef.current = null;
            }
            if (wsRef.current) {
                wsRef.current.close();
                wsRef.current = null;
            }
        };
    }, [connect]);

    return { status };
}
