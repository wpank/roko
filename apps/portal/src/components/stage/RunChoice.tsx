'use client';

/**
 * RunChoice — what Run and Run again ask when the plan's last run can be
 * resumed, and what a run that started over set aside (gap-0eb763).
 *
 * Contract P-7 is kept (Will's call of 2026-10-27, option B): an execute with
 * no body starts the plan over and sets its last run aside. So before Run or
 * Run again, `usePrimaryAction` reads the plan's `resumable_run`; when one is
 * set, the store holds it as the plan's `runChoice` and this asks instead of
 * running. Resume sends `{resume: true}`, so only what is left runs; Start
 * over sends no body. When the answer says a run that started over set aside
 * a run that could have been resumed, this says so, with the command that
 * puts it back, until closed.
 *
 * Exports:
 *   useStartRun — start a plan's run, keeping what it set aside
 *   RunChoice   — the question and the set-aside notice, under the plan header
 */

import { useCallback, useEffect } from 'react';
import { useRunPlan } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { describeRequestError } from '@/lib/apiErrors';
import { resumeQuestion, setAsideFacts, shortRunId } from '@/lib/resumable';
import { Button } from '@/components/atoms/Button';
import { Notice } from '@/components/primitives/Notice';

/**
 * Start a run of a plan: `start(planId, resume)`. It answers the plan's open
 * question, and the store keeps the run the answer says it set aside, when a
 * resume could have continued it.
 */
export function useStartRun(onError?: (message: string) => void) {
  const { mutate } = useRunPlan();
  const setRunChoice = useDashboardStore((s) => s.setRunChoice);
  const setStartedOver = useDashboardStore((s) => s.setStartedOver);

  return useCallback(
    (planId: string, resume: boolean) => {
      setRunChoice(null);
      mutate(resume ? { id: planId, resume: true } : { id: planId }, {
        onSuccess: (accepted) => {
          const setAside = accepted.set_aside;
          setStartedOver(setAside?.resumable ? { planId, setAside } : null);
        },
        onError: (err) => onError?.(describeRequestError(err, 'running plans')),
      });
    },
    [mutate, setRunChoice, setStartedOver, onError],
  );
}

export function RunChoice({
  planId,
  onError,
}: {
  planId: string;
  onError?: (message: string) => void;
}) {
  const choice = useDashboardStore((s) => (s.runChoice?.planId === planId ? s.runChoice.run : null));
  const setAside = useDashboardStore((s) =>
    s.startedOver?.planId === planId ? s.startedOver.setAside : null,
  );
  const running = useDashboardStore((s) => s.run.plans[planId]?.phase === 'running');
  const setRunChoice = useDashboardStore((s) => s.setRunChoice);
  const setStartedOver = useDashboardStore((s) => s.setStartedOver);
  const start = useStartRun(onError);

  // A run that started some other way settles the question.
  useEffect(() => {
    if (running && choice) setRunChoice(null);
  }, [running, choice, setRunChoice]);

  const question = choice && !running ? resumeQuestion(choice) : null;

  return (
    <>
      {question && (
        <div data-region="run-choice" role="group" aria-label="Resume or start over">
          <Notice kind="info" onClose={() => setRunChoice(null)}>
            <span className="block">{question.facts}</span>
            <span className="block">
              {question.resume} {question.startOver}
            </span>
            <span className="mt-2 flex gap-2">
              <Button
                data-choice="resume"
                variant="primary"
                size="sm"
                onClick={() => start(planId, true)}
              >
                Resume
              </Button>
              <Button
                data-choice="start-over"
                variant="secondary"
                size="sm"
                onClick={() => start(planId, false)}
              >
                Start over
              </Button>
            </span>
          </Notice>
        </div>
      )}
      {setAside && (
        <div data-region="set-aside">
          <Notice kind="info" onClose={() => setStartedOver(null)}>
            {setAsideFacts(setAside)} In a terminal,{' '}
            <code>roko resume {shortRunId(setAside.run_id)}</code> puts it back.
          </Notice>
        </div>
      )}
    </>
  );
}
