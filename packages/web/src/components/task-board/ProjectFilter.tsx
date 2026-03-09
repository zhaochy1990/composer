import { useState, useRef, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import type { Project } from '@/types/generated';
import { cn } from '@/lib/utils';

interface ProjectFilterProps {
    projects: Project[];
    selected: string[];
    includeNoProject: boolean;
    onChange: (selected: string[], includeNoProject: boolean) => void;
}

export function ProjectFilter({ projects, selected, includeNoProject, onChange }: ProjectFilterProps) {
    const [open, setOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);

    // Click outside to close
    useEffect(() => {
        if (!open) return;
        function handleMouseDown(e: MouseEvent) {
            if (ref.current && !ref.current.contains(e.target as Node)) {
                setOpen(false);
            }
        }
        document.addEventListener('mousedown', handleMouseDown);
        return () => document.removeEventListener('mousedown', handleMouseDown);
    }, [open]);

    // Escape to close
    useEffect(() => {
        if (!open) return;
        function handleKeyDown(e: KeyboardEvent) {
            if (e.key === 'Escape') {
                e.stopImmediatePropagation();
                setOpen(false);
            }
        }
        document.addEventListener('keydown', handleKeyDown);
        return () => document.removeEventListener('keydown', handleKeyDown);
    }, [open]);

    const isFiltering = selected.length > 0 || includeNoProject;

    function toggleProject(projectId: string) {
        if (selected.includes(projectId)) {
            onChange(selected.filter(id => id !== projectId), includeNoProject);
        } else {
            onChange([...selected, projectId], includeNoProject);
        }
    }

    function toggleNoProject() {
        onChange(selected, !includeNoProject);
    }

    function clearAll() {
        onChange([], false);
    }

    return (
        <div className="relative" ref={ref}>
            <button
                type="button"
                onClick={() => setOpen(!open)}
                aria-expanded={open}
                aria-haspopup="listbox"
                className={cn(
                    'flex items-center gap-1.5 px-2.5 py-1 text-xs rounded border transition-colors',
                    isFiltering
                        ? 'bg-teal-900/40 text-teal-300 border-teal-700'
                        : 'bg-transparent text-gray-500 border-gray-700 hover:text-gray-300 hover:border-gray-500'
                )}
            >
                Project
                {isFiltering && (
                    <span className="bg-teal-800 text-teal-200 px-1 rounded text-[10px]">
                        {selected.length + (includeNoProject ? 1 : 0)}
                    </span>
                )}
                <ChevronDown className="w-3 h-3" />
            </button>

            {open && (
                <div className="absolute top-full left-0 mt-1 z-50 w-56 bg-gray-900 border border-gray-700 rounded-md shadow-lg py-1">
                    <div className="flex items-center justify-between px-3 py-1.5 border-b border-gray-800">
                        <span className="text-xs text-gray-500">Filter by project</span>
                        {isFiltering && (
                            <button
                                type="button"
                                onClick={clearAll}
                                className="text-xs text-gray-400 hover:text-gray-200"
                            >
                                Clear
                            </button>
                        )}
                    </div>

                    <div className="max-h-64 overflow-y-auto">
                        <label className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer hover:bg-gray-800">
                            <input
                                type="checkbox"
                                checked={includeNoProject}
                                onChange={toggleNoProject}
                                className="rounded border-gray-600 bg-gray-700 text-blue-500 focus:ring-blue-500"
                            />
                            <span className="text-gray-400 italic">No project</span>
                        </label>

                        <div className="border-t border-gray-800 my-0.5" />

                        {projects.map(p => (
                            <label
                                key={p.id}
                                className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer hover:bg-gray-800"
                            >
                                <input
                                    type="checkbox"
                                    checked={selected.includes(p.id)}
                                    onChange={() => toggleProject(p.id)}
                                    className="rounded border-gray-600 bg-gray-700 text-blue-500 focus:ring-blue-500"
                                />
                                <span className="text-gray-300 truncate">{p.name}</span>
                            </label>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
