# Gap Summary — 05 Learning

Concise gap list for agents working on learning parity batches.

## Focus Now

### 1. The Docs Still Undersell How Much Learning Code Already Ships — HIGH

- parity materials should start from `roko-learn` as a 42-module, 35,847-LOC subsystem,
- `active_inference`, `prediction`, `cascade_router`, `prompt_experiment`, `drift`, `pattern_discovery`, `runtime_feedback`, `efficiency`, and `regression` are already real modules, not roadmap placeholders.

### 2. The Best Near-Term Bridge Is Cross-Crate, Not A New Learning Theory — HIGH

- the highest-value learning-adjacent change is still the `Engram` HDC fingerprint field,
- that is a small bridge into `roko-core` and `roko-neuro`, not a batch-05 architecture rewrite.

### 3. Heuristic Calibration Is Ship Soon, Not Missing From Scratch — MEDIUM

- `prediction.rs`, `drift.rs`, `regression.rs`, and `roko-neuro/src/tier_progression.rs` already provide the substrate,
- the missing piece is a tighter typed calibration contract, not a whole new worldview system.

### 4. Some Framework Language Still Reads As Runtime Doctrine — MEDIUM

- FEP, Friston, and VSM are academic framing around modules that already exist,
- parity docs should describe those ideas as interpretation, not as required engineering work.

### 5. Demurrage, Worldviews, And Replication-Ledger Work Must Stay Deferred — HIGH

- they still have no code,
- batch `05` should label them as target-state or future work instead of implying they already shape runtime behavior.

## Defer From Batch 05

- full demurrage economics -> later decay hardening, starting from `last_used` and `access_count`
- worldview clustering / dissonance algebra -> later heuristics research
- replication-ledger / Paper / Claim ingestion -> later research-to-runtime pass
- c-factor as a canonical learning doctrine -> keep current metrics, defer broader theory
- exponential scaling, ADAS, and EvoSkills -> research pass

## Working Rule

If a learning task requires inventing a new memory economy, worldview layer, or academic doctrine to justify code that already exists, batch `05` should record the handoff and stop.
