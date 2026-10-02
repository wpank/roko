'use client';

/**
 * ReviewPane — a task whose verified attempt a Graph run holds for review
 * (`[meta] approval = "per_task"`): the attempt's files and patch, Approve, and
 * Reject with a note for the next attempt. The run waiting on the attempt
 * reads the decision, then merges it or fails it.
 */

import React, { useState } from 'react';
import { useSubmitReview, useTaskDiff } from '@/api/queries';
import { cn } from '@/lib/cn';

export interface ReviewPaneProps {
  planId: string;
  taskId: string;
  /** Called once the decision is recorded. */
  onDone?(): void;
}

const BUTTON = cn(
  'inline-flex items-center rounded px-2 py-1 text-xs font-mono border border-border-default',
  'text-text-muted hover:text-text-strong hover:border-border-hover disabled:opacity-50',
);

export function ReviewPane({ planId, taskId, onDone }: ReviewPaneProps) {
  const { data: diff, error } = useTaskDiff(planId, taskId);
  const submit = useSubmitReview();
  const [note, setNote] = useState('');

  const decide = (decision: 'approve' | 'reject') =>
    submit.mutate(
      decision === 'approve'
        ? { id: planId, taskId, decision }
        : { id: planId, taskId, decision, comment: note },
      { onSuccess: () => onDone?.() },
    );

  return (
    <div
      data-region="review"
      className="mt-2 pl-6 flex flex-col gap-2 text-xs font-mono"
      onClick={(e) => e.stopPropagation()}
    >
      <span className="rd-section">review</span>
      {error && <p className="text-accent-error">The task’s diff could not be loaded.</p>}
      {diff?.files.map((file) => (
        <div key={file.path} data-file={file.path} className="flex flex-col gap-0.5">
          <span className="text-text-strong">
            {file.path}{' '}
            <span className="text-text-faint">
              +{file.additions} −{file.deletions}
            </span>
          </span>
          <pre className="whitespace-pre overflow-x-auto text-text-muted">{file.patch}</pre>
        </div>
      ))}
      <textarea
        aria-label="Review note"
        value={note}
        onChange={(e) => setNote(e.target.value)}
        placeholder="What the next attempt should change (sent with Reject)"
        className="rounded border border-border-default bg-transparent p-1 text-text-muted"
      />
      <div className="flex gap-2">
        <button
          type="button"
          data-action="approve"
          disabled={submit.isPending}
          onClick={() => decide('approve')}
          className={BUTTON}
        >
          Approve
        </button>
        <button
          type="button"
          data-action="reject"
          disabled={submit.isPending}
          onClick={() => decide('reject')}
          className={BUTTON}
        >
          Reject
        </button>
      </div>
      {submit.error && <p className="text-accent-error">{submit.error.message}</p>}
    </div>
  );
}
