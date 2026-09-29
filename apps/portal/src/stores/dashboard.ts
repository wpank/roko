/**
 * Dashboard store — thin Zustand wrapper over the pure RunState fold.
 *
 * All mutations flow through `applyEvent` (single-event fold) and
 * `replaceFromSnapshot` (full snapshot replacement on gap recovery or
 * initial load).  The fold functions live in `@/lib/runState`; this
 * module is pure wiring — plus `unsavedPlan`, the one piece of UI state
 * that regions other than its own must see.
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
  /**
   * The plan whose open editor holds unsaved text. No run of it starts and
   * no way of leaving the editor proceeds without asking (design §4.1, §4a).
   */
  unsavedPlan: string | null;

  setSession(r: SessionResult): void;
  /** Replace all state from a materialized snapshot (gap recovery / startup). */
  replaceFromSnapshot(s: WireDashboardSnapshot): void;
  /** Fold one incremental dashboard event into the state. */
  applyEvent(e: WireDashboardEvent): void;
  setConnection(s: ConnectionStatus): void;
  /** Record whether `planId`'s editor holds unsaved text. */
  setUnsaved(planId: string, dirty: boolean): void;
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useDashboardStore = create<DashboardStore>((set, get) => ({
  connection: 'disconnected',
  session: 'pending',
  run: initialRunState(),
  unsavedPlan: null,

  setSession: (r) => set({ session: r }),

  replaceFromSnapshot: (s) => set({ run: fromSnapshot(s, Date.now()) }),

  applyEvent: (e) => set({ run: foldEvent(get().run, e, Date.now()) }),

  setConnection: (s) => set({ connection: s }),

  setUnsaved: (planId, dirty) =>
    set((s) => ({ unsavedPlan: dirty ? planId : s.unsavedPlan === planId ? null : s.unsavedPlan })),
}));

/** Ask before unsaved editor text is thrown away; true when there is none or the operator agrees. */
export function confirmDiscard(dirty: boolean): boolean {
  return !dirty || window.confirm('Discard unsaved edits?');
}
