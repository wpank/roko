'use client';

import React, { useMemo } from 'react';
import { clsx } from 'clsx';
import { useAgents } from '@/api/hooks';
import { useDashboardStore } from '@/stores/dashboard';
import { Spinner } from '@/components/atoms/Spinner';
import { StatusLED } from '@/components/atoms/StatusLED';
import type { AgentStatus } from '@/api/types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function statusToLED(
  status: AgentStatus,
): 'success' | 'warning' | 'error' | 'offline' | 'idle' | 'active' {
  switch (status) {
    case 'active':    return 'active';
    case 'idle':      return 'idle';
    case 'completed': return 'success';
    case 'failed':    return 'error';
    case 'stopped':   return 'offline';
    default:          return 'offline';
  }
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function AgentsTopologyPage() {
  const { data: queriedAgents, isLoading, isError } = useAgents();
  const liveAgents = useDashboardStore((s) => s.agents);

  // Merge REST snapshot with live SSE data; SSE wins on conflict.
  const agents = useMemo(() => {
    const seed: Record<string, (typeof liveAgents)[string]> = {};
    if (queriedAgents) {
      for (const a of queriedAgents) {
        seed[a.id] = a as typeof liveAgents[string];
      }
    }
    return { ...seed, ...liveAgents };
  }, [queriedAgents, liveAgents]);

  const agentList = useMemo(() => Object.values(agents), [agents]);

  // ---------------------------------------------------------------------------
  // Loading / error guards
  // ---------------------------------------------------------------------------

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 gap-2">
        <Spinner size="sm" />
        <span className="font-mono text-xs text-[var(--text-ghost)]">Loading agents…</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-2">
        <span className="font-mono text-xs text-[var(--accent-error)]">
          Failed to load agents. Is roko serve running?
        </span>
      </div>
    );
  }

  if (agentList.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-3 px-4 text-center">
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          No agents registered.
        </span>
        <span className="font-mono text-[10px] text-[var(--text-faint)]">
          Create agents from the Roster tab. Their connections will appear here once active.
        </span>
      </div>
    );
  }

  // Group by status for the topology overview
  const byStatus = useMemo(() => {
    const map: Partial<Record<AgentStatus, typeof agentList>> = {};
    for (const a of agentList) {
      if (!map[a.status]) map[a.status] = [];
      map[a.status]!.push(a);
    }
    return map;
  }, [agentList]);

  const STATUS_ORDER: AgentStatus[] = ['active', 'idle', 'completed', 'failed', 'stopped'];

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Summary header */}
      <div className="flex items-center gap-4 border-b border-b-[var(--text-ghost)] pb-4">
        <span className="font-mono text-xs text-[var(--text-faint)] uppercase tracking-widest">
          Agent Topology
        </span>
        <span className="font-mono text-xs text-[var(--text-ghost)] tabular-nums">
          {agentList.length} total
        </span>
        <span
          className="font-mono text-xs tabular-nums"
          style={{
            color: (byStatus.active?.length ?? 0) > 0
              ? 'var(--sage)'
              : 'var(--text-ghost)',
          }}
        >
          {byStatus.active?.length ?? 0} active
        </span>
      </div>

      {/* Node grid grouped by status */}
      {STATUS_ORDER.map((status) => {
        const group = byStatus[status];
        if (!group || group.length === 0) return null;
        return (
          <section key={status}>
            <div className="flex items-center gap-2 mb-3">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                {status}
              </span>
              <div className="flex-1 h-px bg-[var(--text-ghost)] opacity-30" />
              <span className="font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
                {group.length}
              </span>
            </div>

            <div
              className="grid gap-3"
              style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(160px, 1fr))' }}
            >
              {group.map((agent) => (
                <div
                  key={agent.id}
                  className={clsx(
                    'flex flex-col gap-2 p-3',
                    'border border-[var(--text-ghost)]',
                    'bg-[var(--bg-secondary)]',
                    'hover:border-[var(--border-hover)]',
                    'transition-[border-color] duration-[80ms]',
                  )}
                >
                  {/* LED + name */}
                  <div className="flex items-center gap-2 min-w-0">
                    <StatusLED
                      status={statusToLED(agent.status)}
                      pulse={agent.status === 'active'}
                      size="sm"
                    />
                    <span
                      className="truncate font-mono text-xs text-[var(--text-strong)] leading-none"
                      title={agent.name}
                    >
                      {agent.name}
                    </span>
                  </div>

                  {/* Role + model */}
                  <div className="flex flex-col gap-0.5">
                    <span className="font-mono text-[10px] text-[var(--text-muted)] truncate">
                      {agent.role}
                    </span>
                    <span className="font-mono text-[10px] text-[var(--text-faint)] truncate">
                      {agent.model}
                    </span>
                  </div>

                  {/* Current task */}
                  {agent.currentTask && (
                    <span className="font-mono text-[10px] text-[var(--warning)] truncate border-t border-t-[var(--text-ghost)] pt-1.5">
                      {agent.currentTask}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
