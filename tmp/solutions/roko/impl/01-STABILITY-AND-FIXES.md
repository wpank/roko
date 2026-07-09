# 01 - Stability and Fixes Implementation Plan

All issues sourced from subsystem audits (docs 05, 11, 15-20) on branch `wp-arch2`,
verified against live source code. Every task references specific files and acceptance
criteria. Organized by priority tier (P0 first).

---

## Summary Table

| ID | Title | Priority | Category | Effort | Depends On |
|---|---|---|---|---|---|
| **P0 -- Crashes, Security, Data Loss** | | | | | |
| S01 | Fix `roko config mcp` unreachable panic | P0 | bug-fix | S | -- |
| S02 | Move share routes inside auth middleware | P0 | security | S | -- |
| S03 | Auto-provision auth on cloud deploy | P0 | security | S | -- |
| S04 | Add secret scrubbing to CLI Gist share path | P0 | security | S | -- |
| S05 | Fix `acknowledge_public_risk` bypass | P0 | security | S | -- |
| S06 | Remove `unsafe set_var` for --provider | P0 | bug-fix | S | -- |
| S07 | Fix stub gate verdicts giving false PASS | P0 | correctness | S | -- |
| S08 | Fix dual episode writes in `roko run` | P0 | correctness | S | -- |
| S09 | Normalize `[[gate]]` vs `[gates]` config schema | P0 | correctness | M | -- |
| S10 | Fix `roko init` emitting wrong gate format | P0 | correctness | S | S09 |
| **P1 -- Features Broken or Silently Wrong** | | | | | |
| S11 | Wire CascadeRouter to live callers | P1 | anti-pattern | M | -- |
| S12 | Wire feedback recording to `roko chat` | P1 | anti-pattern | S | -- |
| S13 | Wire feedback recording to ACP pipeline | P1 | anti-pattern | M | -- |
| S14 | Forward streaming events to chat TUI | P1 | bug-fix | M | -- |
| S15 | Wire `build_repo_context` into plan generate | P1 | anti-pattern | S | -- |
| S16 | Inject validation diagnostics into plan regenerate | P1 | bug-fix | M | -- |
| S17 | Wire BudgetGuardrail to live paths | P1 | stability | M | -- |
| S18 | Wire ContextTier into dispatch for small models | P1 | correctness | M | -- |
| S19 | Wire BudgetPredictor to prompt assembly | P1 | anti-pattern | M | S18 |
| S20 | Wire `roko chat` and dispatch_direct through PromptAssemblyService | P1 | anti-pattern | M | -- |
| S21 | Consolidate 4 stream-json parsing copies | P1 | anti-pattern | M | -- |
| S22 | Wire runner v2 CascadeRouter observations | P1 | anti-pattern | S | S11 |
| S23 | Wire runner v2 AdaptiveThreshold observations | P1 | anti-pattern | S | -- |
| S24 | Wire runner v2 episode logging | P1 | anti-pattern | S | -- |
| S25 | Wire runner v2 section effectiveness updates | P1 | anti-pattern | S | -- |
| S26 | Wire runner v2 efficiency event recording | P1 | anti-pattern | S | -- |
| S27 | Wire section effectiveness into PromptAssemblyService | P1 | anti-pattern | S | S25 |
| S28 | Wire gate failure classification to retry/replan routing | P1 | correctness | M | -- |
| S29 | Replace ACP direct subprocess spawns with provider system | P1 | anti-pattern | M | -- |
| S30 | Fix ACP gate rung ordering (clippy before test) | P1 | correctness | S | -- |
| S31 | Wire gate feedback_for_agent into GateService | P1 | anti-pattern | M | -- |
| S32 | Fix model showing "-" in TUI for runner v2 | P1 | bug-fix | S | -- |
| S33 | Remove direct env var reads for API keys | P1 | anti-pattern | M | -- |
| S34 | Fix `signals.jsonl` dead path (writes to `engrams.jsonl`) | P1 | bug-fix | S | -- |
| **P2 -- Correctness, Quality, Stability** | | | | | |
| S35 | Unify model selection paths (auth_detect vs ServiceFactory) | P2 | anti-pattern | L | -- |
| S36 | Normalize model aliases at load time | P2 | correctness | S | -- |
| S37 | Export `rung_for_gate_name` from roko-gate | P2 | anti-pattern | S | -- |
| S38 | Add TaskScheduler state to WorkflowEngine checkpoint | P2 | stability | M | -- |
| S39 | Add `thinking_tokens` to UsageObservation | P2 | correctness | M | -- |
| S40 | Fix singleton rate limiter across providers | P2 | correctness | M | -- |
| S41 | Add retry logic for transient provider failures | P2 | stability | M | -- |
| S42 | Wire provider health circuit breaker to CascadeRouter | P2 | stability | M | S11 |
| S43 | Wire SPC alerts drain to runtime consumers | P2 | anti-pattern | M | -- |
| S44 | Wire Hotelling T-squared to runtime gate pipeline | P2 | anti-pattern | M | -- |
| S45 | Wire domain profiles to AdaptiveThresholds | P2 | anti-pattern | S | -- |
| S46 | Wire conductor bandit to live retry paths | P2 | anti-pattern | M | -- |
| S47 | Wire anomaly detector to live paths | P2 | anti-pattern | M | -- |
| S48 | Wire regression detection alerting path | P2 | anti-pattern | S | -- |
| S49 | Add end-of-run summary to plan runner | P2 | stability | M | -- |
| S50 | Expose `max_concurrent_tasks` from config | P2 | stability | M | -- |
| S51 | Make `dangerously_skip_permissions` configurable | P2 | security | M | -- |
| S52 | Replace ACP inline review prompts with templates | P2 | anti-pattern | M | -- |
| S53 | Fix OpenAI-compat provider quirks fragmentation | P2 | anti-pattern | M | -- |
| S54 | Make tool loop max iterations configurable | P2 | correctness | S | -- |
| S55 | Unify StateHub types between serve and CLI | P2 | anti-pattern | L | -- |
| S56 | Wire dream consolidation trigger | P2 | anti-pattern | M | -- |
| S57 | Wire knowledge candidate ingestion post-run | P2 | anti-pattern | S | -- |
| S58 | Fix `--share` without `--serve` producing dead URL | P2 | bug-fix | S | -- |
| S59 | Add `--dry-run` to `roko plan run` | P2 | stability | M | -- |
| S60 | Make workspace map cap proportional to context tier | P2 | correctness | S | S18 |
| S61 | Wire knowledge store to CascadeRouter model selection | P2 | anti-pattern | M | S11 |
| S62 | Fix GatePipeline / ComposedGatePipeline duplication | P2 | anti-pattern | M | -- |
| S63 | Wire ProcessRewardModel to orchestrator | P2 | anti-pattern | M | -- |
| S64 | Wire AcceptanceContract to gate pipeline | P2 | anti-pattern | M | -- |
| S65 | Add anti-pattern checks as pre-gate step | P2 | correctness | M | -- |
| S66 | Wire VerdictPublisher to all gate dispatch paths | P2 | anti-pattern | S | -- |
| S67 | Add gate budget tracking for LLM judge calls | P2 | correctness | M | -- |
| S68 | Wire StagingBuffer lightweight promotion | P2 | anti-pattern | S | -- |
| S69 | Add cross-session cost aggregation | P2 | correctness | M | -- |
| S70 | Add content-type-aware token counting ratios | P2 | correctness | S | -- |
| S71 | Make knowledge confidence thresholds tier-dependent | P2 | correctness | S | S18 |
| S72 | Wire conversation compaction to `roko chat` | P2 | stability | S | -- |
| S73 | Add prompt caching metrics to ModelCallService | P2 | stability | M | -- |
| S74 | Add disk pressure monitoring pre-dispatch | P2 | stability | M | -- |
| S75 | Add agent execution time monitoring | P2 | stability | M | -- |
| S76 | Fix WorkflowEngine missing worktree integration | P2 | stability | L | -- |
| S77 | Unify two PipelineState state machines | P2 | anti-pattern | L | -- |
| S78 | Consolidate 4 agent dispatch implementations | P2 | anti-pattern | L | -- |

**Totals**: 78 tasks. P0: 10, P1: 24, P2: 44.
Effort: S: 30, M: 37, L: 11.

---

## P0 -- Crashes, Security, Data Loss

These must be fixed before any deployment or user-facing work. Each represents
a crash, security vulnerability, or silent data corruption.

---

### TASK-S01: Fix `roko config mcp` unreachable panic
**Priority**: P0
**Category**: bug-fix
**Files**: `crates/roko-cli/src/commands/config_cmd.rs`
**Problem**: `ConfigCmd::Mcp` arm falls through to `unreachable!()` which panics at runtime. Running `roko config mcp list` crashes the process. Verified as HOLLOW-1 in doc 05. The MCP config subcommand was added to the CLI parser (clap derives the variant) but never wired to a handler.
**Fix**:
1. Add a match arm for `ConfigCmd::Mcp` in the config command dispatcher
2. Read `config.agent.mcp_config` and `config.agent.mcp_servers` from the loaded config
3. For `mcp list`: format and display configured MCP servers (name, command, enabled)
4. For `mcp add/remove/test`: wire to existing MCP config management functions
5. Return a helpful "no MCP servers configured" message when the list is empty
**Acceptance**: `roko config mcp list` runs without panic. With no MCP configured, prints "No MCP servers configured." With MCP configured, prints the server list.
**Depends on**: --
**Effort**: S

---

### TASK-S02: Move share routes inside auth middleware
**Priority**: P0
**Category**: security
**Files**: `crates/roko-serve/src/routes/shared_runs.rs`, `crates/roko-serve/src/lib.rs` (router construction)
**Problem**: `POST /api/runs/{id}/share` is mounted OUTSIDE the auth middleware layer. Any caller -- authenticated or not -- can create share links for any run, exposing agent transcripts (which may contain repository code, API keys, internal documentation). Verified as CRITICAL security finding in doc 05.
**Fix**:
1. In the router construction (likely `lib.rs` or `routes/mod.rs`), move the share routes from the public router to the protected router that sits behind auth middleware
2. Ensure `GET /api/runs/{id}/shared` (read-only viewing of already-shared links) remains public if needed for link recipients
3. Add integration test: unauthenticated POST to `/api/runs/test/share` returns 401
**Acceptance**: `curl -X POST localhost:6677/api/runs/test/share` without auth header returns 401. With valid auth, returns 200.
**Depends on**: --
**Effort**: S

---

### TASK-S03: Auto-provision auth on cloud deploy
**Priority**: P0
**Category**: security
**Files**: `crates/roko-cli/src/commands/deploy.rs` (or equivalent deploy handlers)
**Problem**: `roko deploy railway` does not generate or require an API key. The deployed server is immediately accessible with no authentication on a public URL. Verified as HIGH security finding in doc 05.
**Fix**:
1. In the deploy command handler, generate a random 32-byte hex API key using `rand::thread_rng()`
2. Set `api_auth.enabled = true` in the deployment config
3. Inject the API key as an environment variable (`ROKO_API_KEY`) in the deployment
4. Print the generated API key to stdout with a warning: "Save this API key. It will not be shown again."
5. Apply to all deploy targets: railway, fly, docker
**Acceptance**: `roko deploy railway` output includes an API key. The deployed server rejects unauthenticated requests with 401.
**Depends on**: --
**Effort**: S

---

### TASK-S04: Add secret scrubbing to CLI Gist share path
**Priority**: P0
**Category**: security
**Files**: `crates/roko-cli/src/share.rs`
**Problem**: `roko run --share` creates a GitHub Gist with the raw agent transcript. The HTTP share path (`/api/runs/{id}/share`) applies secret scrubbing before sharing. The CLI path does not. API keys, tokens, and other secrets embedded in agent output are uploaded to GitHub as-is. Verified as LOW security finding in doc 05.
**Fix**:
1. Import or replicate the `scrub_secrets()` function used by the HTTP share path
2. Apply scrubbing to the transcript content before GitHub Gist upload
3. Scrub patterns: API key formats (`sk-*`, `key-*`, `ghp_*`, `gho_*`), bearer tokens, base64-encoded JWT patterns, environment variable values matching `*_KEY`, `*_SECRET`, `*_TOKEN`
4. Replace matches with `[REDACTED]`
**Acceptance**: Generate a transcript containing `ANTHROPIC_API_KEY=sk-ant-test123`. Run `roko run --share`. The uploaded Gist contains `[REDACTED]` instead of the key value.
**Depends on**: --
**Effort**: S

---

### TASK-S05: Fix `acknowledge_public_risk` bypass
**Priority**: P0
**Category**: security
**Files**: `crates/roko-serve/src/lib.rs` (or auth middleware configuration)
**Problem**: Setting `acknowledge_public_risk = true` in config bypasses the terminal-displayed auth warning without checking whether `api_auth.enabled` is actually true. A user can acknowledge the risk warning and still run without auth, believing they have addressed the security concern. Verified as MEDIUM security finding in doc 05.
**Fix**:
1. When `acknowledge_public_risk = true` AND `api_auth.enabled = false`, log a WARNING at startup: "Public risk acknowledged but auth is not enabled. Server is accessible without authentication."
2. When binding to `0.0.0.0` (not localhost), require either `api_auth.enabled = true` OR `acknowledge_public_risk = true` -- the latter should display an explicit "NO AUTH" banner
3. Remove the ability for `acknowledge_public_risk` to suppress the warning when auth is off
**Acceptance**: Start `roko serve` with `acknowledge_public_risk = true` and `api_auth.enabled = false`. A WARNING log line appears. Start with neither set on `0.0.0.0` -- startup refuses or prints a prominent warning.
**Depends on**: --
**Effort**: S

---

### TASK-S06: Remove `unsafe set_var` for --provider
**Priority**: P0
**Category**: bug-fix
**Files**: `crates/roko-cli/src/commands/util.rs` (line ~271 area), `crates/roko-cli/src/auth_detect.rs`
**Problem**: The `--provider` flag uses `unsafe { std::env::set_var() }` to inject the override. This is unsound in multi-threaded contexts (UB per Rust safety model since 1.66) and fragile. Verified in doc 11 section 1.1.
**Fix**:
1. Remove the `unsafe { std::env::set_var() }` call
2. Add a `provider_override: Option<String>` field to the config/context struct passed through dispatch
3. In model resolution, check `provider_override` before environment variables
4. Thread the override through `ServiceFactory::build()` or `resolve_effective_model()`
**Acceptance**: `roko run --provider cerebras "hello"` uses Cerebras without calling `set_var`. `cargo clippy` shows no `unsafe` blocks in CLI code related to env vars.
**Depends on**: --
**Effort**: S

---

### TASK-S07: Fix stub gate verdicts giving false PASS
**Priority**: P0
**Category**: correctness
**Files**: `crates/roko-gate/src/rung_dispatch.rs` (lines 132-138)
**Problem**: When rung inputs are missing (no SymbolManifest, no FactCheckOracle, no JudgeOracle), gates return stub verdicts that PASS. A plan at Complex complexity reports rungs 3-6 all passed when they were never executed. This is the AP-1 anti-pattern (silent-pass stubs) verified in doc 20 I-2.
**Fix**:
1. Change `stub_verdict()` to return a SKIP verdict instead of PASS:
   ```rust
   fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
       let message = format!("stub gate; {}", detail.into());
       Verdict::skip(gate, message)
   }
   ```
2. If `Verdict::skip` constructor does not exist, add one that sets `skipped: true` and `skip_reason: Some(message)` and `passed: false`
3. Update any callers that check `verdict.passed` to also handle `verdict.skipped`
4. Ensure TUI and episode recording distinguish between "passed", "failed", and "skipped"
**Acceptance**: Run a plan with `max_gate_rung = 6` but no judge oracle configured. Rungs 3-6 show as "skipped" (not "passed") in episodes and TUI.
**Depends on**: --
**Effort**: S

---

### TASK-S08: Fix dual episode writes in `roko run`
**Priority**: P0
**Category**: correctness
**Files**: `crates/roko-cli/src/run.rs`
**Problem**: `roko run` writes episodes twice: once via a direct `append_episode_log()` call, and again via `LearningRuntime::record_completed_run()` which internally appends to its own episode log. This produces duplicate records in different files (`.roko/episodes.jsonl` at root vs `.roko/learn/episodes.jsonl`). Verified in doc 18 I-07.
**Fix**:
1. Remove the direct `append_episode_log()` call from `run.rs`
2. Let `LearningRuntime` be the single writer
3. Verify that all episode readers use `LearningPaths.episodes_jsonl` (the learn path)
4. If backward compatibility is needed, add a symlink from the root path to the learn path
**Acceptance**: Run `roko run "hello"`. Count entries in both episode files. Only one file has new entries. No duplicate episode IDs across any episode files.
**Depends on**: --
**Effort**: S

---

### TASK-S09: Normalize `[[gate]]` vs `[gates]` config schema
**Priority**: P0
**Category**: correctness
**Files**: `crates/roko-core/src/config/mod.rs` (or wherever `RokoConfig::from_toml()` lives)
**Problem**: `roko init` writes `[[gate]]` arrays. `RokoConfig::from_toml()` reads `[gates]` table. The mismatch means gates generated by `roko init` are silently discarded by the runtime -- no error, no warning, just default behavior. Verified as AP-6 in doc 05, and AP-3 in doc 17.
**Fix**:
1. In `RokoConfig::from_toml()`, add parsing for `[[gate]]` array format in addition to `[gates]` table format
2. Normalize both to the same internal `GatesConfig` struct
3. When both `[[gate]]` and `[gates]` are present, emit a warning and prefer `[gates]` (newer format)
4. When only `[[gate]]` is present, parse it and convert to internal format
5. Add a unit test that parses a TOML file with `[[gate]]` entries and verifies they appear in the resulting config
**Acceptance**: Write a `roko.toml` with `[[gate]]` entries. Run `roko plan run`. Gate execution respects the configured gates. `roko config show` displays the gates.
**Depends on**: --
**Effort**: M

---

### TASK-S10: Fix `roko init` emitting wrong gate format
**Priority**: P0
**Category**: correctness
**Files**: `crates/roko-cli/src/commands/init.rs`
**Problem**: `roko init` generates `[[gate]]` array format which the runtime's `[gates]` parser discards. Even after S09 adds backward compatibility, new projects should emit the canonical format. Verified in doc 05 AP-6.
**Fix**:
1. Change the `roko init` template to emit `[gates]` format instead of `[[gate]]` arrays:
   ```toml
   [gates]
   enabled = ["compile", "clippy", "test"]
   ```
2. Remove the `[[gate]]` template generation code
3. Update init tests to verify the new format
**Acceptance**: `roko init` creates a `roko.toml` with `[gates]` section. `RokoConfig::from_toml()` parses it successfully.
**Depends on**: S09
**Effort**: S

---

## P1 -- Features Broken or Silently Wrong

These represent features that are claimed to work but either do nothing, produce
wrong results, or bypass critical systems. They degrade the system's value without
crashing.

---

### TASK-S11: Wire CascadeRouter to live callers
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/model_selection.rs`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/chat_session.rs`
**Problem**: CascadeRouter is a sophisticated LinUCB contextual bandit with 4-stage routing, persistence, cost spike detection, and knowledge-informed routing. It has zero live callers -- `resolve_effective_model()` accepts `Option<&CascadeRouter>` but every caller passes `None`. Verified as ISS-01 in doc 19.
**Fix**:
1. At startup in `roko run`, `roko plan run`, and `roko chat`, load CascadeRouter from `.roko/learn/cascade-router.json` (or create new)
2. Pass the loaded router to `resolve_effective_model()` instead of `None`
3. After each model call, call `cascade_router.observe(model, role, success, cost, latency)`
4. Persist router state on graceful shutdown and periodic flush
**Acceptance**: Run `roko run "hello"` twice. After the second run, `.roko/learn/cascade-router.json` has `observations > 0` with at least one model entry.
**Depends on**: --
**Effort**: M

---

### TASK-S12: Wire feedback recording to `roko chat`
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/chat_session.rs`, `crates/roko-learn/src/feedback_service.rs`
**Problem**: `roko chat` makes model calls but records no episodes, no routing observations, no cost tracking, and no learning signals. Every chat session is a lost learning opportunity. Verified as I-01 in doc 18.
**Fix**:
1. Instantiate `FeedbackService::from_roko_dir_with_episodes()` in chat session setup
2. Emit `FeedbackEvent::ModelCall` after each model response with model, tokens, latency, success
3. Emit `FeedbackEvent::WorkflowComplete` when the chat session ends (on `/quit` or Ctrl-D)
4. Optionally attach CascadeRouter to FeedbackService for routing observations
**Acceptance**: Start `roko chat`, send one message, quit. `.roko/learn/efficiency.jsonl` has a new entry. `.roko/episodes.jsonl` has a new entry.
**Depends on**: --
**Effort**: S

---

### TASK-S13: Wire feedback recording to ACP pipeline
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-acp/src/runner.rs`, `crates/roko-acp/src/pipeline.rs`
**Problem**: ACP records only adaptive gate thresholds for rungs 0/1/2. No episodes, no routing, no cost tracking. Editor-integrated usage (VS Code, etc.) produces zero learning signal despite being likely the highest-frequency interaction. Verified as I-02 in doc 18.
**Fix**:
1. Create FeedbackService in ACP pipeline initialization (in `pipeline.rs` or `runner.rs`)
2. Emit `FeedbackEvent::ModelCall` from ACP model dispatch
3. Emit `FeedbackEvent::GateResult` from ACP gate pipeline (instead of only writing thresholds)
4. Thread FeedbackService through the ACP pipeline to all dispatch points
**Acceptance**: Run an ACP session, dispatch a model call, run gates. `.roko/episodes.jsonl` has a new ACP entry. `.roko/learn/cascade-router.json` has increased observations.
**Depends on**: --
**Effort**: M

---

### TASK-S14: Forward streaming events to chat TUI
**Priority**: P1
**Category**: bug-fix
**Files**: `crates/roko-cli/src/chat_inline.rs`
**Problem**: The chat inline handler creates a streaming channel, spawns the agent, then drains events with `while let Some(_event) = event_rx.recv().await {}`. The underscore-prefixed binding discards every streaming event. The TUI shows a spinner until the entire response is complete. Verified as AP-7 in doc 05.
**Fix**:
1. Replace `while let Some(_event) = event_rx.recv().await {}` with actual event processing
2. Map streaming events to `DashboardEvent` variants (or directly to ratatui viewport updates)
3. For text content events: append to the streaming text viewport
4. For tool call events: display tool name and truncated arguments
5. For completion events: finalize the viewport and show cost/token stats
**Acceptance**: `roko chat` displays streaming text character-by-character (or line-by-line) during agent response, not just a spinner followed by a text dump.
**Depends on**: --
**Effort**: M

---

### TASK-S15: Wire `build_repo_context` into plan generate
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/repo_context.rs`
**Problem**: `build_repo_context()` is called from `prd draft new` to give the drafting agent awareness of the repository structure. It is NOT called from `plan generate`, `plan regenerate`, or `prd plan`. Plans are generated without repository awareness, causing agents to propose duplicate crates, reference non-existent modules, and create conflicting file structures. Verified as AP-8 in doc 05.
**Fix**:
1. In `plan generate` handler: call `build_repo_context()` with task keywords before agent dispatch
2. In `plan regenerate` handler: same
3. In `prd plan` handler: same
4. Inject the context into the agent's system prompt or user prompt as a "Repository Structure" section
**Acceptance**: Run `roko plan generate` on a workspace with 18 crates. The generated plan references existing crate names and does not propose new crates that duplicate existing functionality.
**Depends on**: --
**Effort**: S

---

### TASK-S16: Inject validation diagnostics into plan regenerate
**Priority**: P1
**Category**: bug-fix
**Files**: `crates/roko-cli/src/commands/plan.rs`
**Problem**: `plan regenerate` validates after generation but does NOT inject diagnostics into the regeneration prompt. The validation-feedback loop is missing. Verified as HOLLOW-3 in doc 05.
**Fix**:
1. After initial agent generation, validate the output
2. If validation fails, construct a retry prompt that includes:
   - The original generation prompt
   - The generated output (for reference)
   - The specific validation errors, formatted as a list
   - An instruction: "Fix the following validation errors in the plan"
3. Re-run the agent with the error-enriched prompt
4. Repeat up to 2 iterations (configurable)
5. If still failing after retries, output the plan with warnings
**Acceptance**: Create a plan that references a non-existent file. Run `roko plan regenerate`. The regenerated plan fixes the file reference (or reports after 2 failed retries).
**Depends on**: --
**Effort**: M

---

### TASK-S17: Wire BudgetGuardrail to live paths
**Priority**: P1
**Category**: stability
**Files**: `crates/roko-learn/src/budget.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/chat_session.rs`
**Problem**: `BudgetGuardrail` implements 3-scope budget limits (per-task, per-session, per-day) with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It is never instantiated or checked in any live path. A runaway agent can spend unlimited money. Verified as I-05 in doc 18 and ISS-06 in doc 19.
**Fix**:
1. Load budget config from `roko.toml` (`[budget]` section)
2. Instantiate BudgetGuardrail at session start in `roko run`, `roko chat`, and `roko plan run`
3. Check before each model dispatch: call `guardrail.check(estimated_cost)`
4. On `RouteToCheaper`: switch to fallback model
5. On `Block`: abort with clear error message showing cumulative spend
6. Set sensible defaults: per-turn $0.50, per-session $10.00, per-plan $100.00
**Acceptance**: Set `budget.max_session_usd = 0.01` in roko.toml. Run `roko run "write a long essay"`. After the budget is exceeded, the run stops with a budget exceeded message.
**Depends on**: --
**Effort**: M

---

### TASK-S18: Wire ContextTier into dispatch for small models
**Priority**: P1
**Category**: correctness
**Files**: `crates/roko-compose/src/context_provider.rs`, `crates/roko-compose/src/prompt_assembly_service.rs`, `crates/roko-cli/src/run.rs`
**Problem**: `ContextTier` defines correct budgets (4K/12K/24K tokens) and `is_local_model()` correctly identifies small models. But `dispatch_agent_with()` never calls `ContextTier::from_task_and_model()`. Small models receive prompts designed for 200K-context models, causing silent truncation or errors. Verified as ISS-01 in doc 16 (Critical).
**Fix**:
1. In the dispatch path (wherever the prompt is assembled before agent call), resolve the model to a `ContextTier`
2. Call `ContextTier::from_task_and_model(model_slug)` or equivalent
3. Use the tier's token budget as the cap for `PromptAssemblyService::with_token_budget()`
4. For Surgical tier (4K): include only identity + role + task + constraints
5. For Focused tier (12K): add conventions and limited context
6. For Full tier (24K+): include all sections
**Acceptance**: Set `default_model = "ollama/gemma4"` in roko.toml. Run `roko run "hello"`. The assembled prompt is under 4K tokens (Surgical tier). System log shows "ContextTier::Surgical" or equivalent.
**Depends on**: --
**Effort**: M

---

### TASK-S19: Wire BudgetPredictor to prompt assembly
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-compose/src/budget_predictor.rs`, `crates/roko-compose/src/prompt_assembly_service.rs`
**Problem**: `BudgetPredictor` is fully implemented (679 LOC) with EMA-based prediction, failure inflation, partial-match fallback, and persistence. No caller invokes `predictor.predict()` before assembly. Token budgets are static constants. Verified as ISS-02 in doc 16.
**Fix**:
1. Load `BudgetPredictor` from `.roko/learn/budget-predictions.json` (or create new) at startup
2. Before prompt assembly, call `predictor.predict(role, task_id)` to get predicted budget
3. Pass predicted budget to `PromptAssemblyService::with_token_budget()`
4. After task completion, call `predictor.observe(role, task_id, actual_tokens, success)` to update
5. Persist predictor state on flush
**Acceptance**: Run the same task type 5 times. By run 5, the predicted budget converges toward actual usage (within 20% of actual tokens used).
**Depends on**: S18
**Effort**: M

---

### TASK-S20: Wire `roko chat` and dispatch_direct through PromptAssemblyService
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/dispatch_direct.rs`, `crates/roko-cli/src/chat_session.rs`
**Problem**: `roko chat` and `roko "prompt"` (dispatch_direct) send bare prompts to the agent with zero system prompt. No role identity, no conventions, no knowledge injection, no anti-patterns, no playbooks. Verified as ISS-03 in doc 16.
**Fix**:
1. In `chat_session.rs`: before sending to agent, call `PromptAssemblyService::assemble()` with role "assistant" and the user's prompt
2. Pass the assembled system prompt via `--append-system-prompt` to Claude CLI
3. In `dispatch_direct.rs` (if still used): same treatment
4. Use a lightweight assembly (skip heavy sections like PRD context) for chat latency
**Acceptance**: `roko chat` -- type "what project am I working on?". The agent knows the project name and crate structure (from the assembled system prompt). Previously it would have no context.
**Depends on**: --
**Effort**: M

---

### TASK-S21: Consolidate 4 stream-json parsing copies
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/dispatch_direct.rs`, `crates/roko-agent/src/translate/mod.rs`, `crates/roko-cli/src/chat.rs`
**Problem**: The stream-json parsing logic is duplicated 4 times with inconsistent output formats. All 4 copies independently implement the same 4096-byte truncation with char_boundary checks. The canonical parser `parse_stream_line()` in `provider/claude_cli/stream.rs` already exists. Verified as ISS-08 in doc 19.
**Fix**:
1. Replace inline parsing in `translate/mod.rs:extract_text()` with calls to `parse_stream_line()`
2. Replace inline parsing in `translate/mod.rs:extract_tool_outputs()` with calls to `parse_stream_line()`
3. Replace inline parsing in `chat.rs:extract_clean_text()` with calls to `parse_stream_line()`
4. Keep `dispatch_direct.rs` as-is (already behind `legacy-orchestrate` feature gate, will be removed)
5. Add tests that all three replacements produce identical output to the canonical parser for sample inputs
**Acceptance**: `grep -rn 'serde_json::from_str.*result' crates/roko-cli/src/chat.rs crates/roko-agent/src/translate/mod.rs` returns zero matches (all parsing delegated to canonical parser).
**Depends on**: --
**Effort**: M

---

### TASK-S22: Wire runner v2 CascadeRouter observations
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: Runner v2 imports `CascadeRouter` but never calls `cascade_router.observe()` after task completion. The router cannot learn from plan execution outcomes. Verified in doc 05 section 5 gap list (Low complexity).
**Fix**:
1. In `event_loop.rs`, after a task completes (success or failure), construct a routing observation
2. Call `cascade_router.observe(model, role, success, cost, latency)` with the task's actual values
3. Persist router state during periodic flush
**Acceptance**: Run `roko plan run` on a 3-task plan. After completion, `.roko/learn/cascade-router.json` shows `observations >= 3`.
**Depends on**: S11
**Effort**: S

---

### TASK-S23: Wire runner v2 AdaptiveThreshold observations
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: Runner v2 does not call `AdaptiveThresholds::observe()` after gate execution. Gate pass rates are not tracked, and the adaptive skip logic cannot learn. Verified in doc 05 section 5 gap list (Low complexity).
**Fix**:
1. After each gate verdict in the event loop, call `thresholds.observe(rung, passed)`
2. Before each gate dispatch, call `thresholds.should_skip_rung(rung)` to check if adaptive skip applies
3. Record skip decisions in the episode for debugging
**Acceptance**: Run `roko plan run` on a plan with gates. After completion, `.roko/learn/gate-thresholds.json` has per-rung stats with `total_observations > 0`.
**Depends on**: --
**Effort**: S

---

### TASK-S24: Wire runner v2 episode logging
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: Runner v2 does not call `record_episode()` on task completion. Episodes are not written from plan execution, breaking learning continuity. Verified in doc 05 section 5 gap list (Low complexity).
**Fix**:
1. On task completion in the event loop, construct an `Episode` struct with task_id, model, success, usage, gate verdicts, and reflection
2. Write to `.roko/episodes.jsonl` via `EpisodeSink` or `LearningRuntime::record_completed_run()`
3. Include gate results, token counts, cost, and timing
**Acceptance**: Run `roko plan run` on a 3-task plan. `.roko/episodes.jsonl` has 3 new entries (one per task).
**Depends on**: --
**Effort**: S

---

### TASK-S25: Wire runner v2 section effectiveness updates
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: `SectionEffectivenessRegistry` is not updated from runner v2. The registry tracks lift per prompt section but never receives observations from plan execution. Verified in doc 05 section 5 gap list (Low complexity).
**Fix**:
1. On task completion, call `section_effectiveness.observe(sections_included, gate_passed)`
2. The sections included can be derived from the `PromptAssemblyService` output (which sections were active)
3. Persist registry during periodic flush
**Acceptance**: Run a plan. `.roko/learn/section-effects.json` has section entries with non-zero observation counts.
**Depends on**: --
**Effort**: S

---

### TASK-S26: Wire runner v2 efficiency event recording
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: Efficiency events with 30+ fields are not emitted from runner v2 on agent completion. Verified in doc 05 section 5 gap list (Medium complexity).
**Fix**:
1. After each agent completes in the event loop, construct an `AgentEfficiencyEvent` with timing, token counts, tool calls, model, and task metadata
2. Write to `.roko/learn/efficiency.jsonl` via the efficiency sink
3. Flush immediately after write (avoid the earlier dogfood bug of accumulating without flush)
**Acceptance**: Run `roko plan run` on a 3-task plan. `.roko/learn/efficiency.jsonl` has 3 new entries with non-zero token counts.
**Depends on**: --
**Effort**: S

---

### TASK-S27: Wire section effectiveness into PromptAssemblyService
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-compose/src/prompt_assembly_service.rs`, `crates/roko-learn/src/section_effect.rs`
**Problem**: Section effectiveness tracking works and is updated (after S25), but `PromptAssemblyService` does not read the weights during prompt assembly. Data is collected but never acted upon. Verified as I-09 in doc 18.
**Fix**:
1. `PromptAssemblyService` already has a `section_weights` field
2. On construction, load section effectiveness from `.roko/learn/section-effects.json`
3. Apply weights during section budget allocation: multiply each section's budget by its effectiveness score
4. Sections with score < 0.1: exclude entirely
5. Sections with score 0.1-0.5: reduce budget proportionally
6. Log when a section is deprioritized due to negative effectiveness
**Acceptance**: After 10+ runs, a section with consistently negative lift (e.g., PRD context in fast tasks) receives less budget allocation than a section with positive lift.
**Depends on**: S25
**Effort**: S

---

### TASK-S28: Wire gate failure classification to retry/replan routing
**Priority**: P1
**Category**: correctness
**Files**: `crates/roko-gate/src/compile_errors.rs`, `crates/roko-runtime/src/pipeline_state.rs` (or `crates/roko-cli/src/runner/event_loop.rs`)
**Problem**: The compile error classification system computes failure actions (Retry, NeedsReplan, Blocked, NeedsHuman) but the action is rendered and discarded. The orchestrator always retries regardless. Verified as I-10 in doc 20.
**Fix**:
1. After gate failure, call `classify_gate_failure(&output)` to get the recommended action
2. Route based on the action:
   - `Retry`: continue with existing retry logic (feedback to agent)
   - `NeedsReplan`: emit a replan event, invoke strategist agent
   - `Blocked`: pause the task, mark as blocked, log the reason
   - `NeedsHuman`: pause the task, emit a notification, set status to "needs-human"
3. Expose the classification in the episode record
**Acceptance**: Create a task with an architectural type error (e.g., wrong trait impl). Gate failure classifies as `NeedsReplan`. The runner attempts a replan instead of blind retry.
**Depends on**: --
**Effort**: M

---

### TASK-S29: Replace ACP direct subprocess spawns with provider system
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-acp/src/runner.rs`, `crates/roko-acp/src/bridge_events.rs`
**Problem**: Two ACP paths bypass the provider system entirely: `run_claude_cli()` spawns bare subprocess with no model flag, no streaming, no system prompt, no feedback; `run_claude_cognitive_task()` builds its own subprocess. Verified as ISS-04 in doc 19.
**Fix**:
1. Replace `run_claude_cli()` calls with `create_agent_for_model()` via the provider adapter system
2. Replace `run_claude_cognitive_task()` calls similarly
3. Replace `run_openai_compat_cognitive_task()` with provider adapter calls
4. Pass model, system prompt, and feedback service through the provider adapter
5. Cost tracking and credential management now happen automatically via the adapter
**Acceptance**: ACP model calls appear in `.roko/learn/efficiency.jsonl`. Cost tracking shows non-zero values for ACP sessions.
**Depends on**: --
**Effort**: M

---

### TASK-S30: Fix ACP gate rung ordering (clippy before test)
**Priority**: P1
**Category**: correctness
**Files**: `crates/roko-acp/src/runner.rs` (`run_gates()` function)
**Problem**: ACP runs gates in the order: compile -> test -> clippy. The canonical rung order is: compile (0) -> clippy (1) -> test (2). Running test before clippy wastes 5-15 minutes when a trivial lint failure exists. Verified as I-5 in doc 20.
**Fix**:
1. Replace the hardcoded gate order in ACP's `run_gates()` with a call to `GateService` which orders by rung index
2. Or if using GateService is not feasible, reorder the hardcoded list to: compile, clippy, test
3. Add short-circuit: if clippy fails, skip test
**Acceptance**: In an ACP session, introduce a clippy warning. Gate output shows clippy running before test. Test is skipped when clippy fails.
**Depends on**: --
**Effort**: S

---

### TASK-S31: Wire gate feedback_for_agent into GateService
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/feedback.rs`, `crates/roko-gate/src/gate_service.rs`
**Problem**: `feedback_for_agent()` is called only from `orchestrate.rs` (dead code). The `roko run` and ACP paths run gates but never parse output into structured feedback for agent retry. Agents retry blind or get raw stderr dumped into context. Verified as I-4 in doc 20.
**Fix**:
1. In `GateService`, after running gates, call `feedback_for_agent()` on any failed verdicts
2. Include the structured feedback in `GateReport` (add a `feedback: Option<StructuredFeedback>` field)
3. When the pipeline state machine handles `GatesFailed`, extract feedback from the report and inject into the retry prompt
**Acceptance**: Create a task that fails compile. The retry prompt includes structured feedback (specific errors, not raw stderr). Agent output shows it addressing the specific feedback.
**Depends on**: --
**Effort**: M

---

### TASK-S32: Fix model showing "-" in TUI for runner v2
**Priority**: P1
**Category**: bug-fix
**Files**: `crates/roko-cli/src/runner/event_loop.rs` (or TUI bridge)
**Problem**: Runner v2 passes empty string for model in TUI events, causing the dashboard to show "-" instead of the model name. Users see no model info during plan execution. Verified in doc 05 section 7.2.
**Fix**:
1. In the event loop, when dispatching an agent, include the resolved model name in the `DashboardEvent::TaskStarted` (or equivalent)
2. When the agent responds with usage, include model in the `DashboardEvent::TaskProgress`
3. Ensure the model field is populated from the dispatch context, not from agent output (which may not include it)
**Acceptance**: Run `roko plan run` with dashboard visible. Each task shows its model name (e.g., "claude-sonnet-4") instead of "-".
**Depends on**: --
**Effort**: S

---

### TASK-S33: Remove direct env var reads for API keys
**Priority**: P1
**Category**: anti-pattern
**Files**: `crates/roko-neuro/src/episode_completion.rs`, `crates/roko-std/src/tool/builtin/web_search.rs`
**Problem**: Two live code paths read API keys directly from environment variables (`ANTHROPIC_API_KEY`, `PERPLEXITY_API_KEY`) instead of going through the provider configuration system. Keys are not rotated through credential management, calls are not tracked in cost accounting, and there is no fallback on missing env var. Verified as ISS-07 in doc 19.
**Fix**:
1. `episode_completion.rs`: accept a configured `Agent` or `ModelCallService` through dependency injection instead of constructing its own HTTP client
2. `web_search.rs`: accept a provider config or API key through the tool's configuration rather than reading env vars directly
3. Remove the `std::env::var("ANTHROPIC_API_KEY")` and `std::env::var("PERPLEXITY_API_KEY")` calls
4. Fall back to the provider config's `api_key_env` pattern for resolution
**Acceptance**: `grep -rn 'env::var.*API_KEY' crates/roko-neuro/ crates/roko-std/` returns zero matches. Both subsystems still function when the API key is configured in `roko.toml` providers.
**Depends on**: --
**Effort**: M

---

### TASK-S34: Fix `signals.jsonl` dead path (writes to `engrams.jsonl`)
**Priority**: P1
**Category**: bug-fix
**Files**: `crates/roko-fs/src/` (or wherever signal logging is configured)
**Problem**: Signal writes go to `engrams.jsonl` instead of `signals.jsonl`. The signal log is never populated. Verified in doc 05 section 7.2.
**Fix**:
1. Find the signal write path that targets `engrams.jsonl`
2. Change the target to `signals.jsonl` (the documented and expected path)
3. Or if `engrams.jsonl` is the canonical name, update documentation and the `status` command to read from it
**Acceptance**: `roko run "hello"`. `.roko/signals.jsonl` has at least one new entry.
**Depends on**: --
**Effort**: S

---

## P2 -- Correctness, Quality, Stability

These improve the system's robustness, maintainability, and correctness. Not
immediately user-visible but accumulate into significant quality improvements.

---

### TASK-S35: Unify model selection paths (auth_detect vs ServiceFactory)
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-cli/src/auth_detect.rs`, `crates/roko-orchestrator/src/service_factory.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/chat_session.rs`
**Problem**: 9+ dispatch paths have inconsistent model selection. `auth_detect.rs` scans environment variables in fixed priority, ignoring `roko.toml`. ServiceFactory resolves correctly via `resolve_model()`. Setting `default_model = "glm51"` in roko.toml has no effect via `roko run` because `auth_detect.rs` picks up `ANTHROPIC_API_KEY`. Verified as AP-2 in doc 05.
**Fix**:
1. Make all entry points use `ServiceFactory::build()` (or its resolve_model function) for model resolution
2. `auth_detect.rs` should be a fallback for credential discovery, not model selection
3. Model resolution priority: CLI override > task-level config > role config > roko.toml default_model > env var heuristic
4. Test that setting `default_model` in roko.toml actually takes effect from all entry points
**Acceptance**: Set `default_model = "cerebras-70b"` in roko.toml. `roko run "hello"`, `roko plan run`, and `roko chat` all use Cerebras (not Claude from env var detection).
**Depends on**: --
**Effort**: L

---

### TASK-S36: Normalize model aliases at load time
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-orchestrator/src/service_factory.rs`, `crates/roko-core/src/config/mod.rs`
**Problem**: `glm-5-1` on provider "zai" vs `glm51` on provider "zhipu" both resolve to `glm-5.1`. Multiple Claude aliases exist. Duplicate entries waste config space and confuse routing. Verified in doc 05 section 9.
**Fix**:
1. At config load time, normalize model slugs to canonical form
2. Build an alias table: `glm51` -> `glm-5.1`, `glm-5-1` -> `glm-5.1`, etc.
3. Warn when duplicate model entries resolve to the same canonical slug
4. CascadeRouter should use canonical slugs for observation tracking
**Acceptance**: Config with both `glm51` and `glm-5-1` produces a warning. CascadeRouter tracks a single entry.
**Depends on**: --
**Effort**: S

---

### TASK-S37: Export `rung_for_gate_name` from roko-gate
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/lib.rs` (or `gate_service.rs`), `crates/roko-runtime/src/effect_driver.rs` (lines 645-656)
**Problem**: EffectDriver duplicates the gate rung mapping from GateService. The source contains a TODO acknowledging this. If roko-gate changes rung assignments, EffectDriver silently uses stale mappings. Verified as ORCH-008 in doc 17.
**Fix**:
1. Export `rung_for_gate_name()` as a public function from roko-gate
2. Import it in EffectDriver
3. Delete the duplicate mapping in `effect_driver.rs`
**Acceptance**: `grep -rn 'rung_for_gate_name' crates/roko-runtime/` shows an import, not a definition. The function is defined only in roko-gate.
**Depends on**: --
**Effort**: S

---

### TASK-S38: Add TaskScheduler state to WorkflowEngine checkpoint
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-runtime/src/task_scheduler.rs`, `crates/roko-runtime/src/workflow_engine.rs`
**Problem**: WorkflowEngine checkpoints PipelineStateV2 state but not TaskScheduler state. A crash during multi-task execution loses all task-level progress. Resume restarts all tasks from the beginning. Verified as ORCH-009 in doc 17.
**Fix**:
1. Add `Serialize, Deserialize` derives to `TaskStatus` enum
2. Add a `checkpoint()` method to TaskScheduler that returns serializable state
3. Include TaskScheduler state in the WorkflowEngine checkpoint JSON
4. On resume, restore TaskScheduler state from checkpoint
5. Skip completed tasks during restore
**Acceptance**: Start a 5-task plan. Kill the process after task 3 completes. Resume. Tasks 1-3 are skipped, execution continues from task 4.
**Depends on**: --
**Effort**: M

---

### TASK-S39: Add `thinking_tokens` to UsageObservation
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-agent/src/usage.rs`, provider adapters
**Problem**: `UsageObservation` tracks input/output/cache tokens but not thinking/reasoning tokens. Models with thinking (Claude with `--effort`, OpenAI o3/o4-mini, Gemini with reasoning) produce internal reasoning tokens that cost money but are invisible. Verified as ISS-10 in doc 19.
**Fix**:
1. Add `thinking_tokens: Option<u64>` to `UsageObservation`
2. Update Claude CLI stream parser to extract reasoning token counts
3. Update OpenAI-compat parser for `reasoning_tokens` field
4. Update `CostTable` to use thinking-specific pricing
5. Surface thinking tokens in usage reports
**Acceptance**: Run with Claude and `--effort high`. Episode shows non-zero `thinking_tokens`.
**Depends on**: --
**Effort**: M

---

### TASK-S40: Fix singleton rate limiter across providers
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-agent/src/openai_compat_backend.rs`
**Problem**: `shared_rate_limiter()` uses `OnceLock` to create a single global `ProviderRateLimiter` with 60 RPM default. All `OpenAiCompatLlmBackend` instances share it. A provider with 1000 RPM is throttled to 60 RPM. A provider with 10 RPM may exceed its limit. Verified as ISS-16 in doc 19.
**Fix**:
1. Move rate limiter configuration to `ProviderConfig` with per-provider `rate_limit_rpm`
2. Create per-provider rate limiter instances keyed by provider name
3. `with_rate_limiter()` should be auto-wired from config, not manually called
4. Default to 60 RPM only when no config is specified
**Acceptance**: Configure two providers: one with `rate_limit_rpm = 10`, another with `rate_limit_rpm = 1000`. Both respect their individual limits independently.
**Depends on**: --
**Effort**: M

---

### TASK-S41: Add retry logic for transient provider failures
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-agent/src/model_call_service.rs`
**Problem**: `ModelCallService` has `fallback_models` for model-level failover but no retry logic for transient errors. Network timeouts, 500 errors, and rate limit with retry-after cause immediate failover. Verified as ISS-17 in doc 19.
**Fix**:
1. Add configurable retry policy to `ModelCallService`
2. Retry on `ProviderError::RateLimit`: honor `retry_after_ms` header
3. Retry on `ProviderError::ServerError`: exponential backoff (1s, 2s, 4s)
4. Retry on `ProviderError::Timeout`: once with 1.5x timeout
5. Never retry on `AuthFailure`, `ModelNotFound`, `ContextOverflow`
6. Max retries: configurable, default 2
7. After retries exhausted, fall through to fallback models
**Acceptance**: Mock a provider returning 500 once then 200. The call succeeds without switching models. Two 500s followed by success also works.
**Depends on**: --
**Effort**: M

---

### TASK-S42: Wire provider health circuit breaker to CascadeRouter
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-learn/src/provider_health.rs`, `crates/roko-learn/src/cascade_router.rs`
**Problem**: `ProviderHealthTracker` implements circuit breaker logic but CascadeRouter does not consistently wire it. When a provider goes down, the router may continue selecting models from that provider. Verified as I-19 in doc 18.
**Fix**:
1. Load `ProviderHealthRegistry` at CascadeRouter initialization
2. Before UCB scoring, filter out models whose provider circuit is open
3. Feed provider health state from FeedbackService model call success/failure
4. When circuit opens: log a warning with the provider name and failure rate
**Acceptance**: Simulate a provider with 5 consecutive failures. CascadeRouter stops selecting models from that provider until the circuit half-opens.
**Depends on**: S11
**Effort**: M

---

### TASK-S43: Wire SPC alerts drain to runtime consumers
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/adaptive_threshold.rs`
**Problem**: SPC alerts from the CUSUM/EWMA/BOCPD ensemble are collected in `pending_spc_alerts` but `drain_spc_alerts()` is never called from runtime code. Alerts accumulate indefinitely. Verified as I-6 in doc 20.
**Fix**:
1. After each gate pipeline run, call `drain_spc_alerts()`
2. For `OutOfControl` alert: tighten adaptive thresholds immediately
3. For `ChangePoint` detected: reset EMA to adapt faster
4. Log alerts to efficiency events
5. Surface in TUI as threshold update events
**Acceptance**: Introduce a gate pass rate shift. After sufficient observations, SPC alert appears in logs/events.
**Depends on**: --
**Effort**: M

---

### TASK-S44: Wire Hotelling T-squared to runtime gate pipeline
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/hotelling.rs`, `crates/roko-gate/src/adaptive_threshold.rs`
**Problem**: Hotelling T-squared joint anomaly detector is implemented (439 LOC, tested) but `observe_pipeline()` is never called from runtime. Joint anomalies (multiple gates degrading simultaneously) go undetected. Verified as I-7 in doc 20.
**Fix**:
1. After each full pipeline run, call `observe_pipeline()` with the pass-rate vector across all rungs
2. If `joint_anomaly_detected()`, emit a high-priority alert
3. Log the anomaly in the episode record
**Acceptance**: Simulate simultaneous compile and test pass rate drops. Hotelling T-squared fires before individual gate monitors would.
**Depends on**: --
**Effort**: M

---

### TASK-S45: Wire domain profiles to AdaptiveThresholds
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/adaptive_threshold.rs`
**Problem**: Three domain profiles (coding, research, security) with per-rung priors are implemented but never instantiated. All agents start from neutral priors (0.5) regardless of role. Verified as I-8 in doc 20.
**Fix**:
1. At plan start, select a domain profile from the agent role config
2. Apply rung priors as initial EMA values for fresh `AdaptiveThresholds`
3. Default: coding profile for implementer/reviewer roles, research for research role, security for auditor role
**Acceptance**: A new workspace with an "implementer" role starts with coding domain priors (not neutral 0.5).
**Depends on**: --
**Effort**: S

---

### TASK-S46: Wire conductor bandit to live retry paths
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-learn/src/conductor.rs`, `crates/roko-cli/src/run.rs`
**Problem**: The conductor bandit (7 actions, 19-dim context, blended Thompson+linear scoring) decides whether a failing task should continue, receive a hint, escalate, restart, or abort. It is never invoked. All retry decisions are hardcoded. Verified as I-06 in doc 18.
**Fix**:
1. Load ConductorBandit state from `.roko/learn/conductor.json` (or create new)
2. Call `bandit.select_action()` before each retry in `roko run` and `roko plan run`
3. Feed reward after retry outcome
4. Save state on flush
5. Surface conductor decisions in episode records
**Acceptance**: After 20+ retry observations, the conductor learns to abort earlier for certain failure patterns.
**Depends on**: --
**Effort**: M

---

### TASK-S47: Wire anomaly detector to live paths
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-learn/src/anomaly.rs`, `crates/roko-cli/src/run.rs`
**Problem**: Anomaly detector (prompt loops, cost spikes, quality degradation) is session-local and lightweight but never instantiated. Verified as I-14 in doc 18.
**Fix**:
1. Create `AnomalyDetector` at session start in `roko run`
2. Check prompt hash before each dispatch (detect loops)
3. Check cost after each response (detect spikes)
4. On anomaly: log warning, optionally trigger conductor abort
**Acceptance**: Create a prompt that would loop. Anomaly detector fires after 3 identical prompts.
**Depends on**: --
**Effort**: M

---

### TASK-S48: Wire regression detection alerting path
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-learn/src/regression.rs`
**Problem**: `detect_regressions()` produces `RegressionReport` with alerts but reports are returned and discarded. No alerting. Verified as I-13 in doc 18.
**Fix**:
1. Log regression alerts at WARN level
2. Surface in `roko status` output
3. Feed severe regressions to conductor (trigger model switch or abort)
**Acceptance**: After a pass rate drops >15% between consecutive runs, `roko status` shows a regression warning.
**Depends on**: --
**Effort**: S

---

### TASK-S49: Add end-of-run summary to plan runner
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/plan.rs`
**Problem**: After `roko plan run` completes, there is no aggregate outcome summary. The output streams agent text but gate results scroll off screen. Users must read log files to determine what happened. Verified as I-UX02 in doc 15.
**Fix**:
1. After all tasks complete (or on Ctrl-C), collect task results from executor state
2. Print a summary:
   ```
   Run complete: sprint-42
     Passed: 8/10 tasks
     Failed: T6 (gate: clippy), T9 (gate: test)
     Skipped: 0
     Cost: $8.47 | Duration: 34min
     Resume: roko plan run plans/ --resume .roko/state/executor.json
   ```
3. Save the summary to `.roko/state/last-run-summary.json` for `roko status --last-run`
**Acceptance**: `roko plan run` on a 3-task plan prints a summary at the end showing pass/fail counts.
**Depends on**: --
**Effort**: M

---

### TASK-S50: Expose `max_concurrent_tasks` from config
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-core/src/config/mod.rs`
**Problem**: Despite having a full DAG scheduler, runner v2 hardcodes `max_concurrent_tasks: 1`. Plans execute tasks sequentially even when the DAG allows parallelism. Verified as ORCH-001 in doc 17.
**Fix**:
1. Add `max_concurrent_tasks` to `[execution]` config section in roko.toml
2. Read from config in `event_loop.rs` instead of hardcoding 1
3. Default to 1 (safe), allow up to 8
4. Add `--parallel <N>` CLI flag to `roko plan run` for override
5. Guard: only allow N > 1 when worktree isolation is available (S76)
**Acceptance**: Set `max_concurrent_tasks = 4` in config (or `--parallel 4`). With 4 independent tasks, all 4 start simultaneously.
**Depends on**: --
**Effort**: M

---

### TASK-S51: Make `dangerously_skip_permissions` configurable
**Priority**: P2
**Category**: security
**Files**: `crates/roko-cli/src/commands/plan.rs` (line ~394)
**Problem**: Every plan execution path sets `dangerously_skip_permissions: true`. Safety contracts are loaded but fall back to permissive defaults. Agents always run with full permissions. Verified in doc 11 section 7.4.
**Fix**:
1. Add `skip_permissions: bool` to `[execution]` config in roko.toml (default: true for backward compat)
2. Generate default contract YAML during `roko init` with sane restrictions
3. Read the config value in plan.rs instead of hardcoding true
4. Log a warning when running with skip_permissions = true
5. Long-term: change default to false once safety contracts are reliable
**Acceptance**: Set `skip_permissions = false` in config with a contract YAML. Agent runs with permission constraints enforced.
**Depends on**: --
**Effort**: M

---

### TASK-S52: Replace ACP inline review prompts with templates
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-acp/src/runner.rs`, `crates/roko-compose/src/templates/reviewer.rs`
**Problem**: `run_multi_role_review()` hardcodes full role descriptions for "Architect Reviewer" and "Security & Correctness Auditor" in `format!()` strings. These partially duplicate and partially conflict with `ReviewerTemplate`. Verified as ISS-05 in doc 16.
**Fix**:
1. Replace inline `format!()` prompts with calls to `ReviewerTemplate::architect()` and `ReviewerTemplate::security()`
2. If those specific template methods don't exist, add them
3. Remove the inline role description strings
**Acceptance**: `grep -rn 'Architect Reviewer' crates/roko-acp/` returns zero matches in non-template code.
**Depends on**: --
**Effort**: M

---

### TASK-S53: Fix OpenAI-compat provider quirks fragmentation
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-agent/src/openai_compat_backend.rs`
**Problem**: Per-provider workarounds (`skip_session_fields`, `disable_parallel_tool_calls`, `normalize_tool_call_content`) accumulate as boolean flags. The number of flag combinations grows exponentially with new providers. Verified as ISS-12 in doc 19.
**Fix**:
1. Replace boolean flags with a `ProviderQuirks` struct:
   ```rust
   struct ProviderQuirks {
       session_fields: bool,
       parallel_tool_calls: bool,
       normalize_tool_call_content: bool,
       max_tools: Option<usize>,
       timeout_budget: Option<Duration>,
   }
   ```
2. Implement `ProviderQuirks::for_provider(name: &str)` with a match on known providers
3. Replace `if self.skip_session_fields` with `if self.quirks.session_fields` pattern
**Acceptance**: Adding a new strict provider requires only a new `ProviderQuirks::for_provider` entry, not new boolean fields.
**Depends on**: --
**Effort**: M

---

### TASK-S54: Make tool loop max iterations configurable
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-agent/src/provider/cerebras.rs`, `crates/roko-agent/src/provider/openai_compat.rs`
**Problem**: Cerebras adapter sets `tool_loop_max_iterations(50)`, OpenAI-compat uses 30. These are per-adapter constants, not configurable. Verified as ISS-13 in doc 19.
**Fix**:
1. Add `max_tool_iterations` to `ModelProfile` or `ProviderConfig` schema
2. Read from config in each adapter
3. Default: 30 for API providers, 50 for Cerebras
**Acceptance**: Set `max_tool_iterations = 10` in provider config. Agent stops after 10 tool iterations.
**Depends on**: --
**Effort**: S

---

### TASK-S55: Unify StateHub types between serve and CLI
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-serve/src/state_hub_compat.rs`, `crates/roko-cli/src/state_hub.rs`, `crates/roko-core/src/state_hub.rs`
**Problem**: `roko-serve` and `roko-cli` both include `state_hub.rs` via `#[path]` includes, creating two incompatible `StateHub` types. DashboardEvents from `roko run --serve` don't flow to HTTP SSE/WebSocket. Verified in doc 11 section 7.2.
**Fix**:
1. Extract `StateHub` into `roko-core` as a first-class public type
2. Remove `#[path]` includes from both crates
3. Both `roko-serve` and `roko-cli` import from `roko-core::StateHub`
4. Share a single `StateHub` instance between serve and CLI when running together
**Acceptance**: Run `roko run --serve "hello"`. SSE endpoint at `localhost:6677/events` emits real-time DashboardEvents from the run.
**Depends on**: --
**Effort**: L

---

### TASK-S56: Wire dream consolidation trigger
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-dreams/src/runner.rs`, `crates/roko-serve/src/lib.rs`
**Problem**: Dream consolidation is built but has no runtime trigger. `DreamTriggerSink` writes events that nothing reads. Knowledge consolidation only happens via manual `roko knowledge dream run`. Verified as I-08 in doc 18 and doc 11 section 7.7.
**Fix**:
1. Start a dream loop in `roko serve` background tasks
2. Configure trigger via `roko.toml` (`[dreams]` section): cron interval or plan-completion trigger
3. DreamRunner already supports all trigger modes -- just needs instantiation
4. Alternatively: add a post-run hook in `roko plan run` that checks if dream cycle is due
5. Report dream cycle status in `roko status`
**Acceptance**: Set `dreams.trigger = "plan-completion"` in config. After `roko plan run`, a dream cycle runs automatically. `roko status` shows last dream cycle timestamp.
**Depends on**: --
**Effort**: M

---

### TASK-S57: Wire knowledge candidate ingestion post-run
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-learn/src/knowledge_ingestion.rs` (or equivalent)
**Problem**: Knowledge candidates are written to `.roko/learn/knowledge_candidates.jsonl` but never ingested into the KnowledgeStore. Candidates accumulate without bound. Verified in doc 11 section 5.3.
**Fix**:
1. After each plan run (or periodically), read new candidates from the JSONL file
2. Validate and deduplicate against existing knowledge store entries
3. Ingest validated candidates into KnowledgeStore
4. Mark ingested candidates in the JSONL (or truncate the file)
**Acceptance**: After a run that produces knowledge candidates, `roko knowledge stats` shows new entries.
**Depends on**: --
**Effort**: S

---

### TASK-S58: Fix `--share` without `--serve` producing dead URL
**Priority**: P2
**Category**: bug-fix
**Files**: `crates/roko-cli/src/share.rs`, `crates/roko-cli/src/run.rs`
**Problem**: `roko run --share` writes a JSON transcript and prints a `http://localhost:6677/runs/{token}` URL that's inaccessible without serve running. Verified in doc 11 section 1.1.
**Fix**:
1. When `--share` is used without `--serve`: generate a self-contained HTML artifact (inline the transcript JSON as a `<script>` tag with a minimal viewer)
2. Write to `.roko/shared/{token}.html`
3. Print the local file path instead of the dead URL
4. When `--serve` IS active: print the serve URL as before
**Acceptance**: `roko run --share "hello"` without serve running prints a local file path. Opening the HTML file in a browser shows the transcript.
**Depends on**: --
**Effort**: S

---

### TASK-S59: Add `--dry-run` to `roko plan run`
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-cli/src/commands/plan.rs`
**Problem**: No way to preview what `roko plan run` will do without executing. Users cannot estimate cost, verify DAG ordering, or check model selection before committing. Verified in doc 11 recommendation 19 and doc 15 I-UX02.
**Fix**:
1. Add `--dry-run` flag to `plan run` subcommand
2. Load plans, build DAG, compute execution waves
3. Show: wave ordering, per-task model selection, estimated cost, estimated time
4. Do not dispatch any agents
5. Exit after printing the plan
**Acceptance**: `roko plan run plans/ --dry-run` prints wave ordering and estimated costs without spawning agents.
**Depends on**: --
**Effort**: M

---

### TASK-S60: Make workspace map cap proportional to context tier
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-compose/src/prompt_assembly_service.rs` (line 22)
**Problem**: `WORKSPACE_MAP_LINE_LIMIT = 200` is a fixed constant. Large codebases lose file listings. Small models waste context on long maps. Verified as ISS-12 in doc 16.
**Fix**:
1. Make the cap proportional to context tier: Surgical 50, Focused 150, Full 300, Extended 500
2. Or filter the workspace map to show only files relevant to the current task's crate/module
**Acceptance**: With Surgical tier, workspace map is 50 lines max. With Full tier, 300 lines.
**Depends on**: S18
**Effort**: S

---

### TASK-S61: Wire knowledge store to CascadeRouter model selection
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-neuro/src/knowledge_store.rs`, `crates/roko-learn/src/cascade_router.rs`
**Problem**: Knowledge store contains task-specific insights that could inform model routing. CascadeRouter does not query it. Dream routing advice is generated but not loaded. Verified as I-10 in doc 18.
**Fix**:
1. Load `DreamRoutingAdvice` at CascadeRouter initialization
2. Apply `dream_advice_to_routing_bias()` (already implemented in `routing_advice.rs`)
3. Query knowledge store for task-specific model hints during routing
**Acceptance**: After running a dream cycle that produces routing advice, subsequent model selections reflect the advice (e.g., preferring a model the advice recommends).
**Depends on**: S11
**Effort**: M

---

### TASK-S62: Fix GatePipeline / ComposedGatePipeline duplication
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/gate_pipeline.rs`
**Problem**: `GatePipeline` and `ComposedGatePipeline` partially duplicate logic. ComposedGatePipeline's Sequential mode re-implements the loop from GatePipeline, with dead code (`let _ = pipeline;`). Verified as I-11 in doc 20.
**Fix**:
1. ComposedGatePipeline Sequential mode should delegate to GatePipeline
2. Or deprecate GatePipeline in favor of ComposedGatePipeline
3. Remove the dead `let _ = pipeline` code
**Acceptance**: Sequential gate execution runs through a single code path. No dead code assignments.
**Depends on**: --
**Effort**: M

---

### TASK-S63: Wire ProcessRewardModel to orchestrator
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/process_reward.rs`
**Problem**: ProcessRewardModel tracks per-turn gate snapshots and derives Promise (probability of eventual success) and Progress signals. Not instantiated during orchestration. Tasks clearly failing continue consuming budget until retries are exhausted. Verified as I-12 in doc 20.
**Fix**:
1. Instantiate PRM per-task in the event loop
2. After each gate snapshot, update PRM
3. If Promise drops below threshold (e.g., 0.1), abort early instead of exhausting retries
4. Log PRM signals in episode records
**Acceptance**: A task that fails compile 3 times with increasingly worse output is aborted early by PRM instead of running all 5 retry attempts.
**Depends on**: --
**Effort**: M

---

### TASK-S64: Wire AcceptanceContract to gate pipeline
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/acceptance_contract.rs`
**Problem**: AcceptanceContract defines formal requirements (NoStubRequirement, etc.) with evidence collection. Not wired into the gate pipeline. No formal acceptance criteria are checked. Verified as I-13 in doc 20.
**Fix**:
1. Add AcceptanceContract as an optional post-gate verification step
2. Load contract requirements from task definition or plan config
3. After gate pipeline passes, check acceptance contract
4. If contract fails, treat as gate failure
**Acceptance**: Configure a `NoStubRequirement`. A task that introduces a stub function fails the acceptance contract even if compile/clippy/test pass.
**Depends on**: --
**Effort**: M

---

### TASK-S65: Add anti-pattern checks as pre-gate step
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-gate/src/` (new), `crates/roko-runtime/src/effect_driver.rs`
**Problem**: The mega-parity runner uses fast grep-based anti-pattern checks (AP-1 through AP-10) that catch common LLM code generation mistakes in milliseconds. Not integrated into any gate. Verified as ORCH-016 in doc 17.
**Fix**:
1. Create an `AntiPatternGate` in roko-gate that runs grep-based checks
2. Anti-patterns: stub pass, `block_on` in async, duplicate traits, raw `Command::new("claude")`, inline prompt strings, `std::sync::Mutex` across `.await`, empty function bodies, `unimplemented!/unreachable!`, hardcoded localhost/port
3. Run as rung -1 (before compile) -- millisecond cost
4. Return structured feedback per anti-pattern found
**Acceptance**: A task that introduces `unimplemented!()` in non-test code triggers AP-9 check and fails with structured feedback before compile runs.
**Depends on**: --
**Effort**: M

---

### TASK-S66: Wire VerdictPublisher to all gate dispatch paths
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-gate/src/verdict_publisher.rs`, gate dispatch paths
**Problem**: VerdictPublisher is optional in RungExecutionConfig and rarely provided. Gate verdicts are not broadcast to TUI, SSE, or WebSocket. Gates run silently. Verified as I-15 in doc 20.
**Fix**:
1. In each gate dispatch path, provide a VerdictPublisher
2. Wire publisher to DashboardEvent emitter (for TUI)
3. Wire publisher to SSE channel (for serve)
**Acceptance**: During `roko plan run` with dashboard, gate progress appears in real time (not just at the end).
**Depends on**: --
**Effort**: S

---

### TASK-S67: Add gate budget tracking for LLM judge calls
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-gate/src/` (LLM judge implementation)
**Problem**: LLM judge gate invocations have no cost tracking. Each judge call involves a full LLM API call but no episode is recorded, no cost attributed, and no limit prevents runaway invocations. Verified as I-9 in doc 20.
**Fix**:
1. Record an episode per judge invocation with model, tokens, cost
2. Track cumulative gate cost separately from agent cost
3. Cap judge invocations per task at a configurable maximum (default: 3)
4. Include gate cost in the run summary
**Acceptance**: Run a task with LLM judge. Episode log shows a separate judge entry with cost. Total cost includes judge costs.
**Depends on**: --
**Effort**: M

---

### TASK-S68: Wire StagingBuffer lightweight promotion
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-dreams/src/staging.rs`
**Problem**: Knowledge candidates in StagingBuffer progress from Raw -> Replayed -> Validated, but promotion to the durable store only happens during a dream cycle. Without a running dream cycle, the buffer grows without bound. Verified as I-17 in doc 18.
**Fix**:
1. Add a lightweight promotion check in LearningRuntime
2. After each completed run, check if any candidates are in Validated state
3. Promote Validated entries to KnowledgeStore without requiring a full dream cycle
4. Log promotions
**Acceptance**: After 5 runs that produce validated candidates, candidates appear in KnowledgeStore without manual dream run.
**Depends on**: --
**Effort**: S

---

### TASK-S69: Add cross-session cost aggregation
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-learn/src/costs_db.rs`, `crates/roko-learn/src/budget.rs`
**Problem**: Cost tracking per session exists but no cross-session aggregation for daily budget enforcement or dashboard display. The `per_day` budget scope has no way to know previous session spend. Verified as I-12 in doc 18.
**Fix**:
1. `CostsDb.aggregate_since(today_start)` -> daily total
2. Initialize `BudgetGuardrail.day_spent` from aggregate
3. Expose daily/weekly/monthly aggregates via `roko learn efficiency`
**Acceptance**: Run 3 sessions on the same day. `roko learn efficiency` shows cumulative daily cost.
**Depends on**: --
**Effort**: M

---

### TASK-S70: Add content-type-aware token counting ratios
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-compose/src/token_counter.rs`, `crates/roko-compose/src/prompt.rs`
**Problem**: Flat 4:1 character-to-token ratio for all content types. Code content is closer to 3:1, markdown with whitespace closer to 5:1. Errors compound when budget is tight. Verified as ISS-06 in doc 16.
**Fix**:
1. Add `content_type` parameter to `estimate_tokens()`: Code, Prose, Markdown
2. Use ratios: Code 3.0, Prose 4.0, Markdown 5.0
3. When content type is unknown, use 3.5 (conservative, favors not overflowing)
**Acceptance**: `estimate_tokens("fn foo() { bar(); }", ContentType::Code)` returns a higher count than `estimate_tokens` with default ratio.
**Depends on**: --
**Effort**: S

---

### TASK-S71: Make knowledge confidence thresholds tier-dependent
**Priority**: P2
**Category**: correctness
**Files**: `crates/roko-compose/src/prompt_assembly_service.rs`
**Problem**: Knowledge confidence thresholds (domain >= 0.5, techniques >= 0.3, anti-patterns >= 0.2) are hardcoded. Too permissive for small models (wastes context), possibly too restrictive for exploratory tasks. Verified as ISS-13 in doc 16.
**Fix**:
1. Make thresholds dependent on ContextTier:
   - Surgical: 0.8 domain, 0.7 techniques, 0.5 anti-patterns (only high-confidence)
   - Focused: 0.5, 0.3, 0.2 (current defaults)
   - Full: 0.3, 0.2, 0.1 (include more speculative knowledge)
**Acceptance**: With Surgical tier, only high-confidence knowledge entries appear in the prompt.
**Depends on**: S18
**Effort**: S

---

### TASK-S72: Wire conversation compaction to `roko chat`
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-compose/src/compaction.rs`, `crates/roko-cli/src/chat_session.rs`
**Problem**: Long `roko chat` sessions hit context limits. `compact_history()` is fully implemented (anchor preservation, gate carry-forward, iterative summarization) but never called from the chat REPL. Verified as ISS-09 in doc 16.
**Fix**:
1. After each turn, check if conversation history exceeds a threshold (e.g., 80% of context window)
2. If exceeded, call `compact_history()` to summarize older turns
3. Preserve anchor turns (first turn, last gate result, last tool output)
**Acceptance**: In a 50-turn chat session, the system continues working without context overflow. Old turns are summarized.
**Depends on**: --
**Effort**: S

---

### TASK-S73: Add prompt caching metrics to ModelCallService
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-agent/src/model_call_service.rs`
**Problem**: ModelCallService has an L1 response cache but no metrics. No hit rate tracking, no eviction statistics, no cache savings analysis. Verified as ISS-11 in doc 19.
**Fix**:
1. Add `CacheMetrics` to `CacheCell`: hits, misses, evictions, size_bytes
2. Expose metrics via gateway events
3. Add Anthropic server-side cache utilization tracking (from `cache_read_tokens`)
4. Report cache savings in cost panel
**Acceptance**: `roko learn efficiency` shows cache hit rate and estimated cost savings.
**Depends on**: --
**Effort**: M

---

### TASK-S74: Add disk pressure monitoring pre-dispatch
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-runtime/src/effect_driver.rs` (or new module)
**Problem**: Disk exhaustion from cargo build caches causes silent failures. No monitoring exists. With parallel execution, disk usage scales linearly. Verified as ORCH-020 in doc 17.
**Fix**:
1. Before dispatching an agent, check available disk space
2. If below threshold (5GB), pause dispatch with a warning
3. Optionally: run `cargo clean --target-dir` on old worktree targets
4. Resume when space is available
**Acceptance**: Set threshold to a high value (e.g., 100TB). Dispatch pauses with "insufficient disk space" message.
**Depends on**: --
**Effort**: M

---

### TASK-S75: Add agent execution time monitoring
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-cli/src/runner/event_loop.rs`
**Problem**: ~5% of agents ignore explicit instructions (e.g., "do not run cargo"), taking 5-15x longer. No monitoring detects this. Verified as ORCH-021 in doc 17.
**Fix**:
1. Track expected execution time per task tier (fast: 2min, standard: 10min, complex: 30min)
2. Monitor actual agent duration
3. If duration exceeds 3x expected, log a warning
4. Optionally: send SIGTERM and retry with stronger instructions
**Acceptance**: An agent that runs 10x longer than expected triggers a duration warning in logs.
**Depends on**: --
**Effort**: M

---

### TASK-S76: Fix WorkflowEngine missing worktree integration
**Priority**: P2
**Category**: stability
**Files**: `crates/roko-runtime/src/workflow_engine.rs`
**Problem**: WorkflowEngine operates on a single `workdir: PathBuf` with no WorktreeManager or MergeQueue reference. Parallel tasks in the same directory cause file conflicts. Blocks parallel execution. Verified as ORCH-007 in doc 17.
**Fix**:
1. Add optional `WorktreeManager` to `EffectServices`
2. When parallel tasks are dispatched, allocate a worktree per task
3. After task completion, merge via MergeQueue with file-overlap detection
4. Fall back to single-directory mode when worktrees are not available
**Acceptance**: Two parallel tasks that modify different files both succeed without conflicts. Each runs in its own worktree.
**Depends on**: --
**Effort**: L

---

### TASK-S77: Unify two PipelineState state machines
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-runtime/src/pipeline_state.rs`, `crates/roko-core/src/phase.rs`
**Problem**: PipelineStateV2 (10 states) and PlanPhase (14 states) model the same concept but are not interoperable. WorkflowEngine and Runner v2 cannot share state. Monitoring tools must handle both. Verified as ORCH-003 in doc 17.
**Fix**:
1. Define a superset state machine that covers both, with optional phases
2. Map optional phases: Enriching (skip when no enrichment configured), DocRevision (skip when no review), RegeneratingVerify (skip when no generated tests)
3. Both WorkflowEngine and Runner v2 use the unified state machine
4. Add phase adapter for backward compatibility
**Acceptance**: A single `WorkflowPhase` enum is used in both crates. `grep -rn 'PipelineStateV2\|PlanPhase' crates/` returns only adapter/compat code.
**Depends on**: --
**Effort**: L

---

### TASK-S78: Consolidate 4 agent dispatch implementations
**Priority**: P2
**Category**: anti-pattern
**Files**: `crates/roko-acp/src/runner.rs`, `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/orchestrate.rs`, `crates/roko-runtime/src/effect_driver.rs`
**Problem**: Four agent dispatch implementations with different features, error handling, timeout logic, token counting, and safety checks. Bug fixes in one don't propagate. Verified as ORCH-004 in doc 17.
**Fix**:
1. Consolidate into EffectDriver's `ModelCaller` + `PromptAssembler` trait pattern
2. Add service traits for safety, custody, and knowledge routing
3. Compose into `EffectServices` struct
4. All dispatch paths delegate to EffectDriver
5. Delete or deprecate redundant implementations
**Acceptance**: A single dispatch code path handles all cases. Changing dispatch behavior (e.g., adding safety check) affects all surfaces.
**Depends on**: --
**Effort**: L

---

## Dependency Graph (Critical Path)

```
S09 -> S10              (gate config normalization)
S11 -> S22              (cascade router -> runner v2 observations)
S11 -> S42              (cascade router -> health circuit breaker)
S11 -> S61              (cascade router -> knowledge-informed routing)
S18 -> S19              (context tier -> budget predictor)
S18 -> S60              (context tier -> workspace map cap)
S18 -> S71              (context tier -> knowledge confidence)
S25 -> S27              (section effectiveness recording -> reading)

All other tasks are independent and can be parallelized.
```

## Execution Order Recommendation

**Week 1**: S01-S10 (all P0 tasks -- crashes, security, data loss)
**Week 2**: S11-S17 (P1 foundation: cascade router, feedback, budget)
**Week 3**: S18-S27 (P1 prompt/learning: context tiers, stream parsing, runner v2 learning)
**Week 4**: S28-S34 (P1 gates/dispatch: failure routing, ACP fixes, env vars)
**Week 5-6**: S35-S54 (P2 first batch: model unification, retry, thresholds)
**Week 7-8**: S55-S78 (P2 second batch: StateHub, dreams, architecture debt)

P0 tasks are all independent and can run in parallel.
P1 tasks S22-S27 depend on S11 and can run after it.
P2 tasks are mostly independent.

---

## Sources

| Document | Issues Sourced |
|---|---|
| `05-CURRENT-STATE-AND-GAPS.md` | HOLLOW 1-3, AP 1-8, Security 1-4, Gap list |
| `11-CURRENT-STATE-GROUND-TRUTH.md` | Execution paths, gaps, recommendations 1-20 |
| `15-UX-ISSUES.md` | I-UX01 through I-UX18 |
| `16-PROMPT-ISSUES.md` | ISS-01 through ISS-17 |
| `17-ORCH-ISSUES.md` | ORCH-001 through ORCH-021 |
| `18-LEARN-ISSUES.md` | I-01 through I-20 |
| `19-DISPATCH-ISSUES.md` | ISS-01 through ISS-17 |
| `20-GATE-ISSUES.md` | I-1 through I-15, VS 1-4 |
