# Converge Runner Deep Audit

Date: 2026-04-28

Scope: documentation-only audit of the converge runner output, based on the audit prompt in
`tmp/subsystem-audits/converge-runner/AUDIT-PROMPT.md`. This read covered the original intent
documents, the architecture runner context and prompts, the converge runner batch plan and prompts,
the post-merge audit documents, the pre-runner subsystem audits, and the current implementation.

No implementation changes are proposed in this file as patches. This document is the handoff context
for the next convergence pass.

## Executive Verdict

The converge runner produced useful infrastructure and made the new workflow engine compile and run
as the default path for `roko run`. It did not complete the intended architecture. The repository is
now in an intermediate state where core service abstractions exist, but important live entry points
still bypass them, several new abstractions are only partially wired, and the old runtime remains
large and reachable.

The most important gap is not any single bug. The gap is that the convergence changed surfaces before
it proved one semantic path. The desired invariant from `ANTI-PATTERNS.md` is still unmet:
"one feature, one implementation, one code path." The codebase currently has a v2 engine, a legacy
`run_once` path, a 21K-line `orchestrate.rs`, direct dispatch helpers, direct server gateway calls,
and duplicated policy/event/reporting adapters.

The best solution is a second convergence pass that repairs contracts and live wiring before deleting
legacy code. Do not continue by adding feature-specific fixes around the edges. First make the core
contracts complete and structured, then make all entry points use the same service factory and engine,
then retire the legacy paths behind a real feature boundary.

## Audit Inputs

Primary intent documents:

- `VISION.md`
- `ANTI-PATTERNS.md`
- `MASTER-IMPLEMENTATION-PLAN.md`
- `UNIFIED-IMPLEMENTATION-PLAN.md`

Runner and prompt documents:

- `tmp/runners/arch/BATCHES.md`
- `tmp/runners/arch/context-pack/00-RULES.md`
- `tmp/runners/arch/context-pack/01-ARCHITECTURE.md`
- `tmp/runners/arch/context-pack/02-EXISTING-CODE.md`
- `tmp/runners/arch/context-pack/03-ANTI-PATTERNS.md`
- `tmp/runners/arch/prompts/*.md`
- `tmp/runners/converge/BATCHES.md`
- `tmp/runners/converge/prompts/*.md`

Converge runner result documents:

- `tmp/subsystem-audits/converge-runner/README.md`
- `tmp/subsystem-audits/converge-runner/AUDIT.md`
- `tmp/subsystem-audits/converge-runner/OPEN-ISSUES.md`
- `tmp/subsystem-audits/converge-runner/FIXES-APPLIED.md`

Pre-runner subsystem audits:

- Orchestration runtime
- Gate system
- Inference dispatch
- Prompt assembly
- Learning feedback
- Safety and execution
- Cognitive layer

Primary code paths audited:

- `crates/roko-core/src/foundation.rs`
- `crates/roko-core/src/runtime_event.rs`
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-learn/src/feedback_service.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-serve/src/state.rs`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-serve/src/routes/shared_runs.rs`
- `crates/roko-serve/src/adapters.rs`
- `crates/roko-agent/src/gateway_events.rs`

## 1. Implementation Inventory By Track

### F - Foundation Contracts

What landed:

- Core foundation types now exist in `crates/roko-core/src/foundation.rs`.
- `ModelCallRequest`, `ModelCallResult`, `ModelCaller`, `PromptSpec`, `PromptAssembler`,
  `FeedbackEvent`, `FeedbackSink`, `GateConfig`, `GateService`, `EventConsumer`, `Effect`,
  `EffectExecutor`, `AffectPolicy`, and `DispatchModulation` are centralized in core.
- `RuntimeEvent` exists in `crates/roko-core/src/runtime_event.rs`.
- Several crates import the core foundation contracts rather than local definitions.

Remaining reality:

- `crates/roko-runtime/src/effect_driver.rs:31-87` still defines local `AffectContext`,
  `DispatchModulation`, and `AffectPolicy` even though canonical versions exist at
  `crates/roko-core/src/foundation.rs:336-400`.
- `crates/roko-cli/src/run.rs:319-405` contains a runtime policy adapter only because the runtime
  still uses a local policy trait. This preserves a duplicate abstraction boundary.
- `RuntimeEvent` at `crates/roko-core/src/runtime_event.rs:25-97` is not a durable event envelope.
  It has no sequence number, timestamp, schema version, serde representation, routing metadata,
  token/cost fields, prompt assembly fields, or knowledge provenance.
- `crates/roko-runtime/src/jsonl_logger.rs:64-69` writes `format!("{event:?}")`, and
  `crates/roko-runtime/src/projection.rs:161-181` parses debug strings back into state. That is a
  brittle event contract, not a foundation event stream.
- The core `ModelCaller` trait at `crates/roko-core/src/foundation.rs:134-139` supports only
  one-shot `call`. It does not model streaming, provider health probes, tool/MCP capabilities, or
  routing negotiation.

Verdict:

F-track created the right destination names but did not fully establish the contracts as the single
source of truth. The highest-value repair is to make these contracts complete and delete the runtime
copies before touching higher-level behavior.

### S - Shared Services

What landed:

- `ModelCallService` exists and can wrap provider dispatch in `crates/roko-agent/src/model_call_service.rs`.
- `PromptAssemblyService` exists in `crates/roko-compose/src/prompt_assembly_service.rs`.
- `FeedbackService` exists in `crates/roko-learn/src/feedback_service.rs`.
- `GateService` exists in `crates/roko-gate/src/gate_service.rs`.
- The CLI v2 path constructs these services in `crates/roko-cli/src/run.rs:480-547`.

Remaining reality:

- `ModelCallService` is not yet the sole inference path. `crates/roko-cli/src/run.rs:1264-1433`
  still contains `dispatch_agent` with direct Anthropic API, Claude CLI, Ollama, known protocol,
  generic subprocess, and TODO language to migrate to `ModelCallService`.
- `ModelCallService` has optional feedback. If no sink is attached,
  `crates/roko-agent/src/model_call_service.rs:413-426` returns success and records nothing.
- Knowledge advice is computed at `crates/roko-agent/src/model_call_service.rs:465-531`, but in the
  main `call` flow it is mostly logged into `request_id` debug metadata at
  `crates/roko-agent/src/model_call_service.rs:1314-1317`. It does not reliably influence model
  selection.
- Fallback routing is still empty in the provider call path. The live request uses
  `fallback_models: Vec::new()` at `crates/roko-agent/src/model_call_service.rs:1237`.
- The cache key in `crates/roko-agent/src/model_call_service.rs:672-677` sorts messages. That loses
  conversational ordering and can treat different conversations as the same request.
- Budgeting in `crates/roko-agent/src/model_call_service.rs:742-801` is process-local and cumulative,
  not a durable, reserved, cross-run budget.
- The thinking cap path sets extra args, but `crates/roko-agent/src/model_call_service.rs:1230-1235`
  says providers do not currently parse them.
- `PromptAssemblyService` has useful assembly layers at
  `crates/roko-compose/src/prompt_assembly_service.rs:265-390`, but the live CLI service builder uses
  `PromptAssemblyService::new()` at `crates/roko-cli/src/run.rs:526-529`, with no knowledge store,
  episodes, playbooks, tool instructions, or section effectiveness input.
- `FeedbackService` records model calls, can write efficiency metrics, and has knowledge scoring
  helpers, but workflow completion feedback is not emitted by v2. `WorkflowEngine` still has TODOs at
  `crates/roko-runtime/src/workflow_engine.rs:206-207` and `crates/roko-runtime/src/workflow_engine.rs:224-225`.
- `FeedbackService::flush_async` snapshots pending episodes before calling `flush` at
  `crates/roko-learn/src/feedback_service.rs:286-304`; events added between the snapshot and flush can
  be written to efficiency metrics without episode log parity.
- `GateService` supports rung names, but `custom` is a no-op shell gate returning true at
  `crates/roko-gate/src/gate_service.rs:74-77`.
- The judge gate fails closed, which is safer than the original stub, but it is still not a real LLM
  judge. See `crates/roko-gate/src/gate_service.rs:163-187`.
- CLI workflow gate mapping passes `"shell"` for shell gates at
  `crates/roko-cli/src/commands/util.rs:245-250`, but `GateService` recognizes `custom` and
  `custom:shell`, not `"shell"`, at `crates/roko-gate/src/gate_service.rs:49-60`.

Verdict:

S-track built the service shells. The live wiring is too shallow. The next pass must treat services
as mandatory runtime dependencies, not optional enrichments.

### E - Workflow Engine

What landed:

- `WorkflowEngine` exists in `crates/roko-runtime/src/workflow_engine.rs`.
- `PipelineState` exists in `crates/roko-runtime/src/pipeline_state.rs`.
- `EffectDriver` executes agent spawn, gate run, commit, and checkpoint effects in
  `crates/roko-runtime/src/effect_driver.rs`.
- `roko run` defaults to v2 engine through `EngineVariant::V2`.

Remaining reality:

- `PipelineState` is a fixed phase machine, not a general workflow graph. The main step logic at
  `crates/roko-runtime/src/pipeline_state.rs:586-755` hardcodes the strategy, implementation,
  gating, review, revision, and commit loop.
- The TOML workflow loader at `crates/roko-runtime/src/pipeline_state.rs:102-235` is a hand-rolled
  subset parser. It infers a few booleans from `[[workflow.steps]]`; it does not model arbitrary
  workflow nodes, edges, conditions, or typed gate settings.
- `EffectDriver` computes affect modulation at `crates/roko-runtime/src/effect_driver.rs:133-138`
  but does not apply it to the model request, prompt, budget, max tokens, temperature, or retry
  behavior.
- `EffectDriver` emits `AgentSpawned` with `model: String::new()` at
  `crates/roko-runtime/src/effect_driver.rs:165-170`, and creates `ModelCallRequest` with empty
  model, no caller, no budget, and no max tokens at
  `crates/roko-runtime/src/effect_driver.rs:176-189`.
- `WorkflowEngine` also emits `AgentSpawned` events with empty `agent_id` and empty `model` before
  the driver spawn at `crates/roko-runtime/src/workflow_engine.rs:146-151`,
  `crates/roko-runtime/src/workflow_engine.rs:155-160`,
  `crates/roko-runtime/src/workflow_engine.rs:166-171`, and
  `crates/roko-runtime/src/workflow_engine.rs:177-182`. This likely creates duplicate and low-value
  lifecycle events.
- Reviewer output is hardcoded as approval. `crates/roko-runtime/src/workflow_engine.rs:471-477`
  maps any `AgentCompleted` review result to `ReviewApproved`.
- Resume loads checkpoints, but it does not emit the first recovered phase transition. It starts from
  `resumed_output` at `crates/roko-runtime/src/workflow_engine.rs:312-313`.
- Resume emits `StateCheckpointed` with an empty path at
  `crates/roko-runtime/src/workflow_engine.rs:417-421`.
- `EffectDriver::save_checkpoint` correctly writes an atomic file and emits the path at
  `crates/roko-runtime/src/effect_driver.rs:381-410`, but the resume path does not use that same
  effect.
- Commit errors are represented as `CommitDone { hash: "error: ..." }` in
  `crates/roko-runtime/src/effect_driver.rs:311-378`. This avoids crashing but pollutes a success
  variant with error state.

Verdict:

E-track made a real engine skeleton. It is not yet semantically authoritative. It needs typed events,
real effect inputs, applied policies, real review semantics, and durable resume behavior.

### W - Entry Point Wiring

What landed:

- `roko run` can use the workflow engine by default.
- `EngineVariant::V2` is the default at `crates/roko-cli/src/main.rs:287-295`.
- The v2 path is selected in `crates/roko-cli/src/commands/util.rs:237-272`.
- Plan execution has a v2 adapter in `crates/roko-cli/src/orchestrate.rs:7502-7542`.

Remaining reality:

- `--share` is effectively a no-op on the default v2 path. `crates/roko-cli/src/main.rs:1987-1993`
  passes `serve || share`, so share starts the server branch, but
  `crates/roko-cli/src/commands/util.rs:237-272` returns from v2 before the legacy share code at
  `crates/roko-cli/src/commands/util.rs:280-312`.
- `write_shared_run` at `crates/roko-cli/src/run.rs:92-126` expects a `RunReport`, but
  `run_with_workflow_engine` returns `anyhow::Result<()>` at `crates/roko-cli/src/run.rs:565-571`.
- The v2 output prints a hardcoded model value `"claude-sonnet-4-20250514"` at
  `crates/roko-cli/src/run.rs:612`, regardless of the actual model used.
- Plan v2 remains an env-gated adapter inside the old `orchestrate.rs` at
  `crates/roko-cli/src/orchestrate.rs:7502-7509`.
- The plan adapter returns `self.current_report()` at `crates/roko-cli/src/orchestrate.rs:7542`,
  ignoring the actual `PlanWorkflowReport` content.
- The legacy `run_once` path still exists at `crates/roko-cli/src/run.rs:875-1120`.
- Direct dispatch remains at `crates/roko-cli/src/run.rs:1264-1433`.
- `crates/roko-cli/src/orchestrate.rs` is still about 21K lines and is still compiled and exported.

Verdict:

W-track made the default happy path point at v2, but not the full product surface. Sharing, plan
reports, ACP/server alignment, and legacy dispatch retirement remain open.

### O - Observability

What landed:

- `RuntimeEvent` exists.
- `JsonlLogger` records event lines.
- `RuntimeProjection` can read event logs.
- `StateHubBridge` maps some workflow events into CLI/server state.
- `SseAdapter` exists in `crates/roko-serve/src/adapters.rs`.

Remaining reality:

- Event logs are debug strings, not typed JSON. See `crates/roko-runtime/src/jsonl_logger.rs:64-69`.
- Projection parses debug strings at `crates/roko-runtime/src/projection.rs:161-181`.
- `StateHubBridge` maps only `WorkflowStarted`, `PhaseTransition`, and `WorkflowCompleted` at
  `crates/roko-cli/src/run.rs:141-187`; agent, gate, feedback, checkpoint, and cost events are not
  projected.
- The server SSE endpoint subscribes to `state.sse_adapter` at
  `crates/roko-serve/src/routes/mod.rs:158-164`, but the v2 workflow engine is not registered as a
  producer for that adapter.
- `crates/roko-serve/src/adapters.rs:166-171` still describes future route integration.
- `crates/roko-cli/src/lib.rs:32-37` and `crates/roko-serve/src/lib.rs:61-74` each include StateHub
  via `#[path]`, creating split type islands instead of a shared crate boundary.

Verdict:

O-track created useful observability pieces, but the current event stream is not durable enough to be
the system's source of truth.

### R - Retirement

What landed:

- The converge runner created a feature strategy around `legacy-orchestrate`.
- The default `roko run` path is v2.
- `R01` landed; `R02` through `R05` failed according to `OPEN-ISSUES.md`.

Remaining reality:

- `crates/roko-cli/src/orchestrate.rs` remains compiled and exported.
- `crates/roko-cli/src/run.rs` still contains legacy run and dispatch paths.
- Several direct providers and subprocess integrations remain reachable outside `ModelCallService`.
- `Command::new("claude")` still appears in direct command-related code, including
  `crates/roko-cli/src/dispatch_direct.rs:141`. `crates/roko-cli/src/auth_detect.rs:100` also uses
  it for version detection; that may be acceptable if it is explicitly isolated as auth probing.

Verdict:

R-track is the least complete track. Retirement should not be attempted by blind deletion yet; it
should follow a parity proof and a feature-gated compile check.

### C - CLI, Server, ACP, and UX Continuity

What landed:

- v2 CLI output exists.
- Shared run routes exist.
- Gateway endpoint continues to return inference responses.
- Server has an SSE adapter type.

Remaining reality:

- Shared run creation in `crates/roko-serve/src/routes/shared_runs.rs:70-125` creates a placeholder
  transcript with empty agent, empty role, empty prompt, no output, and `success: false`.
- CLI `--share` does not work with the default v2 path, as described in W-track.
- Gateway calls in `crates/roko-serve/src/routes/gateway.rs:265-335` call `ModelCallService`, update
  provider health and counters, and publish a server event, but they do not write durable gateway
  events through `GatewayEventWriter`.
- `crates/roko-serve/src/state.rs:479-482` constructs `ModelCallService` with config only. It does
  not attach feedback, gateway event writer, knowledge, cascade router, or event consumers.

Verdict:

C-track preserved surface availability but not semantic parity. The current UX can report or share
incomplete data because the v2 run result is not a first-class report object.

### T - Tests and Proof

What landed:

- Unit tests exist for several new services and state-machine paths.
- The post-fix document says cargo check, clippy, and tests passed after manual fixes.
- Layer check is present in CI.

Remaining reality:

- Tests mostly exercise mocks and local state. They do not prove the default `roko run` v2 path with
  sharing, server bridge, checkpoint resume, gateway durability, knowledge-informed prompting, or
  legacy-disabled builds.
- `crates/roko-runtime/src/pipeline_state.rs:1016-1080` tests resumed `PipelineState`, not
  `WorkflowEngine::resume` with event emission and checkpoint paths.
- There is no proof that `--share` works with default v2. Current code indicates it does not.
- There is no integration test proving shell gates map to `GateService` names.
- There is no negative test preventing `format!("{event:?}")` debug logs from returning.

Verdict:

T-track needs product-level integration tests. Unit tests are not enough for a convergence effort
whose main risk is path divergence.

### D - Daimon, Affect, and Policy

What landed:

- `crates/roko-daimon/src/policy.rs:35-107` implements the canonical foundation `AffectPolicy`.
- CLI v2 wires a policy adapter into `EffectDriver`.

Remaining reality:

- Runtime still uses a local policy trait at `crates/roko-runtime/src/effect_driver.rs:31-87`.
- CLI has to bridge foundation policy to runtime policy in `crates/roko-cli/src/run.rs:319-405`.
- `EffectDriver` computes modulation but does not apply it. The policy can observe and return
  guidance, but dispatch behavior is unchanged.
- `run_gates` calls `on_gate_result` with fixed `rung: 0` and `confidence: 0.0` at
  `crates/roko-runtime/src/effect_driver.rs:285-289`, so policy feedback is low-fidelity.

Verdict:

D-track attached policy shape but not policy effect. Delete the duplicate trait first, then make the
policy output visible in the actual model and gate requests.

### G - Gateway, Caching, Budget, and Provider Dispatch

What landed:

- Gateway event structures and writer/projection exist in `crates/roko-agent/src/gateway_events.rs`.
- `ModelCallService` has cache, cost prediction, budget tracking, thinking cap, convergence detection,
  and provider calls.
- Server gateway route uses `ModelCallService`.

Remaining reality:

- `GatewayEventWriter` is not instantiated in the live CLI or server path.
- Server gateway stats contain placeholders, including cache hit data at
  `crates/roko-serve/src/routes/gateway.rs:418-425`.
- Cache key generation sorts messages at `crates/roko-agent/src/model_call_service.rs:672-677`,
  which is semantically wrong for chat-like prompts.
- Budget tracking is not persisted or reserved across concurrent calls.
- Provider fallback is empty in live calls.
- MCP config fallback is still TODO at `crates/roko-agent/src/model_call_service.rs:399-405`.

Verdict:

G-track created an important service foundation but not a durable gateway. It needs live event
writing, correct cache keys, fallback routing, and a shared service factory.

### K - Knowledge and Learning Loop

What landed:

- Prompt assembly can query knowledge and include techniques and warnings.
- Feedback service can record knowledge usage and compute knowledge scores.
- Model call service can build knowledge advice.

Remaining reality:

- CLI service construction uses `PromptAssemblyService::new()` with no knowledge store.
- Server `ModelCallService` construction attaches no knowledge store.
- Knowledge advice does not reliably change routing or prompting.
- `FeedbackService::record_knowledge_usage` exists at `crates/roko-learn/src/feedback_service.rs:179-220`,
  but live usage from prompt assembly/model calls was not found.
- The intended closed loop of knowledge consulted, output measured, score updated, and future prompt
  or model choice changed is still mostly aspirational.

Verdict:

K-track added APIs, not a learning loop. The next pass must make knowledge provenance part of the
request and feedback event contracts.

### X - Safety, Contracts, and Fail-Closed Behavior

What landed:

- LLM judge now fails closed instead of passing as a stub.
- Some contract and layer checks exist.
- Effect driver commit no longer panics on git failures.

Remaining reality:

- Safety remains split across legacy dispatch, direct command paths, gates, and workflow effects.
- `crates/roko-cli/src/run.rs:1963-1967` returns `true` for unknown roles in
  `role_allows_dangerous_skip_permissions`, which is an unsafe default for role-based permission
  logic.
- Direct environment reads remain in CLI and server paths. The old rule was to centralize provider
  config rather than scatter direct `std::env::var` checks.
- Debug-string event logs and hand-rolled parsers are contract risks.
- The `"shell"` gate name mismatch is a fail-closed behavior, but it is a product regression rather
  than a deliberate policy decision.

Verdict:

X-track improved one critical fail-open issue but did not centralize safety. Safety should be
expressed through typed gate, model, and effect contracts rather than side-path checks.

### L - Layering and CI

What landed:

- Crate layer metadata exists.
- `scripts/layer_check.rs` exists.
- CI runs clippy, tests, fmt check, and layer check.
- `deny.toml` exists.

Remaining reality:

- `scripts/layer_check.rs` checks direct dependency metadata and is useful, but it is shallow. It
  does not catch semantic layering escapes such as `#[path]` shared modules or duplicated runtime
  foundation traits.
- `crates/roko-runtime/src/lib.rs:16-23` says runtime should avoid domain types, but runtime now
  contains workflow, effect, agent, plan, and gate concepts.
- `deny.toml` warns on multiple versions and unknown git sources, but bans are effectively empty.
- `StateHub` is included by path from two crates instead of being a normal crate dependency.

Verdict:

L-track added CI rails but not enough architectural enforcement. The next pass should add negative
checks for the specific anti-patterns that returned during convergence.

## 2. Gap Analysis: Intent Versus Reality

### Gap 1: One Engine

Intent:

- One workflow engine should own orchestration for CLI, plans, ACP, and server.
- Legacy runtimes should be retired or isolated behind explicit compatibility boundaries.

Reality:

- `WorkflowEngine` exists and is default for one CLI path.
- `run_once`, `dispatch_agent`, direct dispatch helpers, and `orchestrate.rs` remain.
- Plan v2 is env-gated inside legacy orchestration.
- Server gateway calls `ModelCallService` directly, not through the workflow engine.

Impact:

- Behavioral fixes must still be applied in multiple places.
- New services can be bypassed accidentally.
- It is difficult to know which path produced a run, report, share, or feedback event.

### Gap 2: Adapter-First Inference

Intent:

- All model calls should go through `ModelCallService`, provider dispatch, routing, feedback, budget,
  cache, gateway events, and knowledge.

Reality:

- `ModelCallService` exists but direct CLI dispatch remains.
- Gateway route uses `ModelCallService`, but without feedback, gateway writer, knowledge, or cascade.
- CLI v2 uses `ModelCallService`, but requests omit caller, model metadata, and budget.
- Provider fallback and knowledge-informed routing are incomplete.

Impact:

- Cost, routing, provider health, feedback, and learning are incomplete or inconsistent.
- Observability cannot reconstruct all inference behavior.

### Gap 3: Dynamic Prompt Assembly

Intent:

- Prompts should be assembled dynamically from role, workflow, task, gates, knowledge, episodes,
  playbooks, conventions, tool instructions, and token budgets.

Reality:

- `PromptAssemblyService` can assemble rich prompts if configured.
- Live CLI v2 constructs it with defaults and no knowledge or episode sources.
- Legacy paths still use local prompt construction and dispatch-specific prompt logic.

Impact:

- The system does not yet realize the `VISION.md` prompt assembly goal in normal runs.
- Prompt quality and learning integration vary by entry point.

### Gap 4: Learning and Feedback

Intent:

- Every model call, gate, workflow outcome, knowledge usage, and provider decision should feed a
  learning loop.

Reality:

- Model call feedback can be recorded when a sink is attached.
- Workflow final feedback is TODO in `WorkflowEngine`.
- Knowledge usage scoring APIs exist but are not wired into live prompt/model calls.
- Feedback is optional in service construction.

Impact:

- The system cannot reliably learn from normal v2 runs.
- Cascade and router improvements have incomplete data.

### Gap 5: Observability and Resume

Intent:

- Runtime events should be the durable, structured source of truth for dashboards, SSE, transcripts,
  projections, checkpoints, and resume.

Reality:

- Events are debug strings in JSONL.
- Projections parse debug strings.
- StateHub bridge maps only a few event types.
- Resume emits an empty checkpoint path and misses the initial recovered transition.

Impact:

- Dashboards and shared transcripts can be incomplete or incorrect.
- Resume is hard to trust in product workflows.

### Gap 6: Legacy Retirement

Intent:

- Old runtime code should be feature-gated, deleted, or reduced to compatibility shims after parity.

Reality:

- R-track mostly failed.
- The largest legacy files and direct dispatch paths remain.
- `legacy-orchestrate` exists, but the old module is still compiled/exported.

Impact:

- New architecture can regress because old paths keep providing alternate implementations.
- Architecture rules are aspirational rather than enforced.

### Gap 7: Safety and Contract Integrity

Intent:

- Safety decisions should be explicit, typed, fail-closed, and centralized.

Reality:

- Judge fail-closed improved.
- Unknown role dangerous-skip behavior remains permissive.
- Shell gate mapping likely fails because names do not match.
- Contracts still rely on empty strings and debug parsing.

Impact:

- Some failures are safer than before, but behavior is still inconsistent and hard to audit.

## 3. Ideal Design

The ideal design is not a larger system. It is the existing intended system with stricter ownership
and fewer alternate paths.

### Core Contracts

- `RuntimeEvent` should be a serde-serializable event envelope with schema version, run id, sequence,
  timestamp, source, and typed payload.
- Event payloads should include enough metadata to reconstruct a run: workflow phase, agent id, role,
  model, provider, prompt section ids, knowledge ids, gate name, rung, decision, token usage, cost,
  checkpoint path, and error kind.
- `AffectPolicy`, `DispatchModulation`, and related policy types should exist only in
  `roko-core::foundation`.
- `ModelCallRequest` should represent the full contract: caller identity, task/workflow/run context,
  routing hints, budget, cache policy, prompt section provenance, tool/MCP requirements, and expected
  output mode.
- `ModelCaller` should have clear support for one-shot calls, streaming if required by product
  surfaces, and provider health/probe behavior if the gateway owns fallback.

### Workflow Engine

- `PipelineState` should be a pure state transition engine over a typed workflow graph, not a
  hardcoded phase machine with inferred booleans.
- `EffectDriver` should execute only typed effects. It should not make high-level decisions.
- Agent lifecycle events should be emitted by one owner. Prefer the driver, because it knows the real
  model, request id, and provider result.
- Checkpoints should be written through one effect path and resumed through the same schema.
- Review, revision, and gate decisions should be parsed as typed outcomes rather than inferred from
  any successful model call.

### Service Layer

- `ServiceFactory` or equivalent should construct the canonical service set for CLI, plan, ACP, and
  server. This avoids drift between `run.rs`, `serve/state.rs`, and future adapters.
- `ModelCallService` should be the only inference gateway. It should own routing, fallback, cache,
  budget, provider health, gateway events, feedback, and knowledge hints.
- `PromptAssemblyService` should be the only prompt builder for engine-driven work. Legacy prompt
  builders should be deleted or wrapped after parity.
- `GateService` should own rung execution and configuration. Custom shell gates need real command
  config, not a hardcoded `true`.
- `FeedbackService` should be mandatory for production services. Tests can use an explicit no-op
  sink, but production construction should not silently drop feedback.

### Entry Points

- `roko run`, plan execution, ACP sessions, and server background runs should call the same
  `WorkflowEngine`.
- The return type should be a first-class `WorkflowRunReport` that can drive CLI output, JSON output,
  shared transcripts, StateHub updates, and episode feedback.
- Legacy orchestration should be behind a feature flag that can be disabled in CI. When disabled, the
  crate should still compile and the default product path should still work.

### Observability

- JSONL should contain structured `RuntimeEvent` payloads, not debug strings.
- `RuntimeProjection` should deserialize events directly and maintain durable projection state.
- `StateHub` should live in one shared crate, not through duplicate `#[path]` inclusions.
- SSE adapters should be registered as real event consumers for workflow engine runs.
- Gateway events should share the same run/request identifiers as runtime events.

### Learning Loop

- Prompt assembly should return section ids and knowledge ids.
- Model calls should record which prompt sections, knowledge entries, and routing hints were used.
- Feedback should tie model outcomes, gates, revisions, commits, and user-facing success into one
  episode.
- Knowledge scores should update from actual usage and outcome data, then affect future prompt
  assembly and routing.

## 4. Anti-Patterns Introduced Or Preserved

### Duplicate Foundation Traits

`crates/roko-runtime/src/effect_driver.rs:31-87` duplicates policy contracts that already exist in
core. This directly violates the foundation convergence goal and forces adapter code in CLI.

### Debug Strings As Data Contracts

`crates/roko-runtime/src/jsonl_logger.rs:64-69` stores debug strings, and
`crates/roko-runtime/src/projection.rs:161-181` parses them. This makes event logs fragile and
non-versioned.

### Empty String Placeholders

`WorkflowEngine` and `EffectDriver` emit events and requests with empty `agent_id`, `model`, and
checkpoint path fields. Empty strings make invalid state look valid.

### Fake Success Paths

`GateService` maps `custom` to `true`, and shared run creation builds placeholder transcripts. These
are worse than explicit unsupported errors because they appear to work while losing semantics.

### Hand-Rolled Structured Parsing

The workflow TOML parser in `PipelineState` is ad hoc. It cannot safely become the source of truth
for user-authored workflow graphs.

### Shadow Runtime

The v2 engine exists, but legacy run, dispatch, and orchestration code remain reachable. This
preserves the original divergence problem under a new default flag.

### Optional Production Feedback

Feedback can be absent without error. That is acceptable in narrow tests, but not in the product
runtime where learning is a core principle.

### Path-Based Shared Modules

`StateHub` is imported by path from CLI and serve. This bypasses normal crate ownership and type
identity.

### Success Variants Carrying Errors

Commit errors are encoded inside `CommitDone.hash`. Typed error outcomes would be clearer and safer.

### Order-Insensitive Prompt Cache

Sorting chat messages inside cache keys is a semantic bug. Message order is part of the prompt.

### Name Mismatch Between CLI Gates And GateService

CLI passes `"shell"` while `GateService` accepts `custom` or `custom:shell`. This is a convergence
regression caused by independent local mappings.

## 5. Concrete Action Plan

### Priority 1: Repair Core Contracts Before More Feature Work

Goal: make the foundation layer the real contract boundary.

Actions:

- Delete local policy contracts in `crates/roko-runtime/src/effect_driver.rs:31-87` and use
  `crates/roko-core/src/foundation.rs:336-400` directly.
- Delete the now-unnecessary CLI policy bridge in `crates/roko-cli/src/run.rs:319-405`.
- Replace debug-string event logging in `crates/roko-runtime/src/jsonl_logger.rs:64-69` with
  serde-serialized `RuntimeEvent` envelopes.
- Replace projection debug parsing in `crates/roko-runtime/src/projection.rs:161-181` with typed
  event deserialization.
- Extend `crates/roko-core/src/runtime_event.rs:25-97` with sequence, timestamp, schema version,
  source, model/provider metadata, token/cost metadata, prompt section ids, knowledge ids, and
  checkpoint paths.
- Remove empty event fields emitted from `crates/roko-runtime/src/workflow_engine.rs:146-151`,
  `crates/roko-runtime/src/workflow_engine.rs:155-160`,
  `crates/roko-runtime/src/workflow_engine.rs:166-171`,
  `crates/roko-runtime/src/workflow_engine.rs:177-182`, and
  `crates/roko-runtime/src/effect_driver.rs:165-189`.

Acceptance checks:

- No local `trait AffectPolicy` outside core.
- No `format!("{event:?}")` in runtime event logging.
- No parser logic that depends on `Debug` formatting.
- Runtime events can be serialized, deserialized, and projected in tests.

### Priority 2: Make The V2 Engine Semantically Correct

Goal: make the default v2 path trustworthy before deleting legacy fallbacks.

Actions:

- Apply affect modulation computed at `crates/roko-runtime/src/effect_driver.rs:133-138` to the
  actual `ModelCallRequest` and execution policy at
  `crates/roko-runtime/src/effect_driver.rs:176-189`.
- Include caller identity, budget, model/routing hints, role, and run id in every model request.
- Fix gate config and shell gate naming across `crates/roko-cli/src/commands/util.rs:241-250`,
  `crates/roko-cli/src/orchestrate.rs:7514-7523`, and
  `crates/roko-gate/src/gate_service.rs:49-60`.
- Replace the no-op custom gate at `crates/roko-gate/src/gate_service.rs:74-77` with typed custom
  command config or an explicit unsupported error.
- Emit final workflow feedback at `crates/roko-runtime/src/workflow_engine.rs:206-207` and
  `crates/roko-runtime/src/workflow_engine.rs:224-225`.
- Fix resume event behavior around `crates/roko-runtime/src/workflow_engine.rs:312-313` and
  `crates/roko-runtime/src/workflow_engine.rs:417-421`.
- Route all checkpoint writes through the effect path at
  `crates/roko-runtime/src/effect_driver.rs:381-410`.
- Replace commit error-in-success behavior in `crates/roko-runtime/src/effect_driver.rs:311-378`
  with a typed failure outcome.
- Replace hardcoded review approval in `crates/roko-runtime/src/workflow_engine.rs:471-477` with a
  typed review result contract.

Acceptance checks:

- V2 run emits one agent lifecycle sequence with real agent id and model.
- Resume emits the recovered phase and a non-empty checkpoint path.
- Shell/custom gates work through one naming schema.
- Workflow completion always records feedback.

### Priority 3: Centralize Service Construction And Make Services Mandatory

Goal: prevent CLI, server, ACP, and tests from building different runtimes.

Actions:

- Introduce a shared service construction path to replace divergent construction in
  `crates/roko-cli/src/run.rs:480-547` and `crates/roko-serve/src/state.rs:479-482`.
- Attach feedback with episode logging and cascade/router data instead of only
  `FeedbackService::from_roko_dir` at `crates/roko-cli/src/run.rs:513-514`.
- Construct `PromptAssemblyService` with knowledge store, episodes, playbooks, tool instructions, and
  section effectiveness rather than `PromptAssemblyService::new()` at
  `crates/roko-cli/src/run.rs:526-529`.
- Attach model router, cascade router, knowledge store, gateway event writer, and event consumers to
  `ModelCallService`.
- Make production feedback mandatory. Keep explicit no-op sinks only in tests. The silent success
  branch at `crates/roko-agent/src/model_call_service.rs:413-426` should not be used in production
  service construction.
- Fix prompt cache key generation at `crates/roko-agent/src/model_call_service.rs:672-677` so message
  order is preserved.
- Make gateway calls in `crates/roko-serve/src/routes/gateway.rs:265-335` write durable gateway
  events and feed the same event/feedback path.

Acceptance checks:

- CLI and server use the same factory for core services.
- Gateway inference writes `.roko/learn/gateway.jsonl` or equivalent durable events.
- Knowledge ids and prompt section ids appear in model feedback.
- Cache tests prove message order changes the key.

### Priority 4: Retire Legacy Only After V2 Parity Proof

Goal: remove duplicate implementations without breaking product behavior.

Actions:

- Make `legacy-orchestrate` a real compile boundary around module declarations and exports in
  `crates/roko-cli/src/lib.rs:77` and `crates/roko-cli/src/lib.rs:126`.
- Move plan execution off the env-gated adapter at
  `crates/roko-cli/src/orchestrate.rs:7502-7509` and into the same v2 workflow entry point used by
  `roko run`.
- Fix plan report propagation instead of returning `self.current_report()` at
  `crates/roko-cli/src/orchestrate.rs:7542`.
- Migrate or delete direct dispatch from `crates/roko-cli/src/run.rs:1264-1433`.
- Make `crates/roko-cli/src/run.rs:875-1120` either a thin wrapper over `WorkflowEngine` or feature
  gated legacy code.
- Isolate acceptable command probes, such as `auth_detect`, from runtime dispatch. Remove or gate
  direct runtime `Command::new("claude")` usage such as `crates/roko-cli/src/dispatch_direct.rs:141`.

Acceptance checks:

- `cargo check --no-default-features` or an equivalent legacy-disabled check proves v2 compiles
  without old orchestration.
- Negative grep checks fail CI for direct runtime dispatch patterns outside approved probes.
- Plan execution produces a v2 report from the same engine result type as `roko run`.

### Priority 5: Prove The Product Flows

Goal: replace mock-only confidence with end-to-end convergence proof.

Actions:

- Add an integration test for default `roko run --share`; current flow passes `serve || share` at
  `crates/roko-cli/src/main.rs:1987-1993` and returns before share handling at
  `crates/roko-cli/src/commands/util.rs:237-272`.
- Add an integration test for `WorkflowEngine::resume` with an actual checkpoint file, non-empty
  checkpoint path event, and recovered phase transition. Existing resume tests at
  `crates/roko-runtime/src/pipeline_state.rs:1016-1080` only cover state behavior.
- Add a gateway durability test around `crates/roko-serve/src/routes/gateway.rs:299-335`.
- Add a StateHub/SSE bridge test proving workflow events reach the server subscriber at
  `crates/roko-serve/src/routes/mod.rs:158-164`.
- Add a knowledge loop test proving a knowledge entry included by prompt assembly is recorded by
  feedback and can influence a later request.
- Strengthen layer checks to catch `#[path]` StateHub duplication in
  `crates/roko-cli/src/lib.rs:32-37` and `crates/roko-serve/src/lib.rs:61-74`.

Acceptance checks:

- Default v2 run can generate a real share transcript.
- Resume works from a real checkpoint file.
- Gateway stats derive from durable gateway events, not placeholders.
- CI has negative checks for the known convergence anti-patterns.

## 6. Second Converge Runner Batch Plan

The next runner should not repeat an 87-patch broad sweep. Use fewer, stricter waves with explicit
acceptance tests per wave. The safest order is contracts, engine semantics, service wiring, entry
point convergence, retirement, then proof.

### Wave A: Contract Repair

A01 - Runtime event envelope:

- Add serde event envelope with run id, sequence, timestamp, schema version, source, and typed payload.
- Migrate `RuntimeEvent` variants without changing behavior yet.

A02 - Single affect contract:

- Delete runtime-local `AffectPolicy`, `AffectContext`, and `DispatchModulation`.
- Use core foundation types directly in runtime and daimon.

A03 - Request and feedback metadata:

- Extend model request and feedback event metadata for caller, role, run id, prompt section ids,
  knowledge ids, budget, model, provider, and gateway request id.

A04 - Structured logs and projection:

- Replace debug-string JSONL with typed event JSON.
- Replace projection string parsing with serde deserialization.

A05 - Contract guard tests:

- Add negative checks for duplicate foundation traits and debug-string event logging.

### Wave B: Engine Semantics

B01 - Agent event ownership:

- Make the driver the owner of real agent lifecycle events, or make the engine emit only high-level
  requested events with a distinct event type. Remove empty duplicate spawn events.

B02 - Apply policy modulation:

- Convert `DispatchModulation` into concrete request/execution changes such as model routing hints,
  max turns, token budget, temperature, or review strictness.

B03 - Gate config schema:

- Replace loose string gate names with typed gate config.
- Fix shell/custom mapping and remove the no-op custom gate.

B04 - Resume and checkpoint correctness:

- Route checkpoint save/resume through one schema.
- Emit recovered phase and non-empty checkpoint path events.

B05 - Workflow completion feedback:

- Emit workflow complete/fail feedback and flush it through the same feedback sink.

B06 - Review and commit outcomes:

- Replace hardcoded review approval and commit error-as-hash with typed success/failure outcomes.

### Wave C: Service Wiring

C01 - Shared service factory:

- Create one construction path for CLI, server, ACP, and tests.
- Support explicit test no-op services without weakening production defaults.

C02 - ModelCallService completeness:

- Attach feedback, gateway writer, router, cascade, knowledge, event consumers, cache, budget, and
  provider health through the factory.

C03 - Gateway durability:

- Use `GatewayEventWriter` in CLI and server model calls.
- Make gateway stats read from durable/projection state.

C04 - Prompt assembly live context:

- Wire knowledge, episodes, playbooks, conventions, tool instructions, and section effectiveness into
  live prompt assembly.

C05 - Feedback and knowledge provenance:

- Record which prompt sections and knowledge ids were used.
- Update knowledge scores from real outcomes.

C06 - Cache and budget correctness:

- Preserve message order in cache keys.
- Add tests for concurrent budget behavior or explicitly scope budget as best-effort local state.

### Wave D: Entry Point Convergence

D01 - Unified run report:

- Make `WorkflowEngine` return a first-class report that drives CLI text, JSON output, StateHub,
  sharing, and feedback.

D02 - `roko run --share` parity:

- Make share work on default v2. Remove legacy-only share assumptions.

D03 - Plan execution on v2:

- Move plan execution out of `orchestrate.rs` env gating and into the same engine service.

D04 - Server workflow execution:

- Register workflow event consumers with SSE/StateHub in server run paths.

D05 - ACP session convergence:

- Ensure ACP uses the same engine and service factory, with the same feedback and event stream.

### Wave E: Legacy Retirement

E01 - Real `legacy-orchestrate` boundary:

- Feature-gate module declarations, exports, and old adapters.

E02 - Direct dispatch removal:

- Delete or feature-gate old dispatch paths after parity tests pass.

E03 - Legacy-disabled compile:

- Add CI check that compiles and tests without legacy orchestration.

E04 - Legacy-enabled compatibility:

- Keep a targeted compatibility check while old feature remains.

E05 - Prompt and parser cleanup:

- Delete old prompt builders, hand-rolled report shims, and duplicate parsers once all entry points
  use the services.

### Wave F: Proof And Enforcement

F01 - Default v2 run fixture:

- Mock model provider, real prompt assembly, real feedback, real event log, real projection.

F02 - Share transcript fixture:

- Prove `roko run --share` produces a transcript with real agent, role, prompt, model, output, gates,
  and success state.

F03 - Resume fixture:

- Start run, checkpoint, resume, finish, and verify event sequence and final report.

F04 - Gateway fixture:

- Hit server gateway route, verify durable gateway event, feedback event, provider health, and stats.

F05 - Knowledge loop fixture:

- Include a knowledge entry in prompt assembly, record usage, score it after outcome, and show that a
  later request can observe the updated score.

F06 - Architecture negative checks:

- Fail CI on duplicate foundation traits, debug event parsing, direct model subprocess runtime paths,
  no-op gates, empty model event fields, and path-based shared modules.

## Final Recommendation

Freeze feature expansion until the second convergence pass repairs the architectural contracts and
proves one live path. The fastest path to the intended system is:

1. Make core contracts typed, durable, and unique.
2. Make v2 engine events, gates, checkpoints, reviews, commits, and feedback semantically correct.
3. Build all runtime services through one factory.
4. Prove default CLI, plan, server, ACP, share, gateway, and resume flows.
5. Retire or feature-gate every legacy path that still duplicates the new engine.

The current codebase has enough of the new architecture to converge cleanly. The next pass should be
less about adding new pieces and more about making the pieces impossible to bypass.
