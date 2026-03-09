import { Sun, Monitor, Moon } from 'lucide-react';
import { useThemeStore } from '@/stores/theme-store';

type ThemeMode = 'light' | 'system' | 'dark';

const modes: { value: ThemeMode; icon: typeof Sun; label: string }[] = [
    { value: 'light', icon: Sun, label: 'Light' },
    { value: 'system', icon: Monitor, label: 'System' },
    { value: 'dark', icon: Moon, label: 'Dark' },
];

export function ThemeToggle() {
    const { mode, setMode } = useThemeStore();

    return (
        <div className="flex items-center gap-1 rounded-md border border-border-primary bg-bg-elevated p-1">
            {modes.map(({ value, icon: Icon, label }) => (
                <button
                    key={value}
                    type="button"
                    onClick={() => setMode(value)}
                    title={label}
                    className={`rounded px-2 py-1 transition-colors ${
                        mode === value
                            ? 'bg-bg-interactive text-text-primary'
                            : 'text-text-muted hover:text-text-secondary'
                    }`}
                >
                    <Icon className="h-3.5 w-3.5" />
                </button>
            ))}
        </div>
    );
}
