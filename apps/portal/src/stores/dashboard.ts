/**
 * Dashboard store — thin Zustand wrapper over the pure RunState fold.
 *
 * All mutations flow through `applyEvent` (single-event fold) and
 * `replaceFromSnapshot` (full snapshot replacement on gap recovery or
 * initial load).  The fold functions live in `@/lib/runState`; this
 * module is pure wiring.
 */

import { create } from 'zustand';
import type { ConnectionStatus, SessionResult } from '@/lib/bootstrap';
import type { WireDashboardEvent, WireDashboardSnapshot } from '@/api/contracts';
import {
  initialRunState,
  fromSnapshot,
  applyEvent as foldEvent,
} from '@/lib/runState';
import type { RunState } from '@/lib/runState';

// ---------------------------------------------------------------------------
// Store interface
// ---------------------------------------------------------------------------

interface DashboardStore {
  /** SSE connection state. */
  connection: ConnectionStatus;
  /** Session cookie state — 'pending' until the sign-in step settles. */
  session: SessionResult | 'pending';
  /** Accumulated run state built by folding dashboard events. */
  run: RunState;

  setSession(r: SessionResult): void;
  /** Replace all state from a materialized snapshot (gap recovery / startup). */
  replaceFromSnapshot(s: WireDashboardSnapshot): void;
  /** Fold one incremental dashboard event into the state. */
  applyEvent(e: WireDashboardEvent): void;
  setConnection(s: ConnectionStatus): void;
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  connection: 'disconnected',
  session: 'pending',
  run: initialRunState(),

  setSession: (r) => set({ session: r }),

  replaceFromSnapshot: (s) => set({ run: fromSnapshot(s, Date.now()) }),

  applyEvent: (e) => set({ run: foldEvent(get().run, e, Date.now()) }),

  setConnection: (s) => set({ connection: s }),
}));
