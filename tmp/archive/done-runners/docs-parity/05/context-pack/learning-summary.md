# Learning Summary — Batch 05

Concise runtime picture for agents entering `05` without prior context.

## Current Footprint

- `roko-learn` is already a large shipped subsystem: 42 modules and 35,847 Rust LOC.
- The runtime entry point is still `LearningRuntime::record_completed_run(...)` in `runtime_feedback.rs`.
- Batch `05` is mainly about contract cleanup and truthful scope control, not inventing a new learning architecture.

## Shipped Learning Surfaces

- `episode_logger.rs`, `pattern_discovery.rs`, and `hdc_clustering.rs` already cover episode capture, retention/compaction, and pattern mining.
- `runtime_feedback.rs`, `efficiency.rs`, `task_metric.rs`, `regression.rs`, and `drift.rs` already cover per-run feedback, efficiency events, metrics, regressions, and drift utilities.
- `prediction.rs`, `active_inference.rs`, `cascade_router.rs`, `routing_log.rs`, and `prompt_experiment.rs` already cover routed prediction, calibration inputs, and experiment feedback loops.
- `skill_library.rs`, `playbook.rs`, and `playbook_rules.rs` already provide learned guidance and reusable skill surfaces.
- `roko-neuro/src/tier_progression.rs` is a real tier-progression layer; knowledge tiers are not just a doc concept.

## Highest-Value Near-Term Bridges

- Adding `fingerprint: Option<HdcVector>` to `Engram` remains the clearest ship-now bridge between learning, neuro, and core.
- A typed heuristic calibration struct is a ship-soon follow-on, building on `prediction.rs`, `drift.rs`, `regression.rs`, and `tier_progression.rs`.

## Explicitly Deferred

- Demurrage is not a shipped memory model. If revisited, start with `last_used` and `access_count` on existing decay rather than a new economic substrate.
- Worldview clustering, dissonance algebra, and replication-ledger or Paper/Claim pipelines remain target-state only.
- FEP, Friston, and VSM framing may stay as references, but they are not required to explain the code that already ships.

## What Batch 05 Should Actually Do

1. Separate shipped learning code from planned or research-heavy ideas in the parity materials.
2. Point agents at the real modules already doing the work.
3. Keep cross-crate bridge ideas as explicit handoffs instead of present-tense claims.
