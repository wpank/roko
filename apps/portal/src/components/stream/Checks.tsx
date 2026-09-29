'use client';

import { useState } from 'react';
import type { CheckRun } from '@/lib/runState';
import { digestEnding, digestOutput, printedNothing } from '@/lib/checks';
import type { DigestGroup, DigestEntry } from '@/lib/checks';
import { buildRungs } from '@/lib/rungs';
import type { Rung } from '@/lib/rungs';
import { StatusGlyph } from '@/components/primitives/StatusGlyph';
import { glyphStateForCheck } from '@/lib/glyphs';

// ── Helpers ───────────────────────────────────────────────────────────────────

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

function CheckRow({ rung, declaredCommand }: { rung: Rung; declaredCommand: string | null }) {
  const { label, state } = rung;
  const output = rung.check?.output ?? '';
  const digest = rung.check ? digestOutput(output) : null;
  // The command the step printed first, else the authored one (unreached steps).
  const command = digest?.command ?? declaredCommand;
  const ending = state === 'failed' && digest ? digestEnding(digest) : null;

  return (
    <div
      className="check-row"
      data-status={state}
      style={{ marginBottom: '0.75em' }}
    >
      {/* Header: glyph · label · $ command */}
      <div
        className="check-row-header"
        style={{ display: 'flex', alignItems: 'baseline', gap: '0.5em' }}
      >
        <StatusGlyph
          state={glyphStateForCheck(state)}
          title={`${label}: ${state === 'pending' ? 'not reached' : state}`}
        />
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

      {/* Failed: digest first, then how it ended, then the raw output toggle */}
      {state === 'failed' && digest && (
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
          {ending && (
            <div
              className="check-row-exit"
              data-exit=""
              style={{ fontFamily: 'monospace', color: 'var(--text-strong)' }}
            >
              {ending}
            </div>
          )}
          {output && <RawOutputToggle output={output} />}
        </div>
      )}

      {/* Running: live output tail */}
      {state === 'running' && output && (
        <div style={{ paddingLeft: '1.5em' }}>
          <OutputTail output={output} />
        </div>
      )}

      {/* Passed: what it printed, behind the same toggle a failure uses */}
      {state === 'passed' && digest && !printedNothing(digest) && (
        <div style={{ paddingLeft: '1.5em' }}>
          <RawOutputToggle output={output} />
        </div>
      )}
    </div>
  );
}

// ── Checks ─────────────────────────────────────────────────────────────────────

/**
 * Checks — a task's verify steps in `verify[i:phase]` order (design §5).
 *
 * - `declared` is the task's authored verify list: a step not reached yet
 *   shows `·` and its command. Steps outside the list follow it.
 * - Failed step: the digest leads, then how the step ended; raw output is
 *   behind a toggle.
 * - Passed step: its output is behind the same toggle.
 * - Running step: live output tail is shown.
 * - No step declared or run: a single "no verify step has run" line.
 */
export function Checks({
  checks,
  declared = null,
}: {
  checks: CheckRun[];
  declared?: readonly { phase: string; command: string }[] | null;
}) {
  const rungs = buildRungs(checks, declared);
  return (
    <div data-region="checks" className="checks-region rd-stream-body">
      {rungs.length === 0 ? (
        <div className="checks-empty" style={{ color: 'var(--text-faint)' }}>
          no verify step has run
        </div>
      ) : (
        rungs.map((rung, i) => (
          <CheckRow
            key={rung.check?.name ?? `verify-${i}`}
            rung={rung}
            declaredCommand={(rung.index !== null && declared?.[rung.index]?.command) || null}
          />
        ))
      )}
    </div>
  );
}
