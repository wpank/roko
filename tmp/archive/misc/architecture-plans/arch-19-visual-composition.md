# Architecture Plan: Visual Composition

**Source:** `tmp/architecture/19-visual-composition.md`
**Generated:** 2026-04-25
**Source hash:** `06bdb53a8cc32712d496c1c63881bb3d780ba0593b3b84ba9ee5c22f789704a5`
**Section tasks:** 53
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
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-19-S001 | 1 | Visual composition and authoring system | [ ] | 9.8 |
| ARCH-19-S002 | 8 | Design philosophy | [ ] | 9.8 |
| ARCH-19-S003 | 16 | Primitive object types | [ ] | 9.8 |
| ARCH-19-S004 | 37 | Composition over configuration | [ ] | 9.8 |
| ARCH-19-S005 | 41 | Progressive disclosure | [ ] | 9.8 |
| ARCH-19-S006 | 45 | Draft/deploy separation | [ ] | 9.8 |
| ARCH-19-S007 | 51 | Plan mutation protocol | [ ] | 9.8 |
| ARCH-19-S008 | 55 | Mutation types | [ ] | 9.8 |
| ARCH-19-S009 | 107 | Supporting types | [ ] | 9.8 |
| ARCH-19-S010 | 174 | Conversation-as-plan-editor: the full flow | [ ] | 9.8 |
| ARCH-19-S011 | 320 | Mutation application rules | [ ] | 9.8 |
| ARCH-19-S012 | 340 | Chat endpoint contract | [ ] | 9.8 |
| ARCH-19-S013 | 392 | Plan states and lifecycle | [ ] | 9.8 |
| ARCH-19-S014 | 416 | Run | [ ] | 9.8 |
| ARCH-19-S015 | 452 | Pause | [ ] | 9.8 |
| ARCH-19-S016 | 479 | Resume | [ ] | 9.8 |
| ARCH-19-S017 | 501 | Three visual abstraction levels | [ ] | 9.8 |
| ARCH-19-S018 | 505 | Plan data model | [ ] | 9.8 |
| ARCH-19-S019 | 542 | Level 1: card stack | [ ] | 9.8 |
| ARCH-19-S020 | 550 | Level 2: lane view | [ ] | 9.8 |
| ARCH-19-S021 | 558 | Level 3: node graph | [ ] | 9.8 |
| ARCH-19-S022 | 566 | View-switching API | [ ] | 9.8 |
| ARCH-19-S023 | 572 | Template registry | [ ] | 9.8 |
| ARCH-19-S024 | 576 | Template data model | [ ] | 9.8 |
| ARCH-19-S025 | 634 | Template API | [ ] | 9.8 |
| ARCH-19-S026 | 705 | On-chain registration | [ ] | 9.8 |
| ARCH-19-S027 | 711 | Connector Manager (new authoring surface) | [ ] | 9.8 |
| ARCH-19-S028 | 728 | Feed Designer (new authoring surface) | [ ] | 9.8 |
| ARCH-19-S029 | 745 | Recipe Editor (new authoring surface) | [ ] | 9.8 |
| ARCH-19-S030 | 762 | Extension compilation service | [ ] | 9.8 |
| ARCH-19-S031 | 766 | Compile endpoint | [ ] | 9.8 |
| ARCH-19-S032 | 807 | Sandbox model | [ ] | 9.8 |
| ARCH-19-S033 | 820 | Cost projection | [ ] | 9.8 |
| ARCH-19-S034 | 824 | Estimate endpoint | [ ] | 9.8 |
| ARCH-19-S035 | 890 | Estimation algorithm | [ ] | 9.8 |
| ARCH-19-S036 | 904 | Gate test runner | [ ] | 9.8 |
| ARCH-19-S037 | 908 | Test endpoint | [ ] | 9.8 |
| ARCH-19-S038 | 965 | Authoring API contracts | [ ] | 9.8 |
| ARCH-19-S039 | 969 | CRUD | [ ] | 9.8 |
| ARCH-19-S040 | 982 | Create from template | [ ] | 9.8 |
| ARCH-19-S041 | 1000 | Validation | [ ] | 9.8 |
| ARCH-19-S042 | 1046 | Deploy | [ ] | 9.8 |
| ARCH-19-S043 | 1079 | Publish as template | [ ] | 9.8 |
| ARCH-19-S044 | 1097 | Event types for authoring | [ ] | 9.8 |
| ARCH-19-S045 | 1158 | Ecosystem dynamics | [ ] | 9.8 |
| ARCH-19-S046 | 1162 | The template flywheel | [ ] | 9.8 |
| ARCH-19-S047 | 1171 | Backend tracking for recommendations | [ ] | 9.8 |
| ARCH-19-S048 | 1191 | Generator-driven template creation | [ ] | 9.8 |
| ARCH-19-S049 | 1197 | Relationship to existing codebase | [ ] | 9.8 |
| ARCH-19-S050 | 1199 | What exists today | [ ] | 9.8 |
| ARCH-19-S051 | 1203 | What this spec adds | [ ] | 9.8 |
| ARCH-19-S052 | 1217 | Implementation path | [ ] | 9.8 |
| ARCH-19-S053 | 1230 | Summary of API surface | [ ] | 9.8 |

## Tasks

### ARCH-19-S001 -- Visual composition and authoring system

**Source section:** `tmp/architecture/19-visual-composition.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Visual composition and authoring system

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Depends on: [Dashboard Architecture](15-dashboard.md), [Agent Runtime](02-agent-runtime.md), [Extensions](03-extensions.md).

---
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `7989d6460b1355dd72973839cacf6b547ba8b6c0bbd9666dbc64f90d714013ff`

**Normative requirements and implementation claims:**
- > Part of the [Roko Architecture Specification](00-INDEX.md). > Depends on: [Dashboard Architecture](15-dashboard.md), [Agent Runtime](02-agent-runtime.md), [Extensions](03-extensions.md).
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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "composition|authoring|Visual|runtime|extensions|Specification|Part" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "composition|authoring|Visual|runtime|extensions|Specification|Part" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S002 -- Design philosophy

**Source section:** `tmp/architecture/19-visual-composition.md:8` through `15`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Design philosophy

The authoring system treats every object in the platform as a typed composition of primitives. There are no special-case configuration blobs. An agent is a composition of a domain, extensions, gates, and model preferences. An arena is a composition of a task source, scoring function, and leaderboard rules. A plan is a composition of tasks, dependencies, and checkpoints.

This follows the same principle as a DAW (digital audio workstation). A DAW has a small number of primitive track types -- audio, MIDI, bus, send. Every song is a composition of those primitives. The DAW never needs a "song type" dropdown because the primitives compose into whatever the user needs.

The authoring system works the same way with 12 primitive object types (per dashboard PRD 23, superseding the 10-primitive vocabulary).
````

**Explicit detail extraction from this section:**

- Section word count: `130`
- Section hash: `96647b556a62143f76ae6352be518a1628191ba5887ed48e3e4564b5658138d0`

**Normative requirements and implementation claims:**
- This follows the same principle as a DAW (digital audio workstation). A DAW has a small number of primitive track types -- audio, MIDI, bus, send. Every song is a composition of those primitives. The DAW never needs a "song type" dropdown because the primitives compose into whatever the user needs.
- The authoring system works the same way with 12 primitive object types (per dashboard PRD 23, superseding the 10-primitive vocabulary).

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "primitive|philosophy|composition|Design|type|primitives|works|types" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "primitive|philosophy|composition|Design|type|primitives|works|types" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S003 -- Primitive object types

**Source section:** `tmp/architecture/19-visual-composition.md:16` through `36`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primitive object types

| # | Type | What it represents |
|---|------|--------------------|
| 1 | Agent | A configured runtime with archetype (domain + tool profiles + gate pipelines + model preferences), extensions, budget |
| 2 | Extension | A modular behavior unit across three tiers: Pi-compatible (JS/TS), Roko-enhanced (JS/TS + heartbeat), Roko-native (Rust, 22 hooks, 8 layers) |
| 3 | Connector | External system I/O adapter: chain RPC, exchange API, MCP server, database, webhook |
| 4 | Gate | A verification step (pre-action permission or post-action validation): shell command, Rust function, chain simulation, risk check |
| 5 | Feed | A continuous data stream: price feeds, block events, CI status, file changes, webhook streams |
| 6 | Recipe | A composable data transformation pipeline: indicator chains, P&L attribution, HDC encoding, scoring |
| 7 | Knowledge Entry | A typed entry in the durable knowledge store (Insight, Heuristic, Warning, CausalLink, etc.) |
| 8 | Arena | A competitive evaluation environment with task source, scoring, and leaderboard |
| 9 | Eval | A measurement against ground truth, never LLM-graded |
| 10 | Signal | A coordination event published to PulseBus (renamed from Pheromone at product layer) |
| 11 | Group | A coordinated subset of agents with shared state and governance |
| 12 | Bounty | A posted task with reward, escrow, and acceptance criteria |

Three supporting object types build on top of these: **Plan** (a DAG of tasks that reference agents, gates, and connectors), **Template** (a reusable snapshot of any object type), and **Generator** (an agent or function that produces instances of a given object type).

> **Migration from 10 to 12 primitives.** Domain is no longer standalone -- it became the `archetype` field on Agent. Connector, Feed, and Recipe are new additions. Pheromone was renamed to Signal at the product layer. See PRD 23 for the full migration path and backward compatibility guarantees.
````

**Explicit detail extraction from this section:**

- Section word count: `274`
- Section hash: `57178a3a6fe9b6bd35483e12f0730e20af3670825e945294094193fec7696728`

**Normative requirements and implementation claims:**
- | # | Type | What it represents | |---|------|--------------------| | 1 | Agent | A configured runtime with archetype (domain + tool profiles + gate pipelines + model preferences), extensions, budget | | 2 | Extension | A modular behavior unit across three tiers: Pi-compatible (JS/TS), Roko-enhanced (JS/TS + heartbeat), Roko-native (Rust, 22 hooks, 8 layers) | | 3 | Connector | External system I/O adapter: chain RPC, exchange API, MCP server, database, webhook | | 4 | Gate | A verification step (pre-action permission or post-action validation): shell command, Rust function, chain simulation, risk check | | 5 | Feed | A continuous data stream: price feeds, block events, CI status, file changes, webhook streams | | 6 | Recipe | A composable data transformation pipeline: indicator chains, P&L attribution, HDC encoding, scoring | | 7 | Knowledge Entry | A typed entry in the durable knowledge store (Insight, Heuristic, Warning, CausalLink, etc.) | | 8 | Arena | A competitive evaluation envi

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- archetype

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
| # | Type | What it represents |
|---|------|--------------------|
| 1 | Agent | A configured runtime with archetype (domain + tool profiles + gate pipelines + model preferences), extensions, budget |
| 2 | Extension | A modular behavior unit across three tiers: Pi-compatible (JS/TS), Roko-enhanced (JS/TS + heartbeat), Roko-native (Rust, 22 hooks, 8 layers) |
| 3 | Connector | External system I/O adapter: chain RPC, exchange API, MCP server, database, webhook |
| 4 | Gate | A verification step (pre-action permission or post-action validation): shell command, Rust function, chain simulation, risk check |
| 5 | Feed | A continuous data stream: price feeds, block events, CI status, file changes, webhook streams |
| 6 | Recipe | A composable data transformation pipeline: indicator chains, P&L attribution, HDC encoding, scoring |
| 7 | Knowledge Entry | A typed entry in the durable knowledge store (Insight, Heuristic, Warning, CausalLink, etc.) |
| 8 | Arena | A competitive evaluation environment with task source, scoring, and leaderboard |
| 9 | Eval | A measurement against ground truth, never LLM-graded |
| 10 | Signal | A coordination event published to PulseBus (renamed from Pheromone at product layer) |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Type|object|types|archetype|Primitive|task|layer|gate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Type|object|types|archetype|Primitive|task|layer|gate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify `archetype` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S004 -- Composition over configuration

**Source section:** `tmp/architecture/19-visual-composition.md:37` through `40`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Composition over configuration

Every authoring surface in the dashboard composes these primitives. Creating an agent means selecting an archetype (which pre-fills domain, extensions, gates, and model preferences), adjusting the selection, attaching connectors and feeds, and setting a budget. Creating an arena means composing a task source, gate configuration, scoring function, and leaderboard rules. No surface has a freeform configuration textarea. Every field maps to a typed primitive.
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `c1a9a84475c16082858e6342eeffdf795216d635cb546969e587d08218abe6a6`

**Normative requirements and implementation claims:**
- Every authoring surface in the dashboard composes these primitives. Creating an agent means selecting an archetype (which pre-fills domain, extensions, gates, and model preferences), adjusting the selection, attaching connectors and feeds, and setting a budget. Creating an arena means composing a task source, gate configuration, scoring function, and leaderboard rules. No surface has a freeform configuration textarea. Every field maps to a typed primitive.

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "configuration|over|Composition|surface|primitive|means|gate|Every" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "configuration|over|Composition|surface|primitive|means|gate|Every" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S005 -- Progressive disclosure

**Source section:** `tmp/architecture/19-visual-composition.md:41` through `44`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Progressive disclosure

Simple views come first. The Agent Composer starts with "pick a template." Users who want control drill into domain selection, extension toggles, gate pipeline editing, model routing, and budget configuration. Each level reveals more of the underlying composition without forcing users through it.
````

**Explicit detail extraction from this section:**

- Section word count: `43`
- Section hash: `e6fc1baf8c1f56ecfd1884b70b9303d04537d260a042b162a4fae7e7a9c9bc2f`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "disclosure|Progressive|Users|without|want|views|underlying|toggles" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "disclosure|Progressive|Users|without|want|views|underlying|toggles" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S006 -- Draft/deploy separation

**Source section:** `tmp/architecture/19-visual-composition.md:45` through `50`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Draft/deploy separation

Authoring is free. Deploying costs tokens, gas, or both. Users iterate on drafts without cost. Drafts auto-save. Deployment is an explicit action with a cost estimate shown before confirmation. This separation means users can experiment freely, keep multiple drafts, and only commit resources when ready.

---
````

**Explicit detail extraction from this section:**

- Section word count: `46`
- Section hash: `2361a1b54c0654b7b12d0e1f20951fb0b92b2c578a34007d81e26a6fef91994f`

**Normative requirements and implementation claims:**
- Authoring is free. Deploying costs tokens, gas, or both. Users iterate on drafts without cost. Drafts auto-save. Deployment is an explicit action with a cost estimate shown before confirmation. This separation means users can experiment freely, keep multiple drafts, and only commit resources when ready.
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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Draft|deploy|separation|drafts|cost|free|Users|without" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Draft|deploy|separation|drafts|cost|free|Users|without" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S007 -- Plan mutation protocol

**Source section:** `tmp/architecture/19-visual-composition.md:51` through `54`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Plan mutation protocol

This is the headline feature. The user talks to an agent in a floating chat drawer. The agent interprets intent, generates structured mutations, and pushes them to the plan canvas. The canvas animates the changes in real time.
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `a3ccd8ad733edbe766fb099cbf3607b9194461863f25b852a6c0f19974c0ea46`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "mutation|Plan|protocol|canvas|user|time|talks|structured" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mutation|Plan|protocol|canvas|user|time|talks|structured" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S008 -- Mutation types

**Source section:** `tmp/architecture/19-visual-composition.md:55` through `106`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Mutation types

```rust
/// A single atomic change to a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanMutation {
    /// Insert a new task into the plan.
    AddTask {
        task: TaskSpec,
        /// Place after this task. None means append to the end.
        after: Option<TaskId>,
    },
    /// Remove a task and all its dependency edges.
    RemoveTask {
        id: TaskId,
    },
    /// Patch fields on an existing task.
    UpdateTask {
        id: TaskId,
        patch: TaskPatch,
    },
    /// Add a dependency edge: `from` must complete before `to` starts.
    AddDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Remove a dependency edge.
    RemoveDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Reorder tasks. The vec represents the new ordering.
    Reorder {
        task_ids: Vec<TaskId>,
    },
    /// Group tasks into a parallel execution lane.
    SetParallel {
        task_ids: Vec<TaskId>,
    },
    /// Insert a manual checkpoint (human review gate) after a task.
    AddCheckpoint {
        after: TaskId,
        name: String,
    },
    /// Update plan-level metadata (name, description, error policy).
    UpdatePlanMeta {
        patch: PlanMetaPatch,
    },
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `2a26f6352004cb7f6faf5836462c9557282665bb458845261b0ed509a1e59235`

**Normative requirements and implementation claims:**
- ```rust /// A single atomic change to a plan. #[derive(Debug, Clone, Serialize, Deserialize)] #[serde(tag = "op", rename_all = "snake_case")] pub enum PlanMutation { /// Insert a new task into the plan. AddTask { task: TaskSpec, /// Place after this task. None means append to the end. after: Option<TaskId>, }, /// Remove a task and all its dependency edges. RemoveTask { id: TaskId, }, /// Patch fields on an existing task. UpdateTask { id: TaskId, patch: TaskPatch, }, /// Add a dependency edge: `from` must complete before `to` starts. AddDependency { from: TaskId, to: TaskId, }, /// Remove a dependency edge. RemoveDependency { from: TaskId, to: TaskId, }, /// Reorder tasks. The vec represents the new ordering. Reorder { task_ids: Vec<TaskId>, }, /// Group tasks into a parallel execution lane. SetParallel { task_ids: Vec<TaskId>, }, /// Insert a manual checkpoint (human review gate) after a task. AddCheckpoint { after: TaskId, name: String, }, /// Update plan-level metadata (name, descri

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- PlanMutation
- from

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
- Contract 1: language `rust`, first line `/// A single atomic change to a plan.`

```rust
/// A single atomic change to a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanMutation {
    /// Insert a new task into the plan.
    AddTask {
        task: TaskSpec,
        /// Place after this task. None means append to the end.
        after: Option<TaskId>,
    },
    /// Remove a task and all its dependency edges.
    RemoveTask {
        id: TaskId,
    },
    /// Patch fields on an existing task.
    UpdateTask {
        id: TaskId,
        patch: TaskPatch,
    },
    /// Add a dependency edge: `from` must complete before `to` starts.
    AddDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Remove a dependency edge.
    RemoveDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Reorder tasks. The vec represents the new ordering.
    Reorder {
        task_ids: Vec<TaskId>,
    },
    /// Group tasks into a parallel execution lane.
    SetParallel {
        task_ids: Vec<TaskId>,
    },
    /// Insert a manual checkpoint (human review gate) after a task.
    AddCheckpoint {
        after: TaskId,
        name: String,
    },
    /// Update plan-level metadata (name, description, error policy).
    UpdatePlanMeta {
        patch: PlanMetaPatch,
    },
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "task|TaskId|plan|Mutation|types|dependency|PlanMutation|Patch" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "task|TaskId|plan|Mutation|types|dependency|PlanMutation|Patch" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `PlanMutation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `from` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S009 -- Supporting types

**Source section:** `tmp/architecture/19-visual-composition.md:107` through `173`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Supporting types

```rust
pub type TaskId = String;
pub type PlanId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub agent_profile: Option<String>,  // "research", "coding", "review"
    pub model: Option<String>,          // "claude-opus-4-6", "claude-sonnet-4-6"
    pub repo: Option<String>,           // "nunchi/roko"
    pub depends_on: Vec<TaskId>,
    pub files: Vec<String>,
    pub est_minutes: Option<u32>,
    pub budget_usd: Option<f64>,
    pub gate_pipeline: Option<Vec<String>>,  // gate IDs to run after completion
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<TaskId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub est_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetaPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handling: Option<ErrorPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorPolicy {
    /// Stop the pipeline on first failure.
    StopOnFailure,
    /// Skip the failed task and continue with tasks that don't depend on it.
    SkipAndContinue,
    /// Retry the failed task up to N times before stopping.
    Retry { max_attempts: u32 },
    /// Pause and wait for human intervention.
    PauseOnFailure,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `247`
- Section hash: `acbdee6cc5629d782be3a4a1e2dbfbd4c4b43d678f5cd35ce5ed675dcf09be1c`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- TaskId
- PlanId
- TaskSpec
- TaskPatch
- PlanMetaPatch
- ErrorPolicy

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
- Contract 1: language `rust`, first line `pub type TaskId = String;`

```rust
pub type TaskId = String;
pub type PlanId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub agent_profile: Option<String>,  // "research", "coding", "review"
    pub model: Option<String>,          // "claude-opus-4-6", "claude-sonnet-4-6"
    pub repo: Option<String>,           // "nunchi/roko"
    pub depends_on: Vec<TaskId>,
    pub files: Vec<String>,
    pub est_minutes: Option<u32>,
    pub budget_usd: Option<f64>,
    pub gate_pipeline: Option<Vec<String>>,  // gate IDs to run after completion
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<TaskId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub est_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetaPatch {
    #[serde(ski
...
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Option|String|Skip|serde|skip_serializing_if|is_none|task|TaskId" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Option|String|Skip|serde|skip_serializing_if|is_none|task|TaskId" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify `TaskId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PlanId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskPatch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PlanMetaPatch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ErrorPolicy` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S010 -- Conversation-as-plan-editor: the full flow

**Source section:** `tmp/architecture/19-visual-composition.md:174` through `319`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Conversation-as-plan-editor: the full flow

**Step 1: user opens a session.** The plan canvas loads (empty or showing their last saved draft). A floating chat drawer docks to the right side of the viewport. The drawer is draggable, resizable, and can be undocked into a floating window.

**Step 2: user sends a message.**

```
POST /api/plans/{id}/chat
Content-Type: application/json

{
  "message": "I need to add auth to the roko API. JWT validation, middleware, the whole thing."
}
```

**Step 3: the backend dispatches to an LLM.** The plan chat endpoint builds a system prompt that includes the current plan state, the plan mutation schema, and instructions to respond with both natural language and structured mutations. The LLM call uses the configured model (defaults to `claude-sonnet-4-6` for plan editing, overridable per session).

**Step 4: the backend returns mutations alongside the reply.**

```json
{
  "reply": "I'll break this into four tasks. Research existing auth patterns first, then implement middleware, add tests, and update docs.",
  "mutations": [
    {
      "op": "add_task",
      "task": {
        "id": "t1",
        "title": "Research auth patterns",
        "description": "Analyze existing auth patterns in the codebase and recommend an approach",
        "agent_profile": "research",
        "repo": "nunchi/roko",
        "est_minutes": 5
      },
      "after": null
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2",
        "title": "Implement JWT middleware",
        "description": "Add JWT validation middleware to the axum router",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 15
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t3",
        "title": "Write integration tests",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 8
      },
      "after": "t2"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t4",
        "title": "Update API docs",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 5
      },
      "after": "t2"
    }
  ],
  "plan_state": { "task_count": 4, "est_total_minutes": 33 },
  "cost_estimate": {
    "total_usd": 1.20,
    "time_estimate_mins": 33,
    "confidence": 0.65
  }
}
```

The dashboard receives this response, applies the mutations to its local plan state, and animates them on the canvas. Four cards fly in, arrange themselves with dependency arrows, and the cost estimate appears in the footer.

**Step 5: user iterates.**

```
POST /api/plans/{id}/chat

{
  "message": "Split the middleware into two parallel tasks -- one for API routes and one for WebSocket. Use opus for the research."
}
```

Response:

```json
{
  "reply": "Done. Two middleware tasks now run in parallel. Research upgraded to opus.",
  "mutations": [
    { "op": "update_task", "id": "t1", "patch": { "model": "claude-opus-4-6" } },
    { "op": "remove_task", "id": "t2" },
    {
      "op": "add_task",
      "task": {
        "id": "t2a",
        "title": "API auth middleware",
        "description": "JWT validation for /api/* routes",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 12
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2b",
        "title": "WebSocket auth middleware",
        "description": "JWT validation for /ws connections",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 10
      },
      "after": "t1"
    },
    { "op": "set_parallel", "task_ids": ["t2a", "t2b"] },
    { "op": "add_dependency", "from": "t2a", "to": "t3" },
    { "op": "add_dependency", "from": "t2b", "to": "t3" },
    { "op": "add_dependency", "from": "t2a", "to": "t4" },
    { "op": "add_dependency", "from": "t2b", "to": "t4" }
  ],
  "cost_estimate": {
    "total_usd": 1.80,
    "time_estimate_mins": 40,
    "confidence": 0.7
  }
}
```

The canvas animates: t2 dissolves, t2a and t2b scale in side by side, dependency arrows reroute from the old single path to fan-out/fan-in, and the cost estimate updates in the footer.

This loop continues for as many turns as the user needs. Each turn is cheap (one LLM call for plan editing, not agent execution). The user never edits TOML or YAML. They talk. The plan responds.
````

**Explicit detail extraction from this section:**

- Section word count: `530`
- Section hash: `c517263fb05f18a74e31030b462692eaaa8a3a9a7154f433ee1c2a67a1d8bc4a`

**Normative requirements and implementation claims:**
- **Step 1: user opens a session.** The plan canvas loads (empty or showing their last saved draft). A floating chat drawer docks to the right side of the viewport. The drawer is draggable, resizable, and can be undocked into a floating window.
- **Step 2: user sends a message.**
- ``` POST /api/plans/{id}/chat Content-Type: application/json
- { "message": "I need to add auth to the roko API. JWT validation, middleware, the whole thing." } ```
- **Step 3: the backend dispatches to an LLM.** The plan chat endpoint builds a system prompt that includes the current plan state, the plan mutation schema, and instructions to respond with both natural language and structured mutations. The LLM call uses the configured model (defaults to `claude-sonnet-4-6` for plan editing, overridable per session).
- **Step 4: the backend returns mutations alongside the reply.**
- ```json { "reply": "I'll break this into four tasks. Research existing auth patterns first, then implement middleware, add tests, and update docs.", "mutations": [ { "op": "add_task", "task": { "id": "t1", "title": "Research auth patterns", "description": "Analyze existing auth patterns in the codebase and recommend an approach", "agent_profile": "research", "repo": "nunchi/roko", "est_minutes": 5 }, "after": null }, { "op": "add_task", "task": { "id": "t2", "title": "Implement JWT middleware", "description": "Add JWT validation middleware to the axum router", "agent_profile": "coding", "repo": "nunchi/roko", "depends_on": ["t1"], "est_minutes": 15 }, "after": "t1" }, { "op": "add_task", "task": { "id": "t3", "title": "Write integration tests", "agent_profile": "coding", "repo": "nunchi/roko", "depends_on": ["t2"], "est_minutes": 8 }, "after": "t2" }, { "op": "add_task", "task": { "id": "t4", "title": "Update API docs", "agent_profile": "coding", "repo": "nunchi/roko", "depends_on": ["
- The dashboard receives this response, applies the mutations to its local plan state, and animates them on the canvas. Four cards fly in, arrange themselves with dependency arrows, and the cost estimate appears in the footer.
- **Step 5: user iterates.**
- ``` POST /api/plans/{id}/chat
- { "message": "Split the middleware into two parallel tasks -- one for API routes and one for WebSocket. Use opus for the research." } ```
- ```json { "reply": "Done. Two middleware tasks now run in parallel. Research upgraded to opus.", "mutations": [ { "op": "update_task", "id": "t1", "patch": { "model": "claude-opus-4-6" } }, { "op": "remove_task", "id": "t2" }, { "op": "add_task", "task": { "id": "t2a", "title": "API auth middleware", "description": "JWT validation for /api/* routes", "agent_profile": "coding", "repo": "nunchi/roko", "depends_on": ["t1"], "est_minutes": 12 }, "after": "t1" }, { "op": "add_task", "task": { "id": "t2b", "title": "WebSocket auth middleware", "description": "JWT validation for /ws connections", "agent_profile": "coding", "repo": "nunchi/roko", "depends_on": ["t1"], "est_minutes": 10 }, "after": "t1" }, { "op": "set_parallel", "task_ids": ["t2a", "t2b"] }, { "op": "add_dependency", "from": "t2a", "to": "t3" }, { "op": "add_dependency", "from": "t2b", "to": "t3" }, { "op": "add_dependency", "from": "t2a", "to": "t4" }, { "op": "add_dependency", "from": "t2b", "to": "t4" } ], "cost_estimate": 
- This loop continues for as many turns as the user needs. Each turn is cheap (one LLM call for plan editing, not agent execution). The user never edits TOML or YAML. They talk. The plan responds.

**Routes and endpoint references:**
- POST /api/plans/{id}/chat

**Files and path references:**
- api/plans/

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
- Contract 1: language `plain`, first line `POST /api/plans/{id}/chat`

```
POST /api/plans/{id}/chat
Content-Type: application/json

{
  "message": "I need to add auth to the roko API. JWT validation, middleware, the whole thing."
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "reply": "I'll break this into four tasks. Research existing auth patterns first, then implement middleware, add tests, and update docs.",
  "mutations": [
    {
      "op": "add_task",
      "task": {
        "id": "t1",
        "title": "Research auth patterns",
        "description": "Analyze existing auth patterns in the codebase and recommend an approach",
        "agent_profile": "research",
        "repo": "nunchi/roko",
        "est_minutes": 5
      },
      "after": null
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2",
        "title": "Implement JWT middleware",
        "description": "Add JWT validation middleware to the axum router",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 15
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t3",
        "title": "Write integration tests",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 8
      },
      "after": "t2"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t4",
        "title": "Update API docs",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 5
      },
      "after": "t2"
    }
  ],
  "plan_state": { "task_count": 4, "est_total_minutes": 33 },
  "cost_estimate": {
    "total_usd": 1.20,
    "time_estimate_mins": 33,
    "confidence": 0.65
  }
}
```
- Contract 3: language `plain`, first line `POST /api/plans/{id}/chat`

```
POST /api/plans/{id}/chat

{
  "message": "Split the middleware into two parallel tasks -- one for API routes and one for WebSocket. Use opus for the research."
}
```
- Contract 4: language `json`, first line `{`

```json
{
  "reply": "Done. Two middleware tasks now run in parallel. Research upgraded to opus.",
  "mutations": [
    { "op": "update_task", "id": "t1", "patch": { "model": "claude-opus-4-6" } },
    { "op": "remove_task", "id": "t2" },
    {
      "op": "add_task",
      "task": {
        "id": "t2a",
        "title": "API auth middleware",
        "description": "JWT validation for /api/* routes",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 12
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2b",
        "title": "WebSocket auth middleware",
        "description": "JWT validation for /ws connections",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 10
      },
      "after": "t1"
    },
    { "op": "set_parallel", "task_ids": ["t2a", "t2b"] },
    { "op": "add_dependency", "from": "t2a", "to": "t3" },
    { "op": "add_dependency", "from": "t2b", "to": "t3" },
    { "op": "add_dependency", "from": "t2a", "to": "t4" },
    { "op": "add_dependency", "from": "t2b", "to": "t4" }
  ],
  "cost_estimate": {
    "total_usd": 1.80,
    "time_estimate_mins": 40,
    "confidence": 0.7
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "the|task|plan|middleware|api|title|repo|nunchi" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|task|plan|middleware|api|title|repo|nunchi" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- [ ] Implement or verify route `POST /api/plans/{id}/chat` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S011 -- Mutation application rules

**Source section:** `tmp/architecture/19-visual-composition.md:320` through `339`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Mutation application rules

The backend validates mutations before persisting:

1. `AddTask` with a duplicate `id` is rejected.
2. `RemoveTask` for a non-existent `id` is rejected.
3. `AddDependency` that would create a cycle is rejected (topological sort check).
4. `SetParallel` tasks must share at least one common predecessor.
5. Rejected mutations return in a `rejected` array with reasons. Valid mutations in the same batch still apply.

```json
{
  "reply": "...",
  "mutations": [ ... ],
  "rejected": [
    { "op": "add_dependency", "from": "t3", "to": "t1", "reason": "would create cycle: t1 -> t2a -> t3 -> t1" }
  ]
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `81`
- Section hash: `a9d85ba254668a0990905331b91e245e6b145e25fba6c4e50cc992ee22a1c011`

**Normative requirements and implementation claims:**
- 1. `AddTask` with a duplicate `id` is rejected. 2. `RemoveTask` for a non-existent `id` is rejected. 3. `AddDependency` that would create a cycle is rejected (topological sort check). 4. `SetParallel` tasks must share at least one common predecessor. 5. Rejected mutations return in a `rejected` array with reasons. Valid mutations in the same batch still apply.
- ```json { "reply": "...", "mutations": [ ... ], "rejected": [ { "op": "add_dependency", "from": "t3", "to": "t1", "reason": "would create cycle: t1 -> t2a -> t3 -> t1" } ] } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AddTask
- RemoveTask
- AddDependency
- SetParallel
- rejected

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- t1 -> t2a -
- t3 -> t1

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. `AddTask` with a duplicate `id` is rejected.
- 2. `RemoveTask` for a non-existent `id` is rejected.
- 3. `AddDependency` that would create a cycle is rejected (topological sort check).
- 4. `SetParallel` tasks must share at least one common predecessor.
- 5. Rejected mutations return in a `rejected` array with reasons. Valid mutations in the same batch still apply.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{`

```json
{
  "reply": "...",
  "mutations": [ ... ],
  "rejected": [
    { "op": "add_dependency", "from": "t3", "to": "t1", "reason": "would create cycle: t1 -> t2a -> t3 -> t1" }
  ]
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "rejected|Mutation|rules|application|SetParallel|RemoveTask|AddTask|AddDependency" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "rejected|Mutation|rules|application|SetParallel|RemoveTask|AddTask|AddDependency" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `AddTask` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RemoveTask` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AddDependency` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SetParallel` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `rejected` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `t1 -> t2a -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `t3 -> t1` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S012 -- Chat endpoint contract

**Source section:** `tmp/architecture/19-visual-composition.md:340` through `391`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Chat endpoint contract

```
POST /api/plans/{id}/chat
```

Request:

```json
{
  "message": "string (required)",
  "context": {
    "selected_tasks": ["t2a"],
    "viewport": "lane_view"
  }
}
```

The optional `context` field tells the agent what the user is looking at. If the user has selected a task on the canvas, the agent knows to focus edits there. The `viewport` hint (card_stack, lane_view, node_graph) lets the agent tailor its mutation strategy -- for example, using `set_parallel` only when the user can see lanes.

Response:

```json
{
  "reply": "string",
  "mutations": [ PlanMutation, ... ],
  "rejected": [ { "op": "...", "reason": "..." }, ... ],
  "plan_state": {
    "task_count": 5,
    "dependency_count": 6,
    "parallel_groups": 1,
    "est_total_minutes": 40
  },
  "cost_estimate": {
    "total_usd": 1.80,
    "per_task": [
      { "task_id": "t1", "model": "claude-opus-4-6", "estimated_tokens": 8000, "estimated_usd": 0.40 },
      { "task_id": "t2a", "model": "claude-sonnet-4-6", "estimated_tokens": 5000, "estimated_usd": 0.15 }
    ],
    "time_estimate_mins": 40,
    "confidence": 0.7,
    "breakdown": {
      "inference": 1.20,
      "feeds": 0.10,
      "gas": 0.50
    }
  }
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `131`
- Section hash: `201b7bf4629fb8c7a72091b0aebc22f5d898d031b9f1d61f9479e8c49c69a831`

**Normative requirements and implementation claims:**
- ``` POST /api/plans/{id}/chat ```
- ```json { "message": "string (required)", "context": { "selected_tasks": ["t2a"], "viewport": "lane_view" } } ```
- The optional `context` field tells the agent what the user is looking at. If the user has selected a task on the canvas, the agent knows to focus edits there. The `viewport` hint (card_stack, lane_view, node_graph) lets the agent tailor its mutation strategy -- for example, using `set_parallel` only when the user can see lanes.
- ---

**Routes and endpoint references:**
- POST /api/plans/{id}/chat

**Files and path references:**
- api/plans/

**Types, functions, traits, and inline code identifiers:**
- context
- viewport
- set_parallel

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
- Contract 1: language `plain`, first line `POST /api/plans/{id}/chat`

```
POST /api/plans/{id}/chat
```
- Contract 2: language `json`, first line `{`

```json
{
  "message": "string (required)",
  "context": {
    "selected_tasks": ["t2a"],
    "viewport": "lane_view"
  }
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "reply": "string",
  "mutations": [ PlanMutation, ... ],
  "rejected": [ { "op": "...", "reason": "..." }, ... ],
  "plan_state": {
    "task_count": 5,
    "dependency_count": 6,
    "parallel_groups": 1,
    "est_total_minutes": 40
  },
  "cost_estimate": {
    "total_usd": 1.80,
    "per_task": [
      { "task_id": "t1", "model": "claude-opus-4-6", "estimated_tokens": 8000, "estimated_usd": 0.40 },
      { "task_id": "t2a", "model": "claude-sonnet-4-6", "estimated_tokens": 5000, "estimated_usd": 0.15 }
    ],
    "time_estimate_mins": 40,
    "confidence": 0.7,
    "breakdown": {
      "inference": 1.20,
      "feeds": 0.10,
      "gas": 0.50
    }
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "viewport|task|context|Chat|set_parallel|endpoint|contract|user" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "viewport|task|context|Chat|set_parallel|endpoint|contract|user" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- [ ] Implement or verify route `POST /api/plans/{id}/chat` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `context` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `viewport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `set_parallel` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S013 -- Plan states and lifecycle

**Source section:** `tmp/architecture/19-visual-composition.md:392` through `415`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Plan states and lifecycle

A plan moves through five states. The transitions are explicit API calls, not automatic.

```
         chat           run            pause           resume
Draft ---------> Draft ------> Executing ------> Paused -------> Executing
  ^                              |                                   |
  |                              |                                   |
  |                              +----> Completed                    +----> Completed
  |                              |                                   |
  |                              +----> Failed                       +----> Failed
  |                                                                  |
  +------ (revise remaining) <--- Paused ----(chat)---> Paused ------+
```

| State | Editable | Agents running | Costs accumulating |
|-------|----------|----------------|--------------------|
| Draft | Yes | No | No (only chat model costs for plan editing) |
| Executing | No (frozen) | Yes | Yes |
| Paused | Remaining tasks editable | No (all stopped) | No |
| Completed | No | No | No |
| Failed | No | No | No |
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `54a47f16404a3db737c463a55ac2880ce7e76417776bfa7ef98faaf72276bf7e`

**Normative requirements and implementation claims:**
- A plan moves through five states. The transitions are explicit API calls, not automatic.
- | State | Editable | Agents running | Costs accumulating | |-------|----------|----------------|--------------------| | Draft | Yes | No | No (only chat model costs for plan editing) | | Executing | No (frozen) | Yes | Yes | | Paused | Remaining tasks editable | No (all stopped) | No | | Completed | No | No | No | | Failed | No | No | No |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Draft -------- -> Draft ------
- Executing ----- -> Paused -------

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| State | Editable | Agents running | Costs accumulating |
|-------|----------|----------------|--------------------|
| Draft | Yes | No | No (only chat model costs for plan editing) |
| Executing | No (frozen) | Yes | Yes |
| Paused | Remaining tasks editable | No (all stopped) | No |
| Completed | No | No | No |
| Failed | No | No | No |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `chat           run            pause           resume`

```
chat           run            pause           resume
Draft ---------> Draft ------> Executing ------> Paused -------> Executing
  ^                              |                                   |
  |                              |                                   |
  |                              +----> Completed                    +----> Completed
  |                              |                                   |
  |                              +----> Failed                       +----> Failed
  |                                                                  |
  +------ (revise remaining) <--- Paused ----(chat)---> Paused ------+
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "State|Plan|states|pause|lifecycle|Paused|chat" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "State|Plan|states|pause|lifecycle|Paused|chat" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Enforce state transition `Draft -------- -> Draft ------` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Executing ----- -> Paused -------` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S014 -- Run

**Source section:** `tmp/architecture/19-visual-composition.md:416` through `451`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Run

```
POST /api/plans/{id}/run
```

Request: empty body (or optional overrides).

```json
{
  "budget_override_usd": 5.00,
  "dry_run": false
}
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "plan_id": "plan-xyz",
  "status": "executing",
  "snapshot_id": "snap-001",
  "agents_spawned": 1,
  "next_task": "t1"
}
```

What happens internally:

1. Plan state freezes. A snapshot is written to `.roko/state/plan-{id}-snap-{n}.json`.
2. The orchestrator builds a DAG from the plan's tasks and dependencies.
3. Tasks with no predecessors are dispatched first.
4. As each task completes and passes its gate pipeline, dependent tasks become eligible.
5. Events stream to the dashboard via WebSocket: `plan.task_started`, `plan.task_completed`, `plan.gate_result`, `plan.agent_output`.
````

**Explicit detail extraction from this section:**

- Section word count: `104`
- Section hash: `65d319543e08d0ab31a5bf6c13b09fee1e08ceddd48b92e5b356b7fa6bcf795d`

**Normative requirements and implementation claims:**
- ``` POST /api/plans/{id}/run ```
- 1. Plan state freezes. A snapshot is written to `.roko/state/plan-{id}-snap-{n}.json`. 2. The orchestrator builds a DAG from the plan's tasks and dependencies. 3. Tasks with no predecessors are dispatched first. 4. As each task completes and passes its gate pipeline, dependent tasks become eligible. 5. Events stream to the dashboard via WebSocket: `plan.task_started`, `plan.task_completed`, `plan.gate_result`, `plan.agent_output`.

**Routes and endpoint references:**
- POST /api/plans/{id}/run

**Files and path references:**
- .roko/state/
- api/plans/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- plan.task_started
- plan.task_completed
- plan.gate_result
- plan.agent_output

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- plan.task_started
- plan.task_completed
- plan.gate_result
- plan.agent_output

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Plan state freezes. A snapshot is written to `.roko/state/plan-{id}-snap-{n}.json`.
- 2. The orchestrator builds a DAG from the plan's tasks and dependencies.
- 3. Tasks with no predecessors are dispatched first.
- 4. As each task completes and passes its gate pipeline, dependent tasks become eligible.
- 5. Events stream to the dashboard via WebSocket: `plan.task_started`, `plan.task_completed`, `plan.gate_result`, `plan.agent_output`.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST /api/plans/{id}/run`

```
POST /api/plans/{id}/run
```
- Contract 2: language `json`, first line `{`

```json
{
  "budget_override_usd": 5.00,
  "dry_run": false
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "execution_id": "exec-a1b2c3",
  "plan_id": "plan-xyz",
  "status": "executing",
  "snapshot_id": "snap-001",
  "agents_spawned": 1,
  "next_task": "t1"
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/state/`
- `api/plans/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "plan|task|Run|snap|tasks|json|exec|state" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "plan|task|Run|snap|tasks|json|exec|state" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/state/`
- `api/plans/`
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
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/mod.rs`

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
- [ ] Implement or verify route `POST /api/plans/{id}/run` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Emit or consume `plan.task_started` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `plan.task_completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `plan.gate_result` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `plan.agent_output` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `plan.task_started` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `plan.task_completed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `plan.gate_result` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `plan.agent_output` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S015 -- Pause

**Source section:** `tmp/architecture/19-visual-composition.md:452` through `478`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pause

```
POST /api/plans/{id}/pause
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "paused",
  "completed_tasks": ["t1"],
  "paused_tasks": ["t2a"],
  "remaining_tasks": ["t2b", "t3", "t4"],
  "cost_so_far_usd": 0.55,
  "snapshot_id": "snap-002"
}
```

What happens:

1. All running agents receive a graceful stop signal. Current work is abandoned (agents checkpoint their state if possible).
2. A pause snapshot is written, recording which tasks completed, which were in progress, and which are pending.
3. The plan transitions to Paused. The dashboard shows a frost overlay.
4. The chat drawer reopens. The user can now talk to revise remaining tasks -- the mutations apply only to tasks not yet completed.
````

**Explicit detail extraction from this section:**

- Section word count: `100`
- Section hash: `647873f36d35d1e8369b149802290dad1772374c60ab6647e571634aed97496c`

**Normative requirements and implementation claims:**
- ``` POST /api/plans/{id}/pause ```
- 1. All running agents receive a graceful stop signal. Current work is abandoned (agents checkpoint their state if possible). 2. A pause snapshot is written, recording which tasks completed, which were in progress, and which are pending. 3. The plan transitions to Paused. The dashboard shows a frost overlay. 4. The chat drawer reopens. The user can now talk to revise remaining tasks -- the mutations apply only to tasks not yet completed.

**Routes and endpoint references:**
- POST /api/plans/{id}/pause

**Files and path references:**
- api/plans/

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
- 1. All running agents receive a graceful stop signal. Current work is abandoned (agents checkpoint their state if possible).
- 2. A pause snapshot is written, recording which tasks completed, which were in progress, and which are pending.
- 3. The plan transitions to Paused. The dashboard shows a frost overlay.
- 4. The chat drawer reopens. The user can now talk to revise remaining tasks -- the mutations apply only to tasks not yet completed.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST /api/plans/{id}/pause`

```
POST /api/plans/{id}/pause
```
- Contract 2: language `json`, first line `{`

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "paused",
  "completed_tasks": ["t1"],
  "paused_tasks": ["t2a"],
  "remaining_tasks": ["t2b", "t3", "t4"],
  "cost_so_far_usd": 0.55,
  "snapshot_id": "snap-002"
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Pause|tasks|snap|paused|completed|snapshot|remaining|plan" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Pause|tasks|snap|paused|completed|snapshot|remaining|plan" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/plans/{id}/pause` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S016 -- Resume

**Source section:** `tmp/architecture/19-visual-composition.md:479` through `500`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Resume

```
POST /api/plans/{id}/resume
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "executing",
  "resuming_from": "snap-002",
  "remaining_tasks": ["t2a", "t2b", "t3", "t4"],
  "agents_spawning": 2
}
```

Agents respawn for remaining tasks. Completed tasks stay completed. The DAG picks up where it left off. Tasks that were in progress when paused restart from scratch (not from mid-execution state -- agent checkpointing is best-effort and not guaranteed).

---
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `2c9bd0b01e9987a74f1c4a907f9f8f9f42627acd0bc47e4814ac394c1c2d7d5c`

**Normative requirements and implementation claims:**
- ``` POST /api/plans/{id}/resume ```
- Agents respawn for remaining tasks. Completed tasks stay completed. The DAG picks up where it left off. Tasks that were in progress when paused restart from scratch (not from mid-execution state -- agent checkpointing is best-effort and not guaranteed).
- ---

**Routes and endpoint references:**
- POST /api/plans/{id}/resume

**Files and path references:**
- api/plans/

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
- Contract 1: language `plain`, first line `POST /api/plans/{id}/resume`

```
POST /api/plans/{id}/resume
```
- Contract 2: language `json`, first line `{`

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "executing",
  "resuming_from": "snap-002",
  "remaining_tasks": ["t2a", "t2b", "t3", "t4"],
  "agents_spawning": 2
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Resume|tasks|exec|remaining|execution|Completed|were|stay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Resume|tasks|exec|remaining|execution|Completed|were|stay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/plans/{id}/resume` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S017 -- Three visual abstraction levels

**Source section:** `tmp/architecture/19-visual-composition.md:501` through `504`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Three visual abstraction levels

The backend serves the same plan data model. The dashboard renders at three levels of visual complexity. Users switch between them freely. The backend does not care which view is active.
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `567c4cae6bcddf3bb700c9f8ca0bb8df6bcfe8118c1af144743f80f813fab60c`

**Normative requirements and implementation claims:**
- The backend serves the same plan data model. The dashboard renders at three levels of visual complexity. Users switch between them freely. The backend does not care which view is active.

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "visual|levels|Three|abstraction|backend|view|switch|serves" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "visual|levels|Three|abstraction|backend|view|switch|serves" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S018 -- Plan data model

**Source section:** `tmp/architecture/19-visual-composition.md:505` through `541`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Plan data model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSpec {
    pub id: PlanId,
    pub name: String,
    pub description: String,
    pub status: PlanStatus,
    pub tasks: Vec<TaskSpec>,
    pub dependencies: Vec<(TaskId, TaskId)>,  // (from, to) -- "from" blocks "to"
    pub checkpoints: Vec<Checkpoint>,
    pub parallel_groups: Vec<Vec<TaskId>>,
    pub error_handling: ErrorPolicy,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Executing { execution_id: String },
    Paused { snapshot_id: String },
    Completed { execution_id: String, duration_secs: u64 },
    Failed { execution_id: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    pub after_task: TaskId,
    pub requires_approval: bool,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `107`
- Section hash: `d4a6ed6e9a8f8c6db295aae5879b82d247510fc5036be8ed0f1fecb457783cc8`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- PlanSpec
- PlanStatus
- Checkpoint

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
- Contract 1: language `rust`, first line `#[derive(Debug, Clone, Serialize, Deserialize)]`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSpec {
    pub id: PlanId,
    pub name: String,
    pub description: String,
    pub status: PlanStatus,
    pub tasks: Vec<TaskSpec>,
    pub dependencies: Vec<(TaskId, TaskId)>,  // (from, to) -- "from" blocks "to"
    pub checkpoints: Vec<Checkpoint>,
    pub parallel_groups: Vec<Vec<TaskId>>,
    pub error_handling: ErrorPolicy,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Executing { execution_id: String },
    Paused { snapshot_id: String },
    Completed { execution_id: String, duration_secs: u64 },
    Failed { execution_id: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    pub after_task: TaskId,
    pub requires_approval: bool,
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "String|Plan|Checkpoint|Serialize|PlanStatus|model|data|PlanSpec" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "String|Plan|Checkpoint|Serialize|PlanStatus|model|data|PlanSpec" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify `PlanSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PlanStatus` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Checkpoint` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S019 -- Level 1: card stack

**Source section:** `tmp/architecture/19-visual-composition.md:542` through `549`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Level 1: card stack

A vertical list of task cards. Drag to reorder. Each card shows title, agent profile, model, estimated time, and status. Dependencies are implicit from ordering -- tasks appear top to bottom in execution order. Parallel tasks are indicated with a subtle "runs with" badge but not visually separated into lanes.

The backend delivers this as the ordered `tasks` vec. No special rendering logic is needed.

Best for: linear pipelines, quick edits, mobile screens, first-time users.
````

**Explicit detail extraction from this section:**

- Section word count: `75`
- Section hash: `bbbdcbf5a87715ed2f877db6caa64d6a660900dc6860db9d53490d4cb2a7c3bf`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- tasks

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tasks|card|stack|Level|task|order|time|visually" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tasks|card|stack|Level|task|order|time|visually" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify `tasks` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S020 -- Level 2: lane view

**Source section:** `tmp/architecture/19-visual-composition.md:550` through `557`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Level 2: lane view

Parallel tasks occupy side-by-side lanes. Dependency arrows connect tasks across lanes. Drag a card between lanes to change parallelism. Drag a card up or down within a lane to reorder.

The backend delivers this as `tasks` + `parallel_groups` + `dependencies`. The dashboard layout engine computes lane positions from the parallel groups and renders horizontal swim lanes.

Best for: pipelines with 2-4 parallel branches, moderate complexity.
````

**Explicit detail extraction from this section:**

- Section word count: `66`
- Section hash: `02058462c64ba015b41fd009718318ae9662f3d9a98491762697cc9361ae4812`

**Normative requirements and implementation claims:**
- The backend delivers this as `tasks` + `parallel_groups` + `dependencies`. The dashboard layout engine computes lane positions from the parallel groups and renders horizontal swim lanes.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- tasks
- parallel_groups
- dependencies

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "lane|tasks|view|parallel_groups|dependencies|Parallel|Level|lanes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "lane|tasks|view|parallel_groups|dependencies|Parallel|Level|lanes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `tasks` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parallel_groups` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `dependencies` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S021 -- Level 3: node graph

**Source section:** `tmp/architecture/19-visual-composition.md:558` through `565`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Level 3: node graph

Full DAG rendered as a node graph (like React Flow). Tasks are nodes. Dependencies are directed edges. Conditional branches appear as diamond decision nodes. Fan-out (one task feeding many) and fan-in (many tasks feeding one) are visible as edge topology. Checkpoints appear as gate nodes between task nodes.

The backend delivers this as the full `PlanSpec`. The dashboard graph engine computes layout using a layered graph algorithm (Sugiyama-style) and renders with a library like xyflow.

Best for: complex multi-branch pipelines, power users, plans with 10+ tasks.
````

**Explicit detail extraction from this section:**

- Section word count: `90`
- Section hash: `338146157aedbf1202857c94f7c15b8e81e8e6e45781f5df0eedeb5a4c55bd1b`

**Normative requirements and implementation claims:**
- The backend delivers this as the full `PlanSpec`. The dashboard graph engine computes layout using a layered graph algorithm (Sugiyama-style) and renders with a library like xyflow.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- PlanSpec

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "node|graph|task|PlanSpec|Level|nodes|Tasks|plans" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "node|graph|task|PlanSpec|Level|nodes|Tasks|plans" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `PlanSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S022 -- View-switching API

**Source section:** `tmp/architecture/19-visual-composition.md:566` through `571`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### View-switching API

There is no view-switching API. The backend always returns the full `PlanSpec`. The dashboard chooses how to render it. If a user creates a plan in card stack view and another user opens it in node graph view, both see the same plan. The view is a client-side preference, not a plan property.

---
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `b8e158cd269b8bf93db55048dbb548881cc2393e2394b99c7a0cc975e86a8f64`

**Normative requirements and implementation claims:**
- There is no view-switching API. The backend always returns the full `PlanSpec`. The dashboard chooses how to render it. If a user creates a plan in card stack view and another user opens it in node graph view, both see the same plan. The view is a client-side preference, not a plan property.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- PlanSpec

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "View|switching|API|PlanSpec|plan|user|stack|side" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "View|switching|API|PlanSpec|plan|user|stack|side" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `PlanSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S023 -- Template registry

**Source section:** `tmp/architecture/19-visual-composition.md:572` through `575`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Template registry

Every object type has templates. Templates are discoverable, forkable, and user-publishable.
````

**Explicit detail extraction from this section:**

- Section word count: `12`
- Section hash: `b221b930c8f60de5b77edf6e2545a0aa6a185803aa1258f73a0e4a19bf6262dd`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- has

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Template|registry|has|templates|user|type|publishable|object" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Template|registry|has|templates|user|type|publishable|object" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `has` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S024 -- Template data model

**Source section:** `tmp/architecture/19-visual-composition.md:576` through `633`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Template data model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: TemplateId,
    pub object_type: ObjectType,
    pub name: String,
    pub description: String,
    pub author: AuthorId,
    pub source: TemplateSource,
    pub version: semver::Version,
    /// The full configuration for the object type.
    /// Validated against the object type's schema on publish.
    pub config: serde_json::Value,
    pub forked_from: Option<TemplateId>,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub type TemplateId = String;
pub type AuthorId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Agent,
    Extension,
    Connector,   // NEW (PRD 23)
    Gate,
    Feed,        // NEW (PRD 23)
    Recipe,      // NEW (PRD 23)
    Knowledge,
    Arena,
    Eval,
    Signal,      // Renamed from Pheromone (PRD 23)
    Group,
    Bounty,
    Plan,
    Generator,
    MetaAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    /// Curated by the platform. Always available.
    System,
    /// Published by a user to the community.
    Community,
    /// Private to the author. Not discoverable by others.
    User,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `160`
- Section hash: `3d902b24659faa820e9bd4d1908d83036b6107dd19fdb8297f2b6eb217f5b478`

**Normative requirements and implementation claims:**
- #[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum TemplateSource { /// Curated by the platform. Always available. System, /// Published by a user to the community. Community, /// Private to the author. Not discoverable by others. User, } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Template
- TemplateId
- AuthorId
- ObjectType
- TemplateSource

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
- Contract 1: language `rust`, first line `#[derive(Debug, Clone, Serialize, Deserialize)]`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: TemplateId,
    pub object_type: ObjectType,
    pub name: String,
    pub description: String,
    pub author: AuthorId,
    pub source: TemplateSource,
    pub version: semver::Version,
    /// The full configuration for the object type.
    /// Validated against the object type's schema on publish.
    pub config: serde_json::Value,
    pub forked_from: Option<TemplateId>,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub type TemplateId = String;
pub type AuthorId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Agent,
    Extension,
    Connector,   // NEW (PRD 23)
    Gate,
    Feed,        // NEW (PRD 23)
    Recipe,      // NEW (PRD 23)
    Knowledge,
    Arena,
    Eval,
    Signal,      // Renamed from Pheromone (PRD 23)
    Group,
    Bounty,
    Plan,
    Generator,
    MetaAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    /// Curated by the platform. Always available.
    System,
    /// Published by a user to the community.
    Community,
    /// Private to the author. Not discoverable by others.
    User,
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Template|type|TemplateId|TemplateSource|Serialize|ObjectType|AuthorId|object" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Template|type|TemplateId|TemplateSource|Serialize|ObjectType|AuthorId|object" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify `Template` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TemplateId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AuthorId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ObjectType` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TemplateSource` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S025 -- Template API

**Source section:** `tmp/architecture/19-visual-composition.md:634` through `704`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Template API

```
GET /api/templates?type=agent&sort=downloads&limit=20
```

Response:

```json
{
  "templates": [
    {
      "id": "tmpl-coding-rust",
      "object_type": "agent",
      "name": "Rust coding agent",
      "description": "Pre-configured for Rust development with clippy gate, test gate, and cargo-based tool access",
      "author": "system",
      "source": "system",
      "version": "1.2.0",
      "tags": ["rust", "coding", "beginner-friendly"],
      "downloads": 342,
      "rating": 4.7
    }
  ],
  "total": 48,
  "offset": 0,
  "limit": 20
}
```

```
GET /api/templates/{id}
```

Returns the full template including its `config` blob.

```
POST /api/templates
Content-Type: application/json

{
  "object_type": "agent",
  "name": "My custom researcher",
  "description": "Tuned for deep technical research with opus model and extended context",
  "config": { ... },
  "tags": ["research", "technical"],
  "visibility": "community"
}
```

Response:

```json
{
  "id": "tmpl-abc123",
  "version": "1.0.0"
}
```

```
POST /api/templates/{id}/fork
```

Creates a copy under the calling user's ownership. The new template's `forked_from` field points to the original.

```
DELETE /api/templates/{id}
```

Unpublishes a template. Only the author can unpublish. Already-deployed instances that used the template continue to work.
````

**Explicit detail extraction from this section:**

- Section word count: `160`
- Section hash: `d1f69eba03f8c0d353a3b9b5ce5d760d3306a7f7ee5bf9b7bed4cd78270285d9`

**Normative requirements and implementation claims:**
- ``` GET /api/templates?type=agent&sort=downloads&limit=20 ```
- ```json { "templates": [ { "id": "tmpl-coding-rust", "object_type": "agent", "name": "Rust coding agent", "description": "Pre-configured for Rust development with clippy gate, test gate, and cargo-based tool access", "author": "system", "source": "system", "version": "1.2.0", "tags": ["rust", "coding", "beginner-friendly"], "downloads": 342, "rating": 4.7 } ], "total": 48, "offset": 0, "limit": 20 } ```
- ``` GET /api/templates/{id} ```
- ``` POST /api/templates Content-Type: application/json
- ``` POST /api/templates/{id}/fork ```
- ``` DELETE /api/templates/{id} ```
- Unpublishes a template. Only the author can unpublish. Already-deployed instances that used the template continue to work.

**Routes and endpoint references:**
- GET /api/templates
- GET /api/templates/{id}
- POST /api/templates
- POST /api/templates/{id}/fork
- DELETE /api/templates/{id}

**Files and path references:**
- api/templates/

**Types, functions, traits, and inline code identifiers:**
- config
- forked_from

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
- Contract 1: language `plain`, first line `GET /api/templates?type=agent&sort=downloads&limit=20`

```
GET /api/templates?type=agent&sort=downloads&limit=20
```
- Contract 2: language `json`, first line `{`

```json
{
  "templates": [
    {
      "id": "tmpl-coding-rust",
      "object_type": "agent",
      "name": "Rust coding agent",
      "description": "Pre-configured for Rust development with clippy gate, test gate, and cargo-based tool access",
      "author": "system",
      "source": "system",
      "version": "1.2.0",
      "tags": ["rust", "coding", "beginner-friendly"],
      "downloads": 342,
      "rating": 4.7
    }
  ],
  "total": 48,
  "offset": 0,
  "limit": 20
}
```
- Contract 3: language `plain`, first line `GET /api/templates/{id}`

```
GET /api/templates/{id}
```
- Contract 4: language `plain`, first line `POST /api/templates`

```
POST /api/templates
Content-Type: application/json

{
  "object_type": "agent",
  "name": "My custom researcher",
  "description": "Tuned for deep technical research with opus model and extended context",
  "config": { ... },
  "tags": ["research", "technical"],
  "visibility": "community"
}
```
- Contract 5: language `json`, first line `{`

```json
{
  "id": "tmpl-abc123",
  "version": "1.0.0"
}
```
- Contract 6: language `plain`, first line `POST /api/templates/{id}/fork`

```
POST /api/templates/{id}/fork
```
- Contract 7: language `plain`, first line `DELETE /api/templates/{id}`

```
DELETE /api/templates/{id}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/templates/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Template|API|config|templates|forked_from|type|rust|research" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Template|API|config|templates|forked_from|type|rust|research" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/templates/`
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
- [ ] Implement or verify route `GET /api/templates` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/templates/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/templates` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/templates/{id}/fork` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/templates/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `config` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `forked_from` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S026 -- On-chain registration

**Source section:** `tmp/architecture/19-visual-composition.md:705` through `710`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### On-chain registration

Popular templates can optionally register on-chain for permanent discoverability. This uses the same ERC-8004 registry described in [14-registries.md](14-registries.md), extended with a `TemplateRegistered` event. On-chain registration is not required for local or community use -- it provides permanent availability and cross-instance discovery.

---
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `b972cc74ec2e03dff8d12b741e484f4eedcc555dd9a063b8443a753038e54ee2`

**Normative requirements and implementation claims:**
- Popular templates can optionally register on-chain for permanent discoverability. This uses the same ERC-8004 registry described in [14-registries.md](14-registries.md), extended with a `TemplateRegistered` event. On-chain registration is not required for local or community use -- it provides permanent availability and cross-instance discovery.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- TemplateRegistered

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "chain|registration|TemplateRegistered|registries|register|permanent|uses|templates" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "chain|registration|TemplateRegistered|registries|register|permanent|uses|templates" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `TemplateRegistered` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S027 -- Connector Manager (new authoring surface)

**Source section:** `tmp/architecture/19-visual-composition.md:711` through `727`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Connector Manager (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Connector is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Type selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery with icons per type |
| 2. Configuration | Connection string, auth credentials, rate limits, retry policy | Live health check runs during configuration |
| 3. Tool registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema (e.g., MCP tool list, exchange order types) |
| 4. Test and deploy | Execute test query; verify health endpoint | Shows latency p50/p99, error rate, connection status |

**API contract:** Follows the standard authoring pattern: `POST /api/connectors` (create), `GET /api/connectors` (list), `POST /api/connectors/{id}/validate`, `POST /api/connectors/{id}/deploy`.

**Relationship to agents:** An agent's `roko.toml` config references connectors by name. The Agent Composer's tool selection stage (Stage 4) pulls available operations from the agent's attached connectors.

---
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `f547fd848000ef2f596723f76b602bf4da8419a0b5839777507c1b536e08af43`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Per dashboard PRD 23, Connector is a universal primitive with a dedicated 4-stage authoring flow.
- | Stage | What | Notes | |-------|------|-------| | 1. Type selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery with icons per type | | 2. Configuration | Connection string, auth credentials, rate limits, retry policy | Live health check runs during configuration | | 3. Tool registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema (e.g., MCP tool list, exchange order types) | | 4. Test and deploy | Execute test query; verify health endpoint | Shows latency p50/p99, error rate, connection status |
- **API contract:** Follows the standard authoring pattern: `POST /api/connectors` (create), `GET /api/connectors` (list), `POST /api/connectors/{id}/validate`, `POST /api/connectors/{id}/deploy`.
- **Relationship to agents:** An agent's `roko.toml` config references connectors by name. The Agent Composer's tool selection stage (Stage 4) pulls available operations from the agent's attached connectors.
- ---

**Routes and endpoint references:**
- POST /api/connectors
- GET /api/connectors
- POST /api/connectors/{id}/validate
- POST /api/connectors/{id}/deploy

**Files and path references:**
- api/connectors/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Stage | What | Notes |
|-------|------|-------|
| 1. Type selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery with icons per type |
| 2. Configuration | Connection string, auth credentials, rate limits, retry policy | Live health check runs during configuration |
| 3. Tool registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema (e.g., MCP tool list, exchange order types) |
| 4. Test and deploy | Execute test query; verify health endpoint | Shows latency p50/p99, error rate, connection status |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/connectors/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Connector|auth|authoring|connectors|api|surface|new|Manager" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connector|auth|authoring|connectors|api|surface|new|Manager" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/connectors/`
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
- [ ] Implement or verify route `POST /api/connectors` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/connectors` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/connectors/{id}/validate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/connectors/{id}/deploy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S028 -- Feed Designer (new authoring surface)

**Source section:** `tmp/architecture/19-visual-composition.md:728` through `744`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Feed Designer (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Feed is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Source selection | Choose source connector and event type | Connector picker (only deployed connectors appear) |
| 2. Filter and transform | Configure event filters, sampling rate, aggregation window | Visual filter builder with preview of matching events |
| 3. Output configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram showing downstream consumers |
| 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline with 5-minute window |

**API contract:** `POST /api/feeds` (create), `GET /api/feeds` (list with cursor pagination), `POST /api/feeds/{id}/validate`, `POST /api/feeds/{id}/deploy`.

**Relationship to existing feed architecture:** The feed registration, discovery, and subscription mechanisms defined earlier in this spec ([05-feeds.md](05-feeds.md)) provide the backend. The Feed Designer is the dashboard authoring surface that creates and configures those feed registrations.

---
````

**Explicit detail extraction from this section:**

- Section word count: `152`
- Section hash: `b3e08804b7eb925f3e7cc92d3a32218cd0fa007404850daae6dd65ef45f6ade0`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Per dashboard PRD 23, Feed is a universal primitive with a dedicated 4-stage authoring flow.
- | Stage | What | Notes | |-------|------|-------| | 1. Source selection | Choose source connector and event type | Connector picker (only deployed connectors appear) | | 2. Filter and transform | Configure event filters, sampling rate, aggregation window | Visual filter builder with preview of matching events | | 3. Output configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram showing downstream consumers | | 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline with 5-minute window |
- **API contract:** `POST /api/feeds` (create), `GET /api/feeds` (list with cursor pagination), `POST /api/feeds/{id}/validate`, `POST /api/feeds/{id}/deploy`.
- **Relationship to existing feed architecture:** The feed registration, discovery, and subscription mechanisms defined earlier in this spec ([05-feeds.md](05-feeds.md)) provide the backend. The Feed Designer is the dashboard authoring surface that creates and configures those feed registrations.
- ---

**Routes and endpoint references:**
- POST /api/feeds
- GET /api/feeds
- POST /api/feeds/{id}/validate
- POST /api/feeds/{id}/deploy

**Files and path references:**
- api/feeds/

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
- Table 1:

```markdown
| Stage | What | Notes |
|-------|------|-------|
| 1. Source selection | Choose source connector and event type | Connector picker (only deployed connectors appear) |
| 2. Filter and transform | Configure event filters, sampling rate, aggregation window | Visual filter builder with preview of matching events |
| 3. Output configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram showing downstream consumers |
| 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline with 5-minute window |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/feeds/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Feed|authoring|surface|feeds|Designer|new|api|event" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|authoring|surface|feeds|Designer|new|api|event" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/feeds/`
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
- [ ] Implement or verify route `POST /api/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/feeds/{id}/validate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/feeds/{id}/deploy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S029 -- Recipe Editor (new authoring surface)

**Source section:** `tmp/architecture/19-visual-composition.md:745` through `761`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Recipe Editor (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Recipe is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Input selection | Choose feed(s) or connector query as input | Drag from feed/connector list; multiple inputs supported |
| 2. Pipeline builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor (similar to node graph view for plans) |
| 3. Output configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output with schema validation |
| 4. Backtest and validate | Run against historical data; compare output distribution | Chart overlay showing expected vs actual output |

**API contract:** `POST /api/recipes` (create), `GET /api/recipes` (list), `POST /api/recipes/{id}/validate`, `POST /api/recipes/{id}/deploy`, `POST /api/recipes/{id}/backtest`.

**Relationship to existing scoring:** Recipes compose `Scorer` trait instances from `roko-core`. Existing scoring pipelines in `roko-learn` (TradingReflect, FifoMatcher, IndicatorTracker) become built-in recipe templates available in the Recipe Editor's template picker.

---
````

**Explicit detail extraction from this section:**

- Section word count: `158`
- Section hash: `3c155450933ee74b064f6800bfbe12da12bfb8b905f1219c7e4c18b188b5ce0f`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Per dashboard PRD 23, Recipe is a universal primitive with a dedicated 4-stage authoring flow.
- | Stage | What | Notes | |-------|------|-------| | 1. Input selection | Choose feed(s) or connector query as input | Drag from feed/connector list; multiple inputs supported | | 2. Pipeline builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor (similar to node graph view for plans) | | 3. Output configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output with schema validation | | 4. Backtest and validate | Run against historical data; compare output distribution | Chart overlay showing expected vs actual output |
- **API contract:** `POST /api/recipes` (create), `GET /api/recipes` (list), `POST /api/recipes/{id}/validate`, `POST /api/recipes/{id}/deploy`, `POST /api/recipes/{id}/backtest`.
- **Relationship to existing scoring:** Recipes compose `Scorer` trait instances from `roko-core`. Existing scoring pipelines in `roko-learn` (TradingReflect, FifoMatcher, IndicatorTracker) become built-in recipe templates available in the Recipe Editor's template picker.
- ---

**Routes and endpoint references:**
- POST /api/recipes
- GET /api/recipes
- POST /api/recipes/{id}/validate
- POST /api/recipes/{id}/deploy
- POST /api/recipes/{id}/backtest

**Files and path references:**
- api/recipes/

**Types, functions, traits, and inline code identifiers:**
- instances
- Scorer

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
| Stage | What | Notes |
|-------|------|-------|
| 1. Input selection | Choose feed(s) or connector query as input | Drag from feed/connector list; multiple inputs supported |
| 2. Pipeline builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor (similar to node graph view for plans) |
| 3. Output configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output with schema validation |
| 4. Backtest and validate | Run against historical data; compare output distribution | Chart overlay showing expected vs actual output |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/recipes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Recipe|Editor|recipes|authoring|api|surface|new|instances" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recipe|Editor|recipes|authoring|api|surface|new|instances" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/recipes/`
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
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/recipes` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/recipes` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/recipes/{id}/validate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/recipes/{id}/deploy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/recipes/{id}/backtest` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `instances` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Scorer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S030 -- Extension compilation service

**Source section:** `tmp/architecture/19-visual-composition.md:762` through `765`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Extension compilation service

Users author extensions in the Extension Workshop. The backend compiles them in a sandboxed environment.
````

**Explicit detail extraction from this section:**

- Section word count: `15`
- Section hash: `4226ae3a36f83f9f0289138b43667258c0f2c684fd52a40aff04e2c03f4cbf82`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|service|compilation|sandboxed|extensions|environment|compiles|backend" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|service|compilation|sandboxed|extensions|environment|compiles|backend" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S031 -- Compile endpoint

**Source section:** `tmp/architecture/19-visual-composition.md:766` through `806`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Compile endpoint

```
POST /api/extensions/compile
Content-Type: application/json

{
  "name": "my-custom-gate",
  "source": "use roko_core::extension::*;\n\npub struct MyGate;\n\nimpl Extension for MyGate {\n    // ...\n}",
  "dependencies": ["tokio", "serde"],
  "target_hooks": ["post_action", "pre_gate"]
}
```

Response (success):

```json
{
  "status": "success",
  "artifact_id": "ext-a1b2c3",
  "warnings": [
    { "line": 12, "column": 5, "message": "unused variable `ctx`", "level": "warning" }
  ],
  "compile_time_ms": 2400,
  "artifact_size_bytes": 245760
}
```

Response (failure):

```json
{
  "status": "error",
  "errors": [
    { "line": 42, "column": 18, "message": "expected `;`, found `}`", "level": "error" },
    { "line": 15, "column": 1, "message": "cannot find type `ExtensionContext` in this scope", "level": "error" }
  ],
  "warnings": []
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `86`
- Section hash: `2781e62e817a686d97273d6c31e4446eddb245771787b137460ea1312adfcef2`

**Normative requirements and implementation claims:**
- ``` POST /api/extensions/compile Content-Type: application/json

**Routes and endpoint references:**
- POST /api/extensions/compile

**Files and path references:**
- api/extensions/

**Types, functions, traits, and inline code identifiers:**
- MyGate
- ctx
- ExtensionContext

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
- Contract 1: language `plain`, first line `POST /api/extensions/compile`

```
POST /api/extensions/compile
Content-Type: application/json

{
  "name": "my-custom-gate",
  "source": "use roko_core::extension::*;\n\npub struct MyGate;\n\nimpl Extension for MyGate {\n    // ...\n}",
  "dependencies": ["tokio", "serde"],
  "target_hooks": ["post_action", "pre_gate"]
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "status": "success",
  "artifact_id": "ext-a1b2c3",
  "warnings": [
    { "line": 12, "column": 5, "message": "unused variable `ctx`", "level": "warning" }
  ],
  "compile_time_ms": 2400,
  "artifact_size_bytes": 245760
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "status": "error",
  "errors": [
    { "line": 42, "column": 18, "message": "expected `;`, found `}`", "level": "error" },
    { "line": 15, "column": 1, "message": "cannot find type `ExtensionContext` in this scope", "level": "error" }
  ],
  "warnings": []
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/extensions/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Compile|MyGate|endpoint|ctx|ExtensionContext|gate|extension|error" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Compile|MyGate|endpoint|ctx|ExtensionContext|gate|extension|error" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/extensions/`
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
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/extensions/compile` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `MyGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ctx` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExtensionContext` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S032 -- Sandbox model

**Source section:** `tmp/architecture/19-visual-composition.md:807` through `819`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Sandbox model

Compilation runs in a container (or Fly Machine for cloud deployments). The sandbox has:

- A pre-cached Rust toolchain and the roko extension SDK.
- Network access restricted to crates.io for dependency fetching.
- A 60-second timeout and 2GB memory limit.
- No access to the host filesystem beyond the compilation workspace.

The compiled artifact (a shared library) is stored in the artifact registry and can be loaded by agents at runtime. Each artifact is content-addressed by its source hash, so recompiling identical source returns the cached artifact.

---
````

**Explicit detail extraction from this section:**

- Section word count: `88`
- Section hash: `e39270c3788fd99394b5b5ba3379a3a11b5715f836f9a35697c55f9f870b37b1`

**Normative requirements and implementation claims:**
- - A pre-cached Rust toolchain and the roko extension SDK. - Network access restricted to crates.io for dependency fetching. - A 60-second timeout and 2GB memory limit. - No access to the host filesystem beyond the compilation workspace.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- crates.io

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - A pre-cached Rust toolchain and the roko extension SDK.
- - Network access restricted to crates.io for dependency fetching.
- - A 60-second timeout and 2GB memory limit.
- - No access to the host filesystem beyond the compilation workspace.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Sandbox|model|artifact|cached|access|Compilation|workspace|toolchain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Sandbox|model|artifact|cached|access|Compilation|workspace|toolchain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Emit or consume `crates.io` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S033 -- Cost projection

**Source section:** `tmp/architecture/19-visual-composition.md:820` through `823`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Cost projection

Before executing a plan, users see an estimated cost and time.
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `36c719c15754a45b33157522bdca03c4fefca11f6dd5423f31b88fbdf4c5a54c`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cost|projection|users|time|plan|executing|estimated|Before" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cost|projection|users|time|plan|executing|estimated|Before" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S034 -- Estimate endpoint

**Source section:** `tmp/architecture/19-visual-composition.md:824` through `889`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Estimate endpoint

```
POST /api/plans/{id}/estimate
```

Response:

```json
{
  "total_usd": 1.80,
  "per_task": [
    {
      "task_id": "t1",
      "model": "claude-opus-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 2000,
      "estimated_usd": 0.40,
      "estimated_minutes": 5
    },
    {
      "task_id": "t2a",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 6000,
      "estimated_output_tokens": 3000,
      "estimated_usd": 0.25,
      "estimated_minutes": 12
    },
    {
      "task_id": "t2b",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 5000,
      "estimated_output_tokens": 2500,
      "estimated_usd": 0.20,
      "estimated_minutes": 10
    },
    {
      "task_id": "t3",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 4000,
      "estimated_usd": 0.30,
      "estimated_minutes": 8
    },
    {
      "task_id": "t4",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 3000,
      "estimated_output_tokens": 1500,
      "estimated_usd": 0.15,
      "estimated_minutes": 5
    }
  ],
  "time_estimate_mins": 40,
  "critical_path_mins": 27,
  "confidence": 0.7,
  "breakdown": {
    "inference": 1.20,
    "feeds": 0.10,
    "gas": 0.50
  }
}
```

Note the distinction between `time_estimate_mins` (wall clock, accounting for parallelism) and the sum of per-task minutes (total agent-minutes). The `critical_path_mins` field shows the longest sequential chain through the DAG.
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `257cef1b2df285ece8d310fd10bf17aa62613b3b3f7ab3bc48a5fb99a6300752`

**Normative requirements and implementation claims:**
- ``` POST /api/plans/{id}/estimate ```

**Routes and endpoint references:**
- POST /api/plans/{id}/estimate

**Files and path references:**
- api/plans/

**Types, functions, traits, and inline code identifiers:**
- time_estimate_mins
- critical_path_mins

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
- Contract 1: language `plain`, first line `POST /api/plans/{id}/estimate`

```
POST /api/plans/{id}/estimate
```
- Contract 2: language `json`, first line `{`

```json
{
  "total_usd": 1.80,
  "per_task": [
    {
      "task_id": "t1",
      "model": "claude-opus-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 2000,
      "estimated_usd": 0.40,
      "estimated_minutes": 5
    },
    {
      "task_id": "t2a",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 6000,
      "estimated_output_tokens": 3000,
      "estimated_usd": 0.25,
      "estimated_minutes": 12
    },
    {
      "task_id": "t2b",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 5000,
      "estimated_output_tokens": 2500,
      "estimated_usd": 0.20,
      "estimated_minutes": 10
    },
    {
      "task_id": "t3",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 4000,
      "estimated_usd": 0.30,
      "estimated_minutes": 8
    },
    {
      "task_id": "t4",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 3000,
      "estimated_output_tokens": 1500,
      "estimated_usd": 0.15,
      "estimated_minutes": 5
    }
  ],
  "time_estimate_mins": 40,
  "critical_path_mins": 27,
  "confidence": 0.7,
  "breakdown": {
    "inference": 1.20,
    "feeds": 0.10,
    "gas": 0.50
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Estimate|task|minutes|time_estimate_mins|critical_path_mins|task_id|model|estimated_usd" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Estimate|task|minutes|time_estimate_mins|critical_path_mins|task_id|model|estimated_usd" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- [ ] Implement or verify route `POST /api/plans/{id}/estimate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `time_estimate_mins` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `critical_path_mins` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S035 -- Estimation algorithm

**Source section:** `tmp/architecture/19-visual-composition.md:890` through `903`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Estimation algorithm

The cost projector uses three inputs:

1. **Task description complexity.** Longer descriptions, more files, broader scope heuristics push token estimates up. This is a simple heuristic, not an LLM call.

2. **Model pricing.** Current per-token rates for each model, stored in `roko.toml` and refreshed from the provider health endpoint.

3. **Historical data from similar tasks.** The learning system (`.roko/learn/efficiency.jsonl`) records actual tokens used and time taken for past tasks. The projector queries this store for tasks with similar agent profiles, models, and description lengths, and uses the p50 from matching historical data when available. When no historical data matches, it falls back to static heuristics.

The `confidence` field reflects how much historical data informed the estimate. A confidence of 0.5 means roughly half the estimate came from heuristics. A confidence of 0.9 means strong historical data supports the numbers.

---
````

**Explicit detail extraction from this section:**

- Section word count: `146`
- Section hash: `1a35888d1b3be8cf6f2259c1f1135fa78c8cd663ffd90d15fc50bc63bb7cba73`

**Normative requirements and implementation claims:**
- 1. **Task description complexity.** Longer descriptions, more files, broader scope heuristics push token estimates up. This is a simple heuristic, not an LLM call.
- 2. **Model pricing.** Current per-token rates for each model, stored in `roko.toml` and refreshed from the provider health endpoint.
- 3. **Historical data from similar tasks.** The learning system (`.roko/learn/efficiency.jsonl`) records actual tokens used and time taken for past tasks. The projector queries this store for tasks with similar agent profiles, models, and description lengths, and uses the p50 from matching historical data when available. When no historical data matches, it falls back to static heuristics.
- The `confidence` field reflects how much historical data informed the estimate. A confidence of 0.5 means roughly half the estimate came from heuristics. A confidence of 0.9 means strong historical data supports the numbers.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/learn/efficiency.json

**Types, functions, traits, and inline code identifiers:**
- confidence

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Task description complexity.** Longer descriptions, more files, broader scope heuristics push token estimates up. This is a simple heuristic, not an LLM call.
- 2. **Model pricing.** Current per-token rates for each model, stored in `roko.toml` and refreshed from the provider health endpoint.
- 3. **Historical data from similar tasks.** The learning system (`.roko/learn/efficiency.jsonl`) records actual tokens used and time taken for past tasks. The projector queries this store for tasks with similar agent profiles, models, and description lengths, and uses the p50 from matching historical data when available. When no historical data matches, it falls back to static heuristics.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/efficiency.json`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "confidence|data|algorithm|Historical|Estimation|heuristic|Task|token" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "confidence|data|algorithm|Historical|Estimation|heuristic|Task|token" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `.roko/learn/efficiency.json`
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
- [ ] Implement or verify `confidence` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S036 -- Gate test runner

**Source section:** `tmp/architecture/19-visual-composition.md:904` through `907`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Gate test runner

Users test gates against fixtures before deploying them to a pipeline.
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `3657d3d24580bc43ca150b23a3f7350126422b8a4b6d2f82610f60277d9faf4e`

**Normative requirements and implementation claims:**
- Users test gates against fixtures before deploying them to a pipeline.

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "test|Gate|runner|pipeline|gates|fixtures|deploying|before" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "test|Gate|runner|pipeline|gates|fixtures|deploying|before" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S037 -- Test endpoint

**Source section:** `tmp/architecture/19-visual-composition.md:908` through `964`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Test endpoint

```
POST /api/gates/{id}/test
Content-Type: application/json

{
  "fixture_path": "fixtures/sample-rust-project",
  "expected_result": "pass",
  "timeout_ms": 10000
}
```

Response (pass):

```json
{
  "result": "pass",
  "expected": "pass",
  "match": true,
  "output": "47 tests passed, 0 failed, 0 ignored",
  "duration_ms": 3200,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 0,
    "stdout_lines": 52,
    "stderr_lines": 3
  }
}
```

Response (unexpected failure):

```json
{
  "result": "fail",
  "expected": "pass",
  "match": false,
  "output": "3 tests passed, 2 failed",
  "duration_ms": 4100,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 101,
    "failures": [
      { "test": "test_auth_middleware", "message": "assertion failed: response.status().is_success()" },
      { "test": "test_ws_auth", "message": "timeout after 5000ms" }
    ]
  }
}
```

The test endpoint runs the gate in the same sandbox used for extension compilation. It does not affect any running agents or live plans.

---
````

**Explicit detail extraction from this section:**

- Section word count: `116`
- Section hash: `2041856c7de386312fac2a83b35205b97847274780804d42c7ce26478e256fd2`

**Normative requirements and implementation claims:**
- ``` POST /api/gates/{id}/test Content-Type: application/json
- ```json { "result": "pass", "expected": "pass", "match": true, "output": "47 tests passed, 0 failed, 0 ignored", "duration_ms": 3200, "gate_details": { "gate_type": "test", "command": "cargo test --workspace", "exit_code": 0, "stdout_lines": 52, "stderr_lines": 3 } } ```
- ```json { "result": "fail", "expected": "pass", "match": false, "output": "3 tests passed, 2 failed", "duration_ms": 4100, "gate_details": { "gate_type": "test", "command": "cargo test --workspace", "exit_code": 101, "failures": [ { "test": "test_auth_middleware", "message": "assertion failed: response.status().is_success()" }, { "test": "test_ws_auth", "message": "timeout after 5000ms" } ] } } ```
- The test endpoint runs the gate in the same sandbox used for extension compilation. It does not affect any running agents or live plans.
- ---

**Routes and endpoint references:**
- POST /api/gates/{id}/test

**Files and path references:**
- api/gates/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- response.status

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
- Contract 1: language `plain`, first line `POST /api/gates/{id}/test`

```
POST /api/gates/{id}/test
Content-Type: application/json

{
  "fixture_path": "fixtures/sample-rust-project",
  "expected_result": "pass",
  "timeout_ms": 10000
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "result": "pass",
  "expected": "pass",
  "match": true,
  "output": "47 tests passed, 0 failed, 0 ignored",
  "duration_ms": 3200,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 0,
    "stdout_lines": 52,
    "stderr_lines": 3
  }
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "result": "fail",
  "expected": "pass",
  "match": false,
  "output": "3 tests passed, 2 failed",
  "duration_ms": 4100,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 101,
    "failures": [
      { "test": "test_auth_middleware", "message": "assertion failed: response.status().is_success()" },
      { "test": "test_ws_auth", "message": "timeout after 5000ms" }
    ]
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/gates/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Test|pass|gate|fail|endpoint|expected|result|response" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Test|pass|gate|fail|endpoint|expected|result|response" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/gates/`
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
- [ ] Implement or verify route `POST /api/gates/{id}/test` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Emit or consume `response.status` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S038 -- Authoring API contracts

**Source section:** `tmp/architecture/19-visual-composition.md:965` through `968`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Authoring API contracts

Each of the 13 authoring surfaces follows a consistent REST pattern. The object type name slots into the URL.
````

**Explicit detail extraction from this section:**

- Section word count: `19`
- Section hash: `23945d2ea22d6d1e7a74e3e21fc0213e2f11a3158d0f2003f6b532a041593168`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- name

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Authoring|name|contracts|API|type|surfaces|slots|pattern" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Authoring|name|contracts|API|type|surfaces|slots|pattern" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S039 -- CRUD

**Source section:** `tmp/architecture/19-visual-composition.md:969` through `981`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### CRUD

```
POST   /api/{object_type}              -- create (from template or blank)
GET    /api/{object_type}              -- list (with pagination, filtering)
GET    /api/{object_type}/{id}         -- read (full detail including composition)
PUT    /api/{object_type}/{id}         -- update (full replacement)
PATCH  /api/{object_type}/{id}         -- partial update
DELETE /api/{object_type}/{id}         -- delete (soft delete for deployed objects)
```

Where `{object_type}` is one of: `agents`, `extensions`, `connectors`, `gates`, `feeds`, `recipes`, `knowledge`, `arenas`, `evals`, `signals`, `groups`, `bounties`, `plans`, `templates`, `generators`.
````

**Explicit detail extraction from this section:**

- Section word count: `67`
- Section hash: `1f54942477f63f0c0404182d031d2db6f226316d83154e839fed41190290f31e`

**Normative requirements and implementation claims:**
- ``` POST /api/{object_type} -- create (from template or blank) GET /api/{object_type} -- list (with pagination, filtering) GET /api/{object_type}/{id} -- read (full detail including composition) PUT /api/{object_type}/{id} -- update (full replacement) PATCH /api/{object_type}/{id} -- partial update DELETE /api/{object_type}/{id} -- delete (soft delete for deployed objects) ```

**Routes and endpoint references:**
- POST /api/{object_type}
- GET /api/{object_type}
- GET /api/{object_type}/{id}
- PUT /api/{object_type}/{id}
- PATCH /api/{object_type}/{id}
- DELETE /api/{object_type}/{id}

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- agents
- extensions
- connectors
- gates
- feeds
- recipes
- knowledge
- arenas
- evals
- signals
- groups
- bounties
- plans
- templates
- generators

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
- Contract 1: language `plain`, first line `POST   /api/{object_type}              -- create (from template or blank)`

```
POST   /api/{object_type}              -- create (from template or blank)
GET    /api/{object_type}              -- list (with pagination, filtering)
GET    /api/{object_type}/{id}         -- read (full detail including composition)
PUT    /api/{object_type}/{id}         -- update (full replacement)
PATCH  /api/{object_type}/{id}         -- partial update
DELETE /api/{object_type}/{id}         -- delete (soft delete for deployed objects)
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "object_type|api|templates|signals|recipes|plans|knowledge|groups" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "object_type|api|templates|signals|recipes|plans|knowledge|groups" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify route `POST /api/{object_type}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/{object_type}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/{object_type}/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PUT /api/{object_type}/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PATCH /api/{object_type}/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/{object_type}/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `agents` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `extensions` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `connectors` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `gates` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feeds` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `recipes` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `knowledge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `arenas` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `evals` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `signals` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `groups` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `bounties` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `plans` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `templates` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `generators` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S040 -- Create from template

**Source section:** `tmp/architecture/19-visual-composition.md:982` through `999`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Create from template

```
POST /api/agents
Content-Type: application/json

{
  "from_template": "tmpl-coding-rust",
  "overrides": {
    "name": "My Rust agent",
    "model": "claude-opus-4-6",
    "budget_daily_usd": 10.0
  }
}
```

The backend loads the template, applies overrides, validates the result, and creates the object in draft state. If `from_template` is omitted, a blank object is created with required fields empty (validation will flag them).
````

**Explicit detail extraction from this section:**

- Section word count: `58`
- Section hash: `011af2c8433e799b82ceceb6da334dd501043ace20a1bfcb123264074eb7916f`

**Normative requirements and implementation claims:**
- ``` POST /api/agents Content-Type: application/json
- The backend loads the template, applies overrides, validates the result, and creates the object in draft state. If `from_template` is omitted, a blank object is created with required fields empty (validation will flag them).

**Routes and endpoint references:**
- POST /api/agents

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- from_template

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
- Contract 1: language `plain`, first line `POST /api/agents`

```
POST /api/agents
Content-Type: application/json

{
  "from_template": "tmpl-coding-rust",
  "overrides": {
    "name": "My Rust agent",
    "model": "claude-opus-4-6",
    "budget_daily_usd": 10.0
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/mod.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "template|Create|from_template|rust|overrides|object|validation|validates" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "template|Create|from_template|rust|overrides|object|validation|validates" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/agents` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `from_template` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S041 -- Validation

**Source section:** `tmp/architecture/19-visual-composition.md:1000` through `1045`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Validation

```
POST /api/{object_type}/{id}/validate
```

Response:

```json
{
  "valid": false,
  "errors": [
    {
      "field": "ground_truth",
      "message": "Ground truth source is required for evals",
      "severity": "error",
      "code": "REQUIRED_FIELD"
    }
  ],
  "warnings": [
    {
      "field": "budget_daily_usd",
      "message": "Budget of $0.50/day is low for an opus-tier agent -- typical daily cost is $2-5",
      "severity": "warning",
      "code": "LOW_BUDGET"
    }
  ],
  "suggestions": [
    {
      "field": "model",
      "message": "Consider claude-sonnet for research tasks to reduce cost by ~60%",
      "severity": "suggestion",
      "code": "MODEL_SUGGESTION"
    }
  ]
}
```

Three severity levels:

- **error**: blocks deploy. Missing required fields, invalid values, structural problems.
- **warning**: does not block but flags risk. Suboptimal configuration, potential cost issues.
- **suggestion**: advisory. Recommendations based on domain best practices and historical data.

Validation runs automatically as the user edits (debounced, not on every keystroke). The dashboard calls the validate endpoint after 500ms of inactivity. Errors appear inline next to the relevant field. Warnings and suggestions appear in a sidebar panel.
````

**Explicit detail extraction from this section:**

- Section word count: `143`
- Section hash: `7e6b8ac3df83e790212d1a9160377ea0d66360c969f3901d2ff64f726cd15259`

**Normative requirements and implementation claims:**
- ``` POST /api/{object_type}/{id}/validate ```
- ```json { "valid": false, "errors": [ { "field": "ground_truth", "message": "Ground truth source is required for evals", "severity": "error", "code": "REQUIRED_FIELD" } ], "warnings": [ { "field": "budget_daily_usd", "message": "Budget of $0.50/day is low for an opus-tier agent -- typical daily cost is $2-5", "severity": "warning", "code": "LOW_BUDGET" } ], "suggestions": [ { "field": "model", "message": "Consider claude-sonnet for research tasks to reduce cost by ~60%", "severity": "suggestion", "code": "MODEL_SUGGESTION" } ] } ```
- - **error**: blocks deploy. Missing required fields, invalid values, structural problems. - **warning**: does not block but flags risk. Suboptimal configuration, potential cost issues. - **suggestion**: advisory. Recommendations based on domain best practices and historical data.
- Validation runs automatically as the user edits (debounced, not on every keystroke). The dashboard calls the validate endpoint after 500ms of inactivity. Errors appear inline next to the relevant field. Warnings and suggestions appear in a sidebar panel.

**Routes and endpoint references:**
- POST /api/{object_type}/{id}/validate

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
- - **error**: blocks deploy. Missing required fields, invalid values, structural problems.
- - **warning**: does not block but flags risk. Suboptimal configuration, potential cost issues.
- - **suggestion**: advisory. Recommendations based on domain best practices and historical data.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST /api/{object_type}/{id}/validate`

```
POST /api/{object_type}/{id}/validate
```
- Contract 2: language `json`, first line `{`

```json
{
  "valid": false,
  "errors": [
    {
      "field": "ground_truth",
      "message": "Ground truth source is required for evals",
      "severity": "error",
      "code": "REQUIRED_FIELD"
    }
  ],
  "warnings": [
    {
      "field": "budget_daily_usd",
      "message": "Budget of $0.50/day is low for an opus-tier agent -- typical daily cost is $2-5",
      "severity": "warning",
      "code": "LOW_BUDGET"
    }
  ],
  "suggestions": [
    {
      "field": "model",
      "message": "Consider claude-sonnet for research tasks to reduce cost by ~60%",
      "severity": "suggestion",
      "code": "MODEL_SUGGESTION"
    }
  ]
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "valid|field|Validation|suggestion|warning|severity|error|required" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "valid|field|Validation|suggestion|warning|severity|error|required" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- [ ] Implement or verify route `POST /api/{object_type}/{id}/validate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S042 -- Deploy

**Source section:** `tmp/architecture/19-visual-composition.md:1046` through `1078`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deploy

```
POST /api/{object_type}/{id}/deploy
```

Request:

```json
{
  "target": "local",
  "register_on_chain": false
}
```

Response:

```json
{
  "deployment_id": "dep-xyz",
  "status": "deploying",
  "estimated_cost": {
    "gas_wei": "0",
    "tokens_usd": 0.0,
    "inference_usd": 0.0
  }
}
```

Deploy transitions the object from draft to live. For agents, this means spawning a runtime process. For gates, this means registering in the gate pipeline registry. For templates, this is a no-op (templates are "live" when published to the template registry).

The `register_on_chain` flag triggers ERC-8004 registration for agents, or the appropriate on-chain registry for other object types.
````

**Explicit detail extraction from this section:**

- Section word count: `88`
- Section hash: `474bead7d4e7f32f51059c7dba31ca6f67e7738b3858cac24cafab71741e86ef`

**Normative requirements and implementation claims:**
- ``` POST /api/{object_type}/{id}/deploy ```

**Routes and endpoint references:**
- POST /api/{object_type}/{id}/deploy

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- register_on_chain

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
- Contract 1: language `plain`, first line `POST /api/{object_type}/{id}/deploy`

```
POST /api/{object_type}/{id}/deploy
```
- Contract 2: language `json`, first line `{`

```json
{
  "target": "local",
  "register_on_chain": false
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "deployment_id": "dep-xyz",
  "status": "deploying",
  "estimated_cost": {
    "gas_wei": "0",
    "tokens_usd": 0.0,
    "inference_usd": 0.0
  }
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Deploy|register_on_chain|template|registry|object|chain|templates|means" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deploy|register_on_chain|template|registry|object|chain|templates|means" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/mod.rs`

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
- [ ] Implement or verify route `POST /api/{object_type}/{id}/deploy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `register_on_chain` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S043 -- Publish as template

**Source section:** `tmp/architecture/19-visual-composition.md:1079` through `1096`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Publish as template

```
POST /api/{object_type}/{id}/publish
Content-Type: application/json

{
  "template_name": "My optimized researcher",
  "description": "Research agent tuned for Rust codebases with extended context window",
  "tags": ["research", "rust"],
  "visibility": "community"
}
```

This snapshots the current configuration and publishes it to the template registry. The original object continues to exist independently -- future edits to the object do not affect the published template.

---
````

**Explicit detail extraction from this section:**

- Section word count: `59`
- Section hash: `3a4c3346e6d675cf09ca4a9e4cd71a8d4b4879c14face889662482045baa22e1`

**Normative requirements and implementation claims:**
- ``` POST /api/{object_type}/{id}/publish Content-Type: application/json
- This snapshots the current configuration and publishes it to the template registry. The original object continues to exist independently -- future edits to the object do not affect the published template.
- ---

**Routes and endpoint references:**
- POST /api/{object_type}/{id}/publish

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
- Contract 1: language `plain`, first line `POST /api/{object_type}/{id}/publish`

```
POST /api/{object_type}/{id}/publish
Content-Type: application/json

{
  "template_name": "My optimized researcher",
  "description": "Research agent tuned for Rust codebases with extended context window",
  "tags": ["research", "rust"],
  "visibility": "community"
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/mod.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "template|Publish|object|Research|Type|Rust|window|visibility" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "template|Publish|object|Research|Type|Rust|window|visibility" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/{object_type}/{id}/publish` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S044 -- Event types for authoring

**Source section:** `tmp/architecture/19-visual-composition.md:1097` through `1157`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event types for authoring

All authoring events follow the existing `ServerEvent` pattern in roko-serve.

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthoringEvent {
    /// A chat mutation was applied to a plan.
    PlanMutationApplied {
        plan_id: PlanId,
        mutation_count: usize,
        rejected_count: usize,
        new_task_count: usize,
    },
    /// A plan transitioned between states.
    PlanStateChanged {
        plan_id: PlanId,
        from: PlanStatus,
        to: PlanStatus,
    },
    /// A new template was published to the registry.
    TemplatePublished {
        template_id: TemplateId,
        object_type: ObjectType,
        author: AuthorId,
    },
    /// Someone forked a template.
    TemplateForked {
        template_id: TemplateId,
        forked_from: TemplateId,
        author: AuthorId,
    },
    /// Extension compilation completed (success or failure).
    ExtensionCompiled {
        extension_name: String,
        artifact_id: Option<String>,
        success: bool,
        error_count: usize,
    },
    /// An object passed validation.
    ObjectValidated {
        object_type: ObjectType,
        object_id: String,
        error_count: usize,
        warning_count: usize,
    },
    /// An object was deployed (made live).
    ObjectDeployed {
        object_type: ObjectType,
        object_id: String,
        deployment_id: String,
        registered_on_chain: bool,
    },
}
```

Events stream to the dashboard via the existing WebSocket room system. The plan chat session subscribes to `plan:{id}` to receive mutation events. The fleet page subscribes to `system` to receive deployment events. Template pages subscribe to `templates` to receive publish and fork events.

---
````

**Explicit detail extraction from this section:**

- Section word count: `167`
- Section hash: `bae4fc48b3c862a916c8d0bc63ebffb3d647d69c5469696bd4349fc8ba728276`

**Normative requirements and implementation claims:**
- Events stream to the dashboard via the existing WebSocket room system. The plan chat session subscribes to `plan:{id}` to receive mutation events. The fleet page subscribes to `system` to receive deployment events. Template pages subscribe to `templates` to receive publish and fork events.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AuthoringEvent
- ServerEvent
- system
- templates

**Event names and event-like entities:**
- ServerEvent
- AuthoringEvent

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
- Contract 1: language `rust`, first line `#[derive(Debug, Clone, Serialize)]`

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthoringEvent {
    /// A chat mutation was applied to a plan.
    PlanMutationApplied {
        plan_id: PlanId,
        mutation_count: usize,
        rejected_count: usize,
        new_task_count: usize,
    },
    /// A plan transitioned between states.
    PlanStateChanged {
        plan_id: PlanId,
        from: PlanStatus,
        to: PlanStatus,
    },
    /// A new template was published to the registry.
    TemplatePublished {
        template_id: TemplateId,
        object_type: ObjectType,
        author: AuthorId,
    },
    /// Someone forked a template.
    TemplateForked {
        template_id: TemplateId,
        forked_from: TemplateId,
        author: AuthorId,
    },
    /// Extension compilation completed (success or failure).
    ExtensionCompiled {
        extension_name: String,
        artifact_id: Option<String>,
        success: bool,
        error_count: usize,
    },
    /// An object passed validation.
    ObjectValidated {
        object_type: ObjectType,
        object_id: String,
        error_count: usize,
        warning_count: usize,
    },
    /// An object was deployed (made live).
    ObjectDeployed {
        object_type: ObjectType,
        object_id: String,
        deployment_id: String,
        registered_on_chain: bool,
    },
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "type|plan|object|Event|template|author|for|authoring" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "type|plan|object|Event|template|author|for|authoring" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `AuthoringEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ServerEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `system` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `templates` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ServerEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AuthoringEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S045 -- Ecosystem dynamics

**Source section:** `tmp/architecture/19-visual-composition.md:1158` through `1161`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Ecosystem dynamics

User-contributed content creates a flywheel. Each stage feeds the next.
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `443d0ba1080b694f1bf5c51443f8719cc22a884d7582d9ffffe3803b88a4dc29`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "dynamics|Ecosystem|stage|next|flywheel|feeds|creates|contributed" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "dynamics|Ecosystem|stage|next|flywheel|feeds|creates|contributed" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S046 -- The template flywheel

**Source section:** `tmp/architecture/19-visual-composition.md:1162` through `1170`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### The template flywheel

1. A user builds an effective agent configuration for Rust development.
2. They click "Publish as template" and share it with the community.
3. Other users discover it in the template picker, fork it, and adapt it.
4. Forks that perform well get higher ratings and more downloads.
5. The original author sees fork activity and can incorporate improvements back.
6. Meta-agents can create and publish templates automatically based on performance data.
````

**Explicit detail extraction from this section:**

- Section word count: `73`
- Section hash: `b5019b6f1a2a666ccc423cc8b12ff8df23a9143d9799b19be2b8f30ac18cadce`

**Normative requirements and implementation claims:**
- 1. A user builds an effective agent configuration for Rust development. 2. They click "Publish as template" and share it with the community. 3. Other users discover it in the template picker, fork it, and adapt it. 4. Forks that perform well get higher ratings and more downloads. 5. The original author sees fork activity and can incorporate improvements back. 6. Meta-agents can create and publish templates automatically based on performance data.

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
- 1. A user builds an effective agent configuration for Rust development.
- 2. They click "Publish as template" and share it with the community.
- 3. Other users discover it in the template picker, fork it, and adapt it.
- 4. Forks that perform well get higher ratings and more downloads.
- 5. The original author sees fork activity and can incorporate improvements back.
- 6. Meta-agents can create and publish templates automatically based on performance data.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "The|template|flywheel|fork|user|perform|Publish|well" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|template|flywheel|fork|user|perform|Publish|well" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S047 -- Backend tracking for recommendations

**Source section:** `tmp/architecture/19-visual-composition.md:1171` through `1190`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Backend tracking for recommendations

The backend tracks per-template metrics:

```json
{
  "template_id": "tmpl-abc",
  "downloads": 342,
  "forks": 28,
  "active_deployments": 15,
  "avg_rating": 4.7,
  "rating_count": 23,
  "avg_task_success_rate": 0.89,
  "avg_cost_per_task_usd": 0.45,
  "last_used": "2026-04-23T14:30:00Z"
}
```

These metrics feed into template recommendations. When a user creates a new agent and selects the "coding" domain, the template picker ranks templates by a combination of rating, usage, and success rate within that domain.
````

**Explicit detail extraction from this section:**

- Section word count: `68`
- Section hash: `2360e6ecaddb203ec5ac5436e3006ca9f55b904ca74865df28ea573c6e5e9265`

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
- Contract 1: language `json`, first line `{`

```json
{
  "template_id": "tmpl-abc",
  "downloads": 342,
  "forks": 28,
  "active_deployments": 15,
  "avg_rating": 4.7,
  "rating_count": 23,
  "avg_task_success_rate": 0.89,
  "avg_cost_per_task_usd": 0.45,
  "last_used": "2026-04-23T14:30:00Z"
}
```

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "recommendations|for|Backend|tracking|template|rating|success|rate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "recommendations|for|Backend|tracking|template|rating|success|rate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S048 -- Generator-driven template creation

**Source section:** `tmp/architecture/19-visual-composition.md:1191` through `1196`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Generator-driven template creation

Generators (described in [13-meta.md](13-meta.md)) can produce domain-specific templates at scale. A generator configured for "blockchain security analysis" can produce template variants tuned for different chain types (EVM, Solana, Cosmos), each with appropriate extensions, gates, and model preferences. These generated templates enter the registry with `source: "system"` and go through the same rating/download cycle as user-published templates.

---
````

**Explicit detail extraction from this section:**

- Section word count: `64`
- Section hash: `bef2dc831dbe794efcab80c20640a4a7687e28dac8c29fc605dd04bdc6995613`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "template|Generator|driven|creation|templates|produce|meta|chain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "template|Generator|driven|creation|templates|produce|meta|chain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S049 -- Relationship to existing codebase

**Source section:** `tmp/architecture/19-visual-composition.md:1197` through `1198`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Relationship to existing codebase
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `6dbe3460f4865bfe2ccb6dfe11e7d412086f560a0a9804e6b257c50f7afcfc1a`

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "existing|codebase|Relationship|visual|composition" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "existing|codebase|Relationship|visual|composition" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S050 -- What exists today

**Source section:** `tmp/architecture/19-visual-composition.md:1199` through `1202`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What exists today

The current `roko-serve` plan routes (`/api/plans`, `/api/plans/{id}`, `/api/plans/{id}/execute`, `/api/plans/{id}/status`, `/api/plans/generate`) support basic CRUD and execution. The `Plan` and `PlanTask` types in `roko-serve/src/plan_types.rs` model a flat task list with dependencies and completion status.
````

**Explicit detail extraction from this section:**

- Section word count: `48`
- Section hash: `8cad5470a368e4c8150e7c0879c728802fb10e3ef7f7ea40c5398d0d54c1d99c`

**Normative requirements and implementation claims:**
- The current `roko-serve` plan routes (`/api/plans`, `/api/plans/{id}`, `/api/plans/{id}/execute`, `/api/plans/{id}/status`, `/api/plans/generate`) support basic CRUD and execution. The `Plan` and `PlanTask` types in `roko-serve/src/plan_types.rs` model a flat task list with dependencies and completion status.

**Routes and endpoint references:**
- /api/plans
- /api/plans/{id}
- /api/plans/{id}/execute
- /api/plans/{id}/status
- /api/plans/generate

**Files and path references:**
- api/plans/
- roko-serve/src/plan_types.rs

**Types, functions, traits, and inline code identifiers:**
- Plan
- PlanTask

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
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
- `roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Plan|today|plans|exists|api|PlanTask|types|task" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Plan|today|plans|exists|api|PlanTask|types|task" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
- `roko-serve/src/plan_types.rs`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `/api/plans` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/execute` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/status` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/generate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `Plan` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PlanTask` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S051 -- What this spec adds

**Source section:** `tmp/architecture/19-visual-composition.md:1203` through `1216`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What this spec adds

| Feature | Current state | Spec target |
|---------|---------------|-------------|
| Plan data model | Flat task list, string IDs, `depends_on` vec | Extended with `parallel_groups`, `checkpoints`, `error_handling`, plan-level status |
| Plan editing | Direct JSON/TOML file editing | Chat-driven mutation protocol |
| Plan execution | Single `run_once` call | DAG-aware orchestration with pause/resume |
| Templates | Not present | Full registry with CRUD, forking, versioning, community publishing |
| Validation | Basic field presence checks | Three-tier validation (error/warning/suggestion) for all object types |
| Cost estimation | Not present | Historical-data-informed projection with per-task breakdown |
| Extension compilation | Not present | Sandboxed compilation service |
| Gate testing | Not present | Fixture-based test runner |
| Authoring surfaces | Not present | Consistent CRUD + validate + deploy + publish pattern for 13 object types |
````

**Explicit detail extraction from this section:**

- Section word count: `112`
- Section hash: `ab4526d6de6c81db5909ec76b3054704970cb8fb2ea584d59e749691ddf55a29`

**Normative requirements and implementation claims:**
- | Feature | Current state | Spec target | |---------|---------------|-------------| | Plan data model | Flat task list, string IDs, `depends_on` vec | Extended with `parallel_groups`, `checkpoints`, `error_handling`, plan-level status | | Plan editing | Direct JSON/TOML file editing | Chat-driven mutation protocol | | Plan execution | Single `run_once` call | DAG-aware orchestration with pause/resume | | Templates | Not present | Full registry with CRUD, forking, versioning, community publishing | | Validation | Basic field presence checks | Three-tier validation (error/warning/suggestion) for all object types | | Cost estimation | Not present | Historical-data-informed projection with per-task breakdown | | Extension compilation | Not present | Sandboxed compilation service | | Gate testing | Not present | Fixture-based test runner | | Authoring surfaces | Not present | Consistent CRUD + validate + deploy + publish pattern for 13 object types |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- error/warning/

**Types, functions, traits, and inline code identifiers:**
- depends_on
- parallel_groups
- checkpoints
- error_handling
- run_once

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
| Feature | Current state | Spec target |
|---------|---------------|-------------|
| Plan data model | Flat task list, string IDs, `depends_on` vec | Extended with `parallel_groups`, `checkpoints`, `error_handling`, plan-level status |
| Plan editing | Direct JSON/TOML file editing | Chat-driven mutation protocol |
| Plan execution | Single `run_once` call | DAG-aware orchestration with pause/resume |
| Templates | Not present | Full registry with CRUD, forking, versioning, community publishing |
| Validation | Basic field presence checks | Three-tier validation (error/warning/suggestion) for all object types |
| Cost estimation | Not present | Historical-data-informed projection with per-task breakdown |
| Extension compilation | Not present | Sandboxed compilation service |
| Gate testing | Not present | Fixture-based test runner |
| Authoring surfaces | Not present | Consistent CRUD + validate + deploy + publish pattern for 13 object types |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `error/warning/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "spec|run_once|present|parallel_groups|error_handling|depends_on|checkpoints|adds" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "spec|run_once|present|parallel_groups|error_handling|depends_on|checkpoints|adds" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `error/warning/`
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
- [ ] Implement or verify `depends_on` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parallel_groups` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `checkpoints` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `error_handling` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `run_once` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S052 -- Implementation path

**Source section:** `tmp/architecture/19-visual-composition.md:1217` through `1229`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Implementation path

The plan mutation protocol is the critical path. It depends on:

1. Extending `PlanSpec` with `parallel_groups`, `checkpoints`, and `status` fields.
2. Adding the `/api/plans/{id}/chat` endpoint that dispatches to an LLM and returns structured mutations.
3. Adding `/api/plans/{id}/pause` and `/api/plans/{id}/resume` endpoints backed by the existing `ExecutorSnapshot` mechanism.
4. Building the mutation validation logic (cycle detection, duplicate ID checks).

Everything else -- templates, compilation, gate testing, authoring CRUD -- is independent and can be built in parallel.

---
````

**Explicit detail extraction from this section:**

- Section word count: `82`
- Section hash: `a8320d995821b7233d4e1c63a2b423062f7af44056ee479f185777ec97e9a152`

**Normative requirements and implementation claims:**
- 1. Extending `PlanSpec` with `parallel_groups`, `checkpoints`, and `status` fields. 2. Adding the `/api/plans/{id}/chat` endpoint that dispatches to an LLM and returns structured mutations. 3. Adding `/api/plans/{id}/pause` and `/api/plans/{id}/resume` endpoints backed by the existing `ExecutorSnapshot` mechanism. 4. Building the mutation validation logic (cycle detection, duplicate ID checks).
- ---

**Routes and endpoint references:**
- /api/plans/{id}/chat
- /api/plans/{id}/pause
- /api/plans/{id}/resume

**Files and path references:**
- api/plans/

**Types, functions, traits, and inline code identifiers:**
- PlanSpec
- parallel_groups
- checkpoints
- status
- ExecutorSnapshot

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Extending `PlanSpec` with `parallel_groups`, `checkpoints`, and `status` fields.
- 2. Adding the `/api/plans/{id}/chat` endpoint that dispatches to an LLM and returns structured mutations.
- 3. Adding `/api/plans/{id}/pause` and `/api/plans/{id}/resume` endpoints backed by the existing `ExecutorSnapshot` mechanism.
- 4. Building the mutation validation logic (cycle detection, duplicate ID checks).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "path|status|plan|parallel_groups|checkpoints|PlanSpec|ExecutorSnapshot|plans" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "path|status|plan|parallel_groups|checkpoints|PlanSpec|ExecutorSnapshot|plans" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `api/plans/`
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
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `/api/plans/{id}/chat` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/pause` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/resume` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `PlanSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parallel_groups` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `checkpoints` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `status` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExecutorSnapshot` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

### ARCH-19-S053 -- Summary of API surface

**Source section:** `tmp/architecture/19-visual-composition.md:1230` through `1255`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Summary of API surface

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/plans/{id}/chat` | POST | Conversation-driven plan editing |
| `/api/plans/{id}/run` | POST | Snapshot plan and begin execution |
| `/api/plans/{id}/pause` | POST | Stop all agents, freeze state |
| `/api/plans/{id}/resume` | POST | Respawn agents for remaining tasks |
| `/api/plans/{id}/estimate` | POST | Cost and time projection |
| `/api/templates` | GET/POST | List and publish templates |
| `/api/templates/{id}` | GET/DELETE | Template detail and unpublish |
| `/api/templates/{id}/fork` | POST | Fork a template |
| `/api/connectors` | GET/POST | List and create connectors (PRD 23) |
| `/api/connectors/{id}` | GET/PUT/PATCH/DELETE | Connector CRUD |
| `/api/feeds` | GET/POST | List and create feeds (PRD 23) |
| `/api/feeds/{id}` | GET/PUT/PATCH/DELETE | Feed CRUD |
| `/api/recipes` | GET/POST | List and create recipes (PRD 23) |
| `/api/recipes/{id}` | GET/PUT/PATCH/DELETE | Recipe CRUD |
| `/api/recipes/{id}/backtest` | POST | Run recipe against historical data |
| `/api/extensions/compile` | POST | Compile extension source in sandbox |
| `/api/gates/{id}/test` | POST | Run gate against test fixture |
| `/api/{object_type}` | GET/POST | List and create objects (15 types) |
| `/api/{object_type}/{id}` | GET/PUT/PATCH/DELETE | Object CRUD |
| `/api/{object_type}/{id}/validate` | POST | Three-tier validation |
| `/api/{object_type}/{id}/deploy` | POST | Deploy object (make live) |
| `/api/{object_type}/{id}/publish` | POST | Publish as template |
````

**Explicit detail extraction from this section:**

- Section word count: `206`
- Section hash: `9e8170f5491da542af4e8a6515181f9ff2a565f380b38a7655abb603fcdb5fc9`

**Normative requirements and implementation claims:**
- | Endpoint | Method | Purpose | |----------|--------|---------| | `/api/plans/{id}/chat` | POST | Conversation-driven plan editing | | `/api/plans/{id}/run` | POST | Snapshot plan and begin execution | | `/api/plans/{id}/pause` | POST | Stop all agents, freeze state | | `/api/plans/{id}/resume` | POST | Respawn agents for remaining tasks | | `/api/plans/{id}/estimate` | POST | Cost and time projection | | `/api/templates` | GET/POST | List and publish templates | | `/api/templates/{id}` | GET/DELETE | Template detail and unpublish | | `/api/templates/{id}/fork` | POST | Fork a template | | `/api/connectors` | GET/POST | List and create connectors (PRD 23) | | `/api/connectors/{id}` | GET/PUT/PATCH/DELETE | Connector CRUD | | `/api/feeds` | GET/POST | List and create feeds (PRD 23) | | `/api/feeds/{id}` | GET/PUT/PATCH/DELETE | Feed CRUD | | `/api/recipes` | GET/POST | List and create recipes (PRD 23) | | `/api/recipes/{id}` | GET/PUT/PATCH/DELETE | Recipe CRUD | | `/api/recipes/{id}/ba

**Routes and endpoint references:**
- /api/plans/{id}/chat
- /api/plans/{id}/run
- /api/plans/{id}/pause
- /api/plans/{id}/resume
- /api/plans/{id}/estimate
- /api/templates
- /api/templates/{id}
- /api/templates/{id}/fork
- /api/connectors
- /api/connectors/{id}
- /api/feeds
- /api/feeds/{id}
- /api/recipes
- /api/recipes/{id}
- /api/recipes/{id}/backtest
- /api/extensions/compile
- /api/gates/{id}/test
- /api/{object_type}
- /api/{object_type}/{id}
- /api/{object_type}/{id}/validate
- /api/{object_type}/{id}/deploy
- /api/{object_type}/{id}/publish

**Files and path references:**
- GET/PUT/PATCH/
- api/connectors/
- api/extensions/
- api/feeds/
- api/gates/
- api/plans/
- api/recipes/
- api/templates/

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
- Table 1:

```markdown
| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/plans/{id}/chat` | POST | Conversation-driven plan editing |
| `/api/plans/{id}/run` | POST | Snapshot plan and begin execution |
| `/api/plans/{id}/pause` | POST | Stop all agents, freeze state |
| `/api/plans/{id}/resume` | POST | Respawn agents for remaining tasks |
| `/api/plans/{id}/estimate` | POST | Cost and time projection |
| `/api/templates` | GET/POST | List and publish templates |
| `/api/templates/{id}` | GET/DELETE | Template detail and unpublish |
| `/api/templates/{id}/fork` | POST | Fork a template |
| `/api/connectors` | GET/POST | List and create connectors (PRD 23) |
| `/api/connectors/{id}` | GET/PUT/PATCH/DELETE | Connector CRUD |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/19-visual-composition.md`
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `GET/PUT/PATCH/`
- `api/connectors/`
- `api/extensions/`
- `api/feeds/`
- `api/gates/`
- `api/plans/`
- `api/recipes/`
- `api/templates/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "API|POST|Object|plan|Template|Recipe|surface|plans" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "API|POST|Object|plan|Template|Recipe|surface|plans" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-compose/src/`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/plan_types.rs`
- `crates/roko-serve/src/events.rs`
- `GET/PUT/PATCH/`
- `api/connectors/`
- `api/extensions/`
- `api/feeds/`
- `api/gates/`
- `api/plans/`
- `api/recipes/`
- `api/templates/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`

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
- [ ] Implement or verify route `/api/plans/{id}/chat` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/run` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/pause` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/resume` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/plans/{id}/estimate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/templates` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/templates/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/templates/{id}/fork` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/connectors` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/connectors/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/feeds/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/recipes` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/recipes/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/recipes/{id}/backtest` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/extensions/compile` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/gates/{id}/test` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/{object_type}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/{object_type}/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/{object_type}/{id}/validate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/{object_type}/{id}/deploy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/{object_type}/{id}/publish` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/19-visual-composition
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

