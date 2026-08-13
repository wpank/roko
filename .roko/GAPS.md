# Roko Gaps & Outstanding Work

> **What is this?** This file tracks known gaps, incomplete features, and tech debt in the roko codebase.
> Check here BEFORE starting new work. Update or remove entries as work is completed.
> Last updated: 2026-08-13

## How to read this file

Each entry includes: what's missing, where it lives (crate + file), why it matters, and suggested fix.
Status: OPEN | PARTIAL | RESOLVED

---

## Critical gaps

### event_loop.rs is a ~20K-line god object — OPEN

**What:** The runner's main event loop file has grown to ~19,800 lines. It contains plan execution,
agent dispatch, gate evaluation, dream consolidation, learning subscribers, snapshot persistence,
cancellation logic, and ownership tracking -- all in one file.

**Where:** `crates/roko-cli/src/runner/event_loop.rs`

**Why it matters:** New contributors cannot navigate it. Merge conflicts are constant. Testing
individual subsystems requires compiling the entire CLI crate.

**Suggested fix:** Extract coherent subsystems into submodules under `runner/`:
- `runner/dispatch.rs` -- agent dispatch + model routing
- `runner/gates.rs` -- gate evaluation + threshold flush
- `runner/learning.rs` -- learning subscribers + feedback
- `runner/persistence.rs` -- snapshot save/load
- `runner/cancellation.rs` -- ownership + quarantine tracking

Keep `event_loop.rs` as a thin orchestrator that calls into these modules.

---

### Knowledge store not consulted for model routing — OPEN

**What:** The `CascadeRouter` (which selects which LLM model handles a task) does not query the
neuro knowledge store for historical model performance data. It routes based only on its own
internal stats (success rates, latency).

**Where:** `crates/roko-learn/src/cascade_router.rs` has no imports or references to
`knowledge_store`, `KnowledgeStore`, or `neuro`.

**Why it matters:** The knowledge store accumulates distilled insights about which models work
well for which task types. Without consulting it, the router cannot benefit from cross-session
learning.

**Suggested fix:** Add a `query_knowledge_for_model_hints()` step at the top of
`CascadeRouter::route()` that checks the neuro store for relevant model-performance entries,
then biases the routing weights accordingly.

---

### ProviderOutcomeRecorder not wired into runner event loop — OPEN

**What:** The `ProviderOutcomeRecorder` trait exists in `roko-agent` and is implemented by
`ProviderHealthRegistry` in `roko-learn`. However, the runner event loop does not pass a
recorder instance to `ModelCallService` when dispatching agents. Provider health is currently
updated only via the event bus (`AgentEvent::ProviderError`), not through direct call-site
recording.

**Where:**
- Trait: `crates/roko-agent/src/model_call_service.rs`
- Implementation: `crates/roko-learn/src/provider_health.rs`
- Import exists but unused: `crates/roko-cli/src/dispatch_v2.rs:18`
- Missing from: `crates/roko-cli/src/runner/event_loop.rs`

**Why it matters:** Without call-site recording, provider health scores lag behind reality.
The circuit breaker reacts to failures reported over the event bus, but the cascade router
cannot see per-call success/failure data for fine-grained model routing.

**Suggested fix:** In the runner's dispatch path, construct `ModelCallService` with
`.with_provider_outcome_recorder(health_registry)` so every LLM call records its outcome
directly.

---

## Built-but-unwired subsystems

### TelemetryObserve (periodic observability snapshots) — OPEN

**What:** `TelemetryObserve` trait and `PeriodicObserver` are defined and tested but never
instantiated outside their own test module. No runtime code calls `.observe()`.

**Where:** `crates/roko-core/src/obs/telemetry_observe.rs`

**Why it matters:** The observability lens registry collects metrics (gate verdicts, token usage,
target dir size), but without a periodic observer, these metrics are only available on-demand,
not streamed to logs or dashboards.

**Suggested fix:** Instantiate a `PeriodicObserver` in the runner event loop's main tick
(or in `roko serve`'s background tasks) and write observations to `.roko/metrics/telemetry.jsonl`.

---

### Corrigibility framework — OPEN

**What:** A five-head lexicographic corrigibility ordering (Deference > Switch > Truth > Impact > Task)
is fully implemented with `evaluate_action()`, `CorrigibilityScore`, and `CorrigibilityLevel`.
However, it is only used within its own module's tests. No agent dispatch or safety check calls
`evaluate_action()`.

**Where:** `crates/roko-core/src/corrigibility.rs` (only file that references these types)

**Why it matters:** This is the theoretical safety backbone -- it can veto agent actions that
violate higher-priority corrigibility heads. Without wiring, agent safety relies solely on
the `AgentContract` + `SafetyLayer` (bash/git/network/path policies).

**Suggested fix:** Call `evaluate_action()` as a pre-check in `SafetyLayer::check_tool_call()`
(in `roko-agent/src/safety/mod.rs`). If a higher-priority head vetoes, reject the tool call.

---

### TaintTracker (information flow tracking) — OPEN

**What:** `TaintTracker` records how tainted information (from untrusted sources) flows through
signal chains. It supports marking signals as tainted, propagating taint through DAG edges,
and checking whether a signal is tainted before acting on it. Fully implemented with ~30 tests.
Never instantiated outside tests.

**Where:** `crates/roko-orchestrator/src/safety/taint_propagation.rs`

**Why it matters:** Without taint tracking, an agent could act on data from an untrusted
external webhook as if it were a trusted internal signal. This is a security gap for
any deployment that accepts external inputs.

**Suggested fix:** Create a `TaintTracker` in the runner event loop, mark signals from
external sources (webhooks, MCP, user input) as tainted, and check taint status before
allowing high-privilege operations (file writes, deployments, chain transactions).

---

### SandboxPolicy (subprocess sandboxing) — OPEN

**What:** `SandboxPolicy` and `SandboxEnforcer` validate subprocess actions: allowed filesystem
paths, wall-clock timeouts, network access, env-var scrubbing. Fully implemented with ~20 tests.
Never used outside its own test module and `sandboxing.rs`.

**Where:** `crates/roko-orchestrator/src/safety/sandboxing.rs`

**Why it matters:** Agent-spawned subprocesses (cargo, npm, git) currently run with the full
permissions of the roko process. A `SandboxEnforcer` check before spawning would limit
blast radius.

**Suggested fix:** Build a `SandboxPolicy` from agent contract constraints, then call
`SandboxEnforcer::check_path()` and `check_wall_ms()` before subprocess spawn in
`roko-agent`'s tool loop.

---

### Immune system (knowledge quarantine) — PARTIAL

**What:** The immune system module defines anomaly scoring, quarantine decisions, and
`ImmuneResponse` recovery actions for compromised knowledge. The types are used in
`event_loop.rs` (quarantine tracking for task attempts) and in `roko-dreams` (phase 2),
but the full screening pipeline (score incoming signals, quarantine anomalies, review
and recover) is not wired as an automatic pre-ingestion step.

**Where:** `crates/roko-core/src/immune.rs`

**Why it matters:** Without automatic screening, poisoned or anomalous signals enter the
knowledge store unchecked. The quarantine types are used for task-level failure tracking,
but not for their intended purpose of knowledge-level integrity.

**Suggested fix:** Add an immune screening step in the knowledge store's `ingest()` path
that scores incoming signals and quarantines those above the anomaly threshold.

---

### Graph Engine (plan-to-graph execution) — PARTIAL

**What:** The Graph Engine provides an alternative execution model where plans are converted
to directed graphs with typed cells. Several pieces remain incomplete:

1. **TaskExecutorCell live dispatch:** The `dry_run: false` path falls back to dry-run with a
   warning. Real dispatch should delegate to the runner's agent dispatch path.
   (`crates/roko-graph/src/cells/task_executor.rs`)

2. **Cognitive loop cells:** All 7 cells (`signal-reader`, `relevance-scorer`,
   `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`,
   `event-publisher`) use `PassthroughCell` stubs. (`crates/roko-graph/src/cells/stubs.rs`)

3. **Hot graph state persistence:** `HotPolicy.persist_tick_state` is defined but not
   implemented. (`crates/roko-graph/src/hot.rs`)

4. **Parallel node execution:** The engine executes nodes sequentially; `max_parallel`
   metadata is stored but unused. (`crates/roko-graph/src/engine.rs`)

5. **Graph Engine snapshot/resume:** `--resume-plan` is not supported on the graph path.
   (`crates/roko-cli/src/commands/plan.rs`)

**Why it matters:** The Graph Engine is the intended long-term replacement for the monolithic
event loop. Until these gaps are closed, it remains a dry-run/demo tool only.

**Suggested fix:** Prioritize TaskExecutorCell live dispatch first (enables real plan execution
through graphs), then cognitive loop cells, then hot-graph persistence.

---

### VCG auction (context composition) — PARTIAL

**What:** `vcg_allocate` implements a Vickrey-Clarke-Groves auction for allocating context
budget across competing bidders. It is built and exported, but `CompositionStrategy::Auto`
always resolves to `DensityGreedy` because the bidder observation registry starts empty.
VCG only activates after all bidders reach 10 observations.

**Where:** `crates/roko-compose/src/auction.rs`, `crates/roko-compose/src/strategy.rs`

**Why it matters:** VCG would produce more efficient context allocation than greedy packing,
but the cold-start problem means it never activates in practice.

**Suggested fix:** Either lower the observation threshold, seed initial observations from
historical data, or add a warm-up period that mixes VCG with greedy.

---

### Cold substrate archival (no automatic trigger) — OPEN

**What:** `ArchiveColdSubstrate` can archive old signals from the hot store to compressed
cold storage. The `roko knowledge archive` CLI command and a `roko-serve` handler exist
and work. However, there is no automatic trigger: no cron job, no age-based policy, and
no integration with the scheduler. Archival only happens when a human runs the command.

**Where:**
- Implementation: `crates/roko-fs/src/cold_substrate.rs`
- CLI command: `crates/roko-cli/src/commands/knowledge.rs` (line ~274)
- Server handler: `crates/roko-serve/src/lib.rs` (line ~2200)
- Scheduler (no archive job): `crates/roko-serve/src/scheduler.rs`

**Why it matters:** On long-running instances, the hot store grows without bound. The GC
system handles JSONL compaction but not tier migration to cold storage.

**Suggested fix:** Add a `ColdArchivalJob` to the cron scheduler that runs
`SubstrateMigrator::migrate()` on a configurable interval (e.g., daily).

---

## Technical debt

### AgentContract falls back to hardened default when no YAML exists — PARTIAL

**What:** When no contract YAML file exists for a role, `SafetyLayer::with_defaults()` creates
a `hardened_default` contract instead of a permissive one. This is safer than the old behavior
(which was fully permissive), but it means custom per-role constraints from YAML are only
active when the YAML files are actually present in the workspace.

**Where:** `crates/roko-agent/src/safety/mod.rs` (line ~261), `crates/roko-agent/src/safety/contract.rs`

**Why it matters:** In a fresh workspace without bundled YAML contracts, all agents get the
same hardened-default policy. Role-specific restrictions (e.g., "researcher cannot write files")
only work if someone creates the YAML. This is documented behavior, not a bug, but it
means the contract system is underutilized.

**Suggested fix:** Bundle default contract YAML files for the core roles (implementer,
researcher, reviewer, planner) in `crates/roko-core/src/builtin_roles/`.

---

### RBAC applied to serve routes but not to CLI commands — PARTIAL

**What:** RBAC (Role-Based Access Control) is fully implemented in `roko-serve`: the
`rbac` module defines roles (Owner > Admin > Member > Viewer) and permissions, middleware
maps auth scopes to roles, and route groups enforce permissions (`PlanExecute`, `ConfigEdit`,
`SecretsWrite`, `AgentSpawn`, `TeamManage`). However, CLI commands bypass RBAC entirely --
anyone who can run the binary has full access.

**Where:**
- RBAC module: `crates/roko-serve/src/rbac.rs`
- Middleware: `crates/roko-serve/src/routes/middleware.rs` (line ~1005)
- Route enforcement: `crates/roko-serve/src/routes/mod.rs` (line ~245)
- CLI commands: `crates/roko-cli/src/commands/` (no RBAC checks)

**Why it matters:** For single-user local development, this is fine. For shared or deployed
instances, CLI access is an RBAC bypass.

**Suggested fix:** This is acceptable for now. If multi-user CLI access is needed, add an
optional auth check in `main.rs` that resolves a local token to a role before dispatching
commands.

---

### `loop_tick.rs` defines universal loop but runner reimplements inline — OPEN

**What:** `roko-core/src/loop_tick.rs` defines the canonical universal loop
(query > score > route > compose > act > verify > write > react), but `event_loop.rs`
reimplements this logic inline rather than calling `LoopTick`.

**Where:**
- Canonical definition: `crates/roko-core/src/loop_tick.rs`
- Inline reimplementation: `crates/roko-cli/src/runner/event_loop.rs`
- Only test consumer: `crates/roko-std/tests/universal_loop.rs`

**Why it matters:** The universal loop abstraction is a core architectural principle, but the
runtime does not use it. This makes the trait definition misleading -- it suggests a clean
abstraction that does not match reality.

**Suggested fix:** Either refactor `event_loop.rs` to use `LoopTick` as its execution
skeleton, or remove `loop_tick.rs` and document that the runner is the canonical loop.

---

### roko-acp compile issues — OPEN

**What:** The `roko-acp` crate has pre-existing compile errors related to missing fields
(`mcp_config` on `PipelineConfig`) and import path mismatches. These block full workspace
`cargo check` when `roko-acp` is included.

**Where:** `crates/roko-acp/src/bridge_events.rs`, `crates/roko-acp/src/runner.rs`

**Why it matters:** Any workspace-wide `cargo check` or `cargo test` that includes `roko-acp`
will fail. The crate is likely excluded from the default workspace members, but its presence
can confuse new contributors.

**Suggested fix:** Either fix the missing field/import issues or explicitly exclude `roko-acp`
from the workspace `[members]` list in the root `Cargo.toml`.

---

### Gate threshold flush interval is hardcoded — OPEN

**What:** `GATE_THRESHOLD_FLUSH_INTERVAL = 10` is a compile-time constant. The incremental
flush (which writes `gate-thresholds.json` every N gate observations) cannot be tuned without
recompilation.

**Where:** `crates/roko-cli/src/runner/event_loop.rs`

**Why it matters:** Low-volume runs waste I/O flushing too often; high-volume runs may want
more frequent persistence. Not a high priority, but a minor config gap.

**Suggested fix:** Read the interval from `roko.toml` under `[learning.gate_threshold_flush_interval]`.

---

### Dream consolidation has no automatic runtime trigger — PARTIAL

**What:** Dream consolidation runs at the end of plan execution
(`run_dream_consolidation_if_enabled`) and can be triggered manually via
`roko knowledge dream run` or `POST /api/dream/run`. A `DreamSchedulePolicy` with cron
support exists in `roko-dreams`, and the serve scheduler infrastructure exists. However,
the scheduler does not include dream consolidation as a scheduled job.

**Where:**
- Post-plan trigger: `crates/roko-cli/src/runner/event_loop.rs` (line ~4438, ~11854)
- Manual CLI: `crates/roko-cli/src/commands/knowledge.rs`
- HTTP trigger: `crates/roko-serve/src/routes/dream.rs`
- Schedule policy (built): `crates/roko-dreams/src/runner.rs` (`DreamSchedulePolicy`)
- Scheduler (no dream job): `crates/roko-serve/src/scheduler.rs`

**Why it matters:** On long-running daemon instances, dream consolidation only happens after
plan runs finish. Idle periods (where consolidation would be most useful) are not leveraged.

**Suggested fix:** Add a `DreamConsolidationJob` to the cron scheduler that respects
`DreamSchedulePolicy` settings from config.

---

### Workspace lock exists but coverage is incomplete — PARTIAL

**What:** `acquire_workspace_lock()` uses `flock` to prevent concurrent plan execution.
It is called from `prd plan`, `plan run`, and `daemon start`. However, not all mutating
CLI commands acquire the lock (e.g., `roko run`, `roko agent start`).

**Where:** `crates/roko-cli/src/workspace_lock.rs`

**Why it matters:** Two concurrent `roko run` invocations could corrupt shared state files
(`.roko/state/executor.json`, `.roko/signals.jsonl`).

**Suggested fix:** Add lock acquisition to `roko run` and `roko agent start` entry points.

---

### roko-orchestrator test failures (pre-existing) — OPEN

**What:** 7 of 544 tests in `roko-orchestrator` fail. These appear to be worktree/git-related
tests that depend on specific repository state.

**Where:** `crates/roko-orchestrator/` (test suite)

**Why it matters:** Noisy test failures make it hard to tell whether new changes break
anything. Contributors may ignore real failures because "some tests always fail."

**Suggested fix:** Either fix the git-state-dependent tests or mark them `#[ignore]` with
a clear comment explaining the dependency.

---

## Deferred (Phase 2+)

### roko-chain: 16 shelved modules

All 16 modules below contain tested Rust logic but have zero runtime callers. They are
blocked on the daeji devnet reaching a deployable state with real contracts. The ISFR
vertical (isfr_keeper, isfr_sources, isfr_oracle_submit, isfr_bootstrap) is the only
chain surface wired into the runtime today.

**daeji fence:** daeji is a separate devnet repo that owns node/BFT/precompiles/consensus.
Design-only docs live at `tmp/agentchain-v2/02-daeji/`. roko-chain is the client/runtime
integration side only. Do NOT build node-side or consensus features in this repo.

| Module | Blocking dependency |
|---|---|
| `witness.rs` | daeji witness registry contract + `get_logs` |
| `x402.rs` | ERC-3009 + state channels need live token contract |
| `korai_token.rs` | KORAI.sol not deployed (Deploy.s.sol uses MockERC20) |
| `marketplace.rs` | Runtime uses `.roko/jobs/*.json`, not on-chain marketplace |
| `agent_registry.rs` | Serve uses `sol!` ABI bindings, not this Rust twin |
| `reputation_registry.rs` | Same as agent_registry |
| `validation_registry.rs` | Same as agent_registry |
| `isfr.rs` (IsfrRegistry) | Keeper submits to ISFROracle, not this clearing engine |
| `trace_rank.rs` | No runtime consumer for PageRank reputation |
| `collusion.rs` | No multi-agent marketplace needs collusion detection yet |
| `nelson_siegel.rs` | No ISFR rate consumers need term-structure interpolation |
| `futures_market.rs` | No DeFi derivatives trading needed yet |
| `gate/mev_gate.rs` | Not in the 7-rung gate pipeline |
| `gate/tx_sim_gate.rs` | Not in the 7-rung gate pipeline |
| `gate/wallet_gate.rs` | Not in the 7-rung gate pipeline |
| `heartbeat_ext.rs` | No chain-aware agent lifecycle management needed yet |

**Where:** `crates/roko-chain/src/`

**Decision:** All SHELVE (Phase 2+). Wire after daeji mainnet launch.

---

### Phase-2 stubs in daimon/dreams

**What:** `phase2_stubs.rs` in roko-daimon has 4 `#[allow(dead_code)]` items and `replay.rs`
has 1. These are intentional placeholders with module-level comments.

**Where:** `crates/roko-daimon/`, `crates/roko-dreams/`

**Decision:** Intentional. No action needed until Phase 2 affect-engine work begins.

---

### roko-chain Engram duplicate (RESOLVED)

**What:** `identity_economy_markets.rs:653` previously defined a local `Engram` struct that
duplicated `roko_core::Engram`. This has been removed as of 2026-08-13.

**Where:** `crates/roko-chain/src/identity_economy_markets.rs`

**Decision:** Resolved. The duplicate `struct Engram` no longer exists in roko-chain.

---

## Recently resolved

### AgentContract permissive fallback (was: falls back to fully permissive)

**Resolved:** `SafetyLayer::with_defaults()` now uses `AgentContract::hardened_default()`
instead of `AgentContract::permissive()`. The fallback is still not role-specific (see
"AgentContract falls back to hardened default" in Technical debt above), but it is no
longer wide-open.

### Workspace locking (was: no file locking)

**Resolved:** `workspace_lock.rs` implements `flock`-based locking. Used by `prd plan`,
`plan run`, and `daemon start`. Configurable timeout via `roko.toml`
(`timeouts.workspace_lock_secs`). Remaining coverage gaps tracked under Technical debt above.

### RBAC (was: not implemented)

**Resolved:** Full RBAC module in `roko-serve` with 5 roles, 6+ permissions, middleware
enforcement on route groups, and 12+ tests. See Technical debt for CLI bypass note.

### roko-cli compile errors (was: 24 errors from missing types)

**Resolved:** Fixed in commit 88b3a31 (SH01-T07). Missing `TaskRunCategory` /
`TaskPhaseDurations` types were added.

### E07-T10: Incremental gate-threshold flush

**Resolved:** `maybe_flush_gate_thresholds()` flushes `gate-thresholds.json` every 10
gate observations. Counter resets on flush. Test proves round-trip.

### E09-T08: FsObservabilitySinks in runner-v2

**Resolved:** `flush_all()` on `JsonlTraceSink`, `flush_traces()` and
`flush_registry_snapshot()` on `FsObservabilitySinks`. Event loop shutdown flushes obs
sinks. `RunConfig::from_roko_config()` creates and registers a `MetricRegistry` with
13 standard metrics.

### Task 102: Engine as Default

**Resolved:** Runner v2 is the default engine since E01-T01. `--engine graph` selects
the Graph dry-run stub.
