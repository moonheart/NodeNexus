import path from "path"
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    proxy: {
      // Proxy all WebSocket requests under /ws
      '/ws': {
        target: 'ws://192.168.50.50:8080',
        ws: true,
        changeOrigin: true,
      },
      // Proxy all API requests under /api
      '/api': {
        target: 'http://192.168.50.50:8080',
        changeOrigin: true
      }
    }
  }
})
