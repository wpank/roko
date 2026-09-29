'use client';

/**
 * SourceEditor — Raw TOML editor for a plan file.
 *
 * Loads the plan's raw source via `usePlanSource`, presents it in a monospace
 * textarea, and saves back via `useSavePlanSource`.  The server is the source
 * of truth: we do not parse, rewrite, or validate TOML in the browser.
 *
 * Error handling:
 *  200  → success: onClose() (mutation already invalidated tasks/source/validation)
 *  409  → plan is running; surface a clear message, keep the editor open
 *  422  → validation failure; list diagnostics (rule id, task id links), move
 *         the caret to the first diagnostic's task
 *  404/405 on load or save → feature not supported by this roko-serve build
 */

import React, {
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { usePlanSource, useSavePlanSource } from '@/api/queries';
import { ApiError } from '@/api/client';
import type { WireDiagnostic } from '@/api/contracts';
import { Button } from '@/components/atoms/Button';
import { Spinner } from '@/components/atoms/Spinner';
import { Notice } from '@/components/primitives/Notice';
import { describeRequestError, isMissingRoute, unsupportedMessage } from '@/lib/apiErrors';
import { cn } from '@/lib/cn';
import { confirmDiscard } from '@/stores/dashboard';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SourceEditorProps {
  planId: string;
  running: boolean;
  onClose(): void;
  /** Called whenever the editor's dirty state (text ≠ loaded) changes, and with false on unmount. */
  onDirtyChange?(dirty: boolean): void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Extract diagnostics from an ApiError body regardless of whether they are
 * nested under `body.details.diagnostics` or directly at `body.diagnostics`.
 */
function extractDiagnostics(body: unknown): WireDiagnostic[] {
  if (!body || typeof body !== 'object') return [];
  const b = body as Record<string, unknown>;
  const details = b['details'];
  if (details && typeof details === 'object') {
    const nested = (details as Record<string, unknown>)['diagnostics'];
    if (Array.isArray(nested)) return nested as WireDiagnostic[];
  }
  if (Array.isArray(b['diagnostics'])) return b['diagnostics'] as WireDiagnostic[];
  return [];
}

/** A table header line: `[name]` or `[[name]]`, optionally followed by a comment. */
const TABLE_HEADER = /^\s*\[\[?[^[\]]+\]\]?\s*(#.*)?$/;
/** The header of one task: `[[task]]`. */
const TASK_HEADER = /^\s*\[\[\s*task\s*\]\]\s*(#.*)?$/;
/** A task's own `id` key, in either quote style, with any spacing. */
const ID_KEY = /^\s*(?:id|"id"|'id')\s*=\s*(["'])(.*?)\1/;

/**
 * Find the offset in `toml` of the `[[task]]` line of the task whose `id` is
 * `taskId`. Only an `id` key in the task's own table counts, not one in a
 * sub-table such as `[task.context]`. Returns null when no task has that id.
 */
export function findTaskLine(toml: string, taskId: string): number | null {
  let offset = 0;
  let header: number | null = null;
  for (const line of toml.split('\n')) {
    const start = offset;
    offset += line.length + 1;
    if (TABLE_HEADER.test(line)) {
      header = TASK_HEADER.test(line) ? start : null;
      continue;
    }
    const id = header !== null ? ID_KEY.exec(line) : null;
    if (id && id[2] === taskId) return header;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SourceEditor({ planId, running, onClose, onDirtyChange }: SourceEditorProps) {
  // ------------------------------------------------------------------
  // Remote data
  // ------------------------------------------------------------------

  const {
    data,
    isLoading,
    error: loadError,
  } = usePlanSource(planId, true);

  const saveMutation = useSavePlanSource();

  // ------------------------------------------------------------------
  // Local edit state
  // ------------------------------------------------------------------

  /**
   * `null` = not yet initialised (source still loading).
   * Once set, all user keystrokes keep updating this value.
   */
  const [text, setText] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<WireDiagnostic[]>([]);
  const [inlineError, setInlineError] = useState<string | null>(null);
  /** Set when a save attempt returns 404/405 — shown as a Notice above the editor. */
  const [saveNotice, setSaveNotice] = useState<string | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Seed textarea once the first server payload arrives.
  useEffect(() => {
    if (data?.toml !== undefined && text === null) {
      setText(data.toml);
    }
  }, [data, text]);

  // `dirty` is true when the current edit differs from the last loaded value.
  const dirty =
    text !== null && data !== undefined && text !== data.toml;

  // Report dirty-state changes to the parent.
  const prevDirtyRef = useRef<boolean>(false);
  useEffect(() => {
    if (dirty !== prevDirtyRef.current) {
      prevDirtyRef.current = dirty;
      onDirtyChange?.(dirty);
    }
  }, [dirty, onDirtyChange]);

  // Always report clean when the editor unmounts.
  useEffect(() => {
    return () => { onDirtyChange?.(false); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ------------------------------------------------------------------
  // Close/cancel helpers
  // ------------------------------------------------------------------

  const confirmClose = useCallback(() => {
    if (confirmDiscard(dirty)) onClose();
  }, [dirty, onClose]);

  // Esc closes (with discard prompt when dirty).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') confirmClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [confirmClose]);

  // ------------------------------------------------------------------
  // Caret positioning
  // ------------------------------------------------------------------

  /**
   * Move the textarea caret to the start of the `[[task]]` line of `taskId`
   * and scroll that line into view. Does nothing when the text has no such
   * task (for example, after the author renamed it).
   */
  const jumpToTask = useCallback(
    (taskId: string, currentText: string) => {
      const el = textareaRef.current;
      const at = findTaskLine(currentText, taskId);
      if (!el || at === null) return;
      el.focus();
      el.setSelectionRange(at, at);
      // Scroll the matching line into the visible portion of the textarea.
      const linesAbove = currentText.slice(0, at).split('\n').length - 1;
      const lineHeight =
        parseFloat(getComputedStyle(el).lineHeight) || 20;
      el.scrollTop = Math.max(
        0,
        linesAbove * lineHeight - el.clientHeight / 2,
      );
    },
    [],
  );

  // ------------------------------------------------------------------
  // Save handler
  // ------------------------------------------------------------------

  const handleSave = useCallback(() => {
    if (text === null) return;
    // Clear previous feedback before each save attempt.
    setDiagnostics([]);
    setInlineError(null);
    setSaveNotice(null);
    saveMutation.mutate(
      { id: planId, toml: text },
      {
        onSuccess: () => {
          // Mutation already invalidated tasks, source and validation.
          onClose();
        },
        onError: (err) => {
          if (err.status === 409) {
            setInlineError('The plan is running; edits would be ignored.');
            return;
          }
          if (err.status === 404 || err.status === 405) {
            setSaveNotice(unsupportedMessage('editing plans'));
            return;
          }
          if (err.status === 422) {
            const diags = extractDiagnostics(err.body);
            setDiagnostics(diags);
            // Defer caret move until after React renders the diagnostics.
            const firstTask = diags.find((d) => d.task_id)?.task_id;
            if (firstTask) setTimeout(() => jumpToTask(firstTask, text), 0);
            return;
          }
          setInlineError(`Save failed: ${err.message}`);
        },
      },
    );
  }, [planId, text, saveMutation, onClose, jumpToTask]);

  // ------------------------------------------------------------------
  // Load error states
  // ------------------------------------------------------------------

  const loadIsUnsupported = isMissingRoute(loadError);

  if (isLoading) {
    return (
      <div className="flex flex-col flex-1 items-center justify-center gap-2 text-text-muted">
        <Spinner />
        <span className="text-xs font-mono">Loading source…</span>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex flex-col flex-1 p-4 gap-4">
        <Notice
          kind={loadIsUnsupported ? 'unsupported' : 'error'}
          onClose={onClose}
        >
          {describeRequestError(loadError, 'editing plans')}
        </Notice>
      </div>
    );
  }

  // ------------------------------------------------------------------
  // Render
  // ------------------------------------------------------------------

  return (
    <div className="flex flex-col flex-1">
      {/* ---- Toolbar ---- */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-border-default shrink-0">
        {/* Plan id + dirty indicator */}
        <span className="text-xs font-mono text-text-muted flex-1 truncate">
          {planId}
          {dirty && (
            <span className="ml-1 text-accent-warn" aria-label="unsaved changes">
              ●
            </span>
          )}
        </span>

        {/* Running badge */}
        {running && (
          <span className="text-xs font-mono text-accent-warn shrink-0">
            Read-only while plan is running
          </span>
        )}

        <Button
          variant="ghost"
          size="sm"
          onClick={confirmClose}
          disabled={saveMutation.isPending}
        >
          Cancel
        </Button>

        <Button
          data-action="save-source"
          variant="primary"
          size="sm"
          onClick={handleSave}
          loading={saveMutation.isPending}
          disabled={running || saveMutation.isPending}
        >
          Save
        </Button>
      </div>

      {/* ---- Save notice (404/405 — route not supported by this server) ---- */}
      {saveNotice && (
        <div className="px-3 py-2 border-b border-border-default shrink-0">
          <Notice kind="unsupported" onClose={() => setSaveNotice(null)}>{saveNotice}</Notice>
        </div>
      )}

      {/* ---- Inline error (409, network) ---- */}
      {inlineError && (
        <div className="px-3 py-2 border-b border-border-default shrink-0">
          <Notice kind="error" onClose={() => setInlineError(null)}>{inlineError}</Notice>
        </div>
      )}

      {/* ---- Validation diagnostics (422): severity, rule id, task link, message ---- */}
      {diagnostics.length > 0 && (
        <div
          data-region="diagnostics"
          className="px-3 py-2 border-b border-border-default shrink-0 max-h-36 overflow-y-auto space-y-1"
        >
          {diagnostics.map((d, i) => (
            <div
              key={i}
              data-diagnostic
              className={cn(
                'text-xs font-mono',
                d.severity === 'error'
                  ? 'text-accent-error'
                  : 'text-accent-warn',
              )}
            >
              <span className="uppercase font-semibold">{d.severity}</span>
              {' '}
              <span data-rule-id className="text-text-muted">{d.rule_id}</span>
              {d.task_id && (
                <>
                  {' '}
                  {/* Jumps to the task's [[task]] line in the text below. */}
                  <button
                    type="button"
                    data-task-link={d.task_id}
                    title={`Go to ${d.task_id} in the source`}
                    onClick={() => {
                      if (text !== null) jumpToTask(d.task_id!, text);
                    }}
                    className="font-mono text-xs leading-none px-1.5 py-0.5 border border-text-ghost/40 hover:border-text-ghost text-inherit"
                  >
                    {d.task_id}
                  </button>
                </>
              )}
              {': '}
              {d.message}
            </div>
          ))}
        </div>
      )}

      {/* ---- Editor: the rest of the stage, never under 8rem ---- */}
      <div className="relative flex-1 min-h-32">
        {running && (
          <div
            className="absolute inset-0 z-10 pointer-events-none flex items-start justify-center pt-4"
            aria-hidden="true"
          >
            <span className="text-xs font-mono text-accent-warn bg-bg-base/80 px-2 py-1 rounded">
              Read-only: plan is running
            </span>
          </div>
        )}

        <textarea
          ref={textareaRef}
          data-editor
          spellCheck={false}
          readOnly={running}
          value={text ?? ''}
          onChange={e => {
            setText(e.target.value);
            // Clear stale diagnostics/notices when the user edits.
            if (diagnostics.length > 0) setDiagnostics([]);
            if (inlineError) setInlineError(null);
            if (saveNotice) setSaveNotice(null);
          }}
          className={cn(
            'absolute inset-0 w-full h-full resize-none',
            'outline-none border-0',
            'font-mono text-sm leading-relaxed',
            'bg-transparent text-text-strong p-3',
            running && 'opacity-60 cursor-not-allowed',
          )}
          aria-label="Plan source TOML"
          aria-readonly={running}
        />
      </div>
    </div>
  );
}
