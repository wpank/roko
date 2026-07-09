# roko-graph — Cells, Graphs, Engine (v2 core)

> Status-quo audit · re-verified 2026-07-08 (git HEAD 5852c93c05, `main`) · prior pass 2026-07-07 · sources: 21 crate files (roko-graph src+tests+Cargo.toml, incl. the new `tests/fanout_condition.rs`), 5 roko-cli call-site files, 7 example graph TOMLs, 4 design docs (docs/v2/02-CELL.md, 03-GRAPH.md, 04-EXECUTION.md, 27-ORCHESTRATOR.md) + docs/v2-depth/{02-block,03-graph,05-execution-engine}, `.roko/GAPS.md`. All file:line refs re-checked against source on this date; every prior claim below **still holds**. Operator-facing companion: `37-RUNNER-V2-AND-GRAPH.md`.

## Summary

`roko-graph` (crates/roko-graph/, ~2.6K LOC, layer 2, depends only on roko-core) is a real but skeletal implementation of the v2 Cell/Graph/Engine model. The load-bearing pieces exist and are tested: `Cell` trait (`cell.rs:74`), petgraph-backed `Graph` (`types.rs:89`), TOML loader (`loader.rs:83`), `CellRegistry` (`registry.rs:16`), topological sort (`topo.rs:14`), sequential `GraphEngine` (`engine.rs:111`), plan-to-graph converter (`convert.rs:33`), and a tick-driven Hot Graph loop (`hot.rs:117`).

**The single most consequential finding**: `roko plan run` **defaults to the Graph Engine** (`--engine` clap `default_value = "graph"`, `crates/roko-cli/src/main.rs:1361`), and the Graph Engine path executes every task through `TaskExecutorCell` with `dry_run: true` (`engine.rs:356-358` → `task_executor.rs:31-34`). That cell never dispatches an LLM — it emits a synthetic `"task-output:dry-run:<title>"` engram and returns Complete (`task_executor.rs:70-79`). So the **default `roko plan run plans/` today performs zero real work, persists nothing (no lock, no episodes, no signals, no snapshot), and exits 0 with "SUCCESS"**. Real execution requires explicitly `--engine runner-v2`. The self-hosting workflow in CLAUDE.md (step 5: `roko plan run plans/`) silently no-ops on current defaults. `roko resume` is worse: it hardcodes `engine: PlanEngine::Graph` (`main.rs:2699`) then the Graph path discards the snapshot with a printed note (`commands/plan.rs:260-263`).

Every gap claimed in `.roko/GAPS.md` Tasks 101–103 (`GAPS.md:7-19`) was **verified still true**: dry-run fallback, no snapshot/resume, sequential-only (max_parallel stored as a metadata label only, `convert.rs:51`), all 7 cognitive-loop cells are `PassthroughCell` stubs (`stubs.rs:69-77`), `HotPolicy.persist_tick_state` declared but never read (`hot.rs:40` vs tick loop `hot.rs:145-208`), no `[graph.policy.hot]` TOML parsing (`loader.rs:20-29`), and edges treated as unconditional (`engine.rs:277-300` ignores `Edge.condition`). Additional findings beyond GAPS.md: two incompatible cell API families coexist (Engram-based `Cell` trait vs `NodeOutput`-based `AgentCell`/`ComposeCell` that the Engine cannot run), condition evaluation (`condition.rs:85`) and `BudgetTracker` (`budget.rs:17`) are fully built + tested but never invoked by the engine, and 3 of 7 example graph TOMLs use a schema the loader rejects. CLAUDE.md does not mention roko-graph, `roko graph`, or the `--engine` flag at all — it still describes orchestrate.rs as the main loop.

**New on 2026-07-08:** (a) A `crates/roko-graph/tests/fanout_condition.rs` file has appeared (24 tests, all green), but despite its name it exercises `condition::evaluate`, `BudgetTracker`, `NodeOutput`, and `GraphConfig` **in isolation** plus a linear engine smoke test — it does **not** test engine-level fanout/conditional traversal. The engine still contains zero references to `condition`/`evaluate`/`NodeOutput`/`BudgetTracker`/`max_parallel` (grep of `engine.rs`: 0 matches). The misleading name is itself drift: it reads like the conditional-edge gap is closed when it is not.
(b) The **`Cell` vs `Block` naming split is a docs-only drift**: there is no `Block` trait or struct anywhere in `crates/` (grep: 0 matches — code uses `Cell` exclusively, `cell.rs:74`). The v1→v2 rename landed in code and in `docs/v2/02-CELL.md`, but the `docs/v2-depth/02-block/` directory (16 files) still carries the old "Block" name. Navigation layers must not treat `02-block` as a distinct primitive; it is the deep-dive for the `Cell` primitive.
(c) **Three plan paths, not two.** Besides the Graph Engine (default) and Runner v2 (`--engine runner-v2`, backed by `roko-orchestrator`), a third `legacy-orchestrate` path exists in `run.rs` behind a non-default feature (see Old paradigm §). See `37-RUNNER-V2-AND-GRAPH.md`.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Cell trait | docs/v2/02-CELL.md:20-58 (`input_schema`/`output_schema`, `Result<Vec<Signal>, CellError>`) | `crates/roko-graph/src/cell.rs:74-106` | **Partial** — id/name/version/protocols/cost + `execute(Vec<Engram>) -> roko_core Result`; no TypeSchema, no CellError, protocols not enforced | cell.rs:87-105 |
| Graph struct | docs/v2/03-GRAPH.md:21-60 (GraphId content hash, entry/exits, GraphPolicy, Graph-as-Cell) | `types.rs:89-96` | **Partial** — metadata + petgraph DiGraph + node_map; no GraphId, no policy field, no schemas; entry/exit computed ad hoc (`topo.rs:63,80`); Graph does NOT impl Cell (no fractal nesting) | types.rs:89, 03-GRAPH.md:76-94 |
| Node kinds | 03-GRAPH.md:101-121 (SubGraph, Branch, FanOut, Loop, HumanInput, Wait, Slot, Noop) | `types.rs:56-70` | **Not implemented** — Node is `id + cell_type string + config`; no NodeKind enum | types.rs:56 |
| TOML loader | 03-GRAPH.md §TOML; 14-CONFIG | `loader.rs:83-131` | **Wired** — `[graph]` + `[[nodes]]` + `[[edges]]` w/ conditions; silently drops unknown sections (`[config]`, `[graph.policy.hot]`) | loader.rs:10-29 |
| Engine (sequential) | 04-EXECUTION.md:11-90 (Flow lifecycle, Bus pulses, Store) | `engine.rs:111-301` | **Partial** — topo-order for-loop, skip-on-upstream-failure; no Flow/RunId, no Bus, no Store, no pulses, no pause/cancel/estimate | engine.rs:132-225 |
| Parallel execution | 04-EXECUTION.md:205-215 (frontier + semaphore, `policy.max_parallelism`) | — | **Not implemented** — strictly sequential; `max_parallel` stored as string label only | engine.rs:144, convert.rs:51 |
| Conditional edges | 03-GRAPH.md §edges; cognitive-loop.toml:84,98 | parsed (`loader.rs:54-76`, `types.rs:17-31`) + evaluator built (`condition.rs:85-92`) | **Built, not wired** — engine never reads `Edge.condition`; `gather_inputs`/`has_failed_ancestor` ignore it; two divergent condition types (`EdgeCondition` vs `Condition`) | engine.rs:255-300 |
| Budget enforcement | 04-EXECUTION.md:35, budget pulses :148-150 | `budget.rs:17-180` + tests | **Built, not wired** — engine never calls `BudgetTracker::check()`; `CellContext.budget_remaining` never populated | plan.rs:1646 (`CellContext::new()`) |
| Snapshot / resume | 04-EXECUTION.md:55-56, 392-460 (FlowSnapshot, Workflow/Activity split) | — | **Not implemented** — `--resume-plan` ignored with printed note; `roko resume` hardcodes Graph engine then drops snapshot | plan.rs:260-266, main.rs:2697-2709 |
| Hot Graphs | 03-GRAPH.md §8; 04-EXECUTION.md:351-383 (retain outputs between ticks, checkpoint) | `hot.rs:117-223` | **Partial** — tick loop, cancellation, max_ticks, interval all work (4 tests); `persist_tick_state` never read; each tick starts from empty state; conservative fail-fast only | hot.rs:40,145-208 |
| `[graph.policy.hot]` parsing | implied by hot.rs:28 doc comment | — | **Not implemented** — `RawGraphMeta` has no policy field; HotPolicy constructed programmatically only | loader.rs:20-29, agent_serve.rs:416-420 |
| Plan → Graph converter | 27-ORCHESTRATOR.md:741-760 (runner as Engine precursor) | `convert.rs:33-106` | **Wired** — TaskDef→Node(task-executor), depends_on→unconditional edges, cycle check; cross-plan deps warn+skip | convert.rs:74-99 |
| TaskExecutorCell live dispatch | 04-EXECUTION.md (Activity cells) | `cells/task_executor.rs` | **Stub** — `dry_run:false` falls back to dry-run with a warning; default is `dry_run: true` | task_executor.rs:80-92, 31-34 |
| Cognitive loop (7 cells) | 04-EXECUTION.md:3, 306-345 (T0 short-circuit ~80% ticks); v2-depth/05-execution-engine/cognitive-loop-as-graph.md | `examples/graphs/cognitive-loop.toml` + `stubs.rs` | **Stub** — all 7 cells are PassthroughCell; auto-started as Hot Graph by `roko agent serve` (1s tick, infinite) doing nothing per tick | stubs.rs:69-77, agent_serve.rs:224,385-428 |
| T0 short-circuit | 04-EXECUTION.md:306-331 | — | **Not implemented** | — |
| Engine as `plan run` default | 27-ORCHESTRATOR.md:741-747 (migration step 4) | `main.rs:1361` `default_value = "graph"` | **Wired but hollow** — default path is a dry-run; Runner v2 reachable only via `--engine runner-v2` | plan.rs:257-267, 1567-1715 |
| `roko graph run/validate/show` CLI | — (authoring surface) | `commands/graph.rs:17-157` | **Wired** — load, validate (cycles + unknown cells), execute, show topo order | main.rs:621, 2500 |
| Lifecycle Pulses on Bus | 04-EXECUTION.md:131-150 | — | **Not implemented** — GraphOutput is in-memory; no bus, no events, no episodes on engine path | engine.rs:219-224 |
| GraduationCell | docs/v2 graduation policies | `cells/graduation.rs:37-160` | **Built, not wired** — impls Cell (passthrough in graph mode :115-124) + React (real logic :136-155); zero call sites outside roko-graph | grep: only lib.rs/mod.rs/graduation.rs |

## Cell census

Registered in `default_registry()` (`engine.rs:311-371`) — this is the registry used by both `roko graph run` (graph.rs:61) and `roko plan run` engine path (plan.rs:1644):

| cell_type | Implementation | Real or stub | Notes |
|---|---|---|---|
| `gate.compile` | `ShellCell` → `cargo check --workspace` | **Real** (spawns process) | engine.rs:314-321; config ignored (`\|_config\|`); command hardcoded, cwd inherited |
| `gate.test` | `ShellCell` → `cargo test --workspace` | **Real** | engine.rs:323-330; same limitations |
| `gate.clippy` | `ShellCell` → `cargo clippy … -D warnings` | **Real** | engine.rs:332-339; same limitations |
| `noop` | `NoopCell` (passthrough) | By-design passthrough | engine.rs:341 |
| `score` | `NoopCell` alias "ScoreCell" | **Stub** | engine.rs:343-345 — no scoring |
| `compose` | `NoopCell` alias "ComposeCell" | **Stub** | engine.rs:347-349 — the real `ComposeCell` in cells/compose.rs is NOT what gets registered |
| `act` | `NoopCell` alias "ActCell" | **Stub** | engine.rs:351-353 |
| `task-executor` | `TaskExecutorCell::default()` (`dry_run: true`) | **Stub** | engine.rs:356-358; live path unimplemented (task_executor.rs:80-92) |
| `signal-reader` | `PassthroughCell` | **Stub** | engine.rs:363-368, stubs.rs:69-77 (cognitive SENSE) |
| `relevance-scorer` | `PassthroughCell` | **Stub** | (ASSESS — threshold config ignored) |
| `system-prompt-builder` | `PassthroughCell` | **Stub** | (COMPOSE — roko-compose NOT called) |
| `claude-agent` | `PassthroughCell` | **Stub** | (ACT — no LLM) |
| `gate-pipeline` | `PassthroughCell` | **Stub** | (VERIFY — roko-gate NOT called) |
| `store-writer` | `PassthroughCell` | **Stub** | (PERSIST — nothing written) |
| `event-publisher` | `PassthroughCell` | **Stub** | (REACT — no events) |

Built but **unreachable from the Engine** (wrong interface — they do not implement the `Cell` trait; they use a separate `NodeOutput`-based execute signature):

| Type | File | State |
|---|---|---|
| `AgentCell` + `AgentDispatcher` trait + Mock/Failing dispatchers | `cells/agent.rs:127-314` | Real prompt-building + dispatch abstraction with token/cost accounting; `execute(&self, node_id, &[NodeOutput]) -> NodeOutput` (agent.rs:174) is NOT `Cell::execute`; not registered; used only by its own tests. The `AgentDispatcher` injection pattern is the intended solution to the layer constraint (roko-graph cannot depend on roko-agent) |
| `ComposeCell` | `cells/compose.rs:64-118` | Real `{{var}}` template substitution incl. upstream-output variables and `{{inputs}}`; same NodeOutput interface; not registered — meanwhile registered `compose` is a noop |
| `GraduationCell` | `cells/graduation.rs:37` | Implements `Cell` (but graph-mode execute is passthrough, :115-124) and `React` (real pulse-graduation logic, :136-155); never instantiated anywhere at runtime |

## Engine capabilities vs spec

- **Parallelism**: Spec = frontier scheduling with `tokio` task-per-node + shared semaphore up to `policy.max_parallelism` (04-EXECUTION.md:205-215). Code = single `for node_id in &order` loop awaiting each cell (`engine.rs:144-214`). `max_parallel` from tasks.toml survives only as `metadata.labels["max_parallel"]` string (convert.rs:51) that nothing reads.
- **Conditional edges**: Spec + cognitive-loop.toml require them (edges annotated "condition evaluation is not yet implemented; treated as always-true", cognitive-loop.toml:85,99). Loader parses `EdgeCondition::{Success,Failure,Always,OutputEquals}` (loader.rs:54-76); a richer `Condition::{Always,OnSuccess,OnFailure,When{field,op,value}}` + `evaluate()` exists with 9 passing tests (condition.rs:31-92) — but `GraphEngine::execute` never inspects `Edge.condition`; input gathering takes all predecessors unconditionally (engine.rs:277-300). Two condition types must first be reconciled.
- **Snapshot/resume**: Spec = `Engine::resume(FlowSnapshot)` with Workflow/Activity replay split (04-EXECUTION.md:55-56, 392-460). Code = nothing. `roko plan run --resume-plan` on the engine path prints "not yet supported… snapshots will be ignored" (plan.rs:260-263). `roko resume` builds `PlanCmd::Run { engine: PlanEngine::Graph, resume_plan: Some(snapshot) … }` (main.rs:2697-2708) → snapshot found, then discarded, then everything re-runs as dry-run.
- **Hot graphs**: `start_hot` (hot.rs:117-223) works: background tokio task, per-tick full-graph execution, cancellation token, `max_ticks`, `tick_interval_ms`, `last_output()`; 4 tests pass (hot.rs:238-333). Gaps vs spec (04-EXECUTION.md:351-383): no state retention between ticks (fresh `outputs` map per `engine.execute`; `persist_tick_state` is a dead field), no periodic checkpoint, no external-input injection per tick, no pause, fail = break loop. Only runtime consumer: `roko agent serve` auto-starts cognitive-loop.toml as a Hot Graph with hardcoded `HotPolicy { tick_interval_ms: 1000, max_ticks: None, persist_tick_state: false }` (agent_serve.rs:224, 385-428) — an infinite 1 Hz loop of 7 passthroughs.
- **Budget**: `BudgetTracker` (tokens/cost/deadline, per-node breakdown, tested — budget.rs) and `GraphConfig` (types.rs:334-344) exist; the engine neither constructs nor checks them. `CellContext { trace_id, run_id, budget_remaining }` is created empty at every call site (plan.rs:1646, graph.rs:63, hot.rs:135).
- **Observability/persistence**: none on the engine path — no Bus pulses, no `.roko/episodes.jsonl`, no signals, no workspace lock (lock acquired only inside the runner-v2 branch, plan.rs:272). Results exist only as printed `GraphOutput::summary()`.

## V2-aligned

- Cell trait + CellRegistry + factory-from-TOML-config pattern matches spec shape (cell.rs, registry.rs) — modulo missing schemas/CellError; uses `roko_core::Engram` (the current name of the Signal noun, roko-core/src/engram.rs:63).
- Graph-as-data: TOML `[graph]/[[nodes]]/[[edges]]` load → validate → interpret, never compiled (loader.rs, engine.rs:232-252) — exactly the 03-GRAPH.md:15 posture.
- Plan-to-graph converter cleanly maps Runner v2 tasks.toml → Graph with `task_def_json` round-tripped in node config for a future live cell (convert.rs:143-196); good tests incl. diamond/cycle/cross-plan (convert.rs tests + tests/plan_conversion.rs, 5 integration tests).
- Hot Graph handle/cancellation design (child `CancellationToken`, `wait()` idempotent) matches the resident-graph concept (hot.rs:57-103).
- `roko graph run/validate/show` gives the authoring loop a real CLI surface (commands/graph.rs; wired at main.rs:621, 2500).
- Cognitive loop is expressed as a 7-node graph TOML matching the spec topology Sense→Assess→Compose→Act→Verify→Persist→React with the verify→react failure edge (cognitive-loop.toml:77-108), and it loads/validates/executes green in CI (tests/plan_conversion.rs:132-156).

## Old paradigm & tech debt

- **Dry-run-by-default plan execution**: the flag default (main.rs:1361) makes the hollow engine the production path while the working executor hides behind `--engine runner-v2`. Either the default should revert until TaskExecutorCell is live, or the engine path must loudly declare itself a dry-run (it currently prints "SUCCESS").
- **Feature-flag façade**: `legacy-runner-v2` is a default cargo feature whose comment claims it gates the Runner v2 path (roko-cli/Cargo.toml:15,20), but **no `#[cfg(feature = "legacy-runner-v2")]` exists anywhere in `src/`** (re-verified 2026-07-08: 0 matches under `src/`; only test files reference it). Runner v2 always compiles. `legacy-orchestrate` (non-default) is the one that actually gates code — ~39 `#[cfg(feature = "legacy-orchestrate")]` sites in `run.rs` (lines 22–3724) plus `lib.rs`. That gated `run.rs` path is a **third, even-older orchestrator** (the pre-WorkflowEngine loop); with three plan paths in-tree, the Cargo comment's "runner-v2 is legacy" framing is doubly misleading.
- **Two cell API families**: Engram-based `Cell` trait (engine-executable) vs `NodeOutput`-based `AgentCell`/`ComposeCell`/`condition::evaluate` (engine-invisible). The real compose/agent logic is trapped behind the wrong interface while noops squat on their registry names.
- **Two condition types**: `types::EdgeCondition` (what the loader emits) vs `condition::Condition` (what `evaluate()` consumes). Unifiable but currently disjoint.
- **Example schema drift**: `single-gate.toml`, `linear-gates.toml`, `score-compose.toml`, `cognitive-loop.toml` parse; **`conditional-branch.toml`, `parallel-gates.toml`, `task-execution.toml` do not** — they use top-level `name =` (no `[graph]` table → "missing field `graph`"), cell types `agent`/`gate` (unregistered; registry has `gate.compile` etc.), condition types `on_success`/`on_failure`/`when` (not in `RawEdgeCondition`, loader.rs:56-65), and a `[config]` budget block the loader drops (conditional-branch.toml:8,31,84-124,150-154; parallel-gates.toml:8,33; task-execution.toml:11,42,124,158). They were written against the aspirational schema.
- **Config-ignoring gates**: ShellCell factories discard node config (`|_config|`, engine.rs:314-339) — `[nodes.config] timeout_secs/workspace` in examples are dead; commands run in the roko process cwd, not the plan's workdir (`cmd_plan_run_engine` receives `_workdir` and ignores it, plan.rs:1567-1569).
- **Loader silently swallows unknown sections** (no `deny_unknown_fields`): a user writing `[graph.policy.hot]` or `[config]` gets no warning, just missing behavior.
- **Cross-plan `depends_on_plan` silently skipped** at conversion (warn log only, convert.rs:91-99) — ordering guarantees from multi-plan runs are lost on the engine path.

## Undocumented / under-described: `roko-orchestrator` is the real Runner v2 engine

The `--engine runner-v2` path is backed by `crates/roko-orchestrator/` (driven from `roko-cli/src/orchestrate.rs`, which references `roko_orchestrator` 16×, and `runner/event_loop.rs`). This crate — not roko-graph — is the production plan engine today, yet the graph docs barely mention it. It ships the pieces the Graph Engine is missing:

- `ParallelExecutor` + `ExecutorConfig` + `ExecutorSnapshot`/`DeltaSnapshot` + `RecoveryEngine`/`RecoveryResumePlan` (`executor/` — real parallel dispatch, snapshot/resume, circuit breaker) — `roko-orchestrator/src/lib.rs:69-78`.
- `UnifiedTaskDag` + `IncrementalDag` + `ExecutionWave` + CPM analysis + cycle detection (`dag.rs`) — `lib.rs:62-67`.
- `MergeQueue` + `MergeRequest`/`MergeStatus`/`MergeConflict` (`merge_queue.rs`) — the merge queue the Graph Engine has no analogue for — `lib.rs:79-82`.
- `EventLog` (integrity-checked), `ProgressTracker`, `PostMergeRunner`, `replan`, `worktree`, `safety/` (loop_guard, capability_tokens, taint_propagation, sandboxing, audit_chain), `coordination` (pheromones/subnets/c_factor), `mesh_relay`.

**Navigation implication**: the "canonical engine" question is really "does roko-graph absorb roko-orchestrator's executor/merge/recovery, or does roko-orchestrator gain a Cell/Graph front-end?" The Minimum Graph Parity list in `37-RUNNER-V2-AND-GRAPH.md` is effectively "reimplement roko-orchestrator inside roko-graph" — a large lift that argues for keeping Runner v2 as the honest default until parity lands. This crate deserves its own status-quo doc; it is currently only glancingly covered here and in the orchestration audits.

## Not implemented

- TaskExecutorCell live dispatch (any path from graph node → roko-agent dispatcher → LLM) — task_executor.rs:80-92
- Parallel/frontier execution; any use of `max_parallel` — engine.rs:144
- Conditional-edge evaluation in the engine; `EdgeCondition`⇄`Condition` unification — engine.rs:277-300
- Snapshot write/read, `Engine::resume`, Workflow/Activity split, deterministic replay — 04-EXECUTION.md:392-460
- Budget enforcement wiring (`BudgetTracker.check()` per node; populate `CellContext`) — budget.rs vs engine.rs
- Hot tick state carryover (`persist_tick_state`), periodic checkpoint, per-tick external input — hot.rs:145-208
- `[graph.policy.hot]` (and any `[graph.policy.*]`) TOML parsing — loader.rs:20-29
- All 7 real cognitive cells (Store/Bus/compose/agent/gate integrations) + T0 short-circuit — stubs.rs, 04-EXECUTION.md:306-345
- Bus lifecycle pulses, episode/signal persistence, workspace lock on engine path — engine.rs, plan.rs:1567-1715
- Graph-as-Cell (SubGraph nesting), NodeKind variants (Branch/FanOut/Loop/HumanInput/Wait/Slot), GraphId content addressing, Type schemas — 03-GRAPH.md:21-121
- Engine `pause/cancel/status/list_active/estimate` API — 04-EXECUTION.md:51-78
- GraduationCell runtime registration (React loop on a Bus) — cells/graduation.rs

## Migration checklist

- [ ] **[P0]** Make default `roko plan run` honest: either flip `--engine` default back to `runner-v2` (main.rs:1361) or make the graph path refuse/warn loudly when the registry resolves `task-executor` to a dry-run stub — verify: `cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -i 'dry-run\|runner-v2'` (must not silently print SUCCESS with no agent spawned)
- [ ] **[P0]** Implement TaskExecutorCell live dispatch via an injected dispatcher (reuse the `AgentDispatcher` pattern from cells/agent.rs:134; construct registry in roko-cli where roko-agent is available, register a live `task-executor` factory instead of `TaskExecutorCell::default()` at plan.rs:1644) — verify: `cargo run -p roko-cli -- plan run plans/ --engine graph` produces real agent output + non-synthetic engrams
- [ ] **[P0]** Fix `roko resume`: stop hardcoding `PlanEngine::Graph` at main.rs:2699 (route to runner-v2 until graph snapshots exist) — verify: `cargo run -p roko-cli -- resume` actually resumes from `.roko/state/executor.json` instead of printing "snapshots will be ignored"
- [ ] **[P1]** Wire conditional edges: unify `EdgeCondition` (types.rs:17) with `Condition` (condition.rs:33), evaluate in `GraphEngine::execute` when gathering/activating successors — verify: `cargo test -p roko-graph` with a new engine-level test where an `OnFailure` edge fires only after upstream failure. **NB** `tests/fanout_condition.rs` is misnamed — it tests conditions/budget in isolation, not engine fanout; rename it (e.g. `condition_unit.rs`) or repurpose it into the real engine-level test so the filename stops implying the gap is closed.
- [ ] **[P1]** Parallel frontier execution honoring `max_parallel` (JoinSet + semaphore per 04-EXECUTION.md:205) — verify: `cargo test -p roko-graph parallel` w/ a timing/ordering assertion; then `roko plan run --engine graph` on a 2-branch plan shows concurrent node starts
- [ ] **[P1]** Engine-path persistence + lock: acquire workspace lock, write per-node results as signals/episodes, emit start/complete events (parity with runner-v2 branch plan.rs:272-775) — verify: `roko plan run --engine graph && wc -l .roko/episodes.jsonl` increases
- [ ] **[P1]** Snapshot/resume for graph runs: serialize `{graph_name, completed node outputs}` to `.roko/state/`, skip Complete nodes on resume — verify: interrupt a run, `roko plan run plans/ --engine graph --resume-plan .roko/state/executor.json` re-runs only pending nodes
- [ ] **[P1]** Budget wiring: build `BudgetTracker` from `GraphConfig`, call `check()` before each node and `record()` after; parse `[config]`/`[graph.config]` in loader — verify: `cargo test -p roko-graph budget` incl. an engine test that stops with `BudgetExceeded`
- [ ] **[P2]** Parse `[graph.policy.hot]` into `HotPolicy` in loader.rs (the type already derives Deserialize, hot.rs:29) and read it in `agent_serve.rs:416` instead of hardcoding — verify: set `tick_interval_ms = 5000` in cognitive-loop.toml, `roko agent serve` logs 5s ticks
- [ ] **[P2]** Implement `persist_tick_state`: retain previous tick's exit outputs and feed them as next tick's entry inputs (04-EXECUTION.md:359,374) — verify: `cargo test -p roko-graph hot_graph_persists_state` (new test asserting engram carryover across ticks)
- [ ] **[P2]** Replace the 7 PassthroughCell cognitive stubs with real cells, starting with `store-writer` + `signal-reader` (roko-fs substrate) and `gate-pipeline` (roko-gate) — registered from roko-cli to respect layering — verify: `roko agent serve` tick actually reads/writes `.roko/signals.jsonl`
- [ ] **[P2]** Port `AgentCell`/`ComposeCell` to the `Cell` trait (or delete them) and register real `compose`/`act`/`score` cells over the current noop aliases — verify: `roko graph run examples/graphs/score-compose.toml` performs real template substitution (assert output text)
- [ ] **[P2]** Fix or quarantine the 3 unparseable example TOMLs (conditional-branch, parallel-gates, task-execution) to the current schema; add a CI test that loads every file in examples/graphs/ — verify: `for f in examples/graphs/*.toml; do cargo run -p roko-cli -- graph validate $f; done` all pass
- [ ] **[P2]** Make ShellCell gates honor node config (workdir, timeout, custom args) and run in the plan's workdir, not process cwd — verify: `roko graph run` from a different cwd still gates the target workspace
- [ ] **[P3]** Reconcile the `legacy-runner-v2` feature: either add the promised `cfg` gates around the runner path or delete the feature and its Cargo.toml comment (roko-cli/Cargo.toml:14-20) — verify: `cargo build -p roko-cli --no-default-features` behaves as documented
- [ ] **[P3]** Emit lifecycle pulses per 04-EXECUTION.md:131-150 once a Bus exists on the engine path — verify: `roko dashboard` shows graph-run node transitions live
- [ ] **[P3]** Graph-as-Cell / SubGraph nesting + NodeKind variants; cross-plan `depends_on_plan` via nested graphs — verify: multi-plan run test where plan B waits on plan A
- [ ] **[P3]** Register GraduationCell into a runtime React loop or move it out of roko-graph — verify: grep shows a non-test call site constructing it
- [ ] **[P3]** Update CLAUDE.md: add roko-graph to the crate table, document `--engine` semantics and `roko graph` subcommands — verify: CLAUDE.md mentions `roko-graph` and the current `plan run` default

## Open questions

1. **Is the graph-default for `plan run` intentional?** Making a dry-run stub the default executor (main.rs:1361) reads like a premature flip of 27-ORCHESTRATOR.md's migration step 4 ("Replace event loop with Engine interpretation", :741). If deliberate as a forcing function, the SUCCESS output is still misleading; if accidental, it's a P0 regression to self-hosting.
2. **Which cell interface is canonical?** `Cell` (Engram in/out) vs the `NodeOutput` family (agent.rs/compose.rs/condition.rs). `NodeOutput` carries status/tokens/cost that conditions and budgets need; `Engram` is the spec noun. Probably: keep `Cell`, wrap results in `NodeOutput` inside the engine — but someone must decide before wiring conditions.
3. **How does live dispatch cross the layer boundary?** roko-graph is layer 2 (Cargo.toml:36) and depends only on roko-core; roko-agent is higher. Options: registry injection from roko-cli (cheapest, matches `AgentDispatcher`), or a roko-core dispatch trait. Which?
4. **Signal vs Engram naming**: docs/v2 say `Vec<Signal>`; code is `roko_core::Engram` (engram.rs:63). Is a docs pass planned, or a code rename back?
5. **Where do graph definitions live for real runs?** agent_serve looks in `examples/graphs/` then `.roko/graphs/` (agent_serve.rs:389-392); neither `.roko/graphs/` nor any `.roko/plans/*/tasks.toml` exists in this repo today — is `.roko/graphs/` the intended canonical location?
6. **Hot cognitive loop cost**: `roko agent serve` unconditionally ticks a no-op graph at 1 Hz forever (agent_serve.rs:224). Should it stay off until at least one real cell exists?
