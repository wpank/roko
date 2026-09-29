/**
 * Environment configuration helpers.
 *
 * Resolution order for the roko-serve URL:
 *   1. localStorage `roko-connection-url` (client-side only, user-saved profile)
 *   2. `NEXT_PUBLIC_ROKO_SERVE_URL` environment variable
 *   3. Hard-coded localhost default
 */

export const ROKO_CONNECTION_URL_KEY = 'roko-connection-url';

/**
 * Return the base URL of the roko-serve instance to connect to.
 *
 * In dev mode (running via `next dev`), returns an empty string so that all
 * requests hit the current origin and get forwarded by the Next.js rewrite
 * rules in `next.config.ts` to roko-serve. This avoids CORS issues.
 *
 * In production / embedded mode, the env var or saved profile is used.
 * On the server side (SSR / build), only the env var is available.
 */
export function getRokoServeUrl(): string {
  // Client-side: check for user-saved profile first
  if (typeof window !== 'undefined') {
    try {
      const saved = localStorage.getItem(ROKO_CONNECTION_URL_KEY);
      if (saved) {
        return saved.replace(/\/+$/, '');
      }
    } catch {
      // localStorage may throw in private-browsing or iframe sandboxes.
    }

    // In dev mode, return empty string so requests go through the Next.js
    // proxy rewrite (port 3000 → port 6677). This avoids CORS.
    if (process.env.NODE_ENV === 'development') {
      return '';
    }
  }

  const fromEnv = process.env.NEXT_PUBLIC_ROKO_SERVE_URL;
  if (fromEnv) {
    return fromEnv.replace(/\/+$/, '');
  }
  // Production fallback — assume co-located on same origin (embedded mode)
  return '';
}
