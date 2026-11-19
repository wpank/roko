/**
 * The words for a plan's last run when Run would set it aside (gap-0eb763).
 *
 * Contract P-7 is kept: an execute with no body starts the plan over and sets
 * aside the run its checkpoint records (bug-cbdb6b). So when a resume would
 * continue that run, Run and Run again ask first (Will's call of 2026-10-27,
 * option B). The run is named as roko names it: its short id, its status and
 * its tasks.
 */
import type { WireCheckpointRun } from '@/api/contracts';
import { formatCost } from '@/lib/formatters';

const TRAILING_UUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Run `runId` as people see it: the first 8 characters of the UUID it ends
 * with (a CLI run's id is `graph-<plan>-<uuid>`), or of the id itself
 * (roko's `short_run_id`).
 */
export function shortRunId(runId: string): string {
  return (TRAILING_UUID.exec(runId)?.[0] ?? runId).slice(0, 8);
}

/** "T1", "T1 and T2", "T1, T2 and T3"; past six, "T1, T2, T3, T4, T5 and 3 more" (roko's `task_list`). */
export function taskList(tasks: readonly string[]): string {
  if (tasks.length === 0) return 'no tasks';
  if (tasks.length === 1) return tasks[0];
  if (tasks.length <= 6) return `${tasks.slice(0, -1).join(', ')} and ${tasks[tasks.length - 1]}`;
  return `${tasks.slice(0, 5).join(', ')} and ${tasks.length - 5} more`;
}

/** What Run asks when the plan's last run can be resumed. */
export interface ResumeQuestion {
  /** What the last run did and what is left of it. */
  facts: string;
  /** What Resume runs. */
  resume: string;
  /** What Start over does. */
  startOver: string;
}

/**
 * The question for `run`, a run a resume would continue. A run whose tasks
 * all passed but whose delivery a crash cut has only its delivery left
 * (bug-4fadf8), which a resume runs without running a task again.
 */
export function resumeQuestion(run: WireCheckpointRun): ResumeQuestion {
  const last =
    `This plan's last run, ${shortRunId(run.run_id)} (${run.status}), ` +
    `passed ${taskList(run.passed_tasks)}`;
  const spent =
    run.spent_usd !== undefined && run.spent_usd > 0
      ? `; the ${formatCost(run.spent_usd)} it spent stays spent`
      : '';
  const startOver = `Start over sets that run aside and runs every task again${spent}.`;
  const left = run.tasks_left;
  if (run.delivery_pending === true || left.length === 0) {
    return {
      facts: `${last}, and only the delivery is left.`,
      resume: 'Resume runs no task again, only its delivery.',
      startOver,
    };
  }
  return {
    facts: `${last}, and ${taskList(left)} ${left.length === 1 ? 'is' : 'are'} left.`,
    resume: `Resume runs only ${taskList(left)}.`,
    startOver,
  };
}

/**
 * What a run that started over set aside, as roko's `checkpoint.set_aside`
 * event says it without the manifest's path: "Started over: set aside run
 * 8f3c2a1b (interrupted; passed T01; left T02 and T03)."
 */
export function setAsideFacts(run: WireCheckpointRun): string {
  const facts = [run.status];
  if (run.passed_tasks.length > 0) facts.push(`passed ${taskList(run.passed_tasks)}`);
  if (run.tasks_left.length > 0) facts.push(`left ${taskList(run.tasks_left)}`);
  if (run.spent_usd !== undefined && run.spent_usd > 0) facts.push(`spent ${formatCost(run.spent_usd)}`);
  return `Started over: set aside run ${shortRunId(run.run_id)} (${facts.join('; ')}).`;
}
