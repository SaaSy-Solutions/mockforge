/**
 * Vite Configuration for Coverage Collection
 * 
 * This configuration extends the base vite.config.ts to enable code instrumentation
 * for coverage collection during Playwright E2E tests.
 */

import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import istanbul from 'vite-plugin-istanbul';
import { visualizer } from 'rollup-plugin-visualizer';
import path from 'path';

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    // Instrument code for coverage collection
    istanbul({
      include: 'src/**/*.{ts,tsx}',
      exclude: [
        '**/*.test.{ts,tsx}',
        '**/*.spec.{ts,tsx}',
        '**/__tests__/**',
        '**/node_modules/**',
        '**/dist/**',
        '**/coverage/**',
      ],
      extension: ['.ts', '.tsx'],
      requireEnv: false,
      checkProd: false,
    }),
    visualizer({
      filename: './dist/stats.html',
      open: false,
      gzipSize: true,
      brotliSize: true,
      template: 'treemap',
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    proxy: {
      '/__mockforge': {
        target: `http://localhost:${process.env.ADMIN_PORT || '9080'}`,
        changeOrigin: true,
        secure: false,
      },
      '/api-docs': {
        target: `http://localhost:${process.env.ADMIN_PORT || '9080'}`,
        changeOrigin: true,
        secure: false,
      }
    }
  },
  build: {
    manifest: true,
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      output: {
        entryFileNames: (chunkInfo) => {
          return chunkInfo.name === 'index' ? 'assets/index.js' : 'assets/[name].[hash].js';
        },
        chunkFileNames: `assets/[name].[hash].js`,
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'index.css') {
            return 'assets/index.css';
          }
          return 'assets/[name].[hash].[ext]';
        },
        manualChunks: (id) => {
          if (id.includes('node_modules/react') || id.includes('node_modules/react-dom') || id.includes('node_modules/react-router-dom')) {
            return 'react-vendor';
          }
          if (id.includes('node_modules/@radix-ui')) {
            return 'ui-vendor';
          }
          if (id.includes('node_modules/@tanstack/react-query')) {
            if (id.includes('devtools')) {
              return 'query-devtools';
            }
            return 'query-vendor';
          }
          if (id.includes('node_modules/chart.js') || id.includes('node_modules/react-chartjs-2')) {
            return 'chart-vendor';
          }
          if (id.includes('node_modules/zustand')) {
            return 'state-vendor';
          }
          if (id.includes('node_modules/@mui') || id.includes('node_modules/@emotion')) {
            return 'mui-vendor';
          }
        }
      }
    }
  },
});

