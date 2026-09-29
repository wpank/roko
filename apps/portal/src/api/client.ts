/**
 * Roko Portal — Base HTTP Client
 *
 * Thin wrapper around `fetch` that:
 *  - Automatically prefixes all paths with `/api`
 *  - Attaches a Bearer token from localStorage when present
 *  - Deserialises JSON responses and throws typed `ApiError` on non-2xx
 */

import { getRokoServeUrl } from '@/lib/env';

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly statusText: string,
    public readonly body: unknown,
  ) {
    super(`HTTP ${status} ${statusText}`);
    this.name = 'ApiError';
  }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

const STORAGE_KEY_API_KEY = 'roko-api-key';

class RokoApiClient {
  /** The base URL of the roko-serve instance, as `getRokoServeUrl` resolves it. */
  private readonly baseUrl: string;

  /**
   * Optional Bearer token sent as `Authorization: Bearer <key>`.
   * Read from `localStorage["roko-api-key"]` on construction.
   */
  private readonly apiKey: string | null;

  constructor() {
    this.baseUrl = getRokoServeUrl();
    this.apiKey =
      typeof window !== 'undefined'
        ? localStorage.getItem(STORAGE_KEY_API_KEY)
        : null;
  }

  // -------------------------------------------------------------------------
  // Internal helpers
  // -------------------------------------------------------------------------

  private headers(): HeadersInit {
    const h: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.apiKey) {
      h['Authorization'] = `Bearer ${this.apiKey}`;
    }
    return h;
  }

  /**
   * Build the full URL for a given path.  When `baseUrl` is an empty string
   * (the Next.js dev-proxy case) the path is used as-is so requests go to the
   * current origin and the Next rewrite rule forwards them to roko-serve.
   */
  private url(path: string, params?: Record<string, string>): string {
    const base = this.baseUrl ? this.baseUrl : '';
    const url = new URL(`${base}${path}`, typeof window !== 'undefined' ? window.location.href : 'http://localhost');
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        url.searchParams.set(key, value);
      }
    }
    // When baseUrl is empty the URL was constructed relative to the current
    // origin, which is what we want — return the pathname+search only so we
    // don't accidentally hard-code the dev host.
    return this.baseUrl ? url.toString() : `${url.pathname}${url.search}`;
  }

  private async request<T>(
    method: string,
    path: string,
    options: {
      params?: Record<string, string>;
      body?: unknown;
    } = {},
  ): Promise<T> {
    const { params, body } = options;
    const response = await fetch(this.url(path, params), {
      method,
      headers: this.headers(),
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      let errorBody: unknown;
      try {
        errorBody = await response.json();
      } catch {
        errorBody = await response.text().catch(() => null);
      }
      throw new ApiError(response.status, response.statusText, errorBody);
    }

    // Some endpoints return 204 No Content.
    if (response.status === 204) {
      return undefined as unknown as T;
    }

    return response.json() as Promise<T>;
  }

  // -------------------------------------------------------------------------
  // Public HTTP verbs
  // -------------------------------------------------------------------------

  get<T>(path: string, params?: Record<string, string>): Promise<T> {
    return this.request<T>('GET', path, { params });
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, { body });
  }

  put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PUT', path, { body });
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>('DELETE', path);
  }
}

// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------

export const api = new RokoApiClient();
