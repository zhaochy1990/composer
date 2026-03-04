/// <reference types="vitest" />
import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// Plugin: resolve bare module imports from test files outside packages/web/
// by falling back to this package's node_modules.
function resolveExternalTestDeps(): Plugin {
    const packageDir = __dirname;
    return {
        name: 'resolve-external-test-deps',
        enforce: 'pre',
        async resolveId(source, importer, options) {
            if (
                !importer ||
                !source ||
                source.startsWith('.') ||
                source.startsWith('/') ||
                source.startsWith('\0') ||
                source.startsWith('@/')
            ) {
                return null;
            }
            // Only for files outside this package directory
            const resolvedImporter = path.resolve(importer);
            if (resolvedImporter.startsWith(path.resolve(packageDir))) {
                return null;
            }
            // Re-resolve the import as if it came from within this package
            const result = await this.resolve(source, path.join(packageDir, '_resolver_.js'), {
                ...options,
                skipSelf: true,
            });
            return result;
        },
    };
}

export default defineConfig({
    plugins: [
        react(),
        resolveExternalTestDeps(),
    ],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
    server: {
        port: 5173,
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:3000',
                ws: true,
            },
        },
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
    },
    test: {
        globals: true,
        environment: 'jsdom',
        setupFiles: ['../../tests/web/setup.ts'],
        include: ['../../tests/web/**/*.test.{ts,tsx}'],
        css: false,
    },
});
