# Gaps Summary — Batch 01

This is the post-audit gap picture for orchestration.

The important change is that the gap list is now narrow. Batch `01` is not trying to "finish orchestration."

Treat every gap below as a runtime seam, not as evidence that the architecture still needs its core abstractions.

## Focus Now

### 1. Recovery trust boundary

- snapshot/resume is already live
- the gap is validating persisted state before restore

### 2. Event-log guarding in recovery

- integrity support already exists
- the gap is calling it from the real recovery path where safe

### 3. Speculative action dispatch

- speculative executor actions already exist
- the gap is runtime reachability

### 4. One live DAG surface

- DAG support already exists
- the gap is one real runtime use, not scheduler replacement

### 5. Background conductor response

- conductor baseline already exists
- the gap is one bounded runtime effect beyond logging

### 6. Worktree liveness and one health signal

- worktree lifecycle already exists
- the gap is unattended-runtime hygiene

### 7. Layering honesty

- `roko-conductor -> roko-learn` is a real boundary crossing
- the orchestration pack should name it directly instead of covering it with more design prose

### 8. Bus honesty

- the local orchestration event log is richer than the shared runtime bus
- the shared runtime bus still has only `PlanRevision` and `PrdPublished`
- batch `01` should document that boundary, not solve event unification

## Carry Forward

- event-enum unification / generic bus work
- domain-specific gate suites
- domain-specialized agent behavior
- adaptive routing or reward by domain
- conductor/learn boundary cleanup
- local event-log vs shared-bus mismatch beyond documentation

## Defer From Batch 01

- formal stigmergy
- cross-domain orchestration
- chain-domain runtime work
- templates, sagas, semantic merge, repair engines
- distributed executor or recovery models

## Working Rule

If a proposed fix requires:

- a new orchestration abstraction,
- a new domain model,
- or a broad architecture cleanup across crates,

then it is outside batch `01` unless it can be cut down to one provable runtime seam.
