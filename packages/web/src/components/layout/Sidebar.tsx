import { LayoutDashboard, Bot, FolderGit2, Workflow } from 'lucide-react';
import type { Page } from '@/app/App';
import { ThemeToggle } from './ThemeToggle';

interface SidebarProps {
    currentPage: Page;
    onNavigate: (page: Page) => void;
}

const navItems: { page: Page; label: string; icon: typeof LayoutDashboard }[] = [
    { page: 'tasks', label: 'Task Board', icon: LayoutDashboard },
    { page: 'agents', label: 'Agents', icon: Bot },
    { page: 'projects', label: 'Projects', icon: FolderGit2 },
    { page: 'workflows', label: 'Workflows', icon: Workflow },
];

export function Sidebar({ currentPage, onNavigate }: SidebarProps) {
    return (
        <aside className="w-60 border-r border-border-primary p-4 flex flex-col gap-2">
            <h1 className="text-xl font-bold mb-4 px-2">Composer</h1>
            <nav className="flex flex-col gap-1">
                {navItems.map(({ page, label, icon: Icon }) => (
                    <button
                        key={page}
                        onClick={() => onNavigate(page)}
                        className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left ${
                            currentPage === page
                                ? 'bg-bg-elevated text-text-primary'
                                : 'text-text-muted hover:bg-bg-elevated hover:text-text-primary'
                        }`}
                    >
                        <Icon className="w-4 h-4" />
                        {label}
                    </button>
                ))}
            </nav>
            <div className="mt-auto">
                <ThemeToggle />
            </div>
        </aside>
    );
}
