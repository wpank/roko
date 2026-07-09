# audit-2026-05-01 Issue Tracker

Open issues from the 2026-05-01 audit. Each issue has a batch ID that
maps 1:1 to a runner prompt (`prompts/{ID}.prompt.md`) and a
`batches.toml` entry.

**Source plans** (canonical detail):
`tmp/subsystem-audits/implementation-plans/`. Cross-references are in
each batch's prompt.

**Mark items complete only when**:
1. The corresponding batch has merged green.
2. The verification block in the prompt passes.
3. A re-grep against the current worktree confirms the named
   symbols / files / state.

**Verification truth source**: code in tree, **NOT** this file.
Re-grep before ticking a box.

> Source plans:
> - `12-tier2-delete-dead-code.md` (T2-prefix)
> - `13-tier3-security-hardening.md` (T3-prefix)
> - `14-tier4-feedback-loops.md` (T4-prefix)
> - `15-tier5-architectural.md`, `20-orchestrate-rs-extraction.md` (T5-prefix)
> - `21-..-34-...md` (S-prefix subsystem cross-cutting)
> - Forward-looking plans 40-42 are **not** in this runner.

---

## Already done in tree (skipped by this runner)

These were verified complete on 2026-05-01 against `git log`. Do
**not** re-add. If a regression is found, file a new batch.

### Tier 0 — Stop active bleeding (all done)

- [x] **T0-1** — Mask secrets in GET /api/config/toml. Commit: `196e6087`. Verify: `crates/roko-serve/src/routes/config.rs:48-65`.
- [x] **T0-2** — Expand mask_secret_fields (chain, GH webhooks, providers). Commit: `9264f4fe`. Verify: `crates/roko-serve/src/routes/config.rs:290-320`.
- [x] **T0-3** — Path validation on shared runs. Commit: `fa8f2780`. Verify: `crates/roko-serve/src/routes/shared_runs.rs:145, 149, 277`.
- [x] **T0-4** — Generic webhook behind auth. Commit: `d5e2a353`. Verify: `crates/roko-serve/src/routes/webhooks.rs:29-37`.
- [x] **T0-5** — SSRF validation on agent registration URLs. Commit: `cf186346`. Verify: `crates/roko-serve/src/routes/agents.rs:1635-1727`.
- [x] **T0-6** — Knowledge sink filename alignment. Commit: `91b58eb3`. Verify: `crates/roko-neuro/src/admission.rs:26`.
- [x] **T0-7** — Duplicate model context_window values fixed. Commits: `94291d0d`, `bab972e3`. Verify: `roko.toml`.

### Tier 1 — Silent data corruption (all done)

- [x] **T1-8** — Dispatch metadata on RunnerEvent::TaskAttemptCompleted. Commit: `3245beca`. Verify: `crates/roko-cli/src/runner/types.rs:596+`, `event_loop.rs:1344-1374`.
- [x] **T1-9** — Legacy emit_feedback removed. Same commit as T1-8.
- [x] **T1-10** — Explicit Rung match arms (no _ catch-all). Commit: `130477c4`. Verify: `crates/roko-cli/src/orchestrate.rs::selected_gate_steps`.
- [x] **T1-11** — gate_rung_caps detects scaffolding. Commit: `1ac280cb`. Verify: `crates/roko-cli/src/orchestrate.rs:17291-17297`.
- [x] **T1-12** — validate_strict_config_toml in load_config. Verify: `crates/roko-core/src/config/mod.rs:119`.
- [x] **T1-13** — ContextualBanditPolicy shadow mode removed from runner. Commit: `0538d1d1`. Verify: `crates/roko-cli/src/commands/plan.rs`, `serve_runtime.rs`.
- [x] **T1-14** — observe_pipeline + drain_spc_alerts wired. Commit: `a5eb04bd`. Verify: `crates/roko-cli/src/orchestrate.rs:16923-16933`.
- [x] **T1-15** — Permissive safety fallback replaced with restricted. Commit: `39782f5c`. Verify: `crates/roko-agent/src/safety/mod.rs:246-256, 873-896`.

---

## Group T2 — Delete dead code

**Wave 1** — independent. Run together.

- [ ] **T2-16** — Delete 4 orphan learn files (`resonant_patterns.rs`, `signal_metabolism.rs`, `shapley.rs`, `kalman.rs`). Not in `roko-learn/src/lib.rs`; never compiled. → `prompts/T2-16.prompt.md`
- [ ] **T2-17a** — Delete unused learn modules (research-related, 5 modules: `adversarial`, `adas`, `research_pipeline`, `bandit_research`, `forensic_replay`). Run pre-deletion safety check first. → `prompts/T2-17a.prompt.md`
- [ ] **T2-17b** — Delete unused learn modules (stats-related, 9 modules: `calibration_policy`, `causal`, `reinforce_kind`, `regression`, `drift`, `local_reward`, `section_outcome`, `post_gate_reflection`, `verdict_scorer`). Run pre-deletion safety check first. → `prompts/T2-17b.prompt.md`
- [ ] **T2-18** — Remove 7 phantom config sections: `OneirographyConfig` (tools.rs), `DemurrageConfig` / `AttentionConfig` / `ImmuneConfig` / `TemporalConfig` / `GoalsConfig` (learning.rs), `EnergyConfig` (budget.rs). Schema, hot_reload, compat, roko.toml all touched. → `prompts/T2-18.prompt.md`
- [ ] **T2-19** — Remove 6 phantom `ConductorConfig` fields (`auto_advance_batch`, `auto_merge_on_complete`, `pre_plan`, `conductor_model`, `warm_implementers_per_plan`, `enabled_roles`). Keep: `max_agents`, `max_parallel_plans`, `parallel_enabled`, `express_mode`, `max_auto_fix_attempts`, `auto_fix_model`, `watchers`. → `prompts/T2-19.prompt.md`
- [ ] **T2-20** — Delete write-only sinks: `ConductorObservationSink` and `DreamTriggerSink`. Both have zero consumers. → `prompts/T2-20.prompt.md`
- [ ] **T2-21** — Remove phantom `AgentConfig` fields (`policy_manifests`, `domain`). Keep `data_llm` (real implementation, not yet wired; document with doc-comment). → `prompts/T2-21.prompt.md`

---

## Group T3 — Security hardening

**Wave 1** — independent.

- [ ] **T3-22** — Flip `ServeAuthConfig::default()` `enabled: false` → `true`. Update `roko init` template and tests that assume default-off. Auto-enable-on-public-bind logic at `crates/roko-serve/src/lib.rs:790-805` stays as defense in depth. → `prompts/T3-22.prompt.md`
- [ ] **T3-23** — Add `tower-governor` rate limiting. Global 100 req/s, burst 200. Per-route caps: terminal sessions 5/min, inference 30/min, agent register 10/min, webhooks 60/min. Identify clients by API key when present. → `prompts/T3-23.prompt.md`
- [ ] **T3-24** — Lower global body limit to 4 MiB (currently `RequestBodyLimitLayer::new(32 MiB)` at `routes/mod.rs:183`). Add per-route overrides: webhooks 1 MiB / 256 KiB / 64 KiB; large-upload routes raise explicitly. → `prompts/T3-24.prompt.md`
- [ ] **T3-25** — `PORT` env changes port only. Bind stays `127.0.0.1` unless `serve.bind = "0.0.0.0"` is set explicitly. Currently `lib.rs:236-237` forces `0.0.0.0:$PORT`. → `prompts/T3-25.prompt.md`
- [ ] **T3-26** — Add `max_message_size(1 MiB)` and `max_frame_size(256 KiB)` to all `WebSocketUpgrade` calls. Terminal session WS: 256 KiB / 64 KiB. → `prompts/T3-26.prompt.md`
- [ ] **T3-27** — Fix path traversal + TOML injection in `create_agent` (`routes/agents.rs:605`). (a) `validate_workspace_dir_segment` for agent name + canonicalize containment after `create_dir_all`. (b) Replace `format!`-built TOML with `toml::to_string_pretty(&AgentManifest)`. (c) Restrict `req.domain` to `[a-z][a-z0-9_-]{0,31}`. → `prompts/T3-27.prompt.md`
- [ ] **T3-28** — Replace `allow_methods(Any).allow_headers(Any)` in `cors_layer` (`routes/middleware.rs:432-462`) with explicit lists: `[GET, POST, PUT, DELETE, PATCH, OPTIONS]` and `[CONTENT_TYPE, AUTHORIZATION, ACCEPT, X-API-Key, X-Request-Id, X-Roko-Session]`. Also fix the second `build_cors_layer` in `lib.rs:724-737`. → `prompts/T3-28.prompt.md`

---

## Group T4 — Feedback loop completion

**Wave 1** — independent.

- [ ] **T4-29** — Wire `KnowledgeIngestionSink::with_ingestor()` in `commands/plan.rs:389`. Implement `NeuroKnowledgeIngestor` adapter against `roko_neuro::admission::KnowledgeStore`. Construct shared `Arc<KnowledgeStore>` once at startup. → `prompts/T4-29.prompt.md`
- [ ] **T4-30** — Thread real `RoutingContext` through dispatch. Add to `AgentOutcome`, `RunnerEvent::TaskAttemptCompleted`, `FeedbackEvent::TaskCompleted`. `RoutingObservationSink` calls `observe_multi_objective` when context present, falls back to `record_confidence_outcome` when absent. → `prompts/T4-30.prompt.md`
- [ ] **T4-31a** — Anthropic provider parser preserves absent vs `Some(0)` usage. Use `UsageObservation { input_tokens: Option<u64>, ... }`. Handle both non-stream `usage` block and `message_delta` events. → `prompts/T4-31a.prompt.md`
- [ ] **T4-31b** — Ollama parser: map `prompt_eval_count` / `eval_count` to `UsageObservation`. → `prompts/T4-31b.prompt.md`
- [ ] **T4-31c** — Gemini parser: map `usageMetadata.{promptTokenCount,candidatesTokenCount,totalTokenCount}` preserving `None` when missing. → `prompts/T4-31c.prompt.md`
- [ ] **T4-31d** — Cerebras parser uses OpenAI-compatible. Verify it shares the OpenAI parser; if separate path, share. → `prompts/T4-31d.prompt.md`
- [ ] **T4-31e** — Cursor proxy: same as Cerebras (OpenAI-compatible). → `prompts/T4-31e.prompt.md`
- [ ] **T4-32** — `SystemPromptBuilder::playbooks(&[PlaybookEntry])` consumes top-3 playbook hits keyed by task fingerprint. Suppress section when empty. Wire from `dispatch_agent_with`. → `prompts/T4-32.prompt.md`
- [ ] **T4-33** — Add `runtime_feedback/rotator.rs` with `maybe_rotate(path, threshold=10 MiB, keep=5)`. Apply to episode / efficiency / knowledge sinks before each append. Drop cached file handle when rotation happens. → `prompts/T4-33.prompt.md`
- [ ] **T4-34** — Make chat `/model` switch atomic. Build full next state in temp values; commit to `self.agent_session` only after success. Audit every field that mutates: `model`, `model_call_config`, `display`, `provider`, `adapter`. Failed switch leaves all fields unchanged. → `prompts/T4-34.prompt.md`

---

## Group T5 — Architectural extraction

**Wave 1** — independent.

- [ ] **T5-35a** — Extract `select_model` from `dispatch_agent_with` (~335 lines, line 14575+). New module `orchestrate/dispatch/select_model.rs`. Mechanical move only; no logic change. Three-step commit: add module → delegate from dispatch_agent_with → done. → `prompts/T5-35a.prompt.md`
- [ ] **T5-36a** — Migrate `POST /api/inference/complete` (`routes/inference.rs`) to `state.model_call_service.call(...)`. Remove route-local `reqwest::Client::new()`. → `prompts/T5-36a.prompt.md`
- [ ] **T5-36b** — Migrate `POST /api/research/complete` (`routes/research.rs`) to `model_call_service`. Perplexity Sonar through shared dispatch. → `prompts/T5-36b.prompt.md`
- [ ] **T5-36c** — Migrate per-agent message dispatch in `routes/agents.rs` to `model_call_service`. → `prompts/T5-36c.prompt.md`
- [ ] **T5-36d** — Migrate diagnosis routes (`routes/diagnosis.rs`) to `model_call_service`. → `prompts/T5-36d.prompt.md`
- [ ] **T5-37** — Feature-gate `dispatch_direct.rs` behind `legacy-direct-dispatch`. Migrate production callers (`chat_inline.rs`, `unified.rs`, `marketplace.rs`, `identity_economy_markets.rs`) to `ModelCallService`. Default build excludes module. → `prompts/T5-37.prompt.md`
- [ ] **T5-39** — Wrap Ollama dispatch loop in `RunnerBudgetGuardrail` (`orchestrate.rs:15910-16011`). Reuse existing guardrail; if signature mismatch, add adapter — do not fork `TaskRunner`. → `prompts/T5-39.prompt.md`
- [ ] **T5-40a** — Add `RunLedgerEntry::Gate { rung, status, duration_ms, attempt, rationale }`. Write at gate observation loop. Report builder reads via `ledger.gate_verdicts_by_rung()`. → `prompts/T5-40a.prompt.md`
- [ ] **T5-41** — Migrate one demo scenario (`prd-pipeline.ts`) off prompt scraping. Subscribe to `/api/terminal/sessions/{id}/events` typed-event WebSocket. Success = `Exited.code === 0`. → `prompts/T5-41.prompt.md`
- [ ] **T5-42a** — Anthropic adapter: native messages array `[{role, content: [...blocks]}]`. Tool use / tool result blocks structured. → `prompts/T5-42a.prompt.md`
- [ ] **T5-42b** — Gemini adapter: native `contents: [{role, parts: [{text}|{functionCall}|{functionResponse}]}]`. → `prompts/T5-42b.prompt.md`
- [ ] **T5-42c** — Ollama chat endpoint: `messages: [{role, content}]` array (not flat string). → `prompts/T5-42c.prompt.md`

**Wave 2** — depends on a Wave-1 task.

- [ ] **T5-35b** *(deps: T5-35a)* — Extract `build_prompt` (~350 lines). New module `orchestrate/dispatch/build_prompt.rs`. → `prompts/T5-35b.prompt.md`
- [ ] **T5-35c** *(deps: T5-35b)* — Extract `launch_agent` (~330 lines). New module `orchestrate/dispatch/launch_agent.rs`. → `prompts/T5-35c.prompt.md`
- [ ] **T5-35d** *(deps: T5-35c)* — Extract `record_outcome` (~295 lines). New module `orchestrate/dispatch/record_outcome.rs`. After this slice, `dispatch_agent_with` body is ~80 lines. → `prompts/T5-35d.prompt.md`
- [ ] **T5-40b** *(deps: T5-40a)* — `RunLedgerEntry::Artifact { id, kind, outcome, path }`. Workflow finalizer fails if any required artifact has invalid/missing outcome. Delete legacy `artifact_valid` side fields. → `prompts/T5-40b.prompt.md`
- [ ] **T5-40c** *(deps: T5-40a)* — `RunLedgerEntry::Command { session_id, event, ts_ms }` for workflow-bound terminal sessions. Replay on resume. → `prompts/T5-40c.prompt.md`
- [ ] **T5-40d** *(deps: T5-40a)* — `RunLedgerEntry::Checkpoint { state, sequence, ts_ms }`. Resume reads latest checkpoint from ledger. Drop `executor.json` writes. → `prompts/T5-40d.prompt.md`

---

## Group S — Subsystem cross-cutting

### S-acp — ACP protocol completion (plan 21)

- [ ] **S-acp-1** — Replace `Unvalidated` diagnostics in `DispatchResolver` with typed `DispatchValidationError` (`MissingApiKey`, `ProviderKindMismatch`, `UnknownModel`, `StreamingNotSupported`). Call `validate_for_call` from every site that constructs a `DispatchPlan`. → `prompts/S-acp-1.prompt.md`
- [ ] **S-acp-2** *(deps: S-acp-1)* — Add `crates/roko-acp/tests/transcript_e2e.rs`. Mock `ModelCallService` stream. Assert `session/update` frames for happy path + typed `Failed` event. → `prompts/S-acp-2.prompt.md`
- [ ] **S-acp-3** *(deps: S-acp-1)* — Regression test: ClaudeCli does not require `ANTHROPIC_API_KEY`; Anthropic API does. Audit `bridge_events.rs` for `model_slug.starts_with("claude")` patterns; replace with explicit provider field check. → `prompts/S-acp-3.prompt.md`
- [ ] **S-acp-4** *(deps: S-acp-1)* — Rename or remove stale wrappers (`run_anthropic_cognitive_task`, `run_openai_cognitive_task` etc.) that look like raw HTTP/SSE sites. Update fitness allowlist accordingly. → `prompts/S-acp-4.prompt.md`

### S-config — Config validation pipeline (plan 23, T5-38)

- [ ] **S-config-1** — Add semantic validators to `config/validation.rs`: `validate_provider_auth`, `validate_unique_model_slugs`, `validate_gate_thresholds`, `validate_local_overrides`. Each returns `Result<(), ValidationError>`. → `prompts/S-config-1.prompt.md`
- [ ] **S-config-2** *(deps: S-config-1)* — Define `ResolvedConfig`, `ValidatedConfig`, `FieldProvenance`, `ConfigSource` in `config/provenance.rs`. `load_config` returns `ValidatedConfig`. Wire env var bindings. → `prompts/S-config-2.prompt.md`
- [ ] **S-config-3** *(deps: S-config-2)* — Migrate roko-cli callers to `ValidatedConfig`. → `prompts/S-config-3.prompt.md`
- [ ] **S-config-4** *(deps: S-config-2)* — Migrate roko-serve callers to `ValidatedConfig`. → `prompts/S-config-4.prompt.md`
- [ ] **S-config-5** *(deps: S-config-2)* — Migrate roko-acp callers to `ValidatedConfig`. → `prompts/S-config-5.prompt.md`
- [ ] **S-config-6** *(deps: S-config-2)* — `DangerousPermissionOverride` typed local-only bypass: reason / scope / expiry / source / acknowledgement_env. Strict validator rejects bare `dangerously_skip_permissions = true` in local file. → `prompts/S-config-6.prompt.md`
- [ ] **S-config-7** *(deps: S-config-2, S-config-6)* — `roko config doctor`: per-field provenance + validation warnings + local-overrides table. → `prompts/S-config-7.prompt.md`

### S-ledger — Run ledger plumbing (plan 24)

- [ ] **S-ledger-1** *(deps: T5-40a, T5-40b)* — Fail-closed persistence for gate/artifact ledger writes. Failure = `WorkflowOutcome::LedgerFailure`. Non-critical writes (command events) keep log-and-continue. → `prompts/S-ledger-1.prompt.md`
- [ ] **S-ledger-2** *(deps: T5-40a, T5-40b)* — Report builder reads from `RunLedger`. Delete event-replay paths in workflow_engine. → `prompts/S-ledger-2.prompt.md`

### S-learn — Learning feedback completion (plan 25)

- [ ] **S-learn-A** — Cascade router `learning_stage()` exposes stage / observations / threshold / top_models. CLI `roko show learning` (or `roko learn show`) prints. → `prompts/S-learn-A.prompt.md`
- [ ] **S-learn-B** *(deps: T4-29)* — `KnowledgeIngestionSink` failure budget: `AtomicU32` counters, `failure_budget_percent` (default 5). Log `error!` on breach but never abort the runner. JSONL write remains durable. → `prompts/S-learn-B.prompt.md`
- [ ] **S-learn-C** *(deps: T4-33)* — Episode JSONL: add `schema_version` field. Reader handles current + last-1 versions. → `prompts/S-learn-C.prompt.md`
- [ ] **S-learn-D** *(deps: T2-17a, T2-17b)* — Resolve `roko-learn::event_subscriber` and `roko-learn::feedback_service` overlap with `roko-cli::runtime_feedback`. Pick one canonical home; delete duplicate. → `prompts/S-learn-D.prompt.md`
- [ ] **S-learn-E** *(deps: T4-30, S-learn-A)* — Integration test: cascade router progresses ConfidenceOnly → Contextual after threshold observations with full `RoutingContext`. → `prompts/S-learn-E.prompt.md`

### S-term — Terminal demo truth (plan 26)

- [ ] **S-term-1** — `GET /api/terminal/sessions/{id}/events` typed-event WebSocket. Distinct from `/io` (PTY bytes). Auth-required. WS caps from T3-26. → `prompts/S-term-1.prompt.md`
- [ ] **S-term-2** *(deps: S-term-1)* — Frontend: `useTerminalEvents` hook + TypeScript `CommandEvent` types matching Rust serde shape. → `prompts/S-term-2.prompt.md`
- [ ] **S-term-3** *(deps: S-term-1, S-term-2, T5-41)* — Migrate `knowledge-transfer.ts` scenario off prompt scraping. → `prompts/S-term-3.prompt.md`
- [ ] **S-term-4** *(deps: S-term-1)* — Terminal lifecycle hardening: spawn fail closes WS with typed error; delete temp `ZDOTDIR` on `Cancelled` and `SpawnFailed`. → `prompts/S-term-4.prompt.md`
- [ ] **S-term-5** *(deps: S-term-1)* — Per-session `owner` field; reject reconnect with old `generation` counter. → `prompts/S-term-5.prompt.md`

### S-ci — CI fitness checks (plan 27)

- [ ] **S-ci-1** — `scripts/fitness/allowlist.toml` with initial entries: every current finding has owner + reason + expiry + linked plan. → `prompts/S-ci-1.prompt.md`
- [ ] **S-ci-2** *(deps: S-ci-1)* — `crates/roko-tooling/src/bin/allowlist_check.rs`. `--kind` + `--findings` + `--allowlist` args. Fails on new findings or expired entries. → `prompts/S-ci-2.prompt.md`
- [ ] **S-ci-3** *(deps: S-ci-1, S-ci-2)* — `.github/workflows/fitness.yml`: runs `roko-fitness-checks.sh check` on PR + push to main. → `prompts/S-ci-3.prompt.md`
- [ ] **S-ci-4** *(deps: S-ci-2)* — Promote `docs-status-check.sh` similarly: stale-claim guard with `docs-allowlist.toml`. → `prompts/S-ci-4.prompt.md`

### S-safety — Safety hardening (plan 28)

- [ ] **S-safety-1** — Audit all `permissive(...)` call sites. Each non-test site: replace with `restricted` or document. Mark all test-only sites `#[cfg(test)]`. → `prompts/S-safety-1.prompt.md`
- [ ] **S-safety-2** *(deps: S-safety-1)* — Wire `RecoveryAction` into dispatch failure path (`AskUser` → `RequestPermission`, `FallbackReadOnly` → retry with overlay, `Abort`, `Log`). → `prompts/S-safety-2.prompt.md`
- [ ] **S-safety-3** *(deps: S-safety-1)* — Per-session `SafetyOverlay` with `intersect()` semantics. Sessions narrow further than the role contract. → `prompts/S-safety-3.prompt.md`
- [ ] **S-safety-4** *(deps: S-safety-1)* — ACP dispatch enforces SafetyContract: regression test that blacklisted tool returns typed `SafetyError::ToolDenied`. → `prompts/S-safety-4.prompt.md`

### S-gate — Gate pipeline rungs 3, 5, 6 (plan 29)

- [ ] **S-gate-1** — Symbol gate (Rung 3): consume `SymbolGraph::scan` from roko-codeintel; diff vs persisted manifest. Skipped/Passed/Failed outcome. → `prompts/S-gate-1.prompt.md`
- [ ] **S-gate-2** — PropertyTest gate (Rung 5): run `cargo test --test property -- --include-ignored`. → `prompts/S-gate-2.prompt.md`
- [ ] **S-gate-3** — Integration gate (Rung 6): consume `tests/integration/` scenarios (YAML files defining steps + expected outcomes). → `prompts/S-gate-3.prompt.md`
- [ ] **S-gate-4** *(deps: S-gate-1, S-gate-2, S-gate-3)* — Wire all three gates into the orchestrate pipeline; record outcomes in RunLedger. → `prompts/S-gate-4.prompt.md`

### S-prompt — Prompt assembly completion (plan 30)

- [ ] **S-prompt-1** — Audit prompt construction sites (chat REPL, ACP, research, conductor, agent registration, dispatch). Produce site-by-site builder-vs-inline table; output to `logs/S-prompt-1-audit.md`. → `prompts/S-prompt-1.prompt.md`
- [ ] **S-prompt-2** *(deps: S-prompt-1)* — Migrate chat REPL initial prompt to full `SystemPromptBuilder`. → `prompts/S-prompt-2.prompt.md`
- [ ] **S-prompt-3** — Re-enable HDC similarity in context layer with `prompt.hdc_similarity_enabled` kill switch (default false). → `prompts/S-prompt-3.prompt.md`
- [ ] **S-prompt-4** — Centralize role prompt fragments in `crates/roko-prompt/src/role_text.rs` + per-role markdown assets. → `prompts/S-prompt-4.prompt.md`

### S-cog — Cognitive layer cleanup (plan 31)

- [ ] **S-cog-1** — Inventory pheromone + daimon callers; produce deletion plan. Output: `logs/S-cog-1-inventory.md`. → `prompts/S-cog-1.prompt.md`
- [ ] **S-cog-2** *(deps: S-cog-1)* — Implement `FailureTracker` in `roko-learn` (~2K LOC replacement for daimon). → `prompts/S-cog-2.prompt.md`
- [ ] **S-cog-3** *(deps: S-cog-2)* — Migrate daimon callers to `FailureTracker`. → `prompts/S-cog-3.prompt.md`
- [ ] **S-cog-4** *(deps: S-cog-3)* — Delete `roko-daimon` crate. Remove from workspace. → `prompts/S-cog-4.prompt.md`
- [ ] **S-cog-5** *(deps: S-cog-1)* — Delete pheromones (~68K LOC). Remove crate from workspace. → `prompts/S-cog-5.prompt.md`

### S-http — HTTP / persistence followups (plan 32)

- [ ] **S-http-1** — Audit + consolidate atomic write helpers (multiple `write_json` / `save_json` / `atomic_write` definitions exist). Single canonical helper in `roko-runtime` or `roko-fs`. → `prompts/S-http-1.prompt.md`
- [ ] **S-http-2** *(deps: S-http-1)* — `AtomicWriteSet` for transactional multi-file writes. Stage to temp dir, atomic-rename each into place. → `prompts/S-http-2.prompt.md`

### S-codeintel — Code intelligence followups (plan 33)

- [ ] **S-codeintel-1** — Persist symbol index to `.roko/index/symbols.json`. Today rebuilt fresh every dispatch. → `prompts/S-codeintel-1.prompt.md`
- [ ] **S-codeintel-2** *(deps: S-codeintel-1)* — Incremental updates by file fingerprint. Only re-parse changed files. → `prompts/S-codeintel-2.prompt.md`

### S-chain — Chain / deploy cleanup (plan 34)

- [ ] **S-chain-1** — Inventory dormant chain modules (marketplace.rs, identity_economy_markets.rs, pricing_*, etc.). Produce deletion plan: `logs/S-chain-1-inventory.md`. → `prompts/S-chain-1.prompt.md`
- [ ] **S-chain-2** — Add `contracts/broadcast/**/run-*.json` to `.gitignore`. Foundry artifacts shouldn't be committed. → `prompts/S-chain-2.prompt.md`

---

## Out of scope (deliberately not in this runner)

- **Forward-looking workflow redesign** (`40-..-42-...md`): 5-verb UX,
  ACP-as-universal-backend, WorkItem first-class. These are not bug
  fixes; they require the engine to be clean first. Pick up after this
  runner is green.
- **The actual deletion of `dispatch_direct.rs` file** (T5-37b): only
  feature-gating ships in this runner. Deletion follows after the
  static check is green for 30 days.
- **Foundry contract re-deploys** in `contracts/broadcast/`: those are
  artifacts, not code. S-chain-2 just `.gitignore`s them.

If you want any of these in scope, file a new batch in a follow-up runner.

---

## How to use this tracker

1. Pick a wave, kick the runner: `bash tmp/runners/audit-2026-05-01/run.sh --group T2`.
2. After a batch lands, re-grep the codebase to confirm the symbol/file/line
   referenced in this tracker actually changed (or is gone).
3. Tick the box only when (a) the batch merged green, (b) re-grep passes,
   (c) the verification block in the prompt passes.
4. Do NOT move an item to "done in tree" without a one-line line-number
   citation. The audit will fail otherwise.

## Status snapshot (as of runner creation)

| Group | Open | Done | Total |
|---|---|---|---|
| T2 | 7 | 0 | 7 |
| T3 | 7 | 0 | 7 |
| T4 | 10 | 0 | 10 |
| T5 | 16 | 0 | 16 |
| S | 52 | 0 | 52 |
| **Total** | **92** | **0** | **92** |

Plus 14 reference items at the top: T0-1 through T0-7 and T1-8 through
T1-15 are already done in tree (kept as `[x]` for regression-check
reference, not as work items).

3 of the 92 batches (S-prompt-1, S-cog-1, S-chain-1) are
**audit/inventory batches** — they produce documents under `logs/`
rather than code changes. Their sister batches depend on them.
