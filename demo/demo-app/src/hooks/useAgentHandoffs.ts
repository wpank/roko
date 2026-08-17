import { useState, useMemo, useCallback } from 'react';
import { useServerEventSubscription } from './useEventStream';
import type { DashboardEvent } from '../transport/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface HandoffEntry {
  id: string;
  from: { name: string; role?: string; status?: 'idle' | 'working' | 'done' };
  to: { name: string; role?: string; status?: 'idle' | 'working' | 'done' };
  status: 'pending' | 'active' | 'done' | 'error';
  label: string;
  taskId?: string;
  timestamp: number;
}

export interface UseAgentHandoffsResult {
  /** All observed handoffs (capped at 20, FIFO). */
  handoffs: HandoffEntry[];
  /** Most recent entry with status === 'active', or null. */
  activeHandoff: HandoffEntry | null;
}

// ---------------------------------------------------------------------------
// SSE event shapes
// ---------------------------------------------------------------------------

type AgentSpawnedEvent = Extract<DashboardEvent, { type: 'agent_spawned' }>;
type AgentCompletedEvent = Extract<DashboardEvent, { type: 'agent_completed' }>;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_HANDOFFS = 20;

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Derives agent handoffs from the canonical DashboardEvent agent lifecycle.
 * A spawn is active immediately; completion closes the matching handoff.
 */
export function useAgentHandoffs(): UseAgentHandoffsResult {
  const [handoffs, setHandoffs] = useState<HandoffEntry[]>([]);

  useServerEventSubscription(
    ['agent_spawned', 'agent_completed'],
    useCallback((event: Record<string, unknown>) => {
      const e = event as { type: string };

      if (e.type === 'agent_spawned') {
        const ev = event as unknown as AgentSpawnedEvent;
        const entry: HandoffEntry = {
          id: `handoff-${ev.agent_id}-${Date.now()}`,
          from: { name: 'orchestrator', role: 'dispatcher', status: 'working' },
          to: { name: ev.agent_id, role: ev.role, status: 'working' },
          status: 'active',
          label: ev.task_id || `Running ${ev.agent_id}`,
          taskId: ev.task_id,
          timestamp: Date.now(),
        };
        setHandoffs((prev) => {
          const next = [...prev, entry];
          return next.length > MAX_HANDOFFS ? next.slice(-MAX_HANDOFFS) : next;
        });
      }

      if (e.type === 'agent_completed') {
        const ev = event as unknown as AgentCompletedEvent;
        setHandoffs((prev) =>
          prev.map((h) => {
            if (
              h.to.name === ev.agent_id &&
              (h.status === 'active' || h.status === 'pending')
            ) {
              return {
                ...h,
                status: 'done' as const,
                to: { ...h.to, status: 'done' as const },
                from: { ...h.from, status: 'idle' as const },
              };
            }
            return h;
          }),
        );
      }
    }, []),
  );

  const activeHandoff = useMemo(() => {
    for (let i = handoffs.length - 1; i >= 0; i--) {
      if (handoffs[i].status === 'active') return handoffs[i];
    }
    return null;
  }, [handoffs]);

  return { handoffs, activeHandoff };
}
