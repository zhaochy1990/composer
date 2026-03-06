import { useState, useEffect, useRef, useMemo } from 'react';
import { X, Trash2, Square, Play, Send, RotateCcw, ChevronDown, ChevronRight, ExternalLink, GitPullRequest, Workflow as WorkflowIcon } from 'lucide-react';
import type { Task } from '@/types/generated';
import { useUpdateTask, useDeleteTask, useStartTask } from '@/hooks/use-tasks';
import { useTaskSessions } from '@/hooks/use-task-sessions';
import { useSession, useInterruptSession, useResumeSession, useSendSessionInput, useRetrySession } from '@/hooks/use-sessions';
import { useAgents } from '@/hooks/use-agents';
import { useProjects } from '@/hooks/use-projects';
import { useWorkflows as useAllWorkflows, useWorkflowRun, useWorkflow, useStartWorkflow } from '@/hooks/use-workflows';
import { SessionOutput } from '@/components/sessions/SessionOutput';
import { StatusBadge } from '@/components/sessions/StatusBadge';
import { WorkflowProgress } from '@/components/workflows/WorkflowProgress';
import { PlanReviewPanel } from '@/components/workflows/PlanReviewPanel';
import { shortId, formatDuration, formatTime } from '@/lib/utils';

interface TaskDetailPanelProps {
    task: Task;
    onClose: () => void;
    inline?: boolean;
}

export function TaskDetailPanel({ task, onClose, inline = false }: TaskDetailPanelProps) {
    // --- Task edit form state ---
    const [title, setTitle] = useState(task.title);
    const [description, setDescription] = useState(task.description ?? '');
    const [priority, setPriority] = useState(task.priority);

    const [assignedAgentId, setAssignedAgentId] = useState(task.assigned_agent_id ?? '');
    const [projectId, setProjectId] = useState(task.project_id ?? '');
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
    const [formCollapsed, setFormCollapsed] = useState(false);

    useEffect(() => {
        setTitle(task.title);
        setDescription(task.description ?? '');
        setPriority(task.priority);
        setAssignedAgentId(task.assigned_agent_id ?? '');
        setProjectId(task.project_id ?? '');
        setSelectedWorkflowId(task.workflow_id ?? '');
        setShowDeleteConfirm(false);
    }, [task.id, task.updated_at]);

    const updateTask = useUpdateTask();
    const deleteTask = useDeleteTask();
    const startTask = useStartTask();

    // --- Sessions ---
    const { data: sessions } = useTaskSessions(task.id);
    const { data: agents } = useAgents();
    const { data: projects } = useProjects();
    const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);

    const interruptMutation = useInterruptSession();
    const resumeMutation = useResumeSession();
    const sendInputMutation = useSendSessionInput();
    const retryMutation = useRetrySession();
    const [messageInput, setMessageInput] = useState('');

    // --- Workflows ---
    const { data: allWorkflows } = useAllWorkflows();
    const [selectedWorkflowId, setSelectedWorkflowId] = useState<string>(task.workflow_id ?? '');
    const startWorkflow = useStartWorkflow();
    const { data: workflowRun } = useWorkflowRun(task.workflow_run_id ?? undefined);
    const { data: workflow } = useWorkflow(workflowRun?.workflow_id ?? undefined);
    const [planContent, setPlanContent] = useState<string | null>(null);

    // Default to first available agent if not set
    useEffect(() => {
        if (agents?.length && !assignedAgentId) {
            setAssignedAgentId(agents[0].id);
        }
    }, [agents, assignedAgentId]);

    // Build agent name map
    const agentNameMap = useMemo(() => {
        const map: Record<string, string> = {};
        if (agents) {
            for (const agent of agents) {
                map[agent.id] = agent.name;
            }
        }
        return map;
    }, [agents]);

    // Sort sessions: running first, then by created_at descending
    const sortedSessions = useMemo(() => {
        const list = [...(sessions ?? [])];
        const statusOrder: Record<string, number> = {
            running: 0, paused: 1, created: 2, failed: 3, completed: 4,
        };
        list.sort((a, b) => {
            const orderA = statusOrder[a.status] ?? 5;
            const orderB = statusOrder[b.status] ?? 5;
            if (orderA !== orderB) return orderA - orderB;
            return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        });
        return list;
    }, [sessions]);

    // Auto-select active session (once, on first data load)
    const hasAutoSelected = useRef(false);
    useEffect(() => {
        if (hasAutoSelected.current) return;
        const active = sortedSessions.find(s => s.status === 'running' || s.status === 'paused');
        if (active) {
            setSelectedSessionId(active.id);
            setFormCollapsed(true);
            hasAutoSelected.current = true;
        }
    }, [sortedSessions]);

    const { data: selectedSession, isLoading: selectedSessionLoading } = useSession(selectedSessionId ?? undefined);

    const [saved, setSaved] = useState(false);

    // Clear saved indicator when task data refreshes
    useEffect(() => { setSaved(false); }, [task.updated_at]);

    // --- Handlers ---
    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!title.trim()) return;

        updateTask.mutate(
            {
                id: task.id,
                title: title.trim(),
                description: description.trim() || undefined,
                priority,
                assigned_agent_id: assignedAgentId || undefined,
                project_id: projectId || undefined,
            },
            { onSuccess: () => setSaved(true) },
        );
    }

    function handleDelete() {
        deleteTask.mutate(task.id, { onSuccess: () => onClose() });
    }

    const isRunning = selectedSession?.status === 'running';
    const isPaused = selectedSession?.status === 'paused';
    const isFailed = selectedSession?.status === 'failed';
    const isCompleted = selectedSession?.status === 'completed';

    // Shared panel body — used by both inline and overlay modes
    const panelContent = (
        <>
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-3 border-b border-gray-800">
                <div className="flex items-center gap-3">
                    {task.simple_id && (
                        <span className="font-mono text-sm text-gray-400 bg-gray-800 px-2 py-0.5 rounded">{task.simple_id}</span>
                    )}
                    <h2 className="text-lg font-semibold text-gray-100">{task.title}</h2>
                </div>
                {!inline && (
                    <button
                        type="button"
                        onClick={onClose}
                        className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                    >
                        <X className="w-4 h-4" />
                    </button>
                )}
            </div>

            {/* PR Links */}
            {task.pr_urls.length > 0 && (
                <div className="px-6 py-2 border-b border-gray-800 shrink-0 flex items-center gap-2 flex-wrap">
                    <GitPullRequest className="w-3.5 h-3.5 text-green-400 shrink-0" />
                    {task.pr_urls.map((url) => (
                        <a
                            key={url}
                            href={url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded bg-green-900/40 text-green-300 border border-green-700 hover:bg-green-900/60 transition-colors"
                        >
                            {url.replace(/^https?:\/\//, '').replace(/\/pull\//, '/pull/').replace(/\/pullrequest\//, '/pr/').replace(/\/merge_requests\//, '/mr/')}
                            <ExternalLink className="w-3 h-3" />
                        </a>
                    ))}
                </div>
            )}

            {/* Workflow Progress */}
            {workflowRun && workflow && (
                <div className="px-6 py-3 border-b border-gray-800 shrink-0">
                    <WorkflowProgress workflowRun={workflowRun} workflow={workflow} onPlanContent={setPlanContent} />
                </div>
            )}

            {/* Plan Review Panel — rendered inline when in inline mode */}
            {inline && planContent && (
                <div className="px-6 py-3 border-b border-gray-800 shrink-0">
                    <PlanReviewPanel content={planContent} onClose={() => setPlanContent(null)} />
                </div>
            )}

            {/* Collapsible Task Edit Form */}
            <div className="border-b border-gray-800 shrink-0">
                <button
                    type="button"
                    onClick={() => setFormCollapsed(!formCollapsed)}
                    className="flex items-center gap-2 w-full px-6 py-2.5 text-left text-sm font-semibold text-gray-400 uppercase tracking-wider hover:bg-gray-800/50 transition-colors"
                >
                    {formCollapsed ? <ChevronRight className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                    Task Details
                </button>
                {!formCollapsed && (
                    <form onSubmit={handleSubmit} className="px-6 pb-4">
                        <div className="space-y-3">
                            <div>
                                <label htmlFor="edit-title" className="block text-sm font-medium text-gray-300 mb-1">
                                    Title <span className="text-red-400">*</span>
                                </label>
                                <input
                                    id="edit-title"
                                    type="text"
                                    value={title}
                                    onChange={e => setTitle(e.target.value)}
                                    placeholder="Task title"
                                    required
                                    className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                />
                            </div>

                            <div>
                                <label htmlFor="edit-description" className="block text-sm font-medium text-gray-300 mb-1">
                                    Description
                                </label>
                                <textarea
                                    id="edit-description"
                                    value={description}
                                    onChange={e => setDescription(e.target.value)}
                                    placeholder="Optional description"
                                    rows={2}
                                    className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 resize-none"
                                />
                            </div>

                            <div className="grid grid-cols-3 gap-3">
                                <div>
                                    <label htmlFor="edit-priority" className="block text-sm font-medium text-gray-300 mb-1">
                                        Priority
                                    </label>
                                    <select
                                        id="edit-priority"
                                        value={priority}
                                        onChange={e => setPriority(Number(e.target.value))}
                                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                    >
                                        <option value={0}>None</option>
                                        <option value={1}>Low</option>
                                        <option value={2}>Medium</option>
                                        <option value={3}>High</option>
                                    </select>
                                </div>

                                <div>
                                    <label htmlFor="edit-agent" className="block text-sm font-medium text-gray-300 mb-1">
                                        Agent
                                    </label>
                                    <select
                                        id="edit-agent"
                                        value={assignedAgentId}
                                        onChange={e => setAssignedAgentId(e.target.value)}
                                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                    >
                                        <option value="">None</option>
                                        {agents?.map(agent => (
                                            <option key={agent.id} value={agent.id}>{agent.name}</option>
                                        ))}
                                    </select>
                                </div>

                                <div>
                                    <label htmlFor="edit-project" className="block text-sm font-medium text-gray-300 mb-1">
                                        Project
                                    </label>
                                    <select
                                        id="edit-project"
                                        value={projectId}
                                        onChange={e => setProjectId(e.target.value)}
                                        className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                                    >
                                        <option value="">None</option>
                                        {projects?.map(p => (
                                            <option key={p.id} value={p.id}>{p.name}</option>
                                        ))}
                                    </select>
                                </div>
                            </div>

                            <div className="flex items-center justify-between pt-1">
                                <div className="flex items-center gap-3">
                                    {!showDeleteConfirm ? (
                                        <button
                                            type="button"
                                            onClick={() => setShowDeleteConfirm(true)}
                                            className="flex items-center gap-1 px-3 py-1.5 text-sm text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-md transition-colors"
                                        >
                                            <Trash2 className="w-3.5 h-3.5" />
                                            Delete
                                        </button>
                                    ) : (
                                        <div className="flex items-center gap-2">
                                            <span className="text-sm text-red-400">Delete?</span>
                                            <button
                                                type="button"
                                                onClick={handleDelete}
                                                disabled={deleteTask.isPending}
                                                className="px-3 py-1 text-sm text-white bg-red-600 rounded-md hover:bg-red-500 transition-colors disabled:opacity-50"
                                            >
                                                {deleteTask.isPending ? '...' : 'Yes'}
                                            </button>
                                            <button
                                                type="button"
                                                onClick={() => setShowDeleteConfirm(false)}
                                                className="px-3 py-1 text-sm text-gray-300 bg-gray-800 rounded-md hover:bg-gray-700 transition-colors"
                                            >
                                                No
                                            </button>
                                        </div>
                                    )}
                                    <span className="text-xs text-gray-500">
                                        Created {new Date(task.created_at).toLocaleString()}
                                    </span>
                                </div>

                                <div className="flex gap-2">
                                    {!inline && (
                                        <button
                                            type="button"
                                            onClick={onClose}
                                            className="px-3 py-1.5 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 transition-colors"
                                        >
                                            Cancel
                                        </button>
                                    )}
                                    <button
                                        type="submit"
                                        disabled={!title.trim() || updateTask.isPending}
                                        className={`px-3 py-1.5 text-sm text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${saved ? 'bg-green-600 hover:bg-green-500' : 'bg-blue-600 hover:bg-blue-500'}`}
                                    >
                                        {updateTask.isPending ? 'Saving...' : saved ? 'Saved' : 'Save'}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </form>
                )}
            </div>

            {/* Sessions list (compact) */}
            <div className="px-6 py-2.5 border-b border-gray-800 shrink-0">
                <div className="flex items-center justify-between mb-2">
                    <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Sessions</h3>
                    {task.status === 'backlog' && (() => {
                        const missingAgent = !task.assigned_agent_id;
                        const missingProject = !task.project_id;
                        const canStart = !missingAgent && !missingProject;
                        const hasWorkflows = allWorkflows && allWorkflows.length > 0;
                        const tooltip = missingAgent && missingProject
                            ? 'Assign an agent and project first'
                            : missingAgent ? 'Assign an agent first'
                            : missingProject ? 'Assign a project first'
                            : undefined;
                        return (
                            <div className="flex items-center gap-2">
                                {hasWorkflows && (
                                    <>
                                        <select
                                            value={selectedWorkflowId}
                                            onChange={(e) => setSelectedWorkflowId(e.target.value)}
                                            className="bg-gray-800 border border-gray-600 rounded-md px-2 py-1 text-xs text-gray-100 focus:outline-none focus:border-purple-500"
                                        >
                                            <option value="">No workflow</option>
                                            {allWorkflows.map(wf => (
                                                <option key={wf.id} value={wf.id}>{wf.name}</option>
                                            ))}
                                        </select>
                                        {selectedWorkflowId && (
                                            <button
                                                type="button"
                                                title={tooltip}
                                                onClick={() => startWorkflow.mutate(
                                                    { taskId: task.id, workflowId: selectedWorkflowId },
                                                    {
                                                        onSuccess: () => setFormCollapsed(true),
                                                    },
                                                )}
                                                disabled={!canStart || startWorkflow.isPending}
                                                className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-purple-600 text-white hover:bg-purple-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                            >
                                                <WorkflowIcon className="w-3 h-3" />
                                                {startWorkflow.isPending ? 'Starting...' : 'Start Workflow'}
                                            </button>
                                        )}
                                    </>
                                )}
                                {!selectedWorkflowId && (
                                    <button
                                        type="button"
                                        title={tooltip}
                                        onClick={() => startTask.mutate(task.id, {
                                            onSuccess: (data) => {
                                                setSelectedSessionId(data.session.id);
                                                setFormCollapsed(true);
                                            },
                                        })}
                                        disabled={!canStart || startTask.isPending}
                                        className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-green-600 text-white hover:bg-green-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        <Play className="w-3 h-3" />
                                        {startTask.isPending ? 'Starting...' : 'Start'}
                                    </button>
                                )}
                            </div>
                        );
                    })()}
                    {(startTask.isError || startWorkflow.isError) && (
                        <p className="text-xs text-red-400 mt-1">{((startTask.error || startWorkflow.error) as Error).message}</p>
                    )}
                </div>

                {sortedSessions.length === 0 ? (
                    <p className="text-sm text-gray-500 py-2 text-center">
                        No sessions yet{task.status === 'backlog' ? ' — click Start to begin' : ''}
                    </p>
                ) : (
                    <div className="flex flex-col gap-0.5">
                        {sortedSessions.map((session) => (
                            <button
                                key={session.id}
                                type="button"
                                onClick={() => {
                                    setSelectedSessionId(session.id);
                                    setFormCollapsed(true);
                                }}
                                className={`flex items-center gap-3 px-3 py-1.5 rounded-md text-sm text-left transition-colors ${
                                    selectedSessionId === session.id
                                        ? 'bg-gray-700 text-gray-100'
                                        : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
                                }`}
                            >
                                <span className={`text-xs truncate max-w-[160px] ${session.name ? 'font-medium' : 'font-mono'}`}>{session.name ?? shortId(session.id)}</span>
                                <StatusBadge status={session.status} />
                                <span className="truncate">{agentNameMap[session.agent_id] ?? shortId(session.agent_id)}</span>
                                <span className="ml-auto text-xs text-gray-500">
                                    {formatDuration(session.started_at, session.completed_at)}
                                </span>
                                <span className="text-xs text-gray-600">{formatTime(session.created_at)}</span>
                            </button>
                        ))}
                    </div>
                )}
            </div>

            {/* Session Output — takes all remaining space */}
            {selectedSessionId && selectedSessionLoading && (
                <div className="flex-1 flex items-center justify-center">
                    <p className="text-sm text-gray-500">Loading session...</p>
                </div>
            )}
            {selectedSessionId && selectedSession && (
                <div className="flex-1 flex flex-col min-h-0">
                    {/* Session action bar + metadata */}
                    <div className="px-6 py-2 flex items-center gap-3 border-b border-gray-800 shrink-0">
                        {isRunning && (
                            <button
                                type="button"
                                onClick={() => interruptMutation.mutate(selectedSession.id)}
                                disabled={interruptMutation.isPending}
                                className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-red-900/40 text-red-300 border border-red-700 hover:bg-red-900/60 transition-colors disabled:opacity-50"
                            >
                                <Square className="w-3 h-3" />
                                {interruptMutation.isPending ? 'Interrupting...' : 'Interrupt'}
                            </button>
                        )}
                        {isPaused && (
                            <button
                                type="button"
                                onClick={() => resumeMutation.mutate({ id: selectedSession.id })}
                                disabled={resumeMutation.isPending}
                                className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-green-900/40 text-green-300 border border-green-700 hover:bg-green-900/60 transition-colors disabled:opacity-50"
                            >
                                <Play className="w-3 h-3" />
                                {resumeMutation.isPending ? 'Resuming...' : 'Resume'}
                            </button>
                        )}
                        {isFailed && (
                            <>
                                <button
                                    type="button"
                                    onClick={() => retryMutation.mutate({ id: selectedSession.id })}
                                    disabled={retryMutation.isPending}
                                    className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-orange-900/40 text-orange-300 border border-orange-700 hover:bg-orange-900/60 transition-colors disabled:opacity-50"
                                >
                                    <RotateCcw className="w-3 h-3" />
                                    {retryMutation.isPending ? 'Retrying...' : 'Retry'}
                                </button>
                                {retryMutation.isError && (
                                    <span className="text-xs text-red-400">{(retryMutation.error as Error).message}</span>
                                )}
                            </>
                        )}
                        {isCompleted && (
                            <button
                                type="button"
                                onClick={() => resumeMutation.mutate({ id: selectedSession.id, continueChat: true })}
                                disabled={resumeMutation.isPending}
                                className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-blue-900/40 text-blue-300 border border-blue-700 hover:bg-blue-900/60 transition-colors disabled:opacity-50"
                            >
                                <Send className="w-3 h-3" />
                                {resumeMutation.isPending ? 'Continuing...' : 'Continue Chat'}
                            </button>
                        )}
                        {selectedSession.prompt && (
                            <span className="text-xs text-gray-500 truncate max-w-md" title={selectedSession.prompt}>
                                Prompt: {selectedSession.prompt}
                            </span>
                        )}
                        {selectedSession.result_summary && (
                            <span className="ml-auto text-xs text-yellow-400 truncate max-w-xs" title={selectedSession.result_summary}>
                                Result: {selectedSession.result_summary}
                            </span>
                        )}
                    </div>

                    {/* Output — fills remaining space */}
                    <div className="flex-1 min-h-0">
                        <SessionOutput sessionId={selectedSessionId} />
                    </div>

                    {/* Message input pinned at bottom */}
                    {isRunning && (
                        <div className="px-6 py-3 border-t border-gray-800 shrink-0">
                            <form
                                onSubmit={(e) => {
                                    e.preventDefault();
                                    const msg = messageInput.trim();
                                    if (!msg) return;
                                    sendInputMutation.mutate(
                                        { id: selectedSession.id, message: msg },
                                        { onSuccess: () => setMessageInput('') },
                                    );
                                }}
                                className="flex items-center gap-2"
                            >
                                <input
                                    type="text"
                                    value={messageInput}
                                    onChange={(e) => setMessageInput(e.target.value)}
                                    placeholder="Send a message to the session..."
                                    className="flex-1 bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-green-500 focus:ring-1 focus:ring-green-500"
                                />
                                <button
                                    type="submit"
                                    disabled={!messageInput.trim() || sendInputMutation.isPending}
                                    className="flex items-center gap-1.5 px-3 py-2 rounded-md text-sm font-medium bg-green-700 text-white hover:bg-green-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    <Send className="w-3.5 h-3.5" />
                                    Send
                                </button>
                            </form>
                        </div>
                    )}
                </div>
            )}
            {!selectedSessionId && sortedSessions.length > 0 && (
                <div className="flex-1 flex items-center justify-center">
                    <p className="text-sm text-gray-500">Select a session above to view its output</p>
                </div>
            )}
        </>
    );

    if (inline) {
        return (
            <div className="flex flex-col h-full bg-gray-900 overflow-hidden">
                {panelContent}
            </div>
        );
    }

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/40 z-40"
                onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}
            />

            {/* Plan Review Panel — left of task detail panel */}
            {planContent && (
                <PlanReviewPanel content={planContent} onClose={() => setPlanContent(null)} />
            )}

            {/* Panel */}
            <div className="fixed inset-y-0 right-0 w-[900px] max-w-full z-50 bg-gray-900 border-l border-gray-700 shadow-2xl flex flex-col overflow-hidden">
                {panelContent}
            </div>

        </>
    );
}
