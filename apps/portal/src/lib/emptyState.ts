/**
 * emptyState.ts — the sentence a place with nothing selected shows.
 *
 * `describeEmpty` never returns an empty string or the phrase "no data".
 * It always describes the *current system state*, not the absence of content.
 * A selected plan says where it stands on its own page (`lib/runCard.ts`).
 *
 * Mirrors the philosophy in mori's `widgets/agent_output.rs:327`.
 */

import { SIGN_IN_HINT } from '@/lib/bootstrap';
import type { ConnectionStatus } from '@/lib/bootstrap';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface EmptyStateInput {
  /** Folder name of the workspace (shown in the "no plans" message). */
  workspace: string;
  /** Current SSE connection status. */
  connection: ConnectionStatus;
  /** Total number of plans in the workspace. */
  planCount: number;
}

// ── describeEmpty ──────────────────────────────────────────────────────────────

/**
 * Return a human-readable sentence describing the current system state.
 *
 * Cases are evaluated in strict precedence order:
 * 1. Connection is lost → reconnecting message.
 * 2. No plans → prompt to describe work; otherwise → prompt to select one.
 */
export function describeEmpty(input: EmptyStateInput): string {
  const { connection, workspace, planCount } = input;

  // ── 1. Connection problems ──────────────────────────────────────────────────
  if (connection === 'unauthorized') return SIGN_IN_HINT;
  if (connection === 'disconnected' || connection === 'error') {
    return 'Lost the server; reconnecting.';
  }

  // ── 2. No plan selected ─────────────────────────────────────────────────────
  return planCount === 0
    ? `No plans in ${workspace} yet. Describe what you want to build.`
    : 'Select a plan, or press n to describe a new one.';
}
