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
 *  422  → validation failure; list diagnostics, move caret to first task_id hit
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
import { isMissingRoute, unsupportedMessage } from '@/lib/apiErrors';
import { cn } from '@/lib/cn';

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
    if (dirty) {
      if (!window.confirm('Discard unsaved edits?')) return;
    }
    onClose();
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
   * After a 422, find the first diagnostic with a task_id and move the
   * textarea caret to the `id = "<task_id>"` line so the author can see it.
   */
  const moveCaret = useCallback(
    (diags: WireDiagnostic[], currentText: string) => {
      const taskDiag = diags.find(d => d.task_id);
      if (!taskDiag?.task_id) return;
      const el = textareaRef.current;
      if (!el) return;
      const target = `id = "${taskDiag.task_id}"`;
      const idx = currentText.indexOf(target);
      if (idx === -1) return;
      el.focus();
      el.setSelectionRange(idx, idx + target.length);
      // Scroll the matching line into the visible portion of the textarea.
      const linesAbove = currentText.slice(0, idx).split('\n').length - 1;
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
            setTimeout(() => moveCaret(diags, text), 0);
            return;
          }
          setInlineError(`Save failed: ${err.message}`);
        },
      },
    );
  }, [planId, text, saveMutation, onClose, moveCaret]);

  // ------------------------------------------------------------------
  // Load error states
  // ------------------------------------------------------------------

  const loadIsUnsupported = isMissingRoute(loadError);

  if (isLoading) {
    return (
      <div className="flex flex-col h-full items-center justify-center gap-2 text-text-muted">
        <Spinner />
        <span className="text-xs font-mono">Loading source…</span>
      </div>
    );
  }

  if (loadIsUnsupported) {
    return (
      <div className="flex flex-col h-full p-4 gap-4">
        <Notice kind="unsupported">
          {unsupportedMessage('editing plans')}
        </Notice>
        <Button variant="secondary" size="sm" onClick={onClose}>
          Close
        </Button>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex flex-col h-full items-center justify-center gap-4 text-accent-error">
        <span className="text-sm font-mono">
          Failed to load source: {String(loadError)}
        </span>
        <Button variant="secondary" size="sm" onClick={onClose}>
          Close
        </Button>
      </div>
    );
  }

  // ------------------------------------------------------------------
  // Render
  // ------------------------------------------------------------------

  return (
    <div className="flex flex-col h-full">
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
          <Notice kind="unsupported">{saveNotice}</Notice>
        </div>
      )}

      {/* ---- Inline error (409, network) ---- */}
      {inlineError && (
        <div className="px-3 py-2 text-xs font-mono text-accent-error bg-bg-highlight border-b border-border-default shrink-0">
          {inlineError}
        </div>
      )}

      {/* ---- Validation diagnostics (422) ---- */}
      {diagnostics.length > 0 && (
        <div className="px-3 py-2 border-b border-border-default shrink-0 max-h-36 overflow-y-auto space-y-1">
          {diagnostics.map((d, i) => (
            <div
              key={i}
              className={cn(
                'text-xs font-mono',
                d.severity === 'error'
                  ? 'text-accent-error'
                  : 'text-accent-warn',
              )}
            >
              <span className="uppercase font-semibold">{d.severity}</span>
              {': '}
              {d.message}
              {d.task_id && (
                <span className="ml-1 text-text-ghost">
                  [{d.task_id}]
                </span>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ---- Editor ---- */}
      <div className="relative flex-1 min-h-0">
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
            'w-full h-full resize-none',
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
