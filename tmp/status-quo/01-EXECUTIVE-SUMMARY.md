# Executive Summary

**Date**: 2026-07-08  
**Branch**: `main` at `5852c93c05`  
**Verdict**: Roko has many real subsystems, but the project is in a half-migrated state. The single dominant issue is **engine drift** (see [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md)): default `roko plan run` routes to a Graph Engine whose task executor is a dry-run stub that prints `SUCCESS` while spawning no agent, spending $0, and changing no code. A second, previously under-weighted theme is **security**: an unauthenticated relay proxy and a read-scope auth fallback are exploitable today ([75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md)).

## Current Truth

Roko is best understood as three overlapping systems:

1. **Runner v2** (`crates/roko-cli/src/runner/`, 19 files ~17K LOC) is the real live plan executor. It dispatches agents, gates, feedback sinks, snapshots, resume, merge queue, and StateHub/TUI integration. It is reached by `--engine runner-v2` and **implicitly** by `roko do`/`serve`/`prd`/`worker`. Its cross-cut hooks (daimon, dreams, learning, efficiency, neuro, gate dispatch) fire in `event_loop.rs`, so it is ~80% ported. Holdouts: the conductor supervision loop (`conductor_load` hardcoded `0.0`, `event_loop.rs:4258`), agent-driven gate-failure replan (prompt enrichment only, no `tasks.toml` rewrite), and worktree isolation (built in `roko-orchestrator`, unwired in the runner). See [92-RUNNER-V2-MODULE-FAMILY.md](92-RUNNER-V2-MODULE-FAMILY.md).
2. **Graph Engine** (`crates/roko-graph/`) is the v2 target shape and is wired as the default `roko plan run` engine via Clap `default_value="graph"` (`main.rs:1362`, overriding the enum's own `#[default] RunnerV2` at `main.rs:1301`). Converted plan tasks run through `TaskExecutorCell` whose `dry_run` defaults to `true` (`task_executor.rs:32`) **and** whose live branch is "not yet implemented" (`:81-89`) — it emits a synthetic `task-output:stub:` engram instead of dispatching agents. A real `AgentCell` exists in the crate but is never registered.
3. **Legacy orchestrate.rs** (~23.6K LOC) is `#[cfg(feature="legacy-orchestrate")]` (`lib.rs:90-95`), **dead-by-default**. It still contains cross-cutting ideas and old wiring, but it is not the strategic runtime path and is not in the default binary.

That means the old docs phrase "Roko can self-host with `roko plan run plans/`" is not currently safe. Self-hosting must use `roko plan run --engine runner-v2` until the default graph path is fixed. `roko resume` also hardcodes Graph (`main.rs:2699`) and discards the snapshot it finds.

## Inventory

| Metric | Current count | Notes |
|---|---:|---|
| Cargo workspace members | 35 | 31 crates under `crates/`, 3 apps under `apps/`, plus the `tests/` package. |
| Canonical noun | `Engram` | There is **no `struct Signal`** — `Signal` is a compat re-export only; a second dead `Engram` lives in `roko-chain`. |
| Builtin tool count | 37 | `TOOL_COUNT=37` (old docs say 19). |
| `roko-serve` routes | ~270 | `.route(` declarations under `crates/roko-serve/src`; includes aliases and nested routers (old docs say ~85). |
| LLM providers | 10 | Claude CLI/API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity, etc. |
| TUI tabs | 10 | Old docs say F1–F7 / 7 tabs. |
| Rust files | 1,285 | `rg --files -g '*.rs' -g '!target/**'`. |
| Rust LOC | 728,694 | Includes checked-in large sources/tests; use trend, not as quality signal. |
| Test attributes | 9,968 | `#[test]` and `#[tokio::test]` hits across `crates`, `apps`, `tests`; not a pass count. |
| Whole-workspace raw route declarations | 337 | Includes serve, agent server, relay, Mirage, worker/auth routers, and tests. |
| v1/v2/v2-depth markdown docs | 636 | Many are stale relative to current code. |
| tmp md/toml/sh files | 6,707 | Includes scratch, source designs, migrations, and previous audits. |

Duplicate-definition drift: `GateVerdict` is defined 4×, `RetentionPolicy` 3×, `Engram` twice (second dead copy in `roko-chain`). HDC is compiled out (`hdc_vector: null` per episode). The VCG prompt auction is unreachable at runtime. Chain **canonical consensus lives in a separate `daeji` repo**, not here.

## What Is Real

- **Runner v2**: live event loop, agent dispatch, gate dispatch, snapshots/resume, merge queue, learning feedback facade, output sinks. **Caveat (deep pass, [96](96-TRACE-RUNNER-V2-EXECUTION.md))**: there is **no live task DAG** — scheduling is a flat `task_index` HashMap + per-plan FSM, concurrency is **per-plan** (`max_concurrent_plans=4`, one agent per plan), not intra-plan task parallelism. `runner/task_dag.rs`/`UnifiedTaskDag` are dead/legacy-only.
- **roko-graph**: real graph loader, registry, topo execution, plan-to-graph converter, hot graph loop, CLI surface; not live for agent work.
- **roko-serve**: real HTTP API assembly, StateHub, SSE/WS, auth middleware, job/deploy/bench/chain/ISFR routes, file-backed persistence in several areas.
- **Learning**: `roko-learn` is durable and used; it records episodes, provider/model outcomes, section effects, router state, experiments, costs, rewards, and playbooks.
- **Safety**: dispatcher **pre-checks** are real and fail-closed for bundled contracts (`AgentContract` fails **CLOSED** to `RestrictedFallback`, not permissive). But two gaps: (1) **the entire per-tool safety funnel is bypassed on the default provider** — the `ToolDispatcher`→`SafetyLayer` 9-policy pre-check runs ONLY on the OpenAI-compat roko `ToolLoop`; **Claude CLI (default) and Codex drive their own subprocess tool loop and never touch roko's `SafetyLayer` per tool call** ([99](99-TRACE-AGENT-TURN.md) §7); (2) orchestrator **post-checks are `Warn`-only** — SecretLeak/PathEscape/ContractViolation log and do not block (`safety/mod.rs:749`). Any claim that Roko "safety-gates every tool call" or "blocks secret leaks" is stale.
- **Status upgrades landed since CLAUDE.md's roadmap**: item 13 knowledge-informed routing **is** wired (`orchestrate.rs:15509` + `cascade_router.rs:623`); item 14 cold-substrate archival **is** runtime-wired (`roko-serve/lib.rs:344`) — but it is **copy-not-move** (`cold_substrate.rs:218`), an unbounded hourly re-append (new P1 bug). Workspace-role auth also landed (durable `team.rs` store).
- **Gates**: compile/clippy/test run for real, but the **adaptive-gate story is NOT live** ([101](101-TRACE-GATE-PIPELINE.md)). The live runner gate path uses `RungExecutionInputs::default()` and never calls `enrich_rung_config`; adaptive thresholds (SPC/CUSUM/EWMA), oracles 4-6, ratchet, and `VerdictPublisher` exist only on the dead `orchestrate.rs` `PlanRunner`. Live rungs 3-6 stub-pass `Verdict::pass`, EMA only updates rung 2, and `GateThresholds::save` is never called. Prior "gates live but uneven" nuance is downgraded: the adaptive apparatus is dark on `roko plan run`.
- **Knowledge/dreams/daimon**: knowledge store, dream cycle, and Daimon state are substantive. **daimon and dreams ARE live in Runner v2** — daimon modulates dispatch per task and writes `affect.json`, and `DreamRunner` runs plan-completion consolidation — on `--engine runner-v2`/`serve`/`do`, **not** the default Graph engine ([96](96-TRACE-RUNNER-V2-EXECUTION.md) §13). Holdouts: cron/periodic dream trigger, routing-advice consumption. Not all feedback loops are closed (e.g. demurrage taxes confidence but income/reinforce is dead — balances stuck 0.0).
- **Surfaces**: CLI, TUI dashboard, React demo app (embedded-served by `roko serve` via `rust-embed`, not standalone — [105](105-FRONTEND-DEMO-APP.md)), ACP, agent sidecar server, relay, and static demo artifacts all exist in different degrees of maturity.
- **Chain/ISFR/jobs/deploy**: substantial code exists, but live chain use is optional/config-gated and several paths use mocks or local JSON state.

## P0 Problems

| P0 | Why it matters |
|---|---|
| Default `roko plan run` is stub-success ([95](95-ENGINE-DRIFT.md)) | The default Graph engine reports `SUCCESS` while doing no real agent work ($0, 0 agents, no code changed). |
| `roko resume` routes to Graph and Graph ignores snapshots | Resume hardcodes Graph (`main.rs:2699`) and discards the snapshot it just found. |
| Relay proxy is unauthenticated ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | `/relay/{*path}` + 2 WS are merged **outside** `/api` (`routes/mod.rs:248`) → full GET/POST/DELETE + WS unauthed on any non-loopback deploy. |
| Read-scope auth fallback ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | Unlisted mutating `/api/*` routes fall through to `"read"` (`middleware.rs:385`) → a read key can POST run/jobs/dream/deploy. |
| Per-tool safety funnel bypassed on the default provider ([99](99-TRACE-AGENT-TURN.md)) | The `ToolDispatcher`→`SafetyLayer` 9-policy pre-check runs only on the OpenAI-compat `ToolLoop`; the **default Claude-CLI provider drives its own subprocess tool loop** so roko never per-tool safety-gates the default self-host path (role/bash/net/path/budget/temporal/contract skipped). |
| `research search` / `/search` is 100% broken | Perplexity batch body → HTTP 422; mock tests are false-green (see [40](40-LEARNING-TELEMETRY.md)/[91](91-PRD-RESEARCH.md)). |
| Execution source of truth is unclear | Runner v2, Graph, WorkflowEngine, and legacy orchestrate docs all claim pieces of the runtime. |
| Foundation contracts are fragmented | DispatchPlan, RunLedger, GateStatus, CommitOutcome, RoutingContext exist in pieces, not one canonical layer. |
| Episode/event/state stores are split | Gate verdicts write `signals.jsonl` but dashboards read `engrams.jsonl` (empty panels); `events.jsonl` is 44 MB / 97% `feed_tick` firehose; the real snapshot is `state-snapshot.json` (serve still tries to READ `state/executor.json` → error); episodes triplicated (root/learn/memory). |
| API/frontend contracts are not enforced | 4 frontend→serve 404s (share vs shared, bench/matrix, isfr/stream, ws/agents) plus camelCase/snake_case event drift. |
| Docs overclaim or underclaim critical areas | CLAUDE/docs/v1/v2/tmp contain stale statements about engines, safety, route surfaces (~270 not ~85), chain, ACP, gates, counts (TOOL_COUNT=37 not 19), and the noun (Engram not Signal). |

## Migration Shape

The right sequence is not "build more features." It is:

1. Make the live execution contract honest.
2. Pick one strategic plan engine for users, and make the other opt-in until parity is proven.
3. Consolidate foundation result types and state paths.
4. Move cross-cut features onto the strategic runtime path.
5. Delete or quarantine stale compatibility layers.
6. Only then resume larger v2 Graph/Cell/Pulse migration work.

The roadmap in [12-ROADMAP.md](12-ROADMAP.md) is ordered around that sequence.
