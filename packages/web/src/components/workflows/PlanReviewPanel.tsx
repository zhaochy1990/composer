import { useEffect } from 'react';
import { FileText, X } from 'lucide-react';
import { MarkdownContent } from '@/components/sessions/MarkdownContent';

interface PlanReviewPanelProps {
    content: string;
    onClose: () => void;
}

export function PlanReviewPanel({ content, onClose }: PlanReviewPanelProps) {
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onClose();
        };
        document.addEventListener('keydown', handler);
        return () => document.removeEventListener('keydown', handler);
    }, [onClose]);

    return (
        <div
            className="fixed inset-0 z-[60] flex items-start justify-center bg-black/30"
            onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}
        >
            <div role="dialog" aria-modal="true" aria-label="Design Plan" className="w-full max-w-4xl my-8 mx-4 max-h-[calc(100vh-4rem)] bg-gray-900 border border-gray-700 rounded-lg shadow-2xl flex flex-col overflow-hidden">
                <div className="flex items-center justify-between px-10 py-3 border-b border-gray-800 shrink-0">
                    <div className="flex items-center gap-2">
                        <FileText className="w-4 h-4 text-blue-400" />
                        <h3 className="text-sm font-semibold text-gray-200">Design Plan</h3>
                    </div>
                    <button
                        type="button"
                        onClick={onClose}
                        aria-label="Close design plan"
                        className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                    >
                        <X className="w-4 h-4" />
                    </button>
                </div>
                <div className="flex-1 overflow-y-auto px-10 py-6">
                    <div className="prose prose-invert prose-base max-w-none leading-relaxed">
                        <MarkdownContent content={content} />
                    </div>
                </div>
            </div>
        </div>
    );
}
