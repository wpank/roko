# Roko Redesign Plan

**Date**: 2026-05-02
**Source**: `tmp/infrastructure-audit.md` (160+ issues, 26 sections)
**Approach**: Ground-up redesign in dependency order. Each phase builds the foundation the next phase needs. No bandaids.

---

## Implementation Progress

### Batch 5 — Stable Cascade Model Candidates (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: The learning router mixed two different concepts: the full configured model universe and the subset whose provider credentials are visible in the current process. That caused the persisted `CascadeRouter` arm set to shrink when an env var was missing at startup, which made working models appear as unavailable and prevented future observations.

**Implemented**:
- Added stable config-derived cascade model lists in `RokoConfig`.
- Updated CLI, serve, gateway, provider-explain, and orchestrator routing call sites to initialize/persist routers from the full configured non-embedding slug list.
- Kept dispatch-time candidate filtering credential-aware so missing-key providers are not preferred when usable alternatives exist.
- Corrected current Cerebras model IDs in root `roko.toml`.

**Verified**:
- Focused core unit test for missing credentials and stable cascade candidates.
- Focused serve routing-explain test for provider health eligibility.
- Focused CLI learn-router display test.
- Compile check across `roko-orchestrator`, `roko-serve`, and `roko-cli`.

**Remaining from the same audit area**:
- Record learning observations from direct `--model` and PRD/single-shot dispatch paths.
- Feed successful direct override dispatches back into provider health so circuit breakers can close after recovery.

### Batch 6 — Explicit Provider Config in `ModelCallService` (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `ModelCallService::config_for_model()` was synthesizing provider/model config from runtime env/base-url inputs, which meant dispatch could bypass the canonical `RokoConfig` provider graph.

**Implemented**:
- Removed Anthropic and OpenAI-compatible provider/model synthesis from `ModelCallService`.
- Deprecated `with_openai_base_url()` at this layer.
- Kept explicit env passthrough in `AgentOptions` separate from provider/model configuration.

**Verified**:
- Focused unit test confirms runtime inputs do not mutate provider/model config.
- `cargo check -p roko-agent`.

**Remaining from the same audit area**:
- Provider-factory command synthesis was removed in batch 13; CLI command adapters were made explicit in batch 14.
- Remove or redesign ACP bridge env-based provider synthesis.

### Batch 7 — Gateway Provider Health Keys (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: The gateway updated `ProviderHealthTracker` with model slugs, while routing/explain and provider-health views read state by provider ID. That split meant gateway successes/failures did not update the health entries that routing consults.

**Implemented**:
- Added explicit model key/slug -> provider ID mapping in gateway dispatch.
- Single and batch inference failures record the requested provider.
- Single and batch inference successes record the actual served provider.

**Verified**:
- Durable gateway integration-style test with provider ID intentionally different from model slug.
- `cargo check -p roko-serve`.

**Remaining from the same audit area**:
- Apply the same observation model to CLI/direct override and PRD single-shot dispatch paths.

### Batch 8 — Direct Agent-Exec Learning Persistence (2026-05-03)

**Status**: Complete and committed as the next batch after verification.

**Context**: PRD/research/plan one-shot commands use `agent_exec` instead of the serve gateway. Their capture persistence was writing to `.roko/memory`, recorded model keys instead of API slugs in some paths, and only touched in-memory provider health that vanished when the short-lived process exited.

**Implemented**:
- Canonicalized direct capture persistence to `.roko/learn`.
- Resolved configured model keys to API slugs before building learning episodes.
- Opened `LearningRuntime` for direct captures with the configured cascade model slug set.
- Wrote persistent `.roko/learn/provider-health.json` updates keyed by configured provider ID.
- Passed the resolved slug from logged direct-agent dispatch into capture persistence.

**Verified**:
- Added unit coverage for canonical `.roko/learn` episode persistence.
- Added config-backed coverage proving model key `glm-mini` records slug `glm-5.1`, updates cascade confidence stats, and records provider health under `zai`.
- `cargo check -p roko-cli`.

**Remaining from the same audit area**:
- Review non-`agent_exec` direct call surfaces, especially `dispatch_via_model_call_service()` and chat/inline.
- Decide whether persisted provider health should be a shared service API instead of direct writes from CLI helpers.

### Batch 9 — Shared Direct Learning Helpers for ModelCallService Dispatch (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Batch 8 fixed PRD/research/plan direct agent execution, but `dispatch_via_model_call_service()` still created a feedback sink without a cascade router and did not update persisted provider health.

**Implemented**:
- Moved one-shot learning persistence helpers into `learning_helpers`.
- Reused those helpers from `agent_exec` and `dispatch_v2`.
- Attached a persisted `CascadeRouter` to the `dispatch_via_model_call_service()` feedback sink.
- Saved cascade observations after the direct model call completes.
- Persisted provider-health success/failure outcomes for this direct ModelCallService path.

**Verified**:
- `cargo check -p roko-cli`.
- Existing direct capture unit tests still pass through the shared helper path.
- `git diff --check`.

**Remaining from the same audit area**:
- Chat/inline API-mode turns are covered in batch 12. Review any remaining direct provider construction paths.
- Decide whether `.roko/learn/provider-health.json` writes should move behind a reusable recorder trait/service.

### Batch 10 — Explicit Anthropic Provider Config in ACP (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: The ACP bridge still had its own Anthropic provider synthesis path. It also called `RokoConfig::effective_providers()`, which can synthesize Anthropic providers from legacy env compatibility, so removing only the local branch would not have closed the issue.

**Implemented**:
- Removed ACP-local Anthropic provider creation from `ANTHROPIC_API_KEY`.
- Made ACP Anthropic dispatch use explicit `roko_config.providers`.
- Kept legacy model-profile filling only when an explicit Anthropic API provider exists.
- Updated comments so ACP no longer claims env synthesis behavior.

**Verified**:
- Regression test confirms ACP ignores an Anthropic provider synthesized by `effective_providers()`.
- Existing explicit-provider ACP test still passes.
- `cargo check -p roko-acp`.

**Remaining from the same audit area**:
- Remove or migrate core `RokoConfig::effective_providers()` env synthesis.
- Provider-factory command synthesis was removed in batch 13; CLI command synthesis helpers remain.

### Batch 11 — Remove Core Anthropic Provider Env Synthesis (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: After ACP stopped using synthesized effective providers, core still inserted Anthropic API providers from process env and legacy `agent.env` values. That meant callers using `effective_providers()` could still bypass explicit provider config.

**Implemented**:
- Removed non-empty-config insertion of `providers.anthropic` from process `ANTHROPIC_API_KEY`.
- Removed empty-config insertion of an Anthropic API provider from `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, and legacy agent env values.
- Kept the empty-config `claude_cli` compatibility provider for this batch; batch 20 removes that broader default.
- Updated ACP test expectations for the new core behavior.

**Verified**:
- Core unit test for no Anthropic synthesis from legacy agent env.
- ACP Anthropic config tests.
- Compile check across core, ACP, agent, CLI, and serve.

**Remaining from the same audit area**:
- `effective_models()` still synthesizes model profiles from legacy default/tier slugs.
- CLI command adapters now build explicit transient provider/model config as of batch 14.

### Batch 12 — Chat/inline ModelCallService Feedback (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `ChatAgentSession::send_turn_api()` built a direct `ModelCallService` without a feedback sink. Inline chat session mode uses the same session object, so non-CLI inline turns inherited that blind spot: no efficiency feedback, no cascade observation save, and no persisted provider-health update.

**Implemented**:
- Added a `ChatFeedbackRuntime` that builds a `.roko/learn` `FeedbackService`, attaches a persisted `CascadeRouter`, and saves the router after chat turns.
- Wired `send_turn_api()` to use that feedback sink for direct ModelCallService calls.
- Persisted provider-health success/failure outcomes for chat API-mode turns by configured provider ID.
- Kept provider-health/feedback persistence best-effort so a successful chat turn is not converted into a user-visible failure if learning files cannot be written.
- Allowed non-API session providers to skip the API-key preflight; API providers still preserve the missing-key early return before history mutation.

**Verified**:
- Added a mock-backed chat session test with a local fake Claude CLI provider. The test verifies a chat turn writes `.roko/learn/efficiency.jsonl`, `.roko/learn/provider-health.json`, and `.roko/learn/cascade-router.json`.
- `cargo test -p roko-cli --lib send_turn_api_records_chat_feedback_and_provider_health`.
- `cargo check -p roko-cli`.
- `git diff --check`.

**Remaining from the same audit area**:
- Review direct `create_agent_for_model()` callers outside the three covered paths: agent-exec captures, `dispatch_via_model_call_service()`, and chat session API-mode dispatch.
- Decide whether the repeated persisted provider-health write should become a small recorder service instead of a helper called from each dispatch surface.

### Batch 13 — Explicit Provider Factory Construction (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `create_agent_for_model()` still contained a compatibility branch that looked at `agent.command` / `AgentOptions.command` and invented provider/model definitions for known protocol CLIs. That kept a second implicit provider system alive below the config layer.

**Implemented**:
- Removed command-derived `ProviderConfig` and `ModelProfile` creation from the provider factory.
- Known protocol commands with unknown model keys now return a `MissingConfig` error asking for explicit `[providers]` and `[models]` entries.
- Explicit/default config-backed Claude CLI construction still works through `RokoConfig::effective_providers()` and `effective_models()`.
- Generic raw subprocess fallback remains for non-protocol commands.

**Verified**:
- Added a regression test for known protocol command rejection without model config.
- Updated the configured Claude provider test to prove the supported path is explicit/effective config, not command synthesis.
- `cargo test -p roko-agent create_agent_for_model_rejects_protocol_command_without_model_config`.
- `cargo test -p roko-agent create_agent_for_model_uses_effective_claude_provider_for_configured_model`.
- `cargo check -p roko-agent`.

**Remaining from the same audit area**:
- CLI command-backed compatibility helpers were migrated in batch 14 to explicit transient provider/model entries.
- `effective_models()` still synthesizes model profiles from default/tier model slugs.

### Batch 14 — Explicit CLI Command Adapter Config (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: After provider-factory synthesis was removed, the remaining CLI command helpers still built sparse `RokoConfig` values and relied on core effective-model fallback to fill missing provider/model profiles. That kept implicit behavior in run/orchestrate paths.

**Implemented**:
- Added a centralized command-backed config adapter in `agent_config`.
- Claude and Cursor command paths now create explicit transient providers with command fields and explicit model profiles.
- Codex/OpenAI-compatible command paths now create an explicit `openai_compat` provider requiring `OPENAI_API_KEY`.
- Generic subprocess fallback remains separate and still reaches the raw `ExecAgent` fallback when the command is not a known protocol command.

**Verified**:
- Updated agent-config tests to assert the transient providers/models exist.
- `cargo test -p roko-cli --lib synthesize_`.
- `cargo check -p roko-cli`.
- `git diff --check`.

**Remaining from the same audit area**:
- `RokoConfig::effective_models()` still fills model profiles from default/tier model slugs.
- Function names still say `synthesize_*`; they are compatibility shims now, not provider-factory inference.

### Batch 15 — Direct Provider Chat Feedback (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: The direct provider chat REPL used `create_agent_for_model()` directly and accumulated its own transcript. It bypassed `ModelCallService`, so chat turns were invisible to `.roko/learn` feedback, cascade observations, and provider-health recovery.

**Implemented**:
- Added per-turn `FeedbackEvent::ModelCall` recording in `run_direct_provider_chat()`.
- Attached and saved the persisted cascade router for direct provider chat turns.
- Wrote provider-health success/failure observations by configured provider ID.
- Preserved the existing direct REPL behavior and kept learning persistence best-effort.

**Verified**:
- `cargo check -p roko-cli`.
- `cargo fmt`.
- `git diff --check`.

**Remaining from the same audit area**:
- Review remaining direct provider construction paths: dispatch-v2 factory bridges and serve template dispatch.
- Add a non-interactive integration harness for direct provider chat if this REPL becomes a supported automation surface.

### Batch 16 — Vision Evaluator Feedback (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `roko vision-loop` creates provider agents directly for multimodal screenshot evaluation. That bypassed `ModelCallService`, so vision feedback, cascade observations, and provider-health recovery were not persisted.

**Implemented**:
- Passed the project root into `VisionEvaluator` from the loop orchestrator.
- Recorded each evaluator provider call as `.roko/learn` `FeedbackEvent::ModelCall` feedback.
- Saved `.roko/learn/cascade-router.json` after evaluator observations.
- Persisted `.roko/learn/provider-health.json` using the configured provider ID.
- Counted invalid/unparseable evaluator JSON as a learning failure without marking the provider transport unhealthy when the provider call itself succeeded.

**Verified**:
- Added a mock Claude CLI provider test proving evaluator output is parsed and the efficiency, provider-health, and cascade-router files are written.
- `cargo test -p roko-cli --lib evaluate_records_feedback_and_provider_health`.
- `cargo check -p roko-cli`.
- `cargo fmt`.
- `git diff --check`.

**Remaining from the same audit area**:
- Review serve template dispatch, the remaining direct provider construction path.
- Dispatch-v2 factory bridges are covered by batch 17.
- Core `RokoConfig::effective_models()` still fills model profiles from legacy default/tier model slugs.

### Batch 17 — Dispatch-V2 Bridge Feedback (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `AgentDispatcherV2` can construct provider-backed agents directly through `create_agent_for_model()` in non-streaming, streaming, and MCP-preloaded bridge paths. Those paths produce runner events but previously did not persist `.roko/learn` feedback or provider health.

**Implemented**:
- Added a shared dispatch-v2 feedback recorder.
- Wired `run_agent_result_bridge()`, `run_agent_streaming()`, and `run_agent_result_bridge_with_mcp()` to record after provider execution.
- Saved cascade-router observations under `.roko/learn/cascade-router.json`.
- Persisted provider health under resolved provider IDs.

**Verified**:
- Added a mock Claude CLI provider test for the bridge path that verifies efficiency, provider-health, and cascade-router files.
- `cargo test -p roko-cli --lib run_agent_result_bridge_records_feedback_and_provider_health`.
- `cargo check -p roko-cli`.
- `cargo fmt`.
- `git diff --check`.

**Remaining from the same audit area**:
- Serve template dispatch is covered by batch 18; the direct provider construction feedback blind spot is closed for the reviewed callers.
- Core `RokoConfig::effective_models()` still fills model profiles from legacy default/tier model slugs.

### Batch 18 — Serve Template Dispatch Feedback (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Serve template dispatch is the event/subscription path that creates provider-backed agents directly from templates. It already recorded episodes and anomaly signals, but not the canonical `.roko/learn` model-call feedback or durable provider-health state.

**Implemented**:
- Added template-dispatch feedback recording after agent execution and template gates.
- Persisted efficiency feedback, provider health, and cascade observations by configured provider/model identity.
- Updated in-memory `AppState::provider_health` for the provider ID.
- Used the cached global `AppState` cascade router when writing global observations so server shutdown does not overwrite fresh feedback; repo-specific dispatches continue to use their repo layout.

**Verified**:
- Added a fake Claude CLI template dispatch test that runs through `dispatch_template()` and verifies efficiency feedback, provider health, cascade-router persistence, and in-memory provider health.
- `cargo test -p roko-serve --lib template_dispatch_records_feedback_and_provider_health`.
- `cargo check -p roko-serve`.
- `cargo fmt`.
- `git diff --check`.

**Remaining from the same audit area**:
- Core `RokoConfig::effective_models()` no longer fills model profiles from legacy default/tier model slugs as of batch 19.
- The direct feedback persistence logic is repeated across several surfaces; the next cleanup should extract a shared recorder service.

### Batch 19 — Explicit Effective Model Registry (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Even after provider-factory and command-adapter synthesis were removed, core `RokoConfig::effective_models()` still materialized `ModelProfile` entries from `agent.default_model` and `agent.tier_models`. That meant a model reference could still become a dispatchable model definition without a `[models.*]` entry.

**Implemented**:
- Changed `effective_models()` to return only explicit configured models.
- Tightened core reference validation for `agent.default_model`, `agent.fallback_model`, and `agent.tier_models`.
- Updated CLI semantic validation to report those same missing model references.
- Retained env override materialization as an explicit boundary mutation into `config.models`, not runtime effective-model synthesis.

**Verified**:
- Core regression test proves default/tier profiles are not synthesized.
- Core root-config compatibility test still passes because root `roko.toml` declares explicit model profiles.
- Provider-factory missing-config regression still passes.
- Compile check across core, agent, CLI, and serve.

**Known verification caveat**:
- The focused CLI semantic-validation test could not complete because the full `roko-cli` lib test binary hit the known macOS linker failure with unresolved anonymous LLVM symbols. The affected code compiles via `cargo check -p roko-cli`.

**Remaining from the same audit area**:
- Direct-call feedback persistence now has a shared recorder service as of batch 21.

### Batch 21 — Shared Model-Call Feedback Recorder (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: After the direct call surfaces were wired into `.roko/learn`, the same persistence sequence was repeated in CLI chat, dispatch-v2, vision evaluation, and serve template dispatch. That was becoming a maintenance hazard because feedback, provider health, and cascade observations must remain atomic from the caller's point of view.

**Implemented**:
- Added `ModelCallFeedbackRecorder` in `roko-learn`.
- Centralized durable model-call feedback, provider-health persistence, and cascade-router saves.
- Centralized the cascade-router model-call observation helper used by both `FeedbackService` and direct recorder call sites.
- Replaced duplicated persistence blocks in direct provider chat, vision evaluator, dispatch-v2 bridge dispatch, and serve template dispatch.
- Kept serve's cached global cascade router path as a first-class case so template dispatch updates the in-memory router that shutdown later persists.
- Made the CLI's remaining provider-health helper delegate to the shared `roko-learn` provider-health writer.

**Verified**:
- New `roko-learn` recorder unit test.
- Existing CLI dispatch-v2 feedback regression.
- Existing vision evaluator feedback regression.
- Existing serve template-dispatch feedback regression.
- Compile check across learn, CLI, and serve using isolated `CARGO_TARGET_DIR=target/codex-batch21`.

**Remaining from the same audit area**:
- The direct-call feedback duplication item is closed. Next model/provider work should move to unified config loading, backend heuristic removal, and UI/demo provider hardcoding.

### Batch 22 — Explicit CLI Model Selection (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `resolve_effective_model()` still allowed an unknown model slug to select any configured provider with the provider kind inferred from the slug. That kept a hidden provider-routing path alive after core stopped synthesizing providers and model profiles.

**Implemented**:
- Removed provider-kind fallback selection from CLI model selection.
- Unknown selected models now error with guidance to add an explicit `[models.*]` profile.
- Preserved inferred provider kind in the error text only as diagnostic context.
- Reworked precedence tests to use explicit provider/model fixtures instead of default synthetic model behavior.
- Added regression coverage that a configured `openai_compat` provider is not enough to route an unconfigured GPT-like slug.

**Verified**:
- Full `model_selection::tests` suite.
- `cargo check -p roko-cli` in the isolated batch target.

**Remaining from the same audit area**:
- Core `resolve_model()` still has legacy fallback fields for compatibility. Remaining work should migrate call sites to strict explicit-profile checks, then retire `AgentBackend::from_model()` from production paths.

### Batch 23 — Explicit Serve Provider Identity (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Serve dashboard and gateway health code still inferred provider IDs for unknown model strings. That could make unconfigured models appear to have provider health and could record gateway failures/successes under provider kinds that were never selected from `[models.*]`.

**Implemented**:
- Removed dashboard provider inference from raw model slugs.
- Changed gateway provider-health lookup to return a provider only for explicit configured model profiles.
- Skipped gateway provider-health mutation when the requested/served model is not configured.
- Added regression coverage for unconfigured GPT-like slugs with a configured `openai_compat` provider.

**Verified**:
- Focused gateway provider-id regression.
- Existing durable gateway event/provider-health regression.
- `cargo check -p roko-serve`.

**Remaining from the same audit area**:
- Continue migrating remaining call sites away from core slug heuristics, then retire the heuristic API from production.

### Batch 24 — Provider Factory Missing-Provider Guard (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: A model profile with `provider = "missing-provider"` could reach the provider factory, fail to find a provider config, and then fall through to the raw `ExecAgent` fallback. That turned a config graph error into an unrelated subprocess behavior.

**Implemented**:
- `create_agent_for_model()` now treats model-profile/provider mismatch as `MissingConfig`.
- The raw `ExecAgent` fallback remains only for unconfigured non-protocol command execution.
- Added provider-factory regression coverage for a model profile that references a missing provider.

**Verified**:
- Missing-provider regression.
- Existing known-protocol missing-config regression.
- `cargo check -p roko-agent`.

**Remaining from the same audit area**:
- Consider an explicit `subprocess` provider for raw command fallback, then remove the last non-configured factory path.

### Batch 25 — ACP Exact Config File Loading (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: ACP normal workdir loading already used the core unified loader, but `--config` still meant "load from this file's parent directory". For editor integrations using a nonstandard config filename, ACP could ignore the explicit path and rediscover a sibling or ancestor `roko.toml`.

**Implemented**:
- Added `roko_core::config::loader::load_config_file(path, opts)` for exact config file paths.
- Exact file loading bypasses `ROKO_CONFIG` path discovery and ancestor discovery while keeping global merge, `ROKO__*` overrides, interpolation, file secrets, and strict validation when requested.
- Wired `AcpConfig::load_roko_config()` so explicit `config_path` uses the exact-file loader.
- Added core and ACP tests proving explicit nonstandard config files win over parent `roko.toml`.

**Verified**:
- Core config-loader exact-file integration test.
- ACP config exact-file unit test.
- `cargo check -p roko-core -p roko-acp`.
- `git diff --check`.

**Remaining from the same audit area**:
- Remove ACP's static Anthropic/Sonnet fallback for empty configs.
- Collapse duplicated CLI path/global helpers into core-loader wrappers while preserving CLI provenance and repo-registry behavior.

### Batch 26 — ACP Static Provider Fallback Removal (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: `build_config_options_static()` hardcoded one provider (`anthropic`) and one model (`sonnet`) when ACP received an empty config. After global merge moved into the core loader, keeping this fallback hid configuration problems and kept an invented provider/model surface alive.

**Implemented**:
- Removed `build_config_options_static()`.
- Empty ACP session config state now uses empty provider/model selections.
- `from_roko_config()` no longer fabricates a fallback model when no models are configured.
- `AcpSession::new()` now uses the same config option builder as `new_with_config()`, so empty configs expose empty provider/model option lists instead of hardcoded defaults.
- Added tests for both empty resolved config and legacy no-config session construction.

**Verified**:
- Focused static-fallback regression tests.
- Full `session::tests` suite.
- `cargo check -p roko-acp`.
- `git diff --check`.

**Remaining from the same audit area**:
- Revalidate persisted sessions against the current config on load.
- Show configured-but-unavailable providers with status text instead of filtering them out silently.

### Batch 27 — ACP Persisted Session Revalidation (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Persisted ACP sessions serialized provider/model selections and config options. When config changed between editor sessions, `session/resume` could restore removed providers/models and show stale options.

**Implemented**:
- Added `AcpSession::revalidate_config_state()`.
- `SessionManager::load_from_disk()` now revalidates deserialized sessions against the current `RokoConfig`.
- Missing providers reset provider/model to current config defaults.
- Missing models under a still-valid provider reset to that provider's first configured model, or empty when none exists.
- Rebuilt `config_options` from the current config after revalidation.
- Added tests for stale provider+model and stale model with valid provider.

**Verified**:
- Focused stale persisted-session tests.
- Full ACP `session::tests` suite.
- `cargo check -p roko-acp`.
- `git diff --check`.

**Remaining from the same audit area**:
- Live config watch/reload for project and global config while ACP is running.
- Provider-health/status descriptions for unavailable configured providers.

### Batch 28 — ACP Provider Option Status (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: ACP provider options filtered out providers whose credentials were not visible in the current process. That made configured providers disappear from the status bar instead of telling the user which API key or config was missing.

**Implemented**:
- Provider options now include all configured providers.
- Provider option descriptions show `Ready`, missing API key env, missing API key configuration, or generic unavailable status.
- Model options are listed for the selected provider even when that provider is missing credentials.
- Added a regression for a configured OpenAI-compatible provider with an unset API key env.

**Verified**:
- Focused unavailable-provider option test.
- Full ACP `session::tests` suite.
- `cargo check -p roko-acp`.
- `git diff --check`.

**Remaining from the same audit area**:
- Live endpoint health checks and cached reachability status.
- Model slug pre-validation and clearer dispatch error messages.

### Batch 29 — CLI Config Helper Delegation (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Core already had the canonical global config path, project config discovery, and global merge helpers, but `roko-cli::config` still carried copied implementations for `load_layered()` and config commands.

**Implemented**:
- `roko-cli::config::global_config_path()` delegates to `roko_core::config::loader::global_config_path()`.
- `roko-cli::config::discover_project_config()` delegates to the core loader.
- `roko-cli::config::merge_global_providers()` delegates to `roko_core::config::loader::merge_global_into()`.
- Left `load_layered()` in place because it still owns CLI-specific provenance/source and repo-registry assembly.

**Verified**:
- Focused CLI global-path test.
- Focused CLI project-discovery test.
- `cargo check -p roko-cli`.
- `git diff --check`.

**Remaining from the same audit area**:
- Collapse `load_layered()` effective loading onto the core loader while preserving source diagnostics.
- Remove any now-dead manual global merge call sites after that consolidation.

### Batch 30 — ACP Config Update Validation (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Persisted sessions are now revalidated on resume, but the live ACP config update path still accepted arbitrary provider/model strings from the client.

**Implemented**:
- Unknown provider selections are ignored and the existing provider/model state is preserved.
- Unknown model selections are ignored.
- Cross-provider model selections are ignored unless the selected model belongs to the current provider.
- Provider changes still reset the current model to the first configured model for the new provider when needed.
- Added tests for unknown provider, unknown model, and cross-provider model updates.

**Verified**:
- Focused ACP config-update validation tests.
- Full ACP `session::tests` suite.
- `cargo check -p roko-acp`.
- `git diff --check`.

**Remaining from the same audit area**:
- Validate configured provider slugs against provider-known model lists where possible.
- Return user-visible config-update warnings instead of log-only warnings if the ACP protocol/client supports it.

### Batch 31 — Demo Workspace-Explicit Roko Commands (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: The PRD pipeline demo could create a draft in its temp workspace, then run later `roko prd ...` steps from the repo root if terminal CWD drifted. The scenario helper built commands from ambient CWD instead of passing the server-created workspace identity.

**Implemented**:
- `resolve_workdir()` now canonicalizes existing workdirs, including `--repo`, before returning.
- Added resolver coverage for canonicalizing an existing `--repo` path.
- The demo `roko(ctx, subcommand)` helper now injects `--repo '<workspaceDir>'` into every command it builds.
- Added shell quoting for injected workspace/model values.

**Verified**:
- `cargo test -p roko-cli --bin roko resolve_workdir_`.
- `cargo check -p roko-cli`.
- `npm run build` in `demo/demo-app`.
- `git diff --check`.

**Remaining from the same audit area**:
- Add an end-to-end PRD pipeline replay that verifies files are created/promoted/planned in the same workspace.

### Batch 32 — Demo PRD Pipeline CWD Guard (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: Batch 31 made generated `roko` commands workspace-explicit, but the PRD pipeline still trusted the terminal shell's CWD after long-running agent commands. Hidden setup also still depended on ambient CWD.

**Implemented**:
- Added `ensureWorkspaceCwd(handle, dir)` in the terminal orchestration layer. It re-enters the expected workspace through the marker-based `execCmd` path and returns a typed success/failure result.
- `enterWorkspace()` now delegates to the same CWD guard instead of carrying its own quoted `cd` command.
- `showCmd()` accepts `workspaceDir` and runs the CWD guard before typing visible commands, then clears the output buffer so command result detection only sees the visible command's output.
- The PRD pipeline runner now guards hidden scaffold/init setup and every generated visible command (`prd idea`, `prd draft new`, `draft promote`, `prd plan`, `plan validate`, `plan run`, `learn all`).
- CWD failures are surfaced as pipeline failures with explicit workspace context instead of letting later PRD steps fail as "not found".

**Verified**:
- `npm run build` in `demo/demo-app`.

**Remaining from the same audit area**:
- Add a browser-level demo replay once the dev server orchestration is reliable enough for automated terminal playback.

### Batch 33 — PRD Pipeline Workspace E2E (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: The PRD pipeline needed a real CLI-level regression for the exact pattern emitted by the demo: run commands from a long-lived terminal while passing `--repo <workspace>`. While adding that test, `plan validate` was found to still validate relative plan paths and file references against process CWD instead of the explicit repo.

**Implemented**:
- `roko plan validate <dir>` now resolves relative plan directories against `resolve_workdir(cli)`, so global `--repo` is honored.
- `cmd_plan_validate()` now receives the resolved workdir directly and uses it for model config lookup and file-reference validation.
- Added `crates/roko-cli/tests/prd_pipeline_workspace.rs`, which initializes a selected workspace and a decoy CWD, then runs:
  - `roko --repo <selected> prd idea ...`
  - `roko --repo <selected> prd draft new ...`
  - `roko --repo <selected> prd draft promote ...`
  - `roko --repo <selected> prd plan ...`
  - `roko --repo <selected> plan validate .roko/plans`
- Added `mock-prd-pipeline-fixture`, including explicit distillation turns so background episode distillation does not consume the planning response.
- The regression asserts ideas, draft, published PRD, `tasks.toml`, and `plan.md` are created in the selected workspace and not in the decoy CWD.

**Verified**:
- `CARGO_TARGET_DIR=target/codex-batch33 cargo test -p roko-cli --test prd_pipeline_workspace -- explicit_repo_prd_pipeline_artifacts_stay_in_selected_workspace --nocapture`.

**Remaining from the same audit area**:
- Add a browser-level demo replay for terminal playback and workflow stream rendering.

### Batch 34 — Artifact-Driven PRD Draft Success (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S22.3 tracked `prd draft new` failing the demo even when the draft existed. The same area also had a brittle mtime-based direct-write detector that could miss agent writes on coarse-mtime filesystems.

**Implemented**:
- `prd draft new` now snapshots draft file bytes before/after the agent run instead of comparing mtimes.
- Direct agent file writes are considered successful only when the resulting draft has substantive markdown content.
- Agent text output is materialized into the draft even if the agent process exits non-zero, as long as the output is substantive.
- The command returns `0` when a substantive draft artifact exists, returns `1` when the agent claimed success but produced no substantive draft, and otherwise preserves the raw non-zero agent exit code.
- Sidecars, validation, workspace member collection, and learning episode success are now keyed to artifact success rather than raw subprocess success.
- Added `mock-prd-draft-write-then-fail-fixture`, where the agent writes a draft and returns a failing result.
- Extended `prd_pipeline_workspace.rs` with `prd_draft_new_succeeds_when_agent_writes_draft_then_exits_nonzero`.

**Verified**:
- `RUSTC_WRAPPER= CARGO_TARGET_DIR=/tmp/roko-codex-batch34-target cargo test -p roko-cli --test prd_pipeline_workspace --jobs 1 -- --nocapture`.

**Notes**:
- In-repo Cargo targets were unstable during verification because `cargo clean` was being run concurrently in another terminal. The passing verification used an external `/tmp` target, disabled sccache, and serial jobs to avoid target-file races.

**Remaining from the same audit area**:
- Add progress feedback during PRD tool-loop iterations (S22.5).

### Batch 35 — Safety Contract Trust Boundaries (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S12.5 tracked a critical trust-boundary bug in the declarative safety contracts. Commit-like tool calls could previously satisfy `RequireGateBeforeCommit` by adding `gate_passed: true` to LLM-controlled arguments, and token/cost checks had similar risks around LLM-supplied estimates.

**Implemented**:
- `RequireGateBeforeCommit` now accepts gate approval only from `ToolContext.external_actions`, not from pending tool-call arguments.
- `MaxTokensPerTurn` continues to derive estimates from actual payload strings and has explicit regression coverage that `estimated_tokens` / `max_tokens` claims are ignored.
- `MaxCostPerTurn` now enforces recorded external-action spend and ignores `estimated_cost_usd` on the pending call.
- Added cost metadata extraction for `actual_cost_usd`, `cost_usd`, `total_cost_usd`, `usage.cost_usd`, and `usage.total_cost_usd`.
- Added focused unit regressions for gate bypass rejection, recorded gate approval, token claim bypass rejection, ignored call-argument cost estimates, and recorded spend enforcement.

**Verified**:
- `CARGO_TARGET_DIR=/tmp/roko-codex-batch35-target cargo test -p roko-agent safety::contract --jobs 1 -- --nocapture`.
- Result: 16 `safety::contract` tests passed, including the new bypass regressions.
- Removed `/tmp/roko-codex-batch35-target` after verification to avoid leaving regenerated build artifacts.

**Remaining from the same audit area**:
- Ensure normal provider/tool-loop dispatch paths consistently record provider spend into `ToolContext.external_actions`; the contract now enforces any recorded cost and refuses LLM-supplied estimates, but it can only enforce spend that the orchestrator records.

### Batch 36 — Configured Model Output Ceilings (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S23.2 tracked missing `max_output` values in `roko.toml`, especially Kimi aliases where generic OpenAI-compatible defaults could truncate large tool-call JSON. The audit also called out `docker/railway.roko.toml`, which had no per-model output ceilings at all.

**Implemented**:
- Added explicit `max_output` to all root `roko.toml` model profiles that were still missing one.
- Added explicit `max_output` to every model in `docker/railway.roko.toml`.
- Used existing configured values for duplicate model aliases and provider-family values already present in the repo; did not change already-populated ceilings in this batch.
- Added a roko-core regression that parses both the root and Railway config files and fails when any configured non-embedding model lacks `max_output`.

**Verified**:
- `CARGO_TARGET_DIR=/tmp/roko-codex-batch36-target cargo test -p roko-core project_model_profiles_have_explicit_max_output --jobs 1 -- --nocapture`.
- Result: focused config regression passed.
- `git diff --check`.
- Removed `/tmp/roko-codex-batch36-target` after verification.

**Remaining from the same audit area**:
- Adaptive retry on `finish_reason == "length"` is still open.
- `max_tool_iterations` / unified provider iteration limits are still open.
- A separate model-spec modernization pass can revisit already-populated ceilings where provider docs have changed.

### Batch 37 — Serve Readiness and `roko up` Shutdown (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S5 tracked Railway/deployment reliability issues around health probes and shutdown. `roko serve` already had an unauthenticated top-level `/health` and richer `/api/health`, but there was no top-level `/ready`; `roko up` also bypassed the normal serve shutdown path by spawning `run_server()` and aborting the wrapper task on Ctrl+C.

**Implemented**:
- Top-level `GET /health` now returns `status`, `version`, and `uptime_secs` while staying unauthenticated and outside `/api`.
- Added top-level unauthenticated `GET /ready`; it returns `200`/`ok` while the server is ready and `503`/`shutting_down` after `AppState.cancel` is cancelled.
- Added router regressions proving `/health` and `/ready` are available even when API auth is enabled.
- Added a readiness regression proving `/ready` flips to `503` after cancellation.
- Changed `roko up` to start serve via `roko_serve::start_server_background()` and wait for the returned server task after cancellation instead of aborting it.

**Verified**:
- `cargo test -p roko-serve top_level --jobs 1 -- --nocapture`.
- `cargo test -p roko-serve health_reports_status_version_uptime_and_counts --jobs 1 -- --nocapture`.
- `cargo check -p roko-cli --jobs 1`.
- `git diff --check`.

**Remaining from the same audit area**:
- S5 Docker/Railway work remains: sidecar supervision/separation, proper multi-stage runtime image, and production CORS/deployment defaults.

### Batch 38 — Embedded Safety Contract Assets (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S12.4 tracked safety contracts being loaded from `env!("CARGO_MANIFEST_DIR")`, which points at the build machine source tree and can disappear in deployed binaries. That made missing deployed assets fall into the restricted fallback path for known roles.

**Implemented**:
- Replaced runtime filesystem reads in `AgentContract::load_for_role()` with a compile-time bundled role registry using `include_str!`.
- Embedded all bundled role contracts: architect, auditor, auto-fixer, implementer, researcher, reviewer, scribe, and strategist.
- Kept missing-role behavior fail-closed, but missing asset errors now carry a relative bundle label instead of an absolute build-machine path.
- Updated loader documentation to describe embedded assets instead of crate-root filesystem reads.
- Added regressions for bundled role coverage and missing-role relative path reporting.

**Verified**:
- `cargo test -p roko-agent safety::contract --jobs 1 -- --nocapture`.
- Result: 18 `safety::contract` tests passed, including the new embedded-asset regressions.
- `git diff --check`.

**Remaining from the same audit area**:
- S12.3 remains open: dispatcher construction can still omit a `SafetyLayer`; this batch only fixes contract asset loading once a contract is requested.

### Batch 39 — Roko-Agent Request Timeout Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S6.1 tracks scattered numeric defaults. `roko-core::defaults::DEFAULT_REQUEST_TIMEOUT_MS` already exists, but `crates/roko-agent/src` still had runtime `120_000` request-timeout literals in concrete agents, provider adapters, and shared tool-loop backend construction.

**Implemented**:
- Routed default request timeouts through `DEFAULT_REQUEST_TIMEOUT_MS` in `ClaudeAgent`, `CodexAgent`, `CursorAgent`, `ExecAgent`, and `ClaudeCliAgent`.
- Updated provider adapters for OpenAI-compatible, Anthropic API, Claude CLI, Cursor ACP, Cerebras, Gemini, and Perplexity paths.
- Updated shared tool-loop backend factories, including Gemini-native backend defaults.
- Updated dispatch resolver test fixtures to use `DEFAULT_REQUEST_TIMEOUT_MS` and `DEFAULT_CONNECT_TIMEOUT_MS`.
- Confirmed no `unwrap_or(120_000)`, `timeout_ms: 120_000`, `const DEFAULT_TIMEOUT_MS: u64 = 120_000`, or `Some(120_000)` remain under `crates/roko-agent/src`.

**Verified**:
- `cargo check -p roko-agent --jobs 1`.
- `cargo test -p roko-agent timeout_ms_is_forwarded_to_poster --jobs 1 -- --nocapture`.
- `git diff --check`.

**Remaining from the same audit area**:
- Continue constants cleanup in CLI/serve request timeout sites, retry backoff policies, and workflow/tool iteration defaults.

### Batch 40 — Roko-CLI Request Timeout Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S6.1 request-timeout cleanup continued after batch 39. `crates/roko-cli/src` still had runtime request-timeout fallback literals even though `DEFAULT_REQUEST_TIMEOUT_MS` and `DEFAULT_CONNECT_TIMEOUT_MS` already exist in core defaults.

**Implemented**:
- Routed CLI agent config defaults and provider-config resolution through `DEFAULT_REQUEST_TIMEOUT_MS`.
- Routed provider connect-timeout resolution through `DEFAULT_CONNECT_TIMEOUT_MS`.
- Updated chat direct dispatch, chat-session API timeout fallback, config display defaults, orchestrator Gemini-cache/judge paths, runner event-loop dream config, and vision evaluator dispatch.
- Confirmed no `unwrap_or(120_000)`, `timeout_ms: 120_000`, `Some(120_000)`, or `or(Some(120_000))` remain under `crates/roko-cli/src`.
- Left TUI timeout preset values alone because they are visible user choices, not hidden runtime defaults.

**Verified**:
- `cargo check -p roko-cli --jobs 1`.
- `cargo test -p roko-cli parses_minimal_config --jobs 1 -- --nocapture`.
- `git diff --check`.

**Remaining from the same audit area**:
- Continue constants cleanup in serve/ACP request timeout sites, retry policies, and workflow/tool iteration defaults.

### Batch 41 — Roko-Serve Request Timeout Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S6.1 request-timeout cleanup continued after batches 39-40. `crates/roko-serve/src` still had one runtime `120_000` timeout default in the dream endpoint plus provider-route fixture literals.

**Implemented**:
- Routed the dream-run endpoint's `DreamAgentConfig.timeout_ms` through `DEFAULT_REQUEST_TIMEOUT_MS`.
- Updated provider route test fixtures to use `DEFAULT_REQUEST_TIMEOUT_MS` and `DEFAULT_CONNECT_TIMEOUT_MS`.
- Confirmed no request-timeout `120_000` patterns remain under `crates/roko-serve/src`.

**Verified**:
- `cargo check -p roko-serve --jobs 1`.
- `cargo test -p roko-serve list_providers_returns_configured_providers_with_health --jobs 1 -- --nocapture`.
- `git diff --check`.

**Remaining from the same audit area**:
- Continue constants cleanup in ACP request timeout sites, retry policies, and workflow/tool iteration defaults.

### Batch 42 — Roko-ACP Request Timeout Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S6.1 request-timeout cleanup continued after batches 39-41. `crates/roko-acp/src` had one remaining `120_000` provider timeout fixture in bridge-events tests.

**Implemented**:
- Updated the bridge-events Anthropic provider fixture to use `DEFAULT_REQUEST_TIMEOUT_MS`.
- Updated the same fixture to use `DEFAULT_CONNECT_TIMEOUT_MS`.
- Confirmed no request-timeout `120_000` patterns remain under `crates/roko-acp/src`.

**Verified**:
- `cargo check -p roko-acp --jobs 1`.
- `cargo test -p roko-acp anthropic_model_call_config_routes_legacy_claude_to_anthropic_provider --jobs 1 -- --nocapture`.
- `git diff --check`.

**Remaining from the same audit area**:
- Request-timeout defaults are now cleared across roko-agent, roko-cli, roko-serve, and roko-acp source. Continue with retry policies and workflow/tool iteration defaults.

### Batch 43 — Core Retry Policy Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: S6.1 also called out retry counts and backoff values as scattered magic numbers. The core error taxonomy still encoded retry policy as inline numeric tuples inside `ErrorKind::retry_policy()`, even though `roko-core::defaults` already holds the workspace default namespace.

**Implemented**:
- Added named defaults for rate-limit, timeout, and generic transient retry classes in `crates/roko-core/src/defaults.rs`.
- Rewired `ErrorKind::retry_policy()` to construct `RetryPolicy` values from the named defaults.
- Added an exact regression proving `RateLimited`, `Timeout`, and generic transient error policies match the defaults.
- Extended defaults sanity checks so retry-class relationships are documented by tests.

**Verified**:
- `cargo test -p roko-core retry_policy --jobs 1 -- --nocapture`.
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`.

**Remaining from the same audit area**:
- Active runner/provider workflow and tool-iteration defaults still need cleanup. This batch only centralizes the core error retry policy. Legacy `orchestrate.rs` is feature-gated and not part of the active runner redesign target.

### Batch 44 — Serve Relay Circuit-Breaker Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: After Batch 43 centralized the core error retry policy, `roko-serve/src/relay.rs` still owned its stale-data threshold and heartbeat circuit-breaker defaults beside implementation code. These are server runtime defaults and belong in the shared defaults namespace.

**Implemented**:
- Added relay stale-threshold and circuit-breaker defaults to `crates/roko-core/src/defaults.rs`.
- Rewired relay freshness defaults, heartbeat degradation threshold, exponential backoff base, and backoff cap to consume the core defaults.
- Updated relay behavior tests to derive expected thresholds/backoff values from the defaults instead of duplicating numeric literals.
- Added a core defaults sanity test for relay backoff ordering.

**Verified**:
- `cargo test -p roko-serve circuit_breaker --jobs 1 -- --nocapture`.
- `cargo test -p roko-serve relay_health --jobs 1 -- --nocapture`.
- `cargo test -p roko-core relay_backoff_defaults_are_ordered --jobs 1 -- --nocapture`.

**Remaining from the same audit area**:
- Active runner/provider workflow and tool-iteration defaults still need cleanup. Legacy `orchestrate.rs` retry loops are out-of-scope unless the `legacy-orchestrate` feature is deliberately restored.

### Batch 45 — Runner Plan Timeout and DAG Backoff Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: Runner plan timeout and task-DAG retry backoff defaults were duplicated across the core config schema, CLI config wrapper, runner defaults, and DAG controller. These values define the runner contract and should be tuned from the same defaults namespace.

**Implemented**:
- Added `DEFAULT_PLAN_TIMEOUT_SECS`, `DEFAULT_PLAN_RETRY_BASE_SECS`, `DEFAULT_PLAN_RETRY_MAX_SECS`, and `DEFAULT_PLAN_RETRY_BACKOFF_SHIFT_CAP` to `crates/roko-core/src/defaults.rs`.
- Rewired `CoreRunnerConfig::default_plan_timeout_secs()` and CLI `RunnerConfig::default_plan_timeout_secs()` to the shared timeout default.
- Rewired `RunConfig::default()` to use shared plan timeout and auto-fix retry defaults.
- Rewired `DagConfig::default()` and `DagConfig::backoff_for_attempt()` to use shared backoff defaults.
- Updated DAG and config tests to assert against the shared defaults instead of duplicating values.

**Verified**:
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`.
- `cargo test -p roko-cli runner::task_dag::tests --jobs 1 -- --nocapture`.
- `cargo test -p roko-cli parses_minimal_config --jobs 1 -- --nocapture`.

**Remaining from the same audit area**:
- Active runner/provider workflow and tool-iteration defaults still need cleanup. Do not spend further redesign effort on `crates/roko-cli/src/orchestrate.rs` unless it stops being a legacy feature path.

### Batch 46 — Active Provider Tool-Loop Iteration Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: Active provider adapters still passed numeric defaults into `tool_loop_max_iterations(...)`. Most used `50`, but the OpenAI-compatible path still used `25`, so the workspace default was not actually centralized across provider tool-loop dispatch.

**Implemented**:
- Changed `crates/roko-agent/src/provider/mod.rs::tool_loop_max_iterations()` to own the default from `roko_core::defaults::DEFAULT_MAX_TOOL_ITERATIONS`.
- Updated Anthropic, Gemini, Cerebras, Perplexity, and OpenAI-compatible adapters to call the shared helper without per-provider numeric defaults.
- Aligned the OpenAI-compatible tool-loop path with the workspace default, keeping temperament adjustments in the helper.
- Added a regression for the default and temperament-adjusted iteration budgets.

**Verified**:
- `cargo check -p roko-agent --jobs 1`.
- `cargo test -p roko-agent tool_loop_iterations_derive_from_workspace_default --jobs 1 -- --nocapture`.
- `rg 'tool_loop_max_iterations\\((25|50|[0-9_]+)\\)' crates/roko-agent/src` returns no matches.

**Remaining from the same audit area**:
- Active workflow iteration limits outside provider tool-loop adapters still need review. Legacy `orchestrate.rs` remains out-of-scope.

### Batch 47 — Vision-Loop Defaults (2026-05-04)

**Status**: Complete and ready to commit after final verification.

**Context**: The CLI vision-loop config and serve API route each owned duplicate defaults for max iterations, target score, viewport size, and write-settle timing. These defaults define one user-facing workflow and should not drift between local CLI and serve-triggered runs.

**Implemented**:
- Added vision-loop constants to `crates/roko-core/src/defaults.rs`.
- Rewired `crates/roko-cli/src/vision_loop/mod.rs` defaults and tests to the shared constants.
- Rewired `crates/roko-serve/src/routes/vision_loop.rs` request defaults to the same constants.
- Added defaults sanity coverage for the vision-loop iteration/score relationship in core.

**Verified**:
- `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`.
- `cargo test -p roko-cli default_config_has_sensible_values --jobs 1 -- --nocapture`.
- `cargo check -p roko-serve --jobs 1`.
- `rg` check confirms the old inline CLI/serve vision-loop default patterns are gone.

**Remaining from the same audit area**:
- Active workflow iteration defaults outside provider tool-loop and vision-loop entry points still need review. Legacy `orchestrate.rs` remains out-of-scope.

### Batch 20 — Explicit Effective Provider Registry (2026-05-03)

**Status**: Complete and ready to commit after final verification.

**Context**: Core `RokoConfig::effective_providers()` still created a `claude_cli` provider when the provider table was empty. That preserved a hidden provider path after model synthesis and provider-factory command synthesis had already been removed.

**Implemented**:
- Changed empty-config `effective_providers()` to return an empty registry.
- Preserved compatibility behavior only for explicitly configured providers: `[providers.claude_cli]` can still inherit `agent.command` or default to `claude` when the provider exists.
- Updated core regression coverage so legacy Anthropic env values and empty provider config produce no effective providers.
- Left command-backed CLI compatibility at the boundary where it now creates explicit transient provider/model entries before dispatch.

**Verified**:
- Core regression test for no empty-config provider synthesis.
- Root config compatibility test for explicit provider entries.
- Provider-factory missing-config regression.
- Compile check across core, agent, CLI, serve, and ACP using an isolated target dir because local `cargo-watch` processes hold the default target lock.

**Remaining from the same audit area**:
- Repeated direct-call feedback persistence should be extracted into a shared recorder service.

---

## Phase 0: Critical Boot & Terminal Fixes (2026-05-04)

*These are P0 issues that cause the CLI to freeze or crash before the user can do anything. Must be addressed before any other work.*

### 0.1 Terminal Raw Mode RAII Guard

**Why**: `enable_raw_mode()` in `chat_inline.rs` has no cleanup guard. If the process panics, errors, or freezes, the terminal stays in raw mode forever. Ctrl+C becomes a raw keypress instead of SIGINT. The terminal appears "frozen."

**What**: Create an RAII guard and a panic hook:

```rust
// crates/roko-cli/src/inline/terminal.rs
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

// Install panic hook BEFORE entering raw mode
fn install_terminal_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}
```

Replace `enable_raw_mode()` call in `InlineTerminal::new()` with `RawModeGuard::new()`. Store the guard in the struct. Drop happens automatically.

**Files**: `crates/roko-cli/src/inline/terminal.rs` (guard + panic hook), `crates/roko-cli/src/chat_inline.rs` (use guard).

**Effort**: Low (< 1 hour).

### 0.2 Ctrl+C Handling in All Chat Phases

**Why**: The `Phase::Error` handler in `chat_inline.rs:1323-1338` silently discards Ctrl+C via `_ => {}`. The user is trapped in error state with no way to exit except killing the terminal.

**What**: Add Ctrl+C handling to every phase:

```rust
// chat_inline.rs — in the main event loop match
Phase::Error { ref prompt, .. } => {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.phase = Phase::Done;
            break;
        }
        KeyCode::Char('r') => { /* retry */ }
        KeyCode::Char('q') | KeyCode::Esc => { /* cancel */ }
        _ => {}
    }
}
```

Also audit every other match arm that has `_ => {}` to ensure Ctrl+C is handled.

**Files**: `crates/roko-cli/src/chat_inline.rs` (error phase key handling, any other missing Ctrl+C paths).

**Effort**: Low (< 30 min).

### 0.3 Startup Provider Validation

**Why**: The CLI enters the chat REPL and lets the user type a message before discovering that no provider has valid credentials. The "no API key" error should be caught at startup, not after the first message.

**What**: Before entering the chat event loop, validate that the resolved default provider has credentials:

```rust
// crates/roko-cli/src/unified.rs — in cmd_unified_chat()
let bootstrap = RokoBootstrap::new(&workdir, config_path)?;
if bootstrap.available_providers.is_empty() {
    eprintln!("No LLM providers configured with valid API keys.");
    eprintln!();
    eprintln!("  Set an API key:  export ANTHROPIC_API_KEY=sk-...");
    eprintln!("  Or configure:    roko config init");
    eprintln!("  Or use Claude:   Install `claude` CLI");
    return Ok(1);
}
```

**Files**: `crates/roko-cli/src/unified.rs` (startup validation), `crates/roko-cli/src/auth_detect.rs` (make config-aware).

**Effort**: Medium (2-3 hours — needs auth_detect refactor to use config).

### 0.4 ACP Workspace Auto-Creation

**Why**: ACP crashes with "Failed to Launch" when `.roko/` doesn't exist because the log file path `.roko/acp.log` is invalid.

**What**: Auto-create `.roko/` before attempting to write the log file:

```rust
// crates/roko-acp/src/handler.rs — in run_acp_server()
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    // Ensure workspace directory exists
    let log_dir = config.log_file().parent().unwrap_or(Path::new("."));
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir).with_context(|| {
            format!("failed to create log directory {}", log_dir.display())
        })?;
    }

    let _guard = setup_file_logging(config.log_file())...
```

Also add graceful fallback to `/tmp/roko-acp-{pid}.log` if the primary path fails.

**Files**: `crates/roko-acp/src/handler.rs` (auto-create + fallback).

**Effort**: Low (< 30 min).

### 0.5 ACP Error Response Before Exit

**Why**: When ACP fails to start, it writes to stderr (invisible to Zed) and exits. Zed shows a generic "server shut down unexpectedly" error with no diagnostic info.

**What**: Before exiting, send a JSON-RPC error response so the editor can display a meaningful message:

```rust
// crates/roko-cli/src/main.rs — ACP error path
Err(e) => {
    // Try to send a JSON-RPC error response on stdout
    let error_response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": {
            "code": -32603,
            "message": format!("Roko ACP failed to start: {e:#}")
        }
    });
    let _ = writeln!(std::io::stdout(), "{}", error_response);
    eprintln!("error: {e:#}");
    EXIT_FAILURE
}
```

**Files**: `crates/roko-cli/src/main.rs` (ACP error path).

**Effort**: Low (< 30 min).

### 0.6 Auth Detection Uses Config (Unified Boot)

**Why**: `detect_auth()` probes env vars independently of the config system. It reports "auth: glm-5.1 (OpenAI-compat)" while the actual dispatch tries to use `anthropic_api` because the config's default model points there. The banner lies.

**What**: Replace the independent `detect_auth()` with a config-aware boot sequence:

```rust
struct RokoBootstrap {
    config: RokoConfig,
    workdir: PathBuf,
    effective_provider: Option<ProviderStatus>,
    available_providers: Vec<ProviderStatus>,
}

struct ProviderStatus {
    key: String,
    kind: ProviderKind,
    has_credentials: bool,
    default_model: Option<String>,
}

impl RokoBootstrap {
    fn new(workdir: &Path, config_path: Option<&Path>) -> Result<Self> {
        // 1. Load unified config
        let config = load_config_unified(workdir)?;

        // 2. Check which providers have credentials
        let available_providers = config.providers.iter()
            .map(|(key, pc)| ProviderStatus {
                key: key.clone(),
                kind: pc.kind,
                has_credentials: check_provider_credentials(pc),
                default_model: find_first_model(&config, key),
            })
            .collect();

        // 3. Resolve the effective provider (config default → first with creds)
        let effective_provider = resolve_effective_provider(&config, &available_providers);

        Ok(Self { config, workdir: workdir.to_owned(), effective_provider, available_providers })
    }

    fn auth_label(&self) -> String {
        match &self.effective_provider {
            Some(p) => format!("{} ({})", p.default_model.as_deref().unwrap_or("unknown"), p.kind),
            None => "no provider available".to_string(),
        }
    }
}
```

The banner now shows the provider that will actually be used. If no provider has credentials, the CLI prints setup instructions and exits before entering the chat REPL.

**Files**: `crates/roko-cli/src/unified.rs` (use bootstrap), `crates/roko-cli/src/auth_detect.rs` (rewrite to use config), `crates/roko-cli/src/chat_inline.rs` (receive bootstrap instead of AuthMethod).

**Effort**: Medium (3-4 hours — significant refactor of auth detection and chat initialization).

### Phase 0 Summary

| Item | Effort | Blocks |
|------|--------|--------|
| 0.1 Raw mode guard | Low | Nothing (standalone) |
| 0.2 Ctrl+C in all phases | Low | Nothing (standalone) |
| 0.3 Startup provider validation | Medium | 0.6 (auth refactor) |
| 0.4 ACP workspace auto-create | Low | Nothing (standalone) |
| 0.5 ACP error response | Low | Nothing (standalone) |
| 0.6 Auth uses config (unified boot) | Medium | Phase 1.4 ideally, but can start with current loader |

Items 0.1, 0.2, 0.4, 0.5 can be done immediately with no dependencies. Items 0.3 and 0.6 are best done together and benefit from (but don't strictly require) the unified config loader from Phase 1.4.

---

## Phase 1: Core Foundations

*Everything else depends on these. Do first, in order.*

### 1.1 Error Type Hierarchy

**Why first**: Every subsequent phase needs consistent error handling. Currently mixing `anyhow::Result`, `Box<dyn Error>`, custom enums, and silent swallowing. Fix this once so all new code has a clear pattern.

**What**:
- Define `thiserror` enum per crate: `CoreError`, `AgentError`, `ServeError`, `CliError`, `LearnError`, `ComposeError`, `GateError`
- Each enum has typed variants for its domain (e.g., `AgentError::Provider`, `AgentError::ToolLoop`, `AgentError::Timeout`)
- Crate-boundary functions use their own error type. `anyhow::Result` only at the CLI binary boundary (`main.rs`)
- Every `if let Ok(x)` or `.ok()` that swallows errors must either propagate with `?`, log at `warn!`/`error!`, or have `// intentionally ignoring: <reason>`
- Replace all `.unwrap()` / `.expect()` in non-test code with `?` or `.unwrap_or_default()`

**Files**: Every crate gets a `src/error.rs`. Start with `roko-core`, then `roko-agent`, then the rest.

**Validation**: `cargo clippy --workspace` passes. `grep -rn '\.unwrap()' --include='*.rs' crates/ | grep -v test | grep -v '#\[test\]'` returns zero non-justified results.

### 1.2 Central Constants Module (**95% COMPLETE as of 2026-05-03**)

**Why**: 30+ hardcoded `Some(15_000)`, scattered DEFAULT_MAX_TOKENS (16384 vs 4096), retry counts (3, 5, 10), iteration limits (25, 50). Every provider independently picks defaults.

**Status**: `defaults.rs` exists with 60+ constants. 6 hardcoded 4096 content-truncation sites migrated to `DEFAULT_TOOL_OUTPUT_TRUNCATE_AT`. Batches 39-42 cleared roko-agent, roko-cli, roko-serve, and roko-acp request-timeout defaults to `DEFAULT_REQUEST_TIMEOUT_MS`. Batch 43 centralized the core error retry policy defaults. Batch 44 centralized serve relay stale-threshold and circuit-breaker defaults. Batch 45 centralized runner plan timeout and task-DAG backoff defaults. Batch 46 centralized active provider tool-loop iteration defaults. Batch 47 centralized vision-loop defaults across CLI and serve. Remaining: active workflow iteration defaults outside provider tool-loop and vision-loop entry points. Legacy `orchestrate.rs` is feature-gated behind `legacy-orchestrate` and should not drive the redesign work.

**What**: Create `roko-core/src/defaults.rs`:

```rust
// Timeouts
pub const DEFAULT_TTFT_TIMEOUT_MS: u64 = 15_000;
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 120_000;
pub const DEFAULT_SHUTDOWN_DRAIN_SECS: u64 = 15; // was 3

// Token budgets
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 16_384;
pub const DEFAULT_MAX_TOOL_ITERATIONS: usize = 50; // unify at 50 for all providers
pub const DEFAULT_MAX_RESULT_BYTES: usize = 65_536; // was 16KB, too small

// Retry
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_INITIAL_BACKOFF_MS: u64 = 500;
pub const DEFAULT_RETRY_MAX_BACKOFF_MS: u64 = 30_000;

// Resource limits
pub const DEFAULT_MAX_FILE_READ_BYTES: usize = 10 * 1024 * 1024; // 10 MB
pub const DEFAULT_MAX_FILE_WRITE_BYTES: usize = 10 * 1024 * 1024;
pub const DEFAULT_MAX_GLOB_RESULTS: usize = 1_000;
pub const DEFAULT_MAX_CONCURRENT_TOOLS: usize = 8;

// Workspace
pub const DEFAULT_WORKSPACE_GC_INTERVAL_SECS: u64 = 300; // 5 min, was 3600
```

Replace every hardcoded occurrence. `grep -rn 'Some(15_000)\|Some(15000)\|15_000\|16_384\|4096' crates/ --include='*.rs'` returns only references to the constants module.

**Files**: `roko-core/src/defaults.rs`, then ~40 files importing these constants.

### 1.3 Atomic File I/O Utility

**Why**: Non-atomic writes in state persistence, tool handlers, episode logger, cascade router, PRD promote. Crash during write = corrupted file.

**What**: Create `roko-core/src/io.rs`:

```rust
use std::path::Path;
use std::io;

/// Atomic write: write to .tmp, then rename. Safe on same filesystem.
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Atomic write async variant.
pub async fn atomic_write_async(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, data).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

/// Read file, returning Ok(None) for NotFound instead of Err.
pub fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
```

Replace every `std::fs::write` / `tokio::fs::write` in state-critical paths with `atomic_write`. Replace every `if path.exists() { fs::read_to_string(...) }` TOCTOU with `read_optional`.

**Files**: `roko-core/src/io.rs`, then cascade_router, episode_logger, orchestrate state saves, PRD promote, tool handlers (write_file, edit_file, apply_patch, multi_edit).

### 1.4 Unified Config Loader

**Why**: **12 separate config loading implementations** across the codebase, each with different behavior. ACP doesn't load global config (`~/.roko/config.toml`), so Zed integration in non-roko projects shows only Anthropic/Sonnet. Serve ignores `ROKO__*` env vars. CLI calls `merge_global_providers()` at 8+ sites manually. Same machine, different effective config depending on which binary entry point is used.

**Current state** (all 12 loaders):

| # | Function | Location | Global? | Env vars? |
|---|----------|----------|---------|-----------|
| 1 | `AcpConfig::load_roko_config` | `roko-acp/src/config.rs:48` | **No** | ROKO_CONFIG only |
| 2 | `load_roko_config` | `roko-cli/src/config_helpers.rs:121` | **No** | **No** |
| 3 | `load_roko_config` | `roko-cli/src/orchestrate.rs:858` | **No** | **No** |
| 4 | `load_roko_config` | `roko-cli/src/agent_serve.rs:559` | **No** | **No** |
| 5 | `load_roko_config` | `roko-cli/src/main.rs:2480` | **No** | **No** |
| 6 | `load_roko_config` | `roko-cli/src/event_sources.rs:77` | **No** | **No** |
| 7 | `load_roko_config` | `roko-cli/src/subscriptions.rs:249` | **No** | **No** |
| 8 | `load_roko_config` | `roko-cli/src/vision_loop/orchestrator.rs:300` | **No** | **No** |
| 9 | `load_roko_config` | `roko-serve/src/lib.rs:435` | **No** | **No** |
| 10 | `AppState::load_roko_config` | `roko-serve/src/state.rs:626` | **No** | Cached |
| 11 | `load_layered` | `roko-cli/src/config.rs:2877` | Via merge | ROKO__* |
| 12 | `load_config` | `roko-core/src/config/mod.rs:115` | **No** | **No** |

**What**: Replace all 12 with a single `roko-core/src/config/loader.rs`:

```rust
pub struct LoadOptions {
    pub merge_global: bool,       // default: true
    pub apply_env_overrides: bool, // default: true (ROKO__* vars)
    pub strict_validation: bool,   // default: false (ACP needs lenient)
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self { merge_global: true, apply_env_overrides: true, strict_validation: false }
    }
}

/// Single config loader used by CLI, serve, ACP, and agent-server.
/// 1. Find file: ROKO_CONFIG env var > ancestor search > workdir/roko.toml
/// 2. Parse TOML
/// 3. Merge global config from ~/.roko/config.toml (providers, models)
/// 4. Apply ROKO__* env var overrides
/// 5. Validate schema (fail-fast with clear error)
/// 6. Return validated config
pub fn load_config(workdir: &Path, opts: LoadOptions) -> Result<RokoConfig, CoreError> { ... }

/// Convenience: load with all defaults (global merge + env overrides).
pub fn load_config_default(workdir: &Path) -> Result<RokoConfig, CoreError> {
    load_config(workdir, LoadOptions::default())
}

/// Returns ~/.roko/config.toml path. Now in roko-core, not roko-cli.
pub fn global_config_path() -> PathBuf { ... }

/// Merge providers/models from global config into project config.
/// Now in roko-core, not roko-cli.
fn merge_global(config: &mut RokoConfig) { ... }

/// Dump effective config as TOML (for debugging, workspace creation, config export).
pub fn serialize_effective(config: &RokoConfig) -> Result<String, CoreError> { ... }
```

**Batch 25 progress**: The core loader now exposes `load_config_file(path, opts)` for exact editor/CLI config paths. ACP uses it for `--config`, so nonstandard explicit config filenames no longer fall back to parent-directory discovery.

**Batch 26 progress**: ACP no longer has a static Anthropic/Sonnet fallback. Empty configs remain visible as empty provider/model option lists, which makes missing global/project config explicit instead of silently selecting a provider.

**Batch 27 progress**: ACP persisted sessions now revalidate provider/model selections against the current config during `session/resume`, and config options are rebuilt before returning the resumed session.

**Batch 28 progress**: ACP provider options now show unavailable configured providers with status text instead of hiding them when credentials are missing.

**Batch 29 progress**: CLI global path, project discovery, and global merge helpers now call the core loader helpers instead of maintaining copied implementations.

**Batch 30 progress**: ACP live provider/model config updates now reject unknown provider/model IDs and cross-provider model selections.

**Migration**:
- Move `global_config_path()` and `merge_global_providers()` from `roko-cli/src/config.rs` into `roko-core/src/config/loader.rs`
- Delete all 10 one-off `load_roko_config` functions (#2-#10). Replace each callsite with `load_config_default(workdir)?`
- Update `AcpConfig::load_roko_config` to call `load_config(workdir, LoadOptions::default())` — this is the fix for Zed integration seeing all providers
- CLI's `load_layered` calls `load_config_default()` then wraps with `ResolvedConfig`/`ConfigSources`
- Delete the 8 manual `merge_global_providers()` calls from CLI code — it's now built into the loader
- Config validated at startup. Invalid config = server doesn't start (fail-fast, clear error message)
- Workspace creation uses `serialize_effective()` to write resolved config, not blind `fs::copy`

**Files**: `roko-core/src/config/loader.rs` (new), `roko-core/src/config/mod.rs`, `roko-acp/src/config.rs`, `roko-serve/src/lib.rs`, `roko-cli/src/config.rs`, `roko-cli/src/config_helpers.rs`, `roko-cli/src/orchestrate.rs`, `roko-cli/src/agent_serve.rs`, `roko-cli/src/main.rs`, `roko-cli/src/event_sources.rs`, `roko-cli/src/subscriptions.rs`, `roko-cli/src/vision_loop/orchestrator.rs`, `roko-cli/src/run.rs`, `roko-cli/src/chat_session.rs`, `roko-cli/src/chat_inline.rs`, `roko-cli/src/serve_runtime.rs`, `roko-cli/src/learning_helpers.rs`, `roko-serve/src/routes/workspaces.rs`.

### 1.5 RetryPolicy Utility

**Why**: Retry-without-backoff throughout. Immediate retry loops hammer failing providers.

**What**: Create `roko-core/src/retry.rs`:

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub jitter: bool,
}

impl RetryPolicy {
    pub fn default_llm() -> Self { /* 3 attempts, 500ms initial, 30s max, jitter */ }
    pub fn default_io() -> Self { /* 3 attempts, 100ms initial, 5s max, jitter */ }

    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // exponential backoff with optional jitter
    }
}
```

Replace all `for _ in 0..3 { if let Ok(...) }` loops.

---

## Phase 2: Provider Layer Redesign

*Depends on Phase 1 (error types, constants, retry). Fixes the core model/provider abstraction.*

### 2.1 Streaming-First Backend Trait

**Why**: Non-streaming is the default today. User sees nothing for minutes. TTFT unmeasurable. No progress feedback. Streaming was bolted on as an afterthought.

**What**: Redesign `LlmBackend` to be streaming-first:

```rust
pub struct StreamEvent {
    pub kind: StreamEventKind,
    pub timestamp: Instant,
}

pub enum StreamEventKind {
    FirstToken,                       // TTFT measurable
    TextDelta(String),                // partial text
    ToolCallStart { name: String },   // tool call beginning
    ToolCallDelta { json: String },   // partial tool-call args
    ToolCallEnd { id: String, name: String, args: Value },
    Usage(Usage),                     // final usage stats
    Done { finish_reason: String },   // completion
}

#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Primary: streaming response.
    async fn stream_turn(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &TurnConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>, AgentError>;
}
```

- Every provider implements `stream_turn`.
- `send_turn` (non-streaming) is a helper that collects the stream.
- `StreamEvent::FirstToken` emitted when first chunk arrives → real TTFT.
- `StreamEvent::TextDelta` printed to stderr immediately → user sees progress.
- `StreamEvent::ToolCallStart` printed as `[model] calling write_file...` → tool call visibility.

**TurnConfig** replaces scattered parameters:
```rust
pub struct TurnConfig {
    pub max_tokens: u32,        // from model profile or default
    pub temperature: Option<f32>,
    pub ttft_timeout: Duration,
    pub request_timeout: Duration,
    pub stop_sequences: Vec<String>,
}
```

**Files**: `roko-agent/src/backend.rs` (new trait), all provider adapters, `tool_loop/mod.rs`, `openai_compat_backend.rs`, `claude_cli_agent.rs`.

### 2.2 Unified Token Budget System

**Why**: Three different DEFAULT_MAX_TOKENS (16384 OpenAI-compat, 4096 Anthropic, N/A Claude CLI). Missing `max_output` on many models in roko.toml causes truncated tool calls. No per-model iteration cap.

**What**:

1. Add fields to `ModelProfile`:
```rust
pub struct ModelProfile {
    // existing fields...
    pub max_output: Option<u64>,           // already exists, populate for ALL models
    pub max_tool_iterations: Option<usize>, // NEW: per-model override
    pub token_budget_strategy: TokenBudgetStrategy, // NEW
}

pub enum TokenBudgetStrategy {
    Fixed,      // use max_output as-is
    Adaptive,   // double on finish_reason=length, up to context_window/2
}
```

2. Update `roko.toml` with correct `max_output` for every model:

| Model | max_output | Source |
|---|---|---|
| claude-opus | 32768 | Anthropic docs |
| claude-sonnet | 16384 | Anthropic docs |
| claude-haiku | 16384 | Anthropic docs |
| gpt-4o | 16384 | OpenAI docs |
| gpt-5.4-mini | 16384 | OpenAI docs |
| o3 | 100000 | OpenAI docs |
| o3-mini | 100000 | OpenAI docs |
| gemini-2.5-pro | 65536 | Google docs |
| gemini-2.5-flash | 65536 | Google docs |
| kimi-k2.5 | 65535 | Moonshot docs |
| glm-5.1 | 16384 | ZhiPu docs |
| sonar-pro | 16384 | Perplexity docs |

3. Unify iteration limits at 50 for all providers (remove the OpenAI-compat special case of 25).

4. Fix `tool_format` for Claude models: `"anthropic_blocks"` not `"openai_json"`.

5. Add `max_tool_iterations` to roko.toml for models that need special handling.

**Batch 36 update (2026-05-04)**: Coverage portion completed. Every configured non-embedding model in root `roko.toml` and `docker/railway.roko.toml` now has explicit `max_output`, with a roko-core regression guarding both files. Adaptive retry, per-model iteration caps, and token-budget strategy fields remain open.

**Files**: `roko-core/src/config/agent.rs` (ModelProfile), `roko.toml`, `docker/railway.roko.toml`, all provider adapters.

### 2.3 Fix Provider-Specific Bugs

Each fix is small but critical. Do them all in this phase:

| Bug | Fix | File |
|---|---|---|
| Gemini URL double-prefix | Remove `/v1beta/openai` from roko.toml base_url OR from adapter | `gemini/adapter.rs:30`, `roko.toml` |
| Claude CLI zero-token usage | Parse `result` event for usage from Claude CLI stdout | `claude_cli_agent.rs` |
| Claude CLI `finish_reason` always None | Extract from `result` event `stop_reason` field | `claude_cli_agent.rs`, `translate/mod.rs` |
| Anthropic API tool loop is dead code | Either wire it (add models pointing to it) or delete it | `anthropic_api/tool_loop.rs` |
| `render_assistant_message` missing for Anthropic/Ollama | Implement for all translators | `translate/claude.rs`, `translate/ollama.rs` |
| Tool name sanitization asymmetry | Apply sanitize/unsanitize in Ollama translator too | `translate/ollama.rs` |
| `supports_partial` continuation never used | Wire partial continuation on `finish_reason=length` for OpenAI-compat | `tool_loop/mod.rs` |

### 2.4 Real TTFT Measurement

**Why**: `ResponseMetadata.provider_latency_ms` exists but is never populated. TTFT EMA in `LatencyRegistry` is derived from internal timestamps (inaccurate).

**What**: With the streaming-first backend (2.1), TTFT is measured naturally:

```rust
// In the stream adapter:
let request_start = Instant::now();
let mut stream = backend.stream_turn(...).await?;

// First chunk:
let first = stream.next().await;
let ttft = request_start.elapsed();
metadata.provider_ttft_ms = Some(ttft.as_millis() as u64);
```

Store in `GatewayEvent.ttft_ms` (new field). Feed to `LatencyRegistry` for accurate EMA. Emit as `roko_llm_ttft_seconds` Prometheus histogram.

---

## Phase 3: Tool System Redesign

*Depends on Phase 1 (error types, constants) and Phase 2 (streaming backend, TurnConfig).*

### 3.1 Proper JSON Schemas for All Tools (**COMPLETE 2026-05-03**)

~~**Why**: Every tool sends `{"type": "object"}` with no properties or required fields.~~

**DONE**: All 16 standard tools now have full JSON Schema via `ToolSchema::from_value(json!({...}))` with:
- `properties` with per-parameter `type` and `description`
- `required` arrays for mandatory arguments
- `additionalProperties: false` for strict validation
- `enum` constraints where applicable (e.g. grep modes, build systems)
- Array item schemas for multi_edit.edits and todo_write.todos

Golden tests updated (count 30→33, all 33 pass). TOOL_COUNT constant updated.
Registry-level `validate_against_schema` pre-dispatch validation is a follow-up item.

**Files modified**: All 16 `crates/roko-std/src/tool/builtin/*.rs`, `registry.rs`, `golden_tools.rs`.

### 3.2 Safety Layer: Required, Not Optional

**Why**: `ToolDispatcher::new()` has `safety: None` by default. Any dispatcher without `.with_safety()` has zero checks.

**What**:

```rust
impl ToolDispatcher {
    /// Production constructor — safety required.
    pub fn new(
        registry: Arc<dyn ToolRegistry>,
        resolver: Arc<dyn HandlerResolver>,
        safety: SafetyLayer,
    ) -> Self { ... }

    /// Test-only constructor — no safety checks.
    #[cfg(test)]
    pub fn new_unguarded(
        registry: Arc<dyn ToolRegistry>,
        resolver: Arc<dyn HandlerResolver>,
    ) -> Self { ... }
}
```

All production code paths must provide a `SafetyLayer`. Compile error if they don't (except in tests).

### 3.3 Fix Contract Loading

**Why**: Contracts loaded via `env!("CARGO_MANIFEST_DIR")` — path doesn't exist in deployed binaries. `RestrictedFallback` silently denies all tools.

**What**: Embed contract YAML files into the binary using `include_str!`:

```rust
const IMPLEMENTER_CONTRACT: &str = include_str!("contracts/implementer.yaml");
const STRATEGIST_CONTRACT: &str = include_str!("contracts/strategist.yaml");
// ...

fn load_contract(role: &str) -> Result<AgentContract> {
    let yaml = match role {
        "implementer" => IMPLEMENTER_CONTRACT,
        "strategist" => STRATEGIST_CONTRACT,
        // ...
        _ => return Ok(AgentContract::permissive(role)), // unknown roles: permissive with warning
    };
    parse_contract(yaml)
}
```

Fix `implementer.yaml` ForbiddenTools: `["network", "fetch"]` → `["web_fetch", "web_search"]`.

**Batch 38 update (2026-05-04)**: Implemented the embedded-contract approach in `crates/roko-agent/src/safety/contract.rs`. `AgentContract::load_for_role()` now parses from an `include_str!` role registry for all bundled contracts and no longer depends on `env!("CARGO_MANIFEST_DIR")` or runtime source files. Unknown roles still fail closed through the configured load mode rather than becoming permissive.

### 3.4 Fix Safety Bypasses

**Why**: `gate_passed: true` in tool call arguments bypasses `RequireGateBeforeCommit`. `estimated_tokens` from LLM bypasses `MaxTokensPerTurn`.

**What**:
- Remove `has_gate_approval` from checking tool-call arguments entirely. Gate approval must come from the `ToolContext` (set by the orchestrator after gates actually pass), not from LLM-supplied fields.
- Remove `estimated_tokens` reading from tool-call arguments. Token counting must use actual `Usage` data from the backend, tracked in `ToolContext`.

```rust
fn has_gate_approval(ctx: &ToolContext) -> bool {
    ctx.gate_passed  // set by orchestrator, not by LLM
}

fn estimated_tokens(ctx: &ToolContext) -> u32 {
    ctx.session_tokens_used  // tracked by tool loop, not by LLM
}
```

**Batch 35 update (2026-05-04)**: The immediate trust-boundary bypass is fixed in `crates/roko-agent/src/safety/contract.rs`. Gate approval comes only from `ToolContext.external_actions`; `estimated_tokens`, `max_tokens`, and `estimated_cost_usd` call-argument claims are ignored; and `MaxCostPerTurn` enforces cost already recorded in external-action metadata. Remaining work is provider/tool-loop instrumentation so every live model spend path records cost into that context.

### 3.5 Resource Limits on Tool Execution

**Why**: `read_file` reads multi-GB files into memory. `write_file` can fill disk. `glob` returns unbounded results. No concurrency cap on parallel dispatch.

**What**: Add `ResourceLimits` to `ToolContext`:

```rust
pub struct ResourceLimits {
    pub max_file_read_bytes: usize,
    pub max_file_write_bytes: usize,
    pub max_glob_results: usize,
    pub max_concurrent_tools: usize,
}
```

Enforce at handler level:
- `read_file`: check file size before reading. If > limit, return error with size info.
- `write_file`: check content.len() before writing. If > limit, return error.
- `glob`: stop collecting after max results, add `[truncated: N more matches]`.
- Parallel dispatch: use `tokio::sync::Semaphore(max_concurrent_tools)` around `join_all`.

### 3.6 Detect and Handle Truncated Tool Calls

**Why**: `__truncated` tool calls pass through silently. Tool handler fails with garbage args. User sees nothing.

**What**: In the tool dispatcher, before dispatching:

```rust
if call.arguments.get("__truncated").and_then(|v| v.as_bool()) == Some(true) {
    return ToolResult::err(ToolError::Truncated {
        tool: call.name.clone(),
        raw_len: call.arguments.get("raw").map(|r| r.as_str().map(|s| s.len())),
        message: format!(
            "Your {} tool call was truncated because you exceeded the output token limit. \
             Split the content into smaller pieces or reduce the size.",
            call.name
        ),
    });
}
```

This gives the LLM actionable feedback to fix its approach.

### 3.7 Fix Contract Double-Check

**Why**: `check_pre_execution` and `check_contract` both call the contract internally — every invariant evaluated twice per dispatch.

**What**: Remove the redundant `check_contract` call in `ToolDispatcher::dispatch`. Keep only `check_pre_execution` (which already calls the contract).

---

## Phase 4: Orchestration Redesign

*Depends on Phase 1-3. Fixes plan execution, state management, progress feedback.*

### 4.1 Progress Event Bus

**Why**: No unified progress feedback. CLI silent during LLM calls. Demo app has no status. TUI has no real-time updates from agent dispatch.

**What**: Define a progress event enum and a broadcast channel:

```rust
pub enum ProgressEvent {
    // Agent lifecycle
    AgentCreating { model: String, provider: String },
    AgentStreamStarted { model: String, ttft_ms: u64 },
    AgentTextDelta { text: String },
    AgentToolCallStarted { tool: String, iteration: usize, max_iterations: usize },
    AgentToolCallComplete { tool: String, duration_ms: u64 },
    AgentTurnComplete { iteration: usize, usage: Usage },
    AgentComplete { total_usage: Usage, duration_ms: u64 },
    AgentFailed { error: String },

    // Pipeline
    PipelinePhase { phase: String, status: PhaseStatus },

    // Gate
    GateRungStarted { rung: String },
    GateRungComplete { rung: String, passed: bool, duration_ms: u64 },
}

pub enum PhaseStatus { Started, Running, Completed, Failed(String) }
```

Wire into:
- **CLI stderr**: print human-readable progress lines
- **Demo app**: SSE stream from roko-serve
- **TUI**: update dashboard panels
- **Bench**: capture timing metrics

Every `ToolLoopAgent`, `ClaudeCliAgent`, and orchestrate path emits events to the bus.

### 4.2 Config Caching

**Why**: `load_roko_config` called 70+ times per plan run. O(tasks × rungs) disk reads.

**What**: Create `ConfigCache` using `arc-swap`:

```rust
pub struct ConfigCache {
    config: arc_swap::ArcSwap<RokoConfig>,
    _watcher: notify::RecommendedWatcher,
}

impl ConfigCache {
    pub fn new(workdir: &Path) -> Result<Self> {
        let config = load_config(workdir)?;
        let config = Arc::new(config);
        let swap = ArcSwap::from(config);
        // watch workdir/roko.toml for changes, reload on modify
        Ok(Self { config: swap, _watcher: watcher })
    }

    pub fn get(&self) -> arc_swap::Guard<Arc<RokoConfig>> {
        self.config.load()
    }
}
```

Pass `ConfigCache` to `PlanRunner`, agent dispatch, gate pipeline. Never call `load_roko_config` directly again.

### 4.3 Atomic Group State Persistence

**Why**: Three state files written sequentially. Crash between writes = inconsistent state on resume.

**What**: Write all state as a single atomic unit:

```rust
fn save_state(&self) -> Result<()> {
    let snapshot = StateSnapshot {
        executor: self.executor.snapshot(),
        events: self.event_log.snapshot(),
        trackers: self.task_trackers.snapshot(),
        version: STATE_VERSION,
        checksum: ..., // SHA256 of concatenated JSON
    };
    let json = serde_json::to_vec(&snapshot)?;
    atomic_write(&self.state_path, &json)?;
    Ok(())
}
```

Single file, single atomic write, single checksum. Resume validates checksum.

Also save `adaptive_thresholds` and `gate_ratchet` in the same snapshot (currently only saved on shutdown).

### 4.4 Fix Parallel Task Error Handling

**Why**: `JoinError` (task panic) silently dropped. Task not recorded as failed.

**What**:
```rust
match joined {
    Ok(pair) => results.push(pair),
    Err(join_error) => {
        let task_id = /* extract from JoinError context */;
        tracing::error!(%task_id, "parallel task panicked: {join_error}");
        record_task_failure(task_id, &join_error.to_string());
        any_fatal = true;
    }
}
```

### 4.5 Smart Exit Codes

**Why**: `prd draft new` exits non-zero even when artifact is created (demo shows false failure).

**What**: Define exit codes for each command:

```rust
pub mod exit {
    pub const SUCCESS: i32 = 0;
    pub const AGENT_FAILED: i32 = 1;
    pub const EMPTY_ARTIFACT: i32 = 2;
    pub const VALIDATION_FAILED: i32 = 3;
    pub const TIMEOUT: i32 = 4;
    pub const TOKEN_BUDGET: i32 = 5;
}
```

In `commands/prd.rs`, check artifact existence:
```rust
if artifact_exists && has_substantive_content {
    Ok(exit::SUCCESS)
} else if artifact_exists {
    Ok(exit::EMPTY_ARTIFACT)
} else {
    Ok(exit::AGENT_FAILED)
}
```

### 4.6 Graceful Shutdown

**Why**: `SHUTDOWN_DRAIN_GRACE_SECS = 3` force-kills everything. `SIGTERM` to pgid 0 kills parent processes.

**What**:
- Increase drain grace to 15 seconds (from constant in Phase 1.2)
- Send SIGTERM only to child processes (tracked PIDs), not to process group
- On Ctrl-C: cancel token → agents check token → clean exit
- On second Ctrl-C: force kill children only

### 4.7 Remove Hardcoded Model Strings

**Why**: `"claude-opus-4-6"`, `"claude-sonnet-4-20250514"` (wrong vintage) scattered throughout `orchestrate.rs`.

**What**: Define model references in config, not code:

```toml
# roko.toml
[agent.defaults]
escalation_chain = ["haiku", "sonnet", "opus"]  # references model keys
generic_agent_model = "sonnet"
gate_judge_model = "sonnet"
```

```rust
// orchestrate.rs
let escalation = config.agent.defaults.escalation_chain
    .iter()
    .filter_map(|key| config.models.get(key))
    .collect();
```

No model string literals in Rust source.

---

## Phase 5: Workspace, Serve, and Dev Workflow

*Depends on Phase 1 (config loader, atomic I/O) and Phase 4 (progress bus).*

### 5.1 Persistent Workspace Registry

**Why**: In-memory only. Server restart = all workspaces lost. macOS temp_dir mismatch. No recovery.

**What**:

```rust
// Persisted to .roko/workspaces/registry.json
pub struct WorkspaceRegistry {
    workspaces: HashMap<String, WorkspaceEntry>,
}

pub struct WorkspaceEntry {
    pub id: String,
    pub path: PathBuf,
    pub created: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub config_hash: String,
    pub status: WorkspaceStatus, // Active, Stale, Destroyed
}

impl WorkspaceRegistry {
    /// Load from disk on server startup.
    pub fn load(workdir: &Path) -> Result<Self> { ... }

    /// Get workspace, validating path still exists.
    pub fn get(&mut self, id: &str) -> Option<&WorkspaceEntry> {
        let entry = self.workspaces.get_mut(id)?;
        if !entry.path.exists() {
            entry.status = WorkspaceStatus::Stale;
            return None;
        }
        entry.last_accessed = Utc::now();
        Some(entry)
    }

    /// Create workspace with resolved config (not blind copy).
    pub fn create(&mut self, config: &RokoConfig, prefix: &str) -> Result<WorkspaceEntry> {
        let id = format!("{}-{}", prefix, nanoid::nanoid!(8));
        let path = std::env::temp_dir().join(format!("roko-ws-{}", &id));
        std::fs::create_dir_all(&path)?;
        let config_toml = serialize_effective(config)?;
        atomic_write(&path.join("roko.toml"), config_toml.as_bytes())?;
        // ...
    }
}
```

### 5.2 Terminal Session Reattach

**Why**: Page refresh = new terminal, old session lost. No way to reconnect.

**What**: Two approaches, prefer (a):

**(a) Session persistence**: On WebSocket disconnect, keep PTY alive for 60 seconds. If reconnect arrives with same session ID, reattach. If not, kill PTY.

```rust
struct TerminalSession {
    pty: Child,
    id: String,
    workspace_id: String,
    last_activity: Instant,
    detached: bool,  // true when WS disconnected
    scrollback: VecDeque<Vec<u8>>,  // last 1000 lines
}
```

On reconnect: send scrollback to client, resume streaming.

**(b) tmux backend**: PTY backed by tmux session. Session survives server restart. Reconnect = `tmux attach-session -t {workspace_id}`.

### 5.3 Health Endpoint

**Why**: Railway and other deployment platforms need separate liveness and readiness signals so they can distinguish a live process from a draining instance.

**What**: Add to `roko-serve/src/routes/`:

```rust
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: state.started_at.elapsed().as_secs(),
        workspace_count: state.workspace_registry.read().await.len(),
        active_agents: state.active_runs.read().await.len(),
    })
}
```

Register at `GET /health` and `GET /ready`.

**Batch 37 update (2026-05-04)**: Implemented top-level unauthenticated `GET /health` and `GET /ready` in `crates/roko-serve/src/routes/mod.rs`. `/api/health` remains the richer telemetry route. `GET /ready` now returns `503` once the server cancellation token is tripped, and `roko up` now shuts the serve task down through `start_server_background()` instead of aborting it.

### 5.4 Prometheus `/metrics` Endpoint

**Why**: `MetricRegistry` exists and renders Prometheus text format, but no HTTP endpoint.

**What**: Add route:

```rust
async fn metrics(State(state): State<AppState>) -> String {
    state.metrics.render_prometheus()
}
```

Register at `GET /metrics`.

### 5.5 Dev Orchestrator (`roko dev`)

**Why**: `roko-dev-full` alias causes triple process spawn, port conflicts, cargo lock contention.

**What**: New CLI subcommand:

```
roko dev
  ├── cargo watch -w crates/ -x "build -p roko-cli"  (rebuild only)
  ├── serve manager:
  │   ├── watch target/debug/roko for changes
  │   ├── kill old serve, wait for port release
  │   ├── start new serve with SO_REUSEADDR + retry
  │   └── PID file at .roko/serve.pid
  └── npm run dev (in demo/demo-app/)
```

Signal handling: parent catches SIGINT/SIGTERM, sends to all children, waits 5 seconds, then force-kills.

### 5.6 CORS from Config

**Why**: `CorsLayer::permissive()` hardcoded. Should be restricted in production.

**What**:
```toml
[serve]
port = 6677
cors_origins = ["http://localhost:5173"]  # dev
# cors_origins = ["https://roko.nunchi.dev"]  # prod
```

---

## Phase 6: Learning and Compose Subsystems

*Depends on Phase 1 (atomic I/O, error types) and Phase 2 (streaming, token tracking).*

### 6.1 Wire Episode Log Compaction

**Why**: `compact()` exists. `RetentionPolicy` exists. Neither called at runtime. Log grows unboundedly.

**What**: Call `compact()` after every N writes (e.g., 100 episodes) or on file size threshold:

```rust
impl EpisodeLogger {
    pub async fn log_episode(&self, episode: Episode) -> Result<()> {
        self.append(episode).await?;
        if self.should_compact().await {
            self.compact(RetentionPolicy::default()).await?;
        }
        Ok(())
    }

    async fn should_compact(&self) -> bool {
        let size = tokio::fs::metadata(&self.path).await.ok().map(|m| m.len()).unwrap_or(0);
        size > 5_000_000 // 5 MB
    }
}
```

### 6.2 Fix Episode ID Stability

**Why**: `DefaultHasher` algorithm changes between Rust versions. Episode IDs break on toolchain update.

**What**: Replace with a stable hash:

```rust
use std::hash::{Hash, Hasher};
use siphasher::SipHasher13;

fn derive_id(agent_id: &str, task_id: &str, timestamp: DateTime<Utc>) -> String {
    let mut hasher = SipHasher13::new_with_keys(0, 0); // stable, deterministic
    agent_id.hash(&mut hasher);
    task_id.hash(&mut hasher);
    timestamp.timestamp_nanos_opt().unwrap_or(0).hash(&mut hasher);
    format!("ep_{:016x}", hasher.finish())
}
```

### 6.3 Cascade Router Resilience

**Why**: `load_or_new` silently resets all learning on any parse error. No WAL for observations.

**What**:

1. On parse error, log `error!`, try to read backup, then fall back to fresh:
```rust
pub fn load_or_new(path: &Path, model_slugs: Vec<String>) -> Self {
    match Self::try_load(path) {
        Ok(router) => router,
        Err(e) => {
            tracing::error!("cascade router corrupted at {}: {e}. Checking backup.", path.display());
            let backup = path.with_extension("json.bak");
            match Self::try_load(&backup) {
                Ok(router) => { tracing::info!("restored from backup"); router }
                Err(_) => { tracing::error!("no valid backup. Starting fresh."); Self::new(model_slugs) }
            }
        }
    }
}
```

2. On save, write backup first:
```rust
pub fn save(&self, path: &Path) -> Result<()> {
    let backup = path.with_extension("json.bak");
    if path.exists() { std::fs::copy(path, &backup)?; }
    atomic_write(path, &serde_json::to_vec_pretty(&self.snapshot())?)?;
    Ok(())
}
```

### 6.4 Fix Experiment Persistence

**Why**: `record_outcome` never calls `save()`. Concluded experiments don't update cascade router.

**What**: `record_outcome` returns a `bool` indicating conclusion. Caller saves:
```rust
// In orchestrate.rs after recording outcome:
if experiment_store.record_outcome(id, variant, success, cost, tokens, duration) {
    experiment_store.save(&experiments_path)?;
    experiment_store.sync_cascade_router(&mut cascade_router);
}
```

Or better: `record_outcome` takes `&mut self` and auto-saves on conclusion.

### 6.5 Fix Prompt Assembly Issues

| Issue | Fix |
|---|---|
| Duplicate `## Relevant Knowledge` headings | Rename techniques section to `## Relevant Techniques` |
| Heuristic 4.0 chars/token wrong for code | Make configurable per model family: 3.5 for Claude, 4.0 for OpenAI, 4.5 for Gemini |
| `build_with_counter` skips normalization | Call `normalize_for_caching` in `build_with_counter` too |
| `collect_source_context_from` sync+unbounded | Make async, add depth limit (5) and file count limit (500) |
| Episode context reads only live log | Read live + most recent rotated file |

### 6.6 Wire Context Window Pressure Response

**Why**: Watcher emits `conductor.intervention` signal but nothing consumes it.

**What**: In `orchestrate.rs`, subscribe to intervention signals:
```rust
// When context pressure > 80%:
// 1. Summarize conversation history (keep last 3 turns, compress rest)
// 2. Drop low-priority context sections
// 3. Log warning for user visibility
```

Also extend `context_window_tokens` to cover all models (read from `ModelProfile.context_window`), not just hardcoded Claude values.

---

## Phase 7: Frontend Redesign

*Depends on Phase 4 (progress event bus) and Phase 5 (workspace persistence, terminal reattach).*

### 7.1 SSE-Based State Management

**Why**: Polling loops everywhere. `setInterval` for workspace status, agent status, pipeline progress.

**What**: Replace all polling with SSE subscriptions:

```typescript
// lib/sse.ts
export function subscribeToEvents<T>(
    path: string,
    handlers: { [event: string]: (data: T) => void }
): () => void {
    const source = new EventSource(`${config.apiBase}${path}`);
    for (const [event, handler] of Object.entries(handlers)) {
        source.addEventListener(event, (e) => handler(JSON.parse(e.data)));
    }
    return () => source.close();
}

// Usage in component:
useEffect(() => {
    return subscribeToEvents(`/api/workspaces/${id}/events`, {
        'status': (data) => setStatus(data),
        'progress': (data) => setProgress(data),
    });
}, [id]);
```

### 7.2 Error Boundaries and Recovery

**Why**: Single API error crashes entire page. No recovery path.

**What**: React Error Boundary around each panel:

```tsx
<ErrorBoundary fallback={<PanelError onRetry={() => window.location.reload()} />}>
    <PrdPipelinePanel />
</ErrorBoundary>
```

API errors caught and displayed inline with retry button, not propagated to crash the page.

### 7.3 Pipeline State Machine

**Why**: Sequential `await` chain with no timeout, no rollback, no resume.

**What**: Explicit state machine for each scenario:

```typescript
type PipelineStep = 'workspace' | 'idea' | 'draft' | 'plan' | 'tasks' | 'run';
type StepStatus = 'pending' | 'running' | 'done' | 'failed' | 'skipped';

interface PipelineState {
    steps: Record<PipelineStep, { status: StepStatus; error?: string; startedAt?: number }>;
    currentStep: PipelineStep | null;
}

// Each step has a timeout
const STEP_TIMEOUTS: Record<PipelineStep, number> = {
    workspace: 10_000,
    idea: 5_000,
    draft: 180_000,
    plan: 180_000,
    tasks: 10_000,
    run: 600_000,
};
```

Failed steps show error + retry button. Pipeline can resume from last successful step.

### 7.4 WebSocket Reconnect with Backoff

**Why**: Indefinite reconnect at 500ms fixed interval. Hammers server when down.

**What**:
```typescript
const RECONNECT = { initial: 500, max: 30_000, factor: 2, maxRetries: 20 };

function connectWithBackoff(url: string, attempt = 0): WebSocket {
    const ws = new WebSocket(url);
    ws.onclose = () => {
        if (attempt >= RECONNECT.maxRetries) {
            showError("Server unreachable. Please refresh.");
            return;
        }
        const delay = Math.min(RECONNECT.initial * RECONNECT.factor ** attempt, RECONNECT.max);
        setTimeout(() => connectWithBackoff(url, attempt + 1), delay);
    };
    ws.onopen = () => { attempt = 0; }; // reset on success
    return ws;
}
```

### 7.5 Central Config

**Why**: `http://localhost:6677` hardcoded in 10+ places. Timeouts scattered.

**What**:
```typescript
// lib/config.ts
export const config = {
    apiBase: import.meta.env.VITE_API_URL ?? 'http://localhost:6677',
    wsBase: import.meta.env.VITE_WS_URL ?? 'ws://localhost:6677',
    timeouts: { workspace: 10_000, command: 180_000, terminal: 8_000 },
    reconnect: { initial: 500, max: 30_000, factor: 2, maxRetries: 20 },
} as const;
```

---

## Phase 8: Deployment and CI

*Depends on Phase 1 (config loader), Phase 5 (health endpoint, dev orchestrator).*

### 8.1 Proper Dockerfile

```dockerfile
# Stage 1: Frontend
FROM node:22-alpine AS frontend
WORKDIR /app/demo/demo-app
COPY demo/demo-app/package*.json ./
RUN npm ci --production
COPY demo/demo-app/ ./
RUN npm run build

# Stage 2: Rust build
FROM rust:1.91-bookworm AS backend
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY --from=frontend /app/demo/demo-app/dist demo/demo-app/dist
RUN cargo build --release -p roko-cli

# Stage 3: Runtime — NO toolchain (saves ~1.5 GB)
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates tini && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/target/release/roko /usr/local/bin/roko
ENTRYPOINT ["tini", "--"]
CMD ["roko", "serve"]
HEALTHCHECK CMD curl -f http://localhost:6677/health || exit 1
```

### 8.2 Single Config with Profiles

**Why**: `roko.toml` and `docker/railway.roko.toml` drift independently.

**What**: Single `roko.toml`. Railway overrides via env vars:

```bash
# Railway env vars:
ROKO__SERVE__PORT=8080
ROKO__SERVE__CORS_ORIGINS=https://roko.nunchi.dev
```

Delete `docker/railway.roko.toml`. The unified config loader (Phase 1.4) handles env var overrides.

Add `roko config export --env railway` that prints the env vars needed for a deployment target.

### 8.3 CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo +nightly fmt --all -- --check
      - run: cargo clippy --workspace --no-deps -- -D warnings

  test:
    runs-on: ubuntu-latest
    needs: check
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo test --workspace --features integration

  e2e:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci --prefix demo/demo-app
      - run: npm run build --prefix demo/demo-app
      - run: cargo build --release -p roko-cli
      - run: ./target/release/roko serve &
      - run: npx playwright test --prefix demo/demo-app
```

### 8.4 Pin Rust Toolchain

**Why**: CI uses latest stable which may have different lints than local.

**What**: `rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.91"
components = ["rustfmt", "clippy"]
```

### 8.5 Test Isolation

**Why**: Tests use real filesystem and ports, causing flaky failures.

**What**:
- All tests use `tempdir::TempDir` for workspace/state (not `.roko/` in repo)
- Server tests use `port 0` (OS-assigned) via `TcpListener::bind("127.0.0.1:0")`
- Integration tests behind `#[cfg(feature = "integration")]`
- No `--test-threads=1` needed if each test is isolated

---

## Phase 9: Concurrency Fixes

*Can run in parallel with Phase 5-8. Independent of other phases.*

### 9.1 Replace `parking_lot::Mutex` with `tokio::sync::Mutex` in Async Contexts

**Affected**:
- `roko-serve/src/state.rs:361` — `affect_engine`
- `roko-serve/src/state.rs:411` — `relay_health`
- `roko-learn/src/runtime_feedback.rs` — multiple fields

Replace with `tokio::sync::Mutex` or, if the critical section is short and never awaits, keep `parking_lot` but document why.

### 9.2 Merge Dual Mutexes in CacheCell

`model_call_service.rs:870-965`: Replace `entries: Mutex<HashMap>` + `order: Mutex<Vec>` with single `Mutex<(HashMap, Vec)>`.

### 9.3 Replace Unbounded Channels

All `mpsc::unbounded_channel()` → `mpsc::channel(capacity)`:
- Streaming chunks: `channel(1024)`
- Aggregator mux: `channel(256)`
- Event bus: `channel(4096)`
- File watcher: `channel(64)`

When sender fails (receiver full), log warning and drop the message.

### 9.4 Fix Resource Leaks

- `active_runs` / `operations`: Add GC loop (every 60s, remove entries where JoinHandle is finished)
- `MatrixRunHandle`: Join lane handles on removal
- `StdioTransport`: Ensure child kill completes before dropping

### 9.5 Replace Polling with Notification

- `wait_cancelled`: Use `tokio::sync::Notify` instead of 50ms poll
- Concurrency limiter: Use `tokio::sync::Semaphore` instead of 60ms poll
- Retry backoff: Use `tokio::time::sleep` with calculated delay, not fixed 80ms poll

---

## Execution Order

The phases have dependencies. Here's the critical path:

```
Phase 1 (Foundation)           ← DO FIRST, everything depends on this
   ↓
Phase 2 (Provider Layer)       ← streaming, token budgets, provider fixes
   ↓
Phase 3 (Tool System)          ← schemas, safety, resource limits
   ↓
Phase 4 (Orchestration)        ← progress bus, state, exit codes
   ↓
Phase 5 (Serve/Workspace)      ← workspace persistence, health, dev workflow
   ↓
Phase 6 (Learning/Compose)     ← compaction, experiments, prompt fixes
   ↓
Phase 7 (Frontend)             ← SSE, state machines, error boundaries
   ↓
Phase 8 (Deployment/CI)        ← Dockerfile, CI, toolchain pinning

Phase 9 (Concurrency)          ← can run in parallel with 5-8

Phase 10 (ACP/Editor)          ← depends on Phase 1.4 (unified config), can run after Phase 1
```

**Estimated scope**: ~90 files modified, ~16 new files, ~3500 lines added, ~2200 lines removed.

Each phase should be a separate PR (or set of PRs for large phases). Each PR must pass `cargo fmt`, `cargo clippy --workspace --no-deps -- -D warnings`, and `cargo test --workspace`.

---

## Phase 10: ACP / Editor Integration

*Depends on Phase 1.4 (unified config loader). Most items become trivial once 1.4 is done.*

### 10.1 Kill Static Fallback

**Why**: `build_config_options_static` (`session.rs:1050-1077`) hardcodes Anthropic/Sonnet as the only option. With the unified config loader (Phase 1.4) always merging global config, the "no providers found" case should be impossible for any user who has configured `~/.roko/config.toml`.

**What**: After Phase 1.4, `build_config_options` will always receive a config with providers (from global merge). The static fallback becomes dead code. Remove it and replace with an informative error:

```rust
fn build_config_options(
    state: &SessionConfigState,
    roko_config: &RokoConfig,
) -> Vec<ConfigOption> {
    if roko_config.providers.is_empty() {
        tracing::warn!("No providers configured. Run `roko config init` or create ~/.roko/config.toml");
        // Return empty options with a description explaining the issue
        return vec![ConfigOption {
            id: "provider".to_owned(),
            name: "Provider".to_owned(),
            description: Some("No providers configured. Create ~/.roko/config.toml".to_owned()),
            options: Some(vec![]),
            ..Default::default()
        }];
    }
    // ... existing provider/model building logic
}
```

**Files**: `roko-acp/src/session.rs` — delete `build_config_options_static`, update `build_config_options`.

**Implemented in batch 26**: `build_config_options_static` was deleted. Empty configs now return empty provider/model selections and option lists; the next ACP work should make that state actionable with provider/config status messaging.

### 10.2 Session Re-validation on Load

**Why**: `load_from_disk` (`session.rs:770-781`) blindly trusts persisted provider/model values. If config changes between sessions (provider removed, model renamed, workdir changed), the session references stale values. User sees a selected provider/model that no longer exists.

**What**: After loading a session from disk, validate its state against current config:

```rust
fn load_from_disk(&self, session_id: &str, roko_config: &RokoConfig) -> Option<AcpSession> {
    let path = self.sessions_dir().join(format!("{session_id}.json"));
    let data = std::fs::read_to_string(&path).ok()?;
    let mut session: AcpSession = serde_json::from_str(&data).ok()?;

    // Validate provider still exists
    if !roko_config.providers.contains_key(&session.config_state.provider) {
        tracing::info!(
            old_provider = %session.config_state.provider,
            "persisted provider no longer in config, resetting to first available"
        );
        session.config_state.provider = roko_config.providers.keys()
            .next().cloned().unwrap_or_else(|| "anthropic".to_owned());
    }

    // Validate model still exists under selected provider
    let model_valid = roko_config.models.get(&session.config_state.model)
        .is_some_and(|m| m.provider == session.config_state.provider);
    if !model_valid {
        let first_model = roko_config.models.iter()
            .find(|(_, m)| m.provider == session.config_state.provider)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "sonnet".to_owned());
        tracing::info!(
            old_model = %session.config_state.model,
            new_model = %first_model,
            "persisted model no longer valid, resetting"
        );
        session.config_state.model = first_model;
    }

    session.cached_conventions = AcpSession::load_conventions(&self.workdir);
    session.always_allowed = AcpSession::load_workspace_trust(&self.workdir);
    Some(session)
}
```

**Files**: `roko-acp/src/session.rs` — update `load_from_disk`.

**Implemented in batch 27**: Persisted sessions are revalidated on load and their config options are rebuilt from the current config.

### 10.3 Config Change Detection

**Why**: If user edits `~/.roko/config.toml` or project `roko.toml` while Zed is open, the ACP session keeps using the old config. No way to pick up changes without restarting Zed.

**What**: Watch both config files and reload on change:

```rust
pub struct ConfigWatcher {
    config: arc_swap::ArcSwap<RokoConfig>,
    _watcher: notify::RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn new(workdir: &Path) -> Result<Self> {
        let config = load_config_default(workdir)?;
        let swap = ArcSwap::from(Arc::new(config));

        let mut watcher = notify::recommended_watcher(move |event| {
            // Reload config on modify events
            if let Ok(new_config) = load_config_default(workdir) {
                swap.store(Arc::new(new_config));
                tracing::info!("config reloaded");
            }
        })?;

        // Watch both project and global config
        if let Some(project_toml) = find_roko_toml(workdir) {
            watcher.watch(&project_toml, notify::RecursiveMode::NonRecursive)?;
        }
        let global = global_config_path();
        if global.exists() {
            watcher.watch(&global, notify::RecursiveMode::NonRecursive)?;
        }

        Ok(Self { config: swap, _watcher: watcher })
    }

    pub fn get(&self) -> Arc<RokoConfig> {
        self.config.load_full()
    }
}
```

When config changes, rebuild config options for active sessions. The ACP `initialize` response already supports sending updated config options.

**Files**: `roko-acp/src/config.rs` (new `ConfigWatcher`), `roko-acp/src/session.rs` (use watcher instead of one-shot load).

### 10.4 Model Slug Validation at Config Load Time

**Why**: `roko.toml` has duplicate model keys (`kimi-k25` and `kimi-k2-5` both → slug `"kimi-k2.5"`) and invalid slugs (`kimi-k2` → API returns 404). No validation catches these at load time. Users discover the issue only when a model call fails.

**What**: Add validation to the unified config loader:

```rust
fn validate_models(config: &RokoConfig) -> Vec<ConfigWarning> {
    let mut warnings = vec![];

    // Check for duplicate slugs
    let mut slug_to_keys: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, profile) in &config.models {
        slug_to_keys.entry(&profile.slug).or_default().push(key);
    }
    for (slug, keys) in &slug_to_keys {
        if keys.len() > 1 {
            warnings.push(ConfigWarning::DuplicateSlug {
                slug: slug.to_string(),
                keys: keys.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    // Check model references valid provider
    for (key, profile) in &config.models {
        if !config.providers.contains_key(&profile.provider) {
            warnings.push(ConfigWarning::OrphanedModel {
                model: key.clone(),
                provider: profile.provider.clone(),
            });
        }
    }

    warnings
}
```

Warnings logged at `warn!` level on config load. Not errors — don't block startup, but surface the issue.

**Files**: `roko-core/src/config/loader.rs` (validation), `roko-core/src/config/mod.rs` (warning type).

### 10.5 Zed Settings Documentation and `--global-config` Flag

**Why**: Zed's `settings.json` passes `roko acp` with no flags. Users have no way to control config behavior from Zed settings.

**What**:

1. Add `--global-config` flag to `roko acp` subcommand:
```rust
/// Path to global config file (default: ~/.roko/config.toml)
#[arg(long)]
global_config: Option<PathBuf>,
```

2. Surface config state in ACP's `serverInfo` response so users can debug:
```json
{
    "name": "roko",
    "version": "0.1.0",
    "configSources": {
        "project": "/Users/will/dev/project/roko.toml",
        "global": "/Users/will/.roko/config.toml",
        "providers": ["anthropic", "openai", "gemini"],
        "models": 12
    }
}
```

3. Document recommended Zed settings:
```json
{
    "context_servers": {
        "roko": {
            "command": "roko",
            "args": ["acp", "--global-config", "~/.roko/config.toml"]
        }
    }
}
```

**Files**: `roko-cli/src/main.rs` (flag), `roko-acp/src/session.rs` (serverInfo), docs.

### 10.6 Wire Effort/Thinking to Dispatch

**Why**: The Thinking option (Quick/Standard/Deep/Max) is visible in the ACP status bar but is **pure theater** — `effort` is stored in `SessionConfigState` but never passed to any dispatch function. `ModelCallRequest` has no effort field. The user changes the dropdown, nothing changes in model behavior.

**What**:

1. Add `effort` to `ModelCallRequest`:
```rust
pub struct ModelCallRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub effort: Option<String>,   // NEW: "low", "medium", "high", "max"
    // ...
}
```

2. Pass from ACP dispatch (`bridge_events.rs:1527`):
```rust
ModelCallRequest {
    model: model_key.to_string(),
    effort: Some(session.config_state.effort.clone()),
    // ...
}
```

3. Map to provider-specific parameters in each backend:

| Provider | Effort field mapping |
|----------|---------------------|
| **Anthropic API** | `thinking.budget_tokens` (low=1024, medium=4096, high=16384, max=65536) |
| **OpenAI (o3/o4)** | `reasoning_effort` ("low"/"medium"/"high") |
| **Gemini** | `thinking_config.thinking_budget` (already wired in native backend) |
| **GLM/Kimi** | `thinking.type` = if effort≠"low" then "enabled" else skip |
| **All others** | Silently ignore (no-op) |

**Files**: `roko-core/src/foundation.rs` (ModelCallRequest), `roko-acp/src/bridge_events.rs` (pass effort), `roko-agent/src/provider/openai_compat.rs` (map to API params), `roko-agent/src/provider/anthropic_api/tool_loop.rs` (add thinking block).

### 10.7 Adaptive Config Options by Model Capability

**Why**: Currently all 6 config options are shown regardless of provider/model. This creates ~2,900 combinations where many are nonsensical (Perplexity + Workflow=Standard, Cerebras + Thinking=Max, disabled-gates workflow). Users see toggles that do nothing and get no feedback about why.

**What**: Replace flat option list with capability-aware dynamic options.

1. Add `ThinkingMode` to `ModelProfile`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    /// Model has no thinking support (Cerebras, Perplexity, GPT-4o, Llama)
    None,
    /// Model supports binary thinking toggle (GLM, Kimi)
    Binary,
    /// Model supports graduated effort levels (Claude, OpenAI o3/o4, Gemini)
    Leveled,
}

impl Default for ThinkingMode {
    fn default() -> Self { Self::None }
}
```

2. Populate from `supports_thinking` + provider hints:
```rust
fn resolve_thinking_mode(profile: &ModelProfile, provider: &ProviderConfig) -> ThinkingMode {
    if !profile.supports_thinking { return ThinkingMode::None; }
    match provider.kind.as_str() {
        "anthropic_api" | "claude_cli" => ThinkingMode::Leveled,
        "openai_compat" if is_reasoning_model(&profile.slug) => ThinkingMode::Leveled,
        "gemini" | "gemini_native" => ThinkingMode::Leveled,
        "openai_compat" if is_glm_or_kimi(profile) => ThinkingMode::Binary,
        _ => ThinkingMode::None,
    }
}
```

3. Make `build_config_options` adaptive:
```rust
fn build_config_options(state: &SessionConfigState, config: &RokoConfig) -> Vec<ConfigOption> {
    let profile = config.models.get(&state.model);
    let provider = config.providers.get(&state.provider);
    let mut options = vec![provider_option(state, config), model_option(state, config)];

    // Thinking: show only if model supports it, with appropriate granularity
    if let Some(p) = profile {
        match resolve_thinking_mode(p, provider.unwrap_or(&ProviderConfig::default())) {
            ThinkingMode::None => { /* omit */ }
            ThinkingMode::Binary => options.push(thinking_toggle(state)),     // On / Off
            ThinkingMode::Leveled => options.push(thinking_levels(state)),    // Quick..Max
        }

        // Workflow: only for tool-capable models
        if p.supports_tools {
            options.push(workflow_option(state));
            if state.workflow != "none" {
                options.push(clippy_option(state));
                options.push(tests_option(state));
            }
        }
    }

    options
}
```

4. On model change, reset incompatible options:
```rust
fn on_model_changed(state: &mut SessionConfigState, new_model: &str, config: &RokoConfig) {
    state.model = new_model.to_owned();
    let profile = config.models.get(new_model);
    if let Some(p) = profile {
        let mode = resolve_thinking_mode(p, ...);
        match mode {
            ThinkingMode::None => state.effort = "medium".to_owned(), // reset, won't be used
            ThinkingMode::Binary => {
                if state.effort != "off" { state.effort = "on".to_owned(); }
            }
            ThinkingMode::Leveled => { /* keep current effort level */ }
        }
        if !p.supports_tools && state.workflow != "none" {
            state.workflow = "none".to_owned(); // force reset
        }
    }
}
```

**Files**: `roko-core/src/config/agent.rs` (ThinkingMode), `roko-acp/src/session.rs` (build_config_options, on_model_changed).

### 10.8 Provider Health & Model Validation

**Why**: Users can select kimi-k2 (invalid slug → 404), providers with missing API keys (→ 401), or providers whose endpoints are down. Error messages are raw JSON blobs from upstream APIs.

**What**:

1. **Provider health status in options**: Add a health-check result per provider:
```rust
ConfigOptionValue {
    value: "moonshot".to_owned(),
    name: "Moonshot".to_owned(),
    description: Some(match provider_status {
        ProviderStatus::Ready => "Ready".to_owned(),
        ProviderStatus::NoApiKey => "API key not configured".to_owned(),
        ProviderStatus::Unreachable => "Endpoint unreachable".to_owned(),
    }),
}
```

Run a lightweight health check on ACP startup (HEAD request to each provider's base_url, cached for 5 minutes). Providers with `NoApiKey` or `Unreachable` status are still selectable but shown with warning text.

2. **Model slug pre-validation**: On model selection (config-update), do a best-effort validation:
```rust
fn validate_model_selection(model: &str, config: &RokoConfig) -> Option<String> {
    let profile = config.models.get(model)?;
    // Check for known-bad slugs
    if profile.slug == "kimi-k2" {
        return Some("Model slug 'kimi-k2' may not be recognized by Moonshot API. Try 'kimi-k2.5' or 'kimi-k2.6'.".into());
    }
    // Check for duplicate slug (different key, same slug)
    let dups: Vec<_> = config.models.iter()
        .filter(|(k, p)| *k != model && p.slug == profile.slug)
        .map(|(k, _)| k.as_str())
        .collect();
    if !dups.is_empty() {
        return Some(format!("Duplicate slug '{}': also defined as {}", profile.slug, dups.join(", ")));
    }
    None
}
```

3. **Human-readable error messages**: Wrap raw API errors at the dispatch boundary:
```rust
fn format_agent_error(err: &AgentError, model: &str, provider: &str) -> String {
    match err {
        AgentError::Network { status: 404, .. } =>
            format!("Model '{}' not found on {} API. Check the model slug in roko.toml.", model, provider),
        AgentError::Network { status: 401, .. } =>
            format!("{} API key is invalid or expired. Run `roko config providers health {}`.", provider, provider),
        AgentError::Network { status: 429, .. } =>
            format!("{} rate limit exceeded. Wait a moment and try again.", provider),
        AgentError::Timeout { .. } =>
            format!("{} '{}' timed out. The model may be slow or the endpoint may be down.", provider, model),
        _ => format!("{}: {}", provider, err),
    }
}
```

**Files**: `roko-acp/src/session.rs` (health status, validation), `roko-acp/src/bridge_events.rs` (error formatting).

### 10.9 Remove Dead Config Fields or Wire Them

**Why**: `SessionConfigState` has 4 fields that are stored but never used: `temperament`, `routing_mode`, `review_strictness`, and (partially) `max_iterations`. Dead config creates maintenance burden and confuses anyone reading the code.

**Decision per field**:

| Field | Current state | Recommendation |
|-------|--------------|----------------|
| `temperament` | Stored, never read by any dispatch | **Remove** — or wire into system prompt framing (cautious = more warnings, aggressive = fewer guardrails). If keeping, add to config UI and actually use it. |
| `routing_mode` | Stored, never consulted by CascadeRouter | **Remove** — or wire into cascade_router.observe() to respect manual/auto distinction. Low value until CascadeRouter is more mature. |
| `review_strictness` | Stored in PipelineConfig, never read in review phase | **Wire** — pass to review agent system prompt: quick = "check for obvious errors only", thorough = "line-by-line review with suggestions" |
| `max_iterations` | Stored, passed to PipelineConfig, used as loop limit | **Keep and expose in UI** — it works, just hidden. Add to config options when workflow ≠ none. |

**Preferred approach**: Remove `temperament` and `routing_mode` for now (YAGNI — they can be added when actually needed). Wire `review_strictness` into the review agent prompt. Expose `max_iterations` in the UI.

**Files**: `roko-acp/src/session.rs` (remove/wire fields), `roko-acp/src/pipeline.rs` (review strictness), `roko-acp/src/runner.rs` (review prompt).

---

## Phase 11: First-Run Experience & CLI UX (2026-05-04)

*No hard dependencies. Can proceed in parallel with other phases. Directly reduces user friction.*

### 11.1 Refuse to Chat Without Valid Provider

**Why**: Running `roko` without config silently falls back to `cat` agent — user thinks roko is broken.

**What**: In `cmd_unified_chat()` (unified.rs), after config load, validate that at least one provider is available. If not:
```
No LLM providers configured.

Quick start:
  1. Install Claude CLI:  npm install -g @anthropic-ai/claude-cli && claude login
  2. Or set API key:      export ANTHROPIC_API_KEY=sk-ant-...
  3. Then initialize:     roko init

See `roko config providers available` for all options.
```
Exit with code 1. Never enter the REPL with no valid provider.

**Files**: `unified.rs`, `auth_detect.rs`.
**Effort**: Low.

### 11.2 Provider Validation at End of `roko init`

**Why**: `roko init` generates config but never verifies it works.

**What**: After writing `roko.toml`, run a quick check:
```rust
// At end of cmd_init():
let config = load_config_unified(&target)?;
let providers = check_provider_availability(&config);
for p in &providers {
    match p.status {
        Ready => println!("  ✓ {} — ready", p.name),
        NoBinary => println!("  ✗ {} — binary not found on PATH", p.name),
        NoApiKey => println!("  ✗ {} — API key not set (set {})", p.name, p.env_var),
    }
}
if providers.iter().all(|p| !p.ready()) {
    println!("\n⚠ No providers are ready. Set an API key or install a CLI before using roko.");
}
```

**Files**: `commands/init.rs`, new `provider_check.rs` module.
**Effort**: Low.

### 11.3 `roko config providers add` Command

**Why**: Adding a provider requires manual TOML editing with no guidance.

**What**: Interactive command:
```bash
$ roko config providers add anthropic
Provider: Anthropic API
API key env var: ANTHROPIC_API_KEY

Enter API key (or press Enter to set later): sk-ant-...

Added to roko.toml:
  [providers.anthropic]
  kind = "anthropic_api"
  api_key_env = "ANTHROPIC_API_KEY"

  [models.claude-sonnet-4-6]
  provider = "anthropic"
  slug = "claude-sonnet-4-6"

Testing connection... ✓ Model responds (238ms)
```

Built-in templates for each `ProviderKind`: Anthropic, OpenAI, Gemini, Ollama, Perplexity, Cerebras, Moonshot.

**Files**: New `commands/config_providers.rs` handler, provider templates.
**Effort**: Medium.

### 11.4 `roko config providers available`

**Why**: Users don't know what providers roko supports or how to configure them.

**What**: List all `ProviderKind` variants with setup info:
```
Available providers:

  anthropic       Anthropic API (Claude)
                  Env: ANTHROPIC_API_KEY
                  Add: roko config providers add anthropic

  claude_cli      Claude CLI (local)
                  Install: npm i -g @anthropic-ai/claude-cli
                  Add: roko config providers add claude_cli

  openai_compat   OpenAI-compatible API
                  Env: OPENAI_API_KEY
                  Works with: OpenAI, Azure, local servers
                  Add: roko config providers add openai

  ollama          Ollama (local models)
                  Install: https://ollama.com
                  Add: roko config providers add ollama
  ...
```

**Files**: New subcommand in config providers, template data.
**Effort**: Low.

### 11.5 Plan Run Progress Output

**Why**: `roko plan run` runs for minutes/hours with no CLI feedback.

**What**: Print task-level progress to stderr:
```
[1/15] Running "scaffold-types" (claude-sonnet-4-6)... done (12s, $0.03)
[2/15] Running "implement-auth" (claude-sonnet-4-6)...
       ├─ turn 1: 3 tool calls (read_file ×2, grep)
       └─ turn 2: 2 tool calls (write_file, edit_file)
       done (45s, $0.18)
[3/15] Running "add-tests" (claude-sonnet-4-6)...
```

Use `indicatif` or simple `eprint!` with `\r` for in-place updates. Suppress with `--quiet`.

**Files**: `orchestrate.rs` task dispatch callbacks, new progress formatter.
**Effort**: Medium.

### 11.6 `roko status --quick`

**Why**: Full `roko status` output is 50+ lines. Users want a health check.

**What**:
```
$ roko status --quick
Plan: auth-refactor  8/12 tasks done, 2 running
Cost: $1.47 today ($24.99 budget remaining)
Providers: 3 ok, 1 unhealthy (moonshot: rate limited)
```

3 lines. Exit 0 if healthy, 1 if problems.

**Files**: `commands/util.rs` (status handler), add `--quick` flag.
**Effort**: Low.

### 11.7 Doctor Offers Fixes

**Why**: `roko doctor` diagnoses but doesn't help.

**What**: For each fixable issue, print the fix command:
```
[fail] config: no roko.toml found
  → fix: roko init

[warn] provider: claude CLI not found on PATH
  → fix: npm install -g @anthropic-ai/claude-cli

[warn] provider: ANTHROPIC_API_KEY not set
  → fix: export ANTHROPIC_API_KEY=sk-ant-...
```

**Files**: `doctor.rs` (add fix suggestions per check).
**Effort**: Low.

### 11.8 Starter Template & .env.example

**Why**: The 22KB `roko.toml` is overwhelming. No `.env.example` exists.

**What**: Create `examples/roko.toml.minimal`:
```toml
# Minimal roko config — one provider, one model
[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[models.claude-sonnet]
provider = "claude_cli"
slug = "claude-sonnet-4-6"

[agent]
default_model = "claude-sonnet"
```

Create `examples/.env.example`:
```bash
# Required for API providers (pick one):
# ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# ZAI_API_KEY=...
# GEMINI_API_KEY=...
```

**Files**: New `examples/` directory.
**Effort**: Low.

---

## Phase 12: Error Quality & Observability (2026-05-04)

*Ongoing work. Can be done incrementally per-crate. No dependencies.*

### 12.1 Audit and Fix Silent Error Swallowing

**Why**: 120+ instances of `let _ =` and `.ok()` in production code cause errors to vanish silently. Most impactful: 60 silent channel sends in ACP bridge_events.rs, agent registration in relay.rs, process kill in kill.rs.

**What**: Per-crate audit. For each `let _ =`:
1. If it's a cleanup operation (e.g., `drop`, `close`), add a comment: `// deliberate: cleanup is best-effort`
2. If it's a side-effect (logging, sending, writing), replace with `tracing::warn!` or `?`
3. If it's a core operation (state mutation, registration), propagate the error

Start with the highest-impact crates: `roko-acp`, `roko-serve`, `roko-agent`.

**Effort**: High (ongoing, 2-3 hours per crate).

### 12.2 Replace Unwrap/Expect with Context

**Why**: 150+ `unwrap()` / `expect("generic")` calls produce unhelpful panics.

**What**: Per-crate migration. Replace:
```rust
// Before:
self.jobs.get_mut(job_id).unwrap()
// After:
self.jobs.get_mut(job_id).ok_or_else(|| anyhow!("job {job_id} not found in marketplace"))?
```

Start with code that's most likely to panic at runtime: marketplace, MCP scripts, distiller.

**Effort**: High (ongoing, 1-2 hours per crate).

### 12.3 Fix Panic in Display Impl

**Why**: `roko-core/src/error/mod.rs:652-685` has `panic!("wrong variant")` inside a `Display` implementation. Formatting an error can itself panic.

**What**: Replace with a safe fallback:
```rust
_ => write!(f, "<unknown error variant>")
```

**Effort**: Low (< 15 min).

### 12.4 Typed Error Enums for Key Subsystems

**Why**: `SessionError` is excellent — typed variants with actionable `#[error(...)]` messages. Most other subsystems use generic `anyhow!`.

**What**: Create typed error enums for:
1. `ProviderError` — spawn failures, auth errors, rate limits, timeouts
2. `PlanError` — task failures, gate failures, budget exhaustion
3. `ConfigError` — parsing failures, validation failures, missing fields
4. `GateError` — compilation errors, test failures, clippy warnings

Each variant should include: what happened, what the user should do, and relevant context (file paths, provider names, model slugs).

**Effort**: High (1-2 days across subsystems).

---

## Phase 13: Developer Experience (2026-05-04)

*Reduces friction for contributors and CI. No dependencies.*

### 13.1 Remove Clippy Blanket Suppression

**Why**: `main.rs` has `#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]` — effectively disabling clippy for the largest file.

**What**: Remove the blanket allow. Fix resulting warnings incrementally:
1. First pass: fix `clippy::all` warnings (usually straightforward)
2. Second pass: selectively re-allow `pedantic` / `nursery` lints that are genuinely noisy (e.g., `too_many_lines` for the match dispatch)
3. Add targeted `#[allow(...)]` only where justified with a comment

**Effort**: Medium (2-3 hours for first pass).

### 13.2 Feature-Gate Phase 2+ Dead Code

**Why**: ~70 `#[allow(dead_code)]` instances, mostly in chain/audit/effects scaffolding.

**What**: Put behind `#[cfg(feature = "chain")]`, `#[cfg(feature = "effects")]` features. Default off. Remove `allow(dead_code)`.

**Effort**: Low-Medium.

### 13.3 Add rust-toolchain.toml

**Why**: Workspace requires 1.91+ but doesn't lock it locally. Developers with older rustc get cryptic errors.

**What**:
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

This ensures `rustup` auto-installs the right version.

**Effort**: Low (< 5 min).

### 13.4 Multi-Process File Locking

**Why**: Two `roko` processes on the same `.roko/` directory corrupt state silently.

**What**: Advisory lock via `fs2::FileExt::lock_exclusive()` on `.roko/runtime/roko.lock`:

```rust
use fs2::FileExt;

fn acquire_workspace_lock(roko_dir: &Path) -> Result<std::fs::File> {
    let lock_path = roko_dir.join("runtime/roko.lock");
    std::fs::create_dir_all(lock_path.parent().unwrap())?;
    let file = std::fs::OpenOptions::new()
        .create(true).write(true).open(&lock_path)?;

    match file.try_lock_exclusive() {
        Ok(()) => {
            // Write PID for diagnostics
            use std::io::Write;
            writeln!(&file, "{}", std::process::id())?;
            Ok(file)
        }
        Err(_) => {
            let pid = std::fs::read_to_string(&lock_path)
                .unwrap_or_else(|_| "unknown".into());
            bail!("Another roko process is running (PID {}). \
                   Kill it first or use a different workspace.", pid.trim());
        }
    }
}
```

Lock released automatically when `File` is dropped.

**Files**: New `roko-core/src/lock.rs`, called from boot sequence.
**Effort**: Low.

---

## Phase 14 — CLI Output Redesign (Spinners, Colors, Progress)

**Why**: Current CLI output is a stream of unformatted tracing logs. No spinners, no colors, no icons. Users can't tell what's happening, when something fails, or what to do next. Every modern CLI tool (Claude CLI, Codex, gh, cargo) uses semantic output with progressive disclosure.

**Principle**: The CLI should be **quiet by default** with progressive detail on demand. User-facing output and developer tracing are **separate systems** that never mix.

### 14.1 Output Architecture: `CliReporter` Trait

**What**: Introduce a `CliReporter` trait that all CLI commands use instead of direct `println!`/`eprintln!`/`tracing::info!`. Implementations handle formatting based on output mode (TTY, pipe, JSON).

```rust
// roko-cli/src/reporter.rs

pub trait CliReporter: Send + Sync {
    fn status(&self, icon: Icon, message: &str);
    fn progress(&self, spinner: &SpinnerHandle, message: &str);
    fn success(&self, message: &str);
    fn error(&self, message: &str, help: Option<&str>);
    fn warning(&self, message: &str);
    fn table(&self, headers: &[&str], rows: &[Vec<String>]);
    fn summary(&self, items: &[(&str, String)]);
}

pub enum Icon {
    Check,    // ✓
    Cross,    // ✗
    Arrow,    // →
    Spinner,  // ⠋ (animated)
    Diamond,  // ⟐
    Info,     // ℹ
}
```

Three implementations:
- `TtyReporter` — colors, spinners, unicode icons (when stdout is a terminal)
- `PlainReporter` — no colors, no spinners, ASCII fallbacks (piped output / CI)
- `JsonReporter` — structured JSON lines (for machine consumption, `--output json`)

Auto-selected based on `atty::is(Stream::Stdout)` at startup.

**Files**: New `roko-cli/src/reporter.rs`, new `roko-cli/src/reporter/tty.rs`, `plain.rs`, `json.rs`.
**Effort**: Medium (core infra, then incremental command migration).

### 14.2 Tracing Separation: File-Only by Default

**What**: Remove `tracing_subscriber::fmt().with_writer(std::io::stderr)` from CLI entry points. Replace with file-based subscriber that writes to `.roko/roko.log`. Only enable stderr tracing when `--verbose` flag is set or `RUST_LOG` env var is present.

```rust
// Boot sequence (RokoBootstrap):
fn setup_tracing(verbose: bool, log_file: &Path) {
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(File::create(log_file).unwrap_or_else(|_| /* /dev/null */))
        .with_ansi(false);

    let stderr_layer = if verbose || std::env::var("RUST_LOG").is_ok() {
        Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .init();
}
```

**Effect**: `roko prd plan slug` now outputs ONLY the plan result, not 50 lines of INFO/WARN.

**Files**: `roko-cli/src/main.rs` (tracing init), `roko-cli/src/bootstrap.rs`.
**Effort**: Low.

### 14.3 Spinner Integration with `indicatif`

**What**: Use the `indicatif` crate for all operations that take >1 second. Spinners are started before the operation and finished with success/failure on completion.

```rust
use indicatif::{ProgressBar, ProgressStyle};

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}  ({elapsed})")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// Usage in plan run:
let spinner = create_spinner(&format!("Task {}/{}: {}", i+1, total, task.name));
let result = dispatch_agent(task).await;
spinner.finish_with_message(match result {
    Ok(_) => format!("✓ {} completed", task.name),
    Err(e) => format!("✗ {} failed: {}", task.name, e),
});
```

**Deps**: Add `indicatif = "0.17"` to `roko-cli/Cargo.toml`.
**Files**: Modify `orchestrate.rs`, `chat_inline.rs`, `prd.rs`, `plan.rs`.
**Effort**: Low-Medium (mechanical wrapping of existing operations).

### 14.4 Color System with `owo-colors`

**What**: Semantic colors applied consistently across all output:

| Semantic | Color | Usage |
|----------|-------|-------|
| Success | Green | ✓ completed, gates passed |
| Error | Red | ✗ failed, error messages |
| Warning | Yellow | ⚠ deprecated, degraded |
| Info | Cyan | Task names, spinner text |
| Dim | Gray | Metadata (cost, time, IDs) |
| Bold | White bold | Section headers, summaries |

Use `owo-colors` (zero-alloc, respects `NO_COLOR` env var automatically):

```rust
use owo_colors::OwoColorize;

println!("{} {}", "✓".green(), "Plan complete".bold());
println!("  {} tasks, {}, {}",
    "2/2".cyan(),
    "$0.68".dimmed(),
    "6m 5s".dimmed(),
);
```

**Deps**: Add `owo-colors = "4"` to `roko-cli/Cargo.toml`.
**Files**: All CLI output paths (incremental migration).
**Effort**: Low (incremental).

### 14.5 Plan Run Progress: `MultiProgress` Bar

**What**: During `plan run`, show a live-updating multi-task view using `indicatif::MultiProgress`:

```rust
let multi = MultiProgress::new();

// Header
let header = multi.add(ProgressBar::new(0));
header.set_style(ProgressStyle::default_bar().template("⟐ Running plan: {msg}").unwrap());
header.set_message(format!("{} ({} tasks)", plan_name, task_count));

// Per-task progress
for (i, task) in tasks.iter().enumerate() {
    let pb = multi.add(ProgressBar::new_spinner());
    pb.set_prefix(format!("  [{}/{}]", i+1, task_count));
    // ... update as task progresses
}

// Summary
let summary = multi.add(ProgressBar::new(0));
summary.finish_with_message(format!(
    "✓ Plan complete: {}/{} tasks, ${:.2}, {}",
    done, total, cost, format_duration(elapsed)
));
```

**Files**: `roko-cli/src/orchestrate.rs` (main plan runner loop).
**Effort**: Medium.

### 14.6 Error Deduplication

**What**: Remove all `eprintln!("error: ...")` / `eprintln!("Error: ...")` in command handlers. Implement a single top-level error reporter in `main.rs`:

```rust
fn report_error(err: &anyhow::Error) {
    eprintln!("{} {}", "error:".red().bold(), err);
    // Print cause chain (if any) indented
    for cause in err.chain().skip(1) {
        eprintln!("  {} {}", "caused by:".dimmed(), cause);
    }
    // Print help text if the error type provides it
    if let Some(help) = err.downcast_ref::<HelpfulError>() {
        eprintln!("\n  {} {}", "help:".yellow(), help.suggestion());
    }
}
```

**Files**: `roko-cli/src/main.rs`, remove `eprintln!` from all `cmd_*.rs` files.
**Effort**: Low (mechanical removal).

### 14.7 Command-Specific Output Improvements

Quick wins per command:

| Command | Current | Target |
|---------|---------|--------|
| `roko init` | 2 plain lines | ✓ icons + "Next steps:" section |
| `roko status` | 50+ dump lines | 3-line summary (expand with `--full`) |
| `roko plan list` | Directory names | Table: name, tasks, progress, date |
| `roko plan run` | Tracing logs | Multi-progress with per-task spinners |
| `roko prd plan` | Silent on failure | Error if no tasks extracted + suggestion |
| `roko doctor` | Diagnostic-only | Each fail gets "fix:" suggestion line |
| `roko config show` | Raw TOML dump | Syntax-highlighted + section headers |

**Files**: Each command's handler file.
**Effort**: Medium (one command at a time).

### 14.8 `--output` Flag for Machine Consumption

**What**: Add `--output` / `-o` global flag: `text` (default), `json`, `plain`.

```rust
#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Text,   // TTY-aware with colors/spinners
    Plain,  // No colors, no spinners (for CI/pipes)
    Json,   // Structured JSON lines
}
```

Automatically set to `plain` when stdout is not a TTY (piped to file/grep).

**Files**: `roko-cli/src/main.rs` (global arg), `reporter.rs` (selection logic).
**Effort**: Low.

### Implementation Order

1. **14.2** (tracing separation) — immediate noise reduction, no new deps
2. **14.1** (CliReporter trait) — establish the pattern
3. **14.3** (indicatif spinners) — biggest UX impact for least code
4. **14.4** (owo-colors) — can be done incrementally per-command
5. **14.6** (error dedup) — mechanical cleanup
6. **14.5** (plan run multi-progress) — the showcase feature
7. **14.7** (per-command polish) — ongoing refinement
8. **14.8** (--output flag) — nice-to-have, enables scripting
