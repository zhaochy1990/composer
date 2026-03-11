import { useCallback, useRef, useEffect, useState } from 'react';
import {
    usePanelSizeStore,
    PANEL_MIN_WIDTH,
    PANEL_MAX_WIDTH_RATIO,
    PANEL_DEFAULT_WIDTH,
} from '@/stores/panel-size-store';

interface UseResizablePanelReturn {
    width: number;
    isDragging: boolean;
    handleMouseDown: (e: React.MouseEvent) => void;
    handleDoubleClick: () => void;
}

export function useResizablePanel(): UseResizablePanelReturn {
    const width = usePanelSizeStore((s) => s.detailPanelWidth);
    const setWidth = usePanelSizeStore((s) => s.setDetailPanelWidth);
    const isDraggingRef = useRef(false);
    const startXRef = useRef(0);
    const startWidthRef = useRef(0);
    const [isDragging, setIsDragging] = useState(false);

    const handleMouseDown = useCallback(
        (e: React.MouseEvent) => {
            e.preventDefault();
            isDraggingRef.current = true;
            setIsDragging(true);
            startXRef.current = e.clientX;
            startWidthRef.current = width;
        },
        [width],
    );

    const handleDoubleClick = useCallback(() => {
        setWidth(PANEL_DEFAULT_WIDTH);
    }, [setWidth]);

    useEffect(() => {
        function handleMouseMove(e: MouseEvent) {
            if (!isDraggingRef.current) return;
            // Dragging left increases width (handle is on the left edge of a right-anchored panel)
            const delta = startXRef.current - e.clientX;
            const newWidth = startWidthRef.current + delta;
            const maxWidth = window.innerWidth * PANEL_MAX_WIDTH_RATIO;
            const clamped = Math.max(PANEL_MIN_WIDTH, Math.min(newWidth, maxWidth));
            setWidth(clamped);
        }

        function handleMouseUp() {
            if (isDraggingRef.current) {
                isDraggingRef.current = false;
                setIsDragging(false);
            }
        }

        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);
        return () => {
            document.removeEventListener('mousemove', handleMouseMove);
            document.removeEventListener('mouseup', handleMouseUp);
        };
    }, [setWidth]);

    // Clamp width when window shrinks
    useEffect(() => {
        function handleWindowResize() {
            const maxWidth = window.innerWidth * PANEL_MAX_WIDTH_RATIO;
            if (width > maxWidth) {
                setWidth(maxWidth);
            }
        }
        window.addEventListener('resize', handleWindowResize);
        return () => window.removeEventListener('resize', handleWindowResize);
    }, [width, setWidth]);

    return { width, isDragging, handleMouseDown, handleDoubleClick };
}
