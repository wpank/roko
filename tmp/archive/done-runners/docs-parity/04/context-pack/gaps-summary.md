# Gap Inventory — 04 Verification

Post-audit gap picture for the verification parity pack.

## Highest-Value Corrections

### 1. The old pack understated shipped verification

- `A-D` should no longer read as if the verification runtime is mostly absent.
- The real task is documentation correction, not feature-program expansion.

### 2. Runtime path and canonical docs drift

- the live path is `orchestrate.rs -> rung_dispatch.rs`
- `rung_selector.rs` and `GatePipeline` remain real abstractions, but they are not the main production entrypoint today

### 3. Artifact/ratchet scope needs narrowing

- artifact store is real but in-memory
- ratchet is real but should not be described as a fully active persisted runtime guardrail

### 4. Thresholds are wired; advanced analytics are not

- EMA updates and persistence are real
- retry/skip advice is observable
- SPC-style analytics belong in future work

### 5. Back-half research needs explicit deferral

- process rewards
- autonomous eval and EvoSkills
- forensic replay and verdict analytics

These should be marked `DEFERRED`, not turned into implementation batches inside `04`.
