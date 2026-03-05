import { useState } from 'react';
import { Workflow as WorkflowIcon, Plus } from 'lucide-react';
import { useWorkflows } from '@/hooks/use-workflows';
import type { Workflow } from '@/types/generated';
import { WorkflowCard } from './WorkflowCard';
import { WorkflowEditPanel } from './WorkflowEditPanel';
import { WorkflowCreateDialog } from './WorkflowCreateDialog';

export function WorkflowList() {
    const [createOpen, setCreateOpen] = useState(false);
    const [selectedWorkflow, setSelectedWorkflow] = useState<Workflow | null>(null);
    const { data: workflows, isLoading, isError } = useWorkflows();

    return (
        <div className="h-full flex">
            <div className="flex-1 overflow-y-auto p-6">
                <div className="flex items-center justify-between mb-6">
                    <h1 className="text-xl font-bold text-gray-100">Workflows</h1>
                    <button
                        onClick={() => setCreateOpen(true)}
                        className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
                    >
                        <Plus className="w-4 h-4" />
                        New Workflow
                    </button>
                </div>

                {isLoading && (
                    <div className="flex items-center justify-center h-64">
                        <p className="text-sm text-gray-500">Loading workflows...</p>
                    </div>
                )}

                {isError && (
                    <div className="flex items-center justify-center h-64">
                        <p className="text-sm text-red-400">Failed to load workflows.</p>
                    </div>
                )}

                {!isLoading && !isError && workflows && workflows.length === 0 && (
                    <div className="flex flex-col items-center justify-center h-64 text-center">
                        <WorkflowIcon className="w-12 h-12 text-gray-700 mb-4" />
                        <p className="text-sm text-gray-500">
                            No workflows yet. Create one to define how agents execute tasks.
                        </p>
                    </div>
                )}

                {!isLoading && !isError && workflows && workflows.length > 0 && (
                    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                        {workflows.map(workflow => (
                            <WorkflowCard
                                key={workflow.id}
                                workflow={workflow}
                                onClick={() => setSelectedWorkflow(workflow)}
                            />
                        ))}
                    </div>
                )}

                <WorkflowCreateDialog isOpen={createOpen} onClose={() => setCreateOpen(false)} />
            </div>

            {selectedWorkflow && (
                <WorkflowEditPanel
                    workflow={selectedWorkflow}
                    onClose={() => setSelectedWorkflow(null)}
                />
            )}
        </div>
    );
}
