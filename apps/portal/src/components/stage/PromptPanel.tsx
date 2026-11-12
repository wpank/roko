'use client';

/**
 * PromptPanel — the single field that starts plan generation or revision.
 *
 * Generate mode: POST /api/plans/generate → waitForOperation → onDone(slug)
 * Revise mode:   POST /api/plans/{id}/revise → waitForOperation → onDone(slug)
 *
 * Error handling:
 *   404/405 → feature not supported by this roko-serve build
 *   409 on revise → plan is running or already being revised
 *   Everything else → message shown inline; the typed sentence is preserved.
 */

import React, {
  useCallback,
  useRef,
  useState,
  useEffect,
  KeyboardEvent,
} from 'react';
import { useGeneratePlan, useRevisePlan, planExists, fetchOperation } from '@/api/queries';
import { ApiError } from '@/api/client';
import type { WirePlanDiff } from '@/api/contracts';
import { waitForOperation } from '@/lib/operation';
import { Key } from '@/components/kit/Key';
import { Led } from '@/components/kit/Led';
import { Notice } from '@/components/primitives/Notice';
import { isMissingRoute, unsupportedMessage, describeRequestError } from '@/lib/apiErrors';
import { clock } from '@/lib/words';

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
}: PromptPanelProps) {
  const [prompt, setPrompt] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [noticeError, setNoticeError] = useState<string | null>(null);
  const [waiting, setWaiting] = useState(false);
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const generateMutation = useGeneratePlan();
  const reviseMutation = useRevisePlan();

  // Focus textarea on mount
  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  // Esc closes the prompt, as it closes the editor — but not while a request
  // is in flight, when Cancel is disabled too.
  useEffect(() => {
    if (!onCancel || waiting) return;
    const handler = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [onCancel, waiting]);

  const handleSubmit = useCallback(async () => {
    const text = prompt.trim();
    if (!text || waiting) return;

    setError(null);
    setNoticeError(null);
    setWaiting(true);
    setElapsedMs(0);

    try {
      let accepted;

      if (mode === 'generate') {
        accepted = await generateMutation.mutateAsync({ prompt: text });
      } else {
        if (!planId) throw new Error('planId is required for revise mode');
        accepted = await reviseMutation.mutateAsync({ id: planId, feedback: text });
      }

      const { slug, diff } = await waitForOperation(
        accepted,
        {
          planExists,
          fetchOperation,
          sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
          now: () => Date.now(),
        },
        {
          expect: mode === 'generate' ? 'new-plan' : 'revision',
          onPoll: (elapsed) => {
            setElapsedMs(elapsed);
          },
        },
      );

      setWaiting(false);
      // A revision's plan diff goes to the view that shows it (3229).
      if (diff) {
        onDone(slug, diff);
      } else {
        onDone(slug);
      }
    } catch (err) {
      setWaiting(false);
      setElapsedMs(null);
      const action = mode === 'generate' ? 'generating plans' : 'revising plans';
      if (isMissingRoute(err)) {
        setNoticeError(unsupportedMessage(action));
        setError(null);
      } else if (err instanceof ApiError && err.status === 409 && mode === 'revise') {
        setError(toRevise409Error(err));
        setNoticeError(null);
      } else {
        setError(describeRequestError(err, action));
        setNoticeError(null);
      }
    }
  }, [prompt, waiting, mode, planId, generateMutation, reviseMutation, onDone]);

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
    <section className="rk-module pt-ask">
      <label className="rk-label" htmlFor={`prompt-${mode}`}>
        {question}
      </label>

      <textarea
        id={`prompt-${mode}`}
        ref={textareaRef}
        data-prompt
        rows={4}
        placeholder={placeholder}
        value={prompt}
        disabled={waiting}
        onChange={(e) => {
          setPrompt(e.target.value);
          if (error) setError(null);
          if (noticeError) setNoticeError(null);
        }}
        onKeyDown={handleKeyDown}
      />

      {mode === 'generate' && !waiting && (
        <div className="pt-examples">
          {GENERATE_EXAMPLES.map((example) => (
            <button key={example} type="button" className="pt-example" onClick={() => handleChipClick(example)}>
              {example}
            </button>
          ))}
        </div>
      )}

      {noticeError && (
        <Notice kind="unsupported" onClose={() => setNoticeError(null)}>{noticeError}</Notice>
      )}

      {error && (
        <Notice kind="error" onClose={() => setError(null)}><span data-error>{error}</span></Notice>
      )}

      <div className="pt-line">
        {waiting ? (
          <p className="pt-waiting" data-waiting>
            <Led signal="live" />
            {mode === 'generate' ? 'Writing the plan' : 'Revising the plan'}
            {elapsedMs !== null && elapsedMs > 0 && <span className="rk-data">{clock(elapsedMs)}</span>}
          </p>
        ) : (
          <p className="pt-hint">{hint}</p>
        )}
        <div className="rk-keys">
          {onCancel && (
            <Key disabled={waiting} onClick={onCancel}>
              cancel
            </Key>
          )}
          <Key
            you
            kbd="⌘↵"
            data-action="submit-prompt"
            disabled={waiting || !prompt.trim()}
            onClick={() => void handleSubmit()}
          >
            {mode === 'generate' ? 'write plan' : 'revise plan'}
          </Key>
        </div>
      </div>
    </section>
  );
}
