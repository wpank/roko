/**
 * planProblems.ts — a plan's validation, in a person's words (gap-6c4d24): one line per problem, about the task it
 * is on by its number, without rule ids or config keys (those stay in the fold for whoever wants them). The checker
 * reports one missing check twice (PLAN_035 from the schema, PLAN_037 from the verify rules); it reads as one line.
 */

import type { WireDiagnostic } from '@/api/contracts';

export interface Problem {
  /** The task it is on, or null for the plan as a whole. */
  taskId: string | null;
  severity: 'error' | 'warning';
  /** The line, with code between backticks: `Task 2 has no check, so …`. */
  text: string;
  /** A word or two for the task's pad: `no check`. */
  short: string;
  /** The checker's own words, with its rule id, for the fold. */
  raw: string[];
}

/** `T2` as people say it: `Task 2`, by its place in the plan, else its id. */
function taskName(taskId: string, order: readonly string[]): string {
  const at = order.indexOf(taskId);
  return at === -1 ? `Task ${taskId}` : `Task ${at + 1}`;
}

/** `task 'T2' …` and `T2: …` in a checker message, said as `Task 2`. */
function withTaskNames(message: string, order: readonly string[]): string {
  return message
    .replace(/\btask '([^']+)'/gi, (_, id: string) => taskName(id, order))
    .replace(/^([A-Za-z][\w.-]*): /, (whole, id: string) => (order.includes(id) ? `${taskName(id, order)}: ` : whole));
}

/** The task a diagnostic is about: its task_id, else the task its message names first. */
function taskOf(d: WireDiagnostic, order: readonly string[]): string | null {
  if (d.task_id) return d.task_id;
  const named = d.message.match(/\btask '([^']+)'/i)?.[1] ?? d.message.match(/(?:^|: )([A-Za-z][\w.-]*): /)?.[1];
  return named && order.includes(named) ? named : null;
}

/** A diagnostic that says a task has no check: PLAN_037, or PLAN_035's "missing 'verify'". */
function saysNoCheck(d: WireDiagnostic): boolean {
  return d.rule_id === 'PLAN_037' || (d.rule_id === 'PLAN_035' && /missing 'verify'/.test(d.message));
}

/** One problem's words, by its rule; anything else in the checker's own words, its task named. */
function words(d: WireDiagnostic, taskId: string | null, order: readonly string[]): { text: string; short: string } {
  const task = taskId ? taskName(taskId, order) : 'The plan';
  if (saysNoCheck(d)) {
    return {
      text: `${task} has no check, so roko could never tell when it’s done. Give it a check: a command that fails until the task is done.`,
      short: 'no check',
    };
  }
  const message = d.message;
  switch (d.rule_id) {
    case 'PLAN_001':
      return { text: `The plan’s file isn’t valid TOML: ${message.replace(/^failed to parse TOML:\s*/i, '')}`, short: 'unreadable' };
    case 'PLAN_002':
      return { text: 'The plan has no tasks.', short: 'no tasks' };
    case 'PLAN_004':
      return { text: `Two tasks share the id \`${taskId ?? '?'}\`; each task needs its own.`, short: 'same id' };
    case 'PLAN_005': {
      const unknown = message.match(/unknown task '([^']+)'/)?.[1];
      return {
        text: `${task} waits for a task the plan doesn’t have${unknown ? ` (\`${unknown}\`)` : ''}.`,
        short: 'waits for nothing',
      };
    }
    case 'PLAN_006':
      return { text: `${task} can never start: nothing it waits for starts first.`, short: 'can’t start' };
    case 'PLAN_034':
      return { text: `This plan would fail when it runs: ${message.replace(/^plan would fail at runtime:\s*/i, '')}`, short: 'would fail' };
    case 'PLAN_058':
      return { text: withTaskNames(message, order).replace(/ verify step /, '’s check '), short: 'check can’t pass' };
    default: {
      // The checker's words, its advice about config keys left to the fold.
      const plain = withTaskNames(message.replace(/^schema validation failed:\s*/i, ''), order).split('; ')[0]!;
      return { text: plain.charAt(0).toUpperCase() + plain.slice(1).replace(/\.?$/, '.'), short: 'problem' };
    }
  }
}

/** The plan's problems, errors first, one line per problem. `order` is the plan's task ids, in order. */
export function planProblems(diagnostics: readonly WireDiagnostic[], order: readonly string[]): Problem[] {
  const problems: Problem[] = [];
  for (const d of diagnostics) {
    const taskId = taskOf(d, order);
    const raw = `${d.rule_id} · ${d.message}`;
    if (saysNoCheck(d)) {
      const same = problems.find((p) => p.taskId === taskId && p.short === 'no check');
      if (same) {
        same.raw.push(raw);
        if (d.severity === 'error') same.severity = 'error';
        continue;
      }
    }
    problems.push({ taskId, severity: d.severity, ...words(d, taskId, order), raw: [raw] });
  }
  return problems.sort((a, b) => (a.severity === b.severity ? 0 : a.severity === 'error' ? -1 : 1));
}
