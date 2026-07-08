# Agent Runbook — Batch 01

Use this when executing any active batch from `tmp/docs-parity/01/`.

## Mission

Land one small orchestration runtime improvement without reopening already-wired plumbing.

Each good batch outcome has:

- one live path
- one verify target
- one explicit deferral
- one sentence explaining why the work stayed small

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and confirm the real code paths before editing.
3. Choose the smallest patch that proves the batch goal on a live runtime path.
4. Keep the change inside the named scope from [BATCHES.md](../BATCHES.md).
5. Run the verify command.
6. If the work expands into architecture cleanup, stop and defer it.
7. If docs `12-13` become part of the argument, you are probably outside batch `01`.

## Default Decision Rules

- If the runtime path already exists, harden it instead of rebuilding it.
- If a type or helper already exists, wire one live path before inventing a new abstraction.
- If the change starts to spread across too many responsibilities inside `orchestrate.rs`, cut the scope down.
- If a task touches docs `12-13`, it is outside batch `01`.
- If a finding is really about event taxonomy or bus shape, record it as carry-forward instead of widening the batch.
- If a subsystem is implemented but not clearly on the live path, document that distinction instead of flattening it to "wired."

## Things You Should Assume Are Already True

- plan discovery is wired
- snapshot/resume is wired
- worktree lifecycle is wired
- conductor baseline is wired
- the shared runtime bus has only `PlanRevision` and `PrdPublished`

Do not spend batch time re-litigating those points.

## Good Batch Shapes

- reject bad persisted state before restore
- dispatch an already-defined executor action
- use an already-defined DAG output once
- turn one background finding into one bounded runtime effect
- refresh one existing worktree health signal

## Failure Modes To Avoid

- treating batch `01` like a scheduler rewrite
- using docs `12-13` to justify new domain architecture
- conflating the local event log with the shared runtime event bus
- calling a feature "unwired" when it is already live but imperfect
- calling a feature "wired" when it only exists as supporting code
- treating `O6` like a hidden implementation batch
