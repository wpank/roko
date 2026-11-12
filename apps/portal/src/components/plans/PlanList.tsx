'use client';

/**
 * PlanList — the workspace's plans, the only list in the portal.
 *
 * Each row: a light, the plan's title, one status line in roko's words (lib/planWords.ts) and a square per task.
 * `new plan` sits at the top; `run all` at the foot once there are two plans; the filter once there are more than
 * FILTER_FROM. Groups (plan folders) collapse, and the collapsed set is kept in this browser.
 */

import React, { useCallback, useState } from 'react';
import { Key } from '@/components/kit/Key';
import { Led } from '@/components/kit/Led';
import { Strip } from '@/components/kit/Strip';
import type { PlanGroupModel, PlanRowModel, PlanRowsResult } from '@/lib/planRows';
import { planWords } from '@/lib/planWords';

/** Past this many plans the list gets a filter. */
export const FILTER_FROM = 8;

const COLLAPSED_KEY = 'roko.rail.collapsed';

function readCollapsed(): Set<string> {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? '[]');
    if (Array.isArray(parsed)) return new Set(parsed.filter((v): v is string => typeof v === 'string'));
  } catch {
    // Storage off or unreadable: start with every group open.
  }
  return new Set();
}

function writeCollapsed(set: Set<string>): void {
  try {
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...set]));
  } catch {
    // Storage off: the choice lasts for this page only.
  }
}

export function PlanList({
  result,
  costs,
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
  /** What each plan's latest run spent, by plan id. */
  costs: Readonly<Record<string, number>>;
  selectedPlanId: string | null;
  filter: string;
  onFilterChange(value: string): void;
  filterInputRef: React.RefObject<HTMLInputElement | null>;
  onSelect(id: string): void;
  onNewPlan(): void;
  onRunPlans(ids: string[] | null, label: string): void;
  emptySentence: string;
  /** When set, run all and every group's run are off, and this says why. */
  runDisabledReason?: string | null;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(() => readCollapsed());
  const toggle = useCallback((name: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      writeCollapsed(next);
      return next;
    });
  }, []);

  const { groups, count } = result;
  const showFilter = count > FILTER_FROM || filter !== '';

  return (
    <nav data-region="rail" className="pt-plans" aria-label="Plans">
      <div className="pt-plans-head">
        <span className="rk-label">plans · {count}</span>
        <Key size="s" kbd="n" data-action="new-plan" onClick={onNewPlan}>
          new plan
        </Key>
      </div>

      {showFilter && (
        <input
          ref={filterInputRef}
          type="search"
          className="pt-filter"
          value={filter}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="Filter plans"
          aria-label="Filter plans"
        />
      )}

      {groups.length === 0 ? (
        <p className="pt-plans-empty">{emptySentence}</p>
      ) : (
        groups.map((group) => (
          <PlanGroup
            key={group.name ?? '__top__'}
            group={group}
            costs={costs}
            collapsed={group.name !== null && collapsed.has(group.name)}
            selectedPlanId={selectedPlanId}
            onSelect={onSelect}
            onToggle={toggle}
            onRunGroup={onRunPlans}
            runDisabledReason={runDisabledReason}
          />
        ))
      )}

      {count >= 2 && (
        <div className="pt-plans-foot">
          <Key
            size="s"
            data-action="run-all"
            disabled={runDisabledReason != null}
            title={runDisabledReason ?? `Run all ${count} plans, in the order their dependencies need`}
            onClick={() => onRunPlans(null, `all ${count} plans`)}
          >
            run all
          </Key>
        </div>
      )}
    </nav>
  );
}

function PlanGroup({
  group,
  costs,
  collapsed,
  selectedPlanId,
  onSelect,
  onToggle,
  onRunGroup,
  runDisabledReason,
}: {
  group: PlanGroupModel;
  costs: Readonly<Record<string, number>>;
  collapsed: boolean;
  selectedPlanId: string | null;
  onSelect(id: string): void;
  onToggle(name: string): void;
  onRunGroup(ids: string[] | null, label: string): void;
  runDisabledReason?: string | null;
}) {
  return (
    <>
      {group.name !== null && (
        <div className="pt-group" data-group={group.name}>
          <button
            type="button"
            className="pt-group-toggle"
            aria-expanded={!collapsed}
            onClick={() => onToggle(group.name!)}
          >
            <span className="pt-group-mark" aria-hidden="true">
              {collapsed ? '▸' : '▾'}
            </span>
            <span className="rk-label">
              {group.name} · {group.done}/{group.total}
            </span>
          </button>
          <Key
            size="s"
            data-action="run-group"
            disabled={runDisabledReason != null}
            title={runDisabledReason ?? `Run the ${group.ids.length} plans in ${group.name}`}
            onClick={() => onRunGroup(group.ids, `the ${group.ids.length} plans in ${group.name}`)}
          >
            run
          </Key>
        </div>
      )}
      {!collapsed &&
        group.rows.map((row) => (
          <PlanRow
            key={row.id}
            row={row}
            costUsd={costs[row.id] ?? 0}
            selected={row.id === selectedPlanId}
            onSelect={onSelect}
          />
        ))}
    </>
  );
}

function PlanRow({
  row,
  costUsd,
  selected,
  onSelect,
}: {
  row: PlanRowModel;
  costUsd: number;
  selected: boolean;
  onSelect(id: string): void;
}) {
  const words = planWords(row, costUsd);
  const title = row.title || row.id;
  return (
    <button
      type="button"
      className="pt-plan"
      data-plan-row={row.id}
      data-state={row.state}
      data-signal={words.signal}
      aria-current={selected ? 'true' : undefined}
      title={row.waitReason ? `${title}: ${row.waitReason}` : title}
      onClick={() => onSelect(row.id)}
    >
      <Led signal={words.signal} label={words.word} />
      <span className="pt-plan-title">{title}</span>
      <span className="pt-plan-line">
        <span>
          <span className="pt-plan-word">{words.word}</span>
          {words.rest && ` · ${words.rest}`}
        </span>
        {row.total > 0 && <Strip squares={words.squares} label={`${row.done} of ${row.total} tasks done`} />}
      </span>
    </button>
  );
}
