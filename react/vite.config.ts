import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'


export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    allowedHosts: ["fedora.ibex-mooneye.ts.net"],
    // Proxy must be inside the server block
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
        // rewrite: (path) => path.replace(/^\/api/, ''), // Use this if your Rust server doesn't expect "/api" in the path
      }
    }
  }
})
