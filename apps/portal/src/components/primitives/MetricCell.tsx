'use client';

import React from 'react';

export interface MetricCellProps {
  value: string | number | null | undefined;
  /** Optional fixed width, e.g. "5ch" or "48px". Applied as inline style. */
  width?: string;
}

/**
 * MetricCell — right-aligned tabular cell of optional fixed width.
 *
 * Renders '·' (U+00B7) for null or undefined so the cell is never visually empty.
 * Uses the .tabular utility class from tokens.css for monospaced number rendering.
 */
export function MetricCell({ value, width }: MetricCellProps) {
  const display = value == null ? '·' : value;
  return (
    <span
      className="tabular"
      style={{
        display: 'inline-block',
        width,
        textAlign: 'right',
      }}
    >
      {display}
    </span>
  );
}
