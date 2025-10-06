import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
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
        entryFileNames: `assets/[name].[hash].js`,
        chunkFileNames: `assets/[name].[hash].js`,
        assetFileNames: `assets/[name].[hash].[ext]`,
        manualChunks: (id) => {
          // Core React libraries
          if (id.includes('node_modules/react') || id.includes('node_modules/react-dom') || id.includes('node_modules/react-router-dom')) {
            return 'react-vendor';
          }
          // Radix UI components
          if (id.includes('node_modules/@radix-ui')) {
            return 'ui-vendor';
          }
          // React Query and devtools
          if (id.includes('node_modules/@tanstack/react-query')) {
            // Exclude devtools from production vendor chunk
            if (id.includes('devtools')) {
              return 'query-devtools';
            }
            return 'query-vendor';
          }
          // Monaco Editor (large library)
          if (id.includes('node_modules/monaco-editor') || id.includes('node_modules/@monaco-editor')) {
            return 'editor-vendor';
          }
          // Charts library
          if (id.includes('node_modules/recharts')) {
            return 'chart-vendor';
          }
          // Zustand state management
          if (id.includes('node_modules/zustand')) {
            return 'state-vendor';
          }
          // MUI components
          if (id.includes('node_modules/@mui') || id.includes('node_modules/@emotion')) {
            return 'mui-vendor';
          }
        }
      }
    }
  },
})
