# Orchestration Summary — Batch 01

Use this as the quick orientation note before touching orchestration runtime code.

## Core Split

- `PlanRunner` in `roko-cli` is the live effectful harness.
- `ParallelExecutor` in `roko-orchestrator` is the live runtime state machine.
- `UnifiedTaskDag` is shipped support code, not the owner of the main loop.

## The Main Audit Correction

The orchestration problem is not "missing abstractions."

It is:

- `crates/roko-cli/src/orchestrate.rs` at **17,087** lines
- too much integration responsibility in one file
- a few real but narrow runtime seams that should be wired or extracted one at a time

Treat `orchestrate.rs` as the real debt and the real conflict hotspot.

Use that fact to stay disciplined: batch `01` is about shaving off one provable seam, not redesigning the orchestration layer around it.

## What Is Already Live

- plan discovery
- executor tick/apply loop
- snapshot save
- snapshot resume
- worktree lifecycle
- merge queue
- conductor checks and background watcher runner

Do not spend batch `01` re-proving those features from scratch.

Also do not confuse "already wired" with "perfectly factored." The runtime can be both live and too concentrated in one file.

## What Is Worth Doing In Batch 01

- validate persisted state before restore
- make speculative executor actions runtime-reachable
- expose one real DAG-derived signal
- route one background conductor finding into one bounded runtime effect
- improve worktree liveness and one safe health check

Each of those is small enough to prove without pretending the whole orchestration design is unfinished.

## What Is Not Batch-01 Work

- formal stigmergy
- cross-domain execution
- template systems
- saga coordination
- semantic merge
- plan-repair framework
- distributed recovery

Docs `12-13` are deferred because they do not describe live batch-01 runtime work.

## Working Split

- `live now`: present tense is allowed
- `small seam`: valid `O1-O5` work only
- `deferred`: anything that turns into theory, domain expansion, or cross-arc cleanup
