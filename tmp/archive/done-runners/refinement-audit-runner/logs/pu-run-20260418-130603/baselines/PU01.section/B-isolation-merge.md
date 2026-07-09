# B — Isolation & Merge (Docs 07-08)

Covers: worktree isolation and merge queue behavior.

The audit correction here is to stop talking like these subsystems still need to be built. They are already present. The remaining work is unattended-runtime hygiene.

---

## B.01 — Worktree Isolation (Doc 07) — WIRED

Worktree support is already in the runtime:

- startup prune and idle reclaim are called
- `ensure_for_plan()` is used to provision plan workdirs
- cleanup removes tracked worktrees on completion or failure
- the feature remains gated behind `use_worktrees`

What is not yet strong enough for long unattended runs:

- active execution does not always refresh liveness aggressively enough
- runtime health checks are underused
- the runtime still keeps this feature opt-in through `ExecutorConfig.use_worktrees`

That is a small hardening seam, not a missing subsystem.

## B.02 — Merge Queue (Doc 08) — WIRED

The merge queue and post-merge machinery are already implemented.

Important correction:

- the queue has richer conflict-aware behavior than the runtime currently exercises,
- but sequential merge execution is a valid conservative runtime strategy,
- and this file should not imply a bug just because non-conflicting parallel merges are not enabled.

Batch `01` should not widen into merge redesign.

It should also not mislabel the current conservative posture as "missing merge support." The queue exists; the runtime is choosing safety over wider parallel merge behavior.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Worktree lifecycle | Done | touch active worktrees and consult one safe health signal without changing the opt-in default |
| Merge queue | Done | keep sequential merge as the default safe runtime posture |
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
- queue parallelization
- semantic merge strategies
- broader VCS policy changes
