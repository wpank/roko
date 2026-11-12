'use client';

/**
 * Writing — a plan being written: TIME and SPENT on glass, what roko is doing, the request quoted, and a stop key
 * (gap-6c4d24, gap-f8c57a).
 *
 * It follows the write's operation until the plan is written, the write fails, or it stops. SPENT is what the
 * write's settled planner calls cost, from its progress, against the cap its start answer gave. Stop posts the
 * cancel a run's stop posts (`POST /api/plans/{operation id}/cancel`, bug-36ce72); a write that had already written
 * its plan ends as written. A revision reports no spend and can't be stopped yet, so its glass has TIME alone and it
 * has no stop key.
 */

import React, { useEffect, useRef, useState } from 'react';
import { ApiError } from '@/api/client';
import type { WireAccepted, WirePlanDiff, WireWriteCancelled } from '@/api/contracts';
import { fetchOperation, planExists, useCancelPlan } from '@/api/queries';
import { Key } from '@/components/kit/Key';
import { ReadingCell } from '@/components/plan/Readout';
import { Notice } from '@/components/primitives/Notice';
import { describeRequestError } from '@/lib/apiErrors';
import { WriteStopped, waitForOperation } from '@/lib/operation';
import { writeReadings } from '@/lib/writing';

export interface WritingProps {
  mode: 'generate' | 'revise';
  /** The project's name, or (revising) the plan's title, said in the sentence. */
  workspace: string;
  /** The request, in the person's words. */
  request: string;
  /** The write's start answer: its operation id, its plan's id and its cap; null while the request is on its way. */
  accepted: WireAccepted | null;
  /** How long the write had run when this view opened, in ms: a write opened from its row in the plan list. */
  sinceMs?: number;
  /** The plan is written: `diff` is a revision's plan diff, when the server sent one (3229). */
  onDone(slug: string, diff?: WirePlanDiff): void;
  /** The write stopped before it wrote its plan; `costUsd` is what its settled calls cost. */
  onStopped(costUsd: number | null): void;
  /** The write failed, or roko serve couldn't be followed. */
  onFailed(err: unknown): void;
}

/** Why there's no stop key on a roko serve that can't stop a write. */
export const NO_STOP = 'This roko serve can’t stop a plan write: it ends on its own.';

export function Writing({ mode, workspace, request, accepted, sinceMs = 0, onDone, onStopped, onFailed }: WritingProps) {
  const [elapsedMs, setElapsedMs] = useState(sinceMs);
  const [costUsd, setCostUsd] = useState(0);
  // The cap from the write's progress, for a start answer that didn't give one; undefined until one says.
  const [progressCap, setProgressCap] = useState<number | null | undefined>(undefined);
  const capUsd = accepted?.max_usd !== undefined ? accepted.max_usd : progressCap;
  const [canStop, setCanStop] = useState(mode === 'generate');
  const [stopping, setStopping] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const cancelWrite = useCancelPlan();

  // The view reports how the write ended once, whichever of the follow and the stop knows first.
  const ended = useRef(false);
  const live = useRef(true);
  const report = useRef({ onDone, onStopped, onFailed });
  report.current = { onDone, onStopped, onFailed };
  const end = (how: () => void) => {
    if (!live.current || ended.current) return;
    ended.current = true;
    how();
  };

  useEffect(() => {
    live.current = true;
    if (!accepted) return;
    // Closing the view stops following the write, which goes on, in its row in the plan list.
    let following = true;
    const unfollowed = new Error('the view closed');
    void waitForOperation(
      accepted,
      {
        planExists,
        fetchOperation,
        sleep: async (ms) => {
          await new Promise((resolve) => setTimeout(resolve, ms));
          if (!following) throw unfollowed;
        },
        now: () => Date.now(),
      },
      {
        expect: mode === 'generate' ? 'new-plan' : 'revision',
        onPoll: (elapsed) => {
          if (live.current) setElapsedMs(sinceMs + elapsed);
        },
        onOperation: (op) => {
          if (!live.current) return;
          const progress = op.progress;
          if (typeof progress?.cost_usd === 'number') setCostUsd(progress.cost_usd);
          if (progress && 'max_usd' in progress) setProgressCap(progress.max_usd ?? null);
        },
      },
    ).then(
      ({ slug, diff }) => end(() => report.current.onDone(slug, diff)),
      (err: unknown) => {
        if (err === unfollowed) return;
        end(() =>
          err instanceof WriteStopped ? report.current.onStopped(err.costUsd) : report.current.onFailed(err),
        );
      },
    );
    return () => {
      following = false;
      live.current = false;
    };
    // One follow per write: the callbacks are read through `report`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accepted?.id]);

  const stop = () => {
    if (!accepted) return;
    setStopping(true);
    setNote(null);
    cancelWrite.mutate(
      { id: accepted.id },
      {
        onSuccess: (answer) => {
          const cancelled = answer as Partial<WireWriteCancelled> | null;
          end(() => report.current.onStopped(cancelled?.cost_usd ?? costUsd));
        },
        onError: (err) => {
          if (!live.current) return;
          setStopping(false);
          if (err instanceof ApiError && (err.status === 404 || err.status === 405)) {
            // An older roko serve: a write isn't a run it can find.
            setCanStop(false);
            setNote(NO_STOP);
          } else if (!(err instanceof ApiError && err.status === 409)) {
            setNote(describeRequestError(err, 'stopping the write'));
          }
          // 409: the write had already written its plan, or is saving it; the follow shows it.
        },
      },
    );
  };

  const readings = writeReadings(elapsedMs, costUsd, capUsd);
  const spends = mode === 'generate';
  return (
    <>
      <div className="rk-glass rk-cells pt-writing-glass" data-cells={spends ? 2 : 1} role="group" aria-label="Readings">
        <ReadingCell name="time" reading={readings.time} />
        {spends && <ReadingCell name="spent" reading={readings.spent} />}
      </div>
      <p className="pt-sentence" data-waiting>
        {mode === 'generate'
          ? `roko is reading ${workspace} and splitting your request into tasks, each with a check. You’ll see the plan here before anything runs.`
          : `roko is rewriting ${workspace}’s tasks. You’ll see what changed before anything runs.`}
      </p>
      <blockquote className="pt-request" data-request>
        “{request}”
      </blockquote>
      {note && <Notice kind="info">{note}</Notice>}
      {canStop && (
        <div className="rk-keys">
          <Key data-action="stop-write" disabled={stopping || !accepted} onClick={stop}>
            stop
          </Key>
        </div>
      )}
    </>
  );
}
