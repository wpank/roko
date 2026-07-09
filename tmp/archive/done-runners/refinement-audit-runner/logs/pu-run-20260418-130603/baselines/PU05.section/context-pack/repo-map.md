# Repo Map — Shared Learning Context

Quick reference for agents working on `05` learning parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 05 |
|------|------|----------------------------|
| Runtime learning hub | `crates/roko-learn/src/runtime_feedback.rs` | main fan-out path for completed-run learning updates |
| Episode logger | `crates/roko-learn/src/episode_logger.rs` | retention, schema, compact behavior |
| Patterns / clustering | `crates/roko-learn/src/pattern_discovery.rs`, `hdc_clustering.rs` | real pattern-mining surfaces vs doc clustering claims |
| Playbook rules | `crates/roko-learn/src/playbook_rules.rs` | richer trigger surface currently underused in production |
| Skills | `crates/roko-learn/src/skill_library.rs` | `search_by_tag` vs `SkillQuery::select`, pruning contract |
| Regression | `crates/roko-learn/src/baseline.rs`, `regression.rs` | slice-aware baselines, overall-only alerts gap |
| Prediction / routing logs | `crates/roko-learn/src/prediction.rs`, `routing_log.rs` | calibration contract and routing-log replay path |
| Routing core | `crates/roko-learn/src/model_router.rs`, `cascade_router.rs`, `budget.rs` | cost pressure, experiments, routing behavior |
| Dead scaffolding | `crates/roko-learn/src/event_subscriber.rs`, `drift.rs` | dead-module resolution batch |
| Predictive policy consumers | `crates/roko-core/src/prediction.rs`, `crates/roko-cli/src/orchestrate.rs` | predictive prompt sections and scorer path |
| Main orchestrator | `crates/roko-cli/src/orchestrate.rs` | learned-context assembly and routing-log append path |
| Prompt injection | `crates/roko-compose/src/role_prompts.rs`, `system_prompt_builder.rs` | downstream learning consumers |
| Learning docs | `docs/05-learning/` | source material being checked |
| Parity batch | `tmp/docs-parity/05/` | execution contract and findings |

## Important Corrections

Use these instead of older or misleading assumptions:

- `EpisodeLogger::compact(...)` already exists; the runtime is not purely “append forever with no retention story”.
- `build_learned_context(...)` in `orchestrate.rs` currently populates `MatchContext` with `role` only and uses `search_by_tag(role)` for skills.
- `detect_regressions(...)` computes only overall alerts today even though slice baselines already exist.
- `CalibrationTracker` is not dead: it is loaded from routing logs and already feeds predictive policy / scoring surfaces. What is unused is the direct `PredictionRecord::register/resolve` path.
- `run_learning_subscriber(...)` and `DriftDetector` still have no production caller.

## Search Priorities

Before editing, search these first:

```bash
rg -n "build_learned_context|MatchContext|search_by_tag|SkillQuery|playbook_rules\\(\\)\\.select" crates/roko-cli crates/roko-learn
rg -n "detect_regressions|slice: None|iterations_increase|Baseline::lookup|slices" crates/roko-learn crates/roko-cli
rg -n "PredictionRecord|CalibrationTracker|PredictionPolicy|PredictiveScorer|routing_log|brier|reliability" crates/roko-learn crates/roko-core crates/roko-cli
rg -n "run_learning_subscriber|DriftDetector" crates/roko-learn crates/roko-cli
rg -n "BudgetGuardrail|BudgetAction|apply_cost_pressure|on_experiment_concluded|experiments.json" crates/roko-learn crates/roko-cli
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Strengthen current runtime learning loops before adding new ones.
2. Prefer one canonical source of truth for a learning signal.
3. If a batch only proves one production path, make that path extremely explicit and testable.
4. If a task really belongs to routing research, storage architecture, or governance, record the handoff and stop.
