# Repo Map — Shared Verification Context

Quick reference for the refreshed `04` parity pack.

## Scope

- Docs being refreshed: `docs/04-verification/`
- Parity materials being edited: `tmp/docs-parity/04/`
- Live runtime anchor: `crates/roko-cli/src/orchestrate.rs`

## High-Value Paths

| What | Path | Why it matters now |
|---|---|---|
| Gate trait and engram contract | `crates/roko-core/src/traits.rs`, `engram.rs`, `decay.rs` | current kernel behavior and verdict-signal contract |
| Gate inventory | `crates/roko-gate/src/lib.rs` | verification surface is real, not hypothetical |
| Live runtime rung mapping | `crates/roko-gate/src/rung_dispatch.rs` | best anchor for the current 7-rung execution path |
| Canonical selector model | `crates/roko-gate/src/rung_selector.rs` | real abstraction, but secondary to live runtime path |
| Reusable gate pipeline | `crates/roko-gate/src/gate_pipeline.rs` | real library abstraction, not the main dispatch entrypoint |
| Thresholds | `crates/roko-gate/src/adaptive_threshold.rs` | EMA tracking and persistence |
| Feedback classifier | `crates/roko-gate/src/feedback.rs` | structured gate-output foundation |
| Artifact store / ratchet | `crates/roko-gate/src/artifact_store.rs`, `ratchet.rs` | partial foundations that need narrowed wording |
| Runtime callsites | `crates/roko-cli/src/orchestrate.rs` | executor/plan gate execution, episodes, threshold updates, verdict signals |
| Learning sinks | `crates/roko-learn/src/episode_logger.rs`, `runtime_feedback.rs`, `skill_library.rs` | proof that gate results feed downstream records |
| Threshold views | `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/tui/dashboard.rs`, `crates/roko-serve/src/routes/learning.rs` | proof that threshold state is surfaced outside tests |

## Important Corrections

- Verification core is mostly shipped.
- The best runtime explanation is `orchestrate.rs -> rung_dispatch.rs`.
- `GatePipeline` and `rung_selector` should not be described as the sole production path.
- `ArtifactStore` and `GateRatchet` are real modules, but their broader persisted/runtime role is still limited.
- E/F/G research-heavy concepts must be labeled as deferred rather than current gaps.
