import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  base: '/__admin/',
  build: {
    outDir: 'dist',
    emptyOutDir: true
  }
})
