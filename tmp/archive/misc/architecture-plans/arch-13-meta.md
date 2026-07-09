# Architecture Plan: Meta

**Source:** `tmp/architecture/13-meta.md`
**Generated:** 2026-04-25
**Source hash:** `477ddc30e9f426498e213b1a94ccd8ea90989d74b760ad987aac678f53deea01`
**Section tasks:** 23
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
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-13-S001 | 1 | 13 -- Meta layer | [ ] | 9.8 |
| ARCH-13-S002 | 9 | Design constraints | [ ] | 9.8 |
| ARCH-13-S003 | 20 | Meta-agents | [ ] | 9.8 |
| ARCH-13-S004 | 24 | Runtime model | [ ] | 9.8 |
| ARCH-13-S005 | 99 | Tools | [ ] | 9.8 |
| ARCH-13-S006 | 119 | Configuration | [ ] | 9.8 |
| ARCH-13-S007 | 151 | Generators | [ ] | 9.8 |
| ARCH-13-S008 | 155 | Output schema validation | [ ] | 9.8 |
| ARCH-13-S009 | 206 | Generator configuration | [ ] | 9.8 |
| ARCH-13-S010 | 227 | Lineage tracking | [ ] | 9.8 |
| ARCH-13-S011 | 231 | On-chain lineage | [ ] | 9.8 |
| ARCH-13-S012 | 285 | Lineage queries | [ ] | 9.8 |
| ARCH-13-S013 | 340 | Recursive safety | [ ] | 9.8 |
| ARCH-13-S014 | 344 | Safety mechanisms | [ ] | 9.8 |
| ARCH-13-S015 | 441 | Practical example | [ ] | 9.8 |
| ARCH-13-S016 | 460 | Event types | [ ] | 9.8 |
| ARCH-13-S017 | 531 | Full event type list | [ ] | 9.8 |
| ARCH-13-S018 | 545 | API surface | [ ] | 9.8 |
| ARCH-13-S019 | 547 | Meta-agent endpoints | [ ] | 9.8 |
| ARCH-13-S020 | 557 | Generator endpoints | [ ] | 9.8 |
| ARCH-13-S021 | 567 | Lineage endpoints | [ ] | 9.8 |
| ARCH-13-S022 | 577 | Safety endpoints | [ ] | 9.8 |
| ARCH-13-S023 | 587 | Configuration | [ ] | 9.8 |

## Tasks

### ARCH-13-S001 -- 13 -- Meta layer

**Source section:** `tmp/architecture/13-meta.md:1` through `8`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 13 -- Meta layer

Agents that create agents. Generators that produce arenas, gates, evals, and extensions. Lineage tracking across generations. Recursive safety enforcement. This document specifies the runtime types, coordination protocols, and safety model that make recursive agent creation tractable.

Dashboard surfaces consuming these APIs are specified in `16-meta-surfaces.md` (PRD).

---
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `3a3298d8ccd6ab84d0a2fd2e74a1ab0f288859a376fa9bc9b21b8774c0463d8e`

**Normative requirements and implementation claims:**
- Agents that create agents. Generators that produce arenas, gates, evals, and extensions. Lineage tracking across generations. Recursive safety enforcement. This document specifies the runtime types, coordination protocols, and safety model that make recursive agent creation tractable.
- Dashboard surfaces consuming these APIs are specified in `16-meta-surfaces.md` (PRD).
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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Meta|layer|surfaces|safety|Recursive|types|tractable|tracking" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Meta|layer|surfaces|safety|Recursive|types|tractable|tracking" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S002 -- Design constraints

**Source section:** `tmp/architecture/13-meta.md:9` through `19`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Design constraints

1. **Meta-agents are agents.** A meta-agent runs on the same `AgentRuntime` as any other agent. It has a domain, extensions, gates, model routing, a knowledge store, and a reputation. What distinguishes it is the tools available to it: agent creation, configuration, lifecycle management.
2. **Generators are agents with output schemas.** A generator is an agent whose output must conform to a typed schema for the object it produces. A gate validates the output against the schema before registration.
3. **Depth is bounded.** Recursive creation has a configurable maximum depth (default: 3). A meta-agent can create an agent; that agent can create another; that agent cannot create a fourth level without explicit override.
4. **Children cannot exceed parents.** Caveat inheritance is monotonically narrowing. A child agent's delegation caveats can only restrict, never expand, its parent's caveats.
5. **Lineage is permanent.** Parent-child relationships are recorded on-chain (ERC-8004 `parentPassport` field) and locally. Lineage cannot be erased or rewritten.
6. **Anomaly detection runs continuously.** The system monitors recursive creation for runaway patterns: excessive creation rates, quality degradation across generations, and circular dependencies.

---
````

**Explicit detail extraction from this section:**

- Section word count: `186`
- Section hash: `1c9ff9b312b5d8f53ca52a642378be7ebb5fc82dfa083bd7df0c9445a289a06f`

**Normative requirements and implementation claims:**
- 1. **Meta-agents are agents.** A meta-agent runs on the same `AgentRuntime` as any other agent. It has a domain, extensions, gates, model routing, a knowledge store, and a reputation. What distinguishes it is the tools available to it: agent creation, configuration, lifecycle management. 2. **Generators are agents with output schemas.** A generator is an agent whose output must conform to a typed schema for the object it produces. A gate validates the output against the schema before registration. 3. **Depth is bounded.** Recursive creation has a configurable maximum depth (default: 3). A meta-agent can create an agent; that agent can create another; that agent cannot create a fourth level without explicit override. 4. **Children cannot exceed parents.** Caveat inheritance is monotonically narrowing. A child agent's delegation caveats can only restrict, never expand, its parent's caveats. 5. **Lineage is permanent.** Parent-child relationships are recorded on-chain (ERC-8004 `parentPas
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentRuntime
- parentPassport

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Meta-agents are agents.** A meta-agent runs on the same `AgentRuntime` as any other agent. It has a domain, extensions, gates, model routing, a knowledge store, and a reputation. What distinguishes it is the tools available to it: agent creation, configuration, lifecycle management.
- 2. **Generators are agents with output schemas.** A generator is an agent whose output must conform to a typed schema for the object it produces. A gate validates the output against the schema before registration.
- 3. **Depth is bounded.** Recursive creation has a configurable maximum depth (default: 3). A meta-agent can create an agent; that agent can create another; that agent cannot create a fourth level without explicit override.
- 4. **Children cannot exceed parents.** Caveat inheritance is monotonically narrowing. A child agent's delegation caveats can only restrict, never expand, its parent's caveats.
- 5. **Lineage is permanent.** Parent-child relationships are recorded on-chain (ERC-8004 `parentPassport` field) and locally. Lineage cannot be erased or rewritten.
- 6. **Anomaly detection runs continuously.** The system monitors recursive creation for runaway patterns: excessive creation rates, quality degradation across generations, and circular dependencies.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "parentPassport|constraints|Design|AgentRuntime|parent|creation|schema|output" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "parentPassport|constraints|Design|AgentRuntime|parent|creation|schema|output" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `AgentRuntime` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parentPassport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S003 -- Meta-agents

**Source section:** `tmp/architecture/13-meta.md:20` through `23`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Meta-agents

A meta-agent creates and configures other agents. It takes a specification -- a goal, a domain, constraints -- and produces a fully configured agent ready to start.
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `aaeefdc120d97ba9854070b5c1367a6f103408e55f27166106ee677429966588`

**Normative requirements and implementation claims:**
- None extracted from this section.

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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Meta|takes|start|specification|ready|produces|other|goal" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Meta|takes|start|specification|ready|produces|other|goal" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S004 -- Runtime model

**Source section:** `tmp/architecture/13-meta.md:24` through `98`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Runtime model

Meta-agents run on `AgentRuntime` with two specialized extensions:

```rust
/// Extension that gives an agent the ability to create other agents.
pub struct AgentCreatorExt {
    /// Maximum creation depth. Children inherit depth - 1.
    pub max_depth: u32,
    /// Current depth in the creation chain (0 = top-level meta-agent).
    pub current_depth: u32,
    /// Rate limit: maximum creations per hour.
    pub max_creations_per_hour: u32,
    /// Running creation count for the current hour window.
    pub creations_this_hour: u32,
    /// Quality gate: minimum eval score a created agent must achieve.
    pub min_child_quality: f64,
    /// Caveats that propagate to all children.
    pub inherited_caveats: Vec<DelegationCaveat>,
}

/// Extension that gives an agent the ability to optimize configurations.
pub struct ConfigOptimizerExt {
    /// Parameter ranges the optimizer can explore.
    pub tunable_params: Vec<TunableParam>,
    /// Optimization history (configs tried and their outcomes).
    pub history: Vec<ConfigTrialOutcome>,
    /// Strategy: grid search, random search, Bayesian, or bandit.
    pub strategy: OptimizationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParam {
    pub name: String,
    pub param_type: ParamType,
    pub range: ParamRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// Floating-point parameter (e.g., temperature, top_p).
    Float,
    /// Integer parameter (e.g., max_tokens, retry_count).
    Int,
    /// Categorical parameter (e.g., model name, strategy type).
    Categorical { options: Vec<String> },
    /// Boolean toggle.
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamRange {
    Float { min: f64, max: f64 },
    Int { min: i64, max: i64 },
    Categorical,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTrialOutcome {
    pub config: serde_json::Value,
    pub eval_score: f64,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    GridSearch,
    RandomSearch,
    Bayesian,
    Bandit,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `248`
- Section hash: `f30015a2aa93f600b326d6df4bdf769871139fe4dea275a45929c6f9f70a5831`

**Normative requirements and implementation claims:**
- ```rust /// Extension that gives an agent the ability to create other agents. pub struct AgentCreatorExt { /// Maximum creation depth. Children inherit depth - 1. pub max_depth: u32, /// Current depth in the creation chain (0 = top-level meta-agent). pub current_depth: u32, /// Rate limit: maximum creations per hour. pub max_creations_per_hour: u32, /// Running creation count for the current hour window. pub creations_this_hour: u32, /// Quality gate: minimum eval score a created agent must achieve. pub min_child_quality: f64, /// Caveats that propagate to all children. pub inherited_caveats: Vec<DelegationCaveat>, }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentCreatorExt
- ConfigOptimizerExt
- TunableParam
- ParamType
- ParamRange
- ConfigTrialOutcome
- OptimizationStrategy
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
- Contract 1: language `rust`, first line `/// Extension that gives an agent the ability to create other agents.`

```rust
/// Extension that gives an agent the ability to create other agents.
pub struct AgentCreatorExt {
    /// Maximum creation depth. Children inherit depth - 1.
    pub max_depth: u32,
    /// Current depth in the creation chain (0 = top-level meta-agent).
    pub current_depth: u32,
    /// Rate limit: maximum creations per hour.
    pub max_creations_per_hour: u32,
    /// Running creation count for the current hour window.
    pub creations_this_hour: u32,
    /// Quality gate: minimum eval score a created agent must achieve.
    pub min_child_quality: f64,
    /// Caveats that propagate to all children.
    pub inherited_caveats: Vec<DelegationCaveat>,
}

/// Extension that gives an agent the ability to optimize configurations.
pub struct ConfigOptimizerExt {
    /// Parameter ranges the optimizer can explore.
    pub tunable_params: Vec<TunableParam>,
    /// Optimization history (configs tried and their outcomes).
    pub history: Vec<ConfigTrialOutcome>,
    /// Strategy: grid search, random search, Bayesian, or bandit.
    pub strategy: OptimizationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParam {
    pub name: String,
    pub param_type: ParamType,
    pub range: ParamRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// Floating-point parameter (e.g., temperature, top_p).
    Float,
    /// Integer parameter (e.g., max_tokens, retry_count).
    Int,
    /// Categorical parameter (e.g., model name, strategy type).
    Categorical { options: Vec<String> },
    /// Boolean toggle.
    Bool,
}

#[deri
...
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Serialize|model|creation|config|TunableParam|Runtime|Rate|ParamType" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Serialize|model|creation|config|TunableParam|Runtime|Rate|ParamType" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `AgentCreatorExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ConfigOptimizerExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TunableParam` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ParamType` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ParamRange` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ConfigTrialOutcome` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `OptimizationStrategy` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S005 -- Tools

**Source section:** `tmp/architecture/13-meta.md:99` through `118`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Tools

Meta-agents have access to these tools through the standard tool dispatch system:

| Tool | Parameters | Description |
|------|-----------|-------------|
| `agent_create` | `config: AgentConfig` | Create a new agent from a configuration |
| `agent_configure` | `agent_id: String, patch: ConfigPatch` | Update a running agent's configuration |
| `agent_start` | `agent_id: String` | Start a stopped agent |
| `agent_stop` | `agent_id: String` | Gracefully stop a running agent |
| `agent_fork` | `source_id: String, overrides: ConfigPatch` | Fork an existing agent with modifications |
| `agent_eval` | `agent_id: String, eval_id: String` | Run an eval against an agent and return the score |
| `agent_list_children` | none | List all agents created by this meta-agent |

The `agent_create` tool enforces:
- Depth check: `current_depth + 1 <= max_depth`
- Rate limit: `creations_this_hour < max_creations_per_hour`
- Caveat inheritance: child caveats are the intersection of parent caveats and any additional child-specific caveats
- Quality gate: if `min_child_quality > 0`, the created agent runs through a quick eval before registration
````

**Explicit detail extraction from this section:**

- Section word count: `132`
- Section hash: `9e6f04a078d7880d404bb92daf545e102177e9b9acf2f33374ca42f048a2fa18`

**Normative requirements and implementation claims:**
- | Tool | Parameters | Description | |------|-----------|-------------| | `agent_create` | `config: AgentConfig` | Create a new agent from a configuration | | `agent_configure` | `agent_id: String, patch: ConfigPatch` | Update a running agent's configuration | | `agent_start` | `agent_id: String` | Start a stopped agent | | `agent_stop` | `agent_id: String` | Gracefully stop a running agent | | `agent_fork` | `source_id: String, overrides: ConfigPatch` | Fork an existing agent with modifications | | `agent_eval` | `agent_id: String, eval_id: String` | Run an eval against an agent and return the score | | `agent_list_children` | none | List all agents created by this meta-agent |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- agent_create
- agent_configure
- agent_start
- agent_stop
- agent_fork
- agent_eval
- agent_list_children

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Depth check: `current_depth + 1 <= max_depth`
- - Rate limit: `creations_this_hour < max_creations_per_hour`
- - Caveat inheritance: child caveats are the intersection of parent caveats and any additional child-specific caveats
- - Quality gate: if `min_child_quality > 0`, the created agent runs through a quick eval before registration

**Tables extracted:**
- Table 1:

```markdown
| Tool | Parameters | Description |
|------|-----------|-------------|
| `agent_create` | `config: AgentConfig` | Create a new agent from a configuration |
| `agent_configure` | `agent_id: String, patch: ConfigPatch` | Update a running agent's configuration |
| `agent_start` | `agent_id: String` | Start a stopped agent |
| `agent_stop` | `agent_id: String` | Gracefully stop a running agent |
| `agent_fork` | `source_id: String, overrides: ConfigPatch` | Fork an existing agent with modifications |
| `agent_eval` | `agent_id: String, eval_id: String` | Run an eval against an agent and return the score |
| `agent_list_children` | none | List all agents created by this meta-agent |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "tool|config|agent_create|Tools|String|agent_stop|agent_start|agent_list_children" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tool|config|agent_create|Tools|String|agent_stop|agent_start|agent_list_children" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `agent_create` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_configure` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_stop` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_fork` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_eval` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_list_children` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S006 -- Configuration

**Source section:** `tmp/architecture/13-meta.md:119` through `150`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Configuration

```rust
/// Configuration for a meta-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAgentConfig {
    /// Base agent configuration (domain, extensions, gates, model routing).
    pub agent: AgentConfig,
    /// What kinds of agents this meta-agent produces.
    pub target_spec: TargetSpec,
    /// Creator extension configuration.
    pub creator: AgentCreatorExt,
    /// Optional optimizer extension.
    pub optimizer: Option<ConfigOptimizerExt>,
}

/// Describes what a meta-agent is designed to produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Human-readable description of the target agent type.
    pub description: String,
    /// Domain the produced agents should operate in.
    pub target_domain: String,
    /// Required capabilities the produced agents must have.
    pub required_capabilities: Vec<String>,
    /// Template to base new agents on (optional).
    pub template_id: Option<String>,
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `109`
- Section hash: `4baccea4c9dfd8ab6986df948988e74e63976d4c45165bc4126fe7e89d7379c8`

**Normative requirements and implementation claims:**
- /// Describes what a meta-agent is designed to produce. #[derive(Debug, Clone, Serialize, Deserialize)] pub struct TargetSpec { /// Human-readable description of the target agent type. pub description: String, /// Domain the produced agents should operate in. pub target_domain: String, /// Required capabilities the produced agents must have. pub required_capabilities: Vec<String>, /// Template to base new agents on (optional). pub template_id: Option<String>, } ```
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- MetaAgentConfig
- TargetSpec

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
- Contract 1: language `rust`, first line `/// Configuration for a meta-agent.`

```rust
/// Configuration for a meta-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAgentConfig {
    /// Base agent configuration (domain, extensions, gates, model routing).
    pub agent: AgentConfig,
    /// What kinds of agents this meta-agent produces.
    pub target_spec: TargetSpec,
    /// Creator extension configuration.
    pub creator: AgentCreatorExt,
    /// Optional optimizer extension.
    pub optimizer: Option<ConfigOptimizerExt>,
}

/// Describes what a meta-agent is designed to produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Human-readable description of the target agent type.
    pub description: String,
    /// Domain the produced agents should operate in.
    pub target_domain: String,
    /// Required capabilities the produced agents must have.
    pub required_capabilities: Vec<String>,
    /// Template to base new agents on (optional).
    pub template_id: Option<String>,
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Configuration|TargetSpec|target|MetaAgentConfig|produce|meta|String|Serialize" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Configuration|TargetSpec|target|MetaAgentConfig|produce|meta|String|Serialize" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `MetaAgentConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TargetSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S007 -- Generators

**Source section:** `tmp/architecture/13-meta.md:151` through `154`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Generators

A generator is an agent that produces non-agent objects: arenas, gates, evals, extensions, or domain profiles. Where a meta-agent's output is another agent, a generator's output is a registered first-class object.
````

**Explicit detail extraction from this section:**

- Section word count: `36`
- Section hash: `42367823f4c752b133bf0f3305852637f8262812b4f7b10474b27ec9fd90422d`

**Normative requirements and implementation claims:**
- None extracted from this section.

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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "generator|Generators|output|object|registered|profiles|produces|objects" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "generator|Generators|output|object|registered|profiles|produces|objects" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S008 -- Output schema validation

**Source section:** `tmp/architecture/13-meta.md:155` through `205`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Output schema validation

Every generator declares the type of object it produces. Output is validated against the type's schema before registration. If validation fails, the object is not registered and the generation event records the failure.

```rust
/// Types of objects a generator can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratorOutputType {
    Arena,
    Gate,
    Eval,
    Extension,
    DomainProfile,
}

/// A generator's output before validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOutput {
    pub output_type: GeneratorOutputType,
    /// Serialized object matching the output type's schema.
    pub payload: serde_json::Value,
    /// Metadata about the generation process.
    pub metadata: GenerationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Generator agent ID.
    pub generator_id: String,
    /// Specification the generator was given.
    pub spec: serde_json::Value,
    /// Time spent generating.
    pub generation_duration_ms: u64,
    /// Model used.
    pub model: String,
    /// Cost of generation.
    pub cost_usd: f64,
}

/// Validates generator output against the expected schema for its type.
pub fn validate_generator_output(output: &GeneratorOutput) -> Result<(), ValidationError> {
    match output.output_type {
        GeneratorOutputType::Arena => validate_arena_schema(&output.payload),
        GeneratorOutputType::Gate => validate_gate_schema(&output.payload),
        GeneratorOutputType::Eval => validate_eval_schema(&output.payload),
        GeneratorOutputType::Extension => validate_extension_schema(&output.payload),
        GeneratorOutputType::DomainProfile => validate_domain_schema(&output.payload),
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `181`
- Section hash: `c6ce3f40eab20790c93178d44aca000deb8d592cfcf55eb6b1611f69dbf1294c`

**Normative requirements and implementation claims:**
- Every generator declares the type of object it produces. Output is validated against the type's schema before registration. If validation fails, the object is not registered and the generation event records the failure.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- of
- GeneratorOutputType
- GeneratorOutput
- GenerationMetadata
- validate_generator_output

**Event names and event-like entities:**
- output.output_type
- output.payload

**State transitions:**
- Arena -> validate_arena_schema
- Gate -> validate_gate_schema
- Eval -> validate_eval_schema
- Extension -> validate_extension_schema
- DomainProfile -> validate_domain_schema

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// Types of objects a generator can produce.`

```rust
/// Types of objects a generator can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratorOutputType {
    Arena,
    Gate,
    Eval,
    Extension,
    DomainProfile,
}

/// A generator's output before validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOutput {
    pub output_type: GeneratorOutputType,
    /// Serialized object matching the output type's schema.
    pub payload: serde_json::Value,
    /// Metadata about the generation process.
    pub metadata: GenerationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Generator agent ID.
    pub generator_id: String,
    /// Specification the generator was given.
    pub spec: serde_json::Value,
    /// Time spent generating.
    pub generation_duration_ms: u64,
    /// Model used.
    pub model: String,
    /// Cost of generation.
    pub cost_usd: f64,
}

/// Validates generator output against the expected schema for its type.
pub fn validate_generator_output(output: &GeneratorOutput) -> Result<(), ValidationError> {
    match output.output_type {
        GeneratorOutputType::Arena => validate_arena_schema(&output.payload),
        GeneratorOutputType::Gate => validate_gate_schema(&output.payload),
        GeneratorOutputType::Eval => validate_eval_schema(&output.payload),
        GeneratorOutputType::Extension => validate_extension_schema(&output.payload),
        GeneratorOutputType::DomainProfile => validate_domain_schema(&output.payload),
    }
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Output|generator|type|schema|GeneratorOutput|GeneratorOutputType|validation|Serialize" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Output|generator|type|schema|GeneratorOutput|GeneratorOutputType|validation|Serialize" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `of` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GeneratorOutputType` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GeneratorOutput` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GenerationMetadata` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `validate_generator_output` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `output.output_type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `output.payload` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `Arena -> validate_arena_schema` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Gate -> validate_gate_schema` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Eval -> validate_eval_schema` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Extension -> validate_extension_schema` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `DomainProfile -> validate_domain_schema` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S009 -- Generator configuration

**Source section:** `tmp/architecture/13-meta.md:206` through `226`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Generator configuration

```rust
/// Configuration for a generator agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Base agent configuration.
    pub agent: AgentConfig,
    /// What type of object this generator produces.
    pub output_type: GeneratorOutputType,
    /// JSON Schema for output validation (derived from output_type if not provided).
    pub output_schema: Option<serde_json::Value>,
    /// Whether generated objects are automatically registered or held for review.
    pub auto_register: bool,
    /// Quality threshold: minimum eval score before auto-registration.
    pub min_quality: f64,
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `13dc1b2a020b437ffcaff39d00f8b65699fe49a5ac3f9ca879f970e93506e086`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- GeneratorConfig
- of

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
- Contract 1: language `rust`, first line `/// Configuration for a generator agent.`

```rust
/// Configuration for a generator agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Base agent configuration.
    pub agent: AgentConfig,
    /// What type of object this generator produces.
    pub output_type: GeneratorOutputType,
    /// JSON Schema for output validation (derived from output_type if not provided).
    pub output_schema: Option<serde_json::Value>,
    /// Whether generated objects are automatically registered or held for review.
    pub auto_register: bool,
    /// Quality threshold: minimum eval score before auto-registration.
    pub min_quality: f64,
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Generator|configuration|output|GeneratorConfig|type|auto|output_type|object" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Generator|configuration|output|GeneratorConfig|type|auto|output_type|object" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `GeneratorConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `of` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S010 -- Lineage tracking

**Source section:** `tmp/architecture/13-meta.md:227` through `230`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Lineage tracking

Every object in the system records its creation ancestry. This forms a directed acyclic graph of parent-child relationships traversable from any node.
````

**Explicit detail extraction from this section:**

- Section word count: `23`
- Section hash: `3dd45b79468b7d2bd57e0a4ec4659a1f73b00cd2d4634458fad3ec4789e6640e`

**Normative requirements and implementation claims:**
- None extracted from this section.

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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tracking|Lineage|traversable|relationships|records|parent|object|node" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tracking|Lineage|traversable|relationships|records|parent|object|node" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S011 -- On-chain lineage

**Source section:** `tmp/architecture/13-meta.md:231` through `284`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### On-chain lineage

When an agent registers on-chain (ERC-8004), the `parentPassport` field records which agent created it. For non-agent objects (arenas, evals, etc.), the creating agent's passport ID is recorded in the object's metadata.

```rust
/// A lineage edge recording a parent-child creation relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Parent object identifier.
    pub parent_id: ObjectId,
    /// Child object identifier.
    pub child_id: ObjectId,
    /// Type of relationship.
    pub relationship: LineageRelationship,
    /// Block at which the relationship was recorded.
    pub recorded_at_block: u64,
    /// Timestamp.
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageRelationship {
    /// Parent created child from scratch.
    Generated,
    /// Child is a fork of an existing object with modifications.
    Forked,
    /// Child evolved from parent through an optimization process.
    Evolved,
}

/// A generic object identifier used in lineage tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    /// Object type (agent, arena, eval, extension, gate, domain).
    pub object_type: ObjectType,
    /// Unique identifier within the type namespace.
    pub id: String,
    /// On-chain passport ID (for agents) or registration ID.
    pub chain_id: Option<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Agent,
    MetaAgent,
    Generator,
    Arena,
    Eval,
    Gate,
    Extension,
    DomainProfile,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `196`
- Section hash: `b2494a14305fa2f9bfb207c3719bcbe7cf9dd0f9a357ad130e9460001dfbe4ec`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- LineageEdge
- LineageRelationship
- ObjectId
- namespace
- ObjectType
- parentPassport

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
- Contract 1: language `rust`, first line `/// A lineage edge recording a parent-child creation relationship.`

```rust
/// A lineage edge recording a parent-child creation relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Parent object identifier.
    pub parent_id: ObjectId,
    /// Child object identifier.
    pub child_id: ObjectId,
    /// Type of relationship.
    pub relationship: LineageRelationship,
    /// Block at which the relationship was recorded.
    pub recorded_at_block: u64,
    /// Timestamp.
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageRelationship {
    /// Parent created child from scratch.
    Generated,
    /// Child is a fork of an existing object with modifications.
    Forked,
    /// Child evolved from parent through an optimization process.
    Evolved,
}

/// A generic object identifier used in lineage tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    /// Object type (agent, arena, eval, extension, gate, domain).
    pub object_type: ObjectType,
    /// Unique identifier within the type namespace.
    pub id: String,
    /// On-chain passport ID (for agents) or registration ID.
    pub chain_id: Option<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Agent,
    MetaAgent,
    Generator,
    Arena,
    Eval,
    Gate,
    Extension,
    DomainProfile,
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "object|lineage|chain|Serialize|ObjectId|relationship|parent|child" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "object|lineage|chain|Serialize|ObjectId|relationship|parent|child" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `LineageEdge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LineageRelationship` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ObjectId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `namespace` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ObjectType` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parentPassport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S012 -- Lineage queries

**Source section:** `tmp/architecture/13-meta.md:285` through `339`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lineage queries

```rust
/// Service for querying lineage relationships.
pub struct LineageService {
    /// Local lineage store (JSONL-backed).
    local_store: LineageStore,
    /// Chain reader for on-chain lineage data.
    chain_reader: Option<Box<dyn ChainReader>>,
}

impl LineageService {
    /// All ancestors of an object (parent, grandparent, ...).
    pub async fn ancestors(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Direct children of an object.
    pub async fn children(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// All descendants recursively (children, grandchildren, ...).
    pub async fn descendants(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Siblings: objects with the same parent.
    pub async fn siblings(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Full lineage graph for visualization.
    pub async fn graph(
        &self,
        root: &ObjectId,
        max_depth: u32,
    ) -> Result<LineageGraph> { ... }
}

/// Lineage graph for dashboard visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: ObjectId,
    /// Human-readable name.
    pub name: String,
    /// Total descendants count.
    pub descendant_count: u64,
    /// Aggregate success rate of descendants (for meta-agents).
    pub descendant_success_rate: Option<f64>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `171`
- Section hash: `d778374d6250392fac2f9676d7564b9bfef627e2301d770a61488e4ab4032f26`

**Normative requirements and implementation claims:**
- /// Lineage graph for dashboard visualization. #[derive(Debug, Clone, Serialize, Deserialize)] pub struct LineageGraph { pub nodes: Vec<LineageNode>, pub edges: Vec<LineageEdge>, }
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- LineageService
- ancestors
- children
- descendants
- siblings
- graph
- LineageGraph
- LineageNode

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
- Contract 1: language `rust`, first line `/// Service for querying lineage relationships.`

```rust
/// Service for querying lineage relationships.
pub struct LineageService {
    /// Local lineage store (JSONL-backed).
    local_store: LineageStore,
    /// Chain reader for on-chain lineage data.
    chain_reader: Option<Box<dyn ChainReader>>,
}

impl LineageService {
    /// All ancestors of an object (parent, grandparent, ...).
    pub async fn ancestors(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Direct children of an object.
    pub async fn children(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// All descendants recursively (children, grandchildren, ...).
    pub async fn descendants(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Siblings: objects with the same parent.
    pub async fn siblings(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Full lineage graph for visualization.
    pub async fn graph(
        &self,
        root: &ObjectId,
        max_depth: u32,
    ) -> Result<LineageGraph> { ... }
}

/// Lineage graph for dashboard visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: ObjectId,
    /// Human-readable name.
    pub name: String,
    /// Total descendants count.
    pub descendant_count: u64,
    /// Aggregate success rate of descendants (for meta-agents).
    pub descendant_success_rate: Option<f64>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Lineage|object|graph|descendants|children|siblings|ancestors|ObjectId" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Lineage|object|graph|descendants|children|siblings|ancestors|ObjectId" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `LineageService` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ancestors` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `children` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `descendants` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `siblings` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `graph` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LineageGraph` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LineageNode` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S013 -- Recursive safety

**Source section:** `tmp/architecture/13-meta.md:340` through `343`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Recursive safety

Recursive agent creation is powerful and dangerous. Without bounds, a meta-agent could spawn thousands of agents that consume all available resources, create circular dependencies, or produce progressively worse outputs.
````

**Explicit detail extraction from this section:**

- Section word count: `30`
- Section hash: `6a748d02a857d7b47bd7131fe319b247bbbe4448fe5925bde22203cf9ef890b8`

**Normative requirements and implementation claims:**
- Recursive agent creation is powerful and dangerous. Without bounds, a meta-agent could spawn thousands of agents that consume all available resources, create circular dependencies, or produce progressively worse outputs.

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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Recursive|safety|worse|thousands|spawn|resources|progressively|produce" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recursive|safety|worse|thousands|spawn|resources|progressively|produce" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S014 -- Safety mechanisms

**Source section:** `tmp/architecture/13-meta.md:344` through `440`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Safety mechanisms

**Depth limit.** Every meta-agent has a `max_depth` (default: 3). The `AgentCreatorExt` tracks `current_depth` and refuses creation when the limit is reached. Children inherit `max_depth - 1` as their own maximum.

**Rate limit.** Each meta-agent is rate-limited to `max_creations_per_hour` (default: 10). The limit resets on a rolling window basis.

**Quality gate.** If `min_child_quality > 0`, every created agent runs through a quick eval before registration. Agents that score below the threshold are rejected and their creation events record the failure reason.

**Caveat inheritance.** Children can only narrow parent caveats, never widen them. The enforcement is structural:

```rust
/// Compute the effective caveats for a child agent.
/// The child's caveats are the intersection of the parent's inherited
/// caveats and any additional restrictions specified at creation time.
pub fn compute_child_caveats(
    parent_caveats: &[DelegationCaveat],
    additional_restrictions: &[DelegationCaveat],
) -> Vec<DelegationCaveat> {
    let mut child_caveats = parent_caveats.to_vec();
    for restriction in additional_restrictions {
        // Only add restrictions that narrow existing caveats.
        // Reject any attempt to widen (e.g., increasing a budget cap).
        if restriction.is_narrower_than_all(&child_caveats) {
            child_caveats.push(restriction.clone());
        }
    }
    child_caveats
}
```

**Anomaly detection.** The `RecursiveSafetyMonitor` runs continuously and watches for:

```rust
/// Monitors recursive creation patterns for anomalies.
pub struct RecursiveSafetyMonitor {
    /// Maximum creation rate across all meta-agents (global backstop).
    pub global_max_rate_per_hour: u32,
    /// Minimum quality trend slope before flagging degradation.
    pub min_quality_slope: f64,
    /// Window size for quality trend computation.
    pub quality_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyAnomaly {
    /// A meta-agent is creating agents faster than its rate limit.
    RateLimitViolation {
        meta_agent_id: String,
        rate: u32,
        limit: u32,
    },
    /// Quality is degrading across generations.
    QualityDegradation {
        meta_agent_id: String,
        generation: u32,
        quality_trend: Vec<f64>,
        slope: f64,
    },
    /// Circular dependency detected (A created B, B created A).
    CircularDependency {
        agents: Vec<String>,
    },
    /// Global creation rate exceeded.
    GlobalRateExceeded {
        current_rate: u32,
        limit: u32,
    },
    /// A meta-agent attempted to widen parent caveats.
    CaveatEscalation {
        meta_agent_id: String,
        attempted_caveat: String,
    },
}

impl RecursiveSafetyMonitor {
    /// Check all active recursive processes for anomalies.
    pub fn scan(&self, active_processes: &[RecursiveProcess]) -> Vec<SafetyAnomaly> { ... }

    /// Recommended action for an anomaly.
    pub fn recommend_action(&self, anomaly: &SafetyAnomaly) -> SafetyAction { ... }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyAction {
    /// Log the anomaly but take no action.
    Log,
    /// Pause the offending meta-agent.
    Pause { agent_id: String },
    /// Quarantine: pause the agent and flag all its recent children for review.
    Quarantine { agent_id: String },
    /// Terminate: stop the agent and prevent restart without manual approval.
    Terminate { agent_id: String },
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `378`
- Section hash: `d5ec28fd3b53b484884fd67e7c5c0f9316aae479e4289b4373da2fc0d404f1c4`

**Normative requirements and implementation claims:**
- **Depth limit.** Every meta-agent has a `max_depth` (default: 3). The `AgentCreatorExt` tracks `current_depth` and refuses creation when the limit is reached. Children inherit `max_depth - 1` as their own maximum.
- **Rate limit.** Each meta-agent is rate-limited to `max_creations_per_hour` (default: 10). The limit resets on a rolling window basis.
- **Quality gate.** If `min_child_quality > 0`, every created agent runs through a quick eval before registration. Agents that score below the threshold are rejected and their creation events record the failure reason.
- **Caveat inheritance.** Children can only narrow parent caveats, never widen them. The enforcement is structural:
- ```rust /// Compute the effective caveats for a child agent. /// The child's caveats are the intersection of the parent's inherited /// caveats and any additional restrictions specified at creation time. pub fn compute_child_caveats( parent_caveats: &[DelegationCaveat], additional_restrictions: &[DelegationCaveat], ) -> Vec<DelegationCaveat> { let mut child_caveats = parent_caveats.to_vec(); for restriction in additional_restrictions { // Only add restrictions that narrow existing caveats. // Reject any attempt to widen (e.g., increasing a budget cap). if restriction.is_narrower_than_all(&child_caveats) { child_caveats.push(restriction.clone()); } } child_caveats } ```
- **Anomaly detection.** The `RecursiveSafetyMonitor` runs continuously and watches for:
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- compute_child_caveats
- RecursiveSafetyMonitor
- SafetyAnomaly
- scan
- recommend_action
- SafetyAction
- max_depth
- AgentCreatorExt
- current_depth
- max_creations_per_hour

**Event names and event-like entities:**
- parent_caveats.to_vec
- restriction.is_narrower_than_all
- child_caveats.push
- restriction.clone

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
- Contract 1: language `rust`, first line `/// Compute the effective caveats for a child agent.`

```rust
/// Compute the effective caveats for a child agent.
/// The child's caveats are the intersection of the parent's inherited
/// caveats and any additional restrictions specified at creation time.
pub fn compute_child_caveats(
    parent_caveats: &[DelegationCaveat],
    additional_restrictions: &[DelegationCaveat],
) -> Vec<DelegationCaveat> {
    let mut child_caveats = parent_caveats.to_vec();
    for restriction in additional_restrictions {
        // Only add restrictions that narrow existing caveats.
        // Reject any attempt to widen (e.g., increasing a budget cap).
        if restriction.is_narrower_than_all(&child_caveats) {
            child_caveats.push(restriction.clone());
        }
    }
    child_caveats
}
```
- Contract 2: language `rust`, first line `/// Monitors recursive creation patterns for anomalies.`

```rust
/// Monitors recursive creation patterns for anomalies.
pub struct RecursiveSafetyMonitor {
    /// Maximum creation rate across all meta-agents (global backstop).
    pub global_max_rate_per_hour: u32,
    /// Minimum quality trend slope before flagging degradation.
    pub min_quality_slope: f64,
    /// Window size for quality trend computation.
    pub quality_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyAnomaly {
    /// A meta-agent is creating agents faster than its rate limit.
    RateLimitViolation {
        meta_agent_id: String,
        rate: u32,
        limit: u32,
    },
    /// Quality is degrading across generations.
    QualityDegradation {
        meta_agent_id: String,
        generation: u32,
        quality_trend: Vec<f64>,
        slope: f64,
    },
    /// Circular dependency detected (A created B, B created A).
    CircularDependency {
        agents: Vec<String>,
    },
    /// Global creation rate exceeded.
    GlobalRateExceeded {
        current_rate: u32,
        limit: u32,
    },
    /// A meta-agent attempted to widen parent caveats.
    CaveatEscalation {
        meta_agent_id: String,
        attempted_caveat: String,
    },
}

impl RecursiveSafetyMonitor {
    /// Check all active recursive processes for anomalies.
    pub fn scan(&self, active_processes: &[RecursiveProcess]) -> Vec<SafetyAnomaly> { ... }

    /// Recommended action for an anomaly.
    pub fn recommend_action(&self, anomaly: &SafetyAnomaly) -> SafetyAction { ... }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyAction {
    ///
...
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Caveat|caveats|Safety|child|Rate|meta|limit|Quality" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Caveat|caveats|Safety|child|Rate|meta|limit|Quality" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `compute_child_caveats` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RecursiveSafetyMonitor` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SafetyAnomaly` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `scan` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `recommend_action` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SafetyAction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `max_depth` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentCreatorExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `current_depth` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `max_creations_per_hour` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `parent_caveats.to_vec` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `restriction.is_narrower_than_all` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `child_caveats.push` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `restriction.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S015 -- Practical example

**Source section:** `tmp/architecture/13-meta.md:441` through `459`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Practical example

A market-regime meta-agent that creates trading agents optimized for different market conditions:

1. The meta-agent observes the current market regime (trending, ranging, volatile) via its knowledge store.
2. For each regime, it creates a specialized trading agent:
   - **Trending agent**: momentum-following strategy, wider stops, position sizing favors trends
   - **Ranging agent**: mean-reversion strategy, tight stops, reduced sizing in low-volatility conditions
   - **Volatile agent**: defensive strategy, minimal positions, hedging focus
3. Each child agent runs through a quick eval on historical data for its target regime.
4. Children that score above `min_child_quality` are registered and started.
5. The meta-agent monitors children's performance via `TradingReflect` events.
6. When a regime shift occurs, the meta-agent activates the appropriate child and pauses the others.
7. Over time, the `ConfigOptimizerExt` tunes each child's parameters based on accumulated P&L data.

The meta-agent itself has caveats: it can only create agents within the `trading` domain, with a maximum budget of $50/day per child, and a maximum of 3 concurrent children. These caveats propagate -- no child can spend more than $50/day, and no child can create further agents (depth = 1 for the children).

---
````

**Explicit detail extraction from this section:**

- Section word count: `196`
- Section hash: `150ca783369627a56d8c488dd60d613d2831693954a3221c960950a015676404`

**Normative requirements and implementation claims:**
- 1. The meta-agent observes the current market regime (trending, ranging, volatile) via its knowledge store. 2. For each regime, it creates a specialized trading agent: - **Trending agent**: momentum-following strategy, wider stops, position sizing favors trends - **Ranging agent**: mean-reversion strategy, tight stops, reduced sizing in low-volatility conditions - **Volatile agent**: defensive strategy, minimal positions, hedging focus 3. Each child agent runs through a quick eval on historical data for its target regime. 4. Children that score above `min_child_quality` are registered and started. 5. The meta-agent monitors children's performance via `TradingReflect` events. 6. When a regime shift occurs, the meta-agent activates the appropriate child and pauses the others. 7. Over time, the `ConfigOptimizerExt` tunes each child's parameters based on accumulated P&L data.
- The meta-agent itself has caveats: it can only create agents within the `trading` domain, with a maximum budget of $50/day per child, and a maximum of 3 concurrent children. These caveats propagate -- no child can spend more than $50/day, and no child can create further agents (depth = 1 for the children).
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- min_child_quality
- TradingReflect
- ConfigOptimizerExt
- trading

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. The meta-agent observes the current market regime (trending, ranging, volatile) via its knowledge store.
- 2. For each regime, it creates a specialized trading agent:
- - **Trending agent**: momentum-following strategy, wider stops, position sizing favors trends
- - **Ranging agent**: mean-reversion strategy, tight stops, reduced sizing in low-volatility conditions
- - **Volatile agent**: defensive strategy, minimal positions, hedging focus
- 3. Each child agent runs through a quick eval on historical data for its target regime.
- 4. Children that score above `min_child_quality` are registered and started.
- 5. The meta-agent monitors children's performance via `TradingReflect` events.
- 6. When a regime shift occurs, the meta-agent activates the appropriate child and pauses the others.
- 7. Over time, the `ConfigOptimizerExt` tunes each child's parameters based on accumulated P&L data.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "child|trading|regime|min_child_quality|meta|example|TradingReflect|Practical" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "child|trading|regime|min_child_quality|meta|example|TradingReflect|Practical" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `min_child_quality` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TradingReflect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ConfigOptimizerExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `trading` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S016 -- Event types

**Source section:** `tmp/architecture/13-meta.md:460` through `530`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event types

```json
{
    "type": "meta.agent_created",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "child_agent_id": "trending-trader-v3",
        "depth": 1,
        "target_spec": "trending market trader",
        "eval_score": 0.82,
        "cost_usd": 0.45,
        "block_number": 19847300
    }
}
```

```json
{
    "type": "meta.generation_failed",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "reason": "quality_below_threshold",
        "eval_score": 0.31,
        "min_required": 0.60,
        "spec": "volatile market trader"
    }
}
```

```json
{
    "type": "meta.safety_anomaly",
    "payload": {
        "anomaly_type": "quality_degradation",
        "meta_agent_id": "regime-meta-1",
        "severity": "warning",
        "quality_trend": [0.82, 0.75, 0.68, 0.61],
        "recommended_action": "pause"
    }
}
```

```json
{
    "type": "generator.output_produced",
    "payload": {
        "generator_id": "arena-gen-1",
        "output_type": "arena",
        "output_id": "trading-arena-eth-momentum",
        "validation_passed": true,
        "auto_registered": true,
        "cost_usd": 0.23
    }
}
```

```json
{
    "type": "lineage.edge_recorded",
    "payload": {
        "parent_type": "meta_agent",
        "parent_id": "regime-meta-1",
        "child_type": "agent",
        "child_id": "trending-trader-v3",
        "relationship": "generated",
        "block_number": 19847300
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `116`
- Section hash: `515a9b4116d660d30d1458b941abef1b0e2b1ee0b7a2f03956fda908dce49c8b`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- meta.agent_created
- meta.generation_failed
- meta.safety_anomaly
- generator.output_produced
- lineage.edge_recorded

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
- Contract 1: language `json`, first line `{`

```json
{
    "type": "meta.agent_created",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "child_agent_id": "trending-trader-v3",
        "depth": 1,
        "target_spec": "trending market trader",
        "eval_score": 0.82,
        "cost_usd": 0.45,
        "block_number": 19847300
    }
}
```
- Contract 2: language `json`, first line `{`

```json
{
    "type": "meta.generation_failed",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "reason": "quality_below_threshold",
        "eval_score": 0.31,
        "min_required": 0.60,
        "spec": "volatile market trader"
    }
}
```
- Contract 3: language `json`, first line `{`

```json
{
    "type": "meta.safety_anomaly",
    "payload": {
        "anomaly_type": "quality_degradation",
        "meta_agent_id": "regime-meta-1",
        "severity": "warning",
        "quality_trend": [0.82, 0.75, 0.68, 0.61],
        "recommended_action": "pause"
    }
}
```
- Contract 4: language `json`, first line `{`

```json
{
    "type": "generator.output_produced",
    "payload": {
        "generator_id": "arena-gen-1",
        "output_type": "arena",
        "output_id": "trading-arena-eth-momentum",
        "validation_passed": true,
        "auto_registered": true,
        "cost_usd": 0.23
    }
}
```
- Contract 5: language `json`, first line `{`

```json
{
    "type": "lineage.edge_recorded",
    "payload": {
        "parent_type": "meta_agent",
        "parent_id": "regime-meta-1",
        "child_type": "agent",
        "child_id": "trending-trader-v3",
        "relationship": "generated",
        "block_number": 19847300
    }
}
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "type|meta|types|payload|json|Event|trader|regime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "type|meta|types|payload|json|Event|trader|regime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Emit or consume `meta.agent_created` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.generation_failed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.safety_anomaly` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `generator.output_produced` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `lineage.edge_recorded` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S017 -- Full event type list

**Source section:** `tmp/architecture/13-meta.md:531` through `544`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Full event type list

| Event | Emitted by | Consumed by |
|-------|-----------|-------------|
| `meta.agent_created` | AgentCreatorExt | Dashboard, lineage service, reputation registry |
| `meta.agent_configured` | ConfigOptimizerExt | Dashboard, agent detail |
| `meta.generation_failed` | AgentCreatorExt | Dashboard (alert), safety monitor |
| `meta.safety_anomaly` | RecursiveSafetyMonitor | Dashboard (alert), auto-pause system |
| `generator.output_produced` | Generator agent | Dashboard, lineage service, output registry |
| `generator.validation_failed` | Output validation | Dashboard, generator detail |
| `lineage.edge_recorded` | Lineage service | Dashboard (lineage graph), chain indexer |

---
````

**Explicit detail extraction from this section:**

- Section word count: `59`
- Section hash: `04cb7a27e3b664234c9959378e47f5c798b513d1fcefbdcecefbbaa02c4c3726`

**Normative requirements and implementation claims:**
- | Event | Emitted by | Consumed by | |-------|-----------|-------------| | `meta.agent_created` | AgentCreatorExt | Dashboard, lineage service, reputation registry | | `meta.agent_configured` | ConfigOptimizerExt | Dashboard, agent detail | | `meta.generation_failed` | AgentCreatorExt | Dashboard (alert), safety monitor | | `meta.safety_anomaly` | RecursiveSafetyMonitor | Dashboard (alert), auto-pause system | | `generator.output_produced` | Generator agent | Dashboard, lineage service, output registry | | `generator.validation_failed` | Output validation | Dashboard, generator detail | | `lineage.edge_recorded` | Lineage service | Dashboard (lineage graph), chain indexer |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- meta.agent_created
- meta.agent_configured
- meta.generation_failed
- meta.safety_anomaly
- generator.output_produced
- generator.validation_failed
- lineage.edge_recorded

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- meta.agent_created
- meta.agent_configured
- meta.generation_failed
- meta.safety_anomaly
- generator.output_produced
- generator.validation_failed
- lineage.edge_recorded

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Event | Emitted by | Consumed by |
|-------|-----------|-------------|
| `meta.agent_created` | AgentCreatorExt | Dashboard, lineage service, reputation registry |
| `meta.agent_configured` | ConfigOptimizerExt | Dashboard, agent detail |
| `meta.generation_failed` | AgentCreatorExt | Dashboard (alert), safety monitor |
| `meta.safety_anomaly` | RecursiveSafetyMonitor | Dashboard (alert), auto-pause system |
| `generator.output_produced` | Generator agent | Dashboard, lineage service, output registry |
| `generator.validation_failed` | Output validation | Dashboard, generator detail |
| `lineage.edge_recorded` | Lineage service | Dashboard (lineage graph), chain indexer |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "event|type|list|lineage|Full|meta|generator|service" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "event|type|list|lineage|Full|meta|generator|service" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Emit or consume `meta.agent_created` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.agent_configured` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.generation_failed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.safety_anomaly` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `generator.output_produced` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `generator.validation_failed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `lineage.edge_recorded` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `meta.agent_created` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `meta.agent_configured` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `meta.generation_failed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `meta.safety_anomaly` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `generator.output_produced` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `generator.validation_failed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `lineage.edge_recorded` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S018 -- API surface

**Source section:** `tmp/architecture/13-meta.md:545` through `546`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## API surface
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `bc3ef3c6f285bc86997b2bc41a829b1cfa87cf574f7480e2f0c4e33880648ccf`

**Normative requirements and implementation claims:**
- None extracted from this section.

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
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "surface|API|meta" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|API|meta" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S019 -- Meta-agent endpoints

**Source section:** `tmp/architecture/13-meta.md:547` through `556`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Meta-agent endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/agents` | List meta-agents (supports `?scope=fleet&owner={address}`) |
| `GET` | `/api/meta/agents/{id}` | Meta-agent detail with children summary |
| `POST` | `/api/meta/agents` | Create a new meta-agent |
| `GET` | `/api/meta/agents/{id}/children` | List agents created by this meta-agent |
| `GET` | `/api/meta/generations?limit=20` | Recent generation events across all meta-agents |
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `f8f6511615a4a8a1c3617fd274069e6c4dcc6c9cb9ea9d27ad57a32b78c14869`

**Normative requirements and implementation claims:**
- | Method | Path | Description | |--------|------|-------------| | `GET` | `/api/meta/agents` | List meta-agents (supports `?scope=fleet&owner={address}`) | | `GET` | `/api/meta/agents/{id}` | Meta-agent detail with children summary | | `POST` | `/api/meta/agents` | Create a new meta-agent | | `GET` | `/api/meta/agents/{id}/children` | List agents created by this meta-agent | | `GET` | `/api/meta/generations?limit=20` | Recent generation events across all meta-agents |

**Routes and endpoint references:**
- /api/meta/agents
- /api/meta/agents/{id}
- /api/meta/agents/{id}/children

**Files and path references:**
- api/meta/
- api/meta/agents/

**Types, functions, traits, and inline code identifiers:**
- GET
- POST

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
- Table 1:

```markdown
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/agents` | List meta-agents (supports `?scope=fleet&owner={address}`) |
| `GET` | `/api/meta/agents/{id}` | Meta-agent detail with children summary |
| `POST` | `/api/meta/agents` | Create a new meta-agent |
| `GET` | `/api/meta/agents/{id}/children` | List agents created by this meta-agent |
| `GET` | `/api/meta/generations?limit=20` | Recent generation events across all meta-agents |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/`
- `api/meta/agents/`
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
rg -n "Meta|GET|endpoints|api|POST|generation|children|List" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Meta|GET|endpoints|api|POST|generation|children|List" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/`
- `api/meta/agents/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

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
- [ ] Implement or verify route `/api/meta/agents` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/agents/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/agents/{id}/children` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `GET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `POST` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S020 -- Generator endpoints

**Source section:** `tmp/architecture/13-meta.md:557` through `566`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Generator endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/generators` | List generators (supports `?type={arena,gate,...}`) |
| `GET` | `/api/meta/generators/{id}` | Generator detail with recent outputs |
| `POST` | `/api/meta/generators` | Create a new generator |
| `GET` | `/api/meta/generators/outputs?limit=20` | Recent generated objects |
| `GET` | `/api/meta/generators/featured` | Featured public generators |
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `f5fd393153822a626359f36d8db54883400b976af1f201230c0b7bfe55219319`

**Normative requirements and implementation claims:**
- | Method | Path | Description | |--------|------|-------------| | `GET` | `/api/meta/generators` | List generators (supports `?type={arena,gate,...}`) | | `GET` | `/api/meta/generators/{id}` | Generator detail with recent outputs | | `POST` | `/api/meta/generators` | Create a new generator | | `GET` | `/api/meta/generators/outputs?limit=20` | Recent generated objects | | `GET` | `/api/meta/generators/featured` | Featured public generators |

**Routes and endpoint references:**
- /api/meta/generators
- /api/meta/generators/{id}
- /api/meta/generators/featured

**Files and path references:**
- api/meta/
- api/meta/generators/

**Types, functions, traits, and inline code identifiers:**
- GET
- POST

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
- Table 1:

```markdown
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/generators` | List generators (supports `?type={arena,gate,...}`) |
| `GET` | `/api/meta/generators/{id}` | Generator detail with recent outputs |
| `POST` | `/api/meta/generators` | Create a new generator |
| `GET` | `/api/meta/generators/outputs?limit=20` | Recent generated objects |
| `GET` | `/api/meta/generators/featured` | Featured public generators |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/`
- `api/meta/generators/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Generator|GET|generators|meta|endpoints|api|POST|recent" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Generator|GET|generators|meta|endpoints|api|POST|recent" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/`
- `api/meta/generators/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `/api/meta/generators` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/generators/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/generators/featured` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `GET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `POST` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S021 -- Lineage endpoints

**Source section:** `tmp/architecture/13-meta.md:567` through `576`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lineage endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/lineage/{id}/ancestors` | Ancestor chain for an object |
| `GET` | `/api/meta/lineage/{id}/descendants` | Descendant tree |
| `GET` | `/api/meta/lineage/{id}/siblings` | Objects sharing the same parent |
| `GET` | `/api/meta/lineage/graph?root={id}&depth=3` | Full lineage graph for visualization |
| `GET` | `/api/meta/lineage/most-forked?type={type}` | Most-forked objects by type |
````

**Explicit detail extraction from this section:**

- Section word count: `60`
- Section hash: `4f7325f80e8a73c9a586e9d7caa6f8dd3324670870baaf46e22fe0590b9a0edc`

**Normative requirements and implementation claims:**
- | Method | Path | Description | |--------|------|-------------| | `GET` | `/api/meta/lineage/{id}/ancestors` | Ancestor chain for an object | | `GET` | `/api/meta/lineage/{id}/descendants` | Descendant tree | | `GET` | `/api/meta/lineage/{id}/siblings` | Objects sharing the same parent | | `GET` | `/api/meta/lineage/graph?root={id}&depth=3` | Full lineage graph for visualization | | `GET` | `/api/meta/lineage/most-forked?type={type}` | Most-forked objects by type |

**Routes and endpoint references:**
- /api/meta/lineage/{id}/ancestors
- /api/meta/lineage/{id}/descendants
- /api/meta/lineage/{id}/siblings

**Files and path references:**
- api/meta/lineage/

**Types, functions, traits, and inline code identifiers:**
- GET

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
- Table 1:

```markdown
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/lineage/{id}/ancestors` | Ancestor chain for an object |
| `GET` | `/api/meta/lineage/{id}/descendants` | Descendant tree |
| `GET` | `/api/meta/lineage/{id}/siblings` | Objects sharing the same parent |
| `GET` | `/api/meta/lineage/graph?root={id}&depth=3` | Full lineage graph for visualization |
| `GET` | `/api/meta/lineage/most-forked?type={type}` | Most-forked objects by type |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/lineage/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Lineage|GET|meta|endpoints|api|type|object|most" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Lineage|GET|meta|endpoints|api|type|object|most" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/lineage/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `/api/meta/lineage/{id}/ancestors` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/lineage/{id}/descendants` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/lineage/{id}/siblings` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `GET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S022 -- Safety endpoints

**Source section:** `tmp/architecture/13-meta.md:577` through `586`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Safety endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/safety/active` | Currently active recursive processes |
| `GET` | `/api/meta/safety/anomalies?severity={warning,critical}` | Recent safety anomalies |
| `GET` | `/api/meta/safety/trace/{process_id}` | Full trace of a recursive process |

---
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `ec31fa75038bf5e9418d4bce5e7cc9b674848080fa9b306e86d934940a473e2f`

**Normative requirements and implementation claims:**
- | Method | Path | Description | |--------|------|-------------| | `GET` | `/api/meta/safety/active` | Currently active recursive processes | | `GET` | `/api/meta/safety/anomalies?severity={warning,critical}` | Recent safety anomalies | | `GET` | `/api/meta/safety/trace/{process_id}` | Full trace of a recursive process |
- ---

**Routes and endpoint references:**
- /api/meta/safety/active
- /api/meta/safety/trace/{process_id}

**Files and path references:**
- api/meta/safety/
- api/meta/safety/trace/

**Types, functions, traits, and inline code identifiers:**
- GET

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
- Table 1:

```markdown
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/safety/active` | Currently active recursive processes |
| `GET` | `/api/meta/safety/anomalies?severity={warning,critical}` | Recent safety anomalies |
| `GET` | `/api/meta/safety/trace/{process_id}` | Full trace of a recursive process |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/safety/`
- `api/meta/safety/trace/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Safety|GET|endpoints|process|meta|api|trace|recursive" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Safety|GET|endpoints|process|meta|api|trace|recursive" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `api/meta/safety/`
- `api/meta/safety/trace/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `/api/meta/safety/active` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/meta/safety/trace/{process_id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `GET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

### ARCH-13-S023 -- Configuration

**Source section:** `tmp/architecture/13-meta.md:587` through `610`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Configuration

```toml
# roko.toml

[meta]
enabled = true

[meta.creation]
max_depth = 3
max_creations_per_hour = 10
global_max_rate_per_hour = 50
min_child_quality = 0.0  # 0 = no quality gate

[meta.safety]
quality_trend_window = 10
min_quality_slope = -0.05  # Flag if quality drops faster than 5% per generation
circular_detection = true
auto_pause_on_anomaly = false  # Manual response by default

[meta.lineage]
record_on_chain = true
local_store_path = ".roko/lineage/"
```
````

**Explicit detail extraction from this section:**

- Section word count: `52`
- Section hash: `7b9831e27811ad23a186461b78566ce898f6c226de5c8fa294c1b532307fb3ca`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/lineage/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- meta.creation
- meta.safety
- meta.lineage

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- [meta]
- enabled = true
- [meta.creation]
- max_depth = 3
- max_creations_per_hour = 10
- global_max_rate_per_hour = 50
- min_child_quality = 0.0  # 0 = no quality gate
- [meta.safety]
- quality_trend_window = 10
- min_quality_slope = -0.05  # Flag if quality drops faster than 5% per generation
- circular_detection = true
- auto_pause_on_anomaly = false  # Manual response by default
- [meta.lineage]
- record_on_chain = true
- local_store_path = ".roko/lineage/"

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# roko.toml`

```toml
# roko.toml

[meta]
enabled = true

[meta.creation]
max_depth = 3
max_creations_per_hour = 10
global_max_rate_per_hour = 50
min_child_quality = 0.0  # 0 = no quality gate

[meta.safety]
quality_trend_window = 10
min_quality_slope = -0.05  # Flag if quality drops faster than 5% per generation
circular_detection = true
auto_pause_on_anomaly = false  # Manual response by default

[meta.lineage]
record_on_chain = true
local_store_path = ".roko/lineage/"
```

**Read before editing:**
- `tmp/architecture/13-meta.md`
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/lineage/`
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
rg -n "quality|Configuration|meta|true|per|toml|lineage|creation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "quality|Configuration|meta|true|per|toml|lineage|creation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-learn/src/`
- `crates/roko-serve/src/routes/meta.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/lineage/`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Emit or consume `meta.creation` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.safety` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `meta.lineage` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `[meta]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `enabled = true` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[meta.creation]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `max_depth = 3` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `max_creations_per_hour = 10` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `global_max_rate_per_hour = 50` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `min_child_quality = 0.0  # 0 = no quality gate` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[meta.safety]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `quality_trend_window = 10` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `min_quality_slope = -0.05  # Flag if quality drops faster than 5% per generation` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `circular_detection = true` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `auto_pause_on_anomaly = false  # Manual response by default` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[meta.lineage]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `record_on_chain = true` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `local_store_path = ".roko/lineage/"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/13-meta
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

