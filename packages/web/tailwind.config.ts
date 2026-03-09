import type { Config } from 'tailwindcss';

const config: Config = {
    darkMode: 'class',
    content: ['./index.html', './src/**/*.{ts,tsx}'],
    theme: {
        extend: {
            colors: {
                'bg-app': 'rgb(var(--color-bg-app) / <alpha-value>)',
                'bg-surface': 'rgb(var(--color-bg-surface) / <alpha-value>)',
                'bg-elevated': 'rgb(var(--color-bg-elevated) / <alpha-value>)',
                'bg-interactive': 'rgb(var(--color-bg-interactive) / <alpha-value>)',
                'text-primary': 'rgb(var(--color-text-primary) / <alpha-value>)',
                'text-secondary': 'rgb(var(--color-text-secondary) / <alpha-value>)',
                'text-muted': 'rgb(var(--color-text-muted) / <alpha-value>)',
                'border-primary': 'rgb(var(--color-border-primary) / <alpha-value>)',
                'border-secondary': 'rgb(var(--color-border-secondary) / <alpha-value>)',
                ring: 'rgb(var(--color-ring) / <alpha-value>)',
            },
        },
    },
    plugins: [],
};

export default config;
