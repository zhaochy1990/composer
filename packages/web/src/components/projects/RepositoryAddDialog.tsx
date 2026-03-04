import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Plus } from 'lucide-react';
import { useAddProjectRepository } from '@/hooks/use-projects';
import type { RepositoryRole } from '@/types/generated';

interface RepositoryAddDialogProps {
    isOpen: boolean;
    onClose: () => void;
    projectId: string;
}

export function RepositoryAddDialog({ isOpen, onClose, projectId }: RepositoryAddDialogProps) {
    const [localPath, setLocalPath] = useState('');
    const [remoteUrl, setRemoteUrl] = useState('');
    const [role, setRole] = useState<RepositoryRole>('primary');
    const [displayName, setDisplayName] = useState('');
    const addRepo = useAddProjectRepository();

    function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!localPath.trim()) return;

        addRepo.mutate(
            {
                projectId,
                local_path: localPath.trim(),
                remote_url: remoteUrl.trim() || undefined,
                role,
                display_name: displayName.trim() || undefined,
            },
            {
                onSuccess: () => {
                    setLocalPath('');
                    setRemoteUrl('');
                    setRole('primary');
                    setDisplayName('');
                    onClose();
                },
            },
        );
    }

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => { if (!open) { setLocalPath(''); setRemoteUrl(''); setRole('primary'); setDisplayName(''); onClose(); } }}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40" />
                <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-full max-w-[480px] bg-gray-900 border border-gray-700 rounded-xl shadow-2xl p-6">
                    <div className="flex items-center justify-between mb-4">
                        <Dialog.Title className="text-lg font-semibold text-gray-100">
                            Add Repository
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

                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div>
                            <label htmlFor="repo-local-path" className="block text-sm font-medium text-gray-300 mb-1">
                                Local Path <span className="text-red-400">*</span>
                            </label>
                            <input
                                id="repo-local-path"
                                type="text"
                                value={localPath}
                                onChange={e => setLocalPath(e.target.value)}
                                placeholder="/absolute/path/to/repo"
                                required
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 font-mono"
                            />
                        </div>

                        <div>
                            <label htmlFor="repo-remote-url" className="block text-sm font-medium text-gray-300 mb-1">
                                Remote URL
                            </label>
                            <input
                                id="repo-remote-url"
                                type="text"
                                value={remoteUrl}
                                onChange={e => setRemoteUrl(e.target.value)}
                                placeholder="https://github.com/org/repo"
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>

                        <div>
                            <label htmlFor="repo-role" className="block text-sm font-medium text-gray-300 mb-1">
                                Role
                            </label>
                            <select
                                id="repo-role"
                                value={role}
                                onChange={e => setRole(e.target.value as RepositoryRole)}
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            >
                                <option value="primary">Primary</option>
                                <option value="dependency">Dependency</option>
                            </select>
                        </div>

                        <div>
                            <label htmlFor="repo-display-name" className="block text-sm font-medium text-gray-300 mb-1">
                                Display Name
                            </label>
                            <input
                                id="repo-display-name"
                                type="text"
                                value={displayName}
                                onChange={e => setDisplayName(e.target.value)}
                                placeholder="Optional display name"
                                className="w-full bg-gray-800 border border-gray-600 rounded-md px-3 py-2 text-sm text-gray-100 placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                            />
                        </div>

                        <div className="flex justify-end gap-2 pt-2">
                            <Dialog.Close asChild>
                                <button
                                    type="button"
                                    className="px-4 py-2 text-sm text-gray-300 bg-gray-800 border border-gray-600 rounded-md hover:bg-gray-700 transition-colors"
                                >
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                type="submit"
                                disabled={!localPath.trim() || addRepo.isPending}
                                className="flex items-center gap-1.5 px-4 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                {addRepo.isPending ? 'Adding...' : 'Add Repository'}
                            </button>
                        </div>
                    </form>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
