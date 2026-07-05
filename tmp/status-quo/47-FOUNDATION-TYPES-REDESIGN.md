# Foundation Types Redesign

> Status-quo audit · verified 2026-07-08 · git HEAD 5852c93c0 · sources: `roko-core/src/foundation.rs`, `roko-core/src/dispatch_plan.rs`, `roko-cli/src/dispatch/mod.rs` + `dispatch_v2.rs`, `roko-runtime/src/{run_ledger,pipeline_state,workflow_engine,effect_driver}.rs`, `roko-gate/src/registry.rs`, `roko-cli/src/inline/primitives/gate_block.rs`, `roko-learn/src/model_router.rs`, plus rg census of every consumer. Origin: `tmp/subsystem-audits/05-01`.

## Summary

`tmp/subsystem-audits/05-01` called for a set of foundational types. The current code has
them, and since the original audit a real **shared contract module now exists** —
`roko-core/src/foundation.rs` (561 LOC) — re-exported from `roko-core/src/lib.rs:217`. It holds
`ModelCallRequest`, `TokenUsage`, `TokenBudget`, `GateVerdict`, `GateReport`, `GateConfig`,
`Effect`, `EffectOutcome`, `CachePolicy`, `PromptSpec`, `FeedbackEvent`, `DispatchModulation`,
etc. This is genuine progress toward Migration Goal #1 (single foundation contract in core).

**But the two headline dispatch types are the opposite of consolidated:**

1. **`roko-core` `DispatchPlan`/`DispatchRequest` are exported-but-dead.** `dispatch_plan.rs`
   defines the full model→provider→transport resolution envelope (`DispatchPlan`,
   `DispatchRequest`, `DispatchCaller{Acp,CliChat,CliOneShot,Runner,Serve}`,
   `TransportPlan`, `DispatchAuthStatus`, `FallbackPolicy`). It is re-exported at
   `roko-core/src/lib.rs:206` — and **constructed by nobody in the workspace**. No resolver
   returns `DispatchPlan`; no caller builds `DispatchRequest` (grep: only the CLI's own
   `CliDispatchRequest`/`AgentDispatchRequest` exist). The module's own header admits it:
   *"intentionally data-only … resolution and execution remain in the agent/provider layers
   until the dispatch migration wires them together."* The migration never happened.

2. **Every real dispatch path uses CLI-local near-duplicates.** `roko-cli/src/dispatch/mod.rs`
   `RunnerDispatchPlan` (:201) and `roko-cli/src/dispatch_v2.rs` `ProviderDispatchResolver`
   (:642) + `CliDispatchRequest` (:470) + `AgentDispatchRequest` are the vocabulary the runner
   actually speaks. They describe the same concept as core's `DispatchPlan` in a parallel,
   never-reconciled type family.

3. **Name-collision inflates apparent wiring.** The `DispatchPlan` token appears in
   `orchestrate.rs:9200`, `runner/event_loop.rs:1756/3882`, and three orchestrator files — but
   every one of those is the **unrelated `ExecutorAction::DispatchPlan { plan_id }` DAG-plan
   variant** (`roko-orchestrator/src/executor/action.rs:21`), not the core resolution type.
   A grep for "DispatchPlan" over-reports the core type's reach by ~8 files.

## Type Status

| Concept | Canonical definition | Duplicates / adapters | Status |
|---|---|---|---|
| **Foundation module** | `roko-core/src/foundation.rs` (re-export `lib.rs:217`) | — | ✅ **NEW** since original audit — the shared contract crate goal is partly realized here. |
| `DispatchPlan` (resolution envelope) | `roko-core/src/dispatch_plan.rs:75` | — | 🔌 **Exported (`lib.rs:206`), constructed nowhere.** Data-only fossil awaiting a resolver. |
| `DispatchRequest` (+ `DispatchCaller`, `DispatchRequirement`, `TransportPlan`) | `dispatch_plan.rs:16,48,59,116` | — | 🔌 Same: zero constructors outside the module. |
| Runner dispatch plan | `roko-cli/src/dispatch/mod.rs:201` `RunnerDispatchPlan` | vs core `DispatchPlan` | ✅ live; CLI-specific; **the actual runner contract**. |
| Provider dispatch resolver | `roko-cli/src/dispatch_v2.rs:642` `ProviderDispatchResolver` (+ `CliDispatchRequest:470`, `AgentDispatchRequest`) | vs core `DispatchRequest` | ✅ live; provider-specific; **the actual resolver**. Not built on core types. |
| `ExecutorAction::DispatchPlan { plan_id }` | `roko-orchestrator/src/executor/action.rs:21` | name-collides with core `DispatchPlan` | ✅ live DAG-plan dispatch; **unrelated** to model dispatch — rename candidate. |
| `RunLedger` | `roko-runtime/src/run_ledger.rs:18` | — | ✅ used by `workflow_engine.rs`, `runner/event_loop.rs`. Runtime-only; not the run truth in orchestrate.rs (which uses executor state). |
| `CommitOutcome` | `roko-runtime/src/pipeline_state.rs:40` | legacy `PipelineInput::CommitDone/CommitFailed` adapters | ✅ typed contract w/ `from_pipeline_input`/`from_commit_done`/`from_commit_failed` legacy adapters. Runtime workflow only. |
| `GateStatus` (typed result) | `roko-gate/src/registry.rs:7` (`Passed/Failed/Skipped/NotWired/InvalidConfig`) | `roko-cli/src/inline/primitives/gate_block.rs:24` (TUI: `Pending/Running/Passed/Failed/Skipped`) | ✅✅ **two by design, different audiences**, now with an explicit `From<&GateVerdict>` bridge. Distinct enough that the inline one should be renamed `GateRungDisplay`. |
| `GateVerdict` (data record) | `roko-core/src/foundation.rs:368` (`passed`/`skipped`/`skip_reason`/`output`/`duration_ms`) | **6 more `GateVerdict` structs** — see drift below | 🕰️ canonical exists in foundation but **name reused across 7 crates** with different shapes. |
| `RoutingContext` | `roko-learn/src/model_router.rs:130` | — | ✅ **broadly consumed** (25 files): ACP, serve `providers`/`gateway`, learn cascade/experiment/feedback, CLI `dispatch`/`runner`/`main`/`config_cmd`, orchestrator `service_factory`. The best-consolidated of the set. |

## Current Problem

The names are present; some are now genuinely shared (`foundation.rs`, `RoutingContext`,
`CommitOutcome`, `GateStatus`). The unresolved problem is concentrated in **two clusters**:

- **Dispatch contract split** — core's `DispatchPlan`/`DispatchRequest` were the intended
  single contract, but the runner/provider layer grew its own (`RunnerDispatchPlan`,
  `ProviderDispatchResolver`, `CliDispatchRequest`, `AgentDispatchRequest`) and never adopted
  core's. Result: core carries a dead vocabulary, the CLI carries the live one, and a third
  unrelated `ExecutorAction::DispatchPlan` shares the name. Runner v2, Graph, WorkflowEngine,
  ACP, serve, inline UI, and legacy orchestrate still describe similar work in different words.

- **`GateVerdict` name overload** — one canonical `GateVerdict` in `foundation.rs:368`, but the
  same identifier is redefined in `roko-learn/src/episode_logger.rs:90`,
  `roko-core/src/dashboard_snapshot.rs:290`, and `roko-chain/src/identity_economy_identity.rs:1600`,
  plus `GateVerdictRecord` (`forensic.rs:124`) and `GateVerdictSummary`
  (`event_bus.rs:75`, `runner/types.rs:141`). Some are legitimately different projections
  (forensic record, event-bus summary); others are drift.

## Drift list (half-migrated / duplicate types)

1. **`DispatchPlan` core-vs-CLI split (P0)** — `roko-core::DispatchPlan` (data-only,
   0 constructors) vs `RunnerDispatchPlan` + `ProviderDispatchResolver` (live). The migration
   note in `dispatch_plan.rs:1-4` is a standing TODO nobody executed.
2. **`DispatchPlan` name triple-use (P1)** — core resolution type, CLI runner type, and
   `ExecutorAction::DispatchPlan` DAG variant all share the token. Grep-hostile.
3. **`GateVerdict` × 4 definitions + 2 `*Summary` + 1 `*Record` (P1)** — foundation:368 (canonical),
   learn episode_logger:90, dashboard_snapshot:290, chain:1600. No `From`/conversion between them.
4. **`GateStatus` × 2 (documented, low risk)** — registry typed result vs inline TUI display.
   Only missing piece is a name that signals the boundary (`InlineGateStatus`/`GateRungDisplay`).
5. **`RunLedger` is runtime-only** — the workflow-engine run truth; orchestrate.rs (the main
   plan loop) does not use it, so "run truth" is still split between RunLedger and executor state.

## Migration Goal

Create one of these outcomes:

1. **Single foundation contract**: `foundation.rs` is now the vehicle — finish the job. Move the
   dispatch contract into it (or fold `dispatch_plan.rs` in), and have the runner/provider layer
   *adopt* core's `DispatchPlan`/`DispatchRequest` instead of `RunnerDispatchPlan`/`CliDispatchRequest`.
2. **Documented layered contracts**: keep separate UI/runtime/provider types, but name them so
   boundaries are explicit and conversion is tested — the `From<&GateVerdict> for GateStatus`
   at `registry.rs:56` is the pattern to replicate for every other pair.

Do not keep ambiguous near-duplicates with the same names and different semantics
(`DispatchPlan` ×3, `GateVerdict` ×4).

## Checklist

- [x] **Foundation contract module exists** (`roko-core/src/foundation.rs`) — this was the audit's
      ask; verify: `rg -n '^pub (struct|enum)' crates/roko-core/src/foundation.rs`.
- [x] **First conversion test exists** — `From<&GateVerdict> for GateStatus` at
      `roko-gate/src/registry.rs:56` with `from_legacy_fields`.
- [ ] **[P0]** Wire core `DispatchPlan`/`DispatchRequest` into the live path, or delete them and
      promote `RunnerDispatchPlan`/`ProviderDispatchResolver` into `roko-core`. Verify:
      `rg -n 'DispatchPlan \{' crates/ --glob '!target/**'` returns ≥1 construction of the *core*
      type (not `ExecutorAction::DispatchPlan`).
- [ ] **[P1]** Rename `ExecutorAction::DispatchPlan { plan_id }` → `ExecutorAction::RunPlan` (or
      `DispatchDagPlan`) to kill the token collision. Verify: `rg 'DispatchPlan' crates/roko-orchestrator`
      returns only the type, not a variant.
- [ ] **[P1]** Consolidate `GateVerdict`: make foundation:368 canonical, add `From`/`Into` adapters
      for the learn/dashboard/chain projections, or rename them (`EpisodeGateVerdict`,
      `DashboardGateVerdict`, `ChainGateVerdict`). Verify: `rg 'struct GateVerdict' crates/ --glob '!target/**'`
      = 1 canonical + explicitly-named projections.
- [ ] **[P2]** Rename inline `GateStatus` → `GateRungDisplay` (`gate_block.rs:24`); add its
      `From<registry::GateStatus>` conversion + test.
- [ ] **[P2]** Add conversion tests between runner, workflow, ACP, serve, and inline UI status
      types (only the gate `From` exists today).
- [ ] **[P3]** Decide whether `RunLedger` becomes the single run truth (adopt it in orchestrate.rs)
      or stays a workflow-engine-only projection; document the boundary.
- [ ] Update tmp/doc references that still say these types are *absent* — they exist; the issue is
      duplication and one dead core contract, not missing types.
