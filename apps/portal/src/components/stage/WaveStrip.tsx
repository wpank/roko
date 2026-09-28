'use client';

/**
 * WaveStrip — renders the task DAG as a flat, wrapping row of chips.
 *
 * Layout:  [W1] [chip] [chip] …  [W2] [chip] …
 *
 * Each wave is a flex group with `data-wave={index}`. Chips show a StatusGlyph
 * and the task id. Clicking a chip fires onSelectTask; the selected chip gets a
 * visible focus border.
 *
 * Concurrency hint: when maxParallel is known and below a wave's width the wave
 * label shows "(N tasks · parallel M)" — a common, invisible bottleneck.
 *
 * Diagnostics: tasks named by a cycle or dangling diagnostic render in
 * var(--state-failed) with the diagnostic message as their tooltip title. All
 * diagnostic messages are also listed below the strip.
 *
 * Deliberately NOT a node-and-edge canvas: no pan, zoom or layout engine.
 */

import React from 'react';
import { clsx } from 'clsx';
import type { WaveResult, WaveDiagnostic } from '@/lib/waves';
import type { TaskRowModel } from '@/lib/taskRows';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';

// ── Props ──────────────────────────────────────────────────────────────────────

export interface WaveStripProps {
  waves: WaveResult;
  rows: TaskRowModel[];
  maxParallel: number | null;
  selectedTaskId: string | null;
  onSelectTask(id: string): void;
}

// ── WaveStrip ──────────────────────────────────────────────────────────────────

export function WaveStrip({
  waves,
  rows,
  maxParallel,
  selectedTaskId,
  onSelectTask,
}: WaveStripProps) {
  // ── Lookups ────────────────────────────────────────────────────────────────

  // task id → TaskRowModel
  const rowById = new Map<string, TaskRowModel>();
  for (const row of rows) {
    rowById.set(row.id, row);
  }

  // task id → diagnostics that name it (cycle or dangling)
  const diagByTaskId = new Map<string, WaveDiagnostic[]>();
  for (const diag of waves.diagnostics) {
    for (const id of diag.taskIds) {
      const existing = diagByTaskId.get(id);
      if (existing) {
        existing.push(diag);
      } else {
        diagByTaskId.set(id, [diag]);
      }
    }
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col gap-2">
      {/* ── Wave groups ─────────────────────────────────────────────────── */}
      <div className="flex flex-wrap gap-x-3 gap-y-2 items-start">
        {waves.waves.map((waveIds, waveIdx) => {
          const waveWidth = waveIds.length;
          const isBottleneck =
            maxParallel !== null && maxParallel < waveWidth;

          return (
            <div
              key={waveIdx}
              data-wave={waveIdx}
              className="flex flex-wrap items-center gap-1"
            >
              {/* Wave label */}
              <span
                className="rd-meta select-none shrink-0"
                title={
                  isBottleneck
                    ? `${waveWidth} tasks · parallel ${maxParallel}`
                    : undefined
                }
              >
                W{waveIdx + 1}
                {isBottleneck && (
                  <span className="ml-1 normal-case tracking-normal">
                    {waveWidth} tasks · parallel {maxParallel}
                  </span>
                )}
              </span>

              {/* Task chips */}
              {waveIds.map((taskId) => {
                const row = rowById.get(taskId);
                const diags = diagByTaskId.get(taskId);
                const hasDiag = diags != null && diags.length > 0;
                const isSelected = taskId === selectedTaskId;
                const diagMessage = hasDiag
                  ? diags!.map((d) => d.message).join('; ')
                  : undefined;

                return (
                  <button
                    key={taskId}
                    type="button"
                    onClick={() => onSelectTask(taskId)}
                    aria-pressed={isSelected}
                    title={diagMessage ?? row?.title ?? taskId}
                    className={clsx(
                      'inline-flex items-center gap-1',
                      'font-mono text-xs leading-none',
                      'px-1.5 py-0.5',
                      'border transition-colors duration-[80ms]',
                      // Selection state: visible focus border + ring
                      isSelected
                        ? 'border-text-muted ring-1 ring-text-muted'
                        : 'border-text-ghost/40 hover:border-text-ghost',
                      // Diagnostic tasks render in failed colour
                      hasDiag
                        ? 'cursor-pointer'
                        : 'cursor-pointer',
                    )}
                    style={
                      hasDiag
                        ? { color: 'var(--state-failed)' }
                        : undefined
                    }
                  >
                    {/* Status glyph — always from the row model if present */}
                    {row != null && (
                      <StatusGlyph
                        state={row.state}
                        title={diagMessage ?? row.title}
                      />
                    )}
                    {/* Task id */}
                    <span>{taskId}</span>
                  </button>
                );
              })}
            </div>
          );
        })}
      </div>

      {/* ── Diagnostics list ─────────────────────────────────────────────── */}
      {waves.diagnostics.length > 0 && (
        <ul
          aria-label="wave diagnostics"
          className="flex flex-col gap-0.5 mt-1"
        >
          {waves.diagnostics.map((diag, idx) => (
            <li
              key={idx}
              className="font-mono text-xs leading-snug"
              style={{ color: 'var(--state-failed)' }}
            >
              {diag.message}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
