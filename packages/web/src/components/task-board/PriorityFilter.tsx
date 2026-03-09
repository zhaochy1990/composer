import { priorityConfig } from './priority-config';

const priorities = [3, 2, 1, 0] as const;

interface PriorityFilterProps {
    selected: number[];
    onChange: (selected: number[]) => void;
}

export function PriorityFilter({ selected, onChange }: PriorityFilterProps) {
    function toggle(priority: number) {
        if (selected.includes(priority)) {
            onChange(selected.filter(p => p !== priority));
        } else {
            onChange([...selected, priority]);
        }
    }

    return (
        <div className="flex items-center gap-1.5">
            <span className="text-xs text-text-muted mr-0.5">Priority:</span>
            {priorities.map(p => {
                const config = priorityConfig[p];
                const active = selected.includes(p);
                return (
                    <button
                        key={p}
                        type="button"
                        onClick={() => toggle(p)}
                        className={`px-2 py-0.5 text-xs rounded border transition-colors ${
                            active
                                ? config.className
                                : 'bg-transparent text-text-muted border-border-primary hover:text-text-secondary hover:border-border-secondary'
                        }`}
                    >
                        {config.label}
                    </button>
                );
            })}
            {selected.length > 0 && (
                <button
                    type="button"
                    onClick={() => onChange([])}
                    className="px-1.5 py-0.5 text-xs text-text-muted hover:text-text-secondary transition-colors"
                >
                    Clear
                </button>
            )}
        </div>
    );
}
