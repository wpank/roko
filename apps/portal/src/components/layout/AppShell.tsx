'use client';

import { useEffect, useRef, useState } from 'react';
import { clsx } from 'clsx';
import Sidebar from './Sidebar';
import Header from './Header';
import StatusBar from './StatusBar';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { CommandPalette, useCommandPalette } from '@/components/command-palette';
import { ToastContainer } from '@/components/atoms/Toast';
import { useDashboardStore } from '@/stores/dashboard';
import { useHealth, useAgents } from '@/api/hooks';

interface AppShellProps {
  children: React.ReactNode;
}

/**
 * AppShell — the root chrome for the entire portal.
 *
 * Layout structure:
 *
 *   ┌──────────────────────────────────────┐
 *   │ Header (h-12, full width)            │
 *   ├───────────┬──────────────────────────┤
 *   │ Sidebar   │ main content (flex-1)    │
 *   │ (200|56px)│ overflow-y: auto         │
 *   ├───────────┴──────────────────────────┤
 *   │ StatusBar (h-6, full width)          │
 *   └──────────────────────────────────────┘
 *
 * The atmosphere overlay (film grain + vignette) sits behind all content via
 * pointer-events: none and a z-index below the layout chrome.
 *
 * Sidebar collapse state lives here so Header and content area can react to
 * the width change if needed in the future.
 */
export function AppShell({ children }: AppShellProps) {
  const { open: commandPaletteOpen, setOpen: setCommandPaletteOpen } = useCommandPalette();
  const connectionStatus = useDashboardStore((s) => s.connectionStatus);

  // Resolve live agent + provider counts for the status bar.
  const { data: health } = useHealth();
  const { data: agentList } = useAgents();

  // Measure health-check round-trip latency.
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const latencyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    async function measure() {
      const t0 = performance.now();
      try {
        await fetch('/api/health');
        setLatencyMs(Math.round(performance.now() - t0));
      } catch {
        setLatencyMs(null);
      }
    }
    measure();
    // Re-measure every 30 s.
    const id = setInterval(measure, 30_000);
    return () => {
      clearInterval(id);
      if (latencyTimerRef.current) clearTimeout(latencyTimerRef.current);
    };
  }, []);

  const agentCount = agentList?.length ?? health?.active_agents ?? 0;
  const providerCount = health?.providers?.total ?? 0;

  return (
    // Outermost container — full viewport, clipped to prevent overflow
    <div
      className={clsx(
        'atmosphere atmosphere--subtle',
        'relative flex flex-col h-screen overflow-hidden',
        'bg-[var(--void)]',
      )}
    >
      {/* ---- Header ---- */}
      <Header
        onCommandPalette={() => setCommandPaletteOpen((o) => !o)}
      />

      {/* ---- Body row: Sidebar + main ---- */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* ---- Sidebar ---- */}
        <Sidebar connectionStatus={connectionStatus === 'error' ? 'disconnected' : connectionStatus} />

        {/* ---- Main content ---- */}
        <main
          className={clsx(
            'relative flex-1 flex flex-col min-w-0 min-h-0',
            'bg-[var(--bg-raised)]',
            'overflow-hidden',
          )}
        >
          {/* Scrollable content region */}
          <div className="flex-1 overflow-y-auto overflow-x-hidden">
            <ErrorBoundary name="Page">
              {children}
            </ErrorBoundary>
          </div>
        </main>
      </div>

      {/* ---- Status bar ---- */}
      <StatusBar
        agentCount={agentCount}
        providerCount={providerCount}
        latencyMs={latencyMs}
      />

      {/* ---- Command palette ---- */}
      <CommandPalette
        open={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
      />

      {/* ---- Toast notifications ---- */}
      <ToastContainer />
    </div>
  );
}
