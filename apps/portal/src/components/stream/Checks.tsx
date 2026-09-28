'use client';

import { useState } from 'react';
import type { CheckRun } from '@/lib/runState';
import { digestOutput } from '@/lib/checks';
import type { DigestGroup, DigestEntry } from '@/lib/checks';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import type { GlyphState } from '@/lib/glyphs';

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Map a CheckRun status to a GlyphState. */
function glyphForCheck(status: CheckRun['status']): GlyphState {
  switch (status) {
    case 'passed':  return 'done';
    case 'failed':  return 'failed';
    case 'running': return 'active';
  }
}

/** Format a location prefix: `line:col` or just `line` when col is null. */
function loc(line: number | null, col: number | null): string {
  if (line === null) return '?';
  if (col === null)  return String(line);
  return `${line}:${col}`;
}

// ── DigestGroupView ────────────────────────────────────────────────────────────

function DigestEntryRow({ entry }: { entry: DigestEntry }) {
  const railColor =
    entry.severity === 'error'
      ? 'var(--state-failed)'
      : 'var(--state-accepted)';

  return (
    <div
      className="check-digest-entry"
      style={{
        display: 'flex',
        alignItems: 'baseline',
        gap: '0.5em',
        paddingLeft: '0.75em',
        borderLeft: `2px solid ${railColor}`,
        marginBottom: '1px',
      }}
    >
      <span
        className="check-digest-loc"
        style={{ fontFamily: 'monospace', color: 'var(--text-muted)', whiteSpace: 'nowrap', minWidth: '5em' }}
      >
        {loc(entry.line, entry.column)}
      </span>
      <span className="check-digest-message" style={{ flex: 1 }}>
        {entry.message}
      </span>
      {entry.count > 1 && (
        <span
          className="check-digest-count"
          style={{ color: 'var(--text-faint)', whiteSpace: 'nowrap', fontFamily: 'monospace' }}
        >
          ×{entry.count}
        </span>
      )}
    </div>
  );
}

function DigestGroupView({ group }: { group: DigestGroup }) {
  return (
    <div className="check-digest-group" style={{ marginBottom: '0.5em' }}>
      <div
        className="check-digest-file"
        style={{ fontFamily: 'monospace', color: 'var(--text-muted)', marginBottom: '2px', fontSize: 'var(--type-meta)' }}
      >
        {group.file}
      </div>
      {group.entries.map((entry, i) => (
        <DigestEntryRow key={i} entry={entry} />
      ))}
    </div>
  );
}

// ── RawOutputToggle ────────────────────────────────────────────────────────────

function RawOutputToggle({ output }: { output: string }) {
  const [open, setOpen] = useState(false);
  return (
    <details
      className="check-raw-toggle"
      open={open}
      onToggle={(e) => setOpen((e.currentTarget as HTMLDetailsElement).open)}
      style={{ marginTop: '0.5em' }}
    >
      <summary
        style={{ cursor: 'pointer', color: 'var(--text-faint)', userSelect: 'none', fontSize: 'var(--type-meta)' }}
      >
        raw output
      </summary>
      <pre
        className="check-raw-output"
        style={{
          whiteSpace: 'pre-wrap',
          fontFamily: 'monospace',
          fontSize: 'var(--type-meta)',
          margin: '4px 0 0 0',
          color: 'var(--text-muted)',
          maxHeight: '20em',
          overflowY: 'auto',
        }}
      >
        {output}
      </pre>
    </details>
  );
}

// ── OutputTail ─────────────────────────────────────────────────────────────────

/** Shows the last N lines of live gate output for a running step. */
function OutputTail({ output }: { output: string }) {
  const TAIL_LINES = 8;
  const lines = output.trimEnd().split('\n');
  const tail = lines.length > TAIL_LINES ? lines.slice(-TAIL_LINES) : lines;
  if (!output.trim()) return null;
  return (
    <pre
      className="check-output-tail"
      style={{
        whiteSpace: 'pre-wrap',
        fontFamily: 'monospace',
        fontSize: 'var(--type-meta)',
        margin: '4px 0 0 0',
        color: 'var(--text-muted)',
      }}
    >
      {lines.length > TAIL_LINES && (
        <span style={{ color: 'var(--text-faint)' }}>…{'\n'}</span>
      )}
      {tail.join('\n')}
    </pre>
  );
}

// ── CheckRow ───────────────────────────────────────────────────────────────────

function CheckRow({ check }: { check: CheckRun }) {
  const glyph = glyphForCheck(check.status);
  // Display the phase when it is non-empty, otherwise fall back to full name.
  const label = check.phase || check.name;

  // Extract the command from the digest (or raw output prefix).
  const digest = check.status === 'failed' ? digestOutput(check.output) : null;
  const command = digest?.command ?? extractCommand(check.output);

  return (
    <div
      className="check-row"
      data-status={check.status}
      style={{ marginBottom: '0.75em' }}
    >
      {/* Header: glyph · label · $ command */}
      <div
        className="check-row-header"
        style={{ display: 'flex', alignItems: 'baseline', gap: '0.5em' }}
      >
        <StatusGlyph state={glyph} title={`${label}: ${check.status}`} />
        <span className="check-row-label">{label}</span>
        {command && (
          <span
            className="check-row-command"
            style={{ fontFamily: 'monospace', color: 'var(--text-faint)', fontSize: 'var(--type-meta)' }}
          >
            $ {command}
          </span>
        )}
      </div>

      {/* Failed: digest first, then raw output toggle */}
      {check.status === 'failed' && digest && (
        <div className="check-row-digest" style={{ marginTop: '0.4em', paddingLeft: '1.5em' }}>
          {digest.groups.map((group, i) => (
            <DigestGroupView key={i} group={group} />
          ))}
          {digest.unparsed.length > 0 && (
            <pre
              className="check-digest-unparsed"
              style={{
                whiteSpace: 'pre-wrap',
                fontFamily: 'monospace',
                fontSize: 'var(--type-meta)',
                margin: '0.25em 0',
                color: 'var(--text-faint)',
              }}
            >
              {digest.unparsed.join('\n')}
            </pre>
          )}
          {check.output && <RawOutputToggle output={check.output} />}
        </div>
      )}

      {/* Running: live output tail */}
      {check.status === 'running' && check.output && (
        <div style={{ paddingLeft: '1.5em' }}>
          <OutputTail output={check.output} />
        </div>
      )}

      {/* Passed: output collapsed — nothing shown */}
    </div>
  );
}

/** Extract a `$ command` from the first line of raw output, if present. */
function extractCommand(output: string): string | null {
  if (!output) return null;
  const first = output.split('\n')[0] ?? '';
  const m = /^\$ (.+)$/.exec(first);
  return m ? m[1]! : null;
}

// ── Checks ─────────────────────────────────────────────────────────────────────

/**
 * Checks — lists all gate verify steps for a task, in index order.
 *
 * - CheckRun arrives pre-sorted by index from runState.
 * - Failed step: digestOutput digest leads, raw output is behind a toggle.
 * - Running step: live output tail is shown.
 * - Passed step: output is collapsed (nothing shown).
 * - No checks yet: a single "no verify step has run" line.
 */
export function Checks({ checks }: { checks: CheckRun[] }) {
  return (
    <div data-region="checks" className="checks-region">
      {checks.length === 0 ? (
        <div className="checks-empty" style={{ color: 'var(--text-faint)' }}>
          no verify step has run
        </div>
      ) : (
        checks.map((check) => (
          <CheckRow key={check.name} check={check} />
        ))
      )}
    </div>
  );
}
