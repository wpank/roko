'use client';

import React, { useState, useEffect } from 'react';
import { clsx } from 'clsx';
import { Badge } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type IncidentKind =
  | 'taint_tracked'
  | 'tool_blocked'
  | 'quarantine'
  | 'budget_enforced'
  | 'capability_denied';

type IncidentSeverity = 'critical' | 'high' | 'medium' | 'low';

interface SafetyIncident {
  id: string;
  kind: IncidentKind;
  severity: IncidentSeverity;
  agentName: string;
  description: string;
  detail: string;
  timestamp: string;
  resolved: boolean;
}


// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const KIND_LABELS: Record<IncidentKind, string> = {
  taint_tracked:     'Taint tracked',
  tool_blocked:      'Tool blocked',
  quarantine:        'Quarantined',
  budget_enforced:   'Budget enforced',
  capability_denied: 'Capability denied',
};

const KIND_ICON: Record<IncidentKind, string> = {
  taint_tracked:     '⚡',
  tool_blocked:      '⊘',
  quarantine:        '☣',
  budget_enforced:   '$',
  capability_denied: '⛔',
};

const SEVERITY_BADGE: Record<IncidentSeverity, 'error' | 'warning' | 'info' | 'default'> = {
  critical: 'error',
  high:     'warning',
  medium:   'info',
  low:      'default',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return new Date(iso).toLocaleDateString();
}

// ---------------------------------------------------------------------------
// Incident row
// ---------------------------------------------------------------------------

function IncidentRow({ incident }: { incident: SafetyIncident }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className={clsx(
        'border-b border-b-[var(--text-ghost)]',
        !incident.resolved && incident.severity === 'critical' && 'bg-[rgba(196,31,31,0.06)]',
        !incident.resolved && incident.severity === 'high'     && 'bg-[rgba(192,160,64,0.04)]',
      )}
    >
      {/* Header row */}
      <div
        role="button"
        tabIndex={0}
        onClick={() => setExpanded((x) => !x)}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') setExpanded((x) => !x); }}
        className={clsx(
          'flex items-start gap-3 px-4 py-2.5',
          'cursor-pointer',
          'hover:bg-[var(--bg-highlight)]',
          'transition-[background-color] duration-[80ms]',
          'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose)]',
        )}
      >
        {/* Kind icon */}
        <span
          className="shrink-0 font-mono text-sm leading-snug pt-px"
          style={{ color: incident.resolved ? 'var(--text-ghost)' : 'var(--accent-error)' }}
          aria-hidden
        >
          {KIND_ICON[incident.kind]}
        </span>

        {/* Severity + kind */}
        <div className="shrink-0 flex flex-col gap-1 pt-px">
          <Badge variant={SEVERITY_BADGE[incident.severity]}>
            {incident.severity}
          </Badge>
        </div>

        {/* Kind label */}
        <span className="shrink-0 font-mono text-xs text-[var(--text-muted)] leading-snug w-32 truncate pt-px">
          {KIND_LABELS[incident.kind]}
        </span>

        {/* Description */}
        <span
          className={clsx(
            'flex-1 font-mono text-xs leading-snug min-w-0',
            !expanded && 'truncate',
            incident.resolved ? 'text-[var(--text-faint)]' : 'text-[var(--text-strong)]',
          )}
        >
          {incident.description}
        </span>

        {/* Agent name */}
        <span className="shrink-0 font-mono text-xs text-[var(--accent-cyan)] leading-snug ml-2 pt-px">
          {incident.agentName}
        </span>

        {/* Time */}
        <span className="shrink-0 font-mono text-xs text-[var(--text-ghost)] tabular-nums leading-snug pt-px ml-2">
          {formatRelativeTime(incident.timestamp)}
        </span>

        {/* Resolved badge */}
        {incident.resolved && (
          <span className="shrink-0 font-mono text-[10px] text-[var(--sage)] border border-[var(--sage)] px-1 py-0.5 leading-none">
            resolved
          </span>
        )}

        {/* Expand chevron */}
        <span
          className={clsx(
            'shrink-0 font-mono text-xs text-[var(--text-faint)] leading-none select-none pt-0.5',
            'transition-transform duration-[80ms]',
            expanded && 'rotate-90',
          )}
          aria-hidden
        >
          ›
        </span>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div
          className={clsx(
            'px-4 pb-4 pt-2',
            'bg-[var(--bg-secondary)]',
            'border-t border-t-[var(--text-ghost)]',
          )}
        >
          <p className="font-mono text-xs text-[var(--text-muted)] leading-relaxed mb-3">
            {incident.detail}
          </p>

          <div className="flex flex-wrap gap-x-6 gap-y-1.5">
            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Agent
              </span>
              <span className="font-mono text-xs text-[var(--accent-cyan)]">
                {incident.agentName}
              </span>
            </div>

            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Kind
              </span>
              <span className="font-mono text-xs text-[var(--text-faint)]">
                {KIND_LABELS[incident.kind]}
              </span>
            </div>

            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Time
              </span>
              <span className="font-mono text-xs text-[var(--text-faint)] tabular-nums">
                {new Date(incident.timestamp).toLocaleString()}
              </span>
            </div>

            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                Status
              </span>
              <span
                className="font-mono text-xs"
                style={{ color: incident.resolved ? 'var(--sage)' : 'var(--accent-error)' }}
              >
                {incident.resolved ? 'Resolved' : 'Active'}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveSafetyPage() {
  const [incidents, setIncidents] = useState<SafetyIncident[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    fetch('/api/safety/incidents')
      .then((r) => r.json())
      .then((raw: unknown) => {
        if (cancelled) return;
        const list = Array.isArray(raw)
          ? (raw as SafetyIncident[])
          : Array.isArray((raw as Record<string, unknown>)?.incidents)
          ? ((raw as Record<string, unknown>).incidents as SafetyIncident[])
          : [];
        setIncidents(list);
        setLoading(false);
      })
      .catch(() => {
        if (!cancelled) setLoading(false);
      });

    return () => { cancelled = true; };
  }, []);

  const activeCount   = incidents.filter((i) => !i.resolved).length;
  const resolvedCount = incidents.filter((i) =>  i.resolved).length;
  const criticalCount = incidents.filter((i) => !i.resolved && i.severity === 'critical').length;

  return (
    <div className="flex flex-col min-h-0">

      {/* ---------------------------------------------------------------- */}
      {/* Summary header                                                    */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'flex items-center gap-6 px-4 py-3 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        {/* Active incidents */}
        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Active
          </span>
          <span
            className="font-mono text-lg tabular-nums"
            style={{ color: activeCount > 0 ? 'var(--accent-error)' : 'var(--sage)' }}
          >
            {activeCount}
          </span>
        </div>

        <span className="w-px h-8 bg-[var(--text-ghost)]" aria-hidden />

        {/* Critical */}
        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Critical
          </span>
          <span
            className="font-mono text-lg tabular-nums"
            style={{ color: criticalCount > 0 ? 'var(--accent-error)' : 'var(--text-ghost)' }}
          >
            {criticalCount}
          </span>
        </div>

        {/* Resolved */}
        <div className="flex flex-col gap-0.5">
          <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
            Resolved
          </span>
          <span className="font-mono text-lg tabular-nums text-[var(--sage)]">
            {resolvedCount}
          </span>
        </div>

        {/* Trust-origin IFC note */}
        <div className="ml-auto flex items-center gap-2">
          <span
            className="font-mono text-xs border border-[var(--sage)] px-2 py-1"
            style={{ color: 'var(--sage)' }}
          >
            IFC active
          </span>
          <span
            className="font-mono text-xs border border-[var(--sage)] px-2 py-1"
            style={{ color: 'var(--sage)' }}
          >
            5-head corrigibility
          </span>
        </div>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Incident list                                                     */}
      {/* ---------------------------------------------------------------- */}
      {loading ? (
        <div className="flex items-center justify-center py-24">
          <span className="font-mono text-xs text-[var(--text-ghost)]">Loading incidents…</span>
        </div>
      ) : incidents.length === 0 ? (
        /* Empty state — no incidents recorded (this is the happy path) */
        <div className="flex flex-col items-center justify-center gap-3 py-24">
          <span
            className="font-mono text-3xl"
            style={{ color: 'var(--sage)' }}
            aria-hidden
          >
            ✓
          </span>
          <span className="font-mono text-sm text-[var(--text-muted)]">
            No safety incidents recorded.
          </span>
          <span className="font-mono text-xs text-[var(--text-ghost)]">
            Trust-origin IFC, immune graph, and corrigibility controls are active.
          </span>
        </div>
      ) : (
        <div>
          {/* Active first */}
          {incidents
            .filter((i) => !i.resolved)
            .sort((a, b) => {
              const ord: IncidentSeverity[] = ['critical', 'high', 'medium', 'low'];
              return ord.indexOf(a.severity) - ord.indexOf(b.severity);
            })
            .map((incident) => (
              <IncidentRow key={incident.id} incident={incident} />
            ))}

          {/* Resolved */}
          {resolvedCount > 0 && (
            <>
              <div
                className={clsx(
                  'flex items-center gap-2 px-4 py-1.5',
                  'bg-[var(--bg-secondary)]',
                  'border-b border-b-[var(--text-ghost)]',
                )}
              >
                <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
                  Resolved
                </span>
                <span className="font-mono text-xs text-[var(--text-ghost)] tabular-nums">
                  {resolvedCount}
                </span>
              </div>

              {incidents
                .filter((i) => i.resolved)
                .map((incident) => (
                  <IncidentRow key={incident.id} incident={incident} />
                ))}
            </>
          )}
        </div>
      )}

      {/* ---------------------------------------------------------------- */}
      {/* Footer: residual scope note                                       */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'px-4 py-3 mt-auto',
          'border-t border-t-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <p className="font-mono text-xs text-[var(--text-ghost)] leading-relaxed">
          Data from roko-serve safety routes.
          Provider-owned internals, trace Signals, adaptive immune memory, and
          externally-anchored whole-ledger authenticity are product residuals not yet shown here.
        </p>
      </div>
    </div>
  );
}
