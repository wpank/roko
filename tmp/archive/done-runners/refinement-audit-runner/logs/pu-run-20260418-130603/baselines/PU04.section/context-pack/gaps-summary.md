# Gap Inventory — 04 Verification

Concise gap list for agents working on verification parity batches.

## Focus Now

These are the gaps batch `04` should actively try to close:

### 1. Runtime Rung Semantics Are Not Honest — HIGH

- `Rung` and `select_rungs` are canonical in `roko-gate`,
- `orchestrate.rs` still uses an ad-hoc numeric `match`,
- `Test` and `Lint` are currently swapped on the runtime path.

### 2. Selector / Pipeline / Feedback Code Exists But Runtime Bypasses It — HIGH

- `GatePipeline` is built and tested,
- `feedback_for_agent` is built and tested,
- production paths still bypass both.

### 3. Adaptive Thresholds Learn But Do Not Act — HIGH

- update and persistence are wired,
- retry and skip behavior do not consume the stored policy.

### 4. Long-Running Verification State Is Too Weak — MEDIUM

- `ArtifactStore` is in-memory only,
- `GateRatchet` has no runtime caller and no persistence path.

### 5. Verdict Signals Need A Stronger Contract — MEDIUM

- emission exists,
- explicit decay and tag propagation do not,
- lineage is structural but not actively verified.

## Defer From Batch 04

These are valid findings, but they should usually be documented and handed off:

- Promise / Progress / PRM work -> `05`
- autonomous test-writer agents -> `05` or later eval pass
- advanced EvoSkills -> `05` or research pass
- replay analytics / root-cause / what-if -> later retrospective pass
- predictive gate selection / replanning -> later planning pass
- SPC detector stack -> later analytics hardening

## Working Rule

If a verification task requires:

- a reward model,
- a new agent role for test generation,
- or a full replay-analysis system,

then batch `04` should normally implement the smallest verification-layer contract and defer the rest.
