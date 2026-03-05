import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TaskBoard } from '@/components/task-board/TaskBoard';
import { AgentPool } from '@/components/agents/AgentPool';
import { ProjectList } from '@/components/projects/ProjectList';
import { WorkflowList } from '@/components/workflows/WorkflowList';
import { Sidebar } from '@/components/layout/Sidebar';
import { useWebSocket } from '@/hooks/use-websocket';

export type Page = 'tasks' | 'agents' | 'projects' | 'workflows';

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            staleTime: 5_000,
            retry: 2,
        },
    },
});

function AppContent() {
    const [page, setPage] = useState<Page>('tasks');
    useWebSocket();

    return (
        <div className="flex h-screen bg-gray-950 text-gray-100">
            <Sidebar currentPage={page} onNavigate={setPage} />
            <main className="flex-1 overflow-hidden">
                {page === 'tasks' && <TaskBoard />}
                {page === 'agents' && <AgentPool />}
                {page === 'projects' && <ProjectList />}
                {page === 'workflows' && <WorkflowList />}
            </main>
        </div>
    );
}

export function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <AppContent />
        </QueryClientProvider>
    );
}
