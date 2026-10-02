import { defineConfig, loadEnv, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { readFile } from 'node:fs/promises';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const APP_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURE_BUNDLES = join(APP_DIR, 'e2e', 'showcase', 'bundles');

/**
 * Dev server only, with VITE_ALLOW_FIXTURES=1: serve the showcase fixture bundles of
 * e2e/showcase/bundles at `${base}bundles/`, byte for byte, so the static showcase source can
 * check their SHA-256 digests (S10 §4.8, §7 `showcase-fixture`).
 */
function showcaseFixtureBundles(enabled: boolean): Plugin {
  return {
    name: 'showcase-fixture-bundles',
    apply: 'serve',
    configureServer(server) {
      if (!enabled) return;
      const prefix = `${server.config.base}bundles/`;
      server.middlewares.use(async (req, res, next) => {
        const url = (req.url ?? '').split('?')[0];
        if (!url.startsWith(prefix)) return next();
        let file: string;
        try {
          file = resolve(FIXTURE_BUNDLES, decodeURIComponent(url.slice(prefix.length)));
        } catch {
          res.statusCode = 400;
          res.end('bad path');
          return;
        }
        if (!file.startsWith(FIXTURE_BUNDLES + sep)) {
          res.statusCode = 403;
          res.end('outside the fixture bundles');
          return;
        }
        try {
          const body = await readFile(file);
          const json = extname(file) === '.json';
          res.setHeader('Content-Type', json ? 'application/json' : 'text/plain; charset=utf-8');
          res.setHeader('Cache-Control', 'no-store');
          res.end(body);
        } catch {
          res.statusCode = 404;
          res.end('not found');
        }
      });
    },
  };
}

export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, APP_DIR, 'VITE_');
  const fixtures = process.env.VITE_ALLOW_FIXTURES ?? env.VITE_ALLOW_FIXTURES ?? '';
  // S10 §4.5: a production build never carries fixture data, so refuse it at build time.
  if (command === 'build' && mode === 'production' && fixtures !== '') {
    throw new Error('VITE_ALLOW_FIXTURES is set: a production build must not allow fixtures');
  }
  return {
    plugins: [react(), showcaseFixtureBundles(fixtures === '1')],
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
  };
});
