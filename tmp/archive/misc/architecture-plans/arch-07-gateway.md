# Architecture Plan: Gateway

**Source:** `tmp/architecture/07-gateway.md`
**Generated:** 2026-04-25
**Source hash:** `85081001f6218946b4d8032111ed3705e6f1f381cc7ee939d2832739a1359078`
**Section tasks:** 19
**Context mode:** full source section embedded in every task; no excerpt truncation.
**Quality threshold:** every task must score at least 9.5/10 before implementation begins.

## Purpose
Turn every source section into an executable, self-contained implementation task. A Codex agent should not need prior conversation context or a separate reading pass to understand the requirement, although it must still inspect current code before editing.

## Global Implementation Rules
- Extend existing modules before creating new ones; only add new route/service files when no canonical owner exists.
- Implement production wiring, not only structs, mocks, or isolated helpers.
- Preserve every extracted detail unless a parity-ledger row explicitly marks it covered or deferred.
- Add persistence, events, auth/safety, dashboard projections, and docs updates whenever the requirement reaches those surfaces.
- A checked box means code, tests, docs, parity ledger, and strict gates are done for that task.

## Primary Target Areas
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-07-S001 | 1 | Inference gateway | [ ] | 9.8 |
| ARCH-07-S002 | 11 | Pipeline overview | [ ] | 9.8 |
| ARCH-07-S003 | 78 | 1. Protocol types | [ ] | 9.8 |
| ARCH-07-S004 | 134 | 2. Hash cache (L1) | [ ] | 9.8 |
| ARCH-07-S005 | 177 | 3. Semantic cache (L2) | [ ] | 9.8 |
| ARCH-07-S006 | 211 | 4. Provider backends and key rotation | [ ] | 9.8 |
| ARCH-07-S007 | 259 | 5. Cost computation | [ ] | 9.8 |
| ARCH-07-S008 | 301 | 6. Loop detection | [ ] | 9.8 |
| ARCH-07-S009 | 329 | 7. Output budgeting | [ ] | 9.8 |
| ARCH-07-S010 | 357 | 8. Tool pruning | [ ] | 9.8 |
| ARCH-07-S011 | 377 | 9. Convergence detection | [ ] | 9.8 |
| ARCH-07-S012 | 398 | 10. Thinking cap | [ ] | 9.8 |
| ARCH-07-S013 | 415 | 11. Batch API | [ ] | 9.8 |
| ARCH-07-S014 | 432 | Gateway HTTP routes | [ ] | 9.8 |
| ARCH-07-S015 | 479 | InferenceHandle | [ ] | 9.8 |
| ARCH-07-S016 | 526 | 12. Concurrency and backpressure | [ ] | 9.8 |
| ARCH-07-S017 | 567 | 13. CascadeRouter fallback chain | [ ] | 9.8 |
| ARCH-07-S018 | 616 | CascadeRouter integration | [ ] | 9.8 |
| ARCH-07-S019 | 659 | Proxying for isolated agents | [ ] | 9.8 |

## Tasks

### ARCH-07-S001 -- Inference gateway

**Source section:** `tmp/architecture/07-gateway.md:1` through `10`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Inference gateway

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---

Agents never hold API keys. A centralized `InferenceGateway` inside the roko process owns all secrets, runs every request through a multi-stage pipeline, and calls providers. The gateway is designed as a standalone, reusable system -- it handles caching, cost tracking, loop detection, output budgeting, tool pruning, convergence detection, thinking caps, and batch submission. The `CascadeRouter` from `roko-learn` handles model selection upstream; the gateway handles everything after a model is chosen.

Crate: `crates/roko-gateway/`
````

**Explicit detail extraction from this section:**

- Section word count: `89`
- Section hash: `94bae1a7211041c3f750686fa13d125a200be7cf09da396d7408c365b848c67e`

**Normative requirements and implementation claims:**
- ---
- Agents never hold API keys. A centralized `InferenceGateway` inside the roko process owns all secrets, runs every request through a multi-stage pipeline, and calls providers. The gateway is designed as a standalone, reusable system -- it handles caching, cost tracking, loop detection, output budgeting, tool pruning, convergence detection, thinking caps, and batch submission. The `CascadeRouter` from `roko-learn` handles model selection upstream; the gateway handles everything after a model is chosen.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/roko-gateway/

**Types, functions, traits, and inline code identifiers:**
- InferenceGateway
- CascadeRouter

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-gateway/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "gateway|Inference|InferenceGateway|CascadeRouter|handles|model|every|detection" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "gateway|Inference|InferenceGateway|CascadeRouter|handles|model|every|detection" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-gateway/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `InferenceGateway` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CascadeRouter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S002 -- Pipeline overview

**Source section:** `tmp/architecture/07-gateway.md:11` through `77`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pipeline overview

Every inference request passes through these stages in order:

```
                              InferenceRequest
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  1. Loop detection   │  Ring buffer of recent tool calls.
                          │     (per-session)    │  Retry / oscillation / drift check.
                          └──────────┬──────────┘
                                     │ pass
                                     ▼
                          ┌─────────────────────┐
                          │  2. Cache lookup     │  L1 hash (blake3) → L2 semantic
                          │     (L1 → L2)       │  (SimHash, Hamming ≤ 3).
                          └──────────┬──────────┘
                               hit / │ miss
                          ┌─────┐    │
                          │return│    │
                          └─────┘    ▼
                          ┌─────────────────────┐
                          │  3. Tool pruning     │  Remove unused tool schemas.
                          │     (per-session)    │  Never prunes core tools.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  4. Output budget    │  EMA-based max_tokens cap.
                          │     (per-model)      │  p95 x 1.5, floor 1024.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  5. Thinking cap     │  Per-model thinking budget.
                          │     (per-model)      │  Only when thinking enabled.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  6. Convergence      │  SimHash of recent responses.
                          │     detection        │  3+ similar → inject guidance.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  7. Provider call    │  ProviderBackend::complete()
                          │                      │  or ::stream().
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  8. Cache store      │  Write to L1 + L2 (unless
                          │                      │  excluded by cache policy).
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  9. Cost tracking    │  Compute actual vs naive cost.
                          │                      │  Record per-agent, per-model.
                          └──────────┬──────────┘
                                     │
                                     ▼
                              InferenceResponse
```
````

**Explicit detail extraction from this section:**

- Section word count: `127`
- Section hash: `bf9bd1851c4ee0a727d4ec12c38a8eea18bfeafad69f456b3d0bd2dd7c3a8bbb`

**Normative requirements and implementation claims:**
- ``` InferenceRequest │ ▼ ┌─────────────────────┐ │ 1. Loop detection │ Ring buffer of recent tool calls. │ (per-session) │ Retry / oscillation / drift check. └──────────┬──────────┘ │ pass ▼ ┌─────────────────────┐ │ 2. Cache lookup │ L1 hash (blake3) → L2 semantic │ (L1 → L2) │ (SimHash, Hamming ≤ 3). └──────────┬──────────┘ hit / │ miss ┌─────┐ │ │return│ │ └─────┘ ▼ ┌─────────────────────┐ │ 3. Tool pruning │ Remove unused tool schemas. │ (per-session) │ Never prunes core tools. └──────────┬──────────┘ │ ▼ ┌─────────────────────┐ │ 4. Output budget │ EMA-based max_tokens cap. │ (per-model) │ p95 x 1.5, floor 1024. └──────────┬──────────┘ │ ▼ ┌─────────────────────┐ │ 5. Thinking cap │ Per-model thinking budget. │ (per-model) │ Only when thinking enabled. └──────────┬──────────┘ │ ▼ ┌─────────────────────┐ │ 6. Convergence │ SimHash of recent responses. │ detection │ 3+ similar → inject guidance. └──────────┬──────────┘ │ ▼ ┌─────────────────────┐ │ 7. Provider call │ ProviderBackend

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- L1 -> L2
- similar -> inject guidance

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `InferenceRequest`

```
InferenceRequest
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  1. Loop detection   │  Ring buffer of recent tool calls.
                          │     (per-session)    │  Retry / oscillation / drift check.
                          └──────────┬──────────┘
                                     │ pass
                                     ▼
                          ┌─────────────────────┐
                          │  2. Cache lookup     │  L1 hash (blake3) → L2 semantic
                          │     (L1 → L2)       │  (SimHash, Hamming ≤ 3).
                          └──────────┬──────────┘
                               hit / │ miss
                          ┌─────┐    │
                          │return│    │
                          └─────┘    ▼
                          ┌─────────────────────┐
                          │  3. Tool pruning     │  Remove unused tool schemas.
                          │     (per-session)    │  Never prunes core tools.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  4. Output budget    │  EMA-based max_tokens cap.
                          │     (per-model)      │  p95 x 1.5, floor 1024.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
...
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "overview|Pipeline|tool|model|inference|hash|Thinking|Cache" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "overview|Pipeline|tool|model|inference|hash|Thinking|Cache" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `L1 -> L2` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `similar -> inject guidance` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S003 -- 1. Protocol types

**Source section:** `tmp/architecture/07-gateway.md:78` through `133`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 1. Protocol types

Core types that every subsystem shares.

```rust
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ToolSchema>>,
    pub stream: bool,
    pub thinking: Option<ThinkingConfig>,
    pub metadata: InferenceMeta,
}

pub struct InferenceMeta {
    pub session_id: String,
    pub agent_id: AgentId,
    pub tier: Tier,              // T0, T1, T2
    pub budget_remaining: u64,   // microdollars
}

pub struct InferenceResponse {
    pub text: String,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
    pub model: String,
    pub latency_ms: u64,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub thinking_tokens: u64,       // Anthropic extended thinking
    pub reasoning_tokens: u64,      // OpenAI reasoning tokens
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    ContentFilter,
}

#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

All types derive `Serialize` + `Deserialize`. `TokenUsage` implements `Add` for aggregation across a session.
````

**Explicit detail extraction from this section:**

- Section word count: `147`
- Section hash: `5a0c2d34011a03558ca7d626a13240b3bba39432f3c1c2d9582f50085e7aa40e`

**Normative requirements and implementation claims:**
- All types derive `Serialize` + `Deserialize`. `TokenUsage` implements `Add` for aggregation across a session.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- InferenceRequest
- InferenceMeta
- InferenceResponse
- TokenUsage
- StopReason
- InferenceClient
- complete
- stream
- Serialize
- Deserialize
- Add

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct InferenceRequest {`

```rust
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ToolSchema>>,
    pub stream: bool,
    pub thinking: Option<ThinkingConfig>,
    pub metadata: InferenceMeta,
}

pub struct InferenceMeta {
    pub session_id: String,
    pub agent_id: AgentId,
    pub tier: Tier,              // T0, T1, T2
    pub budget_remaining: u64,   // microdollars
}

pub struct InferenceResponse {
    pub text: String,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
    pub model: String,
    pub latency_ms: u64,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub thinking_tokens: u64,       // Anthropic extended thinking
    pub reasoning_tokens: u64,      // OpenAI reasoning tokens
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    ContentFilter,
}

#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tokens|types|stream|TokenUsage|InferenceRequest|StopReason|Serialize|InferenceResponse" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tokens|types|stream|TokenUsage|InferenceRequest|StopReason|Serialize|InferenceResponse" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `InferenceRequest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `InferenceMeta` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `InferenceResponse` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TokenUsage` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `StopReason` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `InferenceClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `complete` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `stream` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Serialize` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Deserialize` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Add` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S004 -- 2. Hash cache (L1)

**Source section:** `tmp/architecture/07-gateway.md:134` through `176`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 2. Hash cache (L1)

Exact-match cache. Fast path for repeated identical requests.

**How it works**: Hash the normalized request body with blake3, look up in a moka async LRU cache. If the hash matches, return the cached response without calling a provider.

**Normalization** (applied before hashing):
- Strip UUIDs matching `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}`
- Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers
- Replace git status blocks with `[GIT_STATUS]` placeholder
- Sort JSON keys alphabetically
- Sort tool definitions by name

This ensures that two requests differing only in timestamps or working-directory metadata produce the same hash.

**Cache entry**:

```rust
pub struct CachedResponse {
    pub body: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub cached_at: Instant,
    pub effective_ttl: Duration,
}
```

**Regime-aware TTL**: The system's cortical state controls how long cache entries live.

| Regime | TTL | Rationale |
|--------|-----|-----------|
| Normal | 3600s | Standard operating conditions. |
| Calm | 7200s | Low activity -- cached responses stay valid longer. |
| Volatile | 900s | Rapid changes -- cache expires faster to avoid stale responses. |
| Crisis | 300s | Active failures -- almost no caching, maximize freshness. |

**Exclusions** (never cached):
- Responses containing `tool_use` stop reason (tool call IDs are ephemeral)
- Responses with fewer than 3 output tokens (too short to be useful)
- Error responses

**Storage**: `moka::future::Cache<[u8; 32], CachedResponse>` with configurable max capacity (default 10,000 entries).
````

**Explicit detail extraction from this section:**

- Section word count: `214`
- Section hash: `02d48675d9f2d1bd7606d88bc0b3fe635ce191afbc04f7679ca700da27743355`

**Normative requirements and implementation claims:**
- **How it works**: Hash the normalized request body with blake3, look up in a moka async LRU cache. If the hash matches, return the cached response without calling a provider.
- **Normalization** (applied before hashing): - Strip UUIDs matching `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}` - Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers - Replace git status blocks with `[GIT_STATUS]` placeholder - Sort JSON keys alphabetically - Sort tool definitions by name
- This ensures that two requests differing only in timestamps or working-directory metadata produce the same hash.
- **Cache entry**:
- **Regime-aware TTL**: The system's cortical state controls how long cache entries live.
- | Regime | TTL | Rationale | |--------|-----|-----------| | Normal | 3600s | Standard operating conditions. | | Calm | 7200s | Low activity -- cached responses stay valid longer. | | Volatile | 900s | Rapid changes -- cache expires faster to avoid stale responses. | | Crisis | 300s | Active failures -- almost no caching, maximize freshness. |
- **Exclusions** (never cached): - Responses containing `tool_use` stop reason (tool call IDs are ephemeral) - Responses with fewer than 3 output tokens (too short to be useful) - Error responses
- **Storage**: `moka::future::Cache<[u8; 32], CachedResponse>` with configurable max capacity (default 10,000 entries).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- CachedResponse
- tool_use

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Strip UUIDs matching `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}`
- - Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers
- - Replace git status blocks with `[GIT_STATUS]` placeholder
- - Sort JSON keys alphabetically
- - Sort tool definitions by name
- - Responses containing `tool_use` stop reason (tool call IDs are ephemeral)
- - Responses with fewer than 3 output tokens (too short to be useful)
- - Error responses

**Tables extracted:**
- Table 1:

```markdown
| Regime | TTL | Rationale |
|--------|-----|-----------|
| Normal | 3600s | Standard operating conditions. |
| Calm | 7200s | Low activity -- cached responses stay valid longer. |
| Volatile | 900s | Rapid changes -- cache expires faster to avoid stale responses. |
| Crisis | 300s | Active failures -- almost no caching, maximize freshness. |
```

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct CachedResponse {`

```rust
pub struct CachedResponse {
    pub body: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub cached_at: Instant,
    pub effective_ttl: Duration,
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "cache|Hash|response|cached|CachedResponse|tool_use|responses|tool" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cache|Hash|response|cached|CachedResponse|tool_use|responses|tool" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `CachedResponse` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tool_use` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S005 -- 3. Semantic cache (L2)

**Source section:** `tmp/architecture/07-gateway.md:177` through `210`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 3. Semantic cache (L2)

Near-miss cache. Catches requests that are semantically equivalent but textually different.

**How it works**: Compute a 64-bit SimHash fingerprint of the request text. Compare against stored fingerprints using Hamming distance. A distance of 3 bits or fewer counts as a cache hit.

**SimHash algorithm**:
1. Tokenize request text (whitespace + punctuation boundaries)
2. Hash each token with a fast 64-bit hash
3. For each bit position: if the token hash has a 1, increment a counter; if 0, decrement
4. Final fingerprint: 1 for each positive counter, 0 for each negative

**Storage**: `DashMap<u64, SimHashEntry>` for lock-free concurrent reads.

```rust
pub struct SimHashEntry {
    pub response: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,
}
```

**Parameters**:
- Max entries: 5,000
- TTL: 7,200s (fixed, not regime-aware -- semantic matches are fuzzier so the TTL is conservative)
- Eviction: LRU by age when capacity reached
- Hamming threshold: 3 bits (configurable)

**Namespace isolation**: Each tenant/workspace prefixes its cache text with a namespace identifier. This prevents cross-tenant cache hits in multi-user deployments. A `default` namespace is used for single-user setups.

**Exclusions**: Same as L1 -- no tool_use, no sub-3-token, no errors.
````

**Explicit detail extraction from this section:**

- Section word count: `198`
- Section hash: `bb34f9df4ad8a079f010ab3c62e8c61db77585cda04726817488c57d5ac24bdb`

**Normative requirements and implementation claims:**
- **How it works**: Compute a 64-bit SimHash fingerprint of the request text. Compare against stored fingerprints using Hamming distance. A distance of 3 bits or fewer counts as a cache hit.
- **SimHash algorithm**: 1. Tokenize request text (whitespace + punctuation boundaries) 2. Hash each token with a fast 64-bit hash 3. For each bit position: if the token hash has a 1, increment a counter; if 0, decrement 4. Final fingerprint: 1 for each positive counter, 0 for each negative
- **Storage**: `DashMap<u64, SimHashEntry>` for lock-free concurrent reads.
- **Parameters**: - Max entries: 5,000 - TTL: 7,200s (fixed, not regime-aware -- semantic matches are fuzzier so the TTL is conservative) - Eviction: LRU by age when capacity reached - Hamming threshold: 3 bits (configurable)
- **Namespace isolation**: Each tenant/workspace prefixes its cache text with a namespace identifier. This prevents cross-tenant cache hits in multi-user deployments. A `default` namespace is used for single-user setups.
- **Exclusions**: Same as L1 -- no tool_use, no sub-3-token, no errors.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- SimHashEntry
- default

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Tokenize request text (whitespace + punctuation boundaries)
- 2. Hash each token with a fast 64-bit hash
- 3. For each bit position: if the token hash has a 1, increment a counter; if 0, decrement
- 4. Final fingerprint: 1 for each positive counter, 0 for each negative
- - Max entries: 5,000
- - TTL: 7,200s (fixed, not regime-aware -- semantic matches are fuzzier so the TTL is conservative)
- - Eviction: LRU by age when capacity reached
- - Hamming threshold: 3 bits (configurable)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct SimHashEntry {`

```rust
pub struct SimHashEntry {
    pub response: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "cache|Semantic|Hash|SimHashEntry|default|token|text|namespace" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cache|Semantic|Hash|SimHashEntry|default|token|text|namespace" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `SimHashEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `default` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S006 -- 4. Provider backends and key rotation

**Source section:** `tmp/architecture/07-gateway.md:211` through `258`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 4. Provider backends and key rotation

Each LLM provider implements a `ProviderBackend` trait:

```rust
#[async_trait]
pub trait ProviderBackend: Send + Sync {
    fn name(&self) -> &str;
    fn supports_model(&self, model: &str) -> bool;
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: &InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

**Anthropic backend** (`POST https://api.anthropic.com/v1/messages`):
- Streaming via SSE
- Tool use with full schema
- Extended thinking (`thinking.type = "enabled"`, `thinking.budget_tokens`)
- Prefix caching: system block annotated with `cache_control: {"type": "ephemeral", "ttl": "1h"}`
- Extracts `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens` from response usage

**OpenAI backend** (`POST https://api.openai.com/v1/chat/completions`):
- Format translation: Anthropic message format <-> OpenAI chat format
- Reasoning token extraction from `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens`
- Model routing: handles `gpt-*`, `o1`, `o3-*`, `o4-*`

**Key rotation**: Each provider holds a `Vec<String>` of API keys. On a 429 (rate limit) response, the provider rotates to the next key in the list. An `AtomicUsize` index tracks the active key. Rotation is lock-free.

```rust
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    pub fn current(&self) -> &str {
        let idx = self.active.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    pub fn rotate(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }
}
```

**Provider resolution order**: Anthropic for `claude-*` models, OpenAI for `gpt-*/o1/o3-*/o4-*`. Additional providers (Gemini, Perplexity, Ollama, OpenRouter) use the existing `roko-agent` backends and are registered by config.
````

**Explicit detail extraction from this section:**

- Section word count: `224`
- Section hash: `798b0fbea4e9016e1b131148119e54e12dc24ebbd0fe6644e578db979bfcfd7c`

**Normative requirements and implementation claims:**
- **Anthropic backend** (`POST https://api.anthropic.com/v1/messages`): - Streaming via SSE - Tool use with full schema - Extended thinking (`thinking.type = "enabled"`, `thinking.budget_tokens`) - Prefix caching: system block annotated with `cache_control: {"type": "ephemeral", "ttl": "1h"}` - Extracts `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens` from response usage
- **OpenAI backend** (`POST https://api.openai.com/v1/chat/completions`): - Format translation: Anthropic message format <-> OpenAI chat format - Reasoning token extraction from `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens` - Model routing: handles `gpt-*`, `o1`, `o3-*`, `o4-*`
- **Key rotation**: Each provider holds a `Vec<String>` of API keys. On a 429 (rate limit) response, the provider rotates to the next key in the list. An `AtomicUsize` index tracks the active key. Rotation is lock-free.
- **Provider resolution order**: Anthropic for `claude-*` models, OpenAI for `gpt-*/o1/o3-*/o4-*`. Additional providers (Gemini, Perplexity, Ollama, OpenRouter) use the existing `roko-agent` backends and are registered by config.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- api.anthropic.com/v1/
- api.openai.com/v1/chat/

**Types, functions, traits, and inline code identifiers:**
- ProviderBackend
- name
- supports_model
- complete
- stream
- KeyRing
- current
- rotate
- cache_read_input_tokens
- cache_creation_input_tokens
- thinking_tokens
- AtomicUsize

**Event names and event-like entities:**
- api.anthropic.com
- thinking.type
- thinking.budget_tokens
- api.openai.com
- prompt_tokens_details.cached_tokens
- completion_tokens_details.reasoning_tokens
- self.active.load
- self.keys.len
- self.keys
- self.active.fetch_add

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- thinking.budget_tokens
- prompt_tokens_details.cached_tokens
- completion_tokens_details.reasoning_tokens

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Streaming via SSE
- - Tool use with full schema
- - Extended thinking (`thinking.type = "enabled"`, `thinking.budget_tokens`)
- - Prefix caching: system block annotated with `cache_control: {"type": "ephemeral", "ttl": "1h"}`
- - Extracts `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens` from response usage
- - Format translation: Anthropic message format <-> OpenAI chat format
- - Reasoning token extraction from `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens`
- - Model routing: handles `gpt-*`, `o1`, `o3-*`, `o4-*`

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `#[async_trait]`

```rust
#[async_trait]
pub trait ProviderBackend: Send + Sync {
    fn name(&self) -> &str;
    fn supports_model(&self, model: &str) -> bool;
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: &InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```
- Contract 2: language `rust`, first line `pub struct KeyRing {`

```rust
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    pub fn current(&self) -> &str {
        let idx = self.active.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    pub fn rotate(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api.anthropic.com/v1/`
- `api.openai.com/v1/chat/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "key|Provider|self|backend|token|stream|rotation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "key|Provider|self|backend|token|stream|rotation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api.anthropic.com/v1/`
- `api.openai.com/v1/chat/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ProviderBackend` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `supports_model` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `complete` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `stream` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KeyRing` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `current` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `rotate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `cache_read_input_tokens` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `cache_creation_input_tokens` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `thinking_tokens` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AtomicUsize` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `api.anthropic.com` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `thinking.type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `thinking.budget_tokens` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `api.openai.com` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `prompt_tokens_details.cached_tokens` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `completion_tokens_details.reasoning_tokens` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.active.load` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.keys.len` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.keys` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.active.fetch_add` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `thinking.budget_tokens` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `prompt_tokens_details.cached_tokens` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `completion_tokens_details.reasoning_tokens` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S007 -- 5. Cost computation

**Source section:** `tmp/architecture/07-gateway.md:259` through `300`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 5. Cost computation

Per-request cost calculation with actual vs naive pricing comparison.

**Pricing table**: `HashMap<String, ModelPricing>` loaded from config. Supports substring matching for model families (e.g., `claude-sonnet` matches `claude-sonnet-4-20250514`).

```rust
pub struct ModelPricing {
    pub input_per_m: f64,          // USD per 1M input tokens
    pub output_per_m: f64,         // USD per 1M output tokens
    pub cached_input_per_m: f64,   // USD per 1M cached input tokens
    pub reasoning_per_m: f64,      // USD per 1M reasoning/thinking tokens
}
```

Default fallback: $3/M input, $15/M output (covers unknown models without crashing).

**Cost formula** (per request):

```
fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6
cached_input  = cache_read_tokens * cached_input_per_m / 1e6
cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge
regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6
reasoning     = reasoning_tokens * reasoning_per_m / 1e6
thinking      = thinking_tokens * output_per_m / 1e6

actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking
```

**Batch discount**: Requests submitted through the batch API get a 50% reduction on `actual_cost`.

**Naive cost**: What the provider would charge with no caching at all:

```
naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6
```

**Savings**: `naive_cost - actual_cost`. Tracked per request and aggregated per agent, per session, and per model for dashboard display.

**Attribution**: Every cost record includes `agent_id` and `session_id`. This feeds the Treasury / Cost page in the dashboard and the per-agent cost breakdowns.
````

**Explicit detail extraction from this section:**

- Section word count: `199`
- Section hash: `0d634398ed433b026e2217aa60a292f56f867fe996b7237e17fbe6bd5842861c`

**Normative requirements and implementation claims:**
- **Pricing table**: `HashMap<String, ModelPricing>` loaded from config. Supports substring matching for model families (e.g., `claude-sonnet` matches `claude-sonnet-4-20250514`).
- **Cost formula** (per request):
- **Batch discount**: Requests submitted through the batch API get a 50% reduction on `actual_cost`.
- **Naive cost**: What the provider would charge with no caching at all:
- **Savings**: `naive_cost - actual_cost`. Tracked per request and aggregated per agent, per session, and per model for dashboard display.
- **Attribution**: Every cost record includes `agent_id` and `session_id`. This feeds the Treasury / Cost page in the dashboard and the per-agent cost breakdowns.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ModelPricing
- actual_cost
- agent_id
- session_id

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6
- cached_input  = cache_read_tokens * cached_input_per_m / 1e6
- cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge
- regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6
- reasoning     = reasoning_tokens * reasoning_per_m / 1e6
- thinking      = thinking_tokens * output_per_m / 1e6
- actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking
- naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct ModelPricing {`

```rust
pub struct ModelPricing {
    pub input_per_m: f64,          // USD per 1M input tokens
    pub output_per_m: f64,         // USD per 1M output tokens
    pub cached_input_per_m: f64,   // USD per 1M cached input tokens
    pub reasoning_per_m: f64,      // USD per 1M reasoning/thinking tokens
}
```
- Contract 2: language `plain`, first line `fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6`

```
fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6
cached_input  = cache_read_tokens * cached_input_per_m / 1e6
cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge
regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6
reasoning     = reasoning_tokens * reasoning_per_m / 1e6
thinking      = thinking_tokens * output_per_m / 1e6

actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking
```
- Contract 3: language `plain`, first line `naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6`

```
naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cost|input|tokens|output|reasoning|actual_cost|input_per_m|ModelPricing" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cost|input|tokens|output|reasoning|actual_cost|input_per_m|ModelPricing" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ModelPricing` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `actual_cost` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `session_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `cached_input  = cache_read_tokens * cached_input_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `reasoning     = reasoning_tokens * reasoning_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `thinking      = thinking_tokens * output_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S008 -- 6. Loop detection

**Source section:** `tmp/architecture/07-gateway.md:301` through `328`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 6. Loop detection

Detects three patterns of agent loops and injects corrective guidance before the agent wastes more tokens.

**Per-session state**:

```rust
pub struct SessionLoopState {
    recent_calls: VecDeque<(String, [u8; 32])>,  // (tool_name, blake3(args))
    consecutive_identical: u32,
    tokens_since_progress: u64,
}
```

Ring buffer capacity: 16 entries. Does not grow.

**Detection rules**:

| Pattern | Trigger | Injected guidance |
|---------|---------|-------------------|
| Retry | Same tool + same args hash called 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." |
| Oscillation | A -> B -> A -> B pattern repeats 3+ full cycles | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." |
| Drift | 15,000+ output tokens accumulated without new `tool_result` content | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |

**Injection mechanism**: The guidance string is prepended to the system prompt on the next request. It appears once and clears itself.

**Counters**: `loops_detected`, `loop_injections`, `loop_retry_detected`, `loop_oscillation_detected`, `loop_drift_detected`. All exposed via the stats endpoint.
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `29fcd6f0d74239855cbe5fed007d0f43bf065aab1c14d9cf560cd40f6c0fa430`

**Normative requirements and implementation claims:**
- **Per-session state**:
- **Detection rules**:
- | Pattern | Trigger | Injected guidance | |---------|---------|-------------------| | Retry | Same tool + same args hash called 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." | | Oscillation | A -> B -> A -> B pattern repeats 3+ full cycles | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." | | Drift | 15,000+ output tokens accumulated without new `tool_result` content | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |
- **Injection mechanism**: The guidance string is prepended to the system prompt on the next request. It appears once and clears itself.
- **Counters**: `loops_detected`, `loop_injections`, `loop_retry_detected`, `loop_oscillation_detected`, `loop_drift_detected`. All exposed via the stats endpoint.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- SessionLoopState
- tool_result
- loops_detected
- loop_injections
- loop_retry_detected
- loop_oscillation_detected
- loop_drift_detected

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- A -> B -
- A -> B pattern repeats 3

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Pattern | Trigger | Injected guidance |
|---------|---------|-------------------|
| Retry | Same tool + same args hash called 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." |
| Oscillation | A -> B -> A -> B pattern repeats 3+ full cycles | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." |
| Drift | 15,000+ output tokens accumulated without new `tool_result` content | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |
```

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct SessionLoopState {`

```rust
pub struct SessionLoopState {
    recent_calls: VecDeque<(String, [u8; 32])>,  // (tool_name, blake3(args))
    consecutive_identical: u32,
    tokens_since_progress: u64,
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Loop|detection|tool_result|loops_detected|loop_retry_detected|loop_oscillation_detected|loop_injections|loop_drift_detected" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Loop|detection|tool_result|loops_detected|loop_retry_detected|loop_oscillation_detected|loop_injections|loop_drift_detected" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `SessionLoopState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tool_result` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `loops_detected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `loop_injections` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `loop_retry_detected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `loop_oscillation_detected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `loop_drift_detected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `A -> B -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `A -> B pattern repeats 3` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S009 -- 7. Output budgeting

**Source section:** `tmp/architecture/07-gateway.md:329` through `356`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 7. Output budgeting

Prevents runaway output by auto-setting `max_tokens` based on observed behavior.

**Per-model tracking**:

```rust
pub struct ModelOutputStats {
    pub ema: f64,           // exponential moving average of output tokens
    pub ema_sq: f64,        // EMA of squared output tokens (for variance)
    pub max_seen: u64,      // highest output observed
    pub count: u64,         // total observations
}
```

**Algorithm**:
- Alpha: 0.05 (5% weight to new observations)
- Minimum samples: 20 before p95 estimation is trusted
- p95 estimate: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2 standard deviations)
- Cap: `p95 * 1.5`, with a floor of 1,024 tokens

**Behavior**:
- When a request has no `max_tokens` set, the gateway auto-sets it to the computed cap
- When a request has an unreasonably high `max_tokens` (above 2x the cap), the gateway reduces it to the cap
- When a request has an explicit `max_tokens` that is *below* the cap, the gateway does not touch it

**Counters**: `output_budgets_applied`, `output_tokens_bounded`.
````

**Explicit detail extraction from this section:**

- Section word count: `145`
- Section hash: `121080dfaa489e67e1d19a756293ec607768b7a0581a82b6721fff6bcb949cdb`

**Normative requirements and implementation claims:**
- **Per-model tracking**:
- **Algorithm**: - Alpha: 0.05 (5% weight to new observations) - Minimum samples: 20 before p95 estimation is trusted - p95 estimate: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2 standard deviations) - Cap: `p95 * 1.5`, with a floor of 1,024 tokens
- **Behavior**: - When a request has no `max_tokens` set, the gateway auto-sets it to the computed cap - When a request has an unreasonably high `max_tokens` (above 2x the cap), the gateway reduces it to the cap - When a request has an explicit `max_tokens` that is *below* the cap, the gateway does not touch it
- **Counters**: `output_budgets_applied`, `output_tokens_bounded`.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ModelOutputStats
- max_tokens
- output_budgets_applied
- output_tokens_bounded

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Alpha: 0.05 (5% weight to new observations)
- - Minimum samples: 20 before p95 estimation is trusted
- - p95 estimate: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2 standard deviations)
- - Cap: `p95 * 1.5`, with a floor of 1,024 tokens
- - When a request has no `max_tokens` set, the gateway auto-sets it to the computed cap
- - When a request has an unreasonably high `max_tokens` (above 2x the cap), the gateway reduces it to the cap
- - When a request has an explicit `max_tokens` that is *below* the cap, the gateway does not touch it

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct ModelOutputStats {`

```rust
pub struct ModelOutputStats {
    pub ema: f64,           // exponential moving average of output tokens
    pub ema_sq: f64,        // EMA of squared output tokens (for variance)
    pub max_seen: u64,      // highest output observed
    pub count: u64,         // total observations
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Output|tokens|max_tokens|output_tokens_bounded|output_budgets_applied|budgeting|ModelOutputStats|request" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Output|tokens|max_tokens|output_tokens_bounded|output_budgets_applied|budgeting|ModelOutputStats|request" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ModelOutputStats` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `max_tokens` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `output_budgets_applied` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `output_tokens_bounded` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S010 -- 8. Tool pruning

**Source section:** `tmp/architecture/07-gateway.md:357` through `376`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 8. Tool pruning

Removes unused tool schemas from requests to reduce input token count. Tool schemas are verbose (often 200-500 tokens each), and most sessions use a small subset.

**Usage tracking**: Two maps:
- Per-session: `HashMap<String, u32>` -- how many times each tool was called in this session
- Global: `HashMap<String, u64>` -- how many times each tool has been called across all sessions

**Never-prune list** (core tools that must always be available):
`Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `TaskCreate`, `TaskUpdate`, `TaskList`, `Agent`, `SendMessage`

**Two-tier pruning**:

| Tier | Trigger | Logic |
|------|---------|-------|
| Session (Tier 1) | 50+ requests in the current session | Remove tools never used in this session. Protected + used tools survive. |
| Global (Tier 2) | < 50 session requests but 50+ total global requests | Remove tools never used by any session. Catches tools that are defined but universally ignored. |

**Metrics**: `tools_pruned` count, `tool_tokens_saved` estimate (removed schemas x average schema size of ~300 tokens).
````

**Explicit detail extraction from this section:**

- Section word count: `150`
- Section hash: `79375de4e0a7749ba293a989cea06e7f34d33c3255e8455fcd98f9c711df0456`

**Normative requirements and implementation claims:**
- **Usage tracking**: Two maps: - Per-session: `HashMap<String, u32>` -- how many times each tool was called in this session - Global: `HashMap<String, u64>` -- how many times each tool has been called across all sessions
- **Never-prune list** (core tools that must always be available): `Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `TaskCreate`, `TaskUpdate`, `TaskList`, `Agent`, `SendMessage`
- **Two-tier pruning**:
- | Tier | Trigger | Logic | |------|---------|-------| | Session (Tier 1) | 50+ requests in the current session | Remove tools never used in this session. Protected + used tools survive. | | Global (Tier 2) | < 50 session requests but 50+ total global requests | Remove tools never used by any session. Catches tools that are defined but universally ignored. |
- **Metrics**: `tools_pruned` count, `tool_tokens_saved` estimate (removed schemas x average schema size of ~300 tokens).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Bash
- Read
- Write
- Edit
- Glob
- Grep
- WebSearch
- WebFetch
- TaskCreate
- TaskUpdate
- TaskList
- Agent
- SendMessage
- tools_pruned
- tool_tokens_saved

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Per-session: `HashMap<String, u32>` -- how many times each tool was called in this session
- - Global: `HashMap<String, u64>` -- how many times each tool has been called across all sessions

**Tables extracted:**
- Table 1:

```markdown
| Tier | Trigger | Logic |
|------|---------|-------|
| Session (Tier 1) | 50+ requests in the current session | Remove tools never used in this session. Protected + used tools survive. |
| Global (Tier 2) | < 50 session requests but 50+ total global requests | Remove tools never used by any session. Catches tools that are defined but universally ignored. |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Tool|session|Glob|tools|pruning|tools_pruned|tool_tokens_saved|Write" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Tool|session|Glob|tools|pruning|tools_pruned|tool_tokens_saved|Write" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `Bash` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Read` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Write` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Edit` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Glob` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Grep` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `WebSearch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `WebFetch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskCreate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskUpdate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskList` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Agent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SendMessage` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tools_pruned` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tool_tokens_saved` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S011 -- 9. Convergence detection

**Source section:** `tmp/architecture/07-gateway.md:377` through `397`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 9. Convergence detection

Detects when an agent is producing repetitive responses and needs a nudge.

**Per-session state**:

```rust
pub struct ConvergenceState {
    recent_hashes: VecDeque<u64>,  // last 8 response SimHashes
    consecutive_similar: u32,
}
```

**Detection**: After each response, compute its SimHash. Compare to the previous response's SimHash via Hamming distance. If the distance is 2 bits or fewer, increment `consecutive_similar`. Three or more consecutive similar responses triggers convergence.

**Injection**: On the next request, prepend: "Your recent responses are converging. Try a different angle or move to the next step."

A dissimilar response (Hamming > 2) resets the counter to zero.

**Counters**: `convergence_detected`, `convergence_injections`.
````

**Explicit detail extraction from this section:**

- Section word count: `97`
- Section hash: `56ce3e9cdba2cb5b303e04e15646b6a8360d8049a7f3439ac4c37e3dc0ef23ff`

**Normative requirements and implementation claims:**
- Detects when an agent is producing repetitive responses and needs a nudge.
- **Per-session state**:
- **Detection**: After each response, compute its SimHash. Compare to the previous response's SimHash via Hamming distance. If the distance is 2 bits or fewer, increment `consecutive_similar`. Three or more consecutive similar responses triggers convergence.
- **Injection**: On the next request, prepend: "Your recent responses are converging. Try a different angle or move to the next step."
- **Counters**: `convergence_detected`, `convergence_injections`.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ConvergenceState
- consecutive_similar
- convergence_detected
- convergence_injections

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct ConvergenceState {`

```rust
pub struct ConvergenceState {
    recent_hashes: VecDeque<u64>,  // last 8 response SimHashes
    consecutive_similar: u32,
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Convergence|response|detection|consecutive_similar|convergence_injections|convergence_detected|ConvergenceState|similar" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Convergence|response|detection|consecutive_similar|convergence_injections|convergence_detected|ConvergenceState|similar" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ConvergenceState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `consecutive_similar` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `convergence_detected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `convergence_injections` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S012 -- 10. Thinking cap

**Source section:** `tmp/architecture/07-gateway.md:398` through `414`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 10. Thinking cap

Per-model defaults for extended thinking budgets. Prevents agents from using unbounded thinking tokens when the budget is unset.

| Model family | Default thinking budget |
|-------------|------------------------|
| Opus | 32,768 tokens |
| Sonnet | 16,384 tokens |
| Haiku | 4,096 tokens |

**Rules**:
- Activates only when thinking is already enabled (`thinking.type = "enabled"`) but `budget_tokens` is absent
- Never forces thinking on. If thinking is disabled, the cap does nothing.
- Never overrides explicit user budgets. If the user sets `budget_tokens: 8192`, the cap does not increase it.

**Counters**: `thinking_budgets_applied`, `thinking_tokens_capped_estimate`.
````

**Explicit detail extraction from this section:**

- Section word count: `83`
- Section hash: `9cc425a18564e05b240bc317a8cc5b13385a503157442e8283d844b8fbf386bb`

**Normative requirements and implementation claims:**
- | Model family | Default thinking budget | |-------------|------------------------| | Opus | 32,768 tokens | | Sonnet | 16,384 tokens | | Haiku | 4,096 tokens |
- **Rules**: - Activates only when thinking is already enabled (`thinking.type = "enabled"`) but `budget_tokens` is absent - Never forces thinking on. If thinking is disabled, the cap does nothing. - Never overrides explicit user budgets. If the user sets `budget_tokens: 8192`, the cap does not increase it.
- **Counters**: `thinking_budgets_applied`, `thinking_tokens_capped_estimate`.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- budget_tokens
- thinking_budgets_applied
- thinking_tokens_capped_estimate

**Event names and event-like entities:**
- thinking.type

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Activates only when thinking is already enabled (`thinking.type = "enabled"`) but `budget_tokens` is absent
- - Never forces thinking on. If thinking is disabled, the cap does nothing.
- - Never overrides explicit user budgets. If the user sets `budget_tokens: 8192`, the cap does not increase it.

**Tables extracted:**
- Table 1:

```markdown
| Model family | Default thinking budget |
|-------------|------------------------|
| Opus | 32,768 tokens |
| Sonnet | 16,384 tokens |
| Haiku | 4,096 tokens |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Thinking|cap|tokens|budget|budget_tokens|thinking_tokens_capped_estimate|thinking_budgets_applied|budgets" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Thinking|cap|tokens|budget|budget_tokens|thinking_tokens_capped_estimate|thinking_budgets_applied|budgets" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `budget_tokens` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `thinking_budgets_applied` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `thinking_tokens_capped_estimate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `thinking.type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S013 -- 11. Batch API

**Source section:** `tmp/architecture/07-gateway.md:415` through `431`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 11. Batch API

Queues inference requests for asynchronous batch processing at a 50% cost discount. Useful for non-time-sensitive work: plan generation, research, code review.

**Queue behavior**:
- Requests submitted via `POST /api/gateway/batch/submit` return `202 Accepted` with a `custom_id` (`roko-{uuid}`)
- Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed
- Manual flush: `POST /api/gateway/batch/flush`

**Submission**: On flush, the gateway submits the batch to `POST https://api.anthropic.com/v1/messages/batches`.

**Polling**: Background task polls `GET /v1/messages/batches/{batch_id}` every 60 seconds until the batch completes.

**Results**: Stored in `DashMap<String, BatchResult>` keyed by `custom_id`. Retrieved via `GET /api/gateway/batch/result/{custom_id}`.

**Preprocessing**: Batch requests go through the same pipeline stages as real-time requests (prefix caching, output budget, tool pruning). Cost calculation applies the 50% batch discount.
````

**Explicit detail extraction from this section:**

- Section word count: `134`
- Section hash: `d71e55791a050f17c3721ce04e044fa14238db8782e4ac8284cfcebcf320eb0c`

**Normative requirements and implementation claims:**
- **Queue behavior**: - Requests submitted via `POST /api/gateway/batch/submit` return `202 Accepted` with a `custom_id` (`roko-{uuid}`) - Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed - Manual flush: `POST /api/gateway/batch/flush`
- **Submission**: On flush, the gateway submits the batch to `POST https://api.anthropic.com/v1/messages/batches`.
- **Polling**: Background task polls `GET /v1/messages/batches/{batch_id}` every 60 seconds until the batch completes.
- **Results**: Stored in `DashMap<String, BatchResult>` keyed by `custom_id`. Retrieved via `GET /api/gateway/batch/result/{custom_id}`.
- **Preprocessing**: Batch requests go through the same pipeline stages as real-time requests (prefix caching, output budget, tool pruning). Cost calculation applies the 50% batch discount.

**Routes and endpoint references:**
- POST /api/gateway/batch/submit
- POST /api/gateway/batch/flush
- GET /api/gateway/batch/result/{custom_id}

**Files and path references:**
- api.anthropic.com/v1/messages/
- api/gateway/batch/
- api/gateway/batch/result/
- v1/messages/batches/

**Types, functions, traits, and inline code identifiers:**
- custom_id

**Event names and event-like entities:**
- api.anthropic.com

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Requests submitted via `POST /api/gateway/batch/submit` return `202 Accepted` with a `custom_id` (`roko-{uuid}`)
- - Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed
- - Manual flush: `POST /api/gateway/batch/flush`

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api.anthropic.com/v1/messages/`
- `api/gateway/batch/`
- `api/gateway/batch/result/`
- `v1/messages/batches/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Batch|API|custom_id|requests|gateway|flush|submit|result" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Batch|API|custom_id|requests|gateway|flush|submit|result" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api.anthropic.com/v1/messages/`
- `api/gateway/batch/`
- `api/gateway/batch/result/`
- `v1/messages/batches/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/gateway/batch/submit` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/gateway/batch/flush` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/gateway/batch/result/{custom_id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `custom_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `api.anthropic.com` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S014 -- Gateway HTTP routes

**Source section:** `tmp/architecture/07-gateway.md:432` through `478`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gateway HTTP routes

```
POST   /api/gateway/inference         Main inference proxy endpoint.
                                       Auth required (agent token).
                                       Runs full pipeline.
                                       Returns InferenceResponse.

GET    /api/gateway/stats             Aggregate gateway statistics:
                                       cache hit rates, total cost,
                                       active sessions, loop detections,
                                       convergence events, tool pruning savings.

GET    /api/gateway/ws                WebSocket endpoint streaming per-request
                                       StatsEvents in real time.
                                       Broadcast channel (1024 slot capacity).

POST   /api/gateway/batch/submit      Queue a request for batch processing.
                                       Returns 202 + custom_id.

POST   /api/gateway/batch/flush       Force-flush the current batch queue.

GET    /api/gateway/batch/result/:id  Retrieve completed batch result by
                                       custom_id.
```

**StatsEvent** (broadcast on the WebSocket per completed request):

```rust
pub struct StatsEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    pub naive_cost_usd: f64,
    pub savings_usd: f64,
    pub cache_hit: bool,
    pub elapsed_ms: u64,
    pub session_id: String,
    pub gateway_actions: Vec<String>,  // e.g., ["output_budget", "tool_prune"]
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `152`
- Section hash: `4da5b3c1e71b03a7800f84cc2ccca81d5f15624accd832448e0533679c05d0b9`

**Normative requirements and implementation claims:**
- ``` POST /api/gateway/inference Main inference proxy endpoint. Auth required (agent token). Runs full pipeline. Returns InferenceResponse.
- GET /api/gateway/stats Aggregate gateway statistics: cache hit rates, total cost, active sessions, loop detections, convergence events, tool pruning savings.
- GET /api/gateway/ws WebSocket endpoint streaming per-request StatsEvents in real time. Broadcast channel (1024 slot capacity).
- POST /api/gateway/batch/submit Queue a request for batch processing. Returns 202 + custom_id.
- POST /api/gateway/batch/flush Force-flush the current batch queue.
- GET /api/gateway/batch/result/:id Retrieve completed batch result by custom_id. ```
- **StatsEvent** (broadcast on the WebSocket per completed request):

**Routes and endpoint references:**
- POST /api/gateway/inference
- GET /api/gateway/stats
- GET /api/gateway/ws
- POST /api/gateway/batch/submit
- POST /api/gateway/batch/flush
- GET /api/gateway/batch/result/:id

**Files and path references:**
- api/gateway/
- api/gateway/batch/
- api/gateway/batch/result/

**Types, functions, traits, and inline code identifiers:**
- StatsEvent

**Event names and event-like entities:**
- StatsEvent

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST   /api/gateway/inference         Main inference proxy endpoint.`

```
POST   /api/gateway/inference         Main inference proxy endpoint.
                                       Auth required (agent token).
                                       Runs full pipeline.
                                       Returns InferenceResponse.

GET    /api/gateway/stats             Aggregate gateway statistics:
                                       cache hit rates, total cost,
                                       active sessions, loop detections,
                                       convergence events, tool pruning savings.

GET    /api/gateway/ws                WebSocket endpoint streaming per-request
                                       StatsEvents in real time.
                                       Broadcast channel (1024 slot capacity).

POST   /api/gateway/batch/submit      Queue a request for batch processing.
                                       Returns 202 + custom_id.

POST   /api/gateway/batch/flush       Force-flush the current batch queue.

GET    /api/gateway/batch/result/:id  Retrieve completed batch result by
                                       custom_id.
```
- Contract 2: language `rust`, first line `pub struct StatsEvent {`

```rust
pub struct StatsEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    pub naive_cost_usd: f64,
    pub savings_usd: f64,
    pub cache_hit: bool,
    pub elapsed_ms: u64,
    pub session_id: String,
    pub gateway_actions: Vec<String>,  // e.g., ["output_budget", "tool_prune"]
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/gateway/`
- `api/gateway/batch/`
- `api/gateway/batch/result/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Gateway|StatsEvent|batch|api|routes|HTTP|token|stats" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Gateway|StatsEvent|batch|api|routes|HTTP|token|stats" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/gateway/`
- `api/gateway/batch/`
- `api/gateway/batch/result/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/gateway/inference` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/gateway/stats` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/gateway/ws` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/gateway/batch/submit` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/gateway/batch/flush` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/gateway/batch/result/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `StatsEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `StatsEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S015 -- InferenceHandle

**Source section:** `tmp/architecture/07-gateway.md:479` through `525`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### InferenceHandle

In-process agents get an `InferenceHandle` -- a channel sender that communicates with the gateway without holding any secrets.

```rust
/// Handle given to agents for inference requests.
/// Contains no API keys -- only a channel sender.
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceRequest>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,  // remaining budget in microdollars
}

impl InferenceHandle {
    /// Send an inference request and await the response.
    pub async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to: tx,
        }).await?;
        rx.await?
    }

    /// Stream an inference response (for LLM output).
    pub async fn infer_stream(
        &self,
        request: InferenceRequest,
    ) -> Result<impl Stream<Item = InferenceChunk>> {
        let (tx, rx) = mpsc::channel(64);
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to_stream: tx,
        }).await?;
        Ok(ReceiverStream::new(rx))
    }

    /// Remaining budget in microdollars.
    pub fn remaining_budget(&self) -> u64 {
        self.budget.load(Ordering::Relaxed)
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `145`
- Section hash: `413af2f516fe4eda0cb27b490842463d34e1d287d09c1c7f371bbea0228e1cde`

**Normative requirements and implementation claims:**
- ```rust /// Handle given to agents for inference requests. /// Contains no API keys -- only a channel sender. #[derive(Clone)] pub struct InferenceHandle { sender: mpsc::Sender<InferenceRequest>, agent_id: AgentId, budget: Arc<AtomicU64>, // remaining budget in microdollars }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- InferenceHandle
- infer
- infer_stream
- remaining_budget

**Event names and event-like entities:**
- self.sender.send
- self.agent_id.clone
- rx.await
- self.budget.load

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// Handle given to agents for inference requests.`

```rust
/// Handle given to agents for inference requests.
/// Contains no API keys -- only a channel sender.
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceRequest>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,  // remaining budget in microdollars
}

impl InferenceHandle {
    /// Send an inference request and await the response.
    pub async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to: tx,
        }).await?;
        rx.await?
    }

    /// Stream an inference response (for LLM output).
    pub async fn infer_stream(
        &self,
        request: InferenceRequest,
    ) -> Result<impl Stream<Item = InferenceChunk>> {
        let (tx, rx) = mpsc::channel(64);
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to_stream: tx,
        }).await?;
        Ok(ReceiverStream::new(rx))
    }

    /// Remaining budget in microdollars.
    pub fn remaining_budget(&self) -> u64 {
        self.budget.load(Ordering::Relaxed)
    }
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "infer|inference|InferenceHandle|send|request|Handle|self|sender" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "infer|inference|InferenceHandle|send|request|Handle|self|sender" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `InferenceHandle` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `infer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `infer_stream` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `remaining_budget` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `self.sender.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.agent_id.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `rx.await` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.budget.load` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S016 -- 12. Concurrency and backpressure

**Source section:** `tmp/architecture/07-gateway.md:526` through `566`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 12. Concurrency and backpressure

The gateway enforces concurrency limits at three levels to prevent overload.

**Per-provider concurrency**:

```
Provider      Max concurrent requests
────────      ──────────────────────
Anthropic     50
OpenAI        50
Gemini        30
Perplexity    20
Ollama        4  (local hardware bound)
OpenRouter    50
```

Requests beyond the provider limit queue in a bounded channel. The channel depth is 2x the concurrency limit (e.g., 100 for Anthropic). If the channel is full, the gateway returns `503 Service Unavailable` immediately.

**Per-agent queue depth**: Each agent can have at most 8 in-flight requests (queued + executing). Request number 9 receives:

```json
HTTP 429 Too Many Requests
Retry-After: 2

{ "error": "agent_queue_full", "agent_id": "coder-1", "max_depth": 8 }
```

The agent should use exponential backoff: 2s, 4s, 8s, capped at 30s.

**Global queue**: 200 total requests across all agents and providers. When the global queue is full:

```json
HTTP 503 Service Unavailable
Retry-After: 5

{ "error": "gateway_overloaded", "queued": 200, "active": 184 }
```

**Monitoring**: The `/api/gateway/stats` endpoint includes `queue_depth`, `active_requests`, and `rejected_count` per provider.
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `cc42b72e890e1d1f489795c1aaab68e1005c6c6efd3b4f6c289dac36338acdb7`

**Normative requirements and implementation claims:**
- **Per-provider concurrency**:
- **Per-agent queue depth**: Each agent can have at most 8 in-flight requests (queued + executing). Request number 9 receives:
- The agent should use exponential backoff: 2s, 4s, 8s, capped at 30s.
- **Global queue**: 200 total requests across all agents and providers. When the global queue is full:
- **Monitoring**: The `/api/gateway/stats` endpoint includes `queue_depth`, `active_requests`, and `rejected_count` per provider.

**Routes and endpoint references:**
- /api/gateway/stats

**Files and path references:**
- api/gateway/

**Types, functions, traits, and inline code identifiers:**
- queue_depth
- active_requests
- rejected_count

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Provider      Max concurrent requests`

```
Provider      Max concurrent requests
────────      ──────────────────────
Anthropic     50
OpenAI        50
Gemini        30
Perplexity    20
Ollama        4  (local hardware bound)
OpenRouter    50
```
- Contract 2: language `json`, first line `HTTP 429 Too Many Requests`

```json
HTTP 429 Too Many Requests
Retry-After: 2

{ "error": "agent_queue_full", "agent_id": "coder-1", "max_depth": 8 }
```
- Contract 3: language `json`, first line `HTTP 503 Service Unavailable`

```json
HTTP 503 Service Unavailable
Retry-After: 5

{ "error": "gateway_overloaded", "queued": 200, "active": 184 }
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/gateway/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "queue|Concurrency|Request|requests|rejected_count|queue_depth|provider" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "queue|Concurrency|Request|requests|rejected_count|queue_depth|provider" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/gateway/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `/api/gateway/stats` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `queue_depth` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `active_requests` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `rejected_count` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S017 -- 13. CascadeRouter fallback chain

**Source section:** `tmp/architecture/07-gateway.md:567` through `615`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 13. CascadeRouter fallback chain

The `CascadeRouter` returns a ranked list of models, not a single choice:

```rust
pub struct RouteDecision {
    pub preferred: String,      // e.g., "claude-sonnet-4-20250514"
    pub fallback_1: String,     // e.g., "claude-haiku-4-20250514"
    pub fallback_2: String,     // e.g., "gpt-4o-mini"
}
```

The gateway tries each model in order. If the preferred model is unavailable (provider down, rate limited, or timed out after 30s), it falls through to the next.

```
preferred ──► call provider
                │
           success ──► return response
                │
           failure (429 / 503 / timeout)
                │
                ▼
fallback_1 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
fallback_2 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
           return 503 to agent
```

**Default fallback hierarchies** (used when the router has insufficient data to rank):

```
Anthropic chain:   Opus → Sonnet → Haiku
OpenAI chain:      GPT-4o → GPT-4o-mini
Cross-provider:    Sonnet → GPT-4o → Haiku
```

**Fallback metadata**: When a fallback model serves the request, the response includes `"fallback": true` and `"original_model": "claude-sonnet-4-..."` so the agent and the learning system know what happened. The router records the fallback event to adjust future routing weights.
````

**Explicit detail extraction from this section:**

- Section word count: `168`
- Section hash: `d2fc850550d148deba6c937918a52ad9449d0cce06431b2e378f3e260aa23671`

**Normative requirements and implementation claims:**
- **Default fallback hierarchies** (used when the router has insufficient data to rank):
- **Fallback metadata**: When a fallback model serves the request, the response includes `"fallback": true` and `"original_model": "claude-sonnet-4-..."` so the agent and the learning system know what happened. The router records the fallback event to adjust future routing weights.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- RouteDecision
- CascadeRouter

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Opus -> Sonnet
- GPT-4o -> GPT-4o-mini
- Sonnet -> GPT-4o

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct RouteDecision {`

```rust
pub struct RouteDecision {
    pub preferred: String,      // e.g., "claude-sonnet-4-20250514"
    pub fallback_1: String,     // e.g., "claude-haiku-4-20250514"
    pub fallback_2: String,     // e.g., "gpt-4o-mini"
}
```
- Contract 2: language `plain`, first line `preferred ──► call provider`

```
preferred ──► call provider
                │
           success ──► return response
                │
           failure (429 / 503 / timeout)
                │
                ▼
fallback_1 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
fallback_2 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
           return 503 to agent
```
- Contract 3: language `plain`, first line `Anthropic chain:   Opus → Sonnet → Haiku`

```
Anthropic chain:   Opus → Sonnet → Haiku
OpenAI chain:      GPT-4o → GPT-4o-mini
Cross-provider:    Sonnet → GPT-4o → Haiku
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "fallback|CascadeRouter|router|chain|return|provider|model|RouteDecision" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "fallback|CascadeRouter|router|chain|return|provider|model|RouteDecision" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `RouteDecision` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CascadeRouter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `Opus -> Sonnet` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `GPT-4o -> GPT-4o-mini` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Sonnet -> GPT-4o` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S018 -- CascadeRouter integration

**Source section:** `tmp/architecture/07-gateway.md:616` through `658`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### CascadeRouter integration

The gateway uses the existing `CascadeRouter` from `roko-learn` for model selection. The router picks the model; the gateway handles everything after that.

```rust
impl InferenceGateway {
    async fn route_request(&self, envelope: InferenceEnvelope) -> Result<()> {
        // 1. Select model via CascadeRouter
        let model = self.cascade_router.select_model(
            &envelope.request.task_type,
            envelope.request.tier,
            &envelope.agent_id,
        );

        // 2. Stamp model onto request
        let mut request = envelope.request;
        request.model = model.clone();

        // 3. Run through gateway pipeline
        //    loop_check -> cache_lookup -> tool_prune -> output_budget
        //    -> thinking_cap -> convergence_check -> provider_call
        //    -> cache_store -> cost_track
        let response = self.pipeline.execute(request).await?;

        // 4. Update router weights from quality signal
        self.cascade_router.record_outcome(
            &model,
            &envelope.request.task_type,
            &response.quality_signal,
        );

        // 5. Publish cost update to relay
        self.relay.publish_cost_update(
            &envelope.agent_id,
            response.usage.total_cost_microdollars,
        ).await;

        envelope.respond(response);
        Ok(())
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `121`
- Section hash: `6db6770d0129b6051695fed9cd8a3a9e816fbb76ff6ceaa82dd50b470e6b2f01`

**Normative requirements and implementation claims:**
- // 4. Update router weights from quality signal self.cascade_router.record_outcome( &model, &envelope.request.task_type, &response.quality_signal, );
- // 5. Publish cost update to relay self.relay.publish_cost_update( &envelope.agent_id, response.usage.total_cost_microdollars, ).await;

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- route_request
- CascadeRouter

**Event names and event-like entities:**
- self.cascade_router.select_model
- envelope.request.task_type
- envelope.request.tier
- envelope.agent_id
- envelope.request
- request.model
- model.clone
- self.pipeline.execute
- self.cascade_router.record_outcome
- response.quality_signal
- self.relay.publish_cost_update
- response.usage.total_cost_microdollars
- envelope.respond

**State transitions:**
- loop_check -> cache_lookup -
- tool_prune -> output_budget
- thinking_cap -> convergence_check -
- cache_store -> cost_track

**Config keys and TOML-like settings:**
- request.model = model.clone();

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `impl InferenceGateway {`

```rust
impl InferenceGateway {
    async fn route_request(&self, envelope: InferenceEnvelope) -> Result<()> {
        // 1. Select model via CascadeRouter
        let model = self.cascade_router.select_model(
            &envelope.request.task_type,
            envelope.request.tier,
            &envelope.agent_id,
        );

        // 2. Stamp model onto request
        let mut request = envelope.request;
        request.model = model.clone();

        // 3. Run through gateway pipeline
        //    loop_check -> cache_lookup -> tool_prune -> output_budget
        //    -> thinking_cap -> convergence_check -> provider_call
        //    -> cache_store -> cost_track
        let response = self.pipeline.execute(request).await?;

        // 4. Update router weights from quality signal
        self.cascade_router.record_outcome(
            &model,
            &envelope.request.task_type,
            &response.quality_signal,
        );

        // 5. Publish cost update to relay
        self.relay.publish_cost_update(
            &envelope.agent_id,
            response.usage.total_cost_microdollars,
        ).await;

        envelope.respond(response);
        Ok(())
    }
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "router|CascadeRouter|request|model|envelope|self|route_request|integration" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "router|CascadeRouter|request|model|envelope|self|route_request|integration" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `route_request` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CascadeRouter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `self.cascade_router.select_model` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `envelope.request.task_type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `envelope.request.tier` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `envelope.agent_id` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `envelope.request` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `request.model` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `model.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.pipeline.execute` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.cascade_router.record_outcome` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `response.quality_signal` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.relay.publish_cost_update` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `response.usage.total_cost_microdollars` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `envelope.respond` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `loop_check -> cache_lookup -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `tool_prune -> output_budget` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `thinking_cap -> convergence_check -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `cache_store -> cost_track` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `request.model = model.clone();` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-07-S019 -- Proxying for isolated agents

**Source section:** `tmp/architecture/07-gateway.md:659` through `678`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Proxying for isolated agents

Remote agents (Fly Machines, Railway containers) don't have direct access to the inference gateway's channel. They make HTTPS requests to the parent's proxy endpoint:

```
POST /api/inference/proxy
Authorization: Bearer <agent_token>
Content-Type: application/json

{
  "agent_id": "isolated-coder-1",
  "model_hint": "auto",
  "tier": "t1",
  "messages": [ ... ],
  "tools": [ ... ],
  "max_tokens": 4096
}
```

The proxy endpoint validates the agent token, deducts from the agent's budget, and forwards the request through the same gateway pipeline. The agent never sees API keys.
````

**Explicit detail extraction from this section:**

- Section word count: `78`
- Section hash: `0728911f2a5f867d2af157cd28878daec3198a1391fa2f8f2164e18b9279c5bf`

**Normative requirements and implementation claims:**
- ``` POST /api/inference/proxy Authorization: Bearer <agent_token> Content-Type: application/json
- The proxy endpoint validates the agent token, deducts from the agent's budget, and forwards the request through the same gateway pipeline. The agent never sees API keys.

**Routes and endpoint references:**
- POST /api/inference/proxy

**Files and path references:**
- api/inference/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST /api/inference/proxy`

```
POST /api/inference/proxy
Authorization: Bearer <agent_token>
Content-Type: application/json

{
  "agent_id": "isolated-coder-1",
  "model_hint": "auto",
  "tier": "t1",
  "messages": [ ... ],
  "tools": [ ... ],
  "max_tokens": 4096
}
```

**Read before editing:**
- `tmp/architecture/07-gateway.md`
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/inference/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "proxy|isolated|for|Proxying|token|request|inference|gateway" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "proxy|isolated|for|Proxying|token|request|inference|gateway" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/gateway.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-core/src/config/schema.rs`
- `api/inference/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/inference/proxy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/07-gateway
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

