import { create } from 'zustand';

interface SessionLog {
    session_id: string;
    log_type: string;
    content: string;
}

interface SessionOutputStore {
    outputs: Record<string, SessionLog[]>;
    append: (sessionId: string, log: SessionLog) => void;
    clear: (sessionId: string) => void;
}

export const useSessionOutputStore = create<SessionOutputStore>((set) => ({
    outputs: {},
    append: (sessionId, log) =>
        set((state) => ({
            outputs: {
                ...state.outputs,
                [sessionId]: [...(state.outputs[sessionId] ?? []), log],
            },
        })),
    clear: (sessionId) =>
        set((state) => {
            const { [sessionId]: _, ...rest } = state.outputs;
            return { outputs: rest };
        }),
}));
