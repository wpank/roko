/**
 * burn.ts — pure fold for the BURN cell of the run band.
 *
 * Computes per-scope token totals, per-role breakdown, and a rolling
 * tokens-per-minute rate from the `usage` ring.
 *
 * Rules:
 * - Never mutate the input state.
 * - Never call Date.now() — time is passed in via `opts.nowMs`.
 * - Cost is not here; it stays in the header.
 */

import type { RunState } from '@/lib/runState';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface Burn {
  /** Total input + output tokens for all scoped agents. */
  tokens: number;
  /**
   * Rolling token rate scaled to one minute, computed from `usage` samples
   * inside the last `windowMs` milliseconds.
   * `null` when there are no samples in the window (e.g. right after a reload).
   */
  tokensPerMin: number | null;
  /**
   * Per-role breakdown, sorted largest-first.
   * Roles with zero tokens are omitted.
   * `share` = role.tokens / total (0 when total is 0).
   */
  byRole: { role: string; tokens: number; share: number }[];
}

// ── buildBurn ──────────────────────────────────────────────────────────────────

/**
 * Derive a `Burn` from the current `RunState`.
 *
 * Scope rules:
 * - When `run.planSet` is loaded: agents whose `planId` is in `planSet.planIds`.
 * - Otherwise: agents whose `planId` belongs to a running plan.
 *
 * This is reload-safe: the snapshot carries agents with their token counts, so
 * the set-scope path works without live events.
 */
export function buildBurn(
  run: RunState,
  opts: { nowMs: number; windowMs?: number },
): Burn {
  const { nowMs, windowMs = 60_000 } = opts;

  // ── Resolve the set of in-scope planIds ───────────────────────────────────
  let scopedPlanIds: Set<string>;
  if (run.planSet !== null) {
    // Plan set loaded — use exactly those planIds.
    scopedPlanIds = new Set(run.planSet.planIds);
  } else {
    // No plan set — scope to running plans.
    scopedPlanIds = new Set(
      Object.values(run.plans)
        .filter((p) => p.phase === 'running')
        .map((p) => p.planId),
    );
  }

  // ── Aggregate tokens by role across scoped agents ─────────────────────────
  const byRoleMap = new Map<string, number>();
  for (const agent of Object.values(run.agents)) {
    if (agent.planId === null || !scopedPlanIds.has(agent.planId)) continue;
    const agentTokens = agent.inputTokens + agent.outputTokens;
    if (agentTokens === 0) continue; // omit roles with no tokens
    byRoleMap.set(agent.role, (byRoleMap.get(agent.role) ?? 0) + agentTokens);
  }

  // ── Total tokens ──────────────────────────────────────────────────────────
  let tokens = 0;
  for (const t of byRoleMap.values()) tokens += t;

  // ── byRole: sorted descending, share = tokens / total ─────────────────────
  const byRole = Array.from(byRoleMap.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([role, t]) => ({
      role,
      tokens: t,
      share: tokens > 0 ? t / tokens : 0,
    }));

  // ── tokensPerMin: sum of usage samples inside the window, scaled ──────────
  const windowStart = nowMs - windowMs;
  const windowSamples = run.usage.filter(
    (s) => s.atMs >= windowStart && s.atMs <= nowMs,
  );

  let tokensPerMin: number | null = null;
  if (windowSamples.length > 0) {
    const totalInWindow = windowSamples.reduce((s, u) => s + u.tokens, 0);
    // Scale from windowMs to 60 000 ms (one minute).
    tokensPerMin = (totalInWindow / windowMs) * 60_000;
  }

  return { tokens, tokensPerMin, byRole };
}
