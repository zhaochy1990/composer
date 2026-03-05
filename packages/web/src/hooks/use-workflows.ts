import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import type {
    Workflow,
    WorkflowRun,
    WorkflowStepOutput,
    CreateWorkflowRequest,
    UpdateWorkflowRequest,
} from '@/types/generated';

export function useWorkflows() {
    return useQuery({
        queryKey: ['workflows'],
        queryFn: () => apiFetch<Workflow[]>('/workflows'),
    });
}

export function useWorkflow(id: string | undefined) {
    return useQuery({
        queryKey: ['workflows', id],
        queryFn: () => apiFetch<Workflow>(`/workflows/${id}`),
        enabled: !!id,
    });
}

export function useWorkflowsByProject(projectId: string | undefined) {
    return useQuery({
        queryKey: ['workflows', 'by-project', projectId],
        queryFn: () => apiFetch<Workflow[]>(`/workflows/by-project/${projectId}`),
        enabled: !!projectId,
    });
}

export function useCreateWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: CreateWorkflowRequest) =>
            apiFetch<Workflow>('/workflows', { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
    });
}

export function useUpdateWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, ...data }: { id: string } & UpdateWorkflowRequest) =>
            apiFetch<Workflow>(`/workflows/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
    });
}

export function useDeleteWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/workflows/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
    });
}

export function useStartWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ taskId, workflowId }: { taskId: string; workflowId: string }) =>
            apiFetch<WorkflowRun>(`/tasks/${taskId}/start-workflow`, {
                method: 'POST',
                body: JSON.stringify({ workflow_id: workflowId }),
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
    });
}

export function useWorkflowRun(id: string | undefined) {
    return useQuery({
        queryKey: ['workflow-runs', id],
        queryFn: () => apiFetch<WorkflowRun>(`/workflow-runs/${id}`),
        enabled: !!id,
        refetchInterval: (query) => {
            const data = query.state.data;
            if (data && (data.status === 'running' || data.status === 'paused')) {
                return 5000;
            }
            return false;
        },
    });
}

export function useWorkflowStepOutputs(runId: string | undefined) {
    return useQuery({
        queryKey: ['workflow-runs', runId, 'steps'],
        queryFn: () => apiFetch<WorkflowStepOutput[]>(`/workflow-runs/${runId}/steps`),
        enabled: !!runId,
        refetchInterval: 5000,
    });
}

export function useResumeWorkflowRun() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (runId: string) =>
            apiFetch<WorkflowRun>(`/workflow-runs/${runId}/resume`, { method: 'POST' }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
    });
}

export function useSubmitWorkflowDecision() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ runId, approved, comments }: { runId: string; approved: boolean; comments?: string }) =>
            apiFetch<WorkflowRun>(`/workflow-runs/${runId}/decision`, {
                method: 'POST',
                body: JSON.stringify({ approved, comments }),
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
    });
}
