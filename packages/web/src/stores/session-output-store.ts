import { create } from 'zustand';

const MAX_OUTPUT_LINES = 5000;

export interface SessionLogEntry {
    session_id: string;
    log_type: string;
    content: string;
    seq: number;
}

interface SessionOutputStore {
    outputs: Record<string, SessionLogEntry[]>;
    _seqCounters: Record<string, number>;
    _hydrated: Record<string, boolean>;
    append: (sessionId: string, log: { session_id: string; log_type: string; content: string }) => void;
    hydrate: (sessionId: string, logs: SessionLogEntry[]) => void;
    isHydrated: (sessionId: string) => boolean;
    clear: (sessionId: string) => void;
}

export const useSessionOutputStore = create<SessionOutputStore>((set, get) => ({
    outputs: {},
    _seqCounters: {},
    _hydrated: {},
    append: (sessionId, log) =>
        set((state) => {
            const nextSeq = (state._seqCounters[sessionId] ?? 0) + 1;
            const entry: SessionLogEntry = { ...log, seq: nextSeq };
            const existing = state.outputs[sessionId] ?? [];
            const updated = [...existing, entry];
            // Rolling window cap
            const trimmed = updated.length > MAX_OUTPUT_LINES
                ? updated.slice(updated.length - MAX_OUTPUT_LINES)
                : updated;
            return {
                outputs: { ...state.outputs, [sessionId]: trimmed },
                _seqCounters: { ...state._seqCounters, [sessionId]: nextSeq },
            };
        }),
    hydrate: (sessionId, logs) =>
        set((state) => {
            // Don't overwrite if we already have live data from WebSocket
            if ((state.outputs[sessionId]?.length ?? 0) > 0) {
                return { _hydrated: { ...state._hydrated, [sessionId]: true } };
            }
            const entries: SessionLogEntry[] = logs.map((log, i) => ({
                ...log,
                seq: i + 1,
            }));
            const trimmed = entries.length > MAX_OUTPUT_LINES
                ? entries.slice(entries.length - MAX_OUTPUT_LINES)
                : entries;
            return {
                outputs: { ...state.outputs, [sessionId]: trimmed },
                _seqCounters: { ...state._seqCounters, [sessionId]: entries.length },
                _hydrated: { ...state._hydrated, [sessionId]: true },
            };
        }),
    isHydrated: (sessionId) => get()._hydrated[sessionId] ?? false,
    clear: (sessionId) =>
        set((state) => {
            const { [sessionId]: _, ...restOutputs } = state.outputs;
            const { [sessionId]: __, ...restCounters } = state._seqCounters;
            const { [sessionId]: ___, ...restHydrated } = state._hydrated;
            return { outputs: restOutputs, _seqCounters: restCounters, _hydrated: restHydrated };
        }),
}));
