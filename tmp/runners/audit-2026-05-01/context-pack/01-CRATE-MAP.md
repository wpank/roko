# Crate Map (audit-2026-05-01 subset)

This is the subset of the crate graph touched by this runner's batches.
Read it before guessing where a symbol lives.

## roko-core (foundation)

`crates/roko-core/src/`
- `config/schema.rs` — `RokoConfig` and all sub-config structs
  (`AgentConfig`, `ConductorConfig`, `ServeConfig`, ...). Phantom config
  sections (T2-18) and phantom `ConductorConfig` fields (T2-19) live here.
- `config/agent.rs` — `AgentConfig` (`default_model`, `effort`, ...).
  `policy_manifests` (line 70), `domain` (line 90), `data_llm` (line 72)
  are the T2-21 candidates.
- `config/budget.rs` — `EnergyConfig` (T2-18 candidate).
- `config/learning.rs` — `DemurrageConfig`, `AttentionConfig`, `ImmuneConfig`,
  `TemporalConfig`, `GoalsConfig` (all T2-18 candidates).
- `config/tools.rs` — `OneirographyConfig` (T2-18 candidate).
- `config/mod.rs` — `load_config`. Strict validator wired via
  `validate_strict_config_toml` (T1-12 done).
- `config/validation.rs` — strict validator. T5-38 / S-config plans
  extend this with semantic validators.
- `config/provenance.rs` — `ResolvedConfig`/`ValidatedConfig` skeletons.
  Plan 23 / S-config builds these out.
- `config/hot_reload.rs` — diffing for hot reload. T2-18 deletions also
  remove diff arms here.
- `config/compat.rs` — config-format migrations. T2-19 deletions also
  remove migration arms here.
- `usage.rs` — `UsageObservation` with `Option<u64>` fields. The
  canonical telemetry type for T4-31.
- `dispatch_plan.rs` — `DispatchPlan` (skeleton).
- `foundation.rs` — shared traits (`AgentContract`, etc.).

## roko-runtime

`crates/roko-runtime/src/`
- `run_ledger.rs` — `RunLedger` skeleton (R3 done). `RunLedgerEntry` is
  the place to add typed `Gate`, `Artifact`, `Command`, `Checkpoint`
  variants for plan 24 / S-ledger.
- `effect_driver.rs`, `workflow_engine.rs` — workflow orchestration. The
  report builder reads typed entries from `RunLedger` after migration.
- `pipeline_state.rs` — pipeline state machine.

## roko-agent (LLM dispatch)

`crates/roko-agent/src/`
- `model_call_service.rs` — the **canonical** LLM dispatch surface.
  `ModelCallService::call`, `ModelCallService::stream`. Plan 22 / T5-36
  migrate route-local HTTP through here.
- `dispatch_resolver.rs` — selects + validates a `DispatchPlan`. Today
  returns `Unvalidated` diagnostics (S-acp1 / plan 21 ACP-1 fixes).
- `safety/mod.rs` — `SafetyLayer`, `AgentContract`. T1-15 made the
  default `restricted("default")`. Plan 28 wires recovery actions and
  10-stage pipeline hardening.
- `providers/anthropic*.rs` — Anthropic provider adapter. T4-31a
  migrates the usage parser to `UsageObservation`.
- `providers/ollama*.rs` — Ollama adapter. T4-31b.
- `providers/gemini*.rs` — Gemini adapter. T4-31c.
- `providers/cerebras*.rs` — OpenAI-compatible. T4-31d.
- `providers/cursor*.rs` — Cursor proxy (OpenAI-compatible). T4-31e.
- `translate/openai.rs` — already preserves `UsageObservation`. Reference
  template for T4-31.

## roko-cli (the largest crate)

`crates/roko-cli/src/`
- `orchestrate.rs` — **22,756 lines, the god file.** Contains
  `dispatch_agent_with` (~2,059 lines starting at line 14575). T5-35
  extracts it into 4 units. Many helpers live here that other plans
  reference.
- `chat_inline.rs` — chat REPL state machine (~4.1K lines). T4-34
  makes `/model` switch atomic. The slash-command palette lives here.
- `dispatch_direct.rs` — legacy direct-dispatch path. Feature-gated and
  removed by T5-37.
- `runner/event_loop.rs` — runner v2 event loop.
- `runner/types.rs` — `RunnerEvent` / `RunState`.
- `runtime_feedback/` — sinks (`EpisodeSink`, `RoutingObservationSink`,
  `KnowledgeIngestionSink`, conductor + dreams).
  - `episodes.rs`, `knowledge.rs`, `routing.rs` — keep.
  - `conductor.rs`, `dreams.rs` — delete (T2-20).
  - `mod.rs` — module declarations + facade.
- `commands/plan.rs` — plan-run command. Constructs the `FeedbackFacade`
  at line ~378-396; T2-20, T4-29 edit here.
- `commands/config_cmd.rs` — `roko config doctor` etc.
- `unified.rs` — single-prompt CLI path. Some legacy `dispatch_direct`
  callers (T5-37 migrates).

## roko-acp

`crates/roko-acp/src/`
- `session.rs` — `AcpSession` with conversation history (40 turns / 64K
  chars), config state, approval flow, workflow integration.
- `bridge_events.rs` — JSON-RPC frame handling.
- `types.rs` — protocol DTOs (`SessionUpdate`, `ContentBlock`, etc.).

## roko-serve (HTTP/WS API)

`crates/roko-serve/src/`
- `lib.rs` — server bootstrap. `validate_bind_safety` (line 641),
  `build_server_router` (line 705), `build_app_state` (line 780, includes
  the auto-enable-auth-on-public-bind logic).
- `routes/mod.rs` — router assembly. `build_router` (line ~80),
  middleware layering. T3-23 (rate limit), T3-24 (body limit), T3-26 (WS)
  all touch this file.
- `routes/middleware.rs` — `cors_layer` (line 432; T3-28 restricts),
  `require_api_key`, `require_scope`, `scrub_secrets`.
- `routes/config.rs` — config get/put/reload, `mask_secret_fields`
  (T0-2 done).
- `routes/agents.rs` — agent CRUD. `create_agent` (line 605, T3-27
  fixes). `validate_agent_url` (line 1635, T0-5 done).
- `routes/shared_runs.rs` — shared run links (T0-3 done).
- `routes/webhooks.rs` — public webhooks split (T0-4 done).
- `routes/inference.rs` — inference endpoint, T5-36 candidate.
- `routes/research.rs` — research/Sonar endpoint, T5-36 candidate.
- `routes/providers.rs` — provider test (D9 done; verify).
- `command_events.rs` — typed `CommandEvent` DTOs (`Started`, `Output`,
  `Exited`, `SpawnFailed`, `Cancelled`). Plan 26 / T5-41 wires the
  `/events` endpoint and demo consumer.
- `terminal/` — PTY terminal session machinery. T3-26, plan 26 § Phase 5
  hardening.

## roko-gate

`crates/roko-gate/src/`
- `lib.rs` — `Rung` enum, gate trait, registry.
- `registry.rs` — `GateRegistry` (R6 done). One duplicate map removed
  (R7).
- `adaptive_threshold.rs` — `observe`, `observe_pipeline`,
  `drain_spc_alerts` (T1-14 wired).
- (Plan 29 adds: `symbol_gate.rs`, `property_test_gate.rs`,
  `integration_gate.rs`.)

## roko-learn

`crates/roko-learn/src/`
- `lib.rs` — module list. **The 14 unused modules T2-17 deletes**:
  `adversarial`, `adas`, `calibration_policy`, `causal`,
  `reinforce_kind`, `research_pipeline`, `regression`, `bandit_research`,
  `forensic_replay`, `drift`, `local_reward`, `section_outcome`,
  `post_gate_reflection`, `verdict_scorer`.
- **The 4 orphan files T2-16 deletes**: `resonant_patterns.rs`,
  `signal_metabolism.rs`, `shapley.rs`, `kalman.rs` (not in lib.rs at
  all).
- `cascade_router.rs` — model-selection learner. `record_confidence_outcome`
  and `observe_multi_objective`. Plan 25 § Phase A surfaces stage state.
- `contextual_bandit.rs` — implementation kept after T1-13 (no longer
  wired from runner).
- `playbook.rs` — playbook store. Plan 30 / T4-32 reads from here.
- `event_subscriber.rs`, `feedback_service.rs` — partial overlap with
  `roko-cli/runtime_feedback/`. Plan 25 § Phase E resolves.

## roko-neuro

`crates/roko-neuro/src/`
- `admission.rs` — `KnowledgeStore`, `KnowledgeAdmissionInput`.
  `DEFAULT_KNOWLEDGE_CANDIDATES_FILE = "knowledge-candidates.jsonl"`
  (line 26). Plan 14 / T4-29's ingestor adapter goes through this.
- `context.rs` — context retrieval helpers.

## roko-prompt

`crates/roko-prompt/src/`
- `lib.rs` — `SystemPromptBuilder` (the 9-layer assembly).
- (Layer files: identity, capability, role, task, context_layer (HDC),
  playbook_layer, scratch, system, hint.)

## roko-codeintel

`crates/roko-codeintel/src/`
- `index.rs` — symbol graph + HDC fingerprints.
- (Plan 33: persistent index, incremental updates.)

## demo/demo-app (frontend)

`demo/demo-app/src/`
- `lib/scenario-runners/*.ts` — the regex-prompt-scraping consumers.
  Plan 26 / T5-41 migrates to typed `CommandEvent` over a SSE/WS
  endpoint.
- `lib/terminal-session.ts`, `hooks/useTerminal.ts` — terminal client
  helpers. Plan 26 Phase 4 updates.
- `lib/serve-url.ts` — base URL helper.
- `vite.config.ts` — Vite config; proxies to roko-serve.

## scripts

`scripts/`
- `roko-fitness-checks.sh` — inventory script. Plan 27 / S-ci-1 makes
  it a no-new-violations gate.
- `docs-status-check.sh` — docs status inventory. Plan 27 § Phase 6
  promotes similarly.
- `fitness/allowlist.toml` — **does not yet exist**. Plan 27 § Phase 1
  creates.

## Where things WERE before recent commits

These line ranges were verified 2026-05-01 against the worktree. If
your batch's line numbers are off by more than 50 lines, recompute:

| Symbol | File | Line |
|---|---|---|
| `dispatch_agent_with` | `roko-cli/src/orchestrate.rs` | 14575 |
| `selected_gate_steps` | `roko-cli/src/orchestrate.rs` | 17240 |
| `gate_rung_caps` | `roko-cli/src/orchestrate.rs` | 17209 |
| `runner_event_to_feedback` | `roko-cli/src/runner/event_loop.rs` | 1339 |
| `RunnerEvent::TaskAttemptCompleted` (variant) | `roko-cli/src/runner/types.rs` | 596 |
| `mask_secret_fields` | `roko-serve/src/routes/config.rs` | 290 |
| `validate_agent_url` | `roko-serve/src/routes/agents.rs` | 1635 |
| `create_agent` | `roko-serve/src/routes/agents.rs` | 605 |
| `cors_layer` | `roko-serve/src/routes/middleware.rs` | 432 |
| `validate_bind_safety` | `roko-serve/src/lib.rs` | 641 |
| `build_app_state` (auto-enable auth) | `roko-serve/src/lib.rs` | 780 |
| `KnowledgeIngestionSink` (`with_ingestor`) | `roko-cli/src/runtime_feedback/knowledge.rs` | 88 |
| `RoutingObservationSink::on_event` | `roko-cli/src/runtime_feedback/routing.rs` | 61 |
| `FeedbackFacade::new()`+sinks | `roko-cli/src/commands/plan.rs` | 378 |
| `SafetyLayer::with_defaults` | `roko-agent/src/safety/mod.rs` | 246 |
| `contract_for_role` | `roko-agent/src/safety/mod.rs` | 866 |
| `validate_strict_config_toml` (call) | `roko-core/src/config/mod.rs` | 119 |

If your prompt's line number doesn't match: re-grep for the symbol,
update the line number in the prompt log, and continue. Don't fail.
