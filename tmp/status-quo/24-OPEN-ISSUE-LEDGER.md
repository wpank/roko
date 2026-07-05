# Open Issue Ledger

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`. Evidence detail: [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md), [92-RUNNER-V2-MODULE-FAMILY.md](92-RUNNER-V2-MODULE-FAMILY.md), [75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md), [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md), [40-LEARNING-TELEMETRY.md](40-LEARNING-TELEMETRY.md). Second-pass execution traces: [96-TRACE-RUNNER-V2-EXECUTION.md](96-TRACE-RUNNER-V2-EXECUTION.md), [97-TRACE-SERVE-LIFECYCLE.md](97-TRACE-SERVE-LIFECYCLE.md), [99-TRACE-AGENT-TURN.md](99-TRACE-AGENT-TURN.md), [101-TRACE-GATE-PIPELINE.md](101-TRACE-GATE-PIPELINE.md), plus censuses [102](102-SPEC-DEBT-LEDGER.md)/[103](103-DUPLICATE-TYPES-CENSUS.md)/[104](104-DEAD-CODE-AND-FACADE-CENSUS.md).

## P0

| Issue | Current evidence | Done when |
|---|---|---|
| Default `roko plan run` is a dry-run graph path ([95](95-ENGINE-DRIFT.md)) | Clap `default_value="graph"` (`main.rs:1362`) overrides enum `#[default] RunnerV2` (`main.rs:1301`); `TaskExecutorCell` `dry_run:true` default (`task_executor.rs:32`) + live branch unimplemented (`:81-89`) → emits `task-output:stub:`. | Default plan run dispatches real work, or refuses non-dry-run with a clear message. |
| `roko resume` ignores usable snapshots | Resume hardcodes Graph (`main.rs:2699`) and Graph ignores `--resume-plan`. | Resume routes to Runner v2 (auto-resume) or Graph snapshots exist. |
| Relay proxy is fully unauthenticated ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | `/relay/{*path}` + 2 WS merged at top-level router **outside** `/api` (`routes/mod.rs:248`) → GET/POST/DELETE + WS bypass `require_api_key`/`require_scope`. | Relay nested under `/api` or wrapped in auth; 401-without-key test passes. |
| Read-scope auth fallback authorizes writes ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | Unlisted mutating `/api/*` routes fall through to `"read"` (`middleware.rs:385`), which always passes → read key can POST run/jobs/dream/research/deploy. | Fallback is deny/`write`; CI test fails when a mutating route lacks explicit scope. |
| Per-tool safety funnel is bypassed on the default provider ([99](99-TRACE-AGENT-TURN.md)) | The `ToolDispatcher`→`SafetyLayer::check_pre_execution` 9-policy pre-check runs **only** on the OpenAI-compat roko `ToolLoop` (models with `supports_tools`, `openai_compat.rs:388`). The **default Claude-CLI provider** (and Codex) drive their own subprocess tool loop and never touch roko's `SafetyLayer` per tool call — tool safety is delegated wholesale to Claude's own permission system with `--dangerously-skip-permissions:true` default (`dispatch_v2.rs:1202` fork; bypass at `99` §7). Role auth / bash-git / net / path / budget / temporal / contract policies do not run for the default self-host path. | Claude/Codex tool calls run through a roko-side pre-check, or the [BYPASS] is a documented, deliberately-encoded boundary (`build_settings_json` encodes equivalent policy) with an integration test proving the same denial on CLI + ToolLoop paths. |
| `research search` / `/search` is 100% broken | Perplexity **batch** request body → HTTP 422; mock tests are false-green ([40](40-LEARNING-TELEMETRY.md)/[91](91-PRD-RESEARCH.md)). | Correct single/non-batch body; a live (non-mock) test hits Perplexity and returns results. |
| Runtime source of truth is ambiguous | Runner v2, Graph, WorkflowEngine, and legacy docs all claim runtime responsibility. | A decision doc states which path is production and which are compatibility/target. |
| Foundation contracts are fragmented | `DispatchPlan`, `RunLedger`, `CommitOutcome`, `GateStatus`, `RoutingContext` exist in different crates. | One contract layer or explicit adapter boundaries exist. |
| Docs teach unsafe/stale commands | Root docs and v2 orchestration docs still imply default plan run is live. | User-facing docs match current engine semantics. |
| Demo/API has known hard breaks | 4 frontend→serve 404s: `share` vs `shared`, `bench/matrix`, `isfr/stream`, `ws/agents`; plus camelCase/snake_case event drift. | Route aliases or frontend fixes are committed and route-contract tests cover them. |
| Storage divergence hides dashboards & breaks serve | Gate verdicts write `signals.jsonl` but dashboards read `engrams.jsonl` (empty panels); serve still tries to READ `state/executor.json` (real snapshot is `state-snapshot.json`) → error; `events.jsonl` 44 MB / 97% `feed_tick` firehose. See [60](60-STATE-PERSISTENCE-LEDGER.md). | signals↔engrams converge; serve reads `state-snapshot.json`; event firehose trimmed. |
| Source docs lack status/provenance tags | 636 source docs plus research/reference corpora are not assigned current/partial/target/stale/archive status. | Generated manifest tags every source doc and maps it to a status-pack owner. |
| Ops docs overstate deployment readiness | Root Docker/Railway/dev-compose assume `roko.toml`; compose uses stale `--listen`; Fly configs disagree. | Clean-checkout Docker, compose, Railway, and Fly smoke proofs pass or docs are downgraded. |
| Maintained root docs are stale | README and CLAUDE still carry old counts, unsafe default `plan run`, old resume/neuro/TUI/tool/safety claims. | Rewrite queue in `81` is completed and root docs cite current proof gates. |

## P1

| Issue | Current evidence | Done when |
|---|---|---|
| Graph Engine lacks parity | No live task dispatch, resume, gates, budgets, events, parallelism; a real `AgentCell` exists but `default_registry()` never binds it. | Graph run can execute a real plan end-to-end with gate and resume proof. |
| ACP permission gate has zero prod callers ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | `request_permission` fully built + tested but never called; `write_file`/`edit_file`/`bash` run unconditionally (`builtin_tools.rs:291`). | Tool loop calls `request_permission` before mutating tools; E2E for write/bash/fetch. |
| Tool-alias bug strips tools on non-Claude providers | PascalCase vs snake_case mismatch (`openai_compat.rs:252,348`) removes ALL tools → breaks research analyze/enhance/prd agents on OpenAI/Gemini/Ollama. | Alias casing normalized; a non-Claude provider agent completes a tool call. |
| Safety post-checks are Warn-only ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | SecretLeak/PathEscape/ContractViolation are `Warn` severity (`safety/mod.rs:767`) — log, do not block. | SecretLeak + PathEscape promoted to `Block`; a leaking turn is denied. |
| Custody verify is false audit assurance ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | `custody verify` prints "Chain integrity: OK" but only checks JSON-parse + monotonic timestamps (`custody.rs:206`); real hash-chained audit is dead code. | Verify runs the real hash-chained check or the command is removed/relabelled. |
| Cold-substrate archival copies instead of moves | Runtime-wired (`roko-serve/lib.rs:344`) but `cold_substrate.rs:218` copies-not-moves → unbounded hourly re-append. | Archival moves (deletes source) or dedups; cold store size is bounded over repeated cycles. |
| `config show --effective` prints secrets unredacted ([75](75-SECURITY-AUTH-SCOPE-MATRIX.md)) | `serialize_effective` bare TOML dump after secret interpolation (`config_cmd.rs:222`, `loader.rs:567`). | Secret-typed fields redacted; test with a seeded key. |
| Worker callback has no auth header | Deployed worker calls back without an auth token. | Callback carries a scoped token; unauthenticated callback is rejected. |
| Runner v2 legacy holdouts ([92](92-RUNNER-V2-MODULE-FAMILY.md), [96](96-TRACE-RUNNER-V2-EXECUTION.md)) | Conductor supervision loop unwired (`conductor_load` hardcoded 0.0, `event_loop.rs:4258`); agent-driven replan is prompt-enrichment only (`set_replan_context`, no `tasks.toml` rewrite); worktree isolation built in roko-orchestrator, unwired (agents share one dirty tree, "merge" = cargo-check, `merge.rs:146-205`). | Each holdout is ported to the runner or recorded as an explicit non-goal. |
| No live task DAG; intra-plan parallelism does not exist ([96](96-TRACE-RUNNER-V2-EXECUTION.md)) | Runner v2 schedules a flat `task_index: HashMap` + per-plan FSM; `runner/task_dag.rs::TaskDag` and `UnifiedTaskDag` are dead/legacy-only (`event_loop.rs:62,484-497`). Concurrency is **per-plan**, fixed at `max_concurrent_plans = 4` (`defaults.rs:313`), **one agent per plan**. `--max-tasks`/`max_concurrent_tasks` only sizes the gate semaphore (`event_loop.rs:473`), it never runs N tasks of one plan in parallel. "Parallel task execution" in prior docs is per-plan, not per-task. | Real DAG scheduling parallelizes ready tasks within a plan, or the flat-index/per-plan model is documented as the intended design and `task_dag.rs` is deleted. |
| Live gate path is shallow — adaptive gates are NOT live ([101](101-TRACE-GATE-PIPELINE.md)) | `roko plan run` → `event_loop::run` → `gate_dispatch::run_gate_once` builds the pipeline with `RungExecutionInputs::default()` (`gate_dispatch.rs:104`) and **never calls `enrich_rung_config`**. All adaptive thresholds (SPC/CUSUM/EWMA/Hotelling), oracles 4-6 (Symbol/GenTest/FactCheck/LlmJudge/Integration), ratchet, and `VerdictPublisher` live ONLY on the dead `orchestrate.rs` `PlanRunner`. Live path: rungs 3-6 stub-pass `Verdict::pass` (`rung_dispatch.rs:290`), EMA only ever updates rung 2 (`completion.rung ≡ max_gate_rung`, `event_loop.rs:1128`), `GateThresholds::save` has zero callers so `.roko/learn/gate-thresholds.json` is read-only-at-startup, and the incremental rung-climb branch (`event_loop.rs:1206`) is dead. Stub passes inflate the single rung-2 EMA toward 1.0. | Enrichment ported into `run_gate_once`; stubs report Skipped/NotWired and are excluded from the EMA; thresholds persisted per real rung. |
| Gate stubs can look like passes | Missing rich rung inputs return passing stub verdicts (`stub_verdict → Verdict::pass`, `rung_dispatch.rs:290`); `passed = all(verdicts.passed)` counts stub passes (`gate_dispatch.rs:140`); graph gate cell is passthrough. | Stub verdicts are neutral/skipped and excluded from pass-rate learning. |
| CascadeRouter LinUCB state never persisted ([96](96-TRACE-RUNNER-V2-EXECUTION.md)) | The router is loaded/`load_or_new` at dispatch (`plan.rs:440`) and a **second** router lives in the learning subscriber (`event_loop.rs:754`) — two writers to `.roko/learn/cascade-router.json`. The learned LinUCB arm state is not durably persisted across restart on the live path (resets toward identity), and the dual-writer is a concurrency smell. | Single router owner (or file lock); LinUCB matrices survive a restart and a test proves warm state is reused. |
| `events.jsonl` is a write-only firehose nothing reads ([97](97-TRACE-SERVE-LIFECYCLE.md)) | 44 MB / 157,264 lines, **97% `feed_tick`**; `DashboardSnapshot::apply` treats `FeedTick`/`FeedAgentOnline/Offline` as no-ops (`dashboard_snapshot.rs:1223-1225`) and snapshot bootstrap reads a *different* file (`state/events.json`, `dashboard_snapshot.rs:1277`). Two producers (serve StateHub `DashboardEvent` + `roko plan run` runtime records) append to one uncapped file with two schemas; nothing on the dashboard path reads it back. Reinforces empty-panels. | Cap/rotate `events.jsonl` or stop persisting no-op `DashboardEvent`s; feed panels hydrate from the snapshot on reconnect. |
| Builtin tool count vs executable handlers ([99](99-TRACE-AGENT-TURN.md)) | `TOOL_COUNT=37` counts registry `ToolDef`s, but only **16** have executable handlers via `handler_for` (`roko-std/.../tool/handlers.rs:28-43`); a call to any of the other ~21 defs passes dispatcher schema/registry stages then fails at handler resolution (`Other("no handler")`, `dispatcher/mod.rs:462`). | `handler_for` and the registry `ToolDef` set agree, or the def-without-handler set is documented explicitly. |
| Live prompt path bypasses the canonical builder ([103](103-DUPLICATE-TYPES-CENSUS.md) row 12) | Runner-v2 prompts are assembled by the CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`), **not** `SystemPromptBuilder`/12-slot/`RoleSystemPromptSpec`/VCG. The 9-layer builder + VCG attention auction (`roko-compose/auction.rs:380`) run only on non-default paths and are reachable-but-cold (greedy dominates); `AttentionBidder` is compose-path only ([102](102-SPEC-DEBT-LEDGER.md)). So "the 9-layer builder is the live prompt path" is false for `roko plan run`. | One prompt-assembly surface (see [26](26-CANONICAL-DECISIONS.md) D15), or the two are documented as an explicit compat split with the live one named. |
| Episode roots split | Root, learn, and memory episode logs coexist. | One canonical write path; compatibility readers documented. |
| Event model split | Runtime `EventBus`, server `EventBus`, StateHub, DashboardEvent, ServerEvent, RuntimeEvent, learn bus. | Canonical event taxonomy and bridge loss policy documented/tested. |
| Server auth scopes under-protect routes | Scope matrix escalates only a few prefixes. | Mutating routes require write/admin scopes by default. |
| API docs are partial | OpenAPI omits many mounted namespaces and does not label synthetic/proxy/in-memory routes. | Generated route manifest and API docs agree on every mounted route family. |
| ACP permission/capability mismatch | `request_permission` exists; builtin tools can bypass; image and MCP capabilities inconsistent. | ACP initialize truthfully advertises capabilities and tool execution requests permissions. |
| Provider/tool dispatch not universal | roko-std advertises more tools than default handlers; providers have separate bypasses. | Shared dispatcher, safety, metrics, and MCP resolution apply to all managed tool calls. |
| Learning feedback loses fidelity | Runner conversion uses default model source and zeroed totals in some paths. | Model source, cost, latency, and outcome are recorded from real execution. |
| Frontend DataHub migration incomplete | Deprecated providers still wrap app; broken ISFR/dream event endpoints remain. | React app uses one state store and endpoints match server routes. |
| Chain/ISFR integration split | In-memory registries, Solidity contracts, local jobs, chain watcher, and tools are not one authority. | One documented identity/job/ISFR path with live or mocked boundaries explicit. |
| Crate boundaries drift from docs | README/v1 crate maps describe fewer/different crates; `roko-runtime` depends on concrete `roko-gate`. | Current crate map is regenerated and runtime consumes gate contracts through an appropriate boundary. |
| CI/release gates are under-scoped | CI lacks deny, frontend, Foundry, runtime smoke, Docker health, feature matrix, and release preflight; coverage ignores run failures. | Release workflow depends on proof gates for shipped artifacts. |
| Security trust boundaries are under-documented | Unmatched mutating routes fall back to `read`; terminal, workspace creation, Privy admin, ACP permissions, and MCP stdio have separate trust models. | Route auth manifest, terminal/workspace scope tests, ACP permission enforcement, and MCP trust docs exist. |
| Data contracts are hand-maintained | Runtime/Dashboard/Server/Runner/workflow events serialize differently; `.roko/events.jsonl` can mix schemas; TS mirrors drift. | Schema registry, generated fixtures, and event bridge coverage tests exist. |
| Examples and graph TOMLs look proof-like | Some graph examples are stale schema or topology-only; PRD plans include historical `[[tasks]]` shape. | Every example/plan has `live`, `stub`, `target`, `unsupported`, or `archive` status. |
| Env vars lack generated ownership enforcement | Direct env reads span core config, serve, relay, Mirage, MCP, worker, tests, and secrets. | `83` is regenerated in CI and undocumented new direct env reads fail. |

## P2

| Issue | Current evidence | Done when |
|---|---|---|
| VCG prompt auction unreachable | Empty learning bidders mean greedy strategy wins; `vcg_allocate` built + exported but greedy path dominates. | Runtime observations warm bidders or VCG is downgraded to diagnostics. |
| HDC compiled out | `hdc_vector: null` per episode; HDC features off by default in key crates. | HDC is enabled/backfilled or docs say lexical/JSONL retrieval is canonical. |
| Dreams routing advice not consumed | Advice artifacts exist but do not feed CascadeRouter. | Dream advice affects routing or is removed from claims. |
| Daimon state has duplicate paths | `.roko/daimon/affect.json` and `.roko/state/daimon.json` both appear. | One canonical Daimon path. |
| MCP config shape sprawl | `.mcp.json`, `.roko/mcp.json`, `.roko/mcp-config.json`, config fields differ. | One accepted shape plus compatibility migration. |
| Plugins/extensions are half SDK, half hooks | Manifest data and extension hooks do not form a product plugin lifecycle. | `roko plugin` or documented manual lifecycle works end-to-end. |
| Static demo leftovers confuse product status | `demo/demo-web` and `tmp/demo-uis` are not the current React app. | Archived or explicitly labeled. |
| Research prompts can leak into roadmap as facts | v2-depth research prompts include pitch/demo/category claims that are not implementation proof. | Prompts have strategic-source banners and current commands are revalidated. |

## P3

| Issue | Current evidence | Done when |
|---|---|---|
| Old tmp scratch is unranked | Thousands of tmp files include scratch and source designs. | Source ranking is enforced and archive moves are done. |
| Legacy code remains without ownership | `orchestrate.rs` and roko-orchestrator modules carry behavior with no target owner. | Each module is live, ported, quarantined, or deleted. |
| Docs convergence never ran | `tmp/doc-convergence` output is empty. | A docs convergence run produces canonical updated docs or is formally retired. |
| CLI command inventory is stale | `surface_inventory.rs` and init help still mention old surfaces/flags such as `--resume`. | Inventory is generated from Clap and every documented command has a smoke/proof tier. |

## Resolved / Downgraded Since Prior Audit

These CLAUDE.md roadmap items are now DONE and should stop appearing as pending (they moved to the P1 ledger only where a *new* bug was found):

| Item | Status now | Evidence |
|---|---|---|
| Roadmap item 13: knowledge-informed agent routing | **Wired** | `orchestrate.rs:15509` + `cascade_router.rs:623` consult the neuro store for model selection. |
| Roadmap item 14: cold-substrate archival | **Runtime-wired** (but copy-not-move bug filed P1) | `roko-serve/lib.rs:344` triggers hourly; bug at `cold_substrate.rs:218`. Copy-not-move + unbounded re-append confirmed second pass. |
| Workspace-role auth | **Landed** | Durable `team.rs` store. |
| daimon & dreams "dead-by-default" | **Corrected — live per-engine** | On `--engine runner-v2`/`serve`/`do`: daimon modulates dispatch per task and writes `affect.json` (`event_loop.rs:4247,4344,1395`); `DreamRunner` runs plan-completion consolidation (`event_loop.rs:1952`, gated on `agent_calls>0` + config). Dead only on the **default Graph engine**. Remaining holdouts: cron/periodic dream trigger, dream routing-advice consumption. See [96](96-TRACE-RUNNER-V2-EXECUTION.md) §13. |

## Corrections to earlier ledger claims (second pass)

These correct/retire specific wordings elsewhere in the pack; they are not new open issues except where noted.

| Prior claim | Correction | Evidence |
|---|---|---|
| `error_recovery.rs` handles failures | **Does not exist.** No such file; recovery lives in the safety `check_recovery`/retry paths. | [96](96-TRACE-RUNNER-V2-EXECUTION.md)/[99](99-TRACE-AGENT-TURN.md) |
| `AgentContract` falls back to a permissive default when YAML is missing | **Fails CLOSED** to `RestrictedFallback` (`safety/mod.rs:929`). The only permissive case is an operator who explicitly configured TOML role-tools with no tools list (`:949`). | [99](99-TRACE-AGENT-TURN.md) §6.2 |
| `legacy-runner-v2` gates the runner path | **Façade** — a default-on feature with **0** `#[cfg]` sites in `src/`; it gates only 4 test files. The real selector is the runtime `--engine` arg. `legacy-orchestrate` (off) is the only feature that gates code. | [104](104-DEAD-CODE-AND-FACADE-CENSUS.md) Part 1 |
| Demurrage economy is wired | **Taxes only** — confidence decays, but income/reinforce is dead: `RuntimeKnowledgeLifecycle` has 0 external callers, balances stuck at 0.0. | [102](102-SPEC-DEBT-LEDGER.md) |
| Conductor = 39 patterns / 21 categories, wired | **37 patterns / 20 categories**, and unwired in the live engine (only reachable from dead `orchestrate.rs`). | [102](102-SPEC-DEBT-LEDGER.md)/[104](104-DEAD-CODE-AND-FACADE-CENSUS.md) |
| Extension hooks fire broadly | **Only 6 of 17 hooks fire, all observe-only** on the live path. | [102](102-SPEC-DEBT-LEDGER.md) |
| Compose 9-layer builder / VCG is the live prompt path | **Not on the default/runner path** — see the P1 "Live prompt path bypasses the canonical builder" row above and [103](103-DUPLICATE-TYPES-CENSUS.md) row 12. | [103](103-DUPLICATE-TYPES-CENSUS.md) |
