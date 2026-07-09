# 30 - Dispatch and Streaming Agent Packets

Purpose: break `23-dispatch-streaming-migration-plan.md` into mechanical packets.
These packets should be done after Wave 0 guardrails exist.
Use `28-agent-tasking-playbook.md` as the assignment template. Assign one packet at a
time; do not ask a low-tier agent to "unify dispatch" in one pass.

Dispatch anti-patterns to avoid:

- Do not add provider HTTP, SSE parsing, auth env lookup, or max-token defaults to ACP,
  chat, serve, or demo crates.
- Do not preserve a raw fallback path while claiming dispatch is unified.
- Do not hide unknown provider/model/capability state behind empty strings or default
  enum variants.
- Do not migrate every provider in one packet; prove the shape with one provider, then
  repeat mechanically.
- Do not change execution semantics in packets whose purpose is adding shared types or
  compatibility adapters.

## D1: Add Core DispatchPlan Type Skeleton

Context files:

- `tmp/subsystem-audits/05-01/18-model-dispatch-redesign.md`
- `tmp/subsystem-audits/05-01/23-dispatch-streaming-migration-plan.md`

Write scope:

- `crates/roko-core/src/dispatch_plan.rs`
- `crates/roko-core/src/lib.rs` or relevant module exports

Mechanical steps:

1. Add a new module containing only data types:
   `DispatchRequest`, `DispatchRequirement`, `DispatchPlan`, `TransportPlan`,
   `FallbackPolicy`, `DispatchAttempt`, `DispatchError`.
2. Use existing `ProviderKind` and model/profile types where possible.
3. Add `#[derive(Debug, Clone, Serialize, Deserialize)]` only where the crate already
   depends on serde for similar public config/event types.
4. Re-export the module.
5. Add a simple construction/serialization unit test if serde is enabled.

Do not:

- Do not change runtime behavior.
- Do not move existing resolver logic yet.
- Do not add provider HTTP, env lookup, or routing logic.

Verification:

```bash
cargo check -p roko-core
cargo test -p roko-core dispatch_plan
```

Acceptance:

- Core exposes the new types and no production behavior changes.

## D2: Rename Runner-Local DispatchPlan To Avoid Name Collision

Context files:

- `tmp/subsystem-audits/05-01/23-dispatch-streaming-migration-plan.md`

Write scope:

- `crates/roko-cli/src/dispatch/`
- tests that directly reference the runner-local type

Mechanical steps:

1. Find any CLI runner-local `DispatchPlan` type unrelated to the shared execution
   contract.
2. Rename it to `RunnerDispatchPlan`.
3. Update imports/usages mechanically.
4. Do not change fields or behavior.

Do not:

- Do not introduce the shared core plan in this packet.
- Do not alter dispatch execution.

Verification:

```bash
cargo check -p roko-cli
```

Acceptance:

- `DispatchPlan` name is free to mean the shared execution contract.

## D3: Add Resolver Wrapper That Produces DispatchPlan From Existing Selection

Context files:

- `tmp/subsystem-audits/05-01/18-model-dispatch-redesign.md`
- `tmp/subsystem-audits/05-01/23-dispatch-streaming-migration-plan.md`

Write scope:

- `crates/roko-agent/src/dispatch_resolver.rs` or an existing resolver module
- module export files
- focused resolver tests

Mechanical steps:

1. Add `DispatchResolver::resolve_existing(...) -> Result<DispatchPlan>`.
2. Internally call the existing model selection/config resolution path.
3. Populate plan fields with existing known data and explicit `Unknown`/`Unvalidated`
   markers for fields not yet supported.
4. Add tests for CLI override, project default, missing provider, and unsupported
   capability placeholder.

Do not:

- Do not remove existing selection code yet.
- Do not synthesize new providers.
- Do not claim auth/capability validation is complete unless implemented.

Verification:

```bash
cargo test -p roko-agent dispatch_resolver
cargo check -p roko-agent
```

Acceptance:

- A plan can be produced from existing behavior without changing execution.
- Unknown/unvalidated fields are explicit, not empty strings.

## D4: Add ModelStreamEvent Type And Call-To-Stream Adapter

Context files:

- `tmp/subsystem-audits/05-01/18-model-dispatch-redesign.md`
- `tmp/subsystem-audits/05-01/23-dispatch-streaming-migration-plan.md`

Write scope:

- `crates/roko-core/src/foundation.rs` or a new core model stream module
- `crates/roko-agent/src/model_call_service.rs`
- tests for non-streaming adapter behavior

Mechanical steps:

1. Add `ModelStreamEvent` with started, content delta, usage, completed, failed,
   cancelled, and attempt failed variants.
2. Add a type alias for boxed model stream.
3. Add a default adapter that turns an existing `call()` response into:
   `Started`, one `ContentDelta`, optional `Usage`, `Completed`.
4. Add tests for success and failure mapping.

Do not:

- Do not implement provider-native SSE in this packet.
- Do not change ACP/chat consumers yet.

Verification:

```bash
cargo test -p roko-core model_stream
cargo test -p roko-agent model_stream
cargo check -p roko-agent
```

Acceptance:

- Non-streaming providers can be consumed through the stream shape.

## D5: Move One Provider Parser To UsageObservation Without Zero Fill

Context files:

- `tmp/subsystem-audits/05-01/20-learning-telemetry-redesign.md`
- `tmp/subsystem-audits/05-01/25-config-safety-telemetry-plan.md`

Write scope:

- one provider parser, preferably `crates/roko-agent/src/translate/openai.rs`
- local tests for usage parsing

Mechanical steps:

1. Add tests for missing usage block and explicit zero usage block.
2. Change parser output or add a new parser function returning `UsageObservation`.
3. Missing usage block must produce optional `None` fields with `UsageSource::Unknown`.
4. Explicit provider zero must produce `Some(0)` with `UsageSource::ProviderReported`.
5. Keep old `Usage` conversion only as a compatibility adapter if needed.

Do not:

- Do not convert unknown back to zero in the new parser.
- Do not migrate every provider in one packet.

Verification:

```bash
cargo test -p roko-agent parse_usage
cargo check -p roko-agent
```

Acceptance:

- Tests distinguish missing usage from explicit zero usage.

## D6: ACP Thin Stream Consumer Adapter Skeleton

Context files:

- `tmp/subsystem-audits/05-01/13-acp-provider-regression.md`
- `tmp/subsystem-audits/05-01/23-dispatch-streaming-migration-plan.md`

Write scope:

- `crates/roko-acp/src/bridge_events.rs`
- local tests

Mechanical steps:

1. Add a pure function mapping `ModelStreamEvent` to ACP cognitive/session events.
2. Add tests for content delta, completed, failed, usage, and attempt failed.
3. Do not wire live provider dispatch yet.

Do not:

- Do not add provider HTTP.
- Do not remove existing ACP dispatch until provider stream service is ready.

Verification:

```bash
cargo test -p roko-acp model_stream_event
cargo check -p roko-acp
```

Acceptance:

- ACP can map the shared stream shape without knowing provider wire format.

## D7: Block Production `dispatch_direct` Fallback

Context files:

- `tmp/subsystem-audits/05-01/14-chat-session-model-regression.md`
- `tmp/subsystem-audits/05-01/18-model-dispatch-redesign.md`

Write scope:

- `crates/roko-cli/src/unified.rs`
- `crates/roko-cli/src/dispatch_direct.rs`
- tests if present

Mechanical steps:

1. Find production fallback from `ChatAgentSession` or `ModelCallService` to
   `dispatch_direct`.
2. Replace silent fallback with a typed/user-visible error unless a dev-only feature
   is explicitly enabled.
3. Add or update tests so session init failure does not call raw dispatch in normal
   builds.

Do not:

- Do not delete the whole module unless all imports are already gone.
- Do not add another fallback path.

Verification:

```bash
cargo test -p roko-cli dispatch_direct
cargo check -p roko-cli
rg 'dispatch_direct::dispatch_prompt' crates/roko-cli/src
```

Acceptance:

- Normal one-shot/chat production path cannot silently downgrade to raw dispatch.
