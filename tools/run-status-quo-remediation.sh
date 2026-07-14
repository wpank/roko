#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd -- "$SCRIPT_DIR/.." && pwd)"
WORKTREE_ROOT="$(dirname -- "$REPO")/agent-worktrees"
RUN_STATE_ROOT="${HOME}/.local/state/roko"
MASTER_CHECKLIST="$REPO/tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md"
CODEX_BIN="${CODEX_BIN:-codex}"
MAX_AGENTS="${ROKO_MAX_AGENTS:-30}"
AGENT_JOB_TIMEOUT_SECONDS="${ROKO_AGENT_JOB_TIMEOUT_SECONDS:-21600}"

if ! command -v "$CODEX_BIN" >/dev/null 2>&1; then
  echo "error: Codex CLI not found: $CODEX_BIN" >&2
  echo "install or authenticate Codex, or set CODEX_BIN to its executable" >&2
  exit 1
fi

if [[ ! -f "$MASTER_CHECKLIST" ]]; then
  echo "error: canonical checklist not found: $MASTER_CHECKLIST" >&2
  exit 1
fi

if [[ ! "$MAX_AGENTS" =~ ^[0-9]+$ ]] || (( MAX_AGENTS < 1 || MAX_AGENTS > 30 )); then
  echo "error: ROKO_MAX_AGENTS must be an integer from 1 through 30" >&2
  exit 1
fi

if [[ ! "$AGENT_JOB_TIMEOUT_SECONDS" =~ ^[0-9]+$ ]] ||
  (( AGENT_JOB_TIMEOUT_SECONDS < 60 )); then
  echo "error: ROKO_AGENT_JOB_TIMEOUT_SECONDS must be an integer of at least 60" >&2
  exit 1
fi

mkdir -p "$WORKTREE_ROOT" "$RUN_STATE_ROOT"

GOAL_PROMPT="$(cat <<PROMPT
/goal Execute the complete Roko status-quo remediation programme end to end.

Repository:
  $REPO

Canonical control document:
  tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md

Read that entire document before changing anything. Treat it as the durable control
plane, while verifying every claim against current code, tests, Git history,
manifests, and reproducible behavior.

Carry out the work. Do not merely audit, summarize, plan, or create more unchecked
task lists. Continue through implementation, tests, independent review, commits,
integration, documentation reconciliation, organization, cleanup, and final proof.

Use up to $MAX_AGENTS agent threads with maximum safe concurrency: one coordinator,
one integration/release owner, up to twenty file-disjoint implementation workers,
and the remaining capacity as independent reviewers. Continuously reuse completed
slots. Spawn only dependency-ready, non-overlapping work. Never assign simultaneous
writers to the same files, public APIs, manifests, indexes, lockfiles, runner hot
spots, or integration branch.

First execute Wave 0 exactly as documented: preserve and inventory the dirty root,
create the external recovery bundle, seal the original checkout, create a dedicated
integration worktree/branch, attribute every existing modification and untracked
artifact, reconstruct useful dirty work as task-sized reviewed commits, preserve
unrelated user work, and repair validation/dependency/ownership/duplicate-plan
control-plane defects.

For every task, follow the master's full context, implementation, evidence,
independent-review, commit, merge, and post-merge verification contract. Correct every
rejection. Update canonical statuses and counts only after integrated proof.

As behavior changes, organize code, docs, audits, plans, evidence, and changelogs into
clear canonical locations. Use git mv and update every link, manifest, command, index,
test, and reference in the same reviewed change. Preserve history with explicit
baseline/supersession notices and mappings. Remove duplicates only after proving the
canonical replacement. Keep all docs and generated records synchronized with the
exact integrated code.

Do not stop because the programme is large, tests fail, review rejects work, an agent
finishes, or context compacts. Before context exhaustion, update the durable
coordinator checkpoint, commit only safe coherent work, reread the master, and
continue. Keep all independent lanes moving while one is blocked. Retry a blocking
cause with up to three materially different remedies. Pause the whole programme only
when no meaningful work remains possible without credentials, external authority, or
an irreversible decision unavailable from repository evidence.

Authorization: ALLOW_MAIN_MERGE=yes once the original checkout is safely reconciled
and clean. ALLOW_REMOTE_PUSH=no. ALLOW_PR_MERGE=no. ALLOW_DEPLOY=no.
ALLOW_EXTERNAL_MUTATION=no. Do not publish, push, deploy, rotate secrets, or modify
external services.

Completion requires every canonical self-heal, backlog, DOC, P08-P34, side-queue,
issue, audit, and status outcome to be DONE or rigorously SUPERSEDED with merged proof;
zero unexplained work or contradictions; all evidence committed; all accepted work
merged; every master release gate green; a clean reproducible integration branch
merged locally into main; final gates green again on main; and no programme-created
orphan worktrees, locks, claims, processes, or commits.

Do not end this goal until those completion conditions are genuinely verified.
PROMPT
)"

if (( ${#GOAL_PROMPT} > 4000 )); then
  echo "error: generated /goal exceeds Codex's 4,000-character limit" >&2
  exit 1
fi

KEEP_AWAKE=()
if command -v caffeinate >/dev/null 2>&1; then
  KEEP_AWAKE=(caffeinate -dimsu)
fi

echo "Starting Codex status-quo remediation goal"
echo "  repository: $REPO"
echo "  checklist:  $MASTER_CHECKLIST"
echo "  agents:     up to $MAX_AGENTS"
echo "  worktrees:  $WORKTREE_ROOT"
echo "  remote push/deploy: disabled by prompt"

exec "${KEEP_AWAKE[@]}" "$CODEX_BIN" \
  -C "$REPO" \
  --sandbox workspace-write \
  --add-dir "$WORKTREE_ROOT" \
  --add-dir "$RUN_STATE_ROOT" \
  --ask-for-approval never \
  --search \
  --no-alt-screen \
  --enable multi_agent \
  --enable goals \
  -c "agents.max_threads=$MAX_AGENTS" \
  -c 'agents.max_depth=1' \
  -c "agents.job_max_runtime_seconds=$AGENT_JOB_TIMEOUT_SECONDS" \
  "$GOAL_PROMPT"
