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
import { waitForOperation } from '@/lib/operation';
import { compactDuration } from '@/lib/formatters';
import { Button } from '@/components/atoms/Button';
import { Notice } from '@/components/primitives/Notice';
import { Spinner } from '@/components/atoms/Spinner';
import { isMissingRoute, unsupportedMessage, describeRequestError } from '@/lib/apiErrors';
import { cn } from '@/lib/cn';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PromptPanelProps {
  mode: 'generate' | 'revise';
  planId?: string;
  workspace: string;
  firstRun: boolean;
  onDone(slug: string): void;
  onCancel?(): void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const GENERATE_EXAMPLES = [
  'a rust app that prints hello world',
  'a CLI tool that watches a directory and reports changes',
  'a REST API with authentication and a SQLite database',
  'a web scraper that extracts structured data from a URL',
];

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

/**
 * Maps a 409 revise error to a human-readable sentence.
 * All other errors are handled via isMissingRoute / describeRequestError.
 */
function toRevise409Error(err: ApiError): string {
  const body = err.body;
  if (body && typeof body === 'object') {
    const b = body as Record<string, unknown>;
    const msg = typeof b['message'] === 'string' ? b['message'] : null;
    if (msg) {
      const lower = msg.toLowerCase();
      if (lower.includes('running')) {
        return 'This plan is currently running and cannot be revised right now.';
      }
      if (lower.includes('alread') || lower.includes('revis')) {
        return 'This plan is already being revised.';
      }
      return msg;
    }
  }
  return 'This plan is currently running or already being revised.';
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

      const { slug } = await waitForOperation(
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
      onDone(slug);
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

  const placeholder =
    mode === 'generate' ? 'Describe what you want to build' : 'What should change?';

  const waitingLabel =
    mode === 'generate' ? 'Generating plan…' : 'Revising plan…';

  return (
    <div className="flex flex-col gap-4">
      {/* First-run heading */}
      {firstRun && mode === 'generate' && (
        <p className="text-sm font-mono text-text-muted">
          No plans in <span className="text-text-strong">{workspace}</span> yet.{' '}
          Describe what you want to build.
        </p>
      )}

      {/* Textarea */}
      <textarea
        ref={textareaRef}
        data-prompt
        rows={4}
        className={cn(
          'w-full resize-none rounded',
          'border border-border-default bg-bg-highlight',
          'px-3 py-2 font-mono text-sm text-text-strong',
          'placeholder:text-text-ghost',
          'focus:outline-none focus:border-border-hover',
          'disabled:cursor-not-allowed disabled:opacity-50',
        )}
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

      {/* Example chips — generate mode only */}
      {mode === 'generate' && !waiting && (
        <div className="flex flex-wrap gap-2">
          {GENERATE_EXAMPLES.map((example) => (
            <button
              key={example}
              type="button"
              onClick={() => handleChipClick(example)}
              className={cn(
                'rounded font-mono text-xs px-2 py-1',
                'border border-border-default text-text-muted',
                'hover:border-border-hover hover:text-text-strong',
                'transition-[border-color,color] duration-[80ms]',
              )}
            >
              {example}
            </button>
          ))}
        </div>
      )}

      {/* Status row: waiting indicator */}
      {waiting && (
        <div className="flex items-center gap-2 text-sm font-mono text-text-muted">
          <Spinner size="sm" />
          <span>{waitingLabel}</span>
          {elapsedMs !== null && elapsedMs > 0 && (
            <span className="tabular-nums">{compactDuration(elapsedMs)}</span>
          )}
        </div>
      )}

      {/* Missing-route notice */}
      {noticeError && (
        <Notice kind="unsupported">{noticeError}</Notice>
      )}

      {/* Inline error (409, outdated server, network, etc.) */}
      {error && (
        <p data-error className="text-xs font-mono text-accent-error">{error}</p>
      )}

      {/* Action row */}
      <div className="flex items-center gap-2">
        <Button
          data-action="submit-prompt"
          variant="primary"
          disabled={waiting || !prompt.trim()}
          onClick={() => void handleSubmit()}
        >
          {mode === 'generate' ? 'Generate plan' : 'Revise plan'}
        </Button>

        {onCancel && (
          <Button
            variant="ghost"
            disabled={waiting}
            onClick={onCancel}
          >
            Cancel
          </Button>
        )}

        <span className="ml-auto text-xs font-mono text-text-ghost select-none hidden sm:inline">
          ⌘↵ to submit
        </span>
      </div>
    </div>
  );
}
