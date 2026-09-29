/**
 * selection.ts — pure URL-state helpers for plan/task selection.
 *
 * The URL is the single source of truth. No stores, no component state.
 * All three functions are pure (no side effects, no globals).
 */

// ── Types ──────────────────────────────────────────────────────────────────────

export interface Selection {
  plan: string | null;
  task: string | null;
}

// ── parseSelection ─────────────────────────────────────────────────────────────

/**
 * Read `plan` and `task` from a query string.
 * Empty string values are normalised to null.
 *
 * @param search – the raw search string, e.g. `"?plan=foo&task=bar"` or `"plan=foo"`.
 */
export function parseSelection(search: string): Selection {
  const params = new URLSearchParams(search);
  const plan = params.get('plan') || null;
  const task = params.get('task') || null;
  return { plan, task };
}

// ── selectionSearch ────────────────────────────────────────────────────────────

/**
 * Return the new query string after applying `patch` to the current `search`.
 *
 * Rules:
 * - Keeps every unrelated query param.
 * - Drops keys whose patched value is null.
 * - If `patch` contains `plan` but NOT `task`, the existing `task` param is
 *   cleared (changing a plan implicitly resets the task selection).
 * - Returns `''` when the result has no params, otherwise `'?…'`.
 *
 * @param search – current query string (with or without the leading `?`).
 * @param patch  – the fields to change; null values remove the param.
 */
export function selectionSearch(search: string, patch: Partial<Selection>): string {
  const params = new URLSearchParams(search);

  // Changing plan clears task — unless the patch also sets task explicitly.
  if ('plan' in patch && !('task' in patch)) {
    params.delete('task');
  }

  // Apply the patch: null → delete, string → set.
  for (const [key, value] of Object.entries(patch) as [string, string | null][]) {
    if (value === null) {
      params.delete(key);
    } else {
      params.set(key, value);
    }
  }

  // Strip any empty-string values that may have arrived from other sources.
  for (const key of [...params.keys()]) {
    if (params.get(key) === '') {
      params.delete(key);
    }
  }

  const str = params.toString();
  return str ? `?${str}` : '';
}

// ── resolveSelection ───────────────────────────────────────────────────────────

/**
 * Reconcile a raw Selection against the known plan list.
 *
 * Rules (applied in order):
 * 1. Before `known.loaded` is true, return `sel` unchanged — we don't clear
 *    anything until the list has arrived.
 * 2. Once loaded, if `sel.plan` names a plan that is not in `known.planIds`,
 *    clear both `plan` and `task` silently (no error).
 * 3. Clear a `task` silently when no plan is selected, or when the plan's
 *    tasks have loaded (`known.taskIds`) and it is not among them.
 * 4. Only on `known.firstLoad`: if there is no plan selected after step 2,
 *    pick the first running plan (several may be running simultaneously;
 *    first in the array wins). The selection never jumps on its own later.
 */
export function resolveSelection(
  sel: Selection,
  known: {
    loaded: boolean;
    planIds: readonly string[];
    runningPlanIds: readonly string[];
    /** The selected plan's task ids; undefined until they have loaded. */
    taskIds?: readonly string[];
    firstLoad: boolean;
  },
): Selection {
  // Rule 1 — list not yet loaded.
  if (!known.loaded) {
    return sel;
  }

  // Rule 2 — unknown plan → clear selection.
  let resolved = sel;
  if (sel.plan !== null && !known.planIds.includes(sel.plan)) {
    resolved = { plan: null, task: null };
  }

  // Rule 3 — unknown task → clear the task.
  if (
    resolved.task !== null &&
    (resolved.plan === null || (known.taskIds && !known.taskIds.includes(resolved.task)))
  ) {
    resolved = { plan: resolved.plan, task: null };
  }

  // Rule 4 — no plan selected on load → default to the first running plan.
  if (known.firstLoad && resolved.plan === null && known.runningPlanIds.length > 0) {
    return { plan: known.runningPlanIds[0]!, task: null };
  }

  return resolved;
}
