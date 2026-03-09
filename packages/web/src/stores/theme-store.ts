import { create } from 'zustand';

type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeStore {
    mode: ThemeMode;
    effective: 'light' | 'dark';
    setMode: (mode: ThemeMode) => void;
}

function getSystemTheme(): 'light' | 'dark' {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function resolveEffective(mode: ThemeMode): 'light' | 'dark' {
    return mode === 'system' ? getSystemTheme() : mode;
}

function applyTheme(effective: 'light' | 'dark') {
    if (effective === 'dark') {
        document.documentElement.classList.add('dark');
    } else {
        document.documentElement.classList.remove('dark');
    }
}

function getInitialMode(): ThemeMode {
    const stored = localStorage.getItem('theme') as ThemeMode | null;
    if (stored === 'light' || stored === 'dark' || stored === 'system') return stored;
    return 'system';
}

const initialMode = getInitialMode();

export const useThemeStore = create<ThemeStore>((set) => ({
    mode: initialMode,
    effective: resolveEffective(initialMode),
    setMode: (mode) => {
        localStorage.setItem('theme', mode);
        const effective = resolveEffective(mode);
        applyTheme(effective);
        set({ mode, effective });
    },
}));

// Listen for system theme changes
if (typeof window !== 'undefined') {
    window
        .matchMedia('(prefers-color-scheme: dark)')
        .addEventListener('change', () => {
            const state = useThemeStore.getState();
            if (state.mode === 'system') {
                const effective = getSystemTheme();
                applyTheme(effective);
                useThemeStore.setState({ effective });
            }
        });
}
