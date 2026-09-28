'use client';

import React from 'react';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { middleEllipsis, compactDuration } from '@/lib/formatters';
import type { PlanRowModel } from '@/lib/planRows';

// ── PlanRow ────────────────────────────────────────────────────────────────────

/**
 * PlanRow — one row in the plan rail.
 *
 * Layout is handled entirely by the .rd-plan-row CSS classes (T07). No inline
 * grid or layout styles are applied. The five grid tracks are:
 *   glyph · label (name + queue) · count · bar · time
 *
 * A queued row appends a data-wait line spanning columns 2–end with the reason
 * the plan is waiting (this is a direct grid child of the button).
 */

/** Format an estimate in whole minutes — never shows seconds. */
function estimateLabel(ms: number): string {
  const totalMins = Math.round(ms / 60_000);
  if (totalMins < 60) return `~${totalMins}m`;
  const hours = Math.floor(totalMins / 60);
  const mins = totalMins % 60;
  return `~${hours}h${mins}m`;
}

export function PlanRow({
  row,
  selected,
  onSelect,
}: {
  row: PlanRowModel;
  selected: boolean;
  onSelect(id: string): void;
}) {
  // ── Name label ──────────────────────────────────────────────────────────
  const baseName = row.title || row.id;
  const shortName = middleEllipsis(baseName, 22);

  // ── Time cell ───────────────────────────────────────────────────────────
  let timeLabel: string;
  if (row.time.kind === 'estimate' && row.time.ms != null) {
    timeLabel = estimateLabel(row.time.ms);
  } else if ((row.time.kind === 'elapsed' || row.time.kind === 'actual') && row.time.ms != null) {
    timeLabel = compactDuration(row.time.ms);
  } else {
    timeLabel = '·';
  }

  // ── Button title (tooltip) ───────────────────────────────────────────────
  // Includes the wait reason when the plan is queued, so the reason is
  // discoverable even when the wait line is clipped.
  const buttonTitle = row.waitReason ? `${baseName} — ${row.waitReason}` : baseName;

  const pct = Math.round(row.fraction * 100);

  return (
    <button
      type="button"
      className="rd-plan-row"
      data-plan-row={row.id}
      data-state={row.state}
      aria-current={selected ? 'true' : undefined}
      title={buttonTitle}
      onClick={() => onSelect(row.id)}
    >
      {/* Glyph column */}
      <span className="rd-plan-row__glyph">
        <StatusGlyph state={row.state} />
      </span>

      {/* Label column — name + optional queue position or superseded marker */}
      <span className="rd-plan-row__label">
        <span className="rd-plan-row__name" data-name="">
          {shortName}
        </span>
        {row.state === 'queued' && row.queuePosition != null && (
          <span className="rd-plan-row__queue" data-queue="">
            #{row.queuePosition}
          </span>
        )}
        {row.supersededBy != null && (
          <span className="rd-plan-row__queue">→ {row.supersededBy}</span>
        )}
      </span>

      {/* Count column — done/total */}
      <span className="rd-plan-row__count" data-count="">
        {row.done}/{row.total}
      </span>

      {/* Bar column */}
      <span className="rd-plan-row__bar" aria-hidden="true">
        <span style={{ width: `${pct}%`, background: row.barToken }} />
      </span>

      {/* Time column */}
      <span className="rd-plan-row__time" data-time="">
        {timeLabel}
      </span>

      {/* Wait line — queued plans only; spans label→end via grid-column in CSS */}
      {row.waitReason != null && (
        <span className="rd-plan-row__wait" data-wait="">
          {row.waitReason}
        </span>
      )}
    </button>
  );
}
