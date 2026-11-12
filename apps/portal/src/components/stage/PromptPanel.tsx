'use client';

/**
 * PromptPanel — the single field that starts plan generation or revision.
 *
 * Generate mode: POST /api/plans/generate → waitForOperation → onDone(slug)
 * Revise mode:   POST /api/plans/{id}/revise → waitForOperation → onDone(slug)
 *
 * While roko writes, the request stays on screen, quoted, with TIME and SPENT on glass and a stop key (gap-6c4d24,
 * gap-f8c57a: Writing). A stopped write brings the box back with the request in it, and says what it spent.
 *
 * Error handling:
 *   404/405 → feature not supported by this roko-serve build
 *   409 on revise → plan is running or already being revised
 *   A failed write → one sentence for what failed (writeFailure), roko serve's message folded under it
 *   Everything else → message shown inline; the typed sentence is preserved.
 */

import React, {
  useCallback,
  useRef,
  useState,
  useEffect,
  KeyboardEvent,
} from 'react';
import { useGeneratePlan, useRevisePlan, useWorkspace } from '@/api/queries';
import { ApiError } from '@/api/client';
import type { WireAccepted, WirePlanDiff } from '@/api/contracts';
import { Fold } from '@/components/kit/Fold';
import { Key } from '@/components/kit/Key';
import { Notice } from '@/components/primitives/Notice';
import { Writing } from '@/components/stage/Writing';
import { isMissingRoute, unsupportedMessage, describeRequestError } from '@/lib/apiErrors';
import { stoppedSentence } from '@/lib/writing';
import { writeFailure } from '@/lib/writeFailure';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PromptPanelProps {
  mode: 'generate' | 'revise';
  planId?: string;
  /** The project's name, or (revising) the plan's title, said in the question. */
  workspace: string;
  firstRun: boolean;
  /** `diff` is a revision's plan diff, when the server sent one (3229). */
  onDone(slug: string, diff?: WirePlanDiff): void;
  onCancel?(): void;
  /** A request to start the box with: a stopped write's, opened from its row (gap-f8c57a). */
  initialPrompt?: string;
  /** What to say as the box opens: a stopped write's sentence. */
  initialNote?: string | null;
  /** Told when a write starts and when it ends, so the page keeps the box while it runs. */
  onWaitingChange?(waiting: boolean): void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Requests that suit most projects; a click puts one in the box to edit. */
const GENERATE_EXAMPLES = [
  'Add a --version flag',
  'Write tests for the main module',
  'Document how to install and run it',
];

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

/**
 * Maps a 409 revise error to a human-readable sentence, by what the server said: the plan is in a run that hasn't
 * ended ("part of an active run … before revising"), or another revision of it is in progress ("already in
 * progress"). All other errors are handled via isMissingRoute / describeRequestError.
 */
export function toRevise409Error(err: ApiError): string {
  const body = err.body;
  if (body && typeof body === 'object') {
    const b = body as Record<string, unknown>;
    const msg = typeof b['message'] === 'string' ? b['message'] : null;
    if (msg) {
      const lower = msg.toLowerCase();
      if (lower.includes('active run') || lower.includes('running')) {
        return 'This plan is part of a run that hasn’t finished; revise it when the run ends.';
      }
      if (lower.includes('already')) {
        return 'This plan is already being revised.';
      }
      return msg;
    }
  }
  return 'This plan is in a run that hasn’t finished, or already being revised.';
}

// ---------------------------------------------------------------------------
// PromptPanel
// ---------------------------------------------------------------------------

export function PromptPanel({
  mode,
  planId,
  workspace,
  firstRun,
  onDone,
  onCancel,
  initialPrompt = '',
  initialNote = null,
  onWaitingChange,
}: PromptPanelProps) {
  const [prompt, setPrompt] = useState(initialPrompt);
  const [error, setError] = useState<string | null>(null);
  // What a stopped write spent (gap-f8c57a), said above the box it came back to.
  const [note, setNote] = useState<string | null>(initialNote);
  // What roko serve said when a write failed, folded under the error's one sentence.
  const [errorRaw, setErrorRaw] = useState<string | null>(null);
  const { data: workspaceInfo } = useWorkspace();
  const [noticeError, setNoticeError] = useState<string | null>(null);
  // The write roko serve took, which the writing view follows.
  const [accepted, setAccepted] = useState<WireAccepted | null>(null);
  // The request is on its way to roko serve: the writing view shows at once.
  const [submitting, setSubmitting] = useState(false);
  const waiting = submitting || accepted !== null;

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const generateMutation = useGeneratePlan();
  const reviseMutation = useRevisePlan();

  // The box has the focus when it opens, and again when a stopped or failed write gives it back.
  useEffect(() => {
    if (!waiting) textareaRef.current?.focus();
    onWaitingChange?.(waiting);
    // Told on each change of `waiting` alone.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [waiting]);

  // Esc closes the prompt, as it closes the editor — but not while a request
  // is in flight, when Cancel is disabled too.
  // It listens before the rest of the page (capture), so Esc closes this box and nothing else (gap-6c4d24).
  useEffect(() => {
    if (!onCancel || waiting) return;
    const handler = (e: globalThis.KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.stopImmediatePropagation();
      onCancel();
    };
    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  }, [onCancel, waiting]);

  const showError = useCallback(
    (err: unknown) => {
      const action = mode === 'generate' ? 'generating plans' : 'revising plans';
      if (isMissingRoute(err)) {
        setNoticeError(unsupportedMessage(action));
        setError(null);
      } else if (err instanceof ApiError && err.status === 409 && mode === 'revise') {
        setError(toRevise409Error(err));
        setNoticeError(null);
      } else if (!(err instanceof ApiError) && err instanceof Error) {
        // The request was taken, and the write itself failed: say what failed, and fold what roko serve said.
        const failure = writeFailure(err.message, mode, workspaceInfo?.path);
        setError(failure.sentence);
        setErrorRaw(failure.raw);
        setNoticeError(null);
      } else {
        setError(describeRequestError(err, action));
        setNoticeError(null);
      }
    },
    [mode, workspaceInfo?.path],
  );

  const handleSubmit = useCallback(async () => {
    const text = prompt.trim();
    if (!text || waiting || submitting) return;

    setError(null);
    setErrorRaw(null);
    setNoticeError(null);
    setNote(null);
    setSubmitting(true);

    try {
      if (mode === 'generate') {
        setAccepted(await generateMutation.mutateAsync({ prompt: text }));
      } else {
        if (!planId) throw new Error('planId is required for revise mode');
        setAccepted(await reviseMutation.mutateAsync({ id: planId, feedback: text }));
      }
    } catch (err) {
      showError(err);
    } finally {
      setSubmitting(false);
    }
  }, [prompt, waiting, submitting, mode, planId, generateMutation, reviseMutation, showError]);

  const handleWritten = useCallback(
    (slug: string, diff?: WirePlanDiff) => {
      setAccepted(null);
      // A revision's plan diff goes to the view that shows it (3229).
      if (diff) {
        onDone(slug, diff);
      } else {
        onDone(slug);
      }
    },
    [onDone],
  );

  const handleStopped = useCallback((costUsd: number | null) => {
    setAccepted(null);
    setNote(stoppedSentence(costUsd));
  }, []);

  const handleFailed = useCallback(
    (err: unknown) => {
      setAccepted(null);
      showError(err);
    },
    [showError],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit],
  );

  const handleChipClick = useCallback((example: string) => {
    setPrompt(example);
    setError(null);
    textareaRef.current?.focus();
  }, []);

  const question =
    mode === 'generate' ? `What should roko do in ${workspace}?` : `What should change in ${workspace}?`;
  const placeholder =
    mode === 'generate' ? 'Describe a change, as you would to a colleague' : 'Say what to change, in plain words';
  const hint =
    mode === 'revise'
      ? 'roko rewrites the plan’s tasks. Nothing runs until you say so.'
      : firstRun
        ? 'Nothing runs until you’ve seen the plan.'
        : 'roko writes a plan first: small tasks, each with a check. Nothing runs until you say so.';

  return (
    <section className="rk-module pt-ask" data-region={waiting ? 'writing' : 'ask'}>
      <label className="rk-label" htmlFor={`prompt-${mode}`}>
        {question}
      </label>

      {waiting ? (
        <Writing
          mode={mode}
          workspace={workspace}
          request={prompt.trim()}
          accepted={accepted}
          onDone={handleWritten}
          onStopped={handleStopped}
          onFailed={handleFailed}
        />
      ) : (
        <>
          <textarea
            id={`prompt-${mode}`}
            ref={textareaRef}
            data-prompt
            rows={4}
            placeholder={placeholder}
            value={prompt}
            onChange={(e) => {
              setPrompt(e.target.value);
              if (error) {
                setError(null);
                setErrorRaw(null);
              }
              if (noticeError) setNoticeError(null);
              if (note) setNote(null);
            }}
            onKeyDown={handleKeyDown}
          />

          {mode === 'generate' && (
            <div className="pt-examples">
              {GENERATE_EXAMPLES.map((example) => (
                <button key={example} type="button" className="pt-example" onClick={() => handleChipClick(example)}>
                  {example}
                </button>
              ))}
            </div>
          )}
        </>
      )}

      {noticeError && (
        <Notice kind="unsupported" onClose={() => setNoticeError(null)}>{noticeError}</Notice>
      )}

      {note && !waiting && (
        <Notice kind="info" onClose={() => setNote(null)}>
          <span data-stopped>{note}</span>
        </Notice>
      )}

      {error && (
        <Notice
          kind="error"
          onClose={() => {
            setError(null);
            setErrorRaw(null);
          }}
        >
          <span data-error>{error}</span>
          {errorRaw && (
            <Fold summary="what roko said" data-fold="write-error">
              <pre className="pt-raw">{errorRaw}</pre>
            </Fold>
          )}
        </Notice>
      )}

      {!waiting && (
        <div className="pt-line">
          <p className="pt-hint">{hint}</p>
          <div className="rk-keys">
            {onCancel && <Key onClick={onCancel}>cancel</Key>}
            <Key you kbd="⌘↵" data-action="submit-prompt" disabled={!prompt.trim() || submitting} onClick={() => void handleSubmit()}>
              {mode === 'generate' ? 'write plan' : 'revise plan'}
            </Key>
          </div>
        </div>
      )}
    </section>
  );
}
