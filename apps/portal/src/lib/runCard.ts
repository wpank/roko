/**
 * A plan as roko's run card (the design system's §4.8, at full width): its state, the four readings on the glass,
 * the plan drawn as a sequencer, and one line per task that runs now. Pure: the page draws what this returns.
 */

import type { WirePlanSummary, WirePlanTask } from '@/api/contracts';
import type { Signal } from '@/components/kit/Led';
import type { Tone } from '@/components/kit/DotMatrix';
import type { GlyphState } from '@/lib/glyphs';
import type { Problem } from '@/lib/planProblems';
import { queuePosition } from '@/lib/planSet';
import type { CheckRun, PlanRun, RunState, TaskRun, TryRun } from '@/lib/runState';
import { taskKey } from '@/lib/runState';
import type { ToolStep } from '@/lib/streamRecord';
import { buildTaskRows } from '@/lib/taskRows';
import type { TaskRowModel } from '@/lib/taskRows';
import { clock, clockLabel, count, duration, money } from '@/lib/words';

// ── Types ──────────────────────────────────────────────────────────────────────

/** Where a plan stands, in the design system's words. */
export type CardState =
  | 'ready'
  | 'cant-run'
  | 'queued'
  | 'running'
  | 'passed'
  | 'not-checked'
  | 'failed'
  | 'interrupted'
  | 'stopped'
  | 'needs-you';

/** A pad's look: waiting (dashed), live, pass, fail, you (orange), still (a settled grey), skipped. */
export type PadState = 'waiting' | 'live' | 'pass' | 'fail' | 'you' | 'still' | 'skipped';

export interface Pad {
  id: string;
  /** Its number in the plan, two digits: `01`. */
  n: string;
  title: string;
  state: PadState;
  /** One pip per try: failed, passed, or running. */
  tries: Signal[];
  /** The pad's foot: its time, its try, or why it waits. */
  foot: string;
  /** What keeps the task from running (`no check`), from the plan's validation; null when nothing does. */
  problem: string | null;
  /** Why a task the run skipped never ran, in a sentence; null when it wasn't skipped for that. */
  waits: string | null;
}

export interface Step {
  /** Two digits: `01`. */
  n: string;
  /** Live while a pad in it runs; pass once every pad in it passed. */
  state: 'live' | 'pass' | null;
  pads: Pad[];
}

export interface Edge {
  from: string;
  to: string;
  /** Pass once the dependency is met; live into a running pad. */
  state: 'pass' | 'live' | null;
}

export interface Reading {
  /** What the glass shows: `1/3`, `0:09`, `$0.21`. */
  text: string;
  /** What a screen reader says. */
  label: string;
  tone: Tone;
  /** The small line under it, or ''. */
  note: string;
  pips?: Signal[];
  /** Spend against the cap: 0 to 20 lit segments, and their tone. */
  meter?: { lit: number; tone: 'live' | 'pass' | 'you' };
}

/** One line per task that runs now: "Task 2 · writing cli.py · glm-4.7". */
export interface NowLine {
  taskId: string;
  n: string;
  doing: string;
  model: string | null;
  /** The try it is on, from 1. */
  tryN: number;
  /** Why it is on another try, when it is: "the usage check failed". */
  because: string | null;
  /** How long this try has run, or null when unknown. */
  elapsedMs: number | null;
}

export interface Card {
  state: CardState;
  pill: { signal: Signal; word: string };
  readings: { tasks: Reading; time: Reading; spent: Reading; checks: Reading };
  steps: Step[];
  edges: Edge[];
  now: NowLine[];
  /** The task rows the card was drawn from, in plan order. */
  rows: TaskRowModel[];
}

export interface CardInput {
  plan: WirePlanSummary;
  tasks: readonly WirePlanTask[];
  run: RunState;
  /** The project's own checks, run on every task (`[[gates.rungs]]`), when known. */
  rungs: readonly string[];
  /** The plan's spending cap in USD, or null when it has none or it is unknown. */
  capUsd: number | null;
  /** How many checks of its own the plan runs once its tasks pass (`[meta] verify`, gap-c1726f). */
  planChecks?: number;
  /** When roko accepted the plan's run, in ms, when roko serve says (gap-c1726f): TIME counts from it. */
  runStartedMs?: number | null;
  nowMs: number;
  /** The plan's problems in words (lib/planProblems.ts); any error keeps a plan that hasn't run from running. */
  problems?: readonly Problem[];
}

// ── Words ──────────────────────────────────────────────────────────────────────

/** The phase of a task whose verified try waits for a review (roko-cli's AWAITING_APPROVAL_PHASE). */
export const AWAITING_APPROVAL = 'awaiting_approval';

const PILLS: Record<CardState, { signal: Signal; word: string }> = {
  ready: { signal: 'waiting', word: 'ready' },
  'cant-run': { signal: 'you', word: 'needs you' },
  queued: { signal: 'waiting', word: 'queued' },
  running: { signal: 'live', word: 'running' },
  passed: { signal: 'pass', word: 'passed' },
  'not-checked': { signal: 'still', word: 'not checked' },
  failed: { signal: 'fail', word: 'failed' },
  interrupted: { signal: 'you', word: 'interrupted' },
  stopped: { signal: 'still', word: 'stopped' },
  'needs-you': { signal: 'you', word: 'needs you' },
};

/**
 * A check as people say it: `verify[0:usage]`, `verify:0/usage`, `custom:usage` (a phase the gates don't know) and
 * `usage` are all "usage"; `rung[compile]` is "compile".
 */
export function checkWord(name: string): string {
  return name
    .replace(/^verify[[:]\d+[:/]/, '')
    .replace(/^(rung|custom)[[:]/, '')
    .replace(/\]$/, '')
    .trim();
}

/** What a tool call is doing, as a person would say it. */
export function doingWords(step: Pick<ToolStep, 'tool' | 'target'>): string {
  const target = step.target ? ` ${step.target}` : '';
  const tool = step.tool.toLowerCase();
  if (/write|create/.test(tool)) return `writing${target}`;
  if (/edit|patch|replace/.test(tool)) return `editing${target}`;
  if (/read|view|cat/.test(tool)) return `reading${target}`;
  if (/test|check|verify/.test(tool)) return 'running its checks';
  if (/search|grep|find|glob/.test(tool)) return target ? `searching for${target}` : 'searching the code';
  if (/list|ls|tree/.test(tool)) return 'looking around the project';
  if (/bash|shell|exec|run|command/.test(tool)) return target ? `running${target}` : 'running a command';
  return `${tool.replace(/_/g, ' ')}${target}`;
}

const two = (i: number) => String(i + 1).padStart(2, '0');

// ── Pads ───────────────────────────────────────────────────────────────────────

const PAD_STATES: Record<GlyphState, PadState> = {
  done: 'pass',
  preexisting: 'pass',
  satisfied: 'pass',
  active: 'live',
  unverified: 'live',
  accepted: 'you',
  unchecked: 'still',
  marked: 'still',
  failed: 'fail',
  interrupted: 'fail',
  queued: 'waiting',
  pending: 'waiting',
  skipped: 'skipped',
};

function tryPips(live: TaskRun | undefined, state: PadState): Signal[] {
  const tries = live?.tries ?? [];
  if (tries.length > 0) {
    return tries.map((t: TryRun) => (t.state === 'passed' ? 'pass' : t.state === 'failed' ? 'fail' : 'live'));
  }
  if (state === 'pass') return ['pass'];
  if (state === 'fail') return ['fail'];
  if (state === 'live') return ['live'];
  return [];
}

function padFoot(row: TaskRowModel, live: TaskRun | undefined, state: PadState, held: boolean, running: boolean): string {
  const tries = live?.tries?.length ?? row.attempts;
  if (held) return 'waits for you';
  switch (state) {
    case 'live': {
      const elapsed = row.time.kind === 'elapsed' && row.time.ms !== null ? clock(row.time.ms) : '';
      return tries > 1 ? `try ${tries}${elapsed ? ` · ${elapsed}` : ''}` : elapsed;
    }
    case 'pass':
      return row.time.ms !== null && row.time.kind === 'actual' ? duration(row.time.ms) : 'passed';
    case 'fail': {
      const took = row.time.ms !== null && row.time.kind === 'actual' ? duration(row.time.ms) : '';
      return [tries > 1 ? count(tries, 'try', 'tries') : 'failed', took].filter(Boolean).join(' · ');
    }
    case 'you':
      return 'not a clean pass';
    case 'still':
      return row.state === 'marked' ? 'done before' : 'not checked';
    case 'skipped':
      return 'skipped';
    default:
      return row.blocked !== null ? 'blocked' : running ? 'waiting' : 'not run';
  }
}

// ── The card ───────────────────────────────────────────────────────────────────

/**
 * Where the plan stands. A plan with no live record never ran; one a plan set still holds is queued, and one left
 * pending by a set that has ended is ready again.
 */
function stateOf(live: PlanRun | undefined, rows: readonly TaskRowModel[], held: boolean, queued: boolean): CardState {
  if (held) return 'needs-you';
  if (!live) return 'ready';
  switch (live.phase) {
    case 'running':
      return 'running';
    case 'pending':
      return queued ? 'queued' : 'ready';
    case 'completed': {
      // A revision added tasks its last run never ran: the plan is ready to run again, not passed (gap-a5b2e8).
      if (rows.some((r) => r.status === 'pending')) return 'ready';
      if (rows.some((r) => r.status === 'accepted_with_failures')) return 'needs-you';
      return rows.some((r) => r.status === 'unverified') ? 'not-checked' : 'passed';
    }
    case 'failed':
      return rows.some((r) => r.status === 'interrupted') && !rows.some((r) => r.status === 'failed')
        ? 'interrupted'
        : 'failed';
    case 'cancelled':
      return 'stopped';
    default:
      return 'ready';
  }
}

/** The tone of the readings that say how far it got: the state's, or dim before a run. */
function progressTone(state: CardState): Tone {
  if (state === 'running') return 'live';
  if (state === 'passed') return 'pass';
  if (state === 'failed') return 'fail';
  if (state === 'ready' || state === 'cant-run' || state === 'queued') return 'dim';
  return 'on';
}

const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

export function buildCard(input: CardInput): Card {
  const { plan, tasks, run, rungs, capUsd, nowMs } = input;
  const live = run.plans[plan.id];
  const ran = live !== undefined && live.phase !== 'pending';
  const { rows, waves } = tasks.length > 0 ? buildTaskRows([...tasks], run, plan.id, nowMs) : { rows: [], waves: { waves: [], diagnostics: [] } };
  const liveTask = (id: string) => run.tasks[taskKey(plan.id, id)];
  const heldIds = new Set(rows.filter((r) => liveTask(r.id)?.phase === AWAITING_APPROVAL).map((r) => r.id));
  const problems = input.problems ?? [];
  const settled = stateOf(live, rows, heldIds.size > 0 && live?.phase === 'running', queuePosition(run, plan.id) !== null);
  // A plan that hasn't run, and has an error, can't run: it needs its person first.
  const state: CardState = settled === 'ready' && problems.some((p) => p.severity === 'error') ? 'cant-run' : settled;
  const problemOf = (id: string) => problems.find((p) => p.taskId === id && p.severity === 'error')?.short ?? null;
  const order = new Map(tasks.map((t, i) => [t.id, i]));

  // ── Pads and steps
  const pads = new Map<string, Pad>();
  for (const row of rows) {
    const held = heldIds.has(row.id);
    const padState: PadState = held ? 'you' : PAD_STATES[row.state];
    pads.set(row.id, {
      id: row.id,
      n: two(order.get(row.id) ?? 0),
      title: row.title || row.id,
      state: padState,
      tries: tryPips(liveTask(row.id), padState),
      // A task that hasn't run waits only while its run goes on; after it, it simply hasn't run (gap-a5b2e8).
      foot: (!ran && problemOf(row.id)) || padFoot(row, liveTask(row.id), padState, held, state === 'running'),
      problem: ran ? null : problemOf(row.id),
      waits: row.blocked === null ? null : waitsFor(liveTask(row.id)?.blockedBy, order),
    });
  }
  const steps: Step[] = waves.waves.map((ids, i) => {
    const stepPads = ids.map((id) => pads.get(id)).filter((p): p is Pad => p !== undefined);
    const stepState =
      stepPads.some((p) => p.state === 'live' || p.state === 'you')
        ? 'live'
        : stepPads.length > 0 && stepPads.every((p) => p.state === 'pass')
          ? 'pass'
          : null;
    return { n: two(i), state: stepState, pads: stepPads };
  });
  const edges: Edge[] = tasks.flatMap((t) =>
    t.depends_on
      .filter((dep) => pads.has(dep) && pads.has(t.id))
      .map((dep) => {
        const met = pads.get(dep)!.state === 'pass';
        const into = pads.get(t.id)!.state;
        return { from: dep, to: t.id, state: met ? (into === 'live' ? ('live' as const) : ('pass' as const)) : null };
      }),
  );

  // ── Readings
  const tone = progressTone(state);
  const passedPads = rows.filter((r) => ['pass', 'still'].includes(pads.get(r.id)!.state)).length;
  const taskPips = rows.map((r) => padSignal(pads.get(r.id)!.state));
  const tasksReading: Reading = {
    text: `${passedPads}/${rows.length}`,
    label: `${passedPads} of ${count(rows.length, 'task')} done`,
    tone,
    note: '',
    pips: taskPips,
  };

  // TIME runs from when roko accepted the run, which only roko serve knows, else from when the plan's tasks began;
  // once the run has ended, it is the run's own duration, as roko counted it (gap-c1726f).
  const startedAt = ran ? (input.runStartedMs ?? live?.startedAtMs ?? null) : null;
  // The run's duration is this plan's only when the run held this plan alone.
  const aloneInRun = run.planSet
    ? run.planSet.planIds.length === 1 && run.planSet.planIds[0] === plan.id
    : run.run.startedAtMs !== null && run.run.startedAtMs === live?.startedAtMs;
  const runEnded = ran && live?.finishedAtMs != null && run.run.durationMs !== null && aloneInRun;
  const elapsed = runEnded
    ? run.run.durationMs
    : ran && startedAt !== null
      ? (live!.finishedAtMs ?? nowMs) - startedAt
      : null;
  const retried = rows.find((r) => (liveTask(r.id)?.tries?.length ?? r.attempts) > 1);
  const timeNote =
    state === 'running' && startedAt !== null
      ? `started ${hhmm(startedAt)}`
      : ran && retried
        ? `task ${pads.get(retried.id)!.n.replace(/^0/, '')} took ${count(liveTask(retried.id)?.tries?.length ?? retried.attempts, 'try', 'tries')}`
        : ran && live?.finishedAtMs
          ? `finished ${hhmm(live.finishedAtMs)}`
          : plan.estimated_minutes
            ? `about ${plan.estimated_minutes} min`
            : 'not started';
  const timeReading: Reading = {
    text: clock(elapsed ?? 0),
    label: elapsed !== null ? clockLabel(elapsed) : 'not started',
    tone: state === 'running' ? 'live' : ran ? 'on' : 'dim',
    note: timeNote,
  };

  // The whole run's spend: on a resume, what its earlier processes spent too (gap-c1726f).
  const spent = (live?.costUsd ?? 0) + (live?.carriedUsd ?? 0);
  const share = capUsd && capUsd > 0 ? spent / capUsd : 0;
  // An unknown cap isn't no cap (gap-acfe4b): without one the reading is the spend alone, with no meter.
  const spentReading: Reading = {
    text: money(spent),
    label: capUsd ? `${money(spent)} of ${money(capUsd)}` : `${money(spent)} spent`,
    tone: ran ? 'on' : 'dim',
    note: capUsd ? `of ${money(capUsd)}` : '',
    ...(capUsd
      ? {
          meter: {
            lit: Math.min(20, Math.ceil(share * 20 - 1e-9)),
            tone: share >= 0.8 ? 'you' : state === 'passed' ? 'pass' : 'live',
          },
        }
      : {}),
  };

  // Every check roko runs on the work it keeps: each task's steps and rungs, then the plan's own (gap-c1726f).
  const checkStates: Signal[] = [
    ...rows.flatMap((r) => checkSignals(r, liveTask(r.id)?.checks ?? [], rungs, pads.get(r.id)!.state === 'pass')),
    ...planCheckSignals(input.planChecks ?? 0, live?.planCheck ?? null),
  ];
  const checksPassed = checkStates.filter((s) => s === 'pass').length;
  const checksReading: Reading = {
    text: `${checksPassed}/${checkStates.length}`,
    label: `${checksPassed} of ${count(checkStates.length, 'check')} passed`,
    tone: state === 'failed' && checkStates.includes('fail') ? 'fail' : tone,
    note: '',
    pips: checkStates,
  };

  // ── What runs now
  const now: NowLine[] = rows
    .filter((r) => pads.get(r.id)!.state === 'live')
    .map((r) => {
      const task = liveTask(r.id);
      const tries = task?.tries ?? [];
      const current = tries[tries.length - 1];
      const previous = tries.length > 1 ? tries[tries.length - 2] : undefined;
      const agent = task?.agentId ? run.agents[task.agentId] : undefined;
      return {
        taskId: r.id,
        n: pads.get(r.id)!.n,
        doing: nowDoing(task, run, plan.id, current?.attempt ?? 1),
        model: current?.model ?? task?.model ?? null,
        tryN: Math.max(1, tries.length),
        because: previous ? becauseOf(previous) : null,
        elapsedMs: agent?.spawnedAtMs != null ? nowMs - agent.spawnedAtMs : r.time.kind === 'elapsed' ? r.time.ms : null,
      };
    });

  return {
    state,
    pill: PILLS[state],
    readings: { tasks: tasksReading, time: timeReading, spent: spentReading, checks: checksReading },
    steps,
    edges,
    now,
    rows,
  };
}

function padSignal(state: PadState): Signal {
  if (state === 'pass' || state === 'still') return 'pass';
  if (state === 'fail') return 'fail';
  if (state === 'live') return 'live';
  if (state === 'you') return 'you';
  return 'waiting';
}

/**
 * A task's checks as signals: its own declared checks, then the project's, each passed, failed, running or
 * waiting; checks the run reported beyond those count too.
 */
function checkSignals(
  row: TaskRowModel,
  checks: readonly CheckRun[],
  rungs: readonly string[],
  passed = false,
): Signal[] {
  // A task that passed with no results here passed in an earlier process of the run, which a resume kept: its
  // checks passed then (gap-c1726f).
  if (passed && checks.length === 0) return [...row.verify, ...rungs].map((): Signal => 'pass');
  const seen = new Set<string>();
  const signalOf = (c: CheckRun | undefined): Signal =>
    !c ? 'waiting' : c.status === 'passed' ? 'pass' : c.status === 'failed' ? 'fail' : 'live';
  const take = (c: CheckRun | undefined) => {
    if (c) seen.add(c.name);
    return signalOf(c);
  };
  // A task's own steps by their place, as roko names them (`verify[<i>:<phase>]`), so steps that share a phase word
  // count once each (gap-c1726f); a result with no place is matched by its word, as older runs named them.
  const out: Signal[] = row.verify.map((v, i) =>
    take(
      checks.find((c) => !seen.has(c.name) && c.index === i) ??
        checks.find((c) => !seen.has(c.name) && c.index === null && checkWord(c.phase || c.name) === checkWord(v.phase)),
    ),
  );
  for (const rung of rungs) {
    out.push(take(checks.find((c) => !seen.has(c.name) && c.index === null && checkWord(c.name) === rung)));
  }
  for (const c of checks) if (!seen.has(c.name)) out.push(signalOf(c));
  return out;
}

/** The plan's own checks as signals: waiting until the plan check reports, then passed or failed together. */
function planCheckSignals(count: number, result: { passed: boolean } | null): Signal[] {
  const n = Math.max(count, result ? 1 : 0);
  const signal: Signal = !result ? 'waiting' : result.passed ? 'pass' : 'fail';
  return Array.from({ length: n }, () => signal);
}

/** What a running task does now, from its checks and its transcript's latest tool step on this try. */
function nowDoing(task: TaskRun | undefined, run: RunState, planId: string, attempt: number): string {
  if (!task) return 'starting';
  const checking = task.checks.find((c) => c.status === 'running');
  if (checking) return `checking ${checkWord(checking.phase || checking.name)}`;
  const entries = run.transcripts[taskKey(planId, task.taskId)]?.entries ?? [];
  for (let i = entries.length - 1; i >= 0; i -= 1) {
    const e = entries[i]!;
    if (e.kind === 'step' && (e.attempt === attempt || e.attempt === 0)) return doingWords(e);
  }
  return 'reading the task';
}

/** Why a skipped task never ran: the task it needed, by its number, didn't pass. */
function waitsFor(by: string | null | undefined, order: ReadonlyMap<string, number>): string {
  const at = by ? order.get(by) : undefined;
  return at !== undefined
    ? `It didn’t run, because task ${at + 1}, which it needs, didn’t pass.`
    : 'It didn’t run, because a task it needs didn’t pass.';
}

/** Why a try failed, in a clause: "the usage check failed". */
export function becauseOf(t: TryRun): string {
  if (t.failedCheck) return `the ${checkWord(t.failedCheck)} check failed`;
  if (t.detail) return t.detail.split('\n')[0]!.replace(/^[a-z_]+:\s*/, '');
  return 'it did not pass';
}
