import { create } from 'zustand';

const STORAGE_KEY = 'detailPanelWidth';
export const PANEL_DEFAULT_WIDTH = 900;
export const PANEL_MIN_WIDTH = 400;
export const PANEL_MAX_WIDTH_RATIO = 0.8;

function getInitialWidth(): number {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
        const parsed = parseInt(stored, 10);
        if (!isNaN(parsed) && parsed >= PANEL_MIN_WIDTH) return parsed;
    }
    return PANEL_DEFAULT_WIDTH;
}

interface PanelSizeStore {
    detailPanelWidth: number;
    setDetailPanelWidth: (width: number) => void;
}

export const usePanelSizeStore = create<PanelSizeStore>((set) => ({
    detailPanelWidth: getInitialWidth(),
    setDetailPanelWidth: (width: number) => {
        const maxWidth = window.innerWidth * PANEL_MAX_WIDTH_RATIO;
        const clamped = Math.round(Math.max(PANEL_MIN_WIDTH, Math.min(width, maxWidth)));
        localStorage.setItem(STORAGE_KEY, String(clamped));
        set({ detailPanelWidth: clamped });
    },
}));
