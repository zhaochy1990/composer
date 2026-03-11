import { cn } from '@/lib/utils';

interface ResizeHandleProps {
    onMouseDown: (e: React.MouseEvent) => void;
    onDoubleClick: () => void;
    isDragging: boolean;
}

export function ResizeHandle({ onMouseDown, onDoubleClick, isDragging }: ResizeHandleProps) {
    return (
        <>
            {/* Full-screen overlay during drag to prevent cursor flicker */}
            {isDragging && (
                <div className="fixed inset-0 z-[9999] cursor-col-resize" />
            )}
            <div
                onMouseDown={onMouseDown}
                onDoubleClick={onDoubleClick}
                className={cn(
                    'w-1.5 shrink-0 cursor-col-resize relative group',
                    'hover:bg-blue-500/20 transition-colors',
                    isDragging && 'bg-blue-500/30',
                )}
            >
                <div
                    className={cn(
                        'absolute inset-y-0 left-1/2 -translate-x-1/2 w-px',
                        'bg-border-primary group-hover:bg-blue-500 transition-colors',
                        isDragging && 'bg-blue-500',
                    )}
                />
            </div>
        </>
    );
}
