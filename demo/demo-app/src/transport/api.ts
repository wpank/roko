import { SERVE_URL } from '../lib/serve-url';

/** The header the server requires on every non-GET request a session makes (S11 §4.3). */
export const CSRF_HEADER = 'X-Roko-CSRF';

/** The session endpoint (9323); `GET` is the public probe. */
export const SESSION_PATH = '/api/auth/session';

/** `GET /api/auth/session`: whether this browser holds a session, and how the server logs in. */
export interface SessionProbe {
  authenticated: boolean;
  login: 'passphrase' | 'token';
  showcase_mode: boolean;
  scopes: string[];
  expires_at: string | null;
}

/** Where the browser is, and how to send it elsewhere. */
export interface LoginRouting {
  location: { pathname: string; search: string };
  assign: (href: string) => void;
}

/** The app's base path: `/demo/` in a build, `/` in dev or outside Vite. */
function appBase(): string {
  const env = (import.meta as { env?: { BASE_URL?: string } }).env;
  return env?.BASE_URL ?? '/';
}

/** The login page under `base`, sending the visitor back to `next` afterwards. */
export function loginHref(next: string, base: string = appBase()): string {
  const root = base.endsWith('/') ? base : `${base}/`;
  return `${root}login?next=${encodeURIComponent(next)}`;
}

/** The browser's own location and navigation; `null` outside a browser. */
function browserRouting(): LoginRouting | null {
  if (typeof window === 'undefined') return null;
  return { location: window.location, assign: (href) => window.location.assign(href) };
}

/** Go to the login page with the current path as `next`, unless already there. */
export function goToLogin(routing: LoginRouting | null = browserRouting()): void {
  if (!routing || routing.location.pathname.endsWith('/login')) return;
  routing.assign(loginHref(`${routing.location.pathname}${routing.location.search}`));
}

/** `GET /api/auth/session` at `baseUrl`; `null` when the server cannot be asked. */
export async function probeSession(baseUrl: string = SERVE_URL): Promise<SessionProbe | null> {
  try {
    const res = await fetch(baseUrl + SESSION_PATH, { credentials: 'same-origin' });
    if (!res.ok) return null;
    return (await res.json()) as SessionProbe;
  } catch {
    return null;
  }
}

/** Whether a probe says the server takes passphrase logins and this browser holds no session. */
export function loggedOut(probe: SessionProbe | null): boolean {
  return probe !== null && !probe.authenticated && probe.login === 'passphrase';
}

/**
 * After a 401: ask the session endpoint, and when the server takes passphrase logins (showcase
 * mode) and this browser holds no session, go to the login page. Other servers' 401s are left to
 * the caller. Returns whether it navigated.
 */
export async function routeToLoginIfLoggedOut(
  baseUrl: string = SERVE_URL,
  routing: LoginRouting | null = browserRouting(),
): Promise<boolean> {
  if (!routing || !loggedOut(await probeSession(baseUrl))) return false;
  goToLogin(routing);
  return true;
}

/** Options of a {@link RokoApi}. */
export interface RokoApiOptions {
  /** Called on a 401 from any route but the session endpoint. */
  onUnauthorized?: () => void;
}

export interface ApiError {
  status: number;
  statusText: string;
  body: string | null;
}

/** Result type -- never throws. Callers check `.ok` and branch. */
export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** Health probe result cached with TTL. */
export interface HealthSnapshot {
  reachable: boolean;
  checkedAt: number; // Date.now() ms
}

export class RokoApi {
  readonly baseUrl: string;
  private healthCache: HealthSnapshot | null;
  private healthInflight: Promise<HealthSnapshot> | null;
  private readonly onUnauthorized?: () => void;
  private static readonly HEALTH_TTL_MS = 30_000;

  constructor(baseUrl?: string, options: RokoApiOptions = {}) {
    this.baseUrl = baseUrl ?? SERVE_URL;
    this.healthCache = null;
    this.healthInflight = null;
    this.onUnauthorized = options.onUnauthorized;
  }

  /**
   * Internal fetch helper -- never throws. Every request carries the session cookie on the same
   * origin, and every non-GET the CSRF header; a 401 calls `onUnauthorized`.
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    signal?: AbortSignal,
  ): Promise<ApiResult<T>> {
    const url = this.baseUrl + path;
    const headers: Record<string, string> = {};
    if (body !== undefined) {
      headers['Content-Type'] = 'application/json';
    }
    if (method !== 'GET' && method !== 'HEAD') {
      headers[CSRF_HEADER] = '1';
    }
    try {
      const res = await fetch(url, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
        signal,
        credentials: 'same-origin',
      });
      if (res.status === 401 && path !== SESSION_PATH) {
        this.onUnauthorized?.();
      }
      if (!res.ok) {
        const text = await res.text().catch(() => null);
        return { ok: false, error: { status: res.status, statusText: res.statusText, body: text } };
      }
      const data = (await res.json()) as T;
      return { ok: true, data };
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      return { ok: false, error: { status: 0, statusText: message, body: null } };
    }
  }

  /** GET with JSON parse. Returns ApiResult -- never throws. */
  get<T = unknown>(path: string, signal?: AbortSignal): Promise<ApiResult<T>> {
    return this.request<T>('GET', path, undefined, signal);
  }

  /** POST with JSON body. Returns ApiResult -- never throws. */
  post<T = unknown>(path: string, body?: unknown, signal?: AbortSignal): Promise<ApiResult<T>> {
    return this.request<T>('POST', path, body, signal);
  }

  /** PUT with JSON body. Returns ApiResult -- never throws. */
  put<T = unknown>(path: string, body?: unknown, signal?: AbortSignal): Promise<ApiResult<T>> {
    return this.request<T>('PUT', path, body, signal);
  }

  /** DELETE. Returns ApiResult -- never throws. */
  delete<T = unknown>(path: string, signal?: AbortSignal): Promise<ApiResult<T>> {
    return this.request<T>('DELETE', path, undefined, signal);
  }

  /** Probe /health with 30s TTL cache + 2s timeout. Deduplicated -- only one in-flight. */
  probe(force?: boolean): Promise<HealthSnapshot> {
    if (
      !force &&
      this.healthCache &&
      Date.now() - this.healthCache.checkedAt < RokoApi.HEALTH_TTL_MS
    ) {
      return Promise.resolve(this.healthCache);
    }
    if (this.healthInflight) {
      return this.healthInflight;
    }
    this.healthInflight = (async () => {
      let snapshot: HealthSnapshot;
      try {
        const res = await fetch(this.baseUrl + '/health', {
          signal: AbortSignal.timeout(2000),
        });
        snapshot = { reachable: res.ok, checkedAt: Date.now() };
      } catch {
        snapshot = { reachable: false, checkedAt: Date.now() };
      }
      this.healthCache = snapshot;
      this.healthInflight = null;
      return snapshot;
    })();
    return this.healthInflight;
  }
}

/**
 * Singleton instance. Import this everywhere instead of constructing. A 401 sends a logged-out
 * showcase visitor to the login page.
 */
export const api = new RokoApi(undefined, {
  onUnauthorized: () => {
    void routeToLoginIfLoggedOut();
  },
});
