import { create } from 'zustand';

export interface SessionLogEntry {
    id?: number;
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
            return {
                outputs: { ...state.outputs, [sessionId]: [...existing, entry] },
                _seqCounters: { ...state._seqCounters, [sessionId]: nextSeq },
            };
        }),
    hydrate: (sessionId, logs) =>
        set((state) => {
            const historical: SessionLogEntry[] = logs.map((log, i) => ({
                ...log,
                seq: i + 1,
            }));
            // Merge: prepend historical logs before any live WS data that arrived early
            const existing = state.outputs[sessionId] ?? [];
            const merged = existing.length > 0
                ? [...historical, ...existing.map((e, i) => ({ ...e, seq: historical.length + i + 1 }))]
                : historical;
            return {
                outputs: { ...state.outputs, [sessionId]: merged },
                _seqCounters: { ...state._seqCounters, [sessionId]: merged.length },
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
