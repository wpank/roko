import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ command }) => ({
  plugins: [react()],
  // Build for /demo/ so assets resolve correctly under roko serve.
  // Dev server stays at / so Playwright e2e specs (baseURL: localhost:5173) work unchanged.
  base: command === 'build' ? '/demo/' : '/',
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('/node_modules/three/')) return 'vendor-three';
          if (id.includes('/node_modules/@xterm/')) return 'vendor-xterm';
        },
      },
    },
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:6677',
        changeOrigin: true,
        ws: true,
      },
      '/ws': {
        target: 'http://localhost:6677',
        changeOrigin: true,
        ws: true,
      },
      '/relay': {
        target: 'http://localhost:6677',
        changeOrigin: true,
        ws: true,
      },
      '/health': 'http://localhost:6677',
    },
    // Prevent Vite from watching directories that roko-serve or CLI commands
    // write to during demo execution, which triggers unwanted full page reloads.
    watch: {
      ignored: [
        '**/node_modules/**',
        '**/.roko/**',
        '**/roko.toml',
        '**/target/**',
        '/tmp/**',
      ],
    },
    hmr: {
      // Overlay compile errors only — don't let transient HMR disconnects
      // (e.g. roko-serve restart) trigger a full browser reload.
      overlay: true,
    },
  },
}));
