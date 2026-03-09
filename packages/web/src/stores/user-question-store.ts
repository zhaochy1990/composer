import { create } from 'zustand';

export interface PendingQuestion {
    sessionId: string;
    requestId: string;
    questions: unknown;
    planContent: string | null;
}

interface UserQuestionStore {
    pending: Record<string, PendingQuestion>;
    set: (sessionId: string, data: PendingQuestion) => void;
    clear: (sessionId: string) => void;
}

export const useUserQuestionStore = create<UserQuestionStore>((set) => ({
    pending: {},
    set: (sessionId, data) =>
        set((state) => ({
            pending: { ...state.pending, [sessionId]: data },
        })),
    clear: (sessionId) =>
        set((state) => {
            const { [sessionId]: _, ...rest } = state.pending;
            return { pending: rest };
        }),
}));
