/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    host: true, // Listen on all addresses (0.0.0.0)
    allowedHosts: ["fedora.ibex-mooneye.ts.net"],
    proxy: {
      '/api': {
        // Use the Docker service name 'backend' and port 8080
        target: 'http://backend:8080',
        changeOrigin: true,
        // rewrite: (path) => path.replace(/^\/api/, ''), 
      }
    }
  },
  test: {
    environment: 'jsdom',
    setupFiles: './src/test-setup.ts',
    globals: true,
  },
})
