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
    append: (sessionId: string, log: { session_id: string; log_type: string; content: string }) => void;
    clear: (sessionId: string) => void;
}

export const useSessionOutputStore = create<SessionOutputStore>((set) => ({
    outputs: {},
    _seqCounters: {},
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
    clear: (sessionId) =>
        set((state) => {
            const { [sessionId]: _, ...restOutputs } = state.outputs;
            const { [sessionId]: __, ...restCounters } = state._seqCounters;
            return { outputs: restOutputs, _seqCounters: restCounters };
        }),
}));
