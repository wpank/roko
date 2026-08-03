# Roko Gaps Tracker

Canonical list of unfinished items. Check before starting new work.

Last updated: 2026-07-24 (Wave 7 cleanup pass).

## Tasks 101-103 (Wave 5: Migration + Hot Graphs)

### Task 101: Plan-to-Graph Converter
- **TaskExecutorCell live dispatch**: The `dry_run: false` path in `TaskExecutorCell.execute()` falls back to dry-run behavior with a warning. The real implementation should delegate to the Runner v2 agent dispatch path (or the new Engine dispatch path when it replaces Runner v2). Subsystem: `roko-graph/src/cells/task_executor.rs`.
- **Graph Engine snapshot/resume**: The `--resume-plan` flag is not yet supported on the Graph Engine path. Implementing this requires state serialization between graph executions. Subsystem: `roko-cli/src/commands/plan.rs`.

### Task 102: Engine as Default (RESOLVED)
- Runner v2 is the default engine since E01-T01. The `runner/` module (`event_loop.rs`, `gate_dispatch.rs`, `types.rs`) is the live execution path. `--engine graph` selects the explicit Graph dry-run stub.
- **Graph Engine parallel execution**: The Graph engine executes nodes sequentially in topological order. The `max_parallel` metadata from plans is stored but not used for parallel node dispatch. Subsystem: `roko-graph/src/engine.rs`.

### Task 103: Hot Graphs + Cognitive Loop
- **Real cell implementations**: All 7 cognitive loop cells (`signal-reader`, `relevance-scorer`, `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`, `event-publisher`) use `PassthroughCell` stubs. Each needs a real implementation. Subsystem: `roko-graph/src/cells/`.
- **Hot Graph state persistence**: `HotPolicy.persist_tick_state` is defined but not implemented. The tick loop does not save/restore cell outputs between ticks. Subsystem: `roko-graph/src/hot.rs`.
- **TOML `[graph.policy.hot]` parsing**: The loader does not parse `[graph.policy.hot]` sections from TOML files. HotPolicy must be constructed programmatically. Subsystem: `roko-graph/src/loader.rs`.
- **Conditional edge evaluation**: Edges in cognitive-loop.toml note conditions (e.g., "only proceed if relevance above threshold") but the Engine treats all edges as unconditional. Subsystem: `roko-graph/src/engine.rs`.

## Wave 7+ cleanup findings (2026-07-24)

### Dead code and orphans (addressed)
- **Deleted**: `roko-core/src/state_hub.rs` and `pulse_bus.rs` — orphan files not declared in `lib.rs`, duplicates of wired copies in `roko-runtime`.
- **Fixed**: Broken doc-links in `bus_backends.rs`, `traits.rs`, `dashboard_snapshot.rs` referencing deleted modules.
- **Fixed**: Clippy `useless_conversion` in `roko-orchestrator/src/worktree.rs:3640`.

### Documentation gaps
- **`loop_tick` not wired**: `roko-core/src/loop_tick.rs` defines the universal loop but `runner/event_loop.rs` reimplements inline. Tracked under E01/E22.
- **VCG auction cold-start**: `CompositionStrategy::Auto` always resolves to `DensityGreedy` because the bidder observation registry starts empty. VCG activates only after all bidders reach 10 observations. Not a bug — by design.
- **`legacy-runner-v2` feature**: Cargo.toml comment was misleading (claimed it controlled binary behavior). Fixed: it only gates integration tests.

### Pre-existing issues (not yet addressed)
- **roko-cli compile errors**: Resolved. 24 errors from missing TaskRunCategory/TaskPhaseDurations types fixed in SH01-T07 correction (88b3a31).
- **roko-orchestrator test failures**: 7 of 544 tests fail (pre-existing). Likely worktree/git-related tests that depend on repository state.
- **Phase-2 stubs in daimon/dreams**: `phase2_stubs.rs` has 4 `#[allow(dead_code)]` items and `replay.rs` has 1. These are intentional (not yet wired) — documented with module-level comments.
- **roko-chain Engram duplicate**: `identity_economy_markets.rs:653` defines a local `Engram` struct that duplicates `roko_core::Engram`. The gate modules use the canonical core version. The local stub is part of the phase-2 economy types — cleanup tracked under E03-T07.

## roko-chain zero-caller modules: WIRE vs SHELVE decisions (E11-T05)

Last updated: 2026-08-03.

The ISFR vertical (isfr_keeper, isfr_sources, isfr_oracle_submit, isfr_bootstrap) is the only
chain surface wired into the runtime today. The 16 modules below contain real, tested Rust logic
but have zero runtime callers. Each receives an explicit verdict.

**daeji fence**: daeji is a separate devnet repo that owns node/BFT/precompiles/consensus.
Design-only docs live at `tmp/agentchain-v2/02-daeji/`. roko-chain is the client/runtime
integration side only. Do NOT build node-side or consensus features in this repo.

| Module | Verdict | Reason / Blocking dependency |
|---|---|---|
| `witness.rs` | **SHELVE** (Phase 2+) | Requires daeji witness registry contract deployed + `get_logs` for verification. No runtime consumer today. Wire after daeji mainnet launch. |
| `x402.rs` | **SHELVE** (Phase 2+) | ERC-3009 + state channels need a live token contract (KORAI/DAEJI). No runtime consumer. Wire after token launch. |
| `korai_token.rs` | **SHELVE** (Phase 2+) | Demurrage token client — no KORAI.sol deployed (Deploy.s.sol uses MockERC20). Wire after token contract is authored and deployed. |
| `marketplace.rs` | **SHELVE** (Phase 2+) | Spore FSM with Vickrey/Sparrow/Direct auctions. Runtime jobs use `.roko/jobs/*.json` (file-based), not on-chain marketplace. Wire when on-chain job market is needed. |
| `agent_registry.rs` | **SHELVE** (Phase 2+) | ERC-8004 Rust twin for AgentRegistry.sol. Serve routes use `sol!` ABI bindings directly, not this module. Wire when Rust-native registry queries replace sol! calls. |
| `reputation_registry.rs` | **SHELVE** (Phase 2+) | ERC-8004 Rust twin for ReputationRegistry.sol. Same as agent_registry — serve uses sol! bindings. Wire after ERC-8004 trio is deployed and indexed. |
| `validation_registry.rs` | **SHELVE** (Phase 2+) | ERC-8004 Rust twin for ValidationRegistry.sol. Same pattern. Wire after ERC-8004 trio deployment. |
| `isfr.rs` (IsfrRegistry) | **SHELVE** (Phase 2+) | 6-phase commit-reveal clearing engine. Keeper does NOT run this — keeper submits rates to ISFROracle, not to this clearing protocol. Wire when multi-party clearing goes live. |
| `trace_rank.rs` | **SHELVE** (Phase 2+) | PageRank-style reputation propagation over payment edges. Tested primitive, no runtime consumer. Wire when reputation-informed routing (item 13 in CLAUDE.md) is implemented. |
| `collusion.rs` | **SHELVE** (Phase 2+) | Clique-based collusion ring detection on assignment graphs. Tested primitive, no consumer. Wire when multi-agent marketplace requires collusion detection. |
| `nelson_siegel.rs` | **SHELVE** (Phase 2+) | Yield curve model for DeFi oracle rate term structure. Tested primitive. Wire when ISFR rate consumers need term-structure interpolation. |
| `futures_market.rs` | **SHELVE** (Phase 2+) | Interest rate futures market. Tested primitive. Wire when DeFi derivatives trading is needed. |
| `gate/mev_gate.rs` | **SHELVE** (Phase 2+) | MEV detection gate (sandwich bundles, frontrunning). Not in the 7-rung gate pipeline. Wire when chain transactions need MEV protection. |
| `gate/tx_sim_gate.rs` | **SHELVE** (Phase 2+) | Transaction simulation gate. Not in the 7-rung pipeline. Wire when pre-flight tx simulation is needed for chain operations. |
| `gate/wallet_gate.rs` | **SHELVE** (Phase 2+) | Wallet health/balance gate. Not in the 7-rung pipeline. Wire when automated chain spending needs balance guards. |
| `heartbeat_ext.rs` | **SHELVE** (Phase 2+) | Policy-cage extension for chain heartbeat monitoring. No runtime consumer. Wire when chain-aware agent lifecycle management is needed. |

**Summary**: All 16 modules are **SHELVE** (Phase 2+). None are on the critical path for
self-hosting. The blocking dependency for most is the daeji devnet reaching a deployable state
with real contracts. The ISFR vertical is the only chain feature needed for current runtime
operation, and it is already wired.

## E14-T10: Provider health degradation routing (2026-08-03)

### What was implemented
- `ProviderOutcomeRecorder` trait added to `roko-agent::model_call_service` (dependency-safe bridge).
- `ProviderHealthRegistry` (in `roko-learn::provider_health`) implements the trait: `record_provider_success` → `record_success`, `record_provider_failure` (with label→ErrorClass mapping) → `record_failure`.
- `ModelCallService` gains `with_provider_outcome_recorder` builder and records success/failure on every live provider call (rate-limit, timeout, generic, convergence, success paths).
- 5 new tests: 3 in `roko-learn::provider_health` (`provider_outcome_recorder_*`), 2 in `roko-agent::model_call_service` (`provider_outcome_recorder_*`).

### What remains (E48 supersedes this task)
- `CascadeRouter::set_provider_health` method (static provider health injection into routing) — not yet added to cascade_router.rs. This is the E48-T05 responsibility.
- `event_loop.rs` (runner-v2) does not yet pass a `ProviderOutcomeRecorder` to the `ModelCallService` or `ProviderCallCell` it creates. The circuit breaker is currently updated only via the event bus (AgentEvent::ProviderError) in `run_learning_subscriber`. Wiring through `ModelCallService` requires E48-T03 to construct the service with the recorder.
- Legacy `orchestrate.rs` `run_task_plans_inner` dispatch also does not pass a recorder to `ModelCallService`. Tracked under E48-T03.
- `force_backend` routing does not bypass circuit-breaker filtering — this is intentional per acceptance (explicit intent must remain selectable).
