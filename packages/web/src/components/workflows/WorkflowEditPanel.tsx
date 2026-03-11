import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { X, Plus, Trash2, AlertCircle, Lock, Copy } from 'lucide-react';
import type { Workflow, WorkflowStepDefinition, WorkflowStepType, SessionMode } from '@/types/generated';
import { useUpdateWorkflow, useDeleteWorkflow, useCloneWorkflow } from '@/hooks/use-workflows';
import { RoutableEdge, type EdgeRouting, type HandleSide } from './RoutableEdge';
import {
    ReactFlow,
    Background,
    Controls,
    type Node,
    type Edge,
    type OnNodesChange,
    type NodeProps,
    Handle,
    Position,
    useNodesState,
    useEdgesState,
    MarkerType,
    BackgroundVariant,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STEP_TYPES: { value: WorkflowStepType; label: string }[] = [
    { value: 'agentic', label: 'Agentic' },
    { value: 'human_gate', label: 'Human Gate' },
];

const SESSION_MODES: { value: SessionMode; label: string }[] = [
    { value: 'new', label: 'New Session' },
    { value: 'resume', label: 'Resume Session' },
    { value: 'separate', label: 'Separate Session' },
];

const NODE_COLORS: Record<string, { bg: string; border: string; text: string }> = {
    agentic: { bg: 'bg-blue-950', border: 'border-blue-700', text: 'text-blue-300' },
    human_gate: { bg: 'bg-yellow-950', border: 'border-yellow-700', text: 'text-yellow-300' },
};

// ---------------------------------------------------------------------------
// DAG Validation
// ---------------------------------------------------------------------------

function validateDag(steps: WorkflowStepDefinition[]): string[] {
    const errors: string[] = [];
    const ids = new Set(steps.map(s => s.id));

    const seen = new Set<string>();
    for (const step of steps) {
        if (!step.id.trim()) {
            errors.push(`Step "${step.name || '(unnamed)'}" has no ID`);
        } else if (seen.has(step.id)) {
            errors.push(`Duplicate step ID: "${step.id}"`);
        }
        seen.add(step.id);
    }

    for (const step of steps) {
        if (step.step_type === 'agentic' && !step.prompt_template?.trim()) {
            errors.push(`Step "${step.id}": missing prompt template`);
        }
        if (step.step_type === 'human_gate' && !step.on_approve) {
            errors.push(`Step "${step.id}": missing on_approve target`);
        }
        if (step.interactive && step.step_type !== 'agentic') {
            errors.push(`Step "${step.id}": interactive is only valid for agentic steps`);
        }
        for (const dep of step.depends_on) {
            if (!ids.has(dep)) errors.push(`Step "${step.id}" depends on unknown "${dep}"`);
        }
        if (step.on_approve && !ids.has(step.on_approve)) errors.push(`Step "${step.id}" on_approve → unknown "${step.on_approve}"`);
        if (step.on_reject && !ids.has(step.on_reject)) errors.push(`Step "${step.id}" on_reject → unknown "${step.on_reject}"`);
        if (step.loop_back_to && !ids.has(step.loop_back_to)) errors.push(`Step "${step.id}" loop_back_to → unknown "${step.loop_back_to}"`);
    }

    if (errors.length === 0) {
        const inDegree = new Map<string, number>();
        const adj = new Map<string, string[]>();
        for (const step of steps) { inDegree.set(step.id, 0); adj.set(step.id, []); }
        for (const step of steps) {
            for (const dep of step.depends_on) {
                adj.get(dep)?.push(step.id);
                inDegree.set(step.id, (inDegree.get(step.id) ?? 0) + 1);
            }
        }
        const queue: string[] = [];
        for (const [id, deg] of inDegree) { if (deg === 0) queue.push(id); }
        let visited = 0;
        while (queue.length > 0) {
            const node = queue.shift()!;
            visited++;
            for (const n of adj.get(node) ?? []) {
                const deg = (inDegree.get(n) ?? 1) - 1;
                inDegree.set(n, deg);
                if (deg === 0) queue.push(n);
            }
        }
        if (visited !== steps.length) errors.push('Cycle detected in workflow');
    }

    return errors;
}

// ---------------------------------------------------------------------------
// Custom Node Component
// ---------------------------------------------------------------------------

function StepNode({ data, selected }: NodeProps) {
    const step = data.step as WorkflowStepDefinition;
    const colors = NODE_COLORS[step.step_type] ?? NODE_COLORS.agentic;
    const modeLabel = step.step_type === 'agentic' && step.session_mode
        ? SESSION_MODES.find(m => m.value === step.session_mode)?.label ?? ''
        : '';

    const primaryHandle = "!w-2.5 !h-2.5 !bg-gray-500 !border-gray-400";
    const sideHandle = "!w-1.5 !h-1.5 !bg-gray-600 !border-gray-500 !opacity-40 hover:!opacity-100";

    return (
        <>
            {/* Primary vertical handles */}
            <Handle type="target" position={Position.Top} id="top-target" className={primaryHandle} />
            <Handle type="source" position={Position.Bottom} id="bottom-source" className={primaryHandle} />

            {/* Side handles for backward/lateral edges */}
            <Handle type="source" position={Position.Right} id="right-source" className={sideHandle} />
            <Handle type="target" position={Position.Right} id="right-target" className={sideHandle} />
            <Handle type="source" position={Position.Left} id="left-source" className={sideHandle} />
            <Handle type="target" position={Position.Left} id="left-target" className={sideHandle} />

            {/* Extra handles for edge cases */}
            <Handle type="target" position={Position.Bottom} id="bottom-target" className={sideHandle} />
            <Handle type="source" position={Position.Top} id="top-source" className={sideHandle} />

            <div className={`px-4 py-3 rounded-lg border-2 min-w-[160px] max-w-[220px] ${colors.bg} ${selected ? 'border-white' : colors.border} shadow-lg`}>
                <div className="flex items-center gap-1.5 mb-1">
                    <span className={`text-[10px] font-mono ${colors.text} opacity-70`}>{step.id}</span>
                </div>
                <div className="text-sm font-medium text-text-primary truncate">
                    {step.name || step.id}
                </div>
                <div className="flex items-center gap-1.5 mt-1">
                    <span className={`text-[10px] px-1.5 py-0.5 rounded ${colors.text} bg-black/20`}>
                        {step.step_type === 'human_gate' ? 'Gate' : modeLabel || 'Agent'}
                    </span>
                    {step.interactive && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded text-purple-300 bg-black/20">
                            interactive
                        </span>
                    )}
                    {step.loop_back_to && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded text-orange-300 bg-black/20">
                            loop→{step.loop_back_to}
                        </span>
                    )}
                    {step.max_retries != null && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded text-text-muted bg-black/20">
                            max:{step.max_retries}
                        </span>
                    )}
                </div>
            </div>
        </>
    );
}

const nodeTypes = { stepNode: StepNode };
const edgeTypes = { routable: RoutableEdge };

// ---------------------------------------------------------------------------
// Layout helper — simple top-down layered layout
// ---------------------------------------------------------------------------

function layoutNodes(steps: WorkflowStepDefinition[], existingPositions?: Map<string, { x: number; y: number }>): Node[] {
    // Topological sort for layering
    const inDegree = new Map<string, number>();
    const adj = new Map<string, string[]>();
    for (const s of steps) { inDegree.set(s.id, 0); adj.set(s.id, []); }
    for (const s of steps) {
        for (const dep of s.depends_on) {
            adj.get(dep)?.push(s.id);
            inDegree.set(s.id, (inDegree.get(s.id) ?? 0) + 1);
        }
    }

    const layers: string[][] = [];
    const remaining = new Map(inDegree);
    while (remaining.size > 0) {
        const layer = [...remaining.entries()].filter(([, d]) => d === 0).map(([id]) => id);
        if (layer.length === 0) break; // cycle
        layers.push(layer);
        for (const id of layer) {
            remaining.delete(id);
            for (const n of adj.get(id) ?? []) {
                remaining.set(n, (remaining.get(n) ?? 1) - 1);
            }
        }
    }
    // Add any remaining (cycle members) to final layer
    if (remaining.size > 0) layers.push([...remaining.keys()]);

    const nodeWidth = 200;
    const xGap = 60;
    const yGap = 100;

    return steps.map(step => {
        // Use existing position if available (user dragged)
        if (existingPositions?.has(step.id)) {
            return {
                id: step.id,
                type: 'stepNode',
                position: existingPositions.get(step.id)!,
                data: { step },
            };
        }

        const layerIdx = layers.findIndex(l => l.includes(step.id));
        const layer = layers[layerIdx] ?? [step.id];
        const posInLayer = layer.indexOf(step.id);
        const layerWidth = layer.length * nodeWidth + (layer.length - 1) * xGap;
        const startX = -layerWidth / 2 + nodeWidth / 2;

        return {
            id: step.id,
            type: 'stepNode',
            position: {
                x: startX + posInLayer * (nodeWidth + xGap),
                y: layerIdx * (80 + yGap),
            },
            data: { step },
        };
    });
}

const HANDLE_DEFAULTS: Record<string, { sourceHandle: string; targetHandle: string }> = {
    dependency: { sourceHandle: 'bottom-source', targetHandle: 'top-target' },
    approve: { sourceHandle: 'bottom-source', targetHandle: 'top-target' },
    reject: { sourceHandle: 'right-source', targetHandle: 'right-target' },
    loop: { sourceHandle: 'left-source', targetHandle: 'left-target' },
};

function resolveHandles(
    edgeId: string,
    category: string,
    overrides: Map<string, EdgeRouting>,
): { sourceHandle: string; targetHandle: string } {
    const override = overrides.get(edgeId);
    const defaults = HANDLE_DEFAULTS[category] ?? HANDLE_DEFAULTS.dependency;
    return {
        sourceHandle: override?.sourceSide ? `${override.sourceSide}-source` : defaults.sourceHandle,
        targetHandle: override?.targetSide ? `${override.targetSide}-target` : defaults.targetHandle,
    };
}

function buildEdges(steps: WorkflowStepDefinition[], overrides: Map<string, EdgeRouting>): Edge[] {
    const edges: Edge[] = [];
    for (const step of steps) {
        for (const dep of step.depends_on) {
            const edgeId = `dep-${dep}-${step.id}`;
            edges.push({
                id: edgeId,
                source: dep,
                target: step.id,
                type: 'routable',
                ...resolveHandles(edgeId, 'dependency', overrides),
                data: { edgeCategory: 'dependency', color: '#6b7280' },
                markerEnd: { type: MarkerType.ArrowClosed, color: '#6b7280', width: 16, height: 16 },
            });
        }
        if (step.on_approve) {
            const edgeId = `approve-${step.id}-${step.on_approve}`;
            edges.push({
                id: edgeId,
                source: step.id,
                target: step.on_approve,
                type: 'routable',
                ...resolveHandles(edgeId, 'approve', overrides),
                data: { edgeCategory: 'approve', label: 'approve', color: '#22c55e' },
                markerEnd: { type: MarkerType.ArrowClosed, color: '#22c55e', width: 16, height: 16 },
            });
        }
        if (step.on_reject) {
            const edgeId = `reject-${step.id}-${step.on_reject}`;
            edges.push({
                id: edgeId,
                source: step.id,
                target: step.on_reject,
                type: 'routable',
                ...resolveHandles(edgeId, 'reject', overrides),
                data: { edgeCategory: 'reject', label: 'reject', color: '#ef4444' },
                markerEnd: { type: MarkerType.ArrowClosed, color: '#ef4444', width: 16, height: 16 },
            });
        }
        if (step.loop_back_to) {
            const edgeId = `loop-${step.id}-${step.loop_back_to}`;
            edges.push({
                id: edgeId,
                source: step.id,
                target: step.loop_back_to,
                type: 'routable',
                ...resolveHandles(edgeId, 'loop', overrides),
                data: {
                    edgeCategory: 'loop',
                    label: `loop${step.max_retries != null ? ` (max ${step.max_retries})` : ''}`,
                    color: '#f97316',
                    strokeDasharray: '6 3',
                },
                animated: true,
                markerEnd: { type: MarkerType.ArrowClosed, color: '#f97316', width: 16, height: 16 },
            });
        }
    }
    return edges;
}

// ---------------------------------------------------------------------------
// Property Panel (sidebar for selected node)
// ---------------------------------------------------------------------------

function PropertyPanel({
    step,
    allStepIds,
    onUpdate,
    onDelete,
    readOnly,
}: {
    step: WorkflowStepDefinition;
    allStepIds: string[];
    onUpdate: (updates: Partial<WorkflowStepDefinition>) => void;
    onDelete: () => void;
    readOnly?: boolean;
}) {
    const otherIds = allStepIds.filter(id => id !== step.id);

    return (
        <div className="w-[520px] shrink-0 border-l border-border-primary bg-bg-surface overflow-y-auto p-4 space-y-3">
            <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold text-text-primary">Step Properties</h3>
                {!readOnly && (
                    <button onClick={onDelete} className="text-text-muted hover:text-red-400 p-1" title="Delete step">
                        <Trash2 className="w-3.5 h-3.5" />
                    </button>
                )}
            </div>

            {/* ID */}
            <div>
                <label className="block text-xs text-text-muted mb-1">ID</label>
                <input
                    value={step.id}
                    readOnly
                    className="w-full bg-bg-elevated border border-border-primary rounded px-2 py-1 text-xs text-text-secondary font-mono"
                />
            </div>

            {/* Name */}
            <div>
                <label className="block text-xs text-text-muted mb-1">Name</label>
                <input
                    value={step.name}
                    onChange={e => onUpdate({ name: e.target.value })}
                    readOnly={readOnly}
                    className={`w-full bg-bg-elevated border border-border-primary rounded px-2 py-1 text-xs text-text-primary ${readOnly ? 'cursor-default' : 'focus:outline-none focus:border-blue-500'}`}
                />
            </div>

            {/* Type */}
            <div>
                <label className="block text-xs text-text-muted mb-1">Type</label>
                <select
                    value={step.step_type}
                    onChange={e => onUpdate({ step_type: e.target.value as WorkflowStepType })}
                    disabled={readOnly}
                    className={`w-full bg-bg-elevated border border-border-primary rounded px-2 py-1 text-xs text-text-primary ${readOnly ? 'cursor-default opacity-80' : 'focus:outline-none focus:border-blue-500'}`}
                >
                    {STEP_TYPES.map(t => <option key={t.value} value={t.value}>{t.label}</option>)}
                </select>
            </div>

            {/* Depends On */}
            <div>
                <label className="block text-xs text-text-muted mb-1">Depends On</label>
                {step.depends_on.length > 0 ? (
                    <div className="flex flex-wrap gap-x-3 gap-y-1">
                        {readOnly ? (
                            step.depends_on.map(id => (
                                <span key={id} className="text-xs text-text-secondary font-mono bg-bg-elevated px-1.5 py-0.5 rounded">{id}</span>
                            ))
                        ) : (
                            otherIds.map(id => (
                                <label key={id} className="flex items-center gap-1 text-xs text-text-secondary cursor-pointer">
                                    <input
                                        type="checkbox"
                                        checked={step.depends_on.includes(id)}
                                        onChange={e => {
                                            const deps = e.target.checked
                                                ? [...step.depends_on, id]
                                                : step.depends_on.filter(d => d !== id);
                                            onUpdate({ depends_on: deps });
                                        }}
                                        className="rounded border-border-secondary"
                                    />
                                    <span className="font-mono">{id}</span>
                                </label>
                            ))
                        )}
                    </div>
                ) : (
                    <p className="text-xs text-text-muted">{readOnly ? 'None (entry step)' : 'No other steps to depend on'}</p>
                )}
                {!readOnly && step.depends_on.length === 0 && otherIds.length > 0 && (
                    <div className="flex flex-wrap gap-x-3 gap-y-1 mt-1">
                        {otherIds.map(id => (
                            <label key={id} className="flex items-center gap-1 text-xs text-text-secondary cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={false}
                                    onChange={() => onUpdate({ depends_on: [id] })}
                                    className="rounded border-border-secondary"
                                />
                                <span className="font-mono">{id}</span>
                            </label>
                        ))}
                    </div>
                )}
            </div>

            {/* Agentic fields */}
            {step.step_type === 'agentic' && (
                <>
                    <div>
                        <label className="block text-xs text-text-muted mb-1">Session Mode</label>
                        <select
                            value={step.session_mode ?? 'resume'}
                            onChange={e => onUpdate({ session_mode: e.target.value as SessionMode })}
                            disabled={readOnly}
                            className={`w-full bg-bg-elevated border border-border-primary rounded px-2 py-1 text-xs text-text-primary ${readOnly ? 'cursor-default opacity-80' : 'focus:outline-none focus:border-blue-500'}`}
                        >
                            {SESSION_MODES.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
                        </select>
                    </div>
                    <div>
                        <label className="flex items-center gap-2 text-xs text-text-muted cursor-pointer">
                            <input
                                type="checkbox"
                                checked={step.interactive ?? false}
                                onChange={e => onUpdate({ interactive: e.target.checked || undefined })}
                                disabled={readOnly}
                                className="rounded border-border-secondary"
                            />
                            Interactive
                            <span className="text-text-muted">— user can chat during this step</span>
                        </label>
                    </div>
                    <div>
                        <label className="block text-xs text-text-muted mb-1">
                            Prompt Template {!readOnly && <span className="text-red-400">*</span>}
                        </label>
                        <textarea
                            value={step.prompt_template ?? ''}
                            onChange={e => onUpdate({ prompt_template: e.target.value || undefined })}
                            readOnly={readOnly}
                            placeholder="{{task}}, {{step:id}}, {{rejection}}"
                            rows={12}
                            className={`w-full bg-bg-elevated border border-border-primary rounded px-2 py-1.5 text-xs text-text-primary placeholder-text-muted resize-y font-mono ${
                                readOnly ? 'cursor-default' : `focus:outline-none focus:border-blue-500 ${!step.prompt_template?.trim() ? 'border-red-600' : ''}`
                            }`}
                        />
                    </div>
                    {(step.loop_back_to || step.max_retries != null) && (
                        <div className="grid grid-cols-2 gap-2">
                            <div>
                                <label className="block text-xs text-text-muted mb-1">Loop Back To</label>
                                {readOnly ? (
                                    <p className="text-xs text-text-secondary font-mono">{step.loop_back_to ?? 'None'}</p>
                                ) : (
                                    <select
                                        value={step.loop_back_to ?? ''}
                                        onChange={e => onUpdate({ loop_back_to: e.target.value || undefined })}
                                        className="w-full bg-bg-elevated border border-border-secondary rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500"
                                    >
                                        <option value="">None</option>
                                        {otherIds.map(id => <option key={id} value={id}>{id}</option>)}
                                    </select>
                                )}
                            </div>
                            <div>
                                <label className="block text-xs text-text-muted mb-1">Max Retries</label>
                                {readOnly ? (
                                    <p className="text-xs text-text-secondary">{step.max_retries ?? '∞'}</p>
                                ) : (
                                    <input
                                        type="number" min="1"
                                        value={step.max_retries ?? ''}
                                        onChange={e => onUpdate({ max_retries: e.target.value ? parseInt(e.target.value, 10) : undefined })}
                                        placeholder="∞"
                                        className="w-full bg-bg-elevated border border-border-secondary rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500"
                                    />
                                )}
                            </div>
                        </div>
                    )}
                    {!readOnly && !step.loop_back_to && step.max_retries == null && (
                        <div className="grid grid-cols-2 gap-2">
                            <div>
                                <label className="block text-xs text-text-muted mb-1">Loop Back To</label>
                                <select
                                    value=""
                                    onChange={e => onUpdate({ loop_back_to: e.target.value || undefined })}
                                    className="w-full bg-bg-elevated border border-border-secondary rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500"
                                >
                                    <option value="">None</option>
                                    {otherIds.map(id => <option key={id} value={id}>{id}</option>)}
                                </select>
                            </div>
                            <div>
                                <label className="block text-xs text-text-muted mb-1">Max Retries</label>
                                <input
                                    type="number" min="1"
                                    value=""
                                    onChange={e => onUpdate({ max_retries: e.target.value ? parseInt(e.target.value, 10) : undefined })}
                                    placeholder="∞"
                                    className="w-full bg-bg-elevated border border-border-secondary rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500"
                                />
                            </div>
                        </div>
                    )}
                </>
            )}

            {/* Human gate fields */}
            {step.step_type === 'human_gate' && (
                <div className="grid grid-cols-2 gap-2">
                    <div>
                        <label className="block text-xs text-text-muted mb-1">
                            On Approve → {!readOnly && <span className="text-red-400">*</span>}
                        </label>
                        {readOnly ? (
                            <p className="text-xs text-green-400 font-mono">{step.on_approve ?? 'Not set'}</p>
                        ) : (
                            <select
                                value={step.on_approve ?? ''}
                                onChange={e => onUpdate({ on_approve: e.target.value || undefined })}
                                className={`w-full bg-bg-elevated border rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500 ${
                                    !step.on_approve ? 'border-red-600' : 'border-border-secondary'
                                }`}
                            >
                                <option value="">Select...</option>
                                {otherIds.map(id => <option key={id} value={id}>{id}</option>)}
                            </select>
                        )}
                    </div>
                    <div>
                        <label className="block text-xs text-text-muted mb-1">On Reject →</label>
                        {readOnly ? (
                            <p className="text-xs text-red-400 font-mono">{step.on_reject ?? 'None'}</p>
                        ) : (
                            <select
                                value={step.on_reject ?? ''}
                                onChange={e => onUpdate({ on_reject: e.target.value || undefined })}
                                className="w-full bg-bg-elevated border border-border-secondary rounded px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-blue-500"
                            >
                                <option value="">None</option>
                                {otherIds.map(id => <option key={id} value={id}>{id}</option>)}
                            </select>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}

// ---------------------------------------------------------------------------
// Edge Routing Context Menu
// ---------------------------------------------------------------------------

const SIDE_OPTIONS: { value: HandleSide; label: string }[] = [
    { value: 'top', label: 'Top' },
    { value: 'bottom', label: 'Bottom' },
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
];

function EdgeRoutingMenu({
    x,
    y,
    edgeId,
    category,
    currentRouting,
    onSetSide,
    onReset,
    onClose,
}: {
    x: number;
    y: number;
    edgeId: string;
    category: string;
    currentRouting: EdgeRouting;
    onSetSide: (edgeId: string, side: 'sourceSide' | 'targetSide', value: HandleSide) => void;
    onReset: (edgeId: string) => void;
    onClose: () => void;
}) {
    const menuRef = useRef<HTMLDivElement>(null);
    const defaults = HANDLE_DEFAULTS[category] ?? HANDLE_DEFAULTS.dependency;
    const currentSource = currentRouting.sourceSide ?? (defaults.sourceHandle.split('-')[0] as HandleSide);
    const currentTarget = currentRouting.targetSide ?? (defaults.targetHandle.split('-')[0] as HandleSide);

    // Clamp to viewport so menu doesn't render offscreen
    const clampedX = Math.min(x, window.innerWidth - 200);
    const clampedY = Math.min(y, window.innerHeight - 160);

    useEffect(() => {
        function handleClickOutside(e: MouseEvent) {
            if (menuRef.current && !menuRef.current.contains(e.target as HTMLElement)) onClose();
        }
        function handleEscape(e: KeyboardEvent) {
            if (e.key === 'Escape') onClose();
        }
        document.addEventListener('mousedown', handleClickOutside);
        document.addEventListener('keydown', handleEscape);
        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose]);

    return (
        <div
            ref={menuRef}
            className="fixed z-50 bg-bg-surface border border-border-primary rounded-lg shadow-xl p-3 min-w-[180px]"
            style={{ left: clampedX, top: clampedY }}
        >
            <div className="text-xs font-semibold text-text-primary mb-2">Edge Routing</div>

            <div className="mb-2">
                <div className="text-[10px] text-text-muted mb-1">Source side (exit)</div>
                <div className="flex gap-1">
                    {SIDE_OPTIONS.map(opt => (
                        <button
                            key={opt.value}
                            onClick={() => onSetSide(edgeId, 'sourceSide', opt.value)}
                            className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${
                                currentSource === opt.value
                                    ? 'bg-blue-600 border-blue-500 text-white'
                                    : 'bg-bg-elevated border-border-secondary text-text-secondary hover:bg-bg-interactive'
                            }`}
                        >
                            {opt.label}
                        </button>
                    ))}
                </div>
            </div>

            <div className="mb-2">
                <div className="text-[10px] text-text-muted mb-1">Target side (enter)</div>
                <div className="flex gap-1">
                    {SIDE_OPTIONS.map(opt => (
                        <button
                            key={opt.value}
                            onClick={() => onSetSide(edgeId, 'targetSide', opt.value)}
                            className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${
                                currentTarget === opt.value
                                    ? 'bg-blue-600 border-blue-500 text-white'
                                    : 'bg-bg-elevated border-border-secondary text-text-secondary hover:bg-bg-interactive'
                            }`}
                        >
                            {opt.label}
                        </button>
                    ))}
                </div>
            </div>

            <button
                onClick={() => onReset(edgeId)}
                className="w-full text-[10px] text-text-muted hover:text-text-secondary py-1 rounded hover:bg-bg-elevated transition-colors"
            >
                Reset to default
            </button>
        </div>
    );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

interface WorkflowEditPanelProps {
    workflow: Workflow;
    onClose: () => void;
}

export function WorkflowEditPanel({ workflow, onClose }: WorkflowEditPanelProps) {
    const [name, setName] = useState(workflow.name);
    const [steps, setSteps] = useState<WorkflowStepDefinition[]>(workflow.definition.steps);
    const [selectedStepId, setSelectedStepId] = useState<string | null>(null);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
    const [nodePositions, setNodePositions] = useState<Map<string, { x: number; y: number }>>(new Map());
    const [edgeRoutingOverrides, setEdgeRoutingOverrides] = useState<Map<string, EdgeRouting>>(new Map());

    const updateWorkflow = useUpdateWorkflow();
    const deleteWorkflow = useDeleteWorkflow();
    const stepCounterRef = useRef(workflow.definition.steps.length);

    const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
    const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

    const validationErrors = useMemo(() => validateDag(steps), [steps]);
    const hasErrors = validationErrors.length > 0;
    const stepIds = useMemo(() => steps.map(s => s.id), [steps]);
    const selectedStep = useMemo(() => steps.find(s => s.id === selectedStepId), [steps, selectedStepId]);

    // Sync steps → nodes/edges
    useEffect(() => {
        setNodes(layoutNodes(steps, nodePositions));
        setEdges(buildEdges(steps, edgeRoutingOverrides));
    }, [steps, nodePositions, edgeRoutingOverrides]);

    // Reset on workflow change
    useEffect(() => {
        setName(workflow.name);
        setSteps(workflow.definition.steps);
        setSelectedStepId(null);
        setShowDeleteConfirm(false);
        setNodePositions(new Map());
        setEdgeRoutingOverrides(new Map());
    }, [workflow.id]);

    // Track node positions from dragging
    const handleNodesChange: OnNodesChange = useCallback((changes) => {
        onNodesChange(changes);
        for (const change of changes) {
            if (change.type === 'position' && change.position) {
                const pos = change.position;
                setNodePositions(prev => {
                    const next = new Map(prev);
                    next.set(change.id, pos);
                    return next;
                });
            }
        }
    }, [onNodesChange]);

    const handleNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
        setSelectedStepId(node.id);
    }, []);

    // Edge context menu for route customization
    const [edgeContextMenu, setEdgeContextMenu] = useState<{
        edgeId: string;
        x: number;
        y: number;
        currentRouting: EdgeRouting;
        category: string;
    } | null>(null);

    const handlePaneClick = useCallback(() => {
        setSelectedStepId(null);
        setEdgeContextMenu(null);
    }, []);

    const handleEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
        event.preventDefault();
        const routing = edgeRoutingOverrides.get(edge.id) ?? {};
        const category = (edge.data as { edgeCategory?: string })?.edgeCategory ?? 'dependency';
        setEdgeContextMenu({
            edgeId: edge.id,
            x: event.clientX,
            y: event.clientY,
            currentRouting: routing,
            category,
        });
    }, [edgeRoutingOverrides]);

    const updateEdgeRouting = useCallback((edgeId: string, side: 'sourceSide' | 'targetSide', value: HandleSide) => {
        setEdgeRoutingOverrides(prev => {
            const next = new Map(prev);
            const existing = next.get(edgeId) ?? {};
            next.set(edgeId, { ...existing, [side]: value });
            return next;
        });
        // Keep menu open and update its displayed routing state
        setEdgeContextMenu(prev => prev ? { ...prev, currentRouting: { ...prev.currentRouting, [side]: value } } : null);
    }, []);

    const resetEdgeRouting = useCallback((edgeId: string) => {
        setEdgeRoutingOverrides(prev => {
            const next = new Map(prev);
            next.delete(edgeId);
            return next;
        });
        setEdgeContextMenu(null);
    }, []);

    function updateStep(id: string, updates: Partial<WorkflowStepDefinition>) {
        setSteps(prev => prev.map(s => s.id === id ? { ...s, ...updates } : s));
    }

    function addStep() {
        stepCounterRef.current += 1;
        const newId = `step_${stepCounterRef.current}`;
        const newStep: WorkflowStepDefinition = {
            id: newId,
            step_type: 'agentic',
            name: '',
            depends_on: [],
            session_mode: 'resume',
        };
        setSteps(prev => [...prev, newStep]);
        setSelectedStepId(newId);
    }

    function removeStep(id: string) {
        setSteps(prev => prev
            .filter(s => s.id !== id)
            .map(s => ({
                ...s,
                depends_on: s.depends_on.filter(d => d !== id),
                on_approve: s.on_approve === id ? undefined : s.on_approve,
                on_reject: s.on_reject === id ? undefined : s.on_reject,
                loop_back_to: s.loop_back_to === id ? undefined : s.loop_back_to,
            }))
        );
        setSelectedStepId(null);
        setNodePositions(prev => { const next = new Map(prev); next.delete(id); return next; });
    }

    function handleSave() {
        if (hasErrors) return;
        updateWorkflow.mutate({ id: workflow.id, name: name.trim() || undefined, definition: { steps } }, { onSuccess: onClose });
    }

    function handleDelete() {
        deleteWorkflow.mutate(workflow.id, { onSuccess: onClose });
    }

    const cloneWorkflow = useCloneWorkflow();

    // Template read-only view
    if (workflow.is_template) {
        const templateSelectedStep = steps.find(s => s.id === selectedStepId);
        return (
            <div className="h-full flex flex-col bg-bg-app">
                <div className="flex items-center justify-between px-4 py-3 border-b border-border-primary bg-bg-surface">
                    <div className="flex items-center gap-2">
                        <Lock className="w-4 h-4 text-purple-400" />
                        <h2 className="text-sm font-semibold text-text-primary">{workflow.name}</h2>
                        <span className="text-xs px-1.5 py-0.5 rounded bg-purple-900/40 text-purple-300 border border-purple-800">Template</span>
                    </div>
                    <div className="flex items-center gap-2">
                        <button
                            onClick={() => cloneWorkflow.mutate(workflow.id)}
                            disabled={cloneWorkflow.isPending}
                            className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-blue-600 text-white rounded hover:bg-blue-500 transition-colors disabled:opacity-50"
                        >
                            <Copy className="w-3 h-3" />
                            {cloneWorkflow.isPending ? 'Cloning...' : 'Clone to edit'}
                        </button>
                        <button onClick={onClose} className="text-text-muted hover:text-text-primary p-1 rounded hover:bg-bg-elevated">
                            <X className="w-4 h-4" />
                        </button>
                    </div>
                </div>
                <div className="flex-1 flex min-h-0">
                    <div className="flex-1 min-w-0">
                        <ReactFlow
                            nodes={layoutNodes(steps)}
                            edges={buildEdges(steps, new Map())}
                            nodeTypes={nodeTypes}
                            edgeTypes={edgeTypes}
                            onNodeClick={handleNodeClick}
                            onPaneClick={handlePaneClick}
                            fitView
                            nodesDraggable={false}
                            nodesConnectable={false}
                            proOptions={{ hideAttribution: true }}
                            deleteKeyCode={null}
                        >
                            <Background color="#374151" gap={20} variant={BackgroundVariant.Dots} />
                            <Controls showInteractive={false} />
                        </ReactFlow>
                    </div>
                    {templateSelectedStep && (
                        <PropertyPanel
                            step={templateSelectedStep}
                            allStepIds={stepIds}
                            onUpdate={() => {}}
                            onDelete={() => {}}
                            readOnly
                        />
                    )}
                </div>
            </div>
        );
    }

    return (
        <div className="h-full flex flex-col bg-bg-app">
            {/* Top bar */}
            <div className="flex items-center gap-3 px-4 py-2 border-b border-border-primary bg-bg-surface shrink-0">
                <label className="text-xs text-text-muted shrink-0">Name</label>
                <input
                    value={name}
                    onChange={e => setName(e.target.value)}
                    className="flex-1 bg-bg-elevated border border-border-primary rounded px-2 py-1 text-sm text-text-primary focus:outline-none focus:border-blue-500 min-w-0"
                />
                <button
                    onClick={addStep}
                    className="flex items-center gap-1 px-2.5 py-1 text-xs bg-bg-elevated text-text-secondary rounded hover:bg-bg-interactive border border-border-primary shrink-0"
                >
                    <Plus className="w-3 h-3" />
                    Add Step
                </button>

                {/* Validation errors indicator */}
                {hasErrors && (
                    <div className="flex items-center gap-1 text-red-400 shrink-0" title={validationErrors.join('\n')}>
                        <AlertCircle className="w-3.5 h-3.5" />
                        <span className="text-xs">{validationErrors.length} error{validationErrors.length > 1 ? 's' : ''}</span>
                    </div>
                )}

                <div className="flex items-center gap-2 ml-auto shrink-0">
                    {!showDeleteConfirm ? (
                        <button onClick={() => setShowDeleteConfirm(true)}
                            className="px-2.5 py-1 text-xs text-red-400 hover:bg-red-900/30 rounded transition-colors">
                            <Trash2 className="w-3.5 h-3.5" />
                        </button>
                    ) : (
                        <>
                            <span className="text-xs text-red-400">Delete?</span>
                            <button onClick={handleDelete} disabled={deleteWorkflow.isPending}
                                className="px-2 py-0.5 text-xs text-white bg-red-600 rounded hover:bg-red-500 disabled:opacity-50">Yes</button>
                            <button onClick={() => setShowDeleteConfirm(false)}
                                className="px-2 py-0.5 text-xs text-text-secondary bg-bg-elevated rounded hover:bg-bg-interactive">No</button>
                        </>
                    )}
                    <button onClick={onClose}
                        className="px-2.5 py-1 text-xs text-text-secondary bg-bg-elevated border border-border-secondary rounded hover:bg-bg-interactive">
                        Cancel
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={!name.trim() || steps.length === 0 || hasErrors || updateWorkflow.isPending}
                        className="px-3 py-1 text-xs text-white bg-blue-600 rounded hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {updateWorkflow.isPending ? 'Saving...' : 'Save'}
                    </button>
                </div>
            </div>

            {/* Validation errors bar */}
            {hasErrors && (
                <div className="px-4 py-2 bg-red-900/20 border-b border-red-800 flex flex-wrap gap-x-4 gap-y-1">
                    {validationErrors.map((err, i) => (
                        <span key={i} className="flex items-center gap-1 text-xs text-red-300">
                            <AlertCircle className="w-3 h-3 shrink-0" />{err}
                        </span>
                    ))}
                </div>
            )}

            {/* Main area: graph + property panel */}
            <div className="flex-1 flex min-h-0">
                {/* Graph canvas */}
                <div className="flex-1 min-w-0">
                    <ReactFlow
                        nodes={nodes}
                        edges={edges}
                        onNodesChange={handleNodesChange}
                        onEdgesChange={onEdgesChange}
                        onNodeClick={handleNodeClick}
                        onPaneClick={handlePaneClick}
                        onEdgeContextMenu={handleEdgeContextMenu}
                        nodeTypes={nodeTypes}
                        edgeTypes={edgeTypes}
                        fitView
                        proOptions={{ hideAttribution: true }}
                        deleteKeyCode={null}
                    >
                        <Background color="#374151" gap={20} variant={BackgroundVariant.Dots} />
                        <Controls />
                    </ReactFlow>
                </div>

                {/* Property panel */}
                {selectedStep && (
                    <PropertyPanel
                        step={selectedStep}
                        allStepIds={stepIds}
                        onUpdate={(updates) => updateStep(selectedStep.id, updates)}
                        onDelete={() => removeStep(selectedStep.id)}
                    />
                )}
            </div>

            {/* Edge routing context menu */}
            {edgeContextMenu && (
                <EdgeRoutingMenu
                    x={edgeContextMenu.x}
                    y={edgeContextMenu.y}
                    edgeId={edgeContextMenu.edgeId}
                    category={edgeContextMenu.category}
                    currentRouting={edgeContextMenu.currentRouting}
                    onSetSide={updateEdgeRouting}
                    onReset={resetEdgeRouting}
                    onClose={() => setEdgeContextMenu(null)}
                />
            )}

            {/* Edge legend */}
            <div className="flex items-center gap-4 px-4 py-1.5 border-t border-border-primary bg-bg-surface text-[10px] text-text-muted">
                <span className="flex items-center gap-1"><span className="w-4 h-0.5 bg-gray-500 inline-block" /> dependency</span>
                <span className="flex items-center gap-1"><span className="w-4 h-0.5 bg-green-500 inline-block" /> approve</span>
                <span className="flex items-center gap-1"><span className="w-4 h-0.5 bg-red-500 inline-block" /> reject</span>
                <span className="flex items-center gap-1"><span className="w-4 h-0.5 bg-orange-500 inline-block border-dashed" style={{ borderTop: '2px dashed #f97316', height: 0 }} /> loop</span>
                <span className="text-text-muted/50 ml-2">Right-click edge to change routing</span>
            </div>
        </div>
    );
}
