import { create } from 'zustand';

interface UiStore {
    currentPage: 'tasks' | 'agents' | 'sessions';
    setPage: (page: UiStore['currentPage']) => void;
}

export const useUiStore = create<UiStore>((set) => ({
    currentPage: 'tasks',
    setPage: (page) => set({ currentPage: page }),
}));
