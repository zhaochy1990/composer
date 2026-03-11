import {
    BaseEdge,
    EdgeLabelRenderer,
    getSmoothStepPath,
    type EdgeProps,
    type Edge,
} from '@xyflow/react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type HandleSide = 'top' | 'bottom' | 'left' | 'right';

export interface EdgeRouting {
    sourceSide?: HandleSide;
    targetSide?: HandleSide;
}

export interface RoutableEdgeData {
    [key: string]: unknown;
    edgeCategory: 'dependency' | 'approve' | 'reject' | 'loop';
    label?: string;
    color: string;
    strokeDasharray?: string;
}

// ---------------------------------------------------------------------------
// Custom Edge Component
// ---------------------------------------------------------------------------

export function RoutableEdge({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    data,
    markerEnd,
    style,
}: EdgeProps<Edge<RoutableEdgeData>>) {
    const [edgePath, labelX, labelY] = getSmoothStepPath({
        sourceX,
        sourceY,
        sourcePosition,
        targetX,
        targetY,
        targetPosition,
        borderRadius: 8,
        offset: 30,
    });

    const color = data?.color ?? '#6b7280';

    return (
        <>
            <BaseEdge
                path={edgePath}
                markerEnd={markerEnd}
                style={{
                    ...style,
                    stroke: color,
                    strokeWidth: 2,
                    strokeDasharray: data?.strokeDasharray,
                }}
            />
            {data?.label && (
                <EdgeLabelRenderer>
                    <div
                        style={{
                            position: 'absolute',
                            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
                            pointerEvents: 'none',
                        }}
                        className="nodrag nopan"
                    >
                        <span
                            className="text-[10px] font-semibold px-1.5 py-0.5 rounded"
                            style={{
                                color,
                                backgroundColor: 'rgba(17, 24, 39, 0.9)',
                            }}
                        >
                            {data.label}
                        </span>
                    </div>
                </EdgeLabelRenderer>
            )}
        </>
    );
}
