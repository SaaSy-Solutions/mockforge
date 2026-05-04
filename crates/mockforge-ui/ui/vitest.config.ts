/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    exclude: [
      'node_modules',
      'dist',
      '.git',
      '.cache',
      '**/*.e2e.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.d.ts',
        '**/*.config.{js,ts}',
        '**/types/',
        '**/__tests__/',
        '**/dist/',
      ],
      thresholds: {
        global: {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80,
        },
      },
    },
    pool: 'threads',
    poolOptions: {
      threads: {
        singleThread: false,
        minThreads: 1,
        maxThreads: 4,
      },
    },
    testTimeout: 10000,
    hookTimeout: 10000,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      // Mirror the production aliases from vite.config.ts so vitest can
      // resolve the legacy `@mui/material` and `@mui/icons-material`
      // imports against the in-tree shims after the MUI dep was dropped.
      '@mui/material': path.resolve(__dirname, './src/components/ui/mui-shim.tsx'),
      '@mui/icons-material': path.resolve(__dirname, './src/components/ui/mui-icons-shim.tsx'),
    },
  },
});
