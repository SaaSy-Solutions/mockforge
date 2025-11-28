import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { visualizer } from 'rollup-plugin-visualizer'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    visualizer({
      filename: './dist/stats.html',
      open: false,
      gzipSize: true,
      brotliSize: true,
      template: 'treemap', // or 'sunburst', 'network'
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
    dedupe: ['react', 'react-dom'], // Ensure React is not duplicated
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
    minify: false, // Disable minification to see actual React errors
    sourcemap: true, // Enable source maps to help debug React errors
    rollupOptions: {
      output: {
        entryFileNames: (chunkInfo) => {
          // Don't hash the main index file for easier embedding
          return chunkInfo.name === 'index' ? 'assets/index.js' : 'assets/[name].[hash].js';
        },
        chunkFileNames: `assets/[name].[hash].js`,
        assetFileNames: (assetInfo) => {
          // Don't hash the main CSS file for easier embedding
          if (assetInfo.name === 'index.css') {
            return 'assets/index.css';
          }
          return 'assets/[name].[hash].[ext]';
        },
        manualChunks: (id) => {
          // Core React libraries - MUST be in the same chunk to avoid duplicate React instances
          // Include MUI, Emotion, Zustand, and React Query with React since they have tight coupling
          if (id.includes('node_modules/react/') || 
              id.includes('node_modules/react-dom/') || 
              id.includes('node_modules/react-router-dom') ||
              id.includes('node_modules/@mui') || 
              id.includes('node_modules/@emotion') ||
              id.includes('node_modules/zustand') ||
              id.includes('node_modules/@tanstack/react-query')) {
            // Exclude devtools from main vendor chunk
            if (id.includes('devtools')) {
              return 'query-devtools';
            }
            return 'react-vendor';
          }
          // Radix UI components
          if (id.includes('node_modules/@radix-ui')) {
            return 'ui-vendor';
          }
          // Chart.js library
          if (id.includes('node_modules/chart.js') || id.includes('node_modules/react-chartjs-2')) {
            return 'chart-vendor';
          }
        }
      }
    }
  },
})
