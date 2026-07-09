import { createContext, useContext } from 'react';
import { SERVE_URL } from '../lib/config';
import type { DataMode } from '../lib/types';

export class RokoApi {
  private live = false;
  private lastProbe = 0;
  private probeTTL = 30_000;
  private probePromise: Promise<boolean> | null = null;
  private onStatusChange: (mode: DataMode) => void;

  constructor(onStatusChange: (mode: DataMode) => void) {
    this.onStatusChange = onStatusChange;
  }

  get isLive() {
    return this.live;
  }

  get dataMode(): DataMode {
    return this.live ? 'live' : 'seed';
  }

  async probe(): Promise<boolean> {
    const now = Date.now();
    if (now - this.lastProbe < this.probeTTL && this.probePromise) {
      return this.probePromise;
    }

    this.probePromise = (async () => {
      try {
        const res = await fetch(`${SERVE_URL}/api/health`, {
          signal: AbortSignal.timeout(5000),
        });
        const wasLive = this.live;
        this.live = res.ok;
        this.lastProbe = Date.now();
        if (this.live !== wasLive) {
          this.onStatusChange(this.live ? 'live' : 'seed');
        }
        return this.live;
      } catch {
        const wasLive = this.live;
        this.live = false;
        this.lastProbe = Date.now();
        if (wasLive) {
          this.onStatusChange('seed');
        }
        return false;
      }
    })();

    return this.probePromise;
  }

  async get<T>(path: string, fallback: T): Promise<T> {
    await this.probe();
    if (!this.live) return fallback;

    try {
      const res = await fetch(`${SERVE_URL}${path}`, {
        signal: AbortSignal.timeout(10000),
      });
      if (!res.ok) return fallback;
      return await res.json() as T;
    } catch {
      return fallback;
    }
  }

  async post<T>(path: string, body: unknown): Promise<T | null> {
    await this.probe();
    if (!this.live) return null;

    try {
      const res = await fetch(`${SERVE_URL}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(30000),
      });
      if (!res.ok) return null;
      return await res.json() as T;
    } catch {
      return null;
    }
  }
}

export const ApiContext = createContext<RokoApi | null>(null);

export function useApi(): RokoApi {
  const api = useContext(ApiContext);
  if (!api) throw new Error('useApi must be used within ApiContext.Provider');
  return api;
}
