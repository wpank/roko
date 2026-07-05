# Architecture Plan: Agent Runtime

**Source:** `tmp/architecture/02-agent-runtime.md`
**Generated:** 2026-04-25
**Source hash:** `adcfbf9f46b77ddb70cb5cb580bd20f5088317ac3d01048c58567f272ce27cdf`
**Section tasks:** 18
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
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-02-S001 | 1 | Agent runtime | [ ] | 9.8 |
| ARCH-02-S002 | 7 | The AgentRuntime struct | [ ] | 9.8 |
| ARCH-02-S003 | 40 | The run() loop | [ ] | 9.8 |
| ARCH-02-S004 | 77 | The 9-step pipeline | [ ] | 9.8 |
| ARCH-02-S005 | 96 | Three modes | [ ] | 9.8 |
| ARCH-02-S006 | 129 | Three timescales | [ ] | 9.8 |
| ARCH-02-S007 | 141 | T0/T1/T2 gating | [ ] | 9.8 |
| ARCH-02-S008 | 165 | T0 reflex execution | [ ] | 9.8 |
| ARCH-02-S009 | 200 | Adaptive clock algorithm | [ ] | 9.8 |
| ARCH-02-S010 | 263 | Cortical state persistence | [ ] | 9.8 |
| ARCH-02-S011 | 287 | Extension chain | [ ] | 9.8 |
| ARCH-02-S012 | 291 | Domain profiles | [ ] | 9.8 |
| ARCH-02-S013 | 363 | Acceptance criteria (added 2026-04-25) | [ ] | 9.8 |
| ARCH-02-S014 | 367 | AgentMode lifecycle | [ ] | 9.8 |
| ARCH-02-S015 | 375 | T0/T1/T2 gating | [ ] | 9.8 |
| ARCH-02-S016 | 383 | T0 reflex store | [ ] | 9.8 |
| ARCH-02-S017 | 393 | Adaptive clock | [ ] | 9.8 |
| ARCH-02-S018 | 401 | Cortical state persistence | [ ] | 9.8 |

## Tasks

### ARCH-02-S001 -- Agent runtime

**Source section:** `tmp/architecture/02-agent-runtime.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Agent runtime

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---
````

**Explicit detail extraction from this section:**

- Section word count: `14`
- Section hash: `dde85bd434baf72a6355dd156008c3214f9a3227afe262175b4148639abd3bb2`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

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
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "runtime|redesign|Specification|Part|INDEX|Extracted" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "runtime|redesign|Specification|Part|INDEX|Extracted" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
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
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S002 -- The AgentRuntime struct

**Source section:** `tmp/architecture/02-agent-runtime.md:7` through `39`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### The AgentRuntime struct

Every agent -- in-process or remote -- runs the same core loop.

```rust
pub struct AgentRuntime {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// Domain profile (user-defined string, e.g. "coding", "chain", "defi-trader").
    pub profile: DomainProfile,  // newtype over String
    /// Lifecycle mode.
    pub mode: AgentMode,
    /// The 9-step heartbeat pipeline.
    pipeline: TickPipeline,
    /// Cortical state: working memory, goals, beliefs, attention.
    cortical: CorticalState,
    /// Extension chain (ordered list of hooks).
    extensions: Vec<Box<dyn Extension>>,
    /// Inbound message queue.
    inbox: mpsc::Receiver<AgentMessage>,
    /// Handle to the centralized inference gateway.
    inference: InferenceHandle,
    /// Handle to the relay for presence and event publishing.
    relay: RelayHandle,
    /// Adaptive clock controlling tick frequency.
    clock: AdaptiveClock,
    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `116`
- Section hash: `4de51ae44e7d16007e99c43c33b5f90423b657b12dd350f02b8e69a65f9b9daa`

**Normative requirements and implementation claims:**
- ```rust pub struct AgentRuntime { /// Unique agent identifier. pub id: AgentId, /// Human-readable name. pub name: String, /// Domain profile (user-defined string, e.g. "coding", "chain", "defi-trader"). pub profile: DomainProfile, // newtype over String /// Lifecycle mode. pub mode: AgentMode, /// The 9-step heartbeat pipeline. pipeline: TickPipeline, /// Cortical state: working memory, goals, beliefs, attention. cortical: CorticalState, /// Extension chain (ordered list of hooks). extensions: Vec<Box<dyn Extension>>, /// Inbound message queue. inbox: mpsc::Receiver<AgentMessage>, /// Handle to the centralized inference gateway. inference: InferenceHandle, /// Handle to the relay for presence and event publishing. relay: RelayHandle, /// Adaptive clock controlling tick frequency. clock: AdaptiveClock, /// Cancellation token for graceful shutdown. cancel: CancellationToken, } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentRuntime

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
- Contract 1: language `rust`, first line `pub struct AgentRuntime {`

```rust
pub struct AgentRuntime {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// Domain profile (user-defined string, e.g. "coding", "chain", "defi-trader").
    pub profile: DomainProfile,  // newtype over String
    /// Lifecycle mode.
    pub mode: AgentMode,
    /// The 9-step heartbeat pipeline.
    pipeline: TickPipeline,
    /// Cortical state: working memory, goals, beliefs, attention.
    cortical: CorticalState,
    /// Extension chain (ordered list of hooks).
    extensions: Vec<Box<dyn Extension>>,
    /// Inbound message queue.
    inbox: mpsc::Receiver<AgentMessage>,
    /// Handle to the centralized inference gateway.
    inference: InferenceHandle,
    /// Handle to the relay for presence and event publishing.
    relay: RelayHandle,
    /// Adaptive clock controlling tick frequency.
    clock: AdaptiveClock,
    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
}
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "AgentRuntime|The|struct|runtime|Handle|relay|profile|pipeline" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "AgentRuntime|The|struct|runtime|Handle|relay|profile|pipeline" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `AgentRuntime` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S003 -- The run() loop

**Source section:** `tmp/architecture/02-agent-runtime.md:40` through `76`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### The run() loop

```rust
impl AgentRuntime {
    pub async fn run(mut self) -> AgentResult {
        self.relay.announce_presence(&self.id, &self.profile).await;

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => break,
                _ = self.clock.tick() => {
                    let result = self.pipeline.execute_tick(
                        &mut self.cortical,
                        &self.extensions,
                        &self.inference,
                    ).await;

                    self.relay.publish_heartbeat(&self.id, &result).await;

                    if result.should_stop() {
                        break;
                    }
                }
                msg = self.inbox.recv() => {
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
            }
        }

        self.relay.announce_leave(&self.id).await;
        self.cortical.into_result()
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `76`
- Section hash: `d4c434d9688e601ab0c8618e1a9e511bc6fea3fd9b7d33c5850b771db8d18e8c`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- run

**Event names and event-like entities:**
- self.relay.announce_presence
- self.id
- self.profile
- self.cancel.cancelled
- self.clock.tick
- self.pipeline.execute_tick
- self.cortical
- self.extensions
- self.inference
- self.relay.publish_heartbeat
- self.inbox.recv
- self.handle_message
- self.relay.announce_leave
- self.cortical.into_result

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- _ = self.cancel.cancelled() => break,
- _ = self.clock.tick() => {
- msg = self.inbox.recv() => {

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `impl AgentRuntime {`

```rust
impl AgentRuntime {
    pub async fn run(mut self) -> AgentResult {
        self.relay.announce_presence(&self.id, &self.profile).await;

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => break,
                _ = self.clock.tick() => {
                    let result = self.pipeline.execute_tick(
                        &mut self.cortical,
                        &self.extensions,
                        &self.inference,
                    ).await;

                    self.relay.publish_heartbeat(&self.id, &result).await;

                    if result.should_stop() {
                        break;
                    }
                }
                msg = self.inbox.recv() => {
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
            }
        }

        self.relay.announce_leave(&self.id).await;
        self.cortical.into_result()
    }
}
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "self|run|loop|result|await|The|msg|relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "self|run|loop|result|await|The|msg|relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `run` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `self.relay.announce_presence` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.id` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.profile` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.cancel.cancelled` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.clock.tick` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.pipeline.execute_tick` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.cortical` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.extensions` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.inference` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.relay.publish_heartbeat` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.inbox.recv` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.handle_message` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.relay.announce_leave` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.cortical.into_result` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `_ = self.cancel.cancelled() => break,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `_ = self.clock.tick() => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `msg = self.inbox.recv() => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S004 -- The 9-step pipeline

**Source section:** `tmp/architecture/02-agent-runtime.md:77` through `95`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### The 9-step pipeline

Each tick executes these steps in order. Extensions can intercept at each step.

```
Step        Name        What happens
────        ────        ────────────
1           Observe     Read inbox, check triggers, scan environment.
2           Retrieve    Query neuro store, load relevant context.
3           Analyze     Score observations, compute prediction error.
4           Gate        T0/T1/T2 decision. High PE → T2 (full reasoning).
                        Low PE → T0 (fast reflex). Budget exceeded → sleepwalk.
5           Simulate    If T1+: generate candidate actions, evaluate outcomes.
6           Validate    Safety checks, capability verification, budget guard.
7           Execute     Dispatch action (LLM call, tool use, message send).
8           Verify      Check execution result against predictions.
9           Reflect     Update cortical state, log episode, adjust clock.
```
````

**Explicit detail extraction from this section:**

- Section word count: `102`
- Section hash: `11162154d67b3e279265779f6f29f1cae8d99f3c4ed0edfd49a267edfa7b7a1c`

**Normative requirements and implementation claims:**
- ``` Step Name What happens ──── ──── ──────────── 1 Observe Read inbox, check triggers, scan environment. 2 Retrieve Query neuro store, load relevant context. 3 Analyze Score observations, compute prediction error. 4 Gate T0/T1/T2 decision. High PE → T2 (full reasoning). Low PE → T0 (fast reflex). Budget exceeded → sleepwalk. 5 Simulate If T1+: generate candidate actions, evaluate outcomes. 6 Validate Safety checks, capability verification, budget guard. 7 Execute Dispatch action (LLM call, tool use, message send). 8 Verify Check execution result against predictions. 9 Reflect Update cortical state, log episode, adjust clock. ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- T0/T1/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- High PE -> T2
- Low PE -> T0
- Budget exceeded -> sleepwalk

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Step        Name        What happens`

```
Step        Name        What happens
────        ────        ────────────
1           Observe     Read inbox, check triggers, scan environment.
2           Retrieve    Query neuro store, load relevant context.
3           Analyze     Score observations, compute prediction error.
4           Gate        T0/T1/T2 decision. High PE → T2 (full reasoning).
                        Low PE → T0 (fast reflex). Budget exceeded → sleepwalk.
5           Simulate    If T1+: generate candidate actions, evaluate outcomes.
6           Validate    Safety checks, capability verification, budget guard.
7           Execute     Dispatch action (LLM call, tool use, message send).
8           Verify      Check execution result against predictions.
9           Reflect     Update cortical state, log episode, adjust clock.
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `T0/T1/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "step|The|pipeline|check|prediction|action|Execute|Budget" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "step|The|pipeline|check|prediction|action|Execute|Budget" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `T0/T1/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`

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
- [ ] Enforce state transition `High PE -> T2` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Low PE -> T0` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Budget exceeded -> sleepwalk` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S005 -- Three modes

**Source section:** `tmp/architecture/02-agent-runtime.md:96` through `128`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Three modes

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Runs until task completes, then stops.
    Ephemeral,
    /// Runs continuously until manually stopped.
    Persistent,
    /// Sleeps until a trigger fires, wakes, works, sleeps again.
    Reactive,
}
```

**Ephemeral**: the default for task-oriented work. The agent receives a task, executes it through the pipeline, and shuts down when done. Use cases: coding tasks, one-off research, PR review.

**Persistent**: the agent runs its tick loop indefinitely. It processes messages from its inbox, monitors its environment, and maintains long-running state. Use cases: chain monitoring, continuous integration watchers, team coordinators.

**Reactive**: the agent registers triggers (webhooks, cron schedules, chain events, messages) and sleeps. When a trigger fires, the runtime wakes the agent, it processes the event through the full pipeline, then sleeps again. Zero compute cost while sleeping.

```toml
# roko.toml -- reactive agent example
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
  { type = "schedule", cron = "0 9 * * MON" },   # Monday morning sweep
]
```
````

**Explicit detail extraction from this section:**

- Section word count: `167`
- Section hash: `b4bd81c272da0d86de3925dbf3fe40578dfe8f5bf2e7abca80455b86bdd8b4c3`

**Normative requirements and implementation claims:**
- **Ephemeral**: the default for task-oriented work. The agent receives a task, executes it through the pipeline, and shuts down when done. Use cases: coding tasks, one-off research, PR review.
- **Persistent**: the agent runs its tick loop indefinitely. It processes messages from its inbox, monitors its environment, and maintains long-running state. Use cases: chain monitoring, continuous integration watchers, team coordinators.
- **Reactive**: the agent registers triggers (webhooks, cron schedules, chain events, messages) and sleeps. When a trigger fires, the runtime wakes the agent, it processes the event through the full pipeline, then sleeps again. Zero compute cost while sleeping.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentMode

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- name = "pr-reviewer"
- profile = "coding"
- mode = "reactive"
- triggers = [

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `#[derive(Debug, Clone, Copy, Serialize, Deserialize)]`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Runs until task completes, then stops.
    Ephemeral,
    /// Runs continuously until manually stopped.
    Persistent,
    /// Sleeps until a trigger fires, wakes, works, sleeps again.
    Reactive,
}
```
- Contract 2: language `toml`, first line `# roko.toml -- reactive agent example`

```toml
# roko.toml -- reactive agent example
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
  { type = "schedule", cron = "0 9 * * MON" },   # Monday morning sweep
]
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "mode|modes|Three|AgentMode|trigger|task|reactive|Sleeps" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mode|modes|Three|AgentMode|trigger|task|reactive|Sleeps" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `AgentMode` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `name = "pr-reviewer"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "coding"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `mode = "reactive"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `triggers = [` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S006 -- Three timescales

**Source section:** `tmp/architecture/02-agent-runtime.md:129` through `140`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Three timescales

The adaptive clock operates at three frequencies:

| Timescale | Name | Frequency | Purpose |
|-----------|------|-----------|---------|
| Gamma | Fast perception | 100ms - 1s | Reflex responses, environment scanning, heartbeat |
| Theta | Reflective planning | 5s - 30s | Reasoning, strategy adjustment, context retrieval |
| Delta | Deep consolidation | 1m - 10m | Memory consolidation, model updates, knowledge distillation |

The clock adapts based on prediction error and activity. High PE → faster ticks. Low PE → slower ticks. No activity → delta mode (conserve resources).
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `7c08d5dfba5e5308fe0b232df191e7ceeec8f61747ba691de71101620409bc06`

**Normative requirements and implementation claims:**
- | Timescale | Name | Frequency | Purpose | |-----------|------|-----------|---------| | Gamma | Fast perception | 100ms - 1s | Reflex responses, environment scanning, heartbeat | | Theta | Reflective planning | 5s - 30s | Reasoning, strategy adjustment, context retrieval | | Delta | Deep consolidation | 1m - 10m | Memory consolidation, model updates, knowledge distillation |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- High PE -> faster ticks
- Low PE -> slower ticks
- No activity -> delta mode

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Timescale | Name | Frequency | Purpose |
|-----------|------|-----------|---------|
| Gamma | Fast perception | 100ms - 1s | Reflex responses, environment scanning, heartbeat |
| Theta | Reflective planning | 5s - 30s | Reasoning, strategy adjustment, context retrieval |
| Delta | Deep consolidation | 1m - 10m | Memory consolidation, model updates, knowledge distillation |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Timescale|Three|timescales|ticks|mode|consolidation|clock|activity" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Timescale|Three|timescales|ticks|mode|consolidation|clock|activity" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Enforce state transition `High PE -> faster ticks` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Low PE -> slower ticks` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `No activity -> delta mode` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S007 -- T0/T1/T2 gating

**Source section:** `tmp/architecture/02-agent-runtime.md:141` through `164`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### T0/T1/T2 gating

Each tick decides how much reasoning to apply:

```
Input: prediction_error (PE), budget_remaining, cortical_urgency

T0 (reflex):     PE < 0.15 AND no urgent messages
                  → Skip steps 5-6, execute cached/habitual action
                  → Cost: ~0 tokens (no LLM call)

T1 (reflective): PE 0.15-0.40 OR moderate urgency
                  → Run steps 5-6 with lightweight model (Haiku)
                  → Cost: ~500 tokens

T2 (deliberate): PE > 0.40 OR high urgency OR novel situation
                  → Full pipeline with capable model (Sonnet/Opus)
                  → Cost: ~2000-8000 tokens

Sleepwalk:        Budget exhausted OR externally throttled
                  → Steps 1, 9 only (observe + reflect)
                  → Cost: 0 tokens
```
````

**Explicit detail extraction from this section:**

- Section word count: `94`
- Section hash: `738d0cde02c34882438866c06af22c88ad8b7488aa7bce6f83bfb75b280bf37f`

**Normative requirements and implementation claims:**
- Sleepwalk: Budget exhausted OR externally throttled → Steps 1, 9 only (observe + reflect) → Cost: 0 tokens ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- AND no urgent messages -> Skip steps 5-6
- habitual action -> Cost
- OR moderate urgency -> Run steps 5-6 with lightweight model
- OR high urgency OR novel situation -> Full pipeline with capable model
- Budget exhausted OR externally throttled -> Steps 1

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Input: prediction_error (PE), budget_remaining, cortical_urgency`

```
Input: prediction_error (PE), budget_remaining, cortical_urgency

T0 (reflex):     PE < 0.15 AND no urgent messages
                  → Skip steps 5-6, execute cached/habitual action
                  → Cost: ~0 tokens (no LLM call)

T1 (reflective): PE 0.15-0.40 OR moderate urgency
                  → Run steps 5-6 with lightweight model (Haiku)
                  → Cost: ~500 tokens

T2 (deliberate): PE > 0.40 OR high urgency OR novel situation
                  → Full pipeline with capable model (Sonnet/Opus)
                  → Cost: ~2000-8000 tokens

Sleepwalk:        Budget exhausted OR externally throttled
                  → Steps 1, 9 only (observe + reflect)
                  → Cost: 0 tokens
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "gating|tokens|Cost|urgency|steps|reflect|model|Budget" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "gating|tokens|Cost|urgency|steps|reflect|model|Budget" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `AND no urgent messages -> Skip steps 5-6` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `habitual action -> Cost` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `OR moderate urgency -> Run steps 5-6 with lightweight model` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `OR high urgency OR novel situation -> Full pipeline with capable model` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Budget exhausted OR externally throttled -> Steps 1` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S008 -- T0 reflex execution

**Source section:** `tmp/architecture/02-agent-runtime.md:165` through `199`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### T0 reflex execution

T0 skips inference entirely. Instead it runs a rule engine over a local reflex store.

**Reflex store**: `.roko/learn/reflexes.jsonl`. Each line is a condition-action pair learned from previous T2 sessions. When a T2 decision produced a correct outcome (gate passed, no rollback) and the same observation pattern recurs, the decision gets promoted to a reflex rule.

```json
{"condition":{"tool":"bash","args_pattern":"cargo test.*","context":"gate_check"},"action":{"tool":"bash","args":"cargo test --workspace"},"confidence":0.97,"source_episode":"ep_a1b2c3","promoted_at":"2026-04-20T14:30:00Z"}
{"condition":{"message_type":"pr_review_request","file_ext":".rs"},"action":{"tool":"file_read","args":"{path}"},"confidence":0.91,"source_episode":"ep_d4e5f6","promoted_at":"2026-04-21T09:15:00Z"}
{"condition":{"tool":"git","args_pattern":"git status","context":"pre_commit"},"action":{"tool":"bash","args":"cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings"},"confidence":0.95,"source_episode":"ep_g7h8i9","promoted_at":"2026-04-22T11:00:00Z"}
```

**Execution flow**:

```
Observation arrives
       │
       ▼
Match against reflexes.jsonl (linear scan, conditions checked in order)
       │
  match found ──────► Execute action directly (no LLM)
       │                     │
  no match                   ▼
       │              Record outcome, update confidence
       ▼
  Escalate to T1
```

**Promotion criteria**: A T2 decision becomes a T0 reflex when:
- The same observation pattern triggers the same action 3+ times
- Every execution passed its gate (zero failures)
- Confidence > 0.90 (computed as success_count / total_count)

**Demotion**: If a reflex action fails a gate, its confidence is halved. Below 0.50, the rule is deleted and future matches escalate to T1.
````

**Explicit detail extraction from this section:**

- Section word count: `230`
- Section hash: `4830a65c5f9f2571b3bfc4938e01b9c7e5d62c65d4238ee3e3a2b85e6657f6c5`

**Normative requirements and implementation claims:**
- **Reflex store**: `.roko/learn/reflexes.jsonl`. Each line is a condition-action pair learned from previous T2 sessions. When a T2 decision produced a correct outcome (gate passed, no rollback) and the same observation pattern recurs, the decision gets promoted to a reflex rule.
- ```json {"condition":{"tool":"bash","args_pattern":"cargo test.*","context":"gate_check"},"action":{"tool":"bash","args":"cargo test --workspace"},"confidence":0.97,"source_episode":"ep_a1b2c3","promoted_at":"2026-04-20T14:30:00Z"} {"condition":{"message_type":"pr_review_request","file_ext":".rs"},"action":{"tool":"file_read","args":"{path}"},"confidence":0.91,"source_episode":"ep_d4e5f6","promoted_at":"2026-04-21T09:15:00Z"} {"condition":{"tool":"git","args_pattern":"git status","context":"pre_commit"},"action":{"tool":"bash","args":"cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings"},"confidence":0.95,"source_episode":"ep_g7h8i9","promoted_at":"2026-04-22T11:00:00Z"} ```
- **Execution flow**:
- ``` Observation arrives │ ▼ Match against reflexes.jsonl (linear scan, conditions checked in order) │ match found ──────► Execute action directly (no LLM) │ │ no match ▼ │ Record outcome, update confidence ▼ Escalate to T1 ```
- **Promotion criteria**: A T2 decision becomes a T0 reflex when: - The same observation pattern triggers the same action 3+ times - Every execution passed its gate (zero failures) - Confidence > 0.90 (computed as success_count / total_count)
- **Demotion**: If a reflex action fails a gate, its confidence is halved. Below 0.50, the rule is deleted and future matches escalate to T1.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/learn/reflexes.json

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
- - The same observation pattern triggers the same action 3+ times
- - Every execution passed its gate (zero failures)
- - Confidence > 0.90 (computed as success_count / total_count)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{"condition":{"tool":"bash","args_pattern":"cargo test.*","context":"gate_check"},"action":{"tool":"bash","args":"cargo test --workspace"},"confidence":0.97,"so`

```json
{"condition":{"tool":"bash","args_pattern":"cargo test.*","context":"gate_check"},"action":{"tool":"bash","args":"cargo test --workspace"},"confidence":0.97,"source_episode":"ep_a1b2c3","promoted_at":"2026-04-20T14:30:00Z"}
{"condition":{"message_type":"pr_review_request","file_ext":".rs"},"action":{"tool":"file_read","args":"{path}"},"confidence":0.91,"source_episode":"ep_d4e5f6","promoted_at":"2026-04-21T09:15:00Z"}
{"condition":{"tool":"git","args_pattern":"git status","context":"pre_commit"},"action":{"tool":"bash","args":"cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings"},"confidence":0.95,"source_episode":"ep_g7h8i9","promoted_at":"2026-04-22T11:00:00Z"}
```
- Contract 2: language `plain`, first line `Observation arrives`

```
Observation arrives
       │
       ▼
Match against reflexes.jsonl (linear scan, conditions checked in order)
       │
  match found ──────► Execute action directly (no LLM)
       │                     │
  no match                   ▼
       │              Record outcome, update confidence
       ▼
  Escalate to T1
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/reflexes.json`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "reflex|execution|action|confidence|tool|condition|args|promoted" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reflex|execution|action|confidence|tool|condition|args|promoted" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/reflexes.json`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`

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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S009 -- Adaptive clock algorithm

**Source section:** `tmp/architecture/02-agent-runtime.md:200` through `262`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Adaptive clock algorithm

The clock adjusts tick frequency based on the agent's operating regime.

**Gamma interval** (fast perception tick):

```
gamma_interval = base_interval * regime_factor

base_interval = 500ms (configurable per agent)

Regime factors:
  Calm:     4.0x  →  2000ms between gamma ticks
  Normal:   1.0x  →   500ms between gamma ticks
  Volatile: 0.5x  →   250ms between gamma ticks
  Crisis:   0.25x →   125ms between gamma ticks
```

**Theta interval** (reflective planning tick):

```
theta_interval = N * gamma_interval

N varies by regime:
  Calm:     N = 8   →  16000ms (16s) between theta ticks
  Normal:   N = 5   →   2500ms (2.5s) between theta ticks
  Volatile: N = 3   →    750ms between theta ticks
  Crisis:   N = 2   →    250ms between theta ticks
```

**Delta interval** (deep consolidation tick):

Triggers on whichever comes first:
- `idle_timeout`: 60s of no observation activity (no new messages, no tool results)
- `episode_threshold`: 20 episodes accumulated since last delta tick

**Regime detection with hysteresis**:

```
                   ┌──────────────────────────────────────┐
                   │                                      │
                   ▼                                      │
              ┌─────────┐   PE > 0.40 for 3 ticks   ┌────┴────┐
     ┌───────►│  Calm    │─────────────────────────►│ Normal   │
     │        └─────────┘                            └────┬────┘
     │             ▲                                      │
     │   PE < 0.10 │  3 ticks                PE > 0.60   │  3 ticks
     │   3 ticks   │                          3 ticks    │
     │             │                                      ▼
     │        ┌────┴────┐                            ┌─────────┐
     │        │ Normal   │◄───────────────────────── │ Volatile │
     │        └─────────┘   PE < 0.30 for 3 ticks   └────┬────┘
     │                                                    │
     │                                       error_rate   │ > 0.5
     │                                       3 ticks      │
     │                                                    ▼
     │                                               ┌─────────┐
     └───────────────────────────────────────────────│ Crisis   │
               error_rate < 0.1 for 3 ticks          └─────────┘
```

The 3-tick hysteresis window prevents oscillation. A regime must persist for 3 consecutive gamma ticks before the clock adjusts. During the hysteresis window, the clock uses the previous regime's intervals.
````

**Explicit detail extraction from this section:**

- Section word count: `206`
- Section hash: `60c830d4315fafb2c5497d116dfabf8d612234d2f78ae7cb58793601a6a4ab57`

**Normative requirements and implementation claims:**
- **Gamma interval** (fast perception tick):
- **Theta interval** (reflective planning tick):
- **Delta interval** (deep consolidation tick):
- **Regime detection with hysteresis**:
- The 3-tick hysteresis window prevents oscillation. A regime must persist for 3 consecutive gamma ticks before the clock adjusts. During the hysteresis window, the clock uses the previous regime's intervals.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- idle_timeout
- episode_threshold

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- gamma_interval = base_interval * regime_factor
- base_interval = 500ms (configurable per agent)
- theta_interval = N * gamma_interval

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `idle_timeout`: 60s of no observation activity (no new messages, no tool results)
- - `episode_threshold`: 20 episodes accumulated since last delta tick

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `gamma_interval = base_interval * regime_factor`

```
gamma_interval = base_interval * regime_factor

base_interval = 500ms (configurable per agent)

Regime factors:
  Calm:     4.0x  →  2000ms between gamma ticks
  Normal:   1.0x  →   500ms between gamma ticks
  Volatile: 0.5x  →   250ms between gamma ticks
  Crisis:   0.25x →   125ms between gamma ticks
```
- Contract 2: language `plain`, first line `theta_interval = N * gamma_interval`

```
theta_interval = N * gamma_interval

N varies by regime:
  Calm:     N = 8   →  16000ms (16s) between theta ticks
  Normal:   N = 5   →   2500ms (2.5s) between theta ticks
  Volatile: N = 3   →    750ms between theta ticks
  Crisis:   N = 2   →    250ms between theta ticks
```
- Contract 3: language `plain`, first line `┌──────────────────────────────────────┐`

```
┌──────────────────────────────────────┐
                   │                                      │
                   ▼                                      │
              ┌─────────┐   PE > 0.40 for 3 ticks   ┌────┴────┐
     ┌───────►│  Calm    │─────────────────────────►│ Normal   │
     │        └─────────┘                            └────┬────┘
     │             ▲                                      │
     │   PE < 0.10 │  3 ticks                PE > 0.60   │  3 ticks
     │   3 ticks   │                          3 ticks    │
     │             │                                      ▼
     │        ┌────┴────┐                            ┌─────────┐
     │        │ Normal   │◄───────────────────────── │ Volatile │
     │        └─────────┘   PE < 0.30 for 3 ticks   └────┬────┘
     │                                                    │
     │                                       error_rate   │ > 0.5
     │                                       3 ticks      │
     │                                                    ▼
     │                                               ┌─────────┐
     └───────────────────────────────────────────────│ Crisis   │
               error_rate < 0.1 for 3 ticks          └─────────┘
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "tick|ticks|interval|clock|between|Gamma|regime|Theta" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tick|ticks|interval|clock|between|Gamma|regime|Theta" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `idle_timeout` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `episode_threshold` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `gamma_interval = base_interval * regime_factor` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `base_interval = 500ms (configurable per agent)` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `theta_interval = N * gamma_interval` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S010 -- Cortical state persistence

**Source section:** `tmp/architecture/02-agent-runtime.md:263` through `286`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cortical state persistence

Cortical state is serialized to `.roko/agents/{id}/cortical.json` on every theta tick.

```json
{
  "agent_id": "coder-1",
  "snapshot_at": "2026-04-24T14:32:10Z",
  "working_memory": [ ... ],
  "goals": [ ... ],
  "beliefs": { ... },
  "attention": { "focus": "implement auth middleware", "salience": 0.82 },
  "regime": "normal",
  "prediction_error_ema": 0.27,
  "episode_count": 142
}
```

**Restart behavior**:
- On agent startup, check for `.roko/agents/{id}/cortical.json`
- If the snapshot exists and is less than 1 hour old: load it and resume from the saved state
- If the snapshot is older than 1 hour: discard it, start with a fresh `CorticalState::default()`. Stale cortical state produces worse decisions than a cold start because goals, beliefs, and attention weights drift out of alignment with the actual environment.
- If no snapshot file exists: start fresh (first run)
````

**Explicit detail extraction from this section:**

- Section word count: `122`
- Section hash: `6269aeaf4c9badd603efb4f278008f2d44dd7013262ce03a31203158096816ba`

**Normative requirements and implementation claims:**
- Cortical state is serialized to `.roko/agents/{id}/cortical.json` on every theta tick.
- ```json { "agent_id": "coder-1", "snapshot_at": "2026-04-24T14:32:10Z", "working_memory": [ ... ], "goals": [ ... ], "beliefs": { ... }, "attention": { "focus": "implement auth middleware", "salience": 0.82 }, "regime": "normal", "prediction_error_ema": 0.27, "episode_count": 142 } ```
- **Restart behavior**: - On agent startup, check for `.roko/agents/{id}/cortical.json` - If the snapshot exists and is less than 1 hour old: load it and resume from the saved state - If the snapshot is older than 1 hour: discard it, start with a fresh `CorticalState::default()`. Stale cortical state produces worse decisions than a cold start because goals, beliefs, and attention weights drift out of alignment with the actual environment. - If no snapshot file exists: start fresh (first run)

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/agents/

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
- - On agent startup, check for `.roko/agents/{id}/cortical.json`
- - If the snapshot exists and is less than 1 hour old: load it and resume from the saved state
- - If the snapshot is older than 1 hour: discard it, start with a fresh `CorticalState::default()`. Stale cortical state produces worse decisions than a cold start because goals, beliefs, and attention weights drift out of alignment with the actual environment.
- - If no snapshot file exists: start fresh (first run)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{`

```json
{
  "agent_id": "coder-1",
  "snapshot_at": "2026-04-24T14:32:10Z",
  "working_memory": [ ... ],
  "goals": [ ... ],
  "beliefs": { ... },
  "attention": { "focus": "implement auth middleware", "salience": 0.82 },
  "regime": "normal",
  "prediction_error_ema": 0.27,
  "episode_count": 142
}
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/agents/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cortical|state|start|persistence|snapshot|json|hour|goals" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cortical|state|start|persistence|snapshot|json|hour|goals" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/agents/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S011 -- Extension chain

**Source section:** `tmp/architecture/02-agent-runtime.md:287` through `290`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extension chain

See [Extensions](03-extensions.md) for the full extension system, including the `Extension` trait, 8 layers, 22 hooks, domain profiles, and user-authored extensions.
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `4a109d97ed78cc48dd78d77ce0388e1d3c2421169b4c20049f18f1c36f5e967d`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Extension

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
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|chain|extensions|user|trait|profiles|layers|including" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|chain|extensions|user|trait|profiles|layers|including" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S012 -- Domain profiles

**Source section:** `tmp/architecture/02-agent-runtime.md:291` through `362`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Domain profiles

> **Not a standalone primitive (PRD 23).** Domain is a field on Agent, not a separate primitive in the 12-primitive vocabulary. The `DomainProfile` string below maps to the `archetype.domain` field on `ArchetypeManifest`, which bundles domain, tool profiles, gate pipelines, model preferences, and behavioral constraints into a single agent template. This aligns with the existing design -- `DomainProfile` is already a string field on `AgentRuntime`, not an independent object with its own lifecycle.

Domains are not hardcoded. A profile is just a string label with a default set of extensions and tools. Roko ships a handful of built-in profiles, but users create their own by declaring them in config or code. Any profile name is valid.

```rust
/// A domain profile is a user-defined string, not an enum.
/// Built-in profiles provide convenience defaults; custom profiles
/// are first-class and work identically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile(pub String);
```

Built-in profiles ship default extension sets as a convenience:

| Built-in profile | Default extensions | Default tools |
|---------|-----------|---------------|
| `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep |
| `research` | web-search, citation, summarizer | web_search, pdf_read, cite |
| `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |

But there is nothing special about these. A user can define any profile:

```toml
# Custom profile — no built-in knowledge needed
[[agents]]
name = "security-auditor"
profile = "security"        # user-defined, not in any enum
mode = "reactive"
extensions = ["code-scanner", "vuln-db", "report-writer"]
tools = ["grep", "ast_query", "file_read", "web_search"]
triggers = [{ type = "webhook", path = "/hooks/github-pr" }]

[[agents]]
name = "music-composer"
profile = "creative"        # another user-defined profile
mode = "persistent"
extensions = ["midi-gen", "audio-analysis", "feed-publisher"]
feeds = [
  { id = "ambient-soundscape", kind = "derived", schema = "audio_stream_v1", rate_hz = 1.0, access = "public" },
]
```

Profiles with no built-in defaults simply start with an empty extension chain -- the user specifies everything explicitly via `extensions` and `tools`. The extension system is plug-and-play: drop extension code into a known path, reference it by name in config.

Users can also publish profiles as shareable configs:

```toml
# ~/.roko/profiles/defi-trader.toml
[profile]
name = "defi-trader"
description = "DeFi trading agent with risk management and P&L tracking"
extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker", "feed-publisher"]
tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
default_mode = "persistent"
default_budget = { daily_limit_usd = 50.0 }
```

Then reference it:

```toml
[[agents]]
name = "my-trader"
profile = "defi-trader"   # loads from ~/.roko/profiles/defi-trader.toml
mode = "persistent"
```

Extensions themselves are also user-authored. The `Extension` trait (22 hooks, 8 layers) is the composition boundary -- implement the hooks you need, ignore the rest, and your extension plugs into any agent regardless of profile.

---
````

**Explicit detail extraction from this section:**

- Section word count: `432`
- Section hash: `f832df56f627f029e582b2109922f3a1f3111e952a277bd72940295041922c0c`

**Normative requirements and implementation claims:**
- Domains are not hardcoded. A profile is just a string label with a default set of extensions and tools. Roko ships a handful of built-in profiles, but users create their own by declaring them in config or code. Any profile name is valid.
- ```rust /// A domain profile is a user-defined string, not an enum. /// Built-in profiles provide convenience defaults; custom profiles /// are first-class and work identically. #[derive(Debug, Clone, Serialize, Deserialize)] pub struct DomainProfile(pub String); ```
- | Built-in profile | Default extensions | Default tools | |---------|-----------|---------------| | `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep | | `research` | web-search, citation, summarizer | web_search, pdf_read, cite | | `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |
- But there is nothing special about these. A user can define any profile:
- Extensions themselves are also user-authored. The `Extension` trait (22 hooks, 8 layers) is the composition boundary -- implement the hooks you need, ignore the rest, and your extension plugs into any agent regardless of profile.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/profiles/defi-trader.toml

**Types, functions, traits, and inline code identifiers:**
- DomainProfile
- mode
- ArchetypeManifest
- AgentRuntime
- coding
- research
- chain
- extensions
- tools
- Extension

**Event names and event-like entities:**
- archetype.domain

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- archetype.domain
- name = "security-auditor"
- profile = "security"        # user-defined, not in any enum
- mode = "reactive"
- extensions = ["code-scanner", "vuln-db", "report-writer"]
- tools = ["grep", "ast_query", "file_read", "web_search"]
- triggers = [{ type = "webhook", path = "/hooks/github-pr" }]
- name = "music-composer"
- profile = "creative"        # another user-defined profile
- mode = "persistent"
- extensions = ["midi-gen", "audio-analysis", "feed-publisher"]
- feeds = [
- [profile]
- name = "defi-trader"
- description = "DeFi trading agent with risk management and P&L tracking"
- extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker", "feed-publisher"]
- tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
- default_mode = "persistent"
- default_budget = { daily_limit_usd = 50.0 }
- name = "my-trader"
- profile = "defi-trader"   # loads from ~/.roko/profiles/defi-trader.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Built-in profile | Default extensions | Default tools |
|---------|-----------|---------------|
| `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep |
| `research` | web-search, citation, summarizer | web_search, pdf_read, cite |
| `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |
```

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// A domain profile is a user-defined string, not an enum.`

```rust
/// A domain profile is a user-defined string, not an enum.
/// Built-in profiles provide convenience defaults; custom profiles
/// are first-class and work identically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile(pub String);
```
- Contract 2: language `toml`, first line `# Custom profile — no built-in knowledge needed`

```toml
# Custom profile — no built-in knowledge needed
[[agents]]
name = "security-auditor"
profile = "security"        # user-defined, not in any enum
mode = "reactive"
extensions = ["code-scanner", "vuln-db", "report-writer"]
tools = ["grep", "ast_query", "file_read", "web_search"]
triggers = [{ type = "webhook", path = "/hooks/github-pr" }]

[[agents]]
name = "music-composer"
profile = "creative"        # another user-defined profile
mode = "persistent"
extensions = ["midi-gen", "audio-analysis", "feed-publisher"]
feeds = [
  { id = "ambient-soundscape", kind = "derived", schema = "audio_stream_v1", rate_hz = 1.0, access = "public" },
]
```
- Contract 3: language `toml`, first line `# ~/.roko/profiles/defi-trader.toml`

```toml
# ~/.roko/profiles/defi-trader.toml
[profile]
name = "defi-trader"
description = "DeFi trading agent with risk management and P&L tracking"
extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker", "feed-publisher"]
tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
default_mode = "persistent"
default_budget = { daily_limit_usd = 50.0 }
```
- Contract 4: language `toml`, first line `[[agents]]`

```toml
[[agents]]
name = "my-trader"
profile = "defi-trader"   # loads from ~/.roko/profiles/defi-trader.toml
mode = "persistent"
```

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/profiles/defi-trader.toml`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "profile|Extension|profiles|Domain|extensions|tools|mode|defi" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "profile|Extension|profiles|Domain|extensions|tools|mode|defi" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/profiles/defi-trader.toml`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`

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
- [ ] Implement or verify `DomainProfile` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `mode` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentRuntime` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `coding` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `research` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `chain` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `extensions` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tools` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `archetype.domain` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `archetype.domain` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "security-auditor"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "security"        # user-defined, not in any enum` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `mode = "reactive"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `extensions = ["code-scanner", "vuln-db", "report-writer"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tools = ["grep", "ast_query", "file_read", "web_search"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `triggers = [{ type = "webhook", path = "/hooks/github-pr" }]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "music-composer"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "creative"        # another user-defined profile` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `mode = "persistent"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `extensions = ["midi-gen", "audio-analysis", "feed-publisher"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `feeds = [` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[profile]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "defi-trader"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "DeFi trading agent with risk management and P&L tracking"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker", "feed-publisher"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `default_mode = "persistent"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `default_budget = { daily_limit_usd = 50.0 }` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "my-trader"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "defi-trader"   # loads from ~/.roko/profiles/defi-trader.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S013 -- Acceptance criteria (added 2026-04-25)

**Source section:** `tmp/architecture/02-agent-runtime.md:363` through `366`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Acceptance criteria (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase A.
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `90d10d829a751ed600340455b5dc3d2f190bac94f43ac2feb179a7b1d79c3829`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- tmp/architecture-plans/06-architecture-implementation.md

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
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `tmp/architecture-plans/06-architecture-implementation.md`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "criteria|added|Acceptance|plans|Phase|Backported|runtime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "criteria|added|Acceptance|plans|Phase|Backported|runtime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `tmp/architecture-plans/06-architecture-implementation.md`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`

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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S014 -- AgentMode lifecycle

**Source section:** `tmp/architecture/02-agent-runtime.md:367` through `374`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### AgentMode lifecycle

- [ ] Ephemeral agent stops after full task-gate-persist cycle completes (not on first response)
- [ ] Ephemeral timeout: 30 minutes of no completion → log warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800)
- [ ] Persistent agent runs tick loop indefinitely until manually stopped
- [ ] Reactive agent sleeps between triggers (zero CPU). Webhook trigger wakes within 100ms. Cron trigger fires on schedule.
- [ ] `roko agent status --name x` shows `sleeping` for reactive agents between triggers
````

**Explicit detail extraction from this section:**

- Section word count: `69`
- Section hash: `46f0e2c19f30ad616148eaf4a4ccebbf4f784b9fdd22d48100f8b5a8014da579`

**Normative requirements and implementation claims:**
- - [ ] Ephemeral agent stops after full task-gate-persist cycle completes (not on first response) - [ ] Ephemeral timeout: 30 minutes of no completion → log warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800) - [ ] Persistent agent runs tick loop indefinitely until manually stopped - [ ] Reactive agent sleeps between triggers (zero CPU). Webhook trigger wakes within 100ms. Cron trigger fires on schedule. - [ ] `roko agent status --name x` shows `sleeping` for reactive agents between triggers

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- sleeping

**Event names and event-like entities:**
- agent.ephemeral_timeout_secs

**State transitions:**
- minutes of no completion -> log warning and stop

**Config keys and TOML-like settings:**
- agent.ephemeral_timeout_secs

**Commands and operator actions:**
- roko agent status --name x

**Bullet requirements:**
- - [ ] Ephemeral agent stops after full task-gate-persist cycle completes (not on first response)
- - [ ] Ephemeral timeout: 30 minutes of no completion → log warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800)
- - [ ] Persistent agent runs tick loop indefinitely until manually stopped
- - [ ] Reactive agent sleeps between triggers (zero CPU). Webhook trigger wakes within 100ms. Cron trigger fires on schedule.
- - [ ] `roko agent status --name x` shows `sleeping` for reactive agents between triggers

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "cycle|sleeping|lifecycle|AgentMode|trigger|stop|Ephemeral|triggers" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cycle|sleeping|lifecycle|AgentMode|trigger|stop|Ephemeral|triggers" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `sleeping` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.ephemeral_timeout_secs` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `minutes of no completion -> log warning and stop` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `agent.ephemeral_timeout_secs` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `roko agent status --name x` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S015 -- T0/T1/T2 gating

**Source section:** `tmp/architecture/02-agent-runtime.md:375` through `382`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### T0/T1/T2 gating

- [ ] `decide_tier(0.10, 1000, 0.1)` returns T0
- [ ] `decide_tier(0.25, 1000, 0.5)` returns T1
- [ ] `decide_tier(0.50, 1000, 0.8)` returns T2
- [ ] `decide_tier(0.50, 0, 0.8)` returns Sleepwalk
- [ ] No hysteresis on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)
````

**Explicit detail extraction from this section:**

- Section word count: `47`
- Section hash: `16c7c61b658202999cf8d8c828d90d423e715232690345f1ede9c7d200870172`

**Normative requirements and implementation claims:**
- - [ ] `decide_tier(0.10, 1000, 0.1)` returns T0 - [ ] `decide_tier(0.25, 1000, 0.5)` returns T1 - [ ] `decide_tier(0.50, 1000, 0.8)` returns T2 - [ ] `decide_tier(0.50, 0, 0.8)` returns Sleepwalk - [ ] No hysteresis on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

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
- - [ ] `decide_tier(0.10, 1000, 0.1)` returns T0
- - [ ] `decide_tier(0.25, 1000, 0.5)` returns T1
- - [ ] `decide_tier(0.50, 1000, 0.8)` returns T2
- - [ ] `decide_tier(0.50, 0, 0.8)` returns Sleepwalk
- - [ ] No hysteresis on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "tier|gating|returns|decide_tier|hysteresis|tick|regime|fresh" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tier|gating|returns|decide_tier|hysteresis|tick|regime|fresh" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S016 -- T0 reflex store

**Source section:** `tmp/architecture/02-agent-runtime.md:383` through `392`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### T0 reflex store

- [ ] Reflex rule created after 3 identical T2 successes with zero gate failures
- [ ] T0 path matches rule and executes action without LLM call
- [ ] Gate failure halves reflex confidence
- [ ] Rule deleted when confidence < 0.50
- [ ] Mixed success/failure: confidence = success_count / total_count (running ratio)
- [ ] Max 200 rules, evict lowest confidence when full
- [ ] `.roko/learn/reflexes.jsonl` persists across restarts
````

**Explicit detail extraction from this section:**

- Section word count: `56`
- Section hash: `43d9a0578b2eec454bdeb688a9ca4ef11e9985ae34a007203bbb49d475e0a482`

**Normative requirements and implementation claims:**
- - [ ] Reflex rule created after 3 identical T2 successes with zero gate failures - [ ] T0 path matches rule and executes action without LLM call - [ ] Gate failure halves reflex confidence - [ ] Rule deleted when confidence < 0.50 - [ ] Mixed success/failure: confidence = success_count / total_count (running ratio) - [ ] Max 200 rules, evict lowest confidence when full - [ ] `.roko/learn/reflexes.jsonl` persists across restarts

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/learn/reflexes.json

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
- - [ ] Reflex rule created after 3 identical T2 successes with zero gate failures
- - [ ] T0 path matches rule and executes action without LLM call
- - [ ] Gate failure halves reflex confidence
- - [ ] Rule deleted when confidence < 0.50
- - [ ] Mixed success/failure: confidence = success_count / total_count (running ratio)
- - [ ] Max 200 rules, evict lowest confidence when full
- - [ ] `.roko/learn/reflexes.jsonl` persists across restarts

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/reflexes.json`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "reflex|store|rule|confidence|success|failure|gate|zero" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reflex|store|rule|confidence|success|failure|gate|zero" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/reflexes.json`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`

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
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S017 -- Adaptive clock

**Source section:** `tmp/architecture/02-agent-runtime.md:393` through `400`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Adaptive clock

- [ ] Regime changes only after 3 consecutive qualifying ticks (hysteresis)
- [ ] Oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change — counter resets on non-qualifying tick
- [ ] Gamma interval: base × regime_factor (Calm=4.0, Normal=1.0, Volatile=0.5, Crisis=0.25)
- [ ] Delta tick fires on 60s idle OR 20 episodes accumulated (whichever first)
- [ ] `base_interval` configurable via `agent.clock_base_ms` in roko.toml
````

**Explicit detail extraction from this section:**

- Section word count: `66`
- Section hash: `46377ac357355144f550434e52df8057504c0adddf61a697b8a84a90f3124149`

**Normative requirements and implementation claims:**
- - [ ] Regime changes only after 3 consecutive qualifying ticks (hysteresis) - [ ] Oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change — counter resets on non-qualifying tick - [ ] Gamma interval: base × regime_factor (Calm=4.0, Normal=1.0, Volatile=0.5, Crisis=0.25) - [ ] Delta tick fires on 60s idle OR 20 episodes accumulated (whichever first) - [ ] `base_interval` configurable via `agent.clock_base_ms` in roko.toml

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- base_interval

**Event names and event-like entities:**
- agent.clock_base_ms

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- agent.clock_base_ms

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - [ ] Regime changes only after 3 consecutive qualifying ticks (hysteresis)
- - [ ] Oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change — counter resets on non-qualifying tick
- - [ ] Gamma interval: base × regime_factor (Calm=4.0, Normal=1.0, Volatile=0.5, Crisis=0.25)
- - [ ] Delta tick fires on 60s idle OR 20 episodes accumulated (whichever first)
- - [ ] `base_interval` configurable via `agent.clock_base_ms` in roko.toml

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "clock|base_interval|Adaptive|tick|base|Regime|qualifying|interval" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "clock|base_interval|Adaptive|tick|base|Regime|qualifying|interval" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `base_interval` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.clock_base_ms` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `agent.clock_base_ms` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

### ARCH-02-S018 -- Cortical state persistence

**Source section:** `tmp/architecture/02-agent-runtime.md:401` through `406`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cortical state persistence

- [ ] Serialized to `.roko/agents/{id}/cortical.json` on every theta tick (not gamma)
- [ ] Snapshot < 1 hour old → loaded on restart
- [ ] Snapshot >= 1 hour old → discarded (stale beliefs hurt more than cold start)
- [ ] Working memory capped at 50 items (LRU eviction)
````

**Explicit detail extraction from this section:**

- Section word count: `40`
- Section hash: `abc90da7d68921e3cea93f11ccefc3acea5fabaf27eaaad06f2ddd4c7fdb9d7b`

**Normative requirements and implementation claims:**
- - [ ] Serialized to `.roko/agents/{id}/cortical.json` on every theta tick (not gamma) - [ ] Snapshot < 1 hour old → loaded on restart - [ ] Snapshot >= 1 hour old → discarded (stale beliefs hurt more than cold start) - [ ] Working memory capped at 50 items (LRU eviction)

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/agents/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- hour old -> loaded on restart
- hour old -> discarded

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - [ ] Serialized to `.roko/agents/{id}/cortical.json` on every theta tick (not gamma)
- - [ ] Snapshot < 1 hour old → loaded on restart
- - [ ] Snapshot >= 1 hour old → discarded (stale beliefs hurt more than cold start)
- - [ ] Working memory capped at 50 items (LRU eviction)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/02-agent-runtime.md`
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/agents/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cortical|state|persistence|start|hour|Snapshot|tick|theta" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cortical|state|persistence|start|hour|Snapshot|tick|theta" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/heartbeat_probes.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-agent/src/`
- `crates/roko-serve/src/routes/agents.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/agents/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `hour old -> loaded on restart` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `hour old -> discarded` with invalid-transition rejection tests and persistence/restart behavior.
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
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/02-agent-runtime
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

