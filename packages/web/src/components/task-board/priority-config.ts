export const priorityConfig: Record<number, { label: string; className: string }> = {
    3: { label: 'High', className: 'bg-red-100 text-red-800 border-red-300 dark:bg-red-900/60 dark:text-red-300 dark:border-red-700' },
    2: { label: 'Medium', className: 'bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/60 dark:text-yellow-300 dark:border-yellow-700' },
    1: { label: 'Low', className: 'bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/60 dark:text-blue-300 dark:border-blue-700' },
    0: { label: 'None', className: 'bg-bg-elevated text-text-muted border-border-secondary' },
};
