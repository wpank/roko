# Repo Map — Shared Verification Context

Quick reference for agents working on `04` verification parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 04 |
|------|------|----------------------------|
| Core gate trait + engrams | `crates/roko-core/src/traits.rs`, `engram.rs`, `decay.rs`, `kind.rs` | gate contract, verdict-signal contract, decay defaults |
| Gate crate entry | `crates/roko-gate/src/lib.rs` | module inventory and re-exports |
| Rung selection | `crates/roko-gate/src/rung_selector.rs` | canonical `PlanComplexity`, `Rung`, `RungCaps`, `select_rungs` |
| Gate pipeline | `crates/roko-gate/src/gate_pipeline.rs` | sequential verification composer currently bypassed by runtime |
| Adaptive thresholds | `crates/roko-gate/src/adaptive_threshold.rs` | retry / skip policy that currently only dashboards consume |
| Feedback classifier | `crates/roko-gate/src/feedback.rs` | structured gate-output summarization for AutoFix |
| Artifact + ratchet | `crates/roko-gate/src/artifact_store.rs`, `ratchet.rs` | long-running verification state |
| Higher-rung gates | `crates/roko-gate/src/generated_test_gate.rs`, `property_test_gate.rs`, `integration_gate.rs`, `symbol_gate.rs`, `diff_gate.rs` | reachability gap |
| Runtime callsites | `crates/roko-cli/src/orchestrate.rs` | primary production wiring hotspot |
| CLI / TUI / HTTP readers | `crates/roko-cli/src/main.rs`, `tui/dashboard.rs`, `crates/roko-serve/src/routes/learning.rs` | these expose threshold state even though runtime ignores it |
| Learning sinks | `crates/roko-learn/src/episode_logger.rs`, `runtime_feedback.rs`, `skill_library.rs` | downstream consumers of gate verdicts |
| Verification docs | `docs/04-verification/` | source material being checked |
| Parity batch | `tmp/docs-parity/04/` | execution contract and findings |

## Important Corrections

Use these instead of older or misleading assumptions:

- `EngramBuilder::new(...)` defaults to `Decay::None`; verdict signals do **not** get a 24h half-life unless the builder path sets one explicitly.
- `Engram::derive(...)` carries lineage, but it does **not** automatically inherit the parent's tags.
- `run_gate_pipeline(plan_id, rung)` currently treats `rung` as an ad-hoc verification level, not as a trustworthy `Rung` enum value.
- `GeneratedTestGate` uses its own `ArtifactStore` trait in `generated_test_gate.rs`; that is distinct from the simpler top-level `artifact_store.rs` type.
- `Kind::GateVerdict` emission in `orchestrate.rs` currently tags `gate` and `passed`, but not the full stronger contract the docs describe.

## Search Priorities

Before editing, search these first:

```bash
rg -n "run_gate_rung|run_gate_pipeline|ExecutorAction::RunGate" crates/roko-cli/src/orchestrate.rs
rg -n "PlanComplexity|RungCaps|select_rungs|GatePipeline" crates/roko-gate crates/roko-cli
rg -n "suggested_max_retries|should_skip_rung|AdaptiveThresholds" crates/roko-cli crates/roko-gate crates/roko-serve
rg -n "feedback_for_agent|GateFeedback|handle_autofix|last_gate_failure" crates/roko-cli crates/roko-gate
rg -n "ArtifactStore|GateRatchet|record_pass|can_regress|\\.roko/artifacts|gate-ratchet" crates/roko-cli crates/roko-gate
rg -n "Kind::GateVerdict|Decay::HalfLife|lineage|signals.jsonl|verify-chain" crates/roko-core crates/roko-cli
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Make the live orchestrator path stronger before expanding gate theory.
2. Do not leave two incompatible notions of rung semantics in the runtime.
3. If a batch only proves one production path, make that path extremely clear and testable.
4. If a task really belongs to learning, eval, or replay analytics, record the handoff and stop.
