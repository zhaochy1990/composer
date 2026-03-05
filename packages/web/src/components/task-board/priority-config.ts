export const priorityConfig: Record<number, { label: string; className: string }> = {
    3: { label: 'High', className: 'bg-red-900/60 text-red-300 border-red-700' },
    2: { label: 'Medium', className: 'bg-yellow-900/60 text-yellow-300 border-yellow-700' },
    1: { label: 'Low', className: 'bg-blue-900/60 text-blue-300 border-blue-700' },
    0: { label: 'None', className: 'bg-gray-800 text-gray-400 border-gray-600' },
};
