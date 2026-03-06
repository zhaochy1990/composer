import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiFetch } from '@/lib/api';
import { logger } from '@/lib/logger';
import type {
    Workflow,
    WorkflowRun,
    WorkflowStepOutput,
    CreateWorkflowRequest,
    UpdateWorkflowRequest,
    WorkflowResumeRequest,
    WorkflowDefinition,
    ValidationResult,
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

export function useCreateWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: CreateWorkflowRequest) =>
            apiFetch<Workflow>('/workflows', { method: 'POST', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
        onError: (error: Error) => logger.error('Failed to create workflow', { error: error.message }),
    });
}

export function useUpdateWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, ...data }: { id: string } & UpdateWorkflowRequest) =>
            apiFetch<Workflow>(`/workflows/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
        onError: (error: Error) => logger.error('Failed to update workflow', { error: error.message }),
    });
}

export function useDeleteWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<void>(`/workflows/${id}`, { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
        onError: (error: Error) => logger.error('Failed to delete workflow', { error: error.message }),
    });
}

export function useCloneWorkflow() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (id: string) =>
            apiFetch<Workflow>(`/workflows/${id}/clone`, { method: 'POST' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ['workflows'] }),
        onError: (error: Error) => logger.error('Failed to clone workflow', { error: error.message }),
    });
}

export function useValidateWorkflow() {
    return useMutation({
        mutationFn: (definition: WorkflowDefinition) =>
            apiFetch<ValidationResult>(`/workflows/validate`, {
                method: 'POST',
                body: JSON.stringify(definition),
            }),
        onError: (error: Error) => logger.error('Failed to validate workflow', { error: error.message }),
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
        onError: (error: Error) => logger.error('Failed to start workflow', { error: error.message }),
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

export function useWorkflowStepOutputs(runId: string | undefined, runStatus?: string) {
    return useQuery({
        queryKey: ['workflow-runs', runId, 'steps'],
        queryFn: () => apiFetch<WorkflowStepOutput[]>(`/workflow-runs/${runId}/steps`),
        enabled: !!runId,
        refetchInterval: () => {
            if (runStatus && runStatus !== 'running' && runStatus !== 'paused') {
                return false;
            }
            return 5000;
        },
    });
}

export function useResumeWorkflowRun() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ runId, ...req }: { runId: string } & WorkflowResumeRequest) =>
            apiFetch<WorkflowRun>(`/workflow-runs/${runId}/resume`, {
                method: 'POST',
                body: JSON.stringify(req),
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
        onError: (error: Error) => logger.error('Failed to resume workflow run', { error: error.message }),
    });
}

export function useSubmitWorkflowDecision() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ runId, stepId, approved, comments }: {
            runId: string;
            stepId: string;
            approved: boolean;
            comments?: string;
        }) =>
            apiFetch<WorkflowRun>(`/workflow-runs/${runId}/decision`, {
                method: 'POST',
                body: JSON.stringify({ step_id: stepId, approved, comments }),
            }),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['workflow-runs'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            queryClient.invalidateQueries({ queryKey: ['sessions'] });
        },
        onError: (error: Error) => logger.error('Failed to submit workflow decision', { error: error.message }),
    });
}
