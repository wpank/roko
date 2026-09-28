'use client';

import React from 'react';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { MetricCell } from '@/components/primitives/MetricCell';
import { middleEllipsis, compactDuration } from '@/lib/formatters';
import type { PlanRowModel } from '@/lib/planRows';

// ── PlanRow ────────────────────────────────────────────────────────────────────

/**
 * PlanRow — one row in the plan rail.
 *
 * Laid out as a CSS grid with five fixed tracks:
 *   glyph · name · count · bar · time
 *
 * Selected rows receive a rose border and a bone title; resting rows use the
 * ghost border token. Numbers use the .tabular utility class so they scan
 * straight down.
 */
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
  const shortName = middleEllipsis(baseName, 28);

  // Queue position suffix for queued plans (#1, #2, …).
  const nameLabel =
    row.state === 'queued' && row.queuePosition != null
      ? `${shortName} #${row.queuePosition}`
      : shortName;

  // ── Time cell ───────────────────────────────────────────────────────────
  let timeLabel: string;
  if (row.time.kind === 'estimate') {
    timeLabel = `~${compactDuration(row.time.ms)}`;
  } else if (row.time.kind === 'elapsed' || row.time.kind === 'actual') {
    timeLabel = compactDuration(row.time.ms);
  } else {
    timeLabel = '·';
  }

  // ── Colours ─────────────────────────────────────────────────────────────
  const borderColor = selected ? 'var(--focus-border)' : 'var(--blur-border)';
  const nameColor = selected ? 'var(--focus-title)' : undefined;

  return (
    <button
      type="button"
      data-plan-row={row.id}
      data-state={row.state}
      aria-current={selected ? 'true' : undefined}
      onClick={() => onSelect(row.id)}
      className="tabular"
      style={{
        display: 'grid',
        gridTemplateColumns: '1.25rem minmax(0, 1fr) 3.5rem 3rem 3.75rem',
        alignItems: 'center',
        gap: '0 4px',
        width: '100%',
        padding: '3px 6px',
        border: `1px solid ${borderColor}`,
        borderRadius: 3,
        background: 'transparent',
        cursor: 'pointer',
        textAlign: 'left',
        fontFamily: 'inherit',
        fontSize: 'inherit',
        color: 'var(--text-muted)',
      }}
    >
      {/* Glyph column */}
      <span style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <StatusGlyph state={row.state} />
      </span>

      {/* Name column */}
      <span
        title={baseName}
        style={{
          overflow: 'hidden',
          whiteSpace: 'nowrap',
          color: nameColor,
        }}
      >
        {nameLabel}
        {row.supersededBy != null && (
          <span
            style={{
              color: 'var(--text-faint)',
              marginLeft: '0.25em',
            }}
          >
            → {row.supersededBy}
          </span>
        )}
      </span>

      {/* Count column — done/total */}
      <span style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <MetricCell value={`${row.done}/${row.total}`} />
      </span>

      {/* Bar column */}
      <span
        aria-hidden="true"
        style={{
          display: 'block',
          height: '4px',
          borderRadius: 2,
          background: 'var(--text-ghost)',
          position: 'relative',
          overflow: 'hidden',
        }}
      >
        <span
          style={{
            display: 'block',
            position: 'absolute',
            inset: '0 auto 0 0',
            width: `${Math.round(row.fraction * 100)}%`,
            background: row.barToken,
            borderRadius: 2,
          }}
        />
      </span>

      {/* Time column */}
      <span
        style={{
          textAlign: 'right',
          whiteSpace: 'nowrap',
          color: 'var(--text-faint)',
          fontSize: 'var(--text-xs)',
        }}
      >
        {timeLabel}
      </span>
    </button>
  );
}
