# A — Core Orchestration (Docs 00-06)

Covers: layer overview, plan discovery, task DAG, executor, phases, actions, runtime harness.

The audit correction here is straightforward: the core orchestration loop already exists and is wired. Batch `01` should stop pretending the layer is waiting for its main abstraction.

Two size checks matter more than any new concept diagram:

- `crates/roko-cli/src/orchestrate.rs`: `17,087` lines
- `crates/roko-orchestrator/src/dag.rs`: `1,571` lines
- `crates/roko-orchestrator/src/executor/mod.rs`: `915` lines

---

## A.01 — Layer Overview (Doc 00) — DONE

`PlanRunner` is a real effectful harness around a real `ParallelExecutor`.

The main correction is scale, not status:

- `crates/roko-cli/src/orchestrate.rs` is **17,087 lines**
- `PlanRunner` is large and deeply wired
- conductor, learning, knowledge, skills, process supervision, and runtime persistence are already part of the live loop

The real debt is extraction and surface discipline inside `orchestrate.rs`, not inventing another top-level orchestration concept.

## A.02 — Plan Discovery (Doc 01) — DONE

Plan discovery is runtime-active, not an unclaimed helper.

What is live:

- `discover_plans()` in `roko-orchestrator`
- ranking and frontmatter parsing
- runtime use through `PlanRunner::from_plans_dir()`
- recovery constructors in `PlanRunner` reuse the same orchestration substrate rather than a separate resume-only stack

Batch `01` should not spend time "activating" plan discovery. That activation already happened.

## A.03 — Unified Task DAG (Doc 02) — SHIPPED CODE, SMALL RUNTIME SEAM

`UnifiedTaskDag` is shipped and substantial:

- DAG build logic exists
- `waves()` exists
- `critical_path()` exists
- mutation helpers exist
- the module is heavily tested

The live runtime executor, however, is `ParallelExecutor`, not `UnifiedTaskDag`.

That means the useful batch-01 question is narrow:

- can one live path construct a DAG and expose one DAG-derived signal?

It is not:

- can one batch replace the scheduler,
- can one batch remove `TaskTracker`,
- or can one batch turn docs `02` into the sole owner of execution.

Everything else in doc `02` that reads like scheduler theory or optimizer roadmap should stay future-state in this parity pack.

## A.04 — Parallel Executor (Doc 03) — WIRED, WITH ONE DEAD EDGE

`ParallelExecutor` is the live orchestration core:

- tick/apply loop is real
- concurrency and phase transitions are real
- snapshot/restore is real
- runtime dispatch is real for the core action set

The narrow remaining seam is speculative execution:

- speculative actions exist in the enum and executor code
- runtime dispatch for those actions is still incomplete

Advanced material in doc `03` stays aspirational in this pack:

- resource-budget framework
- priority inversion protocol
- Petri-net formalism

## A.05 — Plan Phases (Doc 04) — DONE

The phase machine is not the problem in this layer.

What is already true:

- the phase enum is real,
- transition logic is real,
- retry bounds are real,
- and phase-to-action mapping is part of the live executor surface.

No batch in `01` should reopen phase-system design.

## A.06 — Executor Actions (Doc 05) — CORE SET LIVE

The core executor vocabulary is runtime-consumed today.

The correction is to separate:

- core actions that are already live,
- from extended actions that exist in the type surface but do not yet have a full runtime path.

Current split:

- core dispatch flow: live
- speculative actions: partial
- DAG mutation action: partial

Do not call the extended actions "wired" just because the enum exists.

## A.07 — Runtime Harness (Doc 06) — DONE

The runtime harness is already responsible for:

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
| `UnifiedTaskDag` | Partial runtime ownership | prove one real DAG use; do not replace the scheduler |
| Speculative actions | Partial | wire dispatch only |
| Snapshot/resume | Done | harden trust checks, not basic restore |
| Runtime harness | Done, oversized | keep `orchestrate.rs` as the extraction target |

## What This Section Explicitly Does Not Claim

- It does not claim DAG scheduling already owns the runtime loop.
- It does not claim speculative execution is policy-complete.
- It does not claim the advanced theory sections in docs `02-03` are runtime commitments.

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
