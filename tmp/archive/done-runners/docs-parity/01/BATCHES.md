# Batch Execution Contract

Run: `PU01` (`pu-run-20260418-130603`)

This file defines the post-audit execution contract for the orchestration parity pack.

These are **small code batches**, not a hidden roadmap for rebuilding orchestration. Each active batch should fit inside one focused 30-90 minute session, touch one live path, and leave one explicit deferral behind.

Active set: `O1-O5`  
Deferred lane only: `O6`

---

## Batch Posture

- Prefer runtime wiring and hardening over new orchestration concepts.
- Treat `crates/roko-cli/src/orchestrate.rs` as the conflict hotspot and extraction target.
- Do not reopen plan discovery, snapshot/resume, worktree lifecycle, merge queue, or conductor baseline as if they still need building.
- If a task grows into a new planning model, domain framework, or distributed design, cut it back or defer it.
- Keep docs `12-13` out of the active queue.
- Keep event-unification and generic-bus work as carry-forward notes, not orchestration batch scope.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/orchestration-summary.md](context-pack/orchestration-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

---

## Recommended Serial Order

`O1 -> O5 -> O2 -> O3 -> O4`

This order hardens trust boundaries first, then unattended-runtime hygiene, then the smaller `orchestrate.rs` seams most likely to conflict.

`O6` is deferred and is not part of the default execution order.

---

## Batch Overview

| Batch | Time Box | Purpose | Primary Write Scope | Verify Focus | Status |
|-------|----------|---------|---------------------|--------------|--------|
| O1 | 60-90 min | Reject bad recovery inputs before restore | `roko-orchestrator`, maybe `roko-cli` | `cargo test -p roko-orchestrator -p roko-cli` | Active |
| O2 | 45-90 min | Make speculative executor actions reachable from one runtime path | `roko-cli`, maybe `roko-orchestrator` | `cargo test -p roko-cli -p roko-orchestrator` | Active |
| O3 | 60-90 min | Expose one operator-visible DAG signal on one live path | `roko-cli`, maybe `roko-orchestrator` | `cargo test -p roko-cli -p roko-orchestrator` | Active |
| O4 | 45-90 min | Route one background conductor finding into one bounded runtime effect | `roko-cli`, maybe `roko-conductor` | `cargo test -p roko-cli -p roko-conductor` | Active |
| O5 | 30-60 min | Tighten worktree liveness with one safe health check | `roko-cli`, maybe `roko-orchestrator` | `cargo test -p roko-cli -p roko-orchestrator` | Active |
| O6 | — | Preserve the deferral boundary for docs `12-13` | docs/parity only | none | Deferred |

---

## Dependency Notes

| Batch | Depends on | Notes |
|-------|------------|-------|
| O1 | — | independent trust-boundary work |
| O2 | — | small executor/runtime seam |
| O3 | — | small DAG/runtime seam |
| O4 | O2 or O3 landing first is helpful, not required | shares the same hotspot in `orchestrate.rs` |
| O5 | — | small runtime hygiene patch |
| O6 | — | not executable in batch `01` |

Parallel-safe starts:

- `{O1, O5}` are the least coupled
- `O2` and `O3` can start early, but they are more likely to conflict in `orchestrate.rs`
- `O4` should stay small even if it lands after `O2` or `O3`

---

## Batch Details

### O1 — Recovery Trust Boundary

**Owns**:

- `C.01`
- the recovery seam called out in `C.02`

**Read first**:

- [C-persistence-recovery.md](C-persistence-recovery.md)
- [A-core-orchestration.md](A-core-orchestration.md)

**Problem**:

Snapshot/resume already works, but the runtime still trusts persisted state too early.

**Scope**:

1. Validate snapshot inputs before restore.
2. Call event-log integrity checks during recovery or resume where safe.
3. Add one corruption or truncation test that proves bad state is rejected.

**Out of scope**:

- delta snapshots
- Merkle verification
- CRDT or distributed recovery
- broad persistence redesign

**Files**:

- `crates/roko-orchestrator/src/executor/snapshot.rs`
- `crates/roko-orchestrator/src/executor/recovery.rs`
- `crates/roko-orchestrator/src/event_log.rs`
- `crates/roko-cli/src/orchestrate.rs` only if the live recovery path needs the hook

**Verify**:

```bash
cargo test -p roko-orchestrator -p roko-cli
```

**Acceptance criteria**:

- corrupted or truncated persisted state is rejected,
- the recovery path makes an integrity decision before trusting saved state,
- and the rest of the persistence roadmap stays deferred.

### O2 — Speculative Action Dispatch

**Owns**:

- `A.04`
- the extended-action seam in `A.06`

**Read first**:

- [A-core-orchestration.md](A-core-orchestration.md)

**Problem**:

Speculative executor actions exist, but at least one live runtime path still cannot reach them.

**Scope**:

1. Dispatch `StartSpeculativeExecution`.
2. Dispatch `CancelSpeculativeExecution`.
3. Prove one runtime path can hit those actions.

**Out of scope**:

- new speculation heuristics
- resource-budget framework
- priority inversion protocol
- executor redesign

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/executor/mod.rs` only if a tiny helper is needed

**Verify**:

```bash
cargo test -p roko-cli -p roko-orchestrator
```

**Acceptance criteria**:

- both speculative actions are runtime-reachable,
- one test or dry path proves the dispatch,
- and the remaining policy work stays deferred.

### O3 — Live DAG Surface

**Owns**:

- `A.03`

**Read first**:

- [A-core-orchestration.md](A-core-orchestration.md)
- [context-pack/orchestration-summary.md](context-pack/orchestration-summary.md)

**Problem**:

`UnifiedTaskDag` is shipped code, but the main runtime loop is not DAG-owned.

**Scope**:

1. Construct `UnifiedTaskDag` on one live path.
2. Use `waves()` or `critical_path()` in one operator-visible way.
3. Stop once the chosen path proves the DAG matters at runtime.

**Out of scope**:

- replacing `TaskTracker`
- comprehensive scheduler replacement
- broad mutation plumbing
- partitioning or incremental-DAG redesign

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/dag.rs` only if a tiny helper is needed

**Verify**:

```bash
cargo test -p roko-cli -p roko-orchestrator
cargo run -p roko-cli -- plan run plans/ --dry-run
```

**Acceptance criteria**:

- one live path constructs a DAG,
- one operator-visible surface uses DAG output,
- and the scheduler-rewrite story stays deferred.

### O4 — Background Conductor Response

**Owns**:

- `D.01`

**Read first**:

- [D-monitoring-conductor.md](D-monitoring-conductor.md)
- [A-core-orchestration.md](A-core-orchestration.md)

**Problem**:

Background watcher findings still stop at logging too often, even though the conductor baseline is already wired.

**Scope**:

1. Pick one safe background finding.
2. Route it into one bounded runtime effect beyond logging.
3. Keep the change compatible with the existing conductor surface.

**Out of scope**:

- autonomous remediation framework
- diagnosis redesign
- learning-policy overhaul
- fixing the conductor/learn layering in this batch

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/conductor.rs` only if the bounded effect needs a small helper

**Verify**:

```bash
cargo test -p roko-cli -p roko-conductor
```

**Acceptance criteria**:

- one background finding has one bounded runtime effect,
- the effect is observable or testable,
- and the layering issue is documented, not solved by scope creep.

### O5 — Worktree Runtime Hygiene

**Owns**:

- `B.01`

**Read first**:

- [B-isolation-merge.md](B-isolation-merge.md)

**Problem**:

The worktree lifecycle is already wired, but unattended runs still underuse liveness and health signals.

**Scope**:

1. Touch active worktrees during execution.
2. Consult one safe health check where it adds value.
3. Keep the feature opt-in under `use_worktrees`.

**Out of scope**:

- changing the default to `use_worktrees=true`
- merge-queue redesign
- broad worktree policy changes

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/worktree.rs`

**Verify**:

```bash
cargo test -p roko-cli -p roko-orchestrator
```

**Acceptance criteria**:

- active worktrees get meaningful liveness updates,
- one health signal is consulted safely,
- and the feature remains opt-in.

### O6 — Deferred Coordination And Domain Work

Docs `12-13` are not executable batch-01 work, and `O6` should never be treated as a code-change lane inside this pack.

Keep only the deferral boundary:

- no cross-domain execution batch,
- no formal stigmergy implementation batch,
- no template, saga, or semantic-merge work,
- no chain-domain runtime claim.

If later work needs these ideas, it should start a new parity pack or roadmap item instead of widening `01`.
