'use client';

import React, { useState, useCallback } from 'react';
import { PlanRow } from '@/components/rail/PlanRow';
import type { PlanRowsResult, PlanGroupModel } from '@/lib/planRows';

// ── localStorage helpers ───────────────────────────────────────────────────────

const LS_KEY = 'roko.rail.collapsed';

function readCollapsed(): Set<string> {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return new Set<string>(parsed);
  } catch {
    // ignore
  }
  return new Set();
}

function writeCollapsed(set: Set<string>): void {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify([...set]));
  } catch {
    // ignore
  }
}

// ── PlanRail ──────────────────────────────────────────────────────────────────

/**
 * PlanRail — the only plan list in the product.
 *
 * Presentational: all data, filter state, and selection come from props.
 * Workspace (T09) is the owner.
 *
 * Layout:
 *   nav[data-region="rail"]
 *     header  — "PLANS N" + "▶ Run all"
 *     input   — filter (substring; no chips, no sort menu)
 *     list    — top-level rows first, then named groups
 *     footer  — "+ New plan"
 *
 * Group collapse state persists in localStorage under `roko.rail.collapsed`.
 * Every localStorage call is wrapped in try/catch.
 */
export function PlanRail({
  result,
  selectedPlanId,
  filter,
  onFilterChange,
  filterInputRef,
  onSelect,
  onNewPlan,
  onRunPlans,
  emptySentence,
  runDisabledReason,
}: {
  result: PlanRowsResult;
  selectedPlanId: string | null;
  filter: string;
  onFilterChange(value: string): void;
  filterInputRef: React.RefObject<HTMLInputElement | null>;
  onSelect(id: string): void;
  onNewPlan(): void;
  onRunPlans(ids: string[] | null, label: string): void;
  emptySentence: string;
  /** When non-null, Run all and every group ▶ are disabled with this reason as their title. */
  runDisabledReason?: string | null;
}) {
  // ── Collapsed group state (lazy-initialised from localStorage) ───────────
  const [collapsed, setCollapsed] = useState<Set<string>>(() => readCollapsed());

  const toggleGroup = useCallback((name: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      writeCollapsed(next);
      return next;
    });
  }, []);

  // ── Derived ───────────────────────────────────────────────────────────────
  const { groups, count } = result;
  const hasRows = count > 0;

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <nav
      data-region="rail"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 0,
        height: '100%',
        overflow: 'hidden',
      }}
    >
      {/* ── Header ── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 8px 4px',
          flexShrink: 0,
        }}
      >
        <span
          style={{
            fontSize: 'var(--text-xs)',
            fontWeight: 600,
            letterSpacing: '0.06em',
            color: 'var(--text-muted)',
            textTransform: 'uppercase',
          }}
        >
          PLANS{' '}
          <span
            style={{
              fontVariantNumeric: 'tabular-nums',
              color: 'var(--text-faint)',
            }}
          >
            {count}
          </span>
        </span>

        <button
          type="button"
          data-action="run-all"
          disabled={runDisabledReason != null}
          title={runDisabledReason ?? undefined}
          onClick={() => onRunPlans(null, `all ${count} plans`)}
          style={{
            background: 'transparent',
            border: '1px solid var(--blur-border)',
            borderRadius: 3,
            padding: '1px 6px',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontSize: 'var(--text-xs)',
            color: 'var(--text-muted)',
            display: 'flex',
            alignItems: 'center',
            gap: 3,
          }}
        >
          ▶ Run all
        </button>
      </div>

      {/* ── Filter input ── */}
      <div style={{ padding: '2px 8px 4px', flexShrink: 0 }}>
        <input
          ref={filterInputRef}
          type="text"
          value={filter}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="Filter…"
          aria-label="Filter plans"
          style={{
            width: '100%',
            background: 'transparent',
            border: '1px solid var(--blur-border)',
            borderRadius: 3,
            padding: '2px 6px',
            fontFamily: 'inherit',
            fontSize: 'var(--text-xs)',
            color: 'var(--text-muted)',
            outline: 'none',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* ── Plan list ── */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '0 8px',
          display: 'flex',
          flexDirection: 'column',
          gap: 2,
        }}
      >
        {!hasRows ? (
          <p
            style={{
              margin: '8px 0 0',
              fontSize: 'var(--text-xs)',
              color: 'var(--text-faint)',
            }}
          >
            {emptySentence}
          </p>
        ) : (
          groups.map((group) => (
            <GroupSection
              key={group.name ?? '__top__'}
              group={group}
              collapsed={group.name !== null && collapsed.has(group.name)}
              selectedPlanId={selectedPlanId}
              onSelect={onSelect}
              onToggleCollapse={toggleGroup}
              onRunGroup={onRunPlans}
              runDisabledReason={runDisabledReason}
            />
          ))
        )}
      </div>

      {/* ── Footer ── */}
      <div
        style={{
          flexShrink: 0,
          padding: '4px 8px 6px',
          borderTop: '1px solid var(--blur-border)',
        }}
      >
        <button
          type="button"
          data-action="new-plan"
          onClick={onNewPlan}
          style={{
            width: '100%',
            background: 'transparent',
            border: '1px solid var(--blur-border)',
            borderRadius: 3,
            padding: '3px 6px',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontSize: 'var(--text-xs)',
            color: 'var(--text-muted)',
            textAlign: 'left',
          }}
        >
          + New plan
        </button>
      </div>
    </nav>
  );
}

// ── GroupSection ──────────────────────────────────────────────────────────────

/**
 * Renders either the top-level (null-named) group or a named group with a
 * collapsible header row.
 */
function GroupSection({
  group,
  collapsed,
  selectedPlanId,
  onSelect,
  onToggleCollapse,
  onRunGroup,
  runDisabledReason,
}: {
  group: PlanGroupModel;
  collapsed: boolean;
  selectedPlanId: string | null;
  onSelect(id: string): void;
  onToggleCollapse(name: string): void;
  onRunGroup(ids: string[] | null, label: string): void;
  runDisabledReason?: string | null;
}) {
  const isNamed = group.name !== null;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      {/* Named group header */}
      {isNamed && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            padding: '2px 0',
            marginTop: 4,
          }}
        >
          {/* Collapse toggle */}
          <button
            type="button"
            aria-expanded={!collapsed}
            onClick={() => onToggleCollapse(group.name!)}
            style={{
              background: 'transparent',
              border: 'none',
              padding: '0 2px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontSize: 'var(--text-xs)',
              color: 'var(--text-faint)',
              lineHeight: 1,
              flexShrink: 0,
            }}
          >
            {collapsed ? '▶' : '▼'}
          </button>

          {/* Group name */}
          <span
            style={{
              flex: 1,
              fontSize: 'var(--text-xs)',
              fontWeight: 600,
              color: 'var(--text-muted)',
              overflow: 'hidden',
              whiteSpace: 'nowrap',
              textOverflow: 'ellipsis',
            }}
          >
            {group.name}
          </span>

          {/* done/total */}
          <span
            className="tabular"
            style={{
              fontSize: 'var(--text-xs)',
              color: 'var(--text-faint)',
              flexShrink: 0,
            }}
          >
            {group.done}/{group.total}
          </span>

          {/* Run group button */}
          <button
            type="button"
            data-action="run-group"
            disabled={runDisabledReason != null}
            title={runDisabledReason ?? undefined}
            onClick={() => {
              const ids = group.rows.map((r) => r.id);
              onRunGroup(ids, `the ${ids.length} plans in ${group.name}`);
            }}
            style={{
              background: 'transparent',
              border: '1px solid var(--blur-border)',
              borderRadius: 3,
              padding: '0 4px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontSize: 'var(--text-xs)',
              color: 'var(--text-muted)',
              flexShrink: 0,
              lineHeight: '1.4',
            }}
          >
            ▶
          </button>
        </div>
      )}

      {/* Rows — hidden when collapsed */}
      {!collapsed &&
        group.rows.map((row) => (
          <PlanRow
            key={row.id}
            row={row}
            selected={row.id === selectedPlanId}
            onSelect={onSelect}
          />
        ))}
    </div>
  );
}
