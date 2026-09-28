/**
 * Status-line field builder for the plan overview.
 *
 * Returns only the fields whose values are actually known — never emits a
 * placeholder dot or an undefined/NaN string.  Consumers join the fields with
 * " · " separators.
 *
 * Two modes driven by `hasRun`:
 *   • Before a run: tasks, waves, estimate, parallel.
 *   • Once run:     progress, elapsed, eta, cost, agents.
 */
import { compactDuration, formatCost } from '@/lib/formatters';

export type StatusFieldKey =
  | 'tasks'
  | 'waves'
  | 'estimate'
  | 'parallel'
  | 'progress'
  | 'elapsed'
  | 'eta'
  | 'cost'
  | 'agents';

export interface StatusField {
  key: StatusFieldKey;
  text: string;
}

export interface StatusLineInput {
  hasRun: boolean;
  running: boolean;
  taskCount: number;
  waveCount: number;
  estimatedMinutes: number | null;
  parallel: number | null;
  tasksDone: number;
  tasksTotal: number;
  elapsedMs: number | null;
  etaMinutes: number | null;
  costUsd: number;
  busyAgents: number;
  maxParallel: number | null;
}

/**
 * Build the ordered list of status fields for the given plan state.
 * Fields whose values are not yet known are omitted entirely.
 */
export function statusFields(input: StatusLineInput): StatusField[] {
  const fields: StatusField[] = [];

  if (!input.hasRun) {
    // ── Before a run ──────────────────────────────────────────────────────────
    // tasks: always present
    fields.push({
      key: 'tasks',
      text: `${input.taskCount} ${input.taskCount === 1 ? 'task' : 'tasks'}`,
    });

    // waves: only when at least one wave is known
    if (input.waveCount > 0) {
      fields.push({
        key: 'waves',
        text: `${input.waveCount} ${input.waveCount === 1 ? 'wave' : 'waves'}`,
      });
    }

    // estimate: only when finite and positive
    if (
      input.estimatedMinutes != null &&
      isFinite(input.estimatedMinutes) &&
      input.estimatedMinutes > 0
    ) {
      fields.push({
        key: 'estimate',
        text: `~${input.estimatedMinutes} min`,
      });
    }

    // parallel: only when finite and positive
    if (input.parallel != null && isFinite(input.parallel) && input.parallel > 0) {
      fields.push({
        key: 'parallel',
        text: `parallel ${input.parallel}`,
      });
    }
  } else {
    // ── Once run ──────────────────────────────────────────────────────────────
    // progress: always present
    fields.push({
      key: 'progress',
      text: `${input.tasksDone}/${input.tasksTotal}`,
    });

    // elapsed: only when the start time is known (elapsedMs >= 0)
    if (input.elapsedMs != null && input.elapsedMs >= 0) {
      fields.push({
        key: 'elapsed',
        // compactDuration never returns '·' for a non-negative finite number
        text: compactDuration(input.elapsedMs),
      });
    }

    // eta: only while actively running and when the estimate is available
    if (input.running && input.etaMinutes != null) {
      fields.push({
        key: 'eta',
        text: `~${input.etaMinutes}m`,
      });
    }

    // cost: only when non-zero
    if (input.costUsd > 0) {
      fields.push({
        key: 'cost',
        text: formatCost(input.costUsd),
      });
    }

    // agents: while running or when busy agents remain; omit when idle + zero
    if (input.running || input.busyAgents > 0) {
      const text =
        input.maxParallel != null
          ? `agents ${input.busyAgents}/${input.maxParallel}`
          : `agents ${input.busyAgents}`;
      fields.push({ key: 'agents', text });
    }
  }

  return fields;
}
