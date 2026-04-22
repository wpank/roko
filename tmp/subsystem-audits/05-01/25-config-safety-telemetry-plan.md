# 25 - Config, Safety, Telemetry Convergence Plan

Sources: 09, 10, 11, 13, 14, 15, 17, 18, 20, 22 plus current code in
`roko.toml`, `roko-core::{agent,config}`, `roko-cli::{config,model_selection,runtime_feedback}`,
`roko-agent::{provider,model_call_service,usage,safety}`, and `roko-learn::{cascade_router,model_router,runtime_feedback,efficiency}`.

## Target Contract

Runtime dispatch must flow through one typed contract:

`RawConfig -> MigratedConfig -> ValidatedConfig -> ResolvedRuntimeConfig -> RoutingContext -> DispatchPlan -> ModelCallResponse { usage: UsageObservation } -> LearningObservation`.

Production execution is invalid if any of these are missing:

- a validated config model;
- a non-dangerous permission policy, or a local-only dangerous override with reason and expiry;
- provider/model identity and provenance;
- a dispatch result feedback record;
- a safety contract for the role;
- usage represented as known, unknown, or estimated, never silently coerced to zero.

## Module Ownership

- `crates/roko-core/src/config/*`: single versioned domain schema, migration, validation, config provenance, provider/model ids.
- `crates/roko-cli/src/config.rs`: temporary legacy parse/overlay adapter only; no independent runtime schema after migration.
- `crates/roko-core/src/agent.rs`: model/provider identity types only; remove backend-prefix dispatch authority.
- `crates/roko-cli/src/model_selection.rs`: replace display-only `EffectiveModelSelection` with resolver client returning `DispatchPlan`.
- `crates/roko-agent/src/provider/*`: provider adapters and transport execution only; no config synthesis.
- `crates/roko-agent/src/model_call_service.rs`: consume immutable `ResolvedRuntimeConfig` and `DispatchPlan`; emit telemetry.
- `crates/roko-agent/src/safety/*`, `crates/roko-runtime/src/process.rs`: fail-closed safety defaults and budget enforcement.
- `crates/roko-cli/src/runtime_feedback/*`, `crates/roko-learn/src/*`: observation model, router updates, JSONL migrations.
- `crates/roko-acp`, `crates/roko-cli/src/chat_session.rs`, `crates/roko-serve`: thin dispatch consumers; no raw provider HTTP or auth lookup.

## New Core Types

Add these in `roko-core` unless a crate-local wrapper is explicitly noted:

- `ValidatedConfig { raw, migrated, diagnostics, provenance }`
- `ResolvedRuntimeConfig { providers, models, safety, budgets, routing, feedback_policy }`
- `ConfigProvenance { source: File | Migration | Default | Env | LocalOverride | CliOverride, path, key, reason }`
- `ProviderId(String)`, `ModelAlias(String)`, `BackendModelSlug(String)`
- `ProviderDefinition { id, display_name, kind, transport, auth, capabilities, provenance }`
- `ProviderTransport = Http { base_url } | Cli { command, args } | Acp { command, args } | Local`
- `ProviderAuth = EnvVar { name } | StaticSecretRef { name } | None { local_only: bool }`
- `ModelDefinition { alias, provider_id, backend_slug, capabilities, cost, metadata_source, provenance }`
- `ModelMetadataSource = Config | ProviderDiscovery | HealthProbe | Migration | BuiltInFallback`
- `PermissionPolicy { approvals, dangerous_skip }`
- `DangerousPermissionOverride { enabled, scope, reason, expires_at, ack_env, source }`
- `BudgetPolicy { per_call, per_turn, per_plan, per_tool, provenance }`
- `DispatchPlan { request, selected, provider, transport, auth_status, capabilities, budget, permissions, routing, trace }`
- `RoutingContextStatus = Available(RoutingContext) | Unavailable { reason }`
- `UsageObservation` moved to core with optional token/cost/duration fields and `UsageSource`.
- `LearningObservation = Contextual(ContextualObservation) | Confidence(ConfidenceObservation) | DashboardOnly(DashboardObservation)`

## Phase 0: Stop New Drift

1. Add CI-only recurrence checks before broad refactors:
   - reject `dangerously_skip_permissions = true` outside test fixtures and `.roko/local*.toml`;
   - reject raw `reqwest::Client::new()` in `roko-cli`, `roko-acp`, and `roko-serve` outside provider adapters;
   - reject `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `ZAI_API_KEY` reads outside config/auth/provider boundaries;
   - reject provider parsers that use `Usage::default()` or `unwrap_or(0)` for missing provider usage;
   - reject `RoutingContext::default()` in override learning paths;
   - reject `cascade_router.select(Vec::new())` and learned routing without `RoutingContextStatus::Available`.
2. Add `roko config validate --strict` and run it against the root `roko.toml`.
3. Require all production dispatch tests to assert `DispatchPlan.trace` and feedback emission.

## Phase 1: One Validated Config Model

1. Make `RokoConfig` in `roko-core/src/config/schema.rs` the only runtime domain model.
2. Change `crates/roko-cli/src/config.rs` into:
   - `CliRawConfigLayer` for legacy TOML/overlays;
   - migration helpers into `RokoConfig`;
   - no separate `Config`, `RunnerConfig`, provider, model, learning, or serve runtime semantics.
3. Replace `RokoConfig::from_toml` warning-only version handling with:
   - parse raw config;
   - migrate version 1 to current version;
   - emit migration diagnostics;
   - fail strict/prod loads when migration is required but not applied.
4. Pick canonical schema names:
   - keep `agent.default_model` as the v2 field;
   - migrate `agent.model` into `agent.default_model`;
   - warn in dev and fail in strict/prod after one compatibility window.
5. Remove runtime provider/model synthesis from `effective_providers`, `effective_models`, `provider/mod.rs`, and `model_call_service.rs`.
   Synthesis may exist only in migration/defaulting and must record provenance.
6. Validate provider variants:
   - `ClaudeCli` requires `transport = Cli` and no HTTP `base_url`/API auth;
   - `AnthropicApi`, `OpenAiCompat`, `PerplexityApi`, `GeminiApi`, `CerebrasApi` require HTTP transport and an auth policy;
   - `auth = None` is accepted only for explicit local transports or local HTTP with `local_only = true`;
   - a provider named `anthropic` may not secretly mean Claude CLI unless config says so and diagnostics display it.
7. Validate model links:
   - every model alias points to an existing provider id;
   - backend slugs are distinct from aliases;
   - stale provider-discovered slugs fail with actionable diagnostics;
   - provider override must name a default model or the caller must supply a model.

## Phase 2: Local-Only Dangerous Overrides

1. Default all permission bypass flags to false:
   - `CoreRunnerConfig::default_dangerously_skip_permissions`;
   - `ClaudeCliAgent::default`;
   - all PE_01 hardcoded `true` call sites;
   - root `roko.toml`.
2. Remove `dangerously_skip_permissions` from shared project config as an ordinary field.
3. Add ignored local override files:
   - `.roko/local-overrides.toml`;
   - `.roko/local-safety.toml`.
4. Accept dangerous skip only when all are true:
   - source path is local override, not repository config;
   - `reason` is non-empty;
   - `expires_at` is present and in the future;
   - `scope` names command, role, or task id;
   - `ROKO_ACK_DANGEROUS_PERMISSIONS=1` is set;
   - runtime mode is not production.
5. Thread `PermissionPolicy` into `DispatchPlan`, `AgentOptions`, `dispatch_v2`, runner, ACP, serve, and chat.
6. In production mode, missing safety contract, feedback sink, or permission policy is a hard error.

## Phase 3: Provider Identity, Transport, Auth, and Dispatch Provenance

1. Introduce `DispatchResolver` in core/agent boundary:
   - inputs: `ResolvedRuntimeConfig`, caller requirements, role/task, CLI overrides, optional `RoutingContext`;
   - output: `DispatchPlan`;
   - errors: ambiguous provider, unsupported transport, auth unavailable, capability missing, budget denied.
2. Replace `EffectiveModelSelection` with `DispatchPlan.trace`.
3. Keep identities separate:
   - provider id: config registry key;
   - provider kind: protocol family;
   - transport: HTTP/CLI/ACP/local mechanics;
   - auth: env/static/local-none policy;
   - model alias: local stable key;
   - backend slug: provider wire id;
   - attempted/final model: actual provider result.
4. Add provider-owned streaming to `ModelCaller`/provider adapters.
5. Remove ACP/chat/raw dispatch provider ownership:
   - delete `roko-acp` raw Anthropic streaming client after streaming adapter lands;
   - delete or quarantine `dispatch_direct.rs` behind a dev diagnostic flag;
   - replace API HTTP construction in `chat_session.rs`;
   - ACP `ClaudeCli` must either execute through a real CLI adapter or return a typed unsupported-provider error, never silently substitute Anthropic API.
6. `ModelCallService` must not mutate cloned config or insert providers/models at request time.

## Phase 4: Safety Defaults and Budget Policy

1. Change configured-role missing contract behavior from permissive fallback to restricted/fail-closed.
2. Replace `HallucinationDetector::permissive()` in production wiring with `with_known_tools`; make empty tool lists test-only or explicit `DetectorMode::NamesUnavailable`.
3. Add cumulative spend to `ToolContext` and enforce `MaxCostPerTurn`, per-tool, per-call, and per-plan budgets from `BudgetPolicy`.
4. Change `SupervisionStrategy::default()` to a nonzero restart policy, e.g. `max_restarts = 3`, bounded window, and typed escalation.
5. Serve production safety:
   - validate path segments in agent creation;
   - quote or structurally serialize TOML manifests;
   - move vision-loop state into `AppState`;
   - fail persistence errors loudly;
   - add timeouts to spawned plans;
   - default agent sidecars to `127.0.0.1`;
   - restrict CORS methods/headers and add security headers.

## Phase 5: UsageObservation Through the Stack

1. Move `UsageObservation` out of `roko-agent/src/usage.rs` into core telemetry.
2. Replace `TokenUsage` in `ModelCallResponse` with `UsageObservation`.
3. Keep requested, attempted, and final model/provider on every model-call result.
4. Provider parsers return:
   - `source = ProviderReported` with `Some(value)` only for fields actually present;
   - `source = Unknown` when no usage block exists;
   - `source = Estimated` only from an explicit estimation layer.
5. Remove lossy `From<UsageObservation> for Usage` from runtime paths. Allow display-only conversion at UI/report boundaries and name it `to_display_usage_zero_filled()`.
6. Update `FeedbackEvent::ModelCall`, gateway events, episodes, efficiency events, and summaries to use optional fields plus provenance.
7. Add JSONL schema versions:
   - v1 concrete zeros are legacy and ineligible for contextual learning unless provenance proves they are real;
   - v2 optional fields preserve unknowns.

## Phase 6: Real RoutingContext and Contextual Learning

1. Add `RoutingContextBuilder` at the dispatch boundary.
2. Populate from real inputs:
   - task category and complexity;
   - role and thinking level;
   - iteration/retry count and prior failure;
   - crate familiarity;
   - conductor load, active agents, queue depth, queue wait;
   - daimon policy and tier thresholds;
   - previous model and plan context tokens;
   - budget pressure and permission mode;
   - override source.
3. `DispatchResolver` may call the cascade router only with `RoutingContextStatus::Available`.
4. If context is unavailable, use declared config defaults and record `ContextUnavailable(reason)`.
5. Split observations:
   - `ContextualObservation`: requires real `RoutingContext`, model/provider identity, outcome, reward, usage provenance;
   - `ConfidenceObservation`: updates pass/fail stats only, no LinUCB features;
   - `DashboardOnlyObservation`: missing identity, missing context, unknown result, or legacy zero-filled data.
6. Rename or restrict `CascadeRouter::record_outcome` to `record_confidence_outcome`.
7. Force-backend overrides:
   - contextual only when the original dispatch context is available;
   - dampened reward and `source = Override`;
   - otherwise confidence-only, never `RoutingContext::default()`.

## Phase 7: Production Feedback and Safety Are Mandatory

1. Add `RuntimeMode = Dev | Test | Production` to resolved config.
2. In production:
   - feedback sink required before dispatch;
   - safety contract required before dispatch;
   - dangerous permission bypass rejected;
   - unknown provider/model identity rejected;
   - dispatch attempt and final outcome must be durably recorded;
   - telemetry write failure makes the run failed or quarantined, not successful.
3. In dev:
   - allow best-effort feedback, but label missing data as dashboard-only;
   - local dangerous overrides require acknowledgement and expiry.

## Migration Steps

1. Add new types and strict validator behind feature flag or `ROKO_CONFIG_V2_STRICT=1`.
2. Implement read-only `roko config doctor` showing config migration, provider/model provenance, dangerous overrides, and dispatch traces.
3. Migrate root `roko.toml` to current config version and remove dangerous runner permission.
4. Port CLI load sites from `Config` to `ValidatedConfig`/`ResolvedRuntimeConfig`.
5. Port model selection call sites to `DispatchResolver`.
6. Port provider adapters and model call service to immutable resolved config.
7. Port ACP/chat/serve/runner to `DispatchPlan`.
8. Port telemetry writes to optional usage schema v2.
9. Enable strict/prod failures in CI.
10. Remove compatibility aliases and legacy structs after all callers are migrated.

## Deletion Steps

- Delete CLI/core duplicate runtime schema fields in `crates/roko-cli/src/config.rs`.
- Delete runtime provider/model synthesis in `effective_providers`, `effective_models`, `provider/mod.rs`, and `model_call_service.rs`.
- Delete production `dispatch_direct` fallback.
- Delete raw Anthropic/OpenAI HTTP from ACP and chat surfaces.
- Delete provider env var reads outside config/provider auth.
- Delete hardcoded `dangerously_skip_permissions = true` sites.
- Delete permissive configured-role fallback and production `HallucinationDetector::permissive()`.
- Delete lossy usage conversions from learning/runtime paths.
- Delete `record_override_outcome(..., RoutingContext::default())`.
- Delete or intentionally wire the unexported learning modules called out in audit 10; do not leave 800 LOC of unreachable algorithms.

## Recurrence Checks

Add these as CI scripts or unit tests:

- `rg 'dangerously_skip_permissions\\s*=\\s*true' roko.toml crates/` fails outside fixtures/local override tests.
- `rg 'reqwest::Client::new\\(' crates/roko-cli crates/roko-acp crates/roko-serve` allows only shared provider factories/tests.
- `rg 'ANTHROPIC_API_KEY|OPENAI_API_KEY|ZAI_API_KEY' crates/roko-cli crates/roko-acp crates/roko-serve` fails outside approved auth modules.
- `rg 'Usage::default\\(\\)|unwrap_or\\(0\\)' crates/roko-agent/src/translate crates/roko-cli/src/runtime_feedback crates/roko-learn/src` flags telemetry parser changes for review.
- `rg 'RoutingContext::default\\(\\)' crates/roko-agent crates/roko-cli crates/roko-learn` fails outside tests and explicit builder defaults.
- `rg 'select\\(Vec::new\\(\\)\\)' crates/` fails.
- Compile-time tests ensure `ModelCallResponse.usage` is `UsageObservation`.
- Snapshot tests for `DispatchPlan.trace` on CLI override, provider override, cascade, project default, and fallback.

## Acceptance Tests

Config:

- root/shared config with `dangerously_skip_permissions = true` fails strict validation;
- local override with reason, expiry, scope, and env ack is accepted in dev and rejected in production;
- config version 1 migrates to current version or fails with actionable diagnostics;
- `agent.model` migrates to `agent.default_model` with a deprecation diagnostic;
- provider kind/transport/auth invalid combinations are rejected;
- multiple providers of same kind without priority/default model produce ambiguity errors;
- model alias, backend slug, provider id, and provenance appear in `roko config doctor`.

Dispatch:

- Claude CLI provider without `ANTHROPIC_API_KEY` does not route to Anthropic API;
- ACP/chat/serve route through `DispatchPlan` and provider adapters;
- unsupported transport returns a typed error naming provider id, kind, transport, and caller capability;
- requested, attempted, and final model/provider are recorded when fallback occurs;
- raw dispatch fallback cannot run in production.

Safety and budget:

- no production command emits `--dangerously-skip-permissions` without a valid local override, and production rejects all such overrides;
- configured role with missing contract fails closed or uses restricted contract with error-level diagnostic;
- unknown tool name is rejected when registry is available;
- cumulative tool spend over `MaxCostPerTurn` is rejected;
- default supervision restarts bounded failures and emits typed escalation;
- serve path traversal, TOML injection, persistence failure, and plan timeout tests pass.

Telemetry:

- provider response with no usage block serializes token/cost fields as null/absent, not zero;
- provider response with explicit zero serializes `Some(0)` with `ProviderReported`;
- runtime feedback and efficiency JSONL v2 preserve unknown duration, TTFT, tool counts, cost, and tokens;
- old zero-filled records are marked legacy/dashboard-only unless provenance proves known values.

Learning:

- learned routing is not invoked without `RoutingContextStatus::Available`;
- a real dispatch context increments contextual observation counters and LinUCB state;
- confidence-only observations update only confidence stats;
- force-backend override without context does not update LinUCB;
- force-backend override with real context records dampened contextual reward and provenance;
- production dispatch without feedback sink fails before model execution.
