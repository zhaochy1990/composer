import { create } from 'zustand';

interface UiStore {
    currentPage: 'tasks' | 'agents';
    setPage: (page: UiStore['currentPage']) => void;
}

export const useUiStore = create<UiStore>((set) => ({
    currentPage: 'tasks',
    setPage: (page) => set({ currentPage: page }),
}));
