# 16 - Config & Wiring: Implementation Plan

> Fixes for config field load paths (loaded but discarded), roko.toml schema completion,
> direct env var reads for API keys, ServiceFactory completeness, "built but never
> connected" components, feature flag cleanup, default config validation, config migration,
> and secret management improvements.
>
> Source audits: `05-CURRENT-STATE-AND-GAPS.md` (Section 9, Config Field Load Paths),
> `11-CURRENT-STATE-GROUND-TRUTH.md`, `03-PROVIDER-AND-AGENT-AUDIT.md` (Sections 10-12),
> `19-DISPATCH-ISSUES.md` (ISS-01, ISS-02, ISS-07), `18-LEARN-ISSUES.md` (I-01 through I-05).

---

## Section A: Config Field Load Path Fixes (Fields Loaded But Discarded)

These tasks fix fields that are parsed from `roko.toml`, loaded into `RokoConfig`, and
then ignored by the dispatch or runtime paths that should consume them.

### T01: Make `auth_detect.rs` Respect `roko.toml` Provider Config

**Files:**
- `crates/roko-cli/src/auth_detect.rs`
- `crates/roko-cli/src/model_selection.rs`

**Problem:** `detect_auth()` scans env vars (`ZAI_API_KEY`, `ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`) in a fixed priority order, ignoring the `[providers]` and
`[agent].default_model` from `roko.toml` entirely. Setting `default_model = "cerebras-70b"`
in config has no effect when `ANTHROPIC_API_KEY` is in the environment.

**Fix:**
1. Add a `detect_auth_with_config(config: &RokoConfig) -> AuthMethod` variant that
   resolves from the config's `default_model` and `default_backend` first.
2. Only fall back to env var scanning when config has no explicit provider/model.
3. Update all callers of `detect_auth()` to pass the loaded config when available.
4. Keep the zero-arg `detect_auth()` as the bootstrapping path for when no config exists.

**Acceptance:** Setting `default_model = "cerebras-70b"` in roko.toml and running
`roko run "hello"` dispatches to Cerebras, not Claude.

**Depends on:** None
**Estimated effort:** Medium

---

### T02: Wire `[[gate]]` Array Parsing Into `RokoConfig::from_toml()`

**Files:**
- `crates/roko-core/src/config/schema.rs` (or wherever `RokoConfig::from_toml` lives)
- `crates/roko-cli/src/config.rs`

**Problem:** `roko init` generates `[[gate]]` arrays (TOML array-of-tables syntax).
`RokoConfig::from_toml()` silently discards them because it only reads the `[gates]`
table format. Users who run `roko init` followed by `roko plan run` get no gates.

**Fix:**
1. In `RokoConfig::from_toml()`, accept both `[[gate]]` (array of gate objects) and
   `[gates]` (table with `enabled` list).
2. Normalize both to the internal `Vec<GateConfig>` representation at parse time.
3. If both formats are present, merge them (array entries first, then table entries).
4. Log a deprecation warning when `[[gate]]` format is encountered, guiding users to
   the canonical `[gates]` format.

**Acceptance:** `roko init` generates gates; `roko plan run` uses those gates without
manual config editing. `roko config show` displays the resolved gate list regardless
of source format.

**Depends on:** None
**Estimated effort:** Medium

---

### T03: Thread `workflow.template` Config Into Runner V2

**Files:**
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

**Problem:** `[workflow].template` (express/standard/full) is read by `WorkflowEngine`
but runner v2 (`roko plan run`) uses its own hardcoded config, ignoring the template
entirely. `[workflow].max_iterations` is also ignored by runner v2 (which uses its own
`max_retries` field).

**Fix:**
1. Read `workflow.template` from `RokoConfig` in the plan runner's initialization.
2. Map template to concrete settings: express (no review, 1 iteration), standard
   (optional review, 3 iterations), full (review + judge gate, 5 iterations).
3. Expose `workflow.max_iterations` as the cap on retry attempts, falling back to
   the template's default if not set.
4. Remove the hardcoded `max_retries` in runner v2 in favor of the config value.

**Acceptance:** Setting `workflow.template = "express"` in roko.toml reduces plan
runner iteration count. Setting `max_iterations = 5` overrides the template default.

**Depends on:** None
**Estimated effort:** Small

---

### T04: Wire `learning.replan_on_gate_failure` Into Runner V2

**Files:**
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/config.rs`

**Problem:** `learning.replan_on_gate_failure` is loaded from roko.toml and consumed
by orchestrate.rs (dead code). Runner v2 never reads this flag -- gate failures exhaust
the autofix budget and mark the task as failed without triggering replanning.

**Fix:**
1. Read `learning.replan_on_gate_failure` in the plan runner's event loop config.
2. After autofix budget is exhausted, if the flag is true, call
   `build_gate_failure_plan_revision()` (extract from orchestrate.rs if needed).
3. The revision spawns a strategist agent with gate error context to produce a
   revised approach, which is then retried.
4. Cap replan attempts at 1 per task to prevent infinite loops.

**Acceptance:** With `replan_on_gate_failure = true`, a task that fails all autofix
attempts spawns a strategist before giving up. The strategist's output is visible
in the episode log.

**Depends on:** None
**Estimated effort:** Medium

---

### T05: Wire `budget.max_cost_per_run` Into Runner V2 and WorkflowEngine

**Files:**
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-agent/src/model_call_service.rs`

**Problem:** `budget.max_cost_per_run` is read by orchestrate.rs only. Runner v2
and WorkflowEngine create `ModelCallService` without a cost budget -- `BudgetCell::new(None)`.
Runaway agents can spend unlimited API credits.

**Fix:**
1. Read `budget.max_cost_per_run` from `RokoConfig` in `ServiceFactory::build()`.
2. Pass it to `ModelCallService::with_cost_budget()` during construction.
3. When budget is exceeded, the ModelCallService returns a clear error rather than
   silently dispatching.
4. Add `budget.max_cost_per_task` and `budget.max_cost_per_session` to the schema
   with sensible defaults ($0.50/task, $10/session, $50/plan run).

**Acceptance:** Setting `max_cost_per_run = 5.0` in roko.toml causes `roko plan run`
to stop after $5 of API spend with a budget-exceeded message.

**Depends on:** None
**Estimated effort:** Small

---

### T06: Wire `agent.tier_models` Into Live Dispatch

**Files:**
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-learn/src/cascade_router.rs`

**Problem:** `agent.tier_models` maps task tiers (T0-T3) to model slugs. The mapping
is loaded into `CascadeRouter` via `model_slugs_for_config()` but the tier routing
path is never called at dispatch time. All tasks use the default model regardless
of their declared tier.

**Fix:**
1. In `resolve_effective_model()`, add a `task_tier` parameter.
2. Before cascade routing, check `tier_models` for the task's tier. If a tier-specific
   model is configured, use it (with source `SelectionSource::TierConfig`).
3. Thread `task_tier` through `DispatchContext` in both runner v2 and WorkflowEngine.
4. Document the tier model config in the schema comments.

**Acceptance:** Setting `[agent.tier_models]` with `T3 = "claude-opus-4-6"` causes
T3 tasks to dispatch to Opus, while T0 tasks use the default model.

**Depends on:** None
**Estimated effort:** Medium

---

## Section B: Environment Variable Elimination

Direct `std::env::var()` reads for API keys bypass the provider config system, credential
management, cost tracking, and rotation. These tasks replace them with config-driven
dependency injection.

### T07: Replace Direct `ANTHROPIC_API_KEY` Read in `episode_completion.rs`

**Files:**
- `crates/roko-neuro/src/episode_completion.rs`

**Problem:** `episode_completion.rs` reads `ANTHROPIC_API_KEY` directly from the
environment to construct its own HTTP client for neuro distillation calls. This
bypasses the provider system, credential rotation, and cost tracking.

**Fix:**
1. Add a `model_caller: Arc<dyn ModelCaller>` parameter to the distillation entry
   point (or accept a configured `ModelCallService`).
2. Remove the direct `std::env::var("ANTHROPIC_API_KEY")` read.
3. The caller (currently `KnowledgeStore` or `LearningRuntime`) passes the
   `ModelCallService` it already has access to.
4. If no model caller is available, skip distillation with a logged warning rather
   than silently doing nothing.

**Acceptance:** `episode_completion.rs` has zero `std::env::var` calls. Distillation
cost appears in the learning cost records.

**Depends on:** None
**Estimated effort:** Small

---

### T08: Replace Direct `PERPLEXITY_API_KEY` Read in `web_search.rs`

**Files:**
- `crates/roko-std/src/tool/builtin/web_search.rs`

**Problem:** The web search builtin tool reads `PERPLEXITY_API_KEY` directly from the
environment. This is inconsistent with the provider system's `ProviderConfig.api_key_env`
pattern and prevents cost tracking of search calls.

**Fix:**
1. Add a `search_api_key: Option<String>` field to the tool's context or config,
   populated from `providers.perplexity.api_key_env` at tool registry construction time.
2. Fall back to `std::env::var("PERPLEXITY_API_KEY")` only when no config is available
   (e.g., in tests or standalone tool usage).
3. Log a deprecation warning when using the env var fallback.

**Acceptance:** Web search works when `PERPLEXITY_API_KEY` is configured in the
provider config but not in the environment. The env var fallback still works but
logs a warning.

**Depends on:** None
**Estimated effort:** Small

---

### T09: Replace Direct `PERPLEXITY_API_KEY` Reads in `orchestrate.rs` Research Paths

**Files:**
- `crates/roko-cli/src/orchestrate.rs` (lines ~4473, 4692, 4904, 17368)
- `crates/roko-cli/src/commands/research.rs` (line ~724)

**Problem:** Four locations in orchestrate.rs and one in the research command read
`PERPLEXITY_API_KEY` directly to construct search clients. These should use the
configured provider.

**Fix:**
1. In `commands/research.rs`, resolve the Perplexity API key from the provider
   config (`providers.perplexity.api_key_env` -> env var lookup).
2. In orchestrate.rs (behind feature flag), apply the same pattern. Since this is
   legacy code being phased out, a minimal fix (resolve from config, fall back to
   env) is sufficient.
3. Add a helper `resolve_provider_api_key(config: &RokoConfig, provider_name: &str) -> Option<String>`
   that checks config first, then env var.

**Acceptance:** `roko research search "test"` works when the Perplexity key is
configured only in roko.toml's provider config, not in the environment.

**Depends on:** None
**Estimated effort:** Small

---

### T10: Remove `unsafe { std::env::set_var() }` for Provider Override

**Files:**
- `crates/roko-cli/src/commands/util.rs` (or wherever `--provider` flag is handled)

**Problem:** The `--provider` CLI flag uses `unsafe { std::env::set_var() }` to inject
the override. This is unsound in multi-threaded contexts (Rust 2024 edition marks it
`unsafe` for this reason) and is the wrong mechanism for config propagation.

**Fix:**
1. Thread the provider override through the `ServiceConfig` struct (add an
   `override_provider: Option<String>` field).
2. In `ServiceFactory::build()`, apply the override before model resolution.
3. Remove all `set_var` calls for provider/model overrides.
4. Also check for similar `set_var` patterns for other config overrides and replace them.

**Acceptance:** `roko run --provider anthropic "hello"` selects the Anthropic provider
without calling `set_var`. Running under `RUST_FLAGS=-Dunsafe_code` does not trigger
warnings in the CLI dispatch path.

**Depends on:** None
**Estimated effort:** Small

---

## Section C: ServiceFactory Completeness

The `ServiceFactory` is the canonical construction path for workflow services. These
tasks ensure all entry points use it and that it constructs complete service bundles.

### T11: Route All CLI Entry Points Through `ServiceFactory::build()`

**Files:**
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/chat_inline.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/dispatch_v2.rs`

**Problem:** Nine dispatch paths exist with inconsistent model selection. Only paths
through `ServiceFactory` get full feedback recording, cost tracking, cascade routing,
and knowledge injection. Paths 2-5 from the audit have hardcoded model strings
("claude-sonnet-4-6", "gpt-4o", "llama3.1:8b").

**Fix:**
1. In `run.rs`, replace direct `create_agent_for_model()` calls with `ServiceFactory::build()`.
   Remove the hardcoded "claude-sonnet-4-6" fallback.
2. In `chat_session.rs`, construct `ModelCallService` via `ServiceFactory::build()` for
   the chat session's model caller. This also sets up feedback recording (fixing I-01).
3. In `prd.rs` and `dispatch_v2.rs`, ensure agent dispatch goes through
   `ServiceBundle.model_call_service`.
4. Remove hardcoded model strings. Replace with `config.agent.default_model` resolution.
5. Eliminate the standalone `dispatch_direct.rs` path (already behind `legacy-orchestrate`
   feature gate -- this task verifies it is unreachable from default builds).

**Acceptance:** `grep -rn '"claude-sonnet-4-6"' crates/roko-cli/src/ --include='*.rs'`
returns zero hits outside of test code. All dispatch paths log the model source via
`EffectiveModelSelection`.

**Depends on:** T01 (auth_detect config awareness)
**Estimated effort:** Large

---

### T12: Wire CascadeRouter Into All Live Callers

**Files:**
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

**Problem:** ISS-01: CascadeRouter has zero live callers. `resolve_effective_model()`
accepts `Option<&CascadeRouter>` but every caller passes `None`. The router is loaded
in `ServiceFactory::build()` and wired to `ModelCallService` -- but standalone callers
of `resolve_effective_model()` (e.g., quick model resolution in CLI commands) skip it.

**Fix:**
1. In `ServiceFactory::build()`, verify the CascadeRouter is passed to `ModelCallService`
   (already done -- confirm it flows through to actual dispatch).
2. In `resolve_effective_model_key()`, load the CascadeRouter from disk if a `.roko`
   directory exists, rather than hardcoding `None`.
3. After each dispatch through `ModelCallService`, verify that
   `cascade_router.observe()` is called (via the feedback sink chain).
4. Add a startup log line showing CascadeRouter state: observation count, stage
   (static/confidence/UCB1), number of candidate models.

**Acceptance:** After running 5 tasks, `.roko/learn/cascade-router.json` has
`observations > 0`. `roko learn router` shows the observation count increasing.

**Depends on:** T11 (all paths through ServiceFactory)
**Estimated effort:** Medium

---

### T13: Add FeedbackService to `roko chat` Path

**Files:**
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/chat_inline.rs`

**Problem:** I-01: `roko chat` records zero learning signals. No episodes, no routing
observations, no cost tracking. Chat is likely the most-used interactive entry point.

**Fix:**
1. If T11 routes chat through `ServiceFactory::build()`, this is largely solved
   (the `FeedbackService` is constructed automatically).
2. Additionally, emit `FeedbackEvent::WorkflowComplete` when the chat session ends
   (on `/quit` or Ctrl-D) so the session's total cost is recorded.
3. Emit per-turn episodes for cost tracking (even if chat turns don't have gate
   verdicts, the model/tokens/cost data is valuable).

**Acceptance:** After a 3-turn chat session, `.roko/learn/cascade-router.json` has
3 new observations. `.roko/learn/costs.jsonl` has cost records for the session.

**Depends on:** T11 (ServiceFactory routing)
**Estimated effort:** Small

---

## Section D: "Built But Never Connected" Components

Nine components identified in the audits that are built, compile, may have tests,
but are never called from any live CLI or HTTP path.

### T14: Wire `ConfigCmd::Mcp` Dispatch (HOLLOW 1)

**Files:**
- `crates/roko-cli/src/commands/config_cmd.rs`

**Problem:** `ConfigCmd::Mcp` arm falls through to `unreachable!()`, causing a panic
when `roko config mcp list` is invoked. The MCP config is already in `roko.toml` and
parseable -- just needs a handler.

**Fix:**
1. Replace the `unreachable!()` with a handler that reads `config.agent.mcp_config`
   and `config.agent.mcp_servers`.
2. For `mcp list`: display configured MCP servers with name, transport, and status.
3. For `mcp add <name> <command>`: append to the MCP config section.
4. For `mcp remove <name>`: remove from config.
5. For `mcp test <name>`: attempt connection and report result.

**Acceptance:** `roko config mcp list` shows configured MCP servers without panicking.
`roko config mcp add test-server npx test-mcp` adds an entry.

**Depends on:** None
**Estimated effort:** Small

---

### T15: Wire BudgetGuardrail Into Live Paths

**Files:**
- `crates/roko-learn/src/budget.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

**Problem:** I-05: `BudgetGuardrail` implements 3-scope budget limits with 5 graduated
actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It is never instantiated
in any live path.

**Fix:**
1. Load budget config from `roko.toml` `[budget]` section.
2. Instantiate `BudgetGuardrail` at the start of `roko run`, `roko plan run`, and
   `roko chat`.
3. Before each model dispatch, call `guardrail.check()`.
4. On `Warn`: log warning with current spend.
5. On `RouteToCheaper`: override model selection to cheapest available.
6. On `Block`: return error with clear message showing budget vs actual spend.
7. Wire cumulative cost from `CostsDb` so per-day budgets work across sessions.

**Acceptance:** Setting `budget.max_task_usd = 0.01` in roko.toml causes a large
task to hit the budget warning. Setting `budget.max_cost_per_run = 0.001` blocks
dispatch with a clear error message.

**Depends on:** T05 (budget config in ServiceFactory)
**Estimated effort:** Medium

---

### T16: Wire AnomalyDetector Into Live Paths

**Files:**
- `crates/roko-learn/src/anomaly.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

**Problem:** I-14: The anomaly detector (prompt loops, cost spikes, quality degradation)
is session-local and lightweight. It is never instantiated.

**Fix:**
1. Create `AnomalyDetector` at session start in `roko run` and plan runner.
2. Before each dispatch, call `detector.check_prompt(prompt_hash)` to detect
   prompt loops.
3. After each response, call `detector.check_cost(cost_usd)` to detect cost spikes.
4. On anomaly: log at WARN level, optionally trigger a conductor abort (when
   conductor is also wired -- for now, just log).
5. Report detected anomalies in the episode record's `anomalies` field.

**Acceptance:** Dispatching the same prompt 5 times triggers a prompt-loop warning
in logs. A response costing 10x the session average triggers a cost-spike warning.

**Depends on:** None
**Estimated effort:** Small

---

### T17: Wire ConductorBandit Into Plan Runner Retry Loop

**Files:**
- `crates/roko-learn/src/conductor.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

**Problem:** I-06: The conductor bandit (7 actions, 19-dim context) decides whether a
failing task should continue, receive a hint, escalate, restart, or abort. It is never
invoked. All retry decisions are hardcoded.

**Fix:**
1. Load `ConductorBandit` state from `.roko/learn/conductor.json` at plan runner start.
2. On task failure, call `bandit.select_action(context)` instead of the hardcoded
   retry logic.
3. Map conductor actions to plan runner behaviors: Continue -> retry with same model,
   Hint -> inject failure context, Escalate -> switch to stronger model, Restart ->
   clear state and retry, Abort -> mark task failed.
4. Feed reward after retry outcome.
5. Save state after each observation.

**Acceptance:** After 20+ task completions with mixed success, the conductor's action
distribution is non-uniform (it has learned from outcomes). `.roko/learn/conductor.json`
has observations.

**Depends on:** None
**Estimated effort:** Medium

---

### T18: Wire Streaming Events to TUI in `chat_inline.rs` (AP-7)

**Files:**
- `crates/roko-cli/src/chat_inline.rs`

**Problem:** AP-7: The chat inline handler creates a streaming channel, spawns the
agent, then drains events with `while let Some(_event) = event_rx.recv().await {}`.
Every streaming event is discarded. The TUI shows a spinner until the entire response
is complete.

**Fix:**
1. Replace the `_event` discard with actual event mapping to `DashboardEvent` or
   direct TUI rendering.
2. For `ClaudeStreamEvent::Assistant` events, render text tokens incrementally in
   the viewport.
3. For `ClaudeStreamEvent::Tool` events, show tool call name and a progress indicator.
4. For `ClaudeStreamEvent::Result` events, finalize the response display.

**Acceptance:** Running `roko chat` and sending a message shows tokens appearing
incrementally, not all-at-once after completion.

**Depends on:** None
**Estimated effort:** Medium

---

### T19: Wire `build_repo_context()` Into Plan Generation (AP-8)

**Files:**
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/repo_context.rs`

**Problem:** AP-8: `build_repo_context()` gives agents awareness of the repository
structure. It is called from `prd draft new` but NOT from `plan generate`,
`plan regenerate`, or `prd plan`. Generated plans propose greenfield crates that
duplicate existing functionality.

**Fix:**
1. Call `build_repo_context()` before agent dispatch in `plan generate`,
   `plan regenerate`, and `prd plan` handlers.
2. Include the repo context as a system prompt section (or user context section).
3. For `plan regenerate`, also inject the validation errors from the previous attempt
   (fixing HOLLOW 3 -- plan regenerate diagnostics).

**Acceptance:** Running `roko plan generate` on a workspace with 18 crates produces
a plan that references existing crates rather than proposing new ones.

**Depends on:** None
**Estimated effort:** Small

---

## Section E: roko.toml Schema Completion and Validation

### T20: Add Config Validation on Load

**Files:**
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/config.rs`

**Problem:** `RokoConfig::from_toml()` accepts any syntactically valid TOML without
checking semantic validity. Invalid model references (`default_model = "nonexistent"`),
missing provider keys (`provider = "missing"`), and contradictory settings
(`gates.skip_tests = true` + `gates.enabled = ["test"]`) are silently accepted.

**Fix:**
1. Add `RokoConfig::validate(&self) -> Vec<ConfigWarning>` that checks:
   - `default_model` resolves to a model in the `[models]` table (or is a known alias).
   - Every model's `provider` field references a key in `[providers]`.
   - Provider `api_key_env` values have corresponding env vars set (warn if not).
   - Gate configuration is internally consistent.
   - Budget values are non-negative.
   - `tier_models` values are valid model keys.
2. Call `validate()` on load and print warnings to stderr.
3. `roko config validate` runs validation and returns a structured report.

**Acceptance:** Setting `default_model = "nonexistent"` in roko.toml produces a
warning on startup: `warning: default_model "nonexistent" not found in [models] table`.

**Depends on:** None
**Estimated effort:** Medium

---

### T21: Normalize Model Aliases at Load Time

**Files:**
- `crates/roko-core/src/agent.rs` (`resolve_model()`)
- `crates/roko-orchestrator/src/service_factory.rs`

**Problem:** Duplicate model entries exist: `glm-5-1` on provider "zai" vs `glm51` on
provider "zhipu" both resolve to `glm-5.1`. Multiple Claude aliases resolve to the same
model ID. This causes CascadeRouter to treat them as separate models, fragmenting
observations.

**Fix:**
1. Add a `normalize_model_slug(slug: &str) -> String` function that canonicalizes:
   - `glm-5-1` / `glm51` / `glm-5.1` -> `glm-5.1`
   - `claude-sonnet-4-6` / `claude-sonnet-4-6-20250514` -> canonical form
   - Other known aliases from the model registry.
2. Call `normalize_model_slug()` in `resolve_model()` before returning.
3. Call it in `CascadeRouter` before recording observations or selecting models.
4. Add a deduplicated view in `roko config models list` that groups aliases.

**Acceptance:** Running 3 tasks with `glm-5-1` and 3 with `glm51` produces 6
observations against the same canonical model in the cascade router, not two
separate entries with 3 each.

**Depends on:** None
**Estimated effort:** Small

---

### T22: Complete `[budget]` Schema With Sensible Defaults

**Files:**
- `crates/roko-cli/src/config.rs` (`BudgetConfig`)
- `crates/roko-core/src/config/schema.rs`

**Problem:** The `BudgetConfig` struct exists but may be missing fields that the
`BudgetGuardrail` needs. The defaults may be None/zero, providing no safety net.

**Fix:**
1. Ensure the following fields exist with defaults in `BudgetConfig`:
   - `max_cost_per_task: Option<f64>` (default: None -- no per-task limit)
   - `max_cost_per_session: Option<f64>` (default: None)
   - `max_cost_per_run: Option<f64>` (default: None)
   - `max_cost_per_day: Option<f64>` (default: None)
   - `warn_threshold: f64` (default: 0.8 -- warn at 80% of budget)
   - `route_to_cheaper_threshold: f64` (default: 0.9 -- switch model at 90%)
   - `prompt_token_budget: u32` (default: 0 -- unlimited prompt tokens)
2. Document each field with comments in the generated roko.toml.
3. `roko init` should include a commented-out `[budget]` section showing the fields.

**Acceptance:** `roko config show` displays the budget section with defaults.
`roko init` generates a `# [budget]` section with documented fields.

**Depends on:** None
**Estimated effort:** Small

---

## Section F: Feature Flag Cleanup

### T23: Audit and Document `legacy-orchestrate` Feature Flag

**Files:**
- `crates/roko-cli/Cargo.toml`
- `crates/roko-cli/src/lib.rs`
- All files with `#[cfg(feature = "legacy-orchestrate")]`

**Problem:** The `legacy-orchestrate` feature flag gates ~20+ code blocks across
`auth_detect.rs`, `dispatch_direct.rs`, `chat_inline.rs`, `lib.rs`, `unified.rs`,
and `run.rs`. It is unclear whether the flag is on or off by default, what it
enables/disables, and what the migration path is.

**Fix:**
1. Verify the feature flag state: check `Cargo.toml` for `default = ["legacy-orchestrate"]`
   or absence. Document the current default.
2. Add a `// DEPRECATED: legacy-orchestrate` comment block at the top of every
   `#[cfg(feature = "legacy-orchestrate")]` section explaining what it guards.
3. Create a tracking list of all gated code blocks (file, line, what it does).
4. For code that is purely dead (e.g., the `PlanRunner` struct in orchestrate.rs):
   mark with `#[deprecated]` annotations.
5. For code that is the only implementation of a needed feature (e.g., CLI detection
   in auth_detect.rs): plan extraction to an ungated module.

**Acceptance:** `grep -rn 'legacy-orchestrate' crates/roko-cli/src/` returns results
that all have adjacent documentation comments. A tracking list exists in GAPS.md.

**Depends on:** None
**Estimated effort:** Small

---

### T24: Remove or Migrate `ROKO_ACP_LEGACY` Environment Variable Gate

**Files:**
- `crates/roko-acp/src/bridge_events.rs`
- `crates/roko-acp/src/pipeline.rs`

**Problem:** AP-5: File changes, phase badges, narrative text, and forensic analysis
in the ACP pipeline require `ROKO_ACP_LEGACY` to be set. Without it, these features
are compiled but gated behind env reads, producing less informative output with no
indication that features are being suppressed.

**Fix:**
1. Replace the `ROKO_ACP_LEGACY` env var check with a config field:
   `[acp].legacy_features = true` (default true, enabling all features).
2. Remove the `std::env::var("ROKO_ACP_LEGACY")` reads.
3. If the features were gated for a good reason (e.g., performance), use a proper
   config toggle with documentation.
4. If the features should always be on, remove the gate entirely.

**Acceptance:** ACP sessions produce file change reports and phase badges without
setting any environment variable. No `ROKO_ACP_LEGACY` env reads remain in the codebase.

**Depends on:** None
**Estimated effort:** Small

---

### T25: Evaluate `hdc` Feature Flag for `roko-neuro` Default

**Files:**
- `crates/roko-neuro/Cargo.toml`

**Problem:** I-15: Anti-knowledge gating and HDC-based similarity scoring in
KnowledgeStore require the `hdc` feature flag. Without it, several quality-control
mechanisms are inactive. Default builds may miss anti-knowledge protections.

**Fix:**
1. Check whether `hdc` is in `roko-neuro`'s default features in `Cargo.toml`.
2. If not, add it to `default = ["hdc"]`.
3. If `hdc` has heavy dependencies that increase build time significantly, keep it
   optional but enable it in the workspace `Cargo.toml` for the `roko-cli` binary.
4. Document the feature flag's purpose in the crate-level doc comment.

**Acceptance:** Building `roko-cli` with default features includes HDC fingerprinting
and anti-knowledge gating. `roko knowledge stats` shows HDC-enabled status.

**Depends on:** None
**Estimated effort:** Small

---

## Section G: Config Migration and Secret Management

### T26: Add Config Migration for `[[gate]]` to `[gates]` Format

**Files:**
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`

**Problem:** The `[[gate]]` to `[gates]` format change (T02) creates a breaking change
for existing workspaces. Users with `[[gate]]` arrays in their roko.toml need a
migration path.

**Fix:**
1. Add a `roko config migrate` subcommand that:
   - Reads the current roko.toml.
   - Detects `[[gate]]` array syntax.
   - Rewrites to `[gates]` table format.
   - Preserves all other config sections unchanged.
   - Writes a backup to `roko.toml.bak` before modifying.
2. `RokoConfig::from_toml()` auto-detects old formats and prints a one-time
   migration hint: `hint: run "roko config migrate" to update config format`.
3. Version the config schema: add `config_version = 2` to roko.toml so future
   migrations can detect the format.

**Acceptance:** Running `roko config migrate` on a roko.toml with `[[gate]]` arrays
produces a valid roko.toml with `[gates]` tables. A backup exists at `roko.toml.bak`.

**Depends on:** T02 (both formats accepted)
**Estimated effort:** Medium

---

### T27: Wire Secret Resolution Through Provider Config

**Files:**
- `crates/roko-core/src/config/schema.rs` (ProviderConfig)
- `crates/roko-cli/src/commands/config_cmd.rs`

**Problem:** Each provider has an `api_key_env` field specifying which env var holds the
API key. Some code reads the env var name from config and resolves it. Other code
hardcodes the env var name. The `config secrets` subcommands exist but the resolution
path is inconsistent.

**Fix:**
1. Add `ProviderConfig::resolve_api_key(&self) -> Option<String>` that:
   - Checks `self.api_key` first (inline key, not recommended but supported).
   - Then checks `self.api_key_env` -> `std::env::var(name)`.
   - Then checks the profile-aware secrets store.
2. All provider adapter constructors use `provider_config.resolve_api_key()` instead
   of direct env var reads.
3. `roko config check-secrets` verifies all configured providers have resolvable keys.
4. `roko config providers health` calls `resolve_api_key()` and reports which providers
   have valid credentials.

**Acceptance:** `roko config providers health` shows green/red status for each provider's
credential resolution. A provider with `api_key_env = "CUSTOM_KEY"` resolves correctly
when `CUSTOM_KEY` is set.

**Depends on:** None
**Estimated effort:** Medium

---

### T28: Add Secret Scrubbing to CLI Gist Share Path

**Files:**
- `crates/roko-cli/src/share.rs`

**Problem:** Security LOW finding: `roko run --share` creates a GitHub Gist with the
raw agent transcript. The HTTP share path applies secret scrubbing. The CLI path does
not. API keys and tokens in agent output are uploaded to GitHub unscrubbed.

**Fix:**
1. Apply `scrub_secrets()` (or the equivalent scrubbing function) to the transcript
   before Gist upload.
2. The scrubber should detect: API keys (`sk-ant-*`, `sk-*`), tokens (JWT patterns),
   and any values matching configured `api_key_env` vars.
3. Replace detected secrets with `[REDACTED]`.
4. Add a test: create a transcript with a fake API key, scrub it, verify the key is
   replaced.

**Acceptance:** `roko run --share` with an API key in the agent output produces a
Gist containing `[REDACTED]` instead of the actual key.

**Depends on:** None
**Estimated effort:** Small

---

## Section H: Dual-Path and Anti-Pattern Fixes

### T29: Make `roko init` Emit Config Format the Runtime Reads

**Files:**
- `crates/roko-cli/src/commands/init.rs`

**Problem:** AP-6: `roko init` generates `[[gate]]` arrays but the runtime reads
`[gates]` tables. This is the most basic form of config-runtime disconnect.

**Fix:**
1. Change `append_verification_gates()` in init.rs to emit the `[gates]` table
   format that `RokoConfig::from_toml()` reads.
2. Include `enabled = ["compile", "clippy", "test"]` as the default gate list.
3. Add `shell_gates` section for custom shell commands (commented out with examples).
4. Include inline comments documenting each gate.

**Acceptance:** `roko init` generates a roko.toml. `roko plan run` on a simple plan
uses the gates from that config without manual editing.

**Depends on:** None
**Estimated effort:** Small

---

### T30: Fix Dual Episode Writes in `roko run`

**Files:**
- `crates/roko-cli/src/run.rs`

**Problem:** I-07: `roko run` writes episodes twice -- once via direct
`append_episode_log()` and again via `LearningRuntime::record_completed_run()`.
This produces duplicate records in different files.

**Fix:**
1. Remove the direct `append_episode_log()` call in `run.rs`.
2. Let `LearningRuntime` (via `FeedbackService`) be the single episode writer.
3. Verify that `FeedbackService` writes to the canonical path (`LearningPaths.episodes_jsonl`).
4. If the two paths differ (`.roko/episodes.jsonl` vs `.roko/learn/episodes.jsonl`),
   pick one canonical location and symlink the other for backward compatibility.
5. Update any readers to use the canonical path.

**Acceptance:** Running `roko run "hello"` produces exactly 1 episode record, not 2.
`wc -l .roko/episodes.jsonl` increases by exactly 1 per run.

**Depends on:** T11 (ServiceFactory routing ensures FeedbackService is present)
**Estimated effort:** Small

---

### T31: Consolidate Hardcoded Max-Token Values

**Files:**
- `crates/roko-cli/src/dispatch_direct.rs`
- `crates/roko-agent/src/provider/anthropic_api.rs`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-cli/src/demo_cmd.rs`

**Problem:** Max tokens for the same model vary by entry point: dispatch_direct.rs (8192),
anthropic adapter (4096), gateway (1024), demo (512). This causes inconsistent response
lengths.

**Fix:**
1. Add `max_output_tokens: Option<u32>` to `ModelProfile` in the config schema.
2. `resolve_model()` returns the profile's `max_output_tokens` as part of the resolved
   model info.
3. All dispatch paths use `model_profile.max_output_tokens.unwrap_or(4096)` instead
   of their own hardcoded values.
4. Demo and gateway paths can override to lower values for their specific use cases
   but must do so explicitly via config, not hardcoded constants.

**Acceptance:** Setting `max_output_tokens = 16384` on a model profile in roko.toml
produces longer responses across all dispatch paths.

**Depends on:** None
**Estimated effort:** Small

---

### T32: Move Share Routes Inside Auth Middleware (Security CRITICAL)

**Files:**
- `crates/roko-serve/src/routes/shared_runs.rs`
- `crates/roko-serve/src/lib.rs` (router construction)

**Problem:** Security CRITICAL finding: `POST /api/runs/{id}/share` is mounted OUTSIDE
the auth middleware layer. Any caller can create share links for any run, exposing
agent transcripts that may contain API keys, code, and internal documentation.

**Fix:**
1. Move the share route registration from the public router to the protected router
   (inside the auth middleware layer).
2. Add `GET /runs/{token}` (the public share-viewing route) as the only public route
   for shares -- this is read-only access to already-shared content.
3. Add an integration test: `POST /api/runs/test/share` without auth returns 401.
4. Verify `roko serve` with `auth.enabled = true` blocks unauthenticated share creation.

**Acceptance:** Starting `roko serve` with auth enabled, `curl -X POST localhost:6677/api/runs/test/share`
returns 401. With a valid token, the same request returns 200.

**Depends on:** None
**Estimated effort:** Small

---

### T33: Auto-Provision Auth on Cloud Deploy (Security HIGH)

**Files:**
- `crates/roko-cli/src/commands/util.rs` (or `deploy.rs`)
- `crates/roko-serve/src/lib.rs`

**Problem:** Security HIGH finding: `roko serve` binds to `0.0.0.0:6677` with auth
disabled by default. Cloud deployments (`roko deploy railway`) expose this publicly
with no authentication.

**Fix:**
1. In `roko deploy railway/fly/docker`, auto-generate a random API key if none is
   configured.
2. Set `api_auth.enabled = true` in the deploy config.
3. Print the generated API key to stdout so the user can save it.
4. Set the key as a Railway/Fly secret automatically.
5. In `roko serve`, if binding to `0.0.0.0` and auth is not enabled, print a prominent
   warning with instructions (not just `acknowledge_public_risk` which bypasses without
   fixing).

**Acceptance:** `roko deploy railway` output includes an auto-generated API key.
The deployed server returns 401 for unauthenticated requests.

**Depends on:** None
**Estimated effort:** Medium

---

## Dependency Graph

```
T01 ─────────────────┐
                     ├── T11 ──── T12
T02 ── T26           │           T13
T03                  │
T04                  │
T05 ── T15           │
T06                  │
T07                  │
T08                  │
T09                  │
T10                  │
T14                  │
T16                  │
T17                  │
T18                  │
T19                  │
T20                  │
T21                  │
T22                  │
T23                  │
T24                  │
T25                  │
T27                  │
T28                  │
T29                  │
T30 ─────────────────┘  (soft dep on T11)
T31
T32
T33
```

Most tasks are independent. The critical path is T01 -> T11 -> T12/T13/T30.
T02 -> T26 is a sequential pair. T05 -> T15 is a sequential pair.

## Effort Summary

| Effort | Count | Tasks |
|--------|-------|-------|
| Small  | 17    | T03, T05, T07, T08, T09, T10, T13, T14, T16, T19, T21, T22, T23, T24, T25, T28, T29, T30, T31, T32 |
| Medium | 11    | T01, T02, T04, T06, T12, T15, T17, T18, T20, T26, T27, T33 |
| Large  | 1     | T11 |

Total: 33 tasks. Estimated calendar time: 15-20 days with parallelization.

## Priority Order

**P0 -- Security (do first):**
T32 (share routes auth), T33 (cloud deploy auth), T28 (secret scrubbing)

**P1 -- Config correctness (core functionality):**
T01 (auth_detect config), T02 (gate format), T29 (init format), T05 (budget config),
T11 (ServiceFactory routing), T20 (validation)

**P2 -- Learning wiring (self-improvement):**
T12 (CascadeRouter), T13 (chat feedback), T04 (replan), T15 (budget guardrail),
T17 (conductor), T30 (dual episodes)

**P3 -- UX and polish:**
T03 (workflow template), T06 (tier models), T14 (MCP dispatch), T18 (streaming),
T19 (repo context), T21 (aliases), T16 (anomaly)

**P4 -- Cleanup and migration:**
T07, T08, T09, T10 (env var elimination), T22, T23, T24, T25, T26, T27, T31
