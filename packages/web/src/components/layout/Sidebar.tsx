import { LayoutDashboard, Bot, FolderGit2 } from 'lucide-react';
import type { Page } from '@/app/App';

interface SidebarProps {
    currentPage: Page;
    onNavigate: (page: Page) => void;
}

const navItems: { page: Page; label: string; icon: typeof LayoutDashboard }[] = [
    { page: 'tasks', label: 'Task Board', icon: LayoutDashboard },
    { page: 'agents', label: 'Agents', icon: Bot },
    { page: 'projects', label: 'Projects', icon: FolderGit2 },
];

export function Sidebar({ currentPage, onNavigate }: SidebarProps) {
    return (
        <aside className="w-60 border-r border-gray-800 p-4 flex flex-col gap-2">
            <h1 className="text-xl font-bold mb-4 px-2">Composer</h1>
            <nav className="flex flex-col gap-1">
                {navItems.map(({ page, label, icon: Icon }) => (
                    <button
                        key={page}
                        onClick={() => onNavigate(page)}
                        className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left ${
                            currentPage === page
                                ? 'bg-gray-800 text-gray-100'
                                : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
                        }`}
                    >
                        <Icon className="w-4 h-4" />
                        {label}
                    </button>
                ))}
            </nav>
        </aside>
    );
}
