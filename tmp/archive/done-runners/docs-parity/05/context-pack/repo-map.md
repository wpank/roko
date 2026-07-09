# Repo Map — Shared Learning Context

Quick reference for agents working on `05` learning parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## Learning Snapshot

- `roko-learn`: 42 modules, 35,847 Rust LOC
- `roko-neuro`: tier progression and durable knowledge-store code already ship
- batch `05` should treat most learning surfaces as real and focus on honesty, bridging, and small contract cleanup

## High-Value Paths

| What | Path | Why It Matters In Batch 05 |
|------|------|----------------------------|
| Runtime learning hub | `crates/roko-learn/src/runtime_feedback.rs` | main fan-out path for completed-run learning updates |
| Episode capture and retention | `crates/roko-learn/src/episode_logger.rs` | proves logging and compaction already exist |
| Patterns and clustering | `crates/roko-learn/src/pattern_discovery.rs`, `hdc_clustering.rs` | distinguishes shipped mining from deferred analytics ideas |
| Prediction and calibration substrate | `crates/roko-learn/src/prediction.rs`, `active_inference.rs`, `routing_log.rs` | grounds feedback-loop claims in real code |
| Routing and experiments | `crates/roko-learn/src/cascade_router.rs`, `prompt_experiment.rs`, `model_experiment.rs` | shows routing-bandit work is already wired |
| Metrics and health | `crates/roko-learn/src/efficiency.rs`, `regression.rs`, `cost_table.rs`, `provider_health.rs`, `latency.rs` | proves metrics, cost normalization, and health surfaces are not speculative |
| Learned guidance | `crates/roko-learn/src/playbook.rs`, `playbook_rules.rs`, `skill_library.rs` | shipped learning artifacts that docs must describe accurately |
| Knowledge tiers | `crates/roko-neuro/src/tier_progression.rs`, `knowledge_store.rs` | proves tier progression already exists outside `roko-learn` |
| HDC bridge primitives | `crates/roko-primitives/src/hdc.rs`, `crates/roko-neuro/src/hdc.rs` | supports the carry-forward case for an `Engram` fingerprint field |
| Learning docs and audit | `docs/05-learning/`, `tmp/refinements-audit/02-learning-audit.md` | source of truth for parity corrections |
| Parity materials | `tmp/docs-parity/05/` | files being refreshed in this batch |

## Important Corrections

- `roko-learn` is not a small or early-stage crate; it is already one of the larger subsystems in the workspace.
- `prediction.rs`, `active_inference.rs`, `cascade_router.rs`, `prompt_experiment.rs`, `drift.rs`, `pattern_discovery.rs`, `runtime_feedback.rs`, `efficiency.rs`, and `regression.rs` already exist and should be written about in present tense.
- tier progression already lives in `roko-neuro`; knowledge tiers are not just a learning-doc aspiration.
- demurrage, worldviews, and replication-ledger ideas still have no code and must stay explicitly deferred.

## Search Priorities

Before editing, search these first:

```bash
rg -n "active_inference|prediction|cascade_router|prompt_experiment|drift|pattern_discovery|runtime_feedback|efficiency|regression" crates/roko-learn/src
rg -n "TierProgression|HeuristicRule|KnowledgeTier|PatternMiner" crates/roko-neuro/src
rg -n "fingerprint|HdcVector|query_similar|Engram" crates/roko-core crates/roko-neuro crates/roko-primitives
rg -n "demurrage|worldview|replication-ledger|FEP|Friston|Viable System Model" docs/05-learning tmp/docs-parity/05 tmp/refinements-audit
```

## Verification Baseline

```bash
bash -n tmp/docs-parity/05/run-docs-parity.sh
```

## Practical Rules

1. Use code paths as evidence before changing prose.
2. Prefer small bridge notes over new subsystem proposals.
3. Mark zero-code concepts as target-state or deferred every time.
4. Keep each batch small enough for one Codex agent to verify without guesswork.
