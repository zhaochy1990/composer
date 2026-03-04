import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, FolderOpen, ArrowUp, Loader2 } from 'lucide-react';
import { apiFetch } from '@/lib/api';
import type { BrowseResponse } from '@/types/generated';

interface DirectoryBrowserDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSelect: (path: string) => void;
    initialPath?: string;
}

export function DirectoryBrowserDialog({ isOpen, onClose, onSelect, initialPath }: DirectoryBrowserDialogProps) {
    const [browseData, setBrowseData] = useState<BrowseResponse | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function fetchDirectory(path?: string) {
        setLoading(true);
        setError(null);
        try {
            const query = path ? `?path=${encodeURIComponent(path)}` : '';
            const data = await apiFetch<BrowseResponse>(`/filesystem/browse${query}`);
            setBrowseData(data);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to browse directory');
        } finally {
            setLoading(false);
        }
    }

    useEffect(() => {
        if (isOpen) {
            fetchDirectory(initialPath || undefined);
        }
    }, [isOpen]);

    function handleSelect() {
        if (browseData) {
            onSelect(browseData.current_path);
            onClose();
        }
    }

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] w-full max-w-[560px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-gray-100">
                            Browse Directory
                        </Dialog.Title>
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded hover:bg-gray-800"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </Dialog.Close>
                    </div>

                    {/* Current path */}
                    {browseData && (
                        <div className="flex items-center gap-2 mb-3 px-3 py-2 bg-gray-800 border border-gray-700 rounded-md">
                            <FolderOpen className="w-4 h-4 text-blue-400 shrink-0" />
                            <span className="text-sm text-gray-200 font-mono truncate">
                                {browseData.current_path}
                            </span>
                        </div>
                    )}

                    {/* Navigate up */}
                    {browseData?.parent && (
                        <button
                            type="button"
                            onClick={() => fetchDirectory(browseData.parent!)}
                            disabled={loading}
                            className="flex items-center gap-2 w-full px-3 py-2 mb-1 text-sm text-gray-300 hover:bg-gray-800 rounded-md transition-colors disabled:opacity-50"
                        >
                            <ArrowUp className="w-4 h-4" />
                            ..
                        </button>
                    )}

                    {/* Directory listing */}
                    <div className="h-[300px] overflow-y-auto border border-gray-700 rounded-md bg-gray-800/50">
                        {loading && (
                            <div className="flex items-center justify-center h-full">
                                <Loader2 className="w-5 h-5 text-gray-400 animate-spin" />
                            </div>
                        )}
                        {error && (
                            <div className="flex items-center justify-center h-full p-4">
                                <span className="text-sm text-red-400">{error}</span>
                            </div>
                        )}
                        {!loading && !error && browseData && browseData.entries.length === 0 && (
                            <div className="flex items-center justify-center h-full">
                                <span className="text-sm text-gray-500">No subdirectories</span>
                            </div>
                        )}
                        {!loading && !error && browseData?.entries.map((entry) => (
                            <button
                                key={entry.path}
                                type="button"
                                onClick={() => fetchDirectory(entry.path)}
                                className="flex items-center gap-2 w-full px-3 py-2 text-sm text-gray-200 hover:bg-gray-700 transition-colors text-left"
                            >
                                <FolderOpen className="w-4 h-4 text-yellow-500 shrink-0" />
                                <span className="truncate">{entry.name}</span>
                            </button>
                        ))}
                    </div>

                    {/* Actions */}
                    <div className="flex justify-end gap-2 pt-4">
                        <Dialog.Close asChild>
                            <button
                                type="button"
                                className="px-4 py-2 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 transition-colors"
                            >
                                Cancel
                            </button>
                        </Dialog.Close>
                        <button
                            type="button"
                            onClick={handleSelect}
                            disabled={!browseData}
                            className="flex items-center gap-1.5 px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            Select This Directory
                        </button>
                    </div>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
