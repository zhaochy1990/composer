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
            {/* Left sidebar — compact workflow list */}
            <div className="w-[280px] shrink-0 border-r border-border-primary flex flex-col bg-bg-surface">
                <div className="flex items-center justify-between px-4 py-3 border-b border-border-primary">
                    <h1 className="text-sm font-semibold text-text-primary">Workflows</h1>
                    <button
                        onClick={() => setCreateOpen(true)}
                        className="p-1.5 text-text-muted hover:text-white hover:bg-bg-elevated rounded"
                        title="New Workflow"
                    >
                        <Plus className="w-4 h-4" />
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto">
                    {isLoading && (
                        <p className="text-xs text-text-muted p-4 text-center">Loading...</p>
                    )}

                    {isError && (
                        <p className="text-xs text-red-400 p-4 text-center">Failed to load.</p>
                    )}

                    {!isLoading && !isError && workflows && workflows.length === 0 && (
                        <div className="flex flex-col items-center justify-center py-12 text-center px-4">
                            <WorkflowIcon className="w-8 h-8 text-text-muted mb-3" />
                            <p className="text-xs text-text-muted">
                                No workflows yet.
                            </p>
                        </div>
                    )}

                    {!isLoading && !isError && workflows && workflows.length > 0 && (
                        <div>
                            {workflows.map(workflow => (
                                <WorkflowCard
                                    key={workflow.id}
                                    workflow={workflow}
                                    onClick={() => setSelectedWorkflow(workflow)}
                                    compact
                                    isSelected={selectedWorkflow?.id === workflow.id}
                                />
                            ))}
                        </div>
                    )}
                </div>
            </div>

            {/* Right main area — editor */}
            <div className="flex-1 min-w-0 overflow-hidden">
                {selectedWorkflow ? (
                    <WorkflowEditPanel
                        workflow={selectedWorkflow}
                        onClose={() => setSelectedWorkflow(null)}
                    />
                ) : (
                    <div className="flex flex-col items-center justify-center h-full text-text-muted">
                        <WorkflowIcon className="w-10 h-10 mb-3 text-text-muted" />
                        <p className="text-sm">Select a workflow to edit</p>
                    </div>
                )}
            </div>

            <WorkflowCreateDialog isOpen={createOpen} onClose={() => setCreateOpen(false)} />
        </div>
    );
}
