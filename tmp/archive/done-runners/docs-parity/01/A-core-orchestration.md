# A — Core Orchestration (Docs 00-06)

Covers: layer overview, plan discovery, task DAG, executor, phases, actions, runtime harness.

The audit correction here is simple: the core orchestration loop already exists and is already wired. Batch `01` should stop pretending this layer is waiting for a new central abstraction.

Three numbers matter more than any new concept diagram:

- `crates/roko-cli/src/orchestrate.rs`: **17,087** lines
- `crates/roko-orchestrator/src/dag.rs`: **1,571** lines
- `crates/roko-orchestrator/src/executor/mod.rs`: **915** lines

---

## A.01 — Layer Overview (Doc 00) — WIRED

`PlanRunner` is a real effectful harness around a real `ParallelExecutor`.

The main correction is scale, not status:

- `orchestrate.rs` is the main integration hotspot
- conductor, learning, worktrees, recovery, and dispatch are already on the live path
- the useful follow-on work is extraction and seam discipline, not another top-level orchestration concept

## A.02 — Plan Discovery (Doc 01) — DONE

Plan discovery is runtime-active, not an unclaimed helper.

What is already live:

- `discover_plans()` in `roko-orchestrator`
- ranking and frontmatter parsing
- runtime use through `PlanRunner::from_plans_dir()`
- resume constructors reusing the same orchestration substrate

Batch `01` should not spend time "activating" plan discovery. That activation already happened.

## A.03 — Unified Task DAG (Doc 02) — SHIPPED CODE, SMALL RUNTIME SEAM

`UnifiedTaskDag` is shipped and substantial:

- DAG build logic exists
- `waves()` exists
- `critical_path()` exists
- mutation helpers exist
- the module is heavily tested

The live runtime executor, however, is still `ParallelExecutor`, not `UnifiedTaskDag`.

That makes the useful batch-01 question narrow:

- can one live path construct a DAG and expose one DAG-derived signal?

It does not justify:

- replacing the scheduler,
- removing `TaskTracker`,
- or treating doc `02` as if it already owns execution.

Everything in doc `02` that reads like optimizer theory or future scheduler design should stay target-state in this parity pack.

## A.04 — Parallel Executor (Doc 03) — WIRED, WITH ONE REAL EDGE

`ParallelExecutor` is the live orchestration core:

- tick/apply loop is real
- concurrency and phase transitions are real
- snapshot/restore is real
- runtime dispatch is real for the core action set

The narrow remaining seam is speculative execution:

- speculative actions exist in the executor vocabulary
- the runtime path for those actions is still incomplete

Advanced material in doc `03` stays aspirational here:

- resource-budget framework
- priority inversion protocol
- Petri-net formalism

## A.05 — Plan Phases (Doc 04) — DONE

The phase machine is not the problem in this layer.

What is already true:

- the phase enum is real
- transition logic is real
- retry bounds are real
- phase-to-action mapping is part of the live executor surface

No batch in `01` should reopen phase-system design.

## A.06 — Executor Actions (Doc 05) — CORE SET LIVE

The important correction is to split:

- core actions that already have runtime dispatch,
- from extended actions that exist in the type surface but do not yet have a full live path.

Current posture:

- core dispatch flow: live
- speculative actions: partial
- DAG mutation actions: partial

Do not call the extended actions "wired" just because the enum exists.

## A.07 — Runtime Harness (Doc 06) — DONE, OVERSIZED

The runtime harness already owns:

- plan discovery
- executor ticking
- agent dispatch
- gate and verify flow
- snapshot save
- snapshot resume
- event-log persistence
- worktree cleanup
- conductor polling
- learning and knowledge hooks

This is where the audit lands hardest: the orchestration problem is concentrated in one very large integration file, not in missing first-order runtime plumbing.

It is also where most batch conflicts will happen, so any active batch touching `orchestrate.rs` should prove one seam and stop.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Layer split | Done | keep the state-machine vs harness distinction honest |
| Plan discovery | Done | stop treating it as pending |
| `ParallelExecutor` | Done | use it as the live baseline |
| `UnifiedTaskDag` | Shipped, not runtime-owned | prove one live use without rewriting scheduling |
| Speculative actions | Partial | wire dispatch only |
| Snapshot/resume | Done | harden trust checks, not basic restore |
| Runtime harness | Done, oversized | keep `orchestrate.rs` as the extraction target |

## What This Section Explicitly Does Not Claim

- It does not claim DAG scheduling already owns the runtime loop.
- It does not claim speculative execution is policy-complete.
- It does not claim the advanced theory in docs `02-03` is already on the runtime path.

---

## Batch Guidance

### O2 — Speculative Actions

Good batch outcome:

- both speculative action variants become runtime-reachable,
- one test proves the path,
- policy work stays deferred.

### O3 — Live DAG Surface

Good batch outcome:

- construct `UnifiedTaskDag` on one production path,
- use `waves()` or `critical_path()` once,
- stop before turning this into a scheduler rewrite.

### What Not To Do

- do not rewrite `orchestrate.rs` in one batch
- do not declare DAG ownership complete because the module exists
- do not pull in domain orchestration work from docs `12-13`
