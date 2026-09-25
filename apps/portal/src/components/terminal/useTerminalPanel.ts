'use client';

import { create } from 'zustand';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type PanelState = 'hidden' | 'minimized' | 'open' | 'maximized';
export type SessionStatus = 'active' | 'idle' | 'dead';

export interface TerminalSession {
  id: string;
  name: string;
  status: SessionStatus;
}

interface TerminalStore {
  /** Current visibility / size state of the panel. */
  panelState: PanelState;

  /** All open sessions (tabs). */
  sessions: TerminalSession[];

  /** The session whose content is displayed. Null when no sessions exist. */
  activeSessionId: string | null;

  /**
   * Panel height as a percentage of the viewport height.
   * Only meaningful when panelState === 'open'.
   * Clamped to [15, 75] during resize to prevent unusable extremes.
   */
  panelHeight: number;

  // ---- Actions ------------------------------------------------------------

  /** Toggle: hidden → minimized, minimized → open, open → hidden. */
  toggle: () => void;

  /** Collapse the panel to the 28-px title bar. */
  minimize: () => void;

  /** Expand the panel to fill the viewport. */
  maximize: () => void;

  /** Restore from maximized or minimized back to the normal open state. */
  restore: () => void;

  /** Update the panel height (drag-resize). Value is clamped [15, 75]. */
  setHeight: (h: number) => void;

  /** Create a new session tab and make it active. */
  addSession: () => void;

  /**
   * Close a session tab.
   * If the closed tab was active, the nearest remaining tab is activated.
   * When the last tab is closed the panel transitions to 'minimized'.
   */
  removeSession: (id: string) => void;

  /** Switch to the given session tab. */
  setActiveSession: (id: string) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let _nextSessionNumber = 1;

function makeSession(): TerminalSession {
  return {
    id: `session-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    name: `Session ${_nextSessionNumber++}`,
    status: 'active',
  };
}

function clampHeight(h: number): number {
  return Math.max(15, Math.min(75, h));
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const DEFAULT_HEIGHT = 35;

const initialSession = makeSession();

export const useTerminalPanel = create<TerminalStore>((set) => ({
  panelState: 'hidden',
  sessions: [initialSession],
  activeSessionId: initialSession.id,
  panelHeight: DEFAULT_HEIGHT,

  toggle: () =>
    set((s) => {
      switch (s.panelState) {
        case 'hidden':
          return { panelState: 'minimized' };
        case 'minimized':
          return { panelState: 'open' };
        case 'open':
        case 'maximized':
          return { panelState: 'hidden' };
      }
    }),

  minimize: () => set({ panelState: 'minimized' }),

  maximize: () => set({ panelState: 'maximized' }),

  restore: () => set({ panelState: 'open' }),

  setHeight: (h) => set({ panelHeight: clampHeight(h) }),

  addSession: () =>
    set((s) => {
      const session = makeSession();
      // Opening a new session also opens the panel if it was hidden/minimized.
      const panelState =
        s.panelState === 'hidden' || s.panelState === 'minimized'
          ? 'open'
          : s.panelState;
      return {
        sessions: [...s.sessions, session],
        activeSessionId: session.id,
        panelState,
      };
    }),

  removeSession: (id) =>
    set((s) => {
      const idx = s.sessions.findIndex((sess) => sess.id === id);
      if (idx === -1) return s;

      const next = s.sessions.filter((sess) => sess.id !== id);

      if (next.length === 0) {
        return {
          sessions: next,
          activeSessionId: null,
          panelState: 'minimized',
        };
      }

      // If removing the active tab, activate the nearest remaining one.
      let nextActive = s.activeSessionId;
      if (s.activeSessionId === id) {
        // Prefer the tab to the right; fall back to the left.
        const candidate = next[idx] ?? next[idx - 1];
        nextActive = candidate?.id ?? null;
      }

      return { sessions: next, activeSessionId: nextActive };
    }),

  setActiveSession: (id) => set({ activeSessionId: id }),
}));
