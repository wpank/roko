# Post-Parity Rules

## CRITICAL: Do NOT compile or run tests

**DO NOT run any of these commands:**
- `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`
- `rustc`, `rustfmt`, `cargo fmt`
- Any compilation or test execution

**WHY:** Compilation is handled by a separate validation pipeline AFTER your changes are merged. Running cargo wastes significant time and resources. Just write correct code and commit it. Focus on writing code, not verifying it compiles.

If you need to understand types or signatures, READ the source files instead of compiling.

## Universal Anti-Patterns

- A second provider resolution chain.
- A second prompt assembly path for the same mode.
- A second chat/session state owner.
- Raw provider HTTP in CLI code when an adapter exists.
- Demo data shown as live data.
- Unknown usage recorded as zero.
- Stub gate counted as pass.
- Process success treated as artifact success.
- A new top-level crate for behavior that already exists in a current crate.
- A broad `orchestrate.rs` refactor mixed with behavior changes.

## HTTP-Client Anti-Patterns (Runner PA)

HC-1. **One shared client.** There is exactly ONE `reqwest::Client` per process, owned by a shared struct. Every HTTP call goes through it. If you are tempted to call `reqwest::Client::new()`, STOP. Use the shared client.

HC-2. **Connection pool = performance.** `reqwest::Client` internally manages a connection pool. Creating a new client per request destroys keep-alive, TLS session reuse, and HTTP/2 multiplexing. This causes the 7-43s latency regression observed in production.

HC-3. **Arc<dyn HttpPoster> is the seam.** All LLM dispatch, health checks, and API calls go through `HttpPoster`. Wire it once at startup, pass it everywhere.

## Chat-Dispatch Anti-Patterns (Runner PB)

CD-1. **System prompt reaches the LLM.** If `session.system_message` is set, it MUST appear in the API request body. Storing it and not sending it is a bug.

CD-2. **Tools are passed through dispatch.** `ChatAgentSession.tools` must reach the Claude API/CLI invocation. Listing tools in the session but not sending them means the agent has no capabilities.

CD-3. **History is the conversation.** The message history in `ChatAgentSession` is sent as the `messages` array. Not sending history means every turn is stateless.

CD-4. **One dispatch path.** `dispatch_direct.rs` hand-rolls provider HTTP. This is wrong. Chat dispatch should go through the adapter layer (`ClaudeCliAgent`, `ClaudeApiAgent`, or `ModelCallService`).

## Streaming Anti-Patterns (Runner PC)

ST-1. **StreamingState.append() must be called.** The method exists. SSE token deltas arrive. But the two are not connected. Wire them.

ST-2. **TUI observes StreamingState.** The TUI should read from `StreamingState` for live token display. If append is never called, the TUI shows nothing during generation.

## Slash-Command Anti-Patterns (Runner PD)

SC-1. **Commands that confirm must apply.** If `/effort high` prints "set to high", it MUST actually change the effort level for subsequent dispatches. Confirmation without effect is a lie.

SC-2. **Config writes go through the config system.** `/config set key val` must write to `roko.toml` via the config module, not just print what it would do.

## Safety Anti-Patterns (Runner PE)

SA-1. **Default is safe.** `dangerously_skip_permissions` defaults to `false`. Any code that sets it to `true` without explicit user opt-in is a security hole.

SA-2. **Permissive contracts are test-only.** `AgentContract::permissive()` must never appear outside `#[cfg(test)]` blocks.

## Freeze Anti-Patterns (Runner PF)

FZ-1. **Freeze means freeze.** Changing the `legacy-orchestrate` default to `false` means orchestrate.rs stops compiling by default. No new code goes into orchestrate.rs. Period.

FZ-2. **Compile warning on opt-in.** If someone enables `legacy-orchestrate`, they get a `#[deprecated]` warning.

## Memory Anti-Patterns (Runner PG)

MG-1. **Vectors that grow must drain.** `efficiency_events: Vec<_>` in orchestrate.rs grows unbounded during long runs. It must be flushed to disk periodically.

MG-2. **Display unknown as unknown.** Model name "-" in the TUI should show "unknown", not a dash.

## Plan-Execution Anti-Patterns (Runner PH)

PX-1. **Parallel means parallel.** If `max_concurrent_tasks = 4` and 4 tasks are ready, all 4 dispatch simultaneously. Hardcoding to 1 negates the DAG.

PX-2. **Cargo is a shared resource.** Multiple concurrent `cargo test` processes thrash build caches. Use a semaphore.

PX-3. **Plans have time limits.** A plan without a wall-clock timeout can loop forever.

PX-4. **Exclusive files prevent conflicts.** Two tasks modifying the same file must not run simultaneously.

## Learning-Loop Anti-Patterns (Runner PI)

LL-1. **Episodes record actuals, not requests.** The actual model used (from response headers) matters, not what was requested. The actual provider, not the default.

LL-2. **Thresholds must load.** Gate thresholds saved from a previous run must be loaded on the next. Fresh defaults = lost learning.

LL-3. **Manual overrides are data.** `--force-backend` successes/failures are observations, just dampened.

## Persistence Anti-Patterns (Runner PJ)

PS-1. **Snapshot everything learnable.** Router state, daimon state, cost totals — all in the snapshot. Losing any on crash = lost learning.

PS-2. **JSONL writes are atomic.** A crash must never leave a partial JSON line. Use fsync.

PS-3. **Resume detects drift.** If tasks changed between crash and resume, completed tasks may need re-running.

## Knowledge-Routing Anti-Patterns (Runner PK)

KR-1. **Knowledge informs routing.** The neuro store holds patterns about what works. The router must query it before selecting a model.

KR-2. **Affect modulates routing.** Under stress, prefer cheaper models. High confidence, allow expensive ones.

KR-3. **Knowledge informs prompts.** Previous successful strategies belong in Layer 3 of the system prompt.

KR-4. **Dreams run after plans.** Triggers accumulate during execution. Worker drains them after completion.

KR-5. **No raw API keys in library code.** Episode distillation goes through ModelCallService, not direct env var reads.

## ACP Anti-Patterns (Runner PL)

AC-1. **MCP tools must dispatch.** Declaring MCP servers in the session is useless if tool calls never route to them.

AC-2. **API fallback exists.** Not everyone has Claude CLI. Pipeline phases fall back to API providers.

AC-3. **Cost is tracked.** `total_cost_usd: 0.0` is a lie when real money was spent.

AC-4. **ACP respects safety defaults.** Pipeline phases don't hardcode `--dangerously-skip-permissions`.

## Merge Anti-Patterns (Runner PM)

MQ-1. **Merges must execute.** `MergeBranch` runs `git merge`, not auto-succeed.

MQ-2. **Conflicts are detected.** Merge conflicts are reported and re-queued, not silently corrupted.

MQ-3. **Warm pool reduces latency.** Pre-spawn the next agent during gate execution.

## Demo-Serve Wiring Anti-Patterns (Runner PN)

DS-1. **Missing endpoints return 404, not 500.** If the demo app calls `/api/dream/journal` and the route doesn't exist, the server must return a clear 404 with the route name, not an opaque 500.

DS-2. **SSE events are real data.** Cost, token, and status events published to StateHub must come from actual dispatch outcomes. Never fabricate demo data for SSE.

DS-3. **Path consistency.** If the demo app calls `POST /api/bench/runs`, the server must mount exactly that path — not `/api/bench/run` (singular).

## Observability Anti-Patterns (Runner PO)

OB-1. **Events emit actuals.** `CostEvent.cost_usd` comes from the provider response, not estimated. `TokenUsageEvent` comes from response metadata, not prompt length.

OB-2. **No event on success-path noise.** Don't emit `ModelFallbackEvent` when the primary model succeeds. Only on actual fallback.

OB-3. **Unknown is None, not zero.** If token counts aren't available from the provider, emit `None` — not `0`.

## CLI End-to-End Anti-Patterns (Runner PP)

CE-1. **Provider from response, not heuristic.** `infer_provider` by string matching is wrong. Use the actual provider that dispatched the request.

CE-2. **Session ID must flow.** The session_id returned by Claude on turn 1 must be passed back as `--resume <id>` on turn 2. Storing it and not sending it is a multi-turn conversation bug.

CE-3. **CostMeter in all modes.** If `roko chat` shows cost summaries but `roko run` doesn't, users assume `roko run` is free. Wire CostMeter everywhere dispatch happens.

CE-4. **Silent MCP skip = invisible bug.** Users who configure MCP in roko.toml and get no feedback when it's silently dropped will blame the MCP servers, not the dispatch path.

## Code Intelligence Anti-Patterns (Runner PQ)

CI-1. **Deduplication before enhancement.** Don't improve `code_context_for_task` in one file while an identical copy exists in another. Consolidate first.

CI-2. **HDC is not keyword search.** HDC similarity finds semantically related symbols that share no keywords. Using it with keyword-only queries wastes the capability.

CI-3. **Index rebuilds are expensive.** Every `WorkspaceIndex::load()` is a full parse + graph + PageRank. Persist to SQLite when possible.

## Legacy Migration Anti-Patterns (Runner PR)

LM-1. **Verify before migrating.** Some "legacy" paths already use v2 runner. Don't rewrite what's already correct — audit first.

LM-2. **Dashboard events for background operations.** Auto-plan triggers and cloud jobs run asynchronously. Without dashboard events, users can't tell if they're running.

## Provider Dispatch Anti-Patterns (Runner PS)

PD-1. **One dispatch entry point per process.** Four serve routes each calling `runtime.run_once()` independently means four places to fix when dispatch logic changes.

PD-2. **Provider health keys are specific.** Recording health against `"default"` means all providers share one health counter. A rate-limited Anthropic API hides a healthy OpenAI.

PD-3. **Error classification drives retry.** Rate limits, auth failures, and network timeouts are different. Treating them identically wastes retries on permanent failures.

## ACP Learning Anti-Patterns (Runner PT)

AL-1. **ACP tasks are episodes too.** Every agent dispatch that spends tokens should produce an episode. ACP running invisibly means the learning layer is blind to its work.

AL-2. **Unknown tokens are None, not zero.** Claude CLI `--print` doesn't return token counts. Record `None`, not `0`. Zero means "zero tokens used" which is false.

AL-3. **CascadeRouter needs all sources.** If the router only sees v2 runner outcomes but not ACP outcomes, it optimizes for one execution path and ignores the other.

## Config Hot-Reload Anti-Patterns (Runner PU)

CR-1. **Polling is fallback, not primary.** `notify::RecommendedWatcher` provides sub-second detection. 2-second polling is acceptable as fallback on unsupported platforms, not as default.

CR-2. **Hot-reload sections are documented.** Users must know which config changes take effect immediately vs require restart. Undocumented hot-reload = surprise behavior changes.

## Runner Data Quality Anti-Patterns (Runner PV)

DQ-1. **Feedback carries actuals.** `AgentOutcome` with empty model, zero tokens, zero cost is invisible to the learning layer. Fill from `RunState`.

DQ-2. **Timestamps are epochs.** `started_at_ms` must be a Unix timestamp (e.g., 1745000000000), not elapsed duration since process start (e.g., 45000).

DQ-3. **Normalization must spread.** If `$1 reference / 300s reference` clamps everything to 1.0, the router's multi-objective reward is useless. Use task-tier-aware references.

DQ-4. **Routing context from tasks.** `RoutingContext.task_category = Implementation` for every task means the router can't distinguish research from testing. Read from `TaskDef`.

## Demo Bench + Dashboard Anti-Patterns (Runner PW)

DB-1. **Endpoint shapes match client.** If `CostRace.tsx` expects `{ models: [...] }`, the server returns exactly that shape. Don't invent a different schema.

DB-2. **Empty data is valid data.** Zero bench runs → `{ models: [] }`, not 500. Zero episodes → `{ composite: { overall: 0.0, episode_count: 0 } }`.

DB-3. **SSE events are typed.** Bench SSE must send `BenchTaskStarted`, `BenchTaskCompleted`, `BenchProgress`, `BenchRunCompleted` — exactly what `useBenchSSE.ts` handles.

## Converge Critical Bug Anti-Patterns (Runner PX)

CB-1. **One trait, one definition.** Duplicate `AffectPolicy` traits with incompatible signatures = nothing can implement both. Delete one.

CB-2. **Computed values must apply.** `DispatchModulation` computed and discarded means the affect engine is decoration. Wire `tier_bias` into model selection.

CB-3. **Lock then check, not check then lock.** TOCTOU in `flush_async` means buffer length may change between check and lock acquisition.

## Workflow Convergence Anti-Patterns (Runner PY)

WC-1. **All agent work produces episodes.** `agent_exec` bypasses episode recording. Route through `WorkflowEngine` so PRD/research/plan agent work is tracked.

WC-2. **Preserve behavior, change dispatch.** Migrating from `agent_exec` to `WorkflowEngine` must not change agent output format or quality — only the dispatch path.

## Agent Process Safety Anti-Patterns (Runner PZ)

AP-1. **Persist before return.** PID must be on disk before the spawn function returns. Any window between spawn and persist = orphan risk.

AP-2. **Fail-closed contracts.** Missing contract YAML → restricted fallback, never permissive. A typo in the role name must not grant full access.

AP-3. **Default is safe.** `dangerously_skip_permissions` defaults to `false`. Any code that changes this without explicit user opt-in is a security hole.

## Demo Workflow + Terminal Anti-Patterns (Runner PAA)

DW-1. **PTY lifecycle matches WebSocket.** PTY spawns on WS connect, killed on WS disconnect. No orphan shells.

DW-2. **Workflow projection sends real state.** SSE `state` event is the current snapshot, `delta` events are incremental updates. Don't send delta without initial state.

## Runner Config Threading Anti-Patterns (Runner PAB)

RC-1. **Config drives behavior.** Hardcoded values in the event loop that should come from `roko.toml` are invisible to operators. Thread from `RunConfig`.

RC-2. **Plugin hooks execute manifests.** Logging "hook called" without executing the declared tool profiles or triggers makes the plugin system decorative.

## Phase 0: Stability & Fixes Anti-Patterns (Runners S0*)

SF-1. **Unreachable is a lie.** `unreachable!()` in a match arm that can be reached = runtime panic. Replace with handler or error.

SF-2. **Config must load what init writes.** If `roko init` writes `[[gate]]` but runtime reads `[gates]`, the user must manually edit config. Both formats accepted.

SF-3. **Default model is a contract.** `default_model = "x"` in roko.toml must be respected by ALL dispatch paths, not just some.

SF-4. **No env::set_var in library code.** `unsafe { set_var() }` is UB in multithreaded programs. Thread overrides through config structs.

SF-5. **ServiceFactory is the single entry point.** All dispatch goes through `ServiceFactory::build()`. Direct `create_agent_for_model()` calls bypass routing, learning, and safety.

## Phase 1: Architecture Convergence Anti-Patterns (Runners O1*, D1*, G1*, CD*)

AC-1. **WorkflowEngine is the one runtime.** Runner v2, orchestrate.rs, and WorkflowEngine must converge. New features go into WorkflowEngine.

AC-2. **CascadeRouter observations flow bidirectionally.** Load from disk on startup, record observations on every dispatch, persist on shutdown. Any gap = lost learning.

AC-3. **Episodes from every entry point.** `roko run`, `roko chat`, `roko plan run`, ACP pipeline — all emit episodes. Missing any makes the learning layer blind.

AC-4. **Gate feedback is typed.** `GateReport` carries failure classification and feedback. Raw pass/fail is insufficient for learning.

AC-5. **Dead code is dead.** orchestrate.rs behind `legacy-orchestrate` feature flag means NO new code goes in. Feature extractions go to proper crates.

## Phase 2-3: UX, Prompt, Learning, Innovation Anti-Patterns (Runners QA*, LF*, GE*, UX*, RF*, IN*)

PQ-1. **ContextTier drives budget, not guessing.** Surgical=4K, Focused=12K, Full=24K. Model determines tier, tier determines budget.

PQ-2. **BudgetPredictor learns.** EMA over observed token usage per role/complexity. Raw defaults only on first run.

PQ-3. **VCG auction is welfare-maximizing.** Only activate when budget is tight (<80% of full). Normal budgets use proportional allocation.

LR-1. **FeedbackService in every path.** Chat, run, plan run, ACP — all wire FeedbackService. Missing any = invisible work.

LR-2. **Anomaly detection is not blocking.** AnomalyDetector warns but does not block dispatch. Only BudgetGuardrail blocks.

LR-3. **ConductorBandit replaces hardcoded retry.** The bandit selects retry/escalate/skip. Hardcoded `max_retries = 3` wastes budget on hopeless tasks.

GV-1. **roko-eval wraps, doesn't replace.** BridgeGateService wraps legacy GateService during migration. Zero regression during transition.

GV-2. **Evidence before evaluation.** Criteria require specific EvidenceKind. Running a criterion without its evidence = skip, not fail.

## Phase 4: GTM, Safety, Observability Anti-Patterns (Runners SF*, OB*, GT*, AC*, RP*, XC*, TV*)

GT-1. **Adapters are lazy.** Only instantiated when their trigger fires. Startup with 10 adapters must not call 10 APIs.

GT-2. **SubAdapter traits are opt-in.** An adapter implements only the sub-traits it needs. Forcing all = bloat.

OB-1. **Tracing spans are structured.** `gen_ai.*` semantic conventions for model calls. Random span names are useless.

OB-2. **Anomaly detectors are configurable.** Thresholds in roko.toml, not hardcoded. Different projects have different baselines.

XC-1. **Structured errors, not anyhow.** Public APIs return typed errors with context chains. `anyhow::Error` is for internal use only.

XC-2. **CancelToken propagates.** Long-running operations check cancellation. Ctrl-C should stop agents within 5 seconds.

TV-1. **Integration tests are deterministic.** No network calls, no real LLM dispatch. Mock at the transport boundary.
