# B — Isolation & Merge (Docs 07-08)

Covers: worktree isolation and merge queue behavior.

The audit correction here is to stop talking like these subsystems still need to be built. They are already present. The remaining work is unattended-runtime hygiene and honest status labeling.

---

## B.01 — Worktree Isolation (Doc 07) — WIRED

Worktree support is already on the runtime path:

- `ensure_for_plan()` provisions per-plan workdirs
- `touch()` is already called for tracked worktrees
- `prune()` and `reclaim_idle()` already run during execution
- cleanup removes tracked worktrees on completion or failure
- the feature remains gated behind `use_worktrees`

What is still worth doing:

- refresh liveness more consistently during active execution
- consult one safe health signal where it adds operational value
- keep the work opt-in instead of widening into policy changes

That is a small hardening seam, not a missing subsystem.

## B.02 — Merge Queue (Doc 08) — IMPLEMENTED, NOT CLEARLY LIVE

The merge queue exists as shipped code in `roko-orchestrator`.

Important correction:

- `MergeQueue` and `PostMergeRunner` are implemented,
- but the current `orchestrate.rs` path appears to merge conservatively through direct merge helpers rather than through `MergeQueue`,
- and this file should not claim queue-driven runtime ownership that the main path does not clearly show.

Batch `01` should still not widen into merge redesign.

The honest posture is:

- worktree lifecycle is live,
- merge support exists,
- queue-level coordination is implemented but not clearly the main runtime control point today.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Worktree lifecycle | Done | touch active worktrees and consult one safe health signal without changing the opt-in default |
| Merge queue | Implemented | document it as available code, not clearly the live merge coordinator |
| Post-merge runner | Done | do not widen into semantic merge or queue parallelization |

---

## Batch Guidance

### O5 — Worktree Runtime Hygiene

Good batch outcome:

- active worktrees get touched during execution,
- one safe health check is consulted,
- the feature remains opt-in.

### Source Notes

- `WorktreeManager::ensure_for_plan()` is already the runtime entry point.
- `touch()`, `check_health()`, `reclaim_idle()`, and `prune()` already exist in `roko-orchestrator`.
- cleanup hooks already exist in `orchestrate.rs`; the gap is runtime hygiene, not feature construction.

### What To Defer

- changing the default to `use_worktrees=true`
- wiring `MergeQueue` into the current runtime path
- semantic merge strategies
- broader VCS policy changes
