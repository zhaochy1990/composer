import { FileText } from 'lucide-react';
import { MarkdownContent } from '@/components/sessions/MarkdownContent';

interface PlanReviewPanelProps {
    content: string;
}

export function PlanReviewPanel({ content }: PlanReviewPanelProps) {
    return (
        <div className="fixed inset-y-0 right-[900px] w-[550px] max-w-[calc(100vw-900px)] z-50 bg-gray-900 border-l border-gray-700 shadow-2xl flex-col overflow-hidden hidden min-[1450px]:flex">
            <div className="flex items-center gap-2 px-6 py-3 border-b border-gray-800 shrink-0">
                <FileText className="w-4 h-4 text-blue-400" />
                <h3 className="text-sm font-semibold text-gray-200">Design Plan</h3>
            </div>
            <div className="flex-1 overflow-y-auto px-6 py-4">
                <div className="prose prose-invert prose-sm max-w-none leading-relaxed">
                    <MarkdownContent content={content} />
                </div>
            </div>
        </div>
    );
}
