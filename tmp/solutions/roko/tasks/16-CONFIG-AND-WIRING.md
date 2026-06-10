# 16 - Config & Wiring: Tasks

> Config fields loaded but discarded, env var reads that bypass the provider system,
> ServiceFactory bypass paths, built-but-never-connected components, feature flag
> confusion, and config-runtime disconnects.
>
> Sources: `impl/16-CONFIG-AND-WIRING.md`, `05-CURRENT-STATE-AND-GAPS.md` (Section 9),
> `11-CURRENT-STATE-GROUND-TRUTH.md`.

---

## Overview

Roko's config system has two structural problems:

1. **Config fields are parsed then ignored.** `roko.toml` has 10+ fields that are
   loaded into `RokoConfig` but never consumed by the dispatch or runtime paths that
   should use them. Users set `default_model = "cerebras-70b"` and get Claude because
   `auth_detect.rs` scans env vars in a fixed priority and ignores config entirely.

2. **Built components are never called.** Nine+ components compile, may have tests,
   and export public APIs, but are never instantiated from any live CLI or HTTP path.
   `BudgetGuardrail` (in `roko-learn`) implements 3-scope budget limits with 5
   graduated actions -- and is never constructed by the runner. `AnomalyDetector`,
   `ConductorBandit`, and streaming event forwarding are in the same state.

The fix pattern is consistent: **wire, don't build.** Every task below connects
existing code to existing call sites.

### Verified State (2026-04-29)

Items verified against source on branch `wp-arch2`:

- `legacy-orchestrate` is ON by default (`Cargo.toml` line 16: `default = ["legacy-orchestrate"]`).
- `ConfigCmd::Mcp` has `unreachable!()` at `config_cmd.rs:209` but the dispatch IS
  intercepted earlier in `main.rs:2132` via `dispatch_mcp_cmd()`. HOLLOW 1 is **already fixed**.
- `share.rs` already scrubs secrets via `scrub_share_text()` (lines 49-53). T28 from
  the impl plan is **already fixed**.
- `BudgetConfig` exists at `roko-core/src/config/budget.rs` with only `max_plan_usd`,
  `max_turn_usd`, and `prompt_token_budget`. Missing: per-task, per-session, per-day,
  warn/route thresholds.
- Runner v2 reads `max_plan_usd` and `max_turn_usd` from `BudgetConfig` (confirmed at
  `runner/types.rs:1351-1352`). Per-plan and per-turn budget checks are wired at
  `event_loop.rs:349` and `event_loop.rs:1764`.
- `CascadeRouter` is loaded in `runner/types.rs:1306` and stored in `RunnerConfig`.
  A comment at `event_loop.rs:2347` says "CascadeRouter observation" but no
  `cascade_router.observe()` call exists in the runner.
- `roko init` writes `[[gate]]` format (confirmed at `init.rs:130`). The runtime reads
  `[gates]` format. The disconnect is real.
- `share_routes::auth_routes()` IS inside the auth layer (confirmed at
  `routes/mod.rs:117`). The security CRITICAL finding from the audit is **already fixed**.
- `ProviderConfig::resolve_api_key()` exists at `config/provider.rs:76-80` but is a
  simple env-var lookup. No inline key support, no secrets store fallback.
- `validate_references()` exists at `config/schema.rs:974` and checks provider/model
  references. Partial -- does not check api_key_env resolution or budget validity.
- `roko-neuro` has `default = []` (no hdc). The `hdc` feature requires `dep:roko-primitives`.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Location | Impact |
|----|-------------|----------|--------|
| AP-1 | `auth_detect.rs` ignores `roko.toml` providers | `crates/roko-cli/src/auth_detect.rs:66-98` | `default_model` config has no effect |
| AP-2 | `roko init` writes `[[gate]]` but runtime reads `[gates]` | `crates/roko-cli/src/commands/init.rs:130` | Gates from init are silently discarded |
| AP-3 | Direct env var reads for API keys bypass provider system | 8+ files across crates | No cost tracking, no rotation, no config-driven auth |
| AP-4 | `unsafe { std::env::set_var() }` for --provider flag | `crates/roko-cli/src/commands/util.rs:236` | Unsound in multi-threaded contexts |
| AP-5 | `ROKO_ACP_LEGACY` env gate suppresses features silently | `crates/roko-acp/src/bridge_events.rs:843,2011,2053` | ACP output missing features with no warning |
| AP-6 | CascadeRouter loaded but `.observe()` never called in runner | `crates/roko-cli/src/runner/event_loop.rs` | Router never learns from dispatch outcomes |
| AP-7 | `BudgetGuardrail` (roko-learn) never instantiated in runner | `crates/roko-learn/src/budget.rs` | No graduated budget enforcement |
| AP-8 | Hardcoded `max_tokens` differs per entry point | `dispatch_direct.rs:196` (8192), gateway (1024), demo (512) | Inconsistent response lengths |

---

## Task 16.1: Make `auth_detect.rs` Respect `roko.toml` Provider Config

**Priority:** P1 -- Config Correctness
**Effort:** Medium
**Depends on:** None

### Problem

`detect_auth()` at `crates/roko-cli/src/auth_detect.rs:66` scans env vars
(`ZAI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) in a fixed priority order,
completely ignoring `[providers]`, `[agent].default_model`, and `[agent].default_backend`
from `roko.toml`. Setting `default_model = "cerebras-70b"` in config has zero effect
when `ANTHROPIC_API_KEY` exists in the environment.

Callers: `crates/roko-cli/src/unified.rs` (imports `detect_auth`).

### Files

- `crates/roko-cli/src/auth_detect.rs` -- add config-aware variant
- `crates/roko-cli/src/model_selection.rs` -- `resolve_effective_model()` at line 140
  already takes config; ensure all callers use it
- `crates/roko-cli/src/unified.rs` -- update callers

### Fix

1. Add `detect_auth_with_config(config: &RokoConfig) -> AuthMethod` that resolves
   from `config.agent.default_model` and `config.agent.default_backend` first,
   finding the matching provider in `config.providers`.
2. Only fall back to env var scanning when config has no explicit provider/model
   or when the configured provider's API key cannot be resolved.
3. Update all callers of `detect_auth()` to pass the loaded config when available.
4. Keep the zero-arg `detect_auth()` as the bootstrapping path for when no config exists.

### Acceptance

- Setting `default_model = "cerebras-70b"` in roko.toml and running `roko run "hello"`
  dispatches to Cerebras, not Claude (assuming Cerebras provider key is available).
- With no roko.toml, `detect_auth()` still works via env var scanning.

---

## Task 16.2: Fix `roko init` Gate Format to Match Runtime

**Priority:** P1 -- Config Correctness
**Effort:** Small
**Depends on:** None

### Problem

`roko init` calls `append_shell_gate()` at `crates/roko-cli/src/commands/init.rs:130`
which emits `[[gate]]` TOML array-of-tables syntax. The runtime (`RokoConfig::from_toml()`
at `crates/roko-core/src/config/schema.rs:171`) reads `[gates]` table format with
`enabled`, `clippy_enabled`, `skip_tests`, and `shell_gates` fields. Result: gates
generated by `roko init` are silently discarded by `roko plan run`.

### Files

- `crates/roko-cli/src/commands/init.rs` -- `append_verification_gates()` at line 102,
  `append_shell_gate()` at line 129

### Fix

1. Rewrite `append_verification_gates()` to emit `[gates]` table format:
   ```toml
   [gates]
   clippy_enabled = true
   skip_tests = false

   [[gates.shell_gates]]
   name = "cargo-check"
   command = "cargo check --workspace"
   timeout_ms = 600000
   ```
2. For Rust profile: set `clippy_enabled = true`, `skip_tests = false`, add shell gates
   for `cargo check` and `cargo test`.
3. For TypeScript profile: add shell gates for `npx tsc --noEmit` and `npm test`.
4. Remove `append_shell_gate()` (the `[[gate]]` emitter) or rename it to match the
   new format.
5. Add an inline comment in the generated TOML documenting gate config.

### Acceptance

- `roko init --profile rust` generates gates in `[gates]` format.
- `roko plan run` on a trivial plan uses those gates without manual editing.
- No `[[gate]]` syntax in the generated roko.toml.

---

## Task 16.3: Thread `workflow.template` Config Into Runner V2

**Priority:** P2 -- UX
**Effort:** Small
**Depends on:** None

### Problem

`[workflow].template` (express/standard/full) is read by `WorkflowEngine` but runner v2
(`roko plan run`) uses its own hardcoded config, ignoring the template entirely.
`[workflow].max_iterations` is also ignored by runner v2 (which has its own `max_retries`
field at `runner/types.rs:1385`).

No `workflow.template` or `workflow_template` references exist in `crates/roko-cli/src/runner/`.

### Files

- `crates/roko-cli/src/runner/types.rs` -- `RunnerConfig` struct, line ~1240
- `crates/roko-cli/src/runner/event_loop.rs` -- event loop retry logic

### Fix

1. In `RunnerConfig::from_roko_config()`, read `workflow.template` and
   `workflow.max_iterations` from `RokoConfig`.
2. Map template to concrete settings: express (1 retry), standard (3 retries),
   full (5 retries + review gate).
3. Use `workflow.max_iterations` as the cap on `max_retries`, falling back to the
   template's default if not set.
4. Remove the hardcoded `max_retries` default in favor of the config-derived value.

### Acceptance

- Setting `workflow.template = "express"` in roko.toml reduces plan runner retries to 1.
- Setting `workflow.max_iterations = 5` overrides the template default.

---

## Task 16.4: Wire `learning.replan_on_gate_failure` Into Runner V2

**Priority:** P2 -- Learning
**Effort:** Medium
**Depends on:** None

### Problem

`learning.replan_on_gate_failure` is parsed from roko.toml into
`LearningConfig.replan_on_gate_failure` (`roko-core/src/config/learning.rs:39`,
defaults to `true`). The flag is consumed by orchestrate.rs (`orchestrate.rs:5101`)
but runner v2 never reads it. Gate failures exhaust the autofix budget and mark the
task as failed without triggering replanning.

### Files

- `crates/roko-cli/src/runner/types.rs` -- add field to `RunnerConfig`
- `crates/roko-cli/src/runner/event_loop.rs` -- gate failure handling path
- `crates/roko-cli/src/orchestrate.rs` -- reference: `build_gate_failure_plan_revision()`

### Fix

1. Add `replan_on_gate_failure: bool` to `RunnerConfig`, populated from
   `roko_config.learning.replan_on_gate_failure`.
2. In the event loop, after autofix budget is exhausted, if `replan_on_gate_failure`
   is true, extract or call `build_gate_failure_plan_revision()` from orchestrate.rs.
3. The revision spawns a strategist agent with gate error context to produce a revised
   approach, which is then retried.
4. Cap replan attempts at 1 per task to prevent infinite loops.

### Acceptance

- With `replan_on_gate_failure = true`, a task that fails all autofix attempts spawns a
  strategist before giving up.
- The strategist's output is visible in the episode log.
- With `replan_on_gate_failure = false`, behavior is unchanged from current.

---

## Task 16.5: Complete `[budget]` Schema and Wire `BudgetGuardrail`

**Priority:** P1 -- Config Correctness
**Effort:** Medium
**Depends on:** None

### Problem

Two separate issues:

1. `BudgetConfig` at `crates/roko-core/src/config/budget.rs` has only 3 fields:
   `max_plan_usd` (default 25.0), `max_turn_usd` (default 3.0), `prompt_token_budget`
   (default 10000). Missing: per-task, per-session, per-day, warn threshold,
   route-to-cheaper threshold.

2. `BudgetGuardrail` at `crates/roko-learn/src/budget.rs` implements 3-scope limits
   with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block).
   It exists in two crates (`roko-learn/src/budget.rs:8` and
   `roko-agent/src/task_runner.rs:196`). The roko-learn version is never instantiated
   in any live runner path. The roko-agent version is used only inside `TaskRunner`
   (which is itself only used in orchestrate.rs).

   Runner v2 has basic budget checks at `event_loop.rs:349` (per-turn) and
   `event_loop.rs:1764` (per-plan), but these are simple threshold comparisons,
   not the graduated guardrail.

### Files

- `crates/roko-core/src/config/budget.rs` -- extend `BudgetConfig`
- `crates/roko-learn/src/budget.rs` -- `BudgetGuardrail`
- `crates/roko-cli/src/runner/types.rs` -- `RunnerConfig`
- `crates/roko-cli/src/runner/event_loop.rs` -- instantiate guardrail

### Fix

1. Add fields to `BudgetConfig`:
   - `max_cost_per_task: Option<f64>` (default: None)
   - `max_cost_per_session: Option<f64>` (default: None)
   - `max_cost_per_day: Option<f64>` (default: None)
   - `warn_threshold: f64` (default: 0.8)
   - `route_to_cheaper_threshold: f64` (default: 0.9)
2. Instantiate `BudgetGuardrail` in `RunnerConfig::from_roko_config()`, populating from
   the extended `BudgetConfig`.
3. In event loop, before each dispatch, call `guardrail.check()`.
   - On `Warn`: log warning with current spend.
   - On `RouteToCheaper`: override model selection to cheapest available.
   - On `Block`: return error with clear message.
4. Replace the inline budget checks at `event_loop.rs:349` and `event_loop.rs:1764`
   with `guardrail.check()` calls.

### Acceptance

- `roko config show` displays the budget section with all fields.
- Setting `budget.max_plan_usd = 0.01` causes a plan run to hit budget warning or block.
- `BudgetGuardrail.check()` is called at least once per task dispatch.

---

## Task 16.6: Wire `agent.tier_models` Into Live Dispatch

**Priority:** P2 -- UX
**Effort:** Medium
**Depends on:** None

### Problem

`agent.tier_models` maps task tiers to model slugs. The mapping is loaded into
`CascadeRouter` via `model_slugs_for_config()` at `service_factory.rs:251` and
`model_call_service.rs:814`, but the tier routing path is never called at dispatch
time in runner v2. All tasks use the default model regardless of their declared tier.

The legacy orchestrate.rs path DOES use tier models (confirmed at lines 989, 5367,
9842, 10301, 11720, 12837, 13314, 13634), calling `task.effective_model()` with
`tier_models` from config. Runner v2 does not.

### Files

- `crates/roko-cli/src/runner/event_loop.rs` -- dispatch section
- `crates/roko-cli/src/task_parser.rs` -- `effective_model()` at line 415 already
  accepts `tier_models: Option<&HashMap<String, String>>`
- `crates/roko-cli/src/runner/types.rs` -- RunnerConfig

### Fix

1. In `RunnerConfig`, store `tier_models: HashMap<String, String>` populated from
   `roko_config.agent.tier_models`.
2. At dispatch time in the event loop, call `task.effective_model(&default_model,
   Some(&config.tier_models))` to resolve the model for the current task.
3. Pass the resolved model to the agent spawn command instead of always using the
   default.

### Acceptance

- Setting `[agent.tier_models]` with `T3 = "claude-opus-4-6"` causes T3 tasks to
  dispatch to Opus, while T0 tasks use the default model.

---

## Task 16.7: Replace Direct `ANTHROPIC_API_KEY` Read in `episode_completion.rs`

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

`crates/roko-neuro/src/episode_completion.rs:46` reads `ANTHROPIC_API_KEY` directly
from the environment to construct its own HTTP client for neuro distillation calls.
This bypasses the provider system, credential rotation, and cost tracking.

### Files

- `crates/roko-neuro/src/episode_completion.rs`

### Fix

1. Add a `model_caller: Option<Arc<dyn ModelCaller>>` parameter to the distillation
   entry point (or accept a configured `ModelCallService`).
2. Remove the direct `std::env::var("ANTHROPIC_API_KEY")` read.
3. The caller passes the `ModelCallService` it already has access to.
4. If no model caller is available, skip distillation with a logged warning.

### Acceptance

- `episode_completion.rs` has zero `std::env::var` calls.
- Distillation works when configured through provider config.

---

## Task 16.8: Replace Direct `PERPLEXITY_API_KEY` Reads

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

8 locations read `PERPLEXITY_API_KEY` directly from the environment:

- `crates/roko-std/src/tool/builtin/web_search.rs:332,403`
- `crates/roko-cli/src/orchestrate.rs:4473,4692,4904,17368` (legacy, lower priority)
- `crates/roko-cli/src/chat_inline.rs:3101` (capability check)
- `crates/roko-cli/src/commands/research.rs:724`

### Files

- `crates/roko-std/src/tool/builtin/web_search.rs` -- primary fix
- `crates/roko-cli/src/commands/research.rs` -- primary fix
- Other files listed above -- minimal fix (config-first, env fallback)

### Fix

1. Add a helper: `resolve_provider_api_key(config: &RokoConfig, provider_name: &str)
   -> Option<String>` that checks `config.providers[provider_name].resolve_api_key()`
   first, then falls back to the well-known env var.
2. In `web_search.rs`, accept the key via config at tool registry construction time.
   Fall back to `std::env::var("PERPLEXITY_API_KEY")` only in tests or standalone usage.
3. In `commands/research.rs:724`, resolve from config first.
4. Log a deprecation warning when using the env var fallback in non-test code.

### Acceptance

- `roko research search "test"` works when the Perplexity key is configured only in
  roko.toml's provider config, not in the environment.
- The env var fallback still works but logs a warning.

---

## Task 16.9: Remove `unsafe { std::env::set_var() }` for Provider Override

**Priority:** P1 -- Correctness
**Effort:** Small
**Depends on:** None

### Problem

The `--provider` CLI flag uses `unsafe { std::env::set_var("ROKO_PROVIDER", p) }` at
`crates/roko-cli/src/commands/util.rs:236`. Rust 2024 edition marks `set_var` as
`unsafe` because it is unsound in multi-threaded contexts. Two other `set_var` calls
exist in `main.rs:2225` (`ROKO_HIGH_CONTRAST`) and `main.rs:2229` (`ROKO_REDUCED_MOTION`).

### Files

- `crates/roko-cli/src/commands/util.rs` -- line 236
- `crates/roko-cli/src/main.rs` -- lines 2225, 2229

### Fix

1. For `--provider`: thread the override through a field on the command context or
   `ServiceConfig` struct. Remove the `set_var` call.
2. For `ROKO_HIGH_CONTRAST` and `ROKO_REDUCED_MOTION`: these are set very early
   (before any threads spawn) so they are safe in practice. Move them to before
   tokio runtime creation, or replace with config struct fields. At minimum, add
   a `// SAFETY:` comment explaining why the call is sound (single-threaded at this
   point).

### Acceptance

- `roko run --provider anthropic "hello"` works without calling `set_var`.
- No `unsafe { std::env::set_var }` remains for `ROKO_PROVIDER`.

---

## Task 16.10: Route All CLI Entry Points Through `ServiceFactory::build()`

**Priority:** P1 -- Config Correctness
**Effort:** Large
**Depends on:** 16.1

### Problem

Nine dispatch paths exist with inconsistent model selection. Only paths through
`ServiceFactory` get full feedback recording, cost tracking, cascade routing, and
knowledge injection. `ServiceFactory` is used in `run.rs` but callers in
`chat_session.rs`, `chat_inline.rs`, `commands/prd.rs`, and `dispatch_v2.rs` may
construct agents directly.

Hardcoded model strings like `"claude-sonnet-4-6"` exist in production code paths
(not just tests): `plan_generate.rs:131`, `explain.rs:407` (uses `"gpt-4o"`).

### Files

- `crates/roko-cli/src/run.rs` -- already uses ServiceFactory (confirm completeness)
- `crates/roko-cli/src/chat_session.rs` -- needs ServiceFactory for feedback
- `crates/roko-cli/src/chat_inline.rs` -- needs ServiceFactory
- `crates/roko-cli/src/commands/prd.rs` -- agent dispatch
- `crates/roko-cli/src/dispatch_v2.rs` -- agent dispatch
- `crates/roko-cli/src/plan_generate.rs` -- hardcoded model at line 131
- `crates/roko-cli/src/explain.rs` -- hardcoded `"gpt-4o"` at line 407

### Fix

1. Audit each entry point for direct `create_agent_for_model()` calls that bypass
   `ServiceFactory::build()`. Replace with ServiceFactory.
2. Remove hardcoded model strings in non-test code. Replace with
   `config.agent.default_model` resolution or tier-based lookup.
3. Ensure `FeedbackService` is constructed for every path (fixes the "chat records
   zero learning signals" issue).
4. Verify `dispatch_direct.rs` is unreachable from default builds (it is behind
   `legacy-orchestrate` which is currently default -- so it IS reachable).

### Acceptance

- `grep -rn '"claude-sonnet-4-6"' crates/roko-cli/src/ --include='*.rs' | grep -v test | grep -v doc | grep -v comment`
  returns zero hits in non-test production code paths.
- All dispatch paths log the model source via `EffectiveModelSelection`.

---

## Task 16.11: Wire CascadeRouter Observations Into Runner V2

**Priority:** P2 -- Learning
**Effort:** Medium
**Depends on:** 16.10

### Problem

`CascadeRouter` is loaded at `runner/types.rs:1306` and stored in `RunnerConfig`.
A comment at `event_loop.rs:2347` says "CascadeRouter observation: record gate outcome
for learned model selection" but no `cascade_router.observe()` call exists in the
runner event loop. The router accumulates zero observations from plan runs.

`resolve_effective_model()` at `model_selection.rs:140` accepts
`Option<&CascadeRouter>` but callers frequently pass `None`.

### Files

- `crates/roko-cli/src/runner/event_loop.rs` -- after task completion
- `crates/roko-cli/src/runner/types.rs` -- `RunnerConfig.cascade_router`
- `crates/roko-cli/src/model_selection.rs` -- pass router to callers

### Fix

1. After task completion in the event loop, call `cascade_router.observe()` with the
   model used, success/failure, and response quality metrics.
2. Persist the router to disk after each observation batch (or on flush interval).
3. In `resolve_effective_model_key()` at `model_selection.rs:184`, load the
   CascadeRouter from disk if a `.roko` directory exists instead of hardcoding `None`.
4. Add a startup log line showing CascadeRouter state: observation count, stage, model count.

### Acceptance

- After running a 5-task plan, `.roko/learn/cascade-router.json` has `observations > 0`.
- `roko learn router` shows observation count increasing after each run.

---

## Task 16.12: Add FeedbackService to `roko chat` Path

**Priority:** P2 -- Learning
**Effort:** Small
**Depends on:** 16.10

### Problem

`roko chat` records zero learning signals. No episodes, no routing observations, no
cost tracking. `FeedbackService` and `FeedbackEvent` are not referenced anywhere in
`chat_session.rs` (confirmed via grep). Chat is the most-used interactive entry point.

### Files

- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/chat_inline.rs`

### Fix

1. If 16.10 routes chat through `ServiceFactory::build()`, the `FeedbackService` is
   constructed automatically. Verify this is wired.
2. Emit `FeedbackEvent::WorkflowComplete` when the chat session ends (on `/quit` or
   Ctrl-D) so the session's total cost is recorded.
3. Emit per-turn cost observations for CascadeRouter learning.

### Acceptance

- After a 3-turn chat session, `.roko/learn/cascade-router.json` has 3 new observations.
- `.roko/learn/costs.jsonl` has cost records for the session.

---

## Task 16.13: Wire AnomalyDetector Into Live Paths

**Priority:** P3 -- UX
**Effort:** Small
**Depends on:** None

### Problem

`AnomalyDetector` is defined in `crates/roko-learn/src/anomaly.rs` (also in
`roko-agent/src/task_runner.rs` and re-exported from `roko-agent/src/lib.rs:151`).
It detects prompt loops, cost spikes, and quality degradation. It is referenced in
`roko-cli/src/orchestrate.rs` and `roko-cli/src/learning_helpers.rs` but never
instantiated in the runner v2 event loop.

### Files

- `crates/roko-learn/src/anomaly.rs` -- the detector
- `crates/roko-cli/src/runner/event_loop.rs` -- instantiate and call

### Fix

1. Create `AnomalyDetector` at session start in runner v2 event loop.
2. Before each dispatch, call `detector.check_prompt(prompt_hash)` for loop detection.
3. After each response, call `detector.check_cost(cost_usd)` for spike detection.
4. On anomaly: log at WARN level. Optionally record in episode's anomalies field.

### Acceptance

- Dispatching the same prompt 5 times triggers a prompt-loop warning in logs.
- A response costing 10x the session average triggers a cost-spike warning.

---

## Task 16.14: Wire ConductorBandit Into Plan Runner Retry Loop

**Priority:** P3 -- UX
**Effort:** Medium
**Depends on:** None

### Problem

`ConductorBandit` at `crates/roko-learn/src/conductor.rs` (also in
`roko-agent/src/task_runner.rs`, `roko-conductor/src/interventions.rs`) decides
whether a failing task should continue, receive a hint, escalate, restart, or abort.
It is never invoked in runner v2. All retry decisions use hardcoded logic.

### Files

- `crates/roko-learn/src/conductor.rs`
- `crates/roko-cli/src/runner/event_loop.rs` -- retry/failure handling

### Fix

1. Load `ConductorBandit` state from `.roko/learn/conductor.json` at plan runner start.
2. On task failure, call `bandit.select_action(context)` instead of hardcoded retry.
3. Map actions: Continue -> retry same model, Hint -> inject failure context,
   Escalate -> switch to stronger model, Restart -> clear state and retry,
   Abort -> mark task failed.
4. Feed reward after retry outcome. Save state after each observation.

### Acceptance

- After 20+ task completions with mixed success, conductor's action distribution is
  non-uniform. `.roko/learn/conductor.json` has observations.

---

## Task 16.15: Wire Streaming Events to TUI in `chat_inline.rs`

**Priority:** P2 -- UX
**Effort:** Medium
**Depends on:** None

### Problem

The chat inline handler discards streaming events. Grep for
`while let Some(_event) = event_rx.recv().await` at `chat_inline.rs` returned no
results (the pattern may have been partially fixed), but the audit (AP-7) documents
that streaming events are drained without rendering. The TUI shows a spinner until
the entire response is complete.

### Files

- `crates/roko-cli/src/chat_inline.rs`

### Fix

1. Verify current state of event handling in chat_inline.rs.
2. If events are still discarded: replace `_event` with actual event mapping to the
   inline renderer or `DashboardEvent`.
3. For `ClaudeStreamEvent::Assistant`: render text tokens incrementally in the viewport.
4. For `ClaudeStreamEvent::Tool`: show tool call name and progress indicator.
5. For `ClaudeStreamEvent::Result`: finalize the response display.

### Acceptance

- Running `roko chat` and sending a message shows tokens appearing incrementally,
  not all-at-once after completion.

---

## Task 16.16: Wire `build_repo_context()` Into Plan Generation

**Priority:** P2 -- UX
**Effort:** Small
**Depends on:** None

### Problem

`build_repo_context()` at `crates/roko-cli/src/repo_context.rs:282` gives agents
awareness of repository structure. It IS called from `prd draft new`
(`commands/prd.rs:383`) but NOT from `plan generate`, `plan regenerate`, or `prd plan`.

Generated plans propose greenfield crates that duplicate existing functionality.

### Files

- `crates/roko-cli/src/commands/plan.rs` -- plan generate/regenerate handlers
- `crates/roko-cli/src/prd.rs` -- prd plan handler (line 877 calls it for draft, not plan)
- `crates/roko-cli/src/repo_context.rs` -- the function

### Fix

1. Call `build_repo_context()` before agent dispatch in `plan generate`,
   `plan regenerate`, and `prd plan` handlers.
2. Include the repo context as a system prompt section or user context section.
3. For `plan regenerate`, also inject validation errors from the previous attempt
   into the regeneration prompt.

### Acceptance

- Running `roko plan generate` on a workspace with 18 crates produces a plan that
  references existing crates rather than proposing new ones.

---

## Task 16.17: Add Config Validation on Load

**Priority:** P1 -- Config Correctness
**Effort:** Medium
**Depends on:** None

### Problem

`RokoConfig::from_toml()` at `crates/roko-core/src/config/schema.rs:171` accepts any
syntactically valid TOML without semantic validation. `validate_references()` at line
974 exists and checks provider/model references, but it is not called automatically
on load.

Missing checks: `default_model` resolves to a known model, provider `api_key_env`
values have corresponding env vars, budget values are non-negative, `tier_models`
values are valid model keys.

### Files

- `crates/roko-core/src/config/schema.rs` -- extend `validate_references()`
- `crates/roko-cli/src/config.rs` -- call validation on load

### Fix

1. Extend `validate_references()` to also check:
   - `default_model` resolves to a model in `[models]` table or is a known alias.
   - Provider `api_key_env` values have corresponding env vars set (warn if not).
   - Budget values are non-negative.
   - `tier_models` values are valid model keys.
2. Call `validate_references()` after loading config and print warnings to stderr.
3. `roko config validate` runs the full validation and returns a structured report.

### Acceptance

- Setting `default_model = "nonexistent"` produces a startup warning.
- `roko config validate` reports all detected issues.

---

## Task 16.18: Normalize Model Aliases at Load Time

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

Duplicate model entries exist: `glm-5-1` on provider "zai" vs `glm51` on provider
"zhipu" both resolve to `glm-5.1`. No `normalize_model` function exists in
`roko-core/src/config/` (confirmed via grep). `CascadeRouter` treats them as separate
models, fragmenting observations.

### Files

- `crates/roko-core/src/agent.rs` -- `resolve_model()` at line 5 of model_selection.rs
- `crates/roko-orchestrator/src/service_factory.rs`
- `crates/roko-learn/src/cascade_router.rs`

### Fix

1. Add `normalize_model_slug(slug: &str) -> String` that canonicalizes known aliases:
   `glm-5-1` / `glm51` / `glm-5.1` -> `glm-5.1`,
   `claude-sonnet-4-6` / `claude-sonnet-4-6-20250514` -> canonical form.
2. Call in `resolve_model()` before returning.
3. Call in `CascadeRouter` before recording observations or selecting models.

### Acceptance

- Running tasks with both `glm-5-1` and `glm51` produces observations against a
  single canonical model in the cascade router, not two separate entries.

---

## Task 16.19: Remove or Replace `ROKO_ACP_LEGACY` Environment Variable Gate

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

File changes, phase badges, narrative text, and forensic analysis in the ACP pipeline
require `ROKO_ACP_LEGACY` to be set. Three checks exist at
`crates/roko-acp/src/bridge_events.rs:843,2011,2053`. Without the env var, these
features are compiled but suppressed with no user-facing indication.

### Files

- `crates/roko-acp/src/bridge_events.rs` -- lines 843, 2011, 2053

### Fix

1. Replace the `std::env::var_os("ROKO_ACP_LEGACY")` checks with a config field:
   `[acp].legacy_features = true` (default true, enabling all features).
2. If the features should always be on (likely, since they are informational), remove
   the gate entirely.
3. Remove all `ROKO_ACP_LEGACY` env reads.

### Acceptance

- ACP sessions produce file change reports and phase badges without setting any env var.
- `grep -rn ROKO_ACP_LEGACY crates/` returns zero hits.

---

## Task 16.20: Audit and Document `legacy-orchestrate` Feature Flag

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

The `legacy-orchestrate` feature flag is ON by default (`Cargo.toml` line 16:
`default = ["legacy-orchestrate"]`). It gates code in 7 files:
`run.rs`, `dispatch_direct.rs`, `chat_inline.rs`, `lib.rs`, `auth_detect.rs`,
`unified.rs`, and the massive `orchestrate.rs` (~21K lines).

It is unclear what the migration path is, what the flag controls, and whether
disabling it breaks anything critical.

### Files

- `crates/roko-cli/Cargo.toml` -- feature definition
- All 7 files with `#[cfg(feature = "legacy-orchestrate")]` guards

### Fix

1. Document the current default state and what the flag controls.
2. Add `// DEPRECATED: legacy-orchestrate` comment blocks at every gated section.
3. Create a tracking list of all gated code blocks (file, line range, what it does).
4. For code that is the only implementation of a needed feature: plan extraction to
   an ungated module. For purely dead code: mark with `#[deprecated]`.
5. Append the tracking list to `.roko/GAPS.md`.

### Acceptance

- Every `#[cfg(feature = "legacy-orchestrate")]` section has adjacent documentation.
- A tracking list exists in `.roko/GAPS.md` listing all gated blocks.

---

## Task 16.21: Enable `hdc` Feature by Default for `roko-neuro`

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

`roko-neuro/Cargo.toml` has `default = []` at line 16. The `hdc` feature (line 17:
`hdc = ["dep:roko-primitives"]`) is required for anti-knowledge gating and HDC-based
similarity scoring in `KnowledgeStore`. Without it, quality-control mechanisms are
inactive in default builds.

### Files

- `crates/roko-neuro/Cargo.toml`
- Workspace `Cargo.toml` (if enabling via workspace dependency)

### Fix

1. Check whether `roko-cli`'s dependency on `roko-neuro` enables `hdc`:
   look for `features = ["hdc"]` in roko-cli's Cargo.toml.
2. If not enabled: either add `hdc` to `roko-neuro`'s default features
   (`default = ["hdc"]`), or enable it in roko-cli's dependency declaration.
3. If `roko-primitives` has heavy dependencies, keep `hdc` optional but ensure
   the roko-cli binary enables it.

### Acceptance

- Building `roko-cli` with default features includes HDC fingerprinting.

---

## Task 16.22: Consolidate Hardcoded Max-Token Values

**Priority:** P3 -- Cleanup
**Effort:** Small
**Depends on:** None

### Problem

Max tokens for the same model vary by entry point:
- `dispatch_direct.rs:196,280` -> 8192
- `chat_session.rs:555` -> 4096
- `batch_client.rs:293` -> 4096
- `lifecycle.rs:329` -> 4096
- `gateway.rs:1027` -> 1024
- `demo scenarios` -> 512

### Files

- Multiple files across 4+ crates (listed above)

### Fix

1. Add `max_output_tokens: Option<u32>` to `ModelProfile` in the config schema.
2. `resolve_model()` returns `max_output_tokens` as part of the resolved model info.
3. All dispatch paths use `model_profile.max_output_tokens.unwrap_or(4096)` instead
   of hardcoded values.
4. Demo/gateway can override to lower values but must do so explicitly via config.

### Acceptance

- Setting `max_output_tokens = 16384` on a model profile produces longer responses.
- No hardcoded max_tokens constants remain outside test code and demos.

---

## Task 16.23: Auto-Provision Auth on Cloud Deploy

**Priority:** P0 -- Security
**Effort:** Medium
**Depends on:** None

### Problem

`roko serve` binds to `0.0.0.0:6677` with auth disabled by default. Cloud deployments
(`roko deploy railway`) expose this publicly with no authentication. The
`acknowledge_public_risk` flag at `roko-serve/src/lib.rs:644` bypasses the auth warning
without actually enabling auth.

### Files

- `crates/roko-cli/src/main.rs` (deploy handlers)
- `crates/roko-cli/src/commands/server.rs`
- `crates/roko-serve/src/lib.rs` -- lines 644-656

### Fix

1. In `roko deploy railway/fly/docker`, auto-generate a random API key if none configured.
2. Set `api_auth.enabled = true` in the deploy config.
3. Print the generated API key to stdout so the user can save it.
4. Set the key as a Railway/Fly secret automatically.
5. In `roko serve`, if binding to `0.0.0.0` and auth is not enabled, print a
   prominent warning. The `acknowledge_public_risk` flag should ALSO check that
   auth is actually enabled, not just suppress the warning.

### Acceptance

- `roko deploy railway` output includes an auto-generated API key.
- `acknowledge_public_risk = true` without `auth.enabled = true` still warns.

---

## Task 16.24: Wire Secret Resolution Through Provider Config

**Priority:** P2 -- Correctness
**Effort:** Medium
**Depends on:** None

### Problem

`ProviderConfig::resolve_api_key()` at `crates/roko-core/src/config/provider.rs:76`
only checks `api_key_env` -> env var. It does not support:
- Inline `api_key` field (for testing/simple setups)
- Profile-aware secrets store (`roko config secrets`)

Some code reads env var names from config and resolves them. Other code hardcodes the
env var name directly. The `config secrets` subcommands exist but the resolution path
is inconsistent.

### Files

- `crates/roko-core/src/config/provider.rs` -- `resolve_api_key()`
- `crates/roko-cli/src/commands/config_cmd.rs` -- `config providers health`

### Fix

1. Extend `ProviderConfig::resolve_api_key()` to check in order:
   - `self.api_key` (inline key, if field is added)
   - `self.api_key_env` -> `std::env::var(name)`
   - Profile-aware secrets store (if available)
2. All provider adapter constructors use `provider_config.resolve_api_key()`.
3. `roko config check-secrets` verifies all configured providers have resolvable keys.
4. `roko config providers health` calls `resolve_api_key()` and reports status.

### Acceptance

- `roko config providers health` shows green/red status for each provider's key.
- A provider with `api_key_env = "CUSTOM_KEY"` resolves when `CUSTOM_KEY` is set.

---

## Task 16.25: Fix Dual Episode Writes in `roko run`

**Priority:** P2 -- Correctness
**Effort:** Small
**Depends on:** 16.10

### Problem

`roko run` writes episodes twice:
- Direct `append_episode_log()` call at `run.rs:1301`
- `LearningRuntime::record_completed_run()` at `run.rs:2680`

This produces duplicate records in different files or double entries in the same file.

### Files

- `crates/roko-cli/src/run.rs` -- lines 1301 and 2680

### Fix

1. Remove the direct `append_episode_log()` call at line 1301.
2. Let `LearningRuntime` (via `FeedbackService`) be the single episode writer.
3. Verify that `FeedbackService` writes to the canonical path.
4. If two paths differ (`.roko/episodes.jsonl` vs `.roko/learn/episodes.jsonl`),
   pick one canonical location and update all readers.

### Acceptance

- Running `roko run "hello"` produces exactly 1 episode record, not 2.

---

## Task 16.26: Add Config Migration for Gate Format

**Priority:** P3 -- Migration
**Effort:** Medium
**Depends on:** 16.2

### Problem

The `[[gate]]` to `[gates]` format change (16.2) creates a breaking change for
existing workspaces. Config versioning already exists (`config_version` at
`schema.rs:50`, `schema_version` at `schema.rs:52`) but no migration logic
handles the gate format change.

### Files

- `crates/roko-core/src/config/schema.rs` -- `from_toml()`, version handling
- `crates/roko-cli/src/config.rs` -- CLI config layer

### Fix

1. In `RokoConfig::from_toml()`, detect `[[gate]]` array syntax in the raw TOML
   before deserialization. If present, normalize to `[gates]` format.
2. Print a one-time migration hint: `hint: run "roko config migrate" to update format`.
3. `roko config migrate` rewrites the file, creates `roko.toml.bak` backup.
4. Bump `config_version` to indicate the new format.

### Acceptance

- `roko config migrate` on a roko.toml with `[[gate]]` arrays produces valid `[gates]`.
- A backup exists at `roko.toml.bak`.

---

## Dependency Graph

```
16.1  ──────────────────┐
                        ├── 16.10 ── 16.11
16.2  ── 16.26          │           16.12
16.3                    │           16.25
16.4                    │
16.5                    │
16.6                    │
16.7                    │
16.8                    │
16.9                    │
16.13                   │
16.14                   │
16.15                   │
16.16                   │
16.17                   │
16.18                   │
16.19                   │
16.20                   │
16.21                   │
16.22                   │
16.23                   │
16.24                   │
```

Most tasks are independent. The critical path is 16.1 -> 16.10 -> 16.11/16.12/16.25.
16.2 -> 16.26 is a sequential pair.

---

## Removed Tasks (Already Fixed)

The following tasks from the impl plan were verified as already fixed:

| Impl Task | Finding | Evidence |
|-----------|---------|----------|
| T14 (Wire ConfigCmd::Mcp) | MCP dispatch is wired | `main.rs:2132` intercepts and calls `dispatch_mcp_cmd()` at line 2790 |
| T28 (Secret scrubbing in CLI Gist) | Scrubbing is implemented | `share.rs:49-53` calls `scrub_share_text()` with `LogScrubber` |
| T32 (Share routes auth) | Routes are inside auth layer | `routes/mod.rs:117` mounts `shared_runs::auth_routes()` inside the auth layer |

---

## Priority Order

**P0 -- Security (do first):**
- 16.23 (cloud deploy auth)

**P1 -- Config correctness (core functionality):**
- 16.1 (auth_detect config), 16.2 (init gate format), 16.9 (remove set_var),
  16.5 (budget schema + guardrail), 16.10 (ServiceFactory routing), 16.17 (validation)

**P2 -- Learning + UX:**
- 16.11 (CascadeRouter), 16.12 (chat feedback), 16.4 (replan), 16.25 (dual episodes),
  16.3 (workflow template), 16.6 (tier models), 16.15 (streaming), 16.16 (repo context),
  16.24 (secret resolution)

**P3 -- Cleanup and migration:**
- 16.7, 16.8 (env var elimination), 16.13 (anomaly), 16.14 (conductor),
  16.18 (aliases), 16.19 (ACP legacy), 16.20 (feature flag), 16.21 (hdc),
  16.22 (max tokens), 16.26 (migration)

---

## Effort Summary

| Effort | Count | Tasks |
|--------|-------|-------|
| Small  | 12    | 16.2, 16.3, 16.7, 16.8, 16.9, 16.12, 16.13, 16.16, 16.18, 16.19, 16.20, 16.21, 16.22, 16.25 |
| Medium | 10    | 16.1, 16.4, 16.5, 16.6, 16.11, 16.14, 16.15, 16.17, 16.23, 16.24, 16.26 |
| Large  | 1     | 16.10 |

**Total: 26 tasks** (3 removed as already fixed from impl plan's 33).
