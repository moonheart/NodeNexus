import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      // Proxy WebSocket requests for /ws/metrics
      '/ws/metrics': {
        target: 'ws://localhost:8080', // Your backend WebSocket server
        ws: true, // IMPORTANT: enable WebSocket proxy
        changeOrigin: true, // Recommended for most cases
        // You might need to rewrite the path if your backend expects a different path
        // rewrite: (path) => path.replace(/^\/ws\/metrics/, '/ws/metrics') // Default is usually fine
      },
      // You can add other API proxies here if needed, for example:
      // '/api': {
      //   target: 'http://localhost:8080',
      //   changeOrigin: true,
      //   rewrite: (path) => path.replace(/^\/api/, '')
      // }
    }
  }
})
