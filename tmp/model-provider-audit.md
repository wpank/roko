# Model/Provider Anti-Pattern Audit (2026-05-01)

## Status: IN PROGRESS (partial fixes 2026-05-03: batch 3 centralizes defaults, batch 4 concurrency safety, batch 5 stabilizes cascade model candidates, batch 6 removes ModelCallService provider synthesis, batch 7 fixes gateway provider-health keys, batch 8 wires direct agent-exec learning persistence, batch 9 shares direct learning persistence with ModelCallService dispatch, batch 10 removes ACP Anthropic env synthesis, batch 11 removes core Anthropic provider synthesis, batch 12 wires chat/inline ModelCallService feedback, batch 13 removes provider-factory command synthesis, batch 14 makes CLI command adapters explicit, batch 15 records direct provider chat feedback, batch 16 records vision evaluator feedback, batch 17 records dispatch-v2 bridge feedback, batch 18 records serve template dispatch feedback, batch 19 stops effective-model profile synthesis, batch 20 removes empty-config provider fallback, batch 21 extracts direct model-call feedback recorder, batch 22 stops CLI provider-kind routing for unknown models, batch 23 stops serve provider-health inference for unknown models, batch 24 rejects model profiles with missing providers, batch 25 makes ACP explicit config paths exact, batch 26 removes ACP static Anthropic/Sonnet fallback, batch 27 revalidates persisted ACP sessions, batch 28 shows unavailable ACP providers with status, batch 29 routes CLI config path helpers through core, batch 30 validates ACP provider/model updates, batch 31 makes demo roko commands workspace-explicit, batch 32 guards PRD pipeline terminal CWD, batch 33 adds PRD pipeline workspace E2E, batch 34 makes PRD draft success artifact-driven, batch 35 locks safety-contract trust boundaries, batch 36 covers configured model max_output ceilings, batch 37 adds top-level readiness and `roko up` graceful serve shutdown, batch 38 embeds safety contract assets, batch 39 removes roko-agent runtime request-timeout literals, batch 40 removes roko-cli runtime request-timeout literals, batch 41 removes roko-serve request-timeout literals, batch 42 removes roko-acp request-timeout literals, batch 43 centralizes core retry-policy defaults, batch 44 centralizes serve relay circuit-breaker defaults, batch 45 centralizes runner plan timeout/backoff defaults, batch 46 centralizes active provider tool-loop iteration defaults, batch 47 centralizes vision-loop defaults across CLI and serve)

## Implementation Log

### Batch 5 — Cascade candidate initialization and stale Cerebras slugs (2026-05-03)

**Done**:
- Added `RokoConfig::model_keys_for_cascade()` and `RokoConfig::model_slugs_for_cascade()` as config-derived candidate lists that do not depend on provider credential env vars.
- Kept `available_model_keys_for_cascade()` and `available_model_slugs_for_cascade()` as live dispatch/status filters only.
- Updated CLI learning runtime setup, `roko run` model discovery, serve startup cache warming, gateway routing, provider routing explanation, and orchestrator model routing to load/persist `CascadeRouter` with the stable configured slug set.
- Live routing still narrows candidates with `available_model_slugs_for_cascade()` when credentials are present, falling back to all configured slugs when no credentials are visible in the current process.
- Corrected root `roko.toml` Cerebras slugs to the provider-supported IDs: `gpt-oss-120b` and `llama3.1-8b`.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-core cascade_candidate_lists_include_configured_models_without_credentials`
- `cargo test -p roko-serve explain_routing_reports_scores_and_provider_health`
- `cargo test -p roko-cli learn_router_model_rows`
- `cargo check -p roko-orchestrator -p roko-serve -p roko-cli`

**Still open**:
- §9B/§9C: single-shot PRD and direct `--model` dispatch paths using `agent_exec` are now recorded by batch 8; non-`agent_exec` one-shot paths still need review.
- §10: provider health is now persisted for `agent_exec` captures by batch 8; non-`agent_exec` CLI call paths still need review.

### Batch 6 — Remove runtime provider/model synthesis from `ModelCallService` (2026-05-03)

**Done**:
- Removed `ModelCallService::config_for_model()` synthesis of `openai-compat` and `anthropic` providers from runtime inputs.
- `config_for_model()` now returns the explicit service `RokoConfig` clone only; provider/model definitions must come from config loading.
- Deprecated `with_openai_base_url()` because implicit OpenAI-compatible routing is no longer supported at this layer.
- Preserved `with_anthropic_api_key()` as service env passthrough for agent options, without mutating provider/model config.

**Checked**:
- `cargo fmt`
- `cargo test -p roko-agent config_for_model_does_not_synthesize_providers_from_runtime_inputs`
- `cargo check -p roko-agent`

**Still open**:
- `create_agent_for_model()` no longer synthesizes providers/models from known protocol commands as of batch 13.
- Core `RokoConfig::effective_providers()` Anthropic env compatibility synthesis was removed in batch 11.

### Batch 7 — Gateway provider-health records provider IDs, not model slugs (2026-05-03)

**Done**:
- Added gateway-side provider ID resolution from configured model key/slug to provider ID.
- `POST /api/inference/complete` now records failures against the requested provider and successes against the actual served provider.
- Batch inference records provider health the same way for each item.
- Updated the durable gateway test so model slug and provider ID differ; the test now asserts health is recorded under provider ID and not under model slug.

**Checked**:
- `cargo fmt`
- `cargo test -p roko-serve test_gateway_writes_durable_events`
- `cargo check -p roko-serve`

**Still open**:
- CLI/direct paths that do not use `agent_exec::persist_capture_episode()` still need provider-health observation plumbing.
- `dispatch_via_model_call_service()` and chat/inline API-mode turns now write the same persisted provider-health registry. Remaining direct `create_agent_for_model()` callers still need review.

### Batch 8 — Direct agent-exec learning persistence (2026-05-03)

**Done**:
- Moved direct agent execution feedback from `.roko/memory` to the canonical `.roko/learn` runtime store.
- Direct captures now resolve configured model keys to provider API slugs before building learning episodes, so `CascadeRouter` confidence stats update the same slug arms used by routing.
- Direct captures open `LearningRuntime` with `RokoConfig::model_slugs_for_cascade()` plus the resolved episode model, keeping PRD/research/plan one-shot observations inside the configured model universe.
- Persisted `.roko/learn/provider-health.json` is updated for direct captures using configured provider IDs, so a successful forced run can close the provider circuit in the config/TUI health surface.
- `run_agent_capture_logged()` now passes the resolved slug into persistence instead of the model key.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo check -p roko-cli`
- `cargo test -p roko-cli --lib persist_capture_episode`

**Known verification caveat**:
- `cargo test -p roko-cli persist_capture_episode` still attempts to link the full CLI test binary in this workspace and fails in the environment linker with unresolved anonymous LLVM symbols; the library-only target above exercises the added tests and passes.

**Still open**:
- Chat/inline API-mode turns are now covered by batch 12; remaining review should focus on direct `create_agent_for_model()` callers that bypass `agent_exec`, `dispatch_via_model_call_service()`, and `ChatAgentSession::send_turn_api()`.
- Consider extracting the remaining direct capture behavior into a service object if more one-shot call surfaces need the same persistence.

### Batch 9 — Direct ModelCallService dispatch learns through shared helpers (2026-05-03)

**Done**:
- Moved the direct learning helper logic from `agent_exec` into `learning_helpers` so one-shot call surfaces share model slug resolution, configured provider ID lookup, cascade candidate construction, and persisted provider-health writes.
- Updated `dispatch_via_model_call_service()` to load a `CascadeRouter` from `.roko/learn/cascade-router.json`, attach it to `FeedbackService`, and save the router after the model call.
- Updated `dispatch_via_model_call_service()` to persist provider-health success/failure observations in `.roko/learn/provider-health.json` using provider IDs resolved from config.
- Kept failure-path provider-health persistence best-effort so the original model-call error remains the surfaced error.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo check -p roko-cli`
- `cargo test -p roko-cli --lib persist_capture_episode`

**Still open**:
- Chat/inline API-mode turns are now covered by batch 12. Direct `create_agent_for_model()` callers outside `agent_exec` / `dispatch_via_model_call_service()` / `ChatAgentSession::send_turn_api()` still need review.
- The full `cargo test -p roko-cli ...` command still hits the workspace/environment linker issue described in batch 8; library-only coverage passes.

### Batch 10 — ACP requires explicit Anthropic provider config (2026-05-03)

**Done**:
- Removed ACP bridge-local synthesis of an `AnthropicApi` provider from `ANTHROPIC_API_KEY`.
- Changed `anthropic_model_call_config()` to use explicit `roko_config.providers` instead of `effective_providers()`, because `effective_providers()` still has core backward-compat synthesis.
- Updated ACP dispatch comments to state that Anthropic API providers must be explicitly configured.
- Added a regression test showing ACP ignores providers synthesized by `effective_providers()` from legacy `agent.env` values.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo check -p roko-acp`
- `cargo test -p roko-acp --lib anthropic_model_call_config`

**Still open**:
- Provider-factory command synthesis was removed in batch 13; CLI command adapters were made explicit in batch 14.

### Batch 11 — Core effective providers stop synthesizing Anthropic (2026-05-03)

**Done**:
- Removed `RokoConfig::effective_providers()` insertion of an Anthropic API provider from process `ANTHROPIC_API_KEY`.
- Removed empty-config fallback insertion of an Anthropic API provider from legacy `agent.env` / `ANTHROPIC_BASE_URL` values.
- Kept the legacy `claude_cli` default provider fallback intact for empty configs; this batch only removes the env-driven Anthropic API provider synthesis.
- Updated ACP regression test expectations now that core no longer synthesizes the effective provider.
- Added a core unit test proving `agent.env` Anthropic values do not create an Anthropic provider while the `claude_cli` fallback remains.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-core effective_providers_do_not_synthesize_anthropic_from_agent_env`
- `cargo test -p roko-acp --lib anthropic_model_call_config`
- `cargo check -p roko-core -p roko-acp -p roko-agent -p roko-cli -p roko-serve`

**Still open**:
- `effective_models()` still synthesizes model profiles for legacy default/tier model slugs.
- CLI command-backed compatibility now produces explicit transient provider/model entries at the boundary as of batch 14; the remaining model synthesis is core `effective_models()`.

### Batch 12 — Chat/inline API turns persist ModelCallService feedback (2026-05-03)

**Done**:
- Added `ChatFeedbackRuntime` for `ChatAgentSession::send_turn_api()` so chat API-mode turns use the same `.roko/learn` feedback surfaces as direct ModelCallService dispatch.
- Chat API-mode turns now load `.roko/learn/cascade-router.json`, attach the router to `FeedbackService`, flush feedback after success/failure/cancellation, and save cascade observations.
- Chat API-mode turns now persist `.roko/learn/provider-health.json` success/failure observations keyed by configured provider ID.
- `chat_inline` session mode is covered because non-CLI inline turns call `ChatAgentSession::send_turn_streaming()`, which delegates to `send_turn_api()` and emits synthetic stream events.
- Preserved the missing-key early return for API providers; non-API session providers (`claude_cli`, `cursor_acp`) skip that preflight so test/local provider-backed ModelCallService paths can exercise the same feedback code without env mutation.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-cli --lib send_turn_api_records_chat_feedback_and_provider_health`
- `cargo check -p roko-cli`

**Still open**:
- Direct provider construction paths that bypass `agent_exec`, `dispatch_via_model_call_service()`, and chat session API-mode dispatch still need review.
- `effective_models()` remains open.

### Batch 13 — Provider factory requires explicit config for protocol commands (2026-05-03)

**Done**:
- Removed the `create_agent_for_model()` branch that synthesized `ProviderConfig` / `ModelProfile` pairs from known protocol commands such as `claude`, `codex`, and `cursor-agent`.
- The provider factory now accepts protocol commands only when the model resolves through explicit config or the existing core effective default provider/model path.
- Unknown model keys with known protocol commands now return `AgentCreationError::MissingConfig` instead of silently inventing provider/model config.
- Generic non-protocol command fallback to `ExecAgent` remains unchanged for explicit raw subprocess use.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-agent create_agent_for_model_rejects_protocol_command_without_model_config`
- `cargo test -p roko-agent create_agent_for_model_uses_effective_claude_provider_for_configured_model`
- `cargo check -p roko-agent`

**Still open**:
- CLI command-backed compatibility helpers were migrated in batch 14 to explicit transient provider/model entries.
- `effective_models()` still synthesizes model profiles from legacy default/tier model slugs.

### Batch 14 — CLI command adapters build explicit transient config (2026-05-03)

**Done**:
- Changed `agent_config` command-backed compatibility helpers to populate explicit transient `[providers]` and `[models]` entries before calling the provider factory.
- Claude and Cursor command-backed paths now pass a provider with a command field and a model profile pointing at that provider, instead of relying on provider-factory or `effective_models()` inference.
- Codex/OpenAI-compatible command-backed paths now carry an explicit `openai_compat` provider with `OPENAI_API_KEY` as the required API key env.
- Generic raw subprocess compatibility remains separate through `synthesize_subprocess_config()` and the provider factory's non-protocol `ExecAgent` fallback.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-cli --lib synthesize_`
- `cargo check -p roko-cli`

**Still open**:
- Core `RokoConfig::effective_models()` still fills model profiles from `agent.default_model` and `agent.tier_models`.
- Function names still contain `synthesize_*` for compatibility; behavior is now a centralized legacy adapter that builds explicit config rather than hidden provider-factory synthesis.

### Batch 15 — Direct provider chat records feedback and health (2026-05-03)

**Done**:
- Updated `run_direct_provider_chat()` so direct provider REPL turns record `FeedbackEvent::ModelCall` into `.roko/learn/efficiency.jsonl`.
- Attached a persisted `.roko/learn/cascade-router.json` router to that feedback path and saves it when the REPL exits.
- Persisted `.roko/learn/provider-health.json` success/failure outcomes by configured provider ID for each direct provider chat turn.
- Kept feedback/provider-health write failures non-fatal so the REPL does not lose a provider response because learning persistence failed.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo check -p roko-cli`

**Still open**:
- Other direct `create_agent_for_model()` callers still need review: dispatch-v2 factory paths and serve template dispatch.
- Core `RokoConfig::effective_models()` still fills model profiles from default/tier model slugs.

### Batch 16 — Vision evaluator records feedback and health (2026-05-03)

**Done**:
- Updated `VisionEvaluator` so each multimodal model call records a `FeedbackEvent::ModelCall` into `.roko/learn/efficiency.jsonl`.
- Attached `.roko/learn/cascade-router.json` to the vision feedback path and saves cascade observations after each evaluation.
- Persisted `.roko/learn/provider-health.json` outcomes by configured provider ID for the vision model call.
- Treats invalid/unparseable vision JSON as a learning failure while keeping provider health tied to the provider call's technical success.
- Passes the resolved project root into the evaluator from the loop orchestrator so all learning writes land under the same `.roko/learn` tree as checkpoints.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-cli --lib evaluate_records_feedback_and_provider_health`
- `cargo check -p roko-cli`

**Still open**:
- Remaining direct `create_agent_for_model()` caller: serve template dispatch.
- Dispatch-v2 bridge paths are covered by batch 17.
- Core `RokoConfig::effective_models()` still fills model profiles from default/tier model slugs.

### Batch 17 — Dispatch-v2 bridge records feedback and health (2026-05-03)

**Done**:
- Added one shared dispatch-v2 feedback recorder for provider-factory bridge calls.
- `run_agent_result_bridge()`, `run_agent_streaming()`, and `run_agent_result_bridge_with_mcp()` now record `.roko/learn` `FeedbackEvent::ModelCall` entries after provider execution.
- Attached and saved `.roko/learn/cascade-router.json` for dispatch-v2 bridge observations.
- Persisted `.roko/learn/provider-health.json` outcomes by resolved provider ID.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-cli --lib run_agent_result_bridge_records_feedback_and_provider_health`
- `cargo check -p roko-cli`

**Still open**:
- Direct `create_agent_for_model()` callers identified in this audit section are now covered for learning/provider-health feedback.
- Core `RokoConfig::effective_models()` still fills model profiles from default/tier model slugs.

### Batch 18 — Serve template dispatch records feedback and health (2026-05-03)

**Done**:
- Added template-dispatch feedback recording in `dispatch_template()` after the agent result and template gates are known.
- Persisted `.roko/learn/efficiency.jsonl` model-call feedback with role `template_dispatch`.
- Persisted `.roko/learn/provider-health.json` outcomes by configured provider ID and updated `AppState::provider_health` in memory.
- Saved cascade-router observations for template dispatch; global serve dispatch uses the cached `AppState` router to avoid shutdown overwriting the latest observations, while repo-specific dispatches write to the repo layout.
- Uses final template/gate success as the learning reward and provider call success as the provider-health signal.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-serve --lib template_dispatch_records_feedback_and_provider_health`
- `cargo check -p roko-serve`

**Still open**:
- Consider extracting the repeated direct feedback/provider-health recorder into a shared crate/service now that all direct call surfaces use the same persistence pattern.
- Legacy configs that only set `agent.default_model`/`agent.tier_models` now receive explicit validation warnings and should be migrated to `[models.*]`.

### Batch 19 — Stop core effective model profile synthesis (2026-05-03)

**Done**:
- Changed `RokoConfig::effective_models()` to return only explicit `[models.*]` profiles.
- `agent.default_model`, `agent.fallback_model`, and `agent.tier_models.*` are now validated as references to configured model keys instead of silently creating profiles.
- CLI semantic validation now warns on missing default/tier/fallback model references.
- Kept environment override handling as a boundary adapter: when env overrides mutate a model, they still materialize an explicit `config.models` entry.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `cargo test -p roko-core effective_models_do_not_synthesize_default_or_tier_profiles`
- `cargo test -p roko-core effective_models_backwards_compat`
- `cargo test -p roko-core validate_references`
- `cargo test -p roko-agent --lib create_agent_for_model_rejects_protocol_command_without_model_config`
- `cargo check -p roko-core -p roko-agent -p roko-cli -p roko-serve`

**Known verification caveat**:
- `cargo test -p roko-cli --lib semantic_validate_warns_for_legacy_default_tier_and_fallback_models` attempted to link the full `roko-cli` lib test binary and failed in the environment linker with unresolved anonymous LLVM symbols. The same class of linker failure was already observed in earlier CLI full-test attempts; compile checks pass and the updated core tests exercise the changed model-registry behavior.

**Still open**:
- Direct model-call feedback now has a shared recorder as of batch 21; remaining work should focus on the larger config-loader and heuristic model-backend issues.

### Batch 20 — Stop empty-config provider fallback (2026-05-03)

**Done**:
- Changed `RokoConfig::effective_providers()` so an empty provider table returns an empty registry instead of inventing a `claude_cli` provider.
- Kept existing explicit-provider behavior: when `[providers.claude_cli]` exists, compatibility command defaulting for that explicit provider still fills from `agent.command` or `claude`.
- Updated the core regression test so legacy `agent.env` Anthropic values plus an empty provider table yield no effective providers at all.
- Documented the boundary rule in code: command-backed compatibility must materialize transient providers before dispatch, not rely on hidden core defaults.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch20 cargo test -p roko-core effective_providers_do_not_synthesize_empty_config_providers`
- `CARGO_TARGET_DIR=target/codex-batch20 cargo test -p roko-core effective_providers_backwards_compat`
- `CARGO_TARGET_DIR=target/codex-batch20 cargo test -p roko-agent --lib create_agent_for_model_rejects_protocol_command_without_model_config`
- `CARGO_TARGET_DIR=target/codex-batch20 cargo check -p roko-core -p roko-agent -p roko-cli -p roko-serve -p roko-acp`

**Notes**:
- Verification used an isolated `CARGO_TARGET_DIR` because existing `cargo-watch` dev processes were holding the default target lock.
- Root `roko.toml` compatibility still passes because the repo config declares explicit providers and models.

**Still open**:
- Direct model-call feedback now has a shared recorder as of batch 21; remaining work should focus on the larger config-loader and heuristic model-backend issues.

### Batch 21 — Shared direct model-call feedback recorder (2026-05-03)

**Done**:
- Added `roko_learn::model_call_feedback::ModelCallFeedbackRecorder` as the shared durable path for direct model-call feedback.
- The recorder writes `FeedbackEvent::ModelCall`, flushes `.roko/learn/efficiency.jsonl`, persists `.roko/learn/provider-health.json`, and saves cascade-router observations.
- Moved the model-call cascade observation context vector out of `FeedbackService` so service-attached routers and direct recorder users share the same reward observation shape.
- Updated direct provider chat, vision evaluator dispatch, dispatch-v2 bridge dispatch, and serve template dispatch to use the recorder instead of duplicating feedback/health/router persistence blocks.
- Kept serve's global template-dispatch behavior intact by observing through the cached `AppState` router before recording feedback without a local router, avoiding shutdown overwrites of fresh observations.
- Made the older CLI provider-health helper delegate into `roko-learn` so remaining `ModelCallService` paths share the same provider-health writer.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-learn recorder_writes_feedback_health_and_cascade_router`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-cli --lib run_agent_result_bridge_records_feedback_and_provider_health`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-cli --lib evaluate_records_feedback_and_provider_health`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-serve --lib template_dispatch_records_feedback_and_provider_health`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo check -p roko-learn -p roko-cli -p roko-serve`

**Notes**:
- Verification again used an isolated target dir because dev `cargo-watch` processes hold the default target lock.
- Existing `roko-learn` tests still emit pre-existing warnings for missing docs in `tests/cost_comparison.rs` and an unused `Score` import in `verdict_scorer.rs`; they are unrelated to this batch.

**Still open**:
- Larger model/provider redesign issues still open: unified config loading/global config merge, removing heuristic backend fallback, and any remaining UI/demo hardcoded provider references.

### Batch 22 — CLI model selection requires explicit model profiles (2026-05-03)

**Done**:
- Changed `roko-cli` model selection so an unresolved model slug no longer routes by inferred provider kind.
- `resolve_effective_model()` now returns `UnknownModel` unless the selected model resolves to an explicit `[models.*]` profile.
- Kept the inferred provider kind only in the error message as diagnostic context.
- Updated model-selection precedence tests to declare explicit Claude provider/model fixtures instead of relying on core default/effective-model synthesis.
- Added a regression test proving `--model gpt-new-unconfigured` does not route through a configured `openai_compat` provider without a matching model profile.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-cli --lib model_selection::tests`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo check -p roko-cli`

**Still open**:
- Core `resolve_model()` and `AgentBackend::from_model()` still expose legacy slug inference for compatibility and diagnostics; remaining production call sites should be migrated to explicit-profile checks or a strict resolver.
- Unified config loading/global config merge remains the main blocker for non-roko projects and ACP/Zed behavior.

### Batch 23 — Serve surfaces stop provider inference for unknown models (2026-05-03)

**Done**:
- Removed `routes/agents.rs` dashboard fallback that inferred provider health from a raw model slug via `AgentBackend::from_model()`.
- Changed gateway provider-health lookup to return `None` unless a model key/slug maps to an explicit `[models.*]` profile.
- Gateway success/failure health updates now skip provider-health mutation for unconfigured model strings instead of recording under an inferred provider kind.
- Added a regression test proving a configured `openai_compat` provider does not make an unconfigured GPT-like slug eligible for provider-health recording.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-serve --lib provider_id_for_model_requires_configured_model_profile`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-serve --lib test_gateway_writes_durable_events`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo check -p roko-serve`

**Still open**:
- Core heuristic APIs remain and should be retired or quarantined behind an explicit legacy adapter after remaining call sites are audited.
- Unified config loading/global config merge remains open.

### Batch 24 — Provider factory rejects missing referenced providers (2026-05-03)

**Done**:
- Changed `create_agent_for_model()` so a configured model profile that references a missing provider returns `AgentCreationError::MissingConfig`.
- Preserved the raw `ExecAgent` fallback only for cases with no configured model profile and no known protocol command.
- Added a regression test for `[models.custom-model] provider = "missing-provider"` with no matching `[providers.missing-provider]`.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-agent --lib create_agent_for_model_rejects_model_profile_with_missing_provider`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo test -p roko-agent --lib create_agent_for_model_rejects_protocol_command_without_model_config`
- `CARGO_TARGET_DIR=target/codex-batch21 cargo check -p roko-agent`

**Still open**:
- Raw subprocess fallback still exists for unconfigured non-protocol commands by design; decide later whether that should become an explicit `subprocess` provider kind.
- Unified config loading/global config merge remains open.

### Batch 25 — ACP explicit config path uses exact file (2026-05-03)

**Done**:
- Added `roko_core::config::loader::load_config_file(path, opts)` for callers that already have an explicit config file path.
- The exact-file loader bypasses `ROKO_CONFIG` path discovery and ancestor `roko.toml` discovery while preserving the requested processing options: global merge, `ROKO__*` overrides, interpolation, file secrets, and strict validation.
- Changed `AcpConfig::load_roko_config()` so `roko acp --config /path/to/file.toml` loads that file directly instead of loading from the file's parent directory and rediscovering a sibling/ancestor `roko.toml`.
- Added core and ACP regression coverage proving a nonstandard explicit config filename wins over a parent `roko.toml`.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-core --test config_loader_integration load_config_file_uses_exact_nonstandard_path`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib load_roko_config_uses_explicit_config_path`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-core -p roko-acp`

**Still open**:
- CLI `load_layered()` still owns provenance/repo-registry behavior and duplicates path/global helper functions; migrate its path helpers to the core loader or split provenance from loading in a later batch.
- ACP still has static Anthropic/Sonnet fallback options when the resolved config is empty; remove that next under the ACP editor integration work.

### Batch 26 — ACP empty configs do not invent Anthropic/Sonnet (2026-05-03)

**Done**:
- Removed `build_config_options_static()` from `roko-acp`.
- `SessionConfigState::default()` now starts with empty provider/model selections instead of hardcoded `anthropic` / `sonnet`.
- `SessionConfigState::from_roko_config()` now returns empty provider/model selections when the resolved config has no models/providers, rather than fabricating a fallback model.
- `AcpSession::new()` and `new_with_config()` now both use the same config-option builder, so empty configs produce empty provider/model option lists while non-model options remain available.
- Added regression tests proving empty ACP configs and legacy no-config session construction do not expose static Anthropic/Sonnet options.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib static_provider_or_model`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib session::tests`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-acp`

**Still open**:
- Persisted session load still needs revalidation against the current config so stale provider/model keys are reset or surfaced clearly.
- ACP provider/model options still hide providers whose credentials are unavailable; a later provider-health/status batch should show unavailable configured providers with an actionable status instead of silently filtering them out.

### Batch 27 — ACP persisted sessions revalidate provider/model state (2026-05-03)

**Done**:
- Added `AcpSession::revalidate_config_state()` to reconcile persisted provider/model selections with the current `RokoConfig`.
- `SessionManager::load_from_disk()` now revalidates after deserializing a session and rebuilds `config_options` from the current config before returning the resumed session.
- If the persisted provider is missing, ACP resets provider/model to current config defaults.
- If the provider still exists but the persisted model is missing or belongs to a different provider, ACP resets the model to the first configured model for that provider, or clears it when none exists.
- Added regression tests for stale provider+model and stale model under a still-valid provider.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib stale_persisted`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib session::tests`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-acp`

**Still open**:
- ACP does not yet watch project/global config files and push updated options into already-live sessions.
- ACP provider/model options still silently filter unavailable providers instead of surfacing no-key/unreachable status.

### Batch 28 — ACP config options show unavailable providers with status (2026-05-03)

**Done**:
- Changed ACP provider options to include every configured provider instead of filtering out providers whose credentials are currently unavailable.
- Provider option descriptions now show `Ready`, `API key env <NAME> is not set`, `API key env is not configured`, or `Unavailable`.
- Model options now list models for the selected provider even when that provider is missing credentials, so the user can inspect/select the configured graph before fixing env.
- Added regression coverage for an OpenAI-compatible provider with an unset `api_key_env`; ACP now shows both provider and model with actionable status text.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib config_options_include_unavailable_configured_providers_with_status`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib session::tests`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-acp`

**Still open**:
- ACP does not yet perform live endpoint health checks, so status is credential/config based rather than network reachability based.
- Live config watch/reload remains open.

### Batch 29 — CLI config path helpers delegate to core loader (2026-05-03)

**Done**:
- Changed `roko-cli::config::global_config_path()` to delegate to `roko_core::config::loader::global_config_path()`.
- Changed `roko-cli::config::discover_project_config()` to delegate to the core loader discovery function.
- Changed `roko-cli::config::merge_global_providers()` to delegate to `roko_core::config::loader::merge_global_into()`.
- Preserved `load_layered()` and its CLI-specific provenance/source/repo-registry behavior; this batch only removes duplicated helper implementations.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-cli --lib global_path_ends_in_roko_config_toml`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-cli --lib discover_project_config_walks_upward`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-cli`

**Still open**:
- `load_layered()` still parses and merges `ConfigLayer` independently for provenance. A later batch should split source/provenance reporting from the core effective-config loading path so CLI and non-CLI callers share the same loader end to end.

### Batch 30 — ACP config update validation (2026-05-03)

**Done**:
- ACP `update_config("provider", ...)` now ignores provider IDs that are not in the current `RokoConfig` instead of accepting arbitrary strings.
- ACP `update_config("model", ...)` now ignores model IDs that are missing or that belong to a different provider than the currently selected provider.
- Provider changes still select the first configured model for the new provider when the current model does not belong to it; if the provider has no models, the model is cleared.
- Added regression tests for unknown provider, unknown model, and cross-provider model update attempts.

**Checked**:
- `cargo fmt`
- `git diff --check`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib update_config_ignores`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo test -p roko-acp --lib session::tests`
- `CARGO_TARGET_DIR=target/codex-batch25 cargo check -p roko-acp`

**Still open**:
- ACP still needs provider endpoint health checks and model slug pre-validation for configured-but-bad slugs.
- Live config watch/reload remains open.

## Issues Found

### 1. Deprecated/Stale Model Slugs in Hardcoded Defaults
- `roko-core/config/agent.rs:240` — `"claude-haiku-3-5"` (data LLM default, stale)
- `roko-learn/cascade/helpers.rs:56` — `"claude-haiku-3-5"` (fast tier, stale)
- `roko-learn/cascade/helpers.rs:59` — `"claude-opus-4"` (premium tier, missing `-6`)
- `roko-core/config/presets.rs` — 11 hardcoded model names across minimal/thorough presets
- Cerebras models — FIXED: `llama-3.3-70b` → `gpt-oss-120b`, `llama-4-scout-17b-16e` → `gpt-oss-120b`, `llama-3.1-8b` → `llama3.1-8b` in root `roko.toml`
- **NOTE (2026-05-03)**: Verified `routing.rs` defaults are correct (haiku-4-5, sonnet-4-6, opus-4-6). Role-model table in `helpers.rs` already uses current slugs and is parameterized by available model_slugs. Main remaining stale slugs are in tests/examples (non-production).

### 2. Provider Inference from Model Slug (worst anti-pattern)
- `roko-core/src/agent.rs:126-168` — `AgentBackend::from_model()` + `is_cursor_slug()`
- ~~`starts_with("sonnet-")` / `"gemini-"` routes to Cursor (collides with Claude/Gemini)~~
- Unrecognized slugs silently default to Codex
- **PARTIAL FIX (2026-05-03)**: `gemini-*` no longer routed to Cursor (moved before Cursor check, routes to Codex/OpenAI-compat). `is_cursor_slug()` updated. Doc comment updated. `from_model()` is now deprecated-in-docs; production should use config-backed resolution. Batch 22 removed the CLI model-selection path that chose a provider by inferred kind for unknown slugs. Batch 23 removed serve dashboard/gateway provider-health inference for unknown slugs. The heuristic still exists in core and should be removed after remaining compatibility call sites are migrated.

### 3. Hardcoded Role-to-Model Table in Cascade Router
- `roko-learn/src/cascade/helpers.rs:27-91` — 30+ hardcoded model slugs for stage-1 defaults
- **FIX**: Move to `[routing.role_defaults]` in roko.toml
- **NOTE (2026-05-03)**: Verified this is actually a well-designed priority-ordered preference table parameterized by `model_slugs: &[String]` (the configured model list). Uses `pick_static_slug` which only selects from actually-configured models. The "hardcoded" slugs are fallback preferences, not fixed mappings. Architecture is sound; config override would be nice-to-have but low priority.

### 4. Substring-Based Tier Detection
- `roko-learn/src/cascade/helpers.rs:121-135` — `slug_to_tier()` uses `contains("haiku")`, etc.
- **FIX**: Add `tier` field to `ModelProfile`. Read from config instead of guessing.
- **PARTIAL (2026-05-03)**: Config-driven tier map already exists and takes precedence; heuristic is fallback only. 6 hardcoded 4096 content-truncation literals migrated to `defaults::DEFAULT_TOOL_OUTPUT_TRUNCATE_AT`.

### 5. Second Model Resolution System (Enrichment)
- `roko-cli/src/orchestrate.rs:1832-1860` — `resolve_enrichment_backend()`
- Completely separate routing with substring matching; `"gemini"` → Codex (wrong)
- **FIX**: Delete. Use `create_agent_for_model()` path.

### 6. Runtime Config Synthesis
- `roko-agent/src/model_call_service.rs:321-397` — `config_for_model()`
- Synthesizes providers from env vars, bypasses roko.toml
- **FIX**: Remove synthesis. If provider not configured, error out.
- **FIXED in batch 6, batch 10, batch 11, batch 13, batch 14, batch 19, and batch 20 (2026-05-03)**: `ModelCallService::config_for_model()` no longer synthesizes providers/models from `ANTHROPIC_API_KEY` or `openai_base_url`; it returns the explicit service config. `with_openai_base_url()` is deprecated. ACP no longer synthesizes an Anthropic provider from env/effective-provider compatibility. Core `RokoConfig::effective_providers()` no longer synthesizes Anthropic providers from env and no longer creates a default `claude_cli` provider for empty configs. `create_agent_for_model()` no longer synthesizes providers/models from known protocol commands. CLI command-backed compatibility helpers now build explicit transient provider/model entries at the boundary. Core `effective_models()` no longer fills default/tier model profiles; legacy default/tier fields are validated as references to explicit `[models.*]` entries.

### 7. Eight Different `load_roko_config()` Implementations
- CLI main.rs: Load + merge_global_providers
- CLI orchestrate.rs: Plain load, no merge
- Serve lib.rs: Load + fallback ~/.roko/config.toml (different logic)
- ACP config.rs: Walk-up directory tree
- **FIX**: Single `load_config()` in roko-core with unified merge + global + env
- **PARTIAL (batch 25, 2026-05-03)**: Added `load_config_file()` to the core loader and wired ACP `--config` through it so explicit editor config files are exact paths, not parent-directory rediscovery.
- **PARTIAL (batch 26, 2026-05-03)**: Removed ACP's static Anthropic/Sonnet option fallback for empty configs.
- **PARTIAL (batch 27, 2026-05-03)**: ACP persisted sessions now revalidate provider/model selections against the current config on resume. Remaining work is removing CLI path/global helper duplication and adding live config change detection/status surfacing.
- **PARTIAL (batch 29, 2026-05-03)**: CLI path/global helper functions now call the core loader helpers. Remaining loader work is consolidating `load_layered()` effective loading and provenance around the core path.

### 8. Demo/UI Hardcoded References
- `prd-pipeline-types.ts:151-153` — tier routing by model.includes('opus')
- `palette.ts` — color mappings keyed to slug prefixes
- `scenario-runners/providers.ts` — hardcoded provider list

### 9. Cascade Router Marks Usable Models as "(unavailable)" (2026-05-03)

**Symptom**: `roko learn all` shows `glm-5-1 (unavailable): 0 obs, 0 successes` even though `--model glm-5-1` successfully dispatches agents.

**Observed**:
```
model: glm-5-1 via zai (source: cli override)
roko_agent::provider: creating agent via provider adapter model_key="glm-5-1" slug=glm-5.1 provider=openai_compat base_url=Some("https://api.z.ai/api/paas/v4")
```
Agent runs, completes, writes the draft. But the cascade router shows it as unavailable with 0 observations.

**Root cause chain** (3 independent issues):

**A. Init-time env var gate filters out configured models.**

`schema.rs:496-508` — `available_model_slugs_for_cascade()` calls `provider_available_for_model_key()` which calls `is_provider_available()`. For HTTP providers, availability is gated on `std::env::var(api_key_env).is_ok()`. If the env var isn't set when the cascade router initializes (e.g. the server loads before the env is populated, or the env var is set in a different process context), the model is filtered out of `model_slugs` entirely.

Once filtered at init, the model can never appear in routing candidates, so it never gets selected, so it never records an observation. Self-reinforcing exclusion.

```rust
// schema.rs — is_provider_available()
Some(name) => std::env::var(name).is_ok()     // ← one-shot check at init
    || self.agent_env_value(name).is_some(),
```

**Status (batch 5, 2026-05-03)**: Fixed the router-arm part of this issue without adding a TTL cache. Router persistence/initialization now uses `model_slugs_for_cascade()` / `model_keys_for_cascade()`, which are independent of env vars. Dispatch-time selection and status displays still use the credential-gated `available_*_for_cascade()` methods.

**B. CLI `--model` overrides bypass the cascade router entirely.**

When the user forces a model via `--model glm-5-1`, the dispatch path resolves the model directly via `resolve_model()` and creates the agent via `create_agent_for_model()`. The cascade router is never consulted and never told about the outcome. The comment in `orchestrate.rs:995-996` says:
```rust
// UX34: outcome is persisted to the cascade router's confidence
// stats via record_outcome() in record_task_success/failure.
```
But this only applies to `plan run` (orchestration loop). Single-shot commands like `prd draft new` and `prd plan` never call `record_observation()` on the cascade router at all — they don't even instantiate a `LearningSubsystem`.

**C. PRD commands don't participate in the learning loop.**

`prd.rs` and `commands/prd.rs` import `EpisodeLogger` but never touch the cascade router. Agent dispatches from PRD commands are invisible to the learning subsystem:
- No observation is recorded (success or failure)
- No provider health is updated
- No efficiency event is emitted

The cascade router only learns from `plan run` orchestration. Everything else is a blind spot.

**Impact**: The `roko learn all` display is misleading. Models that work fine via `--model` show as unavailable. Users see a dashboard of "(unavailable)" models and think they're broken, when the real issue is the learning system doesn't observe those code paths.

**Fix** (structural):

1. **Late-binding availability**: Fixed for router initialization/persistence by separating configured arms from live dispatch candidates. A TTL cache is no longer required for this part because the env-gated list is used only when selecting dispatch candidates.

2. **Record observations from all dispatch paths**: Add a lightweight `ObservationRecorder` that the dispatch layer calls automatically. Any code path that calls `create_agent_for_model()` should report success/failure back to the cascade router:
   ```rust
   // In roko-agent/src/provider/mod.rs or a new dispatch wrapper
   pub async fn dispatch_and_observe(
       config: &RokoConfig,
       model_key: &str,
       router: Option<&CascadeRouter>,
       /* ... */
   ) -> Result<AgentResult> {
       let result = create_agent_for_model(config, model_key, opts)?.run(prompt).await;
       if let Some(router) = router {
           router.record_observation(/* ... */);
       }
       result
   }
   ```

3. **PRD commands should instantiate a minimal learning context**: At minimum, load the cascade router from disk, record the observation, and save. This is the same pattern `prd.rs` already uses for `EpisodeLogger`.

### 10. Provider Health Circuit Breaker Doesn't Reset on CLI Override Success

**Related to §9B**. The provider health tracker (`provider_health.rs:143-150`) uses a circuit breaker pattern: 3 consecutive failures → `Open` (provider blocked). But if a user successfully uses `--model glm-5-1` (via the `zai` provider), that success is never reported to the health tracker. The circuit breaker stays in `Open` state even though the provider is clearly working.

**Fix**: Same as §9.2 — dispatch-layer observation recording must feed both the cascade router AND the provider health tracker.

**Partial fix (batch 7, 2026-05-03)**: Serve gateway dispatch now records provider health under provider IDs rather than model slugs, so gateway successes/failures can influence the same provider health state used by routing/explain. CLI direct override and PRD single-shot paths are still open.

## Implementation Order

1. Add `tier: Option<ModelTier>` to `ModelProfile` (schema.rs) — foundation
2. Add `ModelTier` enum to roko-core
3. Update roko.toml with tier fields for all models
4. Replace `slug_to_tier()` in cascade/helpers.rs to use config field
5. Remove `AgentBackend::from_model()` / `is_cursor_slug()` — error on unknown
6. Remove `resolve_enrichment_backend()` from orchestrate.rs
7. Remove `config_for_model()` synthesis in model_call_service.rs
8. Add `[routing.role_defaults]` to config schema, move cascade defaults to toml
9. Unify config loading to single path
10. Fix stale model names in presets.rs, agent.rs defaults
11. Fix demo TypeScript hardcoded references

---

## 11. Auth Detection ↔ Dispatch Config Mismatch (2026-05-04)

**Severity**: P0 — User sees "auth: glm-5.1" in banner but dispatch fails with "no API key for provider 'anthropic_api'".

**Root cause**: `detect_auth()` (auth_detect.rs) and `ChatAgentSession` model selection (chat_session.rs) are **completely independent systems**. Auth detection probes env vars directly (ZAI_API_KEY found → "glm-5.1"). But dispatch loads roko.toml and resolves the default model through the cascade router, which may point to a different provider (anthropic_api).

**What happens**:
1. `detect_auth()` finds `ZAI_API_KEY`, returns `AuthMethod::OpenAiCompat { model: "glm-5.1" }`
2. Banner prints "roko — auth: glm-5.1 (OpenAI-compat)"
3. User sends "hello"
4. `ChatAgentSession` loads config, cascade router picks default model
5. Default model resolves to `anthropic_api` provider
6. `resolve_api_key()` checks `ANTHROPIC_API_KEY` → missing → error

**Why they diverge**: Auth detection is a quick env probe for the banner. Dispatch uses the full config system with model profiles, provider registry, and cascade routing. They share no code.

**Fix**: Auth detection should be config-aware. The banner should show the provider that will actually be used for dispatch, not an independent env probe. Ideally:
1. Load config first (unified loader)
2. Resolve the effective default model and provider
3. Check if that provider has credentials
4. If not, try other configured providers
5. Report the provider that will actually be used

This is tracked as S33.1 in the infrastructure audit.

## 12. Twenty Config Loaders (2026-05-04)

**Updated count**: The original audit found 8, then 12. Current count is **20+ distinct config loading functions**. See infrastructure audit §33.3 for the full table. The core issue remains the same: each entry point can produce a different effective config for the same machine. Batches 25-30 addressed ACP-specific paths but did not reduce the total count.

The unified loader in `roko-core/src/config/loader.rs` now has 6 entry points itself (`load_config_unified`, `load_config_with_options`, `load_config_file`, `load_config_validated`, `load_config_validated_with_options`, plus the `LoadOptions` variants). The intent was consolidation but the result is more surface area. The remaining CLI-side `load_roko_config()` functions have not been migrated to use the core loader.

**Remaining migration**: Delete the 10+ CLI/serve one-off `load_roko_config()` functions and replace each callsite with `roko_core::config::loader::load_config_unified(workdir)`. Reduce the core loader to 2 entry points: `load(workdir)` and `load_file(path)`.

## 13. Provider Error Messages Are Not Actionable (2026-05-04)

Provider failures surface raw HTTP status codes and upstream JSON error bodies to the user. Examples:

- `[request failed: 404] {"error":"model not found"}` — doesn't say which model or suggest a fix
- `no API key for provider 'anthropic_api': set ANTHROPIC_API_KEY` — decent, but doesn't mention `roko.toml` `api_key_env` option
- Claude CLI "spawn failed: No such file or directory" — doesn't say "install the claude CLI"

**Fix**: Wrap raw provider errors at the dispatch boundary with human-readable messages:

| HTTP Status | Current Message | Should Say |
|---|---|---|
| 404 | Raw JSON body | "Model '{slug}' not found on {provider}. Check the slug in `roko.toml [models.{key}]`." |
| 401 | Raw JSON body | "{provider} API key is invalid or expired. Check ${env_var} or run `roko config providers health`." |
| 429 | Raw JSON body | "{provider} rate limit exceeded. Wait and retry, or switch provider with `--provider`." |
| spawn ENOENT | "spawn failed: No such file or directory" | "{command} not found on PATH. Install it or configure a different provider in roko.toml." |

This is tracked as infrastructure audit §36.4 and redesign plan Phase 10.8.

## 14. No Provider Pre-Flight Check (2026-05-04)

Provider availability is checked at dispatch time (after context assembly, which can take minutes for large tasks). Should be checked at startup:

1. **CLI provider binaries**: Check `which claude` / `which cursor` at boot
2. **API provider credentials**: Check env var exists and is non-empty
3. **API provider reachability**: Optional HEAD request to base_url (cached 5 min)

`roko doctor` already does some of this but it's not called automatically. The boot sequence (redesign plan Phase 0.3) should include provider pre-flight.

## 15. Provider Config Discovery is Opaque (2026-05-04)

Users have no way to discover what providers are available or how to configure them without reading docs or the source code.

- `roko config providers list` shows what's configured, not what's possible
- No `roko config providers available` command listing supported provider kinds
- No interactive "add provider" flow
- The 54-model `roko.toml` in the repo is overwhelming as a starting point

**Fix**: Add `roko config providers available` that lists all supported `ProviderKind` variants with:
- Required env var
- Example base_url
- Compatible models
- One-liner to add it: `roko config providers add anthropic --api-key $KEY`

## 16. Batch 43 Core Retry Policy Defaults (2026-05-04)

**Done**: `roko-core/src/defaults.rs` now owns named retry-policy defaults for rate-limit, timeout, and generic transient errors. `ErrorKind::retry_policy()` now constructs policies from those constants instead of inline `(5, 2000, 60000)`, `(3, 1000, 30000)`, and `(3, 500, 15000)` tuples.

**Checked**:
- `cargo test -p roko-core retry_policy --jobs 1 -- --nocapture`
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`

**Remaining**: This clears the core error taxonomy retry policy. Active runner/provider workflow and tool-iteration limits still need follow-up constants cleanup under infrastructure audit S6.1. The legacy `orchestrate.rs` path is feature-gated behind `legacy-orchestrate` and is not the refactor target unless it is reactivated.

## 17. Batch 44 Serve Relay Circuit-Breaker Defaults (2026-05-04)

**Done**: `roko-core/src/defaults.rs` now owns relay stale-threshold and circuit-breaker defaults. `roko-serve/src/relay.rs` consumes those defaults for data freshness, heartbeat degradation threshold, exponential backoff base, and backoff cap instead of module-local values.

**Checked**:
- `cargo test -p roko-serve circuit_breaker --jobs 1 -- --nocapture`
- `cargo test -p roko-serve relay_health --jobs 1 -- --nocapture`
- `cargo test -p roko-core relay_backoff_defaults_are_ordered --jobs 1 -- --nocapture`

**Remaining**: Active runner/provider workflow and tool-iteration limits still need cleanup under S6.1. Legacy `orchestrate.rs` retry loops are deprecated/out-of-scope for the runner redesign path.

## 18. Batch 45 Runner Plan Timeout and DAG Backoff Defaults (2026-05-04)

**Done**: `roko-core/src/defaults.rs` now owns the runner plan timeout, DAG retry base delay, DAG retry max delay, and DAG backoff shift cap. The core config schema, CLI config wrapper, runner `RunConfig::default()`, and task DAG controller now consume the shared defaults.

**Checked**:
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`
- `cargo test -p roko-cli runner::task_dag::tests --jobs 1 -- --nocapture`
- `cargo test -p roko-cli parses_minimal_config --jobs 1 -- --nocapture`

**Remaining**: Active runner/provider workflow and tool-iteration limits still need cleanup under S6.1. Do not spend further redesign effort on `crates/roko-cli/src/orchestrate.rs` unless the `legacy-orchestrate` feature is intentionally restored to production use.

## 19. Batch 46 Active Provider Tool-Loop Iteration Defaults (2026-05-04)

**Done**: Active provider adapters no longer pass per-provider numeric defaults into `tool_loop_max_iterations()`. `crates/roko-agent/src/provider/mod.rs` now owns the helper default from `roko_core::defaults::DEFAULT_MAX_TOOL_ITERATIONS`; Anthropic, Gemini, Cerebras, Perplexity, and OpenAI-compatible tool-loop adapters call the shared helper directly. This also removes the remaining OpenAI-compatible `25` iteration default and aligns active providers on the workspace default with temperament adjustment layered on top.

**Checked**:
- `cargo check -p roko-agent --jobs 1`
- `cargo test -p roko-agent tool_loop_iterations_derive_from_workspace_default --jobs 1 -- --nocapture`
- `rg 'tool_loop_max_iterations\\((25|50|[0-9_]+)\\)' crates/roko-agent/src` returns no matches

**Remaining**: Active runner/provider workflow iteration limits outside provider tool-loop adapters still need review; legacy `orchestrate.rs` remains out-of-scope.

## 20. Batch 47 Vision-Loop Defaults (2026-05-04)

**Done**: `roko-core/src/defaults.rs` now owns the vision-loop request/runtime defaults: max iterations, target score, consecutive target count, regression threshold, viewport size, and write-settle wait. `roko-cli/src/vision_loop/mod.rs` and `roko-serve/src/routes/vision_loop.rs` both consume those shared constants instead of carrying duplicate literals.

**Checked**:
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`
- `cargo test -p roko-cli default_config_has_sensible_values --jobs 1 -- --nocapture`
- `cargo check -p roko-serve --jobs 1`
- `rg` check confirms the old inline CLI/serve vision-loop default patterns are gone

**Remaining**: Active workflow iteration defaults outside provider tool loops and vision-loop entry points still need review; legacy `orchestrate.rs` remains out-of-scope.
