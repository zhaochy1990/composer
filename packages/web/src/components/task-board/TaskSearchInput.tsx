import { Search, X } from 'lucide-react';

interface TaskSearchInputProps {
    value: string;
    onChange: (value: string) => void;
}

export function TaskSearchInput({ value, onChange }: TaskSearchInputProps) {
    return (
        <div className="relative flex items-center">
            <Search className="absolute left-2.5 w-3.5 h-3.5 text-text-muted pointer-events-none" />
            <input
                type="text"
                value={value}
                onChange={(e) => onChange(e.target.value)}
                onKeyDown={(e) => {
                    if (e.key === 'Escape') {
                        e.nativeEvent.stopImmediatePropagation();
                        onChange('');
                        (e.target as HTMLInputElement).blur();
                    }
                }}
                placeholder="Search tasks..."
                className="w-48 bg-bg-elevated border border-border-primary rounded-md pl-8 pr-7 py-1 text-xs text-text-primary placeholder-text-muted focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
            />
            {value && (
                <button
                    type="button"
                    onClick={() => onChange('')}
                    className="absolute right-2 text-text-muted hover:text-text-secondary transition-colors"
                >
                    <X className="w-3.5 h-3.5" />
                </button>
            )}
        </div>
    );
}
