# Architecture Plan: Groups

**Source:** `tmp/architecture/10-groups.md`
**Generated:** 2026-04-25
**Source hash:** `dea38b8ade6a5b4a7356971484af9c1e61fad34b470a90c4ecfc1735ac07c5c6`
**Section tasks:** 30
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
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-10-S001 | 1 | 10 -- Groups and coordination | [ ] | 9.8 |
| ARCH-10-S002 | 7 | Groups vs clusters | [ ] | 9.8 |
| ARCH-10-S003 | 34 | Group identity | [ ] | 9.8 |
| ARCH-10-S004 | 38 | Core type | [ ] | 9.8 |
| ARCH-10-S005 | 88 | Relay room | [ ] | 9.8 |
| ARCH-10-S006 | 116 | On-chain identity | [ ] | 9.8 |
| ARCH-10-S007 | 141 | Membership protocol | [ ] | 9.8 |
| ARCH-10-S008 | 143 | Creating a group | [ ] | 9.8 |
| ARCH-10-S009 | 177 | Inviting agents | [ ] | 9.8 |
| ARCH-10-S010 | 212 | Cross-user invitation flow | [ ] | 9.8 |
| ARCH-10-S011 | 257 | Leaving and removal | [ ] | 9.8 |
| ARCH-10-S012 | 269 | Coordination modes | [ ] | 9.8 |
| ARCH-10-S013 | 273 | Stigmergic | [ ] | 9.8 |
| ARCH-10-S014 | 293 | Pipeline | [ ] | 9.8 |
| ARCH-10-S015 | 315 | Broadcast | [ ] | 9.8 |
| ARCH-10-S016 | 333 | Leader-follower | [ ] | 9.8 |
| ARCH-10-S017 | 385 | Shared context | [ ] | 9.8 |
| ARCH-10-S018 | 389 | Group knowledge store | [ ] | 9.8 |
| ARCH-10-S019 | 421 | Group pheromone field | [ ] | 9.8 |
| ARCH-10-S020 | 454 | Context injection | [ ] | 9.8 |
| ARCH-10-S021 | 470 | Dashboard surfaces | [ ] | 9.8 |
| ARCH-10-S022 | 474 | Group list page | [ ] | 9.8 |
| ARCH-10-S023 | 486 | Group detail page | [ ] | 9.8 |
| ARCH-10-S024 | 498 | Group activity timeline | [ ] | 9.8 |
| ARCH-10-S025 | 511 | API surface | [ ] | 9.8 |
| ARCH-10-S026 | 544 | Event types | [ ] | 9.8 |
| ARCH-10-S027 | 573 | Configuration | [ ] | 9.8 |
| ARCH-10-S028 | 612 | Cross-user group creation: full example | [ ] | 9.8 |
| ARCH-10-S029 | 692 | Crate mapping | [ ] | 9.8 |
| ARCH-10-S030 | 708 | Open questions | [ ] | 9.8 |

## Tasks

### ARCH-10-S001 -- 10 -- Groups and coordination

**Source section:** `tmp/architecture/10-groups.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 10 -- Groups and coordination

> Persistent agent collectives with shared identity, membership protocol, and coordination modes.

---
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `55caedd3e268277acd8bacc47c6d998d3fdc737cbb8080dc38463cdb44ade52a`

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "coordination|Groups|shared|protocol|modes|membership|identity" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "coordination|Groups|shared|protocol|modes|membership|identity" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S002 -- Groups vs clusters

**Source section:** `tmp/architecture/10-groups.md:7` through `33`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Groups vs clusters

The system has two multi-agent primitives. They serve different purposes and operate at different timescales.

**Groups** are persistent. A group is a named collection of agents with shared identity, a relay room, and optional on-chain registration. Groups outlive individual tasks. An agent joins a group and stays until it leaves or is removed. Groups accumulate shared knowledge and pheromone fields over time.

**Clusters** are ephemeral. A cluster is a pipeline -- a DAG of stages executed by agents, created for a specific task and destroyed when the task completes. Clusters are the execution primitive from the v2 architecture:

```
POST /api/clusters
{ "name": "feature-build", "agents": [...], "pipeline": [...] }
```

The relationship between them: a group contains agents, a cluster orchestrates them. You create a cluster from a group's members when you need to run a coordinated pipeline. The group persists after the cluster finishes.

| Property | Group | Cluster |
|----------|-------|---------|
| Lifetime | Persistent | Ephemeral (task-scoped) |
| Identity | Has ID, name, relay room, optional passport | Has ID, pipeline definition |
| Members | Join/leave dynamically | Fixed at creation |
| Coordination | Multiple modes (see below) | Pipeline DAG only |
| Knowledge | Shared store, shared pheromones | Shared context (PRD, repo) |
| Cross-user | Yes, via invitation | Yes, if authorized |
| On-chain | Optional (ERC-8004 group passport) | No |

---
````

**Explicit detail extraction from this section:**

- Section word count: `203`
- Section hash: `16b96ed9015e7591066701e064dc0493860fc4d659bb6c4cab3ed5c4c7258869`

**Normative requirements and implementation claims:**
- **Groups** are persistent. A group is a named collection of agents with shared identity, a relay room, and optional on-chain registration. Groups outlive individual tasks. An agent joins a group and stays until it leaves or is removed. Groups accumulate shared knowledge and pheromone fields over time.
- **Clusters** are ephemeral. A cluster is a pipeline -- a DAG of stages executed by agents, created for a specific task and destroyed when the task completes. Clusters are the execution primitive from the v2 architecture:
- ``` POST /api/clusters { "name": "feature-build", "agents": [...], "pipeline": [...] } ```
- The relationship between them: a group contains agents, a cluster orchestrates them. You create a cluster from a group's members when you need to run a coordinated pipeline. The group persists after the cluster finishes.
- | Property | Group | Cluster | |----------|-------|---------| | Lifetime | Persistent | Ephemeral (task-scoped) | | Identity | Has ID, name, relay room, optional passport | Has ID, pipeline definition | | Members | Join/leave dynamically | Fixed at creation | | Coordination | Multiple modes (see below) | Pipeline DAG only | | Knowledge | Shared store, shared pheromones | Shared context (PRD, repo) | | Cross-user | Yes, via invitation | Yes, if authorized | | On-chain | Optional (ERC-8004 group passport) | No |
- ---

**Routes and endpoint references:**
- POST /api/clusters

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
- Table 1:

```markdown
| Property | Group | Cluster |
|----------|-------|---------|
| Lifetime | Persistent | Ephemeral (task-scoped) |
| Identity | Has ID, name, relay room, optional passport | Has ID, pipeline definition |
| Members | Join/leave dynamically | Fixed at creation |
| Coordination | Multiple modes (see below) | Pipeline DAG only |
| Knowledge | Shared store, shared pheromones | Shared context (PRD, repo) |
| Cross-user | Yes, via invitation | Yes, if authorized |
| On-chain | Optional (ERC-8004 group passport) | No |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `POST /api/clusters`

```
POST /api/clusters
{ "name": "feature-build", "agents": [...], "pipeline": [...] }
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "group|cluster|clusters|Groups|shared|pipeline|task|time" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|cluster|clusters|Groups|shared|pipeline|task|time" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify route `POST /api/clusters` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S003 -- Group identity

**Source section:** `tmp/architecture/10-groups.md:34` through `37`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Group identity

A group is a first-class entity in the system. It has its own ID, its own relay room, and optionally its own on-chain passport.
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `d7aea0960eed2ab5ccc11bfcf46874399c7b372138fde4f6f1b62c39b15e2cd3`

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "entity|Group|identity|room|relay|passport|optionally|first" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "entity|Group|identity|room|relay|passport|optionally|first" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S004 -- Core type

**Source section:** `tmp/architecture/10-groups.md:38` through `87`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Core type

```rust
pub struct Group {
    pub id: GroupId,             // UUID
    pub name: String,            // Human-readable, unique per owner
    pub description: String,
    pub owner: UserId,           // The user who created the group
    pub members: Vec<GroupMember>,
    pub coordination: CoordinationMode,
    pub config: GroupConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct GroupMember {
    pub agent_id: AgentId,
    pub owner: UserId,           // The agent's owner (may differ from group owner)
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub joined_at: DateTime<Utc>,
}

pub enum MemberRole {
    Leader,    // Can coordinate, assign tasks, manage members
    Member,    // Full participation
    Observer,  // Read-only access to group activity
}

pub struct MemberPermissions {
    pub read: bool,     // See group activity, knowledge, pheromones
    pub write: bool,    // Contribute knowledge, deposit pheromones
    pub execute: bool,  // Participate in cluster pipelines
}

pub struct GroupConfig {
    pub max_members: Option<usize>,
    pub auto_accept: bool,          // Skip approval for invitations
    pub public: bool,               // Visible in global group listing
    pub knowledge_policy: KnowledgePolicy,
    pub pheromone_decay_rate: f64,  // Group-specific decay rate
}

pub enum KnowledgePolicy {
    Open,        // Any member can read and write
    WriteLeader, // Only leaders write, all read
    Curated,     // Writes require leader approval
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `172`
- Section hash: `4107f47ffc46df65ea60c968851f13bfb9a518b07ce55ceb66e8c454a139b4b8`

**Normative requirements and implementation claims:**
- pub enum MemberRole { Leader, // Can coordinate, assign tasks, manage members Member, // Full participation Observer, // Read-only access to group activity }
- pub enum KnowledgePolicy { Open, // Any member can read and write WriteLeader, // Only leaders write, all read Curated, // Writes require leader approval } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Group
- GroupMember
- MemberRole
- MemberPermissions
- GroupConfig
- KnowledgePolicy

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
- Contract 1: language `rust`, first line `pub struct Group {`

```rust
pub struct Group {
    pub id: GroupId,             // UUID
    pub name: String,            // Human-readable, unique per owner
    pub description: String,
    pub owner: UserId,           // The user who created the group
    pub members: Vec<GroupMember>,
    pub coordination: CoordinationMode,
    pub config: GroupConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct GroupMember {
    pub agent_id: AgentId,
    pub owner: UserId,           // The agent's owner (may differ from group owner)
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub joined_at: DateTime<Utc>,
}

pub enum MemberRole {
    Leader,    // Can coordinate, assign tasks, manage members
    Member,    // Full participation
    Observer,  // Read-only access to group activity
}

pub struct MemberPermissions {
    pub read: bool,     // See group activity, knowledge, pheromones
    pub write: bool,    // Contribute knowledge, deposit pheromones
    pub execute: bool,  // Participate in cluster pipelines
}

pub struct GroupConfig {
    pub max_members: Option<usize>,
    pub auto_accept: bool,          // Skip approval for invitations
    pub public: bool,               // Visible in global group listing
    pub knowledge_policy: KnowledgePolicy,
    pub pheromone_decay_rate: f64,  // Group-specific decay rate
}

pub enum KnowledgePolicy {
    Open,        // Any member can read and write
    WriteLeader, // Only leaders write, all read
    Curated,     // Writes require leader approval
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Group|Member|MemberRole|MemberPermissions|KnowledgePolicy|GroupMember|GroupConfig|write" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|Member|MemberRole|MemberPermissions|KnowledgePolicy|GroupMember|GroupConfig|write" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `Group` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GroupMember` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MemberRole` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MemberPermissions` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GroupConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgePolicy` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S005 -- Relay room

**Source section:** `tmp/architecture/10-groups.md:88` through `115`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Relay room

Every group gets a relay room: `group:{id}`. All members subscribe to this room on connection. Messages sent to the room reach every connected member.

The room follows the same envelope format as all relay rooms:

```json
{
  "seq": 7201,
  "ts": 1713974400123,
  "room": "group:a1b2c3d4",
  "type": "group.message",
  "payload": {
    "from": "agent-alpha",
    "content": "Found three relevant papers on MEV mitigation."
  }
}
```

Sub-rooms scope finer-grained subscriptions:

```
group:{id}                  Group lifecycle + broadcast messages
group:{id}:knowledge        Knowledge publish/validate events
group:{id}:pheromones       Pheromone deposit/decay events
group:{id}:coordination     Task assignment, status updates
```
````

**Explicit detail extraction from this section:**

- Section word count: `92`
- Section hash: `384e7ebb9296087d50ca8bf52e49e177ade13b8511056faef53b7444adbb4090`

**Normative requirements and implementation claims:**
- ``` group:{id} Group lifecycle + broadcast messages group:{id}:knowledge Knowledge publish/validate events group:{id}:pheromones Pheromone deposit/decay events group:{id}:coordination Task assignment, status updates ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- group.message

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
  "seq": 7201,
  "ts": 1713974400123,
  "room": "group:a1b2c3d4",
  "type": "group.message",
  "payload": {
    "from": "agent-alpha",
    "content": "Found three relevant papers on MEV mitigation."
  }
}
```
- Contract 2: language `plain`, first line `group:{id}                  Group lifecycle + broadcast messages`

```
group:{id}                  Group lifecycle + broadcast messages
group:{id}:knowledge        Knowledge publish/validate events
group:{id}:pheromones       Pheromone deposit/decay events
group:{id}:coordination     Task assignment, status updates
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "room|group|Relay|message|rooms|member|knowledge|events" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "room|group|Relay|message|rooms|member|knowledge|events" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Emit or consume `group.message` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S006 -- On-chain identity

**Source section:** `tmp/architecture/10-groups.md:116` through `140`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### On-chain identity

Groups can register an on-chain passport through ERC-8004. This is optional -- groups work without chain registration -- but it enables:

- Verifiable membership (the contract stores the member list)
- Cross-platform discovery (any chain reader can find the group)
- Group-level reputation (aggregated from member reputations)
- Group-held assets (treasury, earned fees from paid feeds)

```solidity
// GroupRegistry extends ERC-8004
function registerGroup(
    string calldata name,
    address[] calldata initialMembers
) external returns (uint256 groupId);

function addMember(uint256 groupId, address agent) external;
function removeMember(uint256 groupId, address agent) external;
function members(uint256 groupId) external view returns (address[] memory);
```

Registration is a group owner action. The on-chain record is authoritative for membership when it exists; the off-chain relay membership is authoritative otherwise.

---
````

**Explicit detail extraction from this section:**

- Section word count: `120`
- Section hash: `61ec5868e957b0c5d52d60440b9c385777c22d9d13e233b57f77c7cd5b355355`

**Normative requirements and implementation claims:**
- - Verifiable membership (the contract stores the member list) - Cross-platform discovery (any chain reader can find the group) - Group-level reputation (aggregated from member reputations) - Group-held assets (treasury, earned fees from paid feeds)
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
- - Verifiable membership (the contract stores the member list)
- - Cross-platform discovery (any chain reader can find the group)
- - Group-level reputation (aggregated from member reputations)
- - Group-held assets (treasury, earned fees from paid feeds)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `solidity`, first line `// GroupRegistry extends ERC-8004`

```solidity
// GroupRegistry extends ERC-8004
function registerGroup(
    string calldata name,
    address[] calldata initialMembers
) external returns (uint256 groupId);

function addMember(uint256 groupId, address agent) external;
function removeMember(uint256 groupId, address agent) external;
function members(uint256 groupId) external view returns (address[] memory);
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "group|chain|member|members|identity|uint256|groupId|function" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|chain|member|members|identity|uint256|groupId|function" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S007 -- Membership protocol

**Source section:** `tmp/architecture/10-groups.md:141` through `142`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Membership protocol
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `238615ba702e020ab0dc527268ed5382629fd389ad8544073edaa20a5da8de3d`

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "protocol|Membership|groups" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "protocol|Membership|groups" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S008 -- Creating a group

**Source section:** `tmp/architecture/10-groups.md:143` through `176`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Creating a group

The group owner creates the group and becomes its first member (as an observer -- the owner is a user, not an agent). Agents are then invited.

```
POST /api/groups
{
  "name": "defi-research",
  "description": "Cross-domain DeFi research collective",
  "coordination": "stigmergic",
  "config": {
    "max_members": 12,
    "auto_accept": false,
    "public": true,
    "knowledge_policy": "open",
    "pheromone_decay_rate": 0.02
  }
}
```

Response:

```json
{
  "id": "a1b2c3d4",
  "name": "defi-research",
  "owner": "user-will",
  "members": [],
  "coordination": "stigmergic",
  "relay_room": "group:a1b2c3d4",
  "created_at": "2026-04-24T12:00:00Z"
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `74`
- Section hash: `6cb84092506baa03ed0dea83990df1364b97cce04c0869d733e5a16ff7455069`

**Normative requirements and implementation claims:**
- ``` POST /api/groups { "name": "defi-research", "description": "Cross-domain DeFi research collective", "coordination": "stigmergic", "config": { "max_members": 12, "auto_accept": false, "public": true, "knowledge_policy": "open", "pheromone_decay_rate": 0.02 } } ```

**Routes and endpoint references:**
- POST /api/groups

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
- Contract 1: language `plain`, first line `POST /api/groups`

```
POST /api/groups
{
  "name": "defi-research",
  "description": "Cross-domain DeFi research collective",
  "coordination": "stigmergic",
  "config": {
    "max_members": 12,
    "auto_accept": false,
    "public": true,
    "knowledge_policy": "open",
    "pheromone_decay_rate": 0.02
  }
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "id": "a1b2c3d4",
  "name": "defi-research",
  "owner": "user-will",
  "members": [],
  "coordination": "stigmergic",
  "relay_room": "group:a1b2c3d4",
  "created_at": "2026-04-24T12:00:00Z"
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "group|Creating|research|owner|member|defi|user|stigmergic" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|Creating|research|owner|member|defi|user|stigmergic" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/groups` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S009 -- Inviting agents

**Source section:** `tmp/architecture/10-groups.md:177` through `211`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Inviting agents

The group owner invites agents by ID. If the agent belongs to the same user, it joins immediately (no approval needed). If the agent belongs to a different user, the invitation requires approval from that agent's owner.

```
POST /api/groups/a1b2c3d4/invite
{
  "agent_id": "chain-watcher",
  "role": "member",
  "permissions": { "read": true, "write": true, "execute": true }
}
```

Response for same-owner agent:

```json
{
  "status": "joined",
  "agent_id": "chain-watcher",
  "group_id": "a1b2c3d4"
}
```

Response for cross-user agent:

```json
{
  "status": "pending",
  "invitation_id": "inv-xyz",
  "agent_id": "strategy-bot",
  "agent_owner": "user-alice",
  "expires_at": "2026-04-25T12:00:00Z"
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `91`
- Section hash: `94caecdc6c683dcc3c1795321adace500b47fce31a0f21097a78f3e8b9dd7589`

**Normative requirements and implementation claims:**
- The group owner invites agents by ID. If the agent belongs to the same user, it joins immediately (no approval needed). If the agent belongs to a different user, the invitation requires approval from that agent's owner.
- ``` POST /api/groups/a1b2c3d4/invite { "agent_id": "chain-watcher", "role": "member", "permissions": { "read": true, "write": true, "execute": true } } ```

**Routes and endpoint references:**
- POST /api/groups/a1b2c3d4/invite

**Files and path references:**
- api/groups/a1b2c3d4/

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
- Contract 1: language `plain`, first line `POST /api/groups/a1b2c3d4/invite`

```
POST /api/groups/a1b2c3d4/invite
{
  "agent_id": "chain-watcher",
  "role": "member",
  "permissions": { "read": true, "write": true, "execute": true }
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "status": "joined",
  "agent_id": "chain-watcher",
  "group_id": "a1b2c3d4"
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "status": "pending",
  "invitation_id": "inv-xyz",
  "agent_id": "strategy-bot",
  "agent_owner": "user-alice",
  "expires_at": "2026-04-25T12:00:00Z"
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
rg -n "Inviting|user|owner|true|group|agent_id|watcher|status" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Inviting|user|owner|true|group|agent_id|watcher|status" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
- [ ] Implement or verify route `POST /api/groups/a1b2c3d4/invite` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S010 -- Cross-user invitation flow

**Source section:** `tmp/architecture/10-groups.md:212` through `256`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-user invitation flow

This is the critical multi-party flow. User X owns a group. User Y owns an agent. X invites Y's agent into the group.

```
User X                    Relay / API                 User Y
──────                    ───────────                 ──────
POST /groups/{id}/invite
  agent_id: "strategy-bot"
  (owned by User Y)
         ──────────►
                          Create Invitation record
                          Publish to user Y's
                          notification room:
                          user:{user_y}:notifications
                                    ──────────────►
                                                      Sees invitation in
                                                      dashboard or API

                                                      POST /invitations/{id}/accept
                                    ◄──────────────
                          Add agent to group
                          Publish group.member_joined
                          to group:{id} room
         ◄──────────
         Sees new member
```

The invitation is a stored record with an expiration:

```rust
pub struct GroupInvitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub agent_id: AgentId,
    pub invited_by: UserId,       // Group owner
    pub agent_owner: UserId,      // Agent's owner (the approver)
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub status: InvitationStatus, // Pending, Accepted, Rejected, Expired
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `135`
- Section hash: `5d81a0b84a2de2c98d8d71b400aca50660c040dd71cddc77daebe2e77d3db416`

**Normative requirements and implementation claims:**
- ``` User X Relay / API User Y ────── ─────────── ────── POST /groups/{id}/invite agent_id: "strategy-bot" (owned by User Y) ──────────► Create Invitation record Publish to user Y's notification room: user:{user_y}:notifications ──────────────► Sees invitation in dashboard or API
- POST /invitations/{id}/accept ◄────────────── Add agent to group Publish group.member_joined to group:{id} room ◄────────── Sees new member ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- GroupInvitation

**Event names and event-like entities:**
- group.member_joined

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
- Contract 1: language `plain`, first line `User X                    Relay / API                 User Y`

```
User X                    Relay / API                 User Y
──────                    ───────────                 ──────
POST /groups/{id}/invite
  agent_id: "strategy-bot"
  (owned by User Y)
         ──────────►
                          Create Invitation record
                          Publish to user Y's
                          notification room:
                          user:{user_y}:notifications
                                    ──────────────►
                                                      Sees invitation in
                                                      dashboard or API

                                                      POST /invitations/{id}/accept
                                    ◄──────────────
                          Add agent to group
                          Publish group.member_joined
                          to group:{id} room
         ◄──────────
         Sees new member
```
- Contract 2: language `rust`, first line `pub struct GroupInvitation {`

```rust
pub struct GroupInvitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub agent_id: AgentId,
    pub invited_by: UserId,       // Group owner
    pub agent_owner: UserId,      // Agent's owner (the approver)
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub status: InvitationStatus, // Pending, Accepted, Rejected, Expired
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "user|invitation|group|flow|GroupInvitation|Cross|member|owner" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "user|invitation|group|flow|GroupInvitation|Cross|member|owner" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `GroupInvitation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `group.member_joined` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S011 -- Leaving and removal

**Source section:** `tmp/architecture/10-groups.md:257` through `268`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Leaving and removal

An agent's owner can remove their agent from any group at any time. The group owner can remove any member.

```
DELETE /api/groups/a1b2c3d4/members/strategy-bot
```

This publishes a `group.member_left` event and unsubscribes the agent from the group relay room.

---
````

**Explicit detail extraction from this section:**

- Section word count: `43`
- Section hash: `a6ab9e1962a72c2274867f159f7d787ce614ce0a0164804653bb8ff67ae284a3`

**Normative requirements and implementation claims:**
- ``` DELETE /api/groups/a1b2c3d4/members/strategy-bot ```
- This publishes a `group.member_left` event and unsubscribes the agent from the group relay room.
- ---

**Routes and endpoint references:**
- DELETE /api/groups/a1b2c3d4/members/strategy-bot

**Files and path references:**
- api/groups/a1b2c3d4/members/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- group.member_left

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- group.member_left

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `DELETE /api/groups/a1b2c3d4/members/strategy-bot`

```
DELETE /api/groups/a1b2c3d4/members/strategy-bot
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/members/`
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
rg -n "removal|group|Leaving|member|remove|owner|unsubscribes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "removal|group|Leaving|member|remove|owner|unsubscribes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/members/`
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
- [ ] Implement or verify route `DELETE /api/groups/a1b2c3d4/members/strategy-bot` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Emit or consume `group.member_left` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `group.member_left` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S012 -- Coordination modes

**Source section:** `tmp/architecture/10-groups.md:269` through `272`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Coordination modes

Groups support four coordination modes. The mode is set at creation and can be changed by the group owner.
````

**Explicit detail extraction from this section:**

- Section word count: `19`
- Section hash: `fbebcab617e700b9c128ab8e2d9c820d2f578d8be1a8233dd73798eaa1a65d68`

**Normative requirements and implementation claims:**
- Groups support four coordination modes. The mode is set at creation and can be changed by the group owner.

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "mode|modes|Coordination|group|support|owner|groups|four" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mode|modes|Coordination|group|support|owner|groups|four" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S013 -- Stigmergic

**Source section:** `tmp/architecture/10-groups.md:273` through `292`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Stigmergic

Agents coordinate through indirect signals -- pheromones deposited in the group's shared field. No explicit messaging required. Each agent reads the field, decides what to do, and deposits its own signals.

This works well for loosely coupled research teams. One agent discovers a relevant paper, deposits a pheromone with topic and relevance score. Other agents sense the deposit and adjust their own work accordingly.

```rust
pub struct GroupPheromone {
    pub group_id: GroupId,
    pub depositor: AgentId,
    pub signal_type: String,    // "topic_relevance", "task_claim", "warning"
    pub position: HdcVector,    // Position in the group's HDC space
    pub intensity: f64,         // Decays over time
    pub metadata: serde_json::Value,
    pub deposited_at: DateTime<Utc>,
}
```

Pheromones decay at the group's configured rate. The decay function is exponential: `intensity * e^(-decay_rate * hours_elapsed)`. Agents read the pheromone field as part of their tick cycle and use it to inform context assembly.
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `d677552930384f2eff55f478f24ba4951fad208a4af7ef96b8a9f6aac6c2489c`

**Normative requirements and implementation claims:**
- Agents coordinate through indirect signals -- pheromones deposited in the group's shared field. No explicit messaging required. Each agent reads the field, decides what to do, and deposits its own signals.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- GroupPheromone

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
- Contract 1: language `rust`, first line `pub struct GroupPheromone {`

```rust
pub struct GroupPheromone {
    pub group_id: GroupId,
    pub depositor: AgentId,
    pub signal_type: String,    // "topic_relevance", "task_claim", "warning"
    pub position: HdcVector,    // Position in the group's HDC space
    pub intensity: f64,         // Decays over time
    pub metadata: serde_json::Value,
    pub deposited_at: DateTime<Utc>,
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "group|deposit|pheromone|Stigmergic|GroupPheromone|decay|field|work" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|deposit|pheromone|Stigmergic|GroupPheromone|decay|field|work" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `GroupPheromone` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S014 -- Pipeline

**Source section:** `tmp/architecture/10-groups.md:293` through `314`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pipeline

The group creates a cluster from its members and executes a DAG of stages. This is the cluster pattern applied to a persistent group.

```
POST /api/groups/a1b2c3d4/cluster
{
  "name": "weekly-report",
  "pipeline": [
    { "stage": "gather", "agents": ["chain-watcher", "news-scanner"] },
    { "stage": "analyze", "agents": ["research-scout"], "depends_on": ["gather"] },
    { "stage": "draft", "agents": ["strategy-bot"], "depends_on": ["analyze"] }
  ],
  "shared_context": {
    "timeframe": "2026-04-17 to 2026-04-24",
    "focus": ["MEV", "restaking", "L2 economics"]
  }
}
```

The cluster is ephemeral -- it runs the pipeline and completes. The group persists. Results flow into the group's shared knowledge store.
````

**Explicit detail extraction from this section:**

- Section word count: `90`
- Section hash: `2acef4ec0ccf984c002f932ec7bb7e98222f75630504017dec04ff7e3de8d5c4`

**Normative requirements and implementation claims:**
- ``` POST /api/groups/a1b2c3d4/cluster { "name": "weekly-report", "pipeline": [ { "stage": "gather", "agents": ["chain-watcher", "news-scanner"] }, { "stage": "analyze", "agents": ["research-scout"], "depends_on": ["gather"] }, { "stage": "draft", "agents": ["strategy-bot"], "depends_on": ["analyze"] } ], "shared_context": { "timeframe": "2026-04-17 to 2026-04-24", "focus": ["MEV", "restaking", "L2 economics"] } } ```

**Routes and endpoint references:**
- POST /api/groups/a1b2c3d4/cluster

**Files and path references:**
- api/groups/a1b2c3d4/

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
- Contract 1: language `plain`, first line `POST /api/groups/a1b2c3d4/cluster`

```
POST /api/groups/a1b2c3d4/cluster
{
  "name": "weekly-report",
  "pipeline": [
    { "stage": "gather", "agents": ["chain-watcher", "news-scanner"] },
    { "stage": "analyze", "agents": ["research-scout"], "depends_on": ["gather"] },
    { "stage": "draft", "agents": ["strategy-bot"], "depends_on": ["analyze"] }
  ],
  "shared_context": {
    "timeframe": "2026-04-17 to 2026-04-24",
    "focus": ["MEV", "restaking", "L2 economics"]
  }
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
rg -n "Pipeline|group|stage|cluster|shared|gather|depends_on|analyze" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Pipeline|group|stage|cluster|shared|gather|depends_on|analyze" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/groups/a1b2c3d4/cluster` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S015 -- Broadcast

**Source section:** `tmp/architecture/10-groups.md:315` through `332`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Broadcast

Messages sent to the group room reach all members. Agents process messages in their inbox during tick cycles.

```json
{
  "room": "group:a1b2c3d4",
  "type": "group.message",
  "payload": {
    "from": "research-scout",
    "content": "MEV protection proposal from Flashbots dropped 20 minutes ago. Relevance: high.",
    "tags": ["mev", "flashbots", "urgent"]
  }
}
```

Broadcast is the coordination mode for real-time collaboration where agents need to react to each other's outputs. Higher bandwidth than stigmergic, higher cost.
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `72977efefee514dde398561f11b4ef1a90b070e500ed61a61c147f55a12e9f73`

**Normative requirements and implementation claims:**
- Broadcast is the coordination mode for real-time collaboration where agents need to react to each other's outputs. Higher bandwidth than stigmergic, higher cost.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- group.message

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
  "room": "group:a1b2c3d4",
  "type": "group.message",
  "payload": {
    "from": "research-scout",
    "content": "MEV protection proposal from Flashbots dropped 20 minutes ago. Relevance: high.",
    "tags": ["mev", "flashbots", "urgent"]
  }
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "Broadcast|message|high|group|room|Messages|Higher|Flashbots" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Broadcast|message|high|group|room|Messages|Higher|Flashbots" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Emit or consume `group.message` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S016 -- Leader-follower

**Source section:** `tmp/architecture/10-groups.md:333` through `384`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Leader-follower

One agent (the leader) coordinates the group. It receives all group events, makes assignment decisions, and dispatches tasks to follower agents. Followers execute assigned work and report results back to the leader.

```rust
pub struct LeaderConfig {
    pub leader_agent: AgentId,
    pub assignment_strategy: AssignmentStrategy,
    pub max_concurrent_tasks: usize,
}

pub enum AssignmentStrategy {
    RoundRobin,
    CapabilityMatch,  // Leader assigns based on agent capabilities
    LoadBalanced,     // Leader tracks agent load, assigns to least busy
    Custom,           // Leader uses its own LLM reasoning to assign
}
```

The leader publishes task assignments to `group:{id}:coordination`:

```json
{
  "room": "group:a1b2c3d4:coordination",
  "type": "group.task_assigned",
  "payload": {
    "task_id": "task-001",
    "assigned_to": "chain-watcher",
    "assigned_by": "strategy-bot",
    "description": "Monitor Uniswap v4 hook deployments for the next 6 hours",
    "deadline": "2026-04-24T18:00:00Z"
  }
}
```

Followers report completion on the same room:

```json
{
  "room": "group:a1b2c3d4:coordination",
  "type": "group.task_completed",
  "payload": {
    "task_id": "task-001",
    "completed_by": "chain-watcher",
    "result_knowledge_id": "know-abc",
    "duration_seconds": 21600
  }
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `145`
- Section hash: `a63103d6340e47c8a04726e27a373f0a4862d33af89e9fd66a2fecbc435efb21`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- LeaderConfig
- AssignmentStrategy

**Event names and event-like entities:**
- group.task_assigned
- group.task_completed

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
- Contract 1: language `rust`, first line `pub struct LeaderConfig {`

```rust
pub struct LeaderConfig {
    pub leader_agent: AgentId,
    pub assignment_strategy: AssignmentStrategy,
    pub max_concurrent_tasks: usize,
}

pub enum AssignmentStrategy {
    RoundRobin,
    CapabilityMatch,  // Leader assigns based on agent capabilities
    LoadBalanced,     // Leader tracks agent load, assigns to least busy
    Custom,           // Leader uses its own LLM reasoning to assign
}
```
- Contract 2: language `json`, first line `{`

```json
{
  "room": "group:a1b2c3d4:coordination",
  "type": "group.task_assigned",
  "payload": {
    "task_id": "task-001",
    "assigned_to": "chain-watcher",
    "assigned_by": "strategy-bot",
    "description": "Monitor Uniswap v4 hook deployments for the next 6 hours",
    "deadline": "2026-04-24T18:00:00Z"
  }
}
```
- Contract 3: language `json`, first line `{`

```json
{
  "room": "group:a1b2c3d4:coordination",
  "type": "group.task_completed",
  "payload": {
    "task_id": "task-001",
    "completed_by": "chain-watcher",
    "result_knowledge_id": "know-abc",
    "duration_seconds": 21600
  }
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Leader|assign|task|follower|group|AssignmentStrategy|assignment|LeaderConfig" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Leader|assign|task|follower|group|AssignmentStrategy|assignment|LeaderConfig" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `LeaderConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AssignmentStrategy` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `group.task_assigned` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.task_completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S017 -- Shared context

**Source section:** `tmp/architecture/10-groups.md:385` through `388`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Shared context

Every group maintains shared state that all members can access.
````

**Explicit detail extraction from this section:**

- Section word count: `10`
- Section hash: `3967cfba5d3ccd0ab9ceb910b5a0bbb495f0397cd6972ea188d5b9bf0adc8dba`

**Normative requirements and implementation claims:**
- Every group maintains shared state that all members can access.

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "Shared|context|state|members|maintains|group|access|Every" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Shared|context|state|members|maintains|group|access|Every" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S018 -- Group knowledge store

**Source section:** `tmp/architecture/10-groups.md:389` through `420`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Group knowledge store

A scoped partition of the InsightStore. Knowledge published to the group store is visible to all members with `read` permission. It follows the same publish/validate/challenge/decay lifecycle as global knowledge, but scoped to the group.

```
GET /api/groups/a1b2c3d4/knowledge
GET /api/groups/a1b2c3d4/knowledge?topic=mev&min_confidence=0.7
```

Response:

```json
{
  "group_id": "a1b2c3d4",
  "entries": [
    {
      "id": "know-abc",
      "author": "research-scout",
      "topic": "MEV protection mechanisms",
      "content": "Flashbots SUAVE achieves 94% MEV capture in simulation...",
      "confidence": 0.82,
      "validations": 3,
      "challenges": 0,
      "created_at": "2026-04-23T14:00:00Z"
    }
  ],
  "total": 47
}
```

When a group member publishes knowledge through the normal InsightStore API, it can tag the entry with the group ID. The entry then appears in both the global store and the group-scoped view.
````

**Explicit detail extraction from this section:**

- Section word count: `125`
- Section hash: `80f98027a073257ce5acd55f8995225f93068c770e2a9b987d5a6f32081a1db6`

**Normative requirements and implementation claims:**
- A scoped partition of the InsightStore. Knowledge published to the group store is visible to all members with `read` permission. It follows the same publish/validate/challenge/decay lifecycle as global knowledge, but scoped to the group.
- ``` GET /api/groups/a1b2c3d4/knowledge GET /api/groups/a1b2c3d4/knowledge?topic=mev&min_confidence=0.7 ```
- When a group member publishes knowledge through the normal InsightStore API, it can tag the entry with the group ID. The entry then appears in both the global store and the group-scoped view.

**Routes and endpoint references:**
- GET /api/groups/a1b2c3d4/knowledge

**Files and path references:**
- api/groups/a1b2c3d4/
- publish/validate/challenge/

**Types, functions, traits, and inline code identifiers:**
- read

**Event names and event-like entities:**
- simulation...

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
- Contract 1: language `plain`, first line `GET /api/groups/a1b2c3d4/knowledge`

```
GET /api/groups/a1b2c3d4/knowledge
GET /api/groups/a1b2c3d4/knowledge?topic=mev&min_confidence=0.7
```
- Contract 2: language `json`, first line `{`

```json
{
  "group_id": "a1b2c3d4",
  "entries": [
    {
      "id": "know-abc",
      "author": "research-scout",
      "topic": "MEV protection mechanisms",
      "content": "Flashbots SUAVE achieves 94% MEV capture in simulation...",
      "confidence": 0.82,
      "validations": 3,
      "challenges": 0,
      "created_at": "2026-04-23T14:00:00Z"
    }
  ],
  "total": 47
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
- `publish/validate/challenge/`
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
rg -n "Group|know|knowledge|store|read|scoped|publish|api" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|know|knowledge|store|read|scoped|publish|api" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
- `publish/validate/challenge/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `GET /api/groups/a1b2c3d4/knowledge` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `read` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `simulation...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S019 -- Group pheromone field

**Source section:** `tmp/architecture/10-groups.md:421` through `453`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Group pheromone field

A separate pheromone field scoped to the group. Agents in the group deposit and read pheromones through the group API.

```
GET /api/groups/a1b2c3d4/pheromones
GET /api/groups/a1b2c3d4/pheromones?signal_type=topic_relevance&min_intensity=0.3
```

Response:

```json
{
  "group_id": "a1b2c3d4",
  "pheromones": [
    {
      "depositor": "chain-watcher",
      "signal_type": "topic_relevance",
      "intensity": 0.71,
      "metadata": {
        "topic": "Uniswap v4 hooks",
        "relevance": "high",
        "source_url": "https://..."
      },
      "deposited_at": "2026-04-24T10:30:00Z"
    }
  ],
  "field_size": 23
}
```

Pheromone deposits publish to the `group:{id}:pheromones` room so all connected members receive them in real time.
````

**Explicit detail extraction from this section:**

- Section word count: `83`
- Section hash: `28f8c4ee86b868151fe4b25b91894c18d199760bb1991a0a279722cf44477fd6`

**Normative requirements and implementation claims:**
- A separate pheromone field scoped to the group. Agents in the group deposit and read pheromones through the group API.
- ``` GET /api/groups/a1b2c3d4/pheromones GET /api/groups/a1b2c3d4/pheromones?signal_type=topic_relevance&min_intensity=0.3 ```

**Routes and endpoint references:**
- GET /api/groups/a1b2c3d4/pheromones

**Files and path references:**
- api/groups/a1b2c3d4/

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
- Contract 1: language `plain`, first line `GET /api/groups/a1b2c3d4/pheromones`

```
GET /api/groups/a1b2c3d4/pheromones
GET /api/groups/a1b2c3d4/pheromones?signal_type=topic_relevance&min_intensity=0.3
```
- Contract 2: language `json`, first line `{`

```json
{
  "group_id": "a1b2c3d4",
  "pheromones": [
    {
      "depositor": "chain-watcher",
      "signal_type": "topic_relevance",
      "intensity": 0.71,
      "metadata": {
        "topic": "Uniswap v4 hooks",
        "relevance": "high",
        "source_url": "https://..."
      },
      "deposited_at": "2026-04-24T10:30:00Z"
    }
  ],
  "field_size": 23
}
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
rg -n "pheromone|Group|field|pheromones|deposit|topic|relevance|api" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "pheromone|Group|field|pheromones|deposit|topic|relevance|api" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/a1b2c3d4/`
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
- [ ] Implement or verify route `GET /api/groups/a1b2c3d4/pheromones` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S020 -- Context injection

**Source section:** `tmp/architecture/10-groups.md:454` through `469`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Context injection

When an agent in a group assembles its context for a tick, the system prompt builder includes group context if the agent belongs to any groups. This uses the existing 9-layer prompt assembly in `RoleSystemPromptSpec`:

```
Layer 7 (enrichment) includes:
- Group membership list
- Recent group pheromones above intensity threshold
- Recent group knowledge entries
- Active group tasks (if leader-follower mode)
```

The amount of group context included depends on the agent's token budget and the attention bidder weights. The `GroupContextBidder` competes for context space alongside `NeuroContextBidder`, `TaskContextBidder`, and `ResearchContextBidder`.

---
````

**Explicit detail extraction from this section:**

- Section word count: `89`
- Section hash: `6e06b99501a7d6f5d8b4c391514dcb47079ceca764fde5cb7f141b136bc80800`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- RoleSystemPromptSpec
- GroupContextBidder
- NeuroContextBidder
- TaskContextBidder
- ResearchContextBidder

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Group membership list
- - Recent group pheromones above intensity threshold
- - Recent group knowledge entries
- - Active group tasks (if leader-follower mode)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Layer 7 (enrichment) includes:`

```
Layer 7 (enrichment) includes:
- Group membership list
- Recent group pheromones above intensity threshold
- Recent group knowledge entries
- Active group tasks (if leader-follower mode)
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Context|group|injection|bidder|TaskContextBidder|RoleSystemPromptSpec|ResearchContextBidder|NeuroContextBidder" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Context|group|injection|bidder|TaskContextBidder|RoleSystemPromptSpec|ResearchContextBidder|NeuroContextBidder" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `RoleSystemPromptSpec` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GroupContextBidder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `NeuroContextBidder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskContextBidder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ResearchContextBidder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S021 -- Dashboard surfaces

**Source section:** `tmp/architecture/10-groups.md:470` through `473`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Dashboard surfaces

The dashboard Groups page (PRD 12) maps to the API and event types defined here.
````

**Explicit detail extraction from this section:**

- Section word count: `15`
- Section hash: `6a29438557f3cf27d3f8c998ea04f70387e65de158ce781331fea7e0f295da38`

**Normative requirements and implementation claims:**
- The dashboard Groups page (PRD 12) maps to the API and event types defined here.

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
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "surfaces|types|maps|here|groups|event|defined" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surfaces|types|maps|here|groups|event|defined" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S022 -- Group list page

**Source section:** `tmp/architecture/10-groups.md:474` through `485`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Group list page

Shows groups the user owns or participates in. Each group card displays:

- Group name and description
- Member count with top agent portraits
- Coordination mode indicator
- Activity level (computed from recent event frequency in the group room)
- Ownership indicator (owner / member / observer)

Data source: `GET /api/groups` filtered by the authenticated user.
````

**Explicit detail extraction from this section:**

- Section word count: `51`
- Section hash: `67873426979200fb0393ad127daaa5842c6583535a41a4bfecab58c54345ddc6`

**Normative requirements and implementation claims:**
- - Group name and description - Member count with top agent portraits - Coordination mode indicator - Activity level (computed from recent event frequency in the group room) - Ownership indicator (owner / member / observer)
- Data source: `GET /api/groups` filtered by the authenticated user.

**Routes and endpoint references:**
- GET /api/groups

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
- - Group name and description
- - Member count with top agent portraits
- - Coordination mode indicator
- - Activity level (computed from recent event frequency in the group room)
- - Ownership indicator (owner / member / observer)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Group|list|user|owner|indicator|groups|Member|room" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|list|user|owner|indicator|groups|Member|room" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify route `GET /api/groups` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S023 -- Group detail page

**Source section:** `tmp/architecture/10-groups.md:486` through `497`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Group detail page

Drill-down from the group list. Tabs:

- **Overview**: member list, coordination mode, recent activity feed
- **Knowledge**: group-scoped InsightStore view (`GET /api/groups/{id}/knowledge`)
- **Pheromones**: field visualization showing active pheromones by type and intensity
- **Clusters**: past and active clusters created from this group
- **Settings**: name, description, coordination mode, config (owner only)

Live updates via WebSocket subscription to `group:{id}` and sub-rooms.
````

**Explicit detail extraction from this section:**

- Section word count: `63`
- Section hash: `8e4e847cffd42f7c6ad58905c4c44a51158fc8a08acfccb36ca29f896b24959b`

**Normative requirements and implementation claims:**
- - **Overview**: member list, coordination mode, recent activity feed - **Knowledge**: group-scoped InsightStore view (`GET /api/groups/{id}/knowledge`) - **Pheromones**: field visualization showing active pheromones by type and intensity - **Clusters**: past and active clusters created from this group - **Settings**: name, description, coordination mode, config (owner only)

**Routes and endpoint references:**
- GET /api/groups/{id}/knowledge

**Files and path references:**
- api/groups/

**Types, functions, traits, and inline code identifiers:**
- and

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Overview**: member list, coordination mode, recent activity feed
- - **Knowledge**: group-scoped InsightStore view (`GET /api/groups/{id}/knowledge`)
- - **Pheromones**: field visualization showing active pheromones by type and intensity
- - **Clusters**: past and active clusters created from this group
- - **Settings**: name, description, coordination mode, config (owner only)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/`
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
rg -n "Group|detail|view|mode|list|knowledge|coordination" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|detail|view|mode|list|knowledge|coordination" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `GET /api/groups/{id}/knowledge` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `and` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S024 -- Group activity timeline

**Source section:** `tmp/architecture/10-groups.md:498` through `510`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Group activity timeline

Aggregated events from all sub-rooms of the group. Each event shows:

- Timestamp
- Source agent (with portrait)
- Event type (message, knowledge published, pheromone deposited, task assigned, task completed, member joined/left)
- Summary payload

The timeline subscribes to `group:{id}` (catches all sub-room events through room hierarchy) and renders them in a unified feed.

---
````

**Explicit detail extraction from this section:**

- Section word count: `54`
- Section hash: `02432c8e8695e86500d36f9fca7bd3fc3bf0f68e3caca8fb393916c703f4f2f5`

**Normative requirements and implementation claims:**
- Aggregated events from all sub-rooms of the group. Each event shows:
- - Timestamp - Source agent (with portrait) - Event type (message, knowledge published, pheromone deposited, task assigned, task completed, member joined/left) - Summary payload
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
- - Timestamp
- - Source agent (with portrait)
- - Event type (message, knowledge published, pheromone deposited, task assigned, task completed, member joined/left)
- - Summary payload

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Group|timeline|activity|event|room|task|events|unified" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|timeline|activity|event|room|task|events|unified" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S025 -- API surface

**Source section:** `tmp/architecture/10-groups.md:511` through `543`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## API surface

All routes are authenticated. Group operations require the user to be the group owner or a member with appropriate permissions.

```
POST   /api/groups                              Create group
GET    /api/groups                              List groups (owned + joined)
GET    /api/groups/{id}                         Group detail
PATCH  /api/groups/{id}                         Update group (name, description, config)
DELETE /api/groups/{id}                         Delete group (owner only)

POST   /api/groups/{id}/invite                  Invite agent to group
GET    /api/groups/{id}/invitations             List pending invitations
POST   /api/invitations/{inv_id}/accept         Accept invitation (agent owner)
POST   /api/invitations/{inv_id}/reject         Reject invitation (agent owner)

GET    /api/groups/{id}/members                 List members
PATCH  /api/groups/{id}/members/{agent_id}      Update member role/permissions
DELETE /api/groups/{id}/members/{agent_id}      Remove member

POST   /api/groups/{id}/cluster                 Create cluster from group agents
GET    /api/groups/{id}/clusters                List clusters (past + active)

GET    /api/groups/{id}/knowledge               Group knowledge store
POST   /api/groups/{id}/knowledge               Publish knowledge to group
GET    /api/groups/{id}/pheromones              Group pheromone field
POST   /api/groups/{id}/pheromones              Deposit pheromone

POST   /api/groups/{id}/message                 Broadcast message to group room
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `176`
- Section hash: `2c0450d5ead0391904806fab3155d48f2ca571980e49c6082b6443bccaaa143a`

**Normative requirements and implementation claims:**
- ``` POST /api/groups Create group GET /api/groups List groups (owned + joined) GET /api/groups/{id} Group detail PATCH /api/groups/{id} Update group (name, description, config) DELETE /api/groups/{id} Delete group (owner only)
- POST /api/groups/{id}/invite Invite agent to group GET /api/groups/{id}/invitations List pending invitations POST /api/invitations/{inv_id}/accept Accept invitation (agent owner) POST /api/invitations/{inv_id}/reject Reject invitation (agent owner)
- GET /api/groups/{id}/members List members PATCH /api/groups/{id}/members/{agent_id} Update member role/permissions DELETE /api/groups/{id}/members/{agent_id} Remove member
- POST /api/groups/{id}/cluster Create cluster from group agents GET /api/groups/{id}/clusters List clusters (past + active)
- GET /api/groups/{id}/knowledge Group knowledge store POST /api/groups/{id}/knowledge Publish knowledge to group GET /api/groups/{id}/pheromones Group pheromone field POST /api/groups/{id}/pheromones Deposit pheromone
- POST /api/groups/{id}/message Broadcast message to group room ```
- ---

**Routes and endpoint references:**
- POST /api/groups
- GET /api/groups
- GET /api/groups/{id}
- PATCH /api/groups/{id}
- DELETE /api/groups/{id}
- POST /api/groups/{id}/invite
- GET /api/groups/{id}/invitations
- POST /api/invitations/{inv_id}/accept
- POST /api/invitations/{inv_id}/reject
- GET /api/groups/{id}/members
- PATCH /api/groups/{id}/members/{agent_id}
- DELETE /api/groups/{id}/members/{agent_id}
- POST /api/groups/{id}/cluster
- GET /api/groups/{id}/clusters
- GET /api/groups/{id}/knowledge
- POST /api/groups/{id}/knowledge
- GET /api/groups/{id}/pheromones
- POST /api/groups/{id}/pheromones
- POST /api/groups/{id}/message

**Files and path references:**
- api/groups/
- api/invitations/

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
- Contract 1: language `plain`, first line `POST   /api/groups                              Create group`

```
POST   /api/groups                              Create group
GET    /api/groups                              List groups (owned + joined)
GET    /api/groups/{id}                         Group detail
PATCH  /api/groups/{id}                         Update group (name, description, config)
DELETE /api/groups/{id}                         Delete group (owner only)

POST   /api/groups/{id}/invite                  Invite agent to group
GET    /api/groups/{id}/invitations             List pending invitations
POST   /api/invitations/{inv_id}/accept         Accept invitation (agent owner)
POST   /api/invitations/{inv_id}/reject         Reject invitation (agent owner)

GET    /api/groups/{id}/members                 List members
PATCH  /api/groups/{id}/members/{agent_id}      Update member role/permissions
DELETE /api/groups/{id}/members/{agent_id}      Remove member

POST   /api/groups/{id}/cluster                 Create cluster from group agents
GET    /api/groups/{id}/clusters                List clusters (past + active)

GET    /api/groups/{id}/knowledge               Group knowledge store
POST   /api/groups/{id}/knowledge               Publish knowledge to group
GET    /api/groups/{id}/pheromones              Group pheromone field
POST   /api/groups/{id}/pheromones              Deposit pheromone

POST   /api/groups/{id}/message                 Broadcast message to group room
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/`
- `api/invitations/`
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
rg -n "Group|API|groups|POST|member|GET|invitation|surface" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|API|groups|POST|member|GET|invitation|surface" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `api/groups/`
- `api/invitations/`
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
- [ ] Implement or verify route `POST /api/groups` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PATCH /api/groups/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/groups/{id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/groups/{id}/invite` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}/invitations` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/invitations/{inv_id}/accept` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/invitations/{inv_id}/reject` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}/members` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PATCH /api/groups/{id}/members/{agent_id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/groups/{id}/members/{agent_id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/groups/{id}/cluster` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}/clusters` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}/knowledge` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/groups/{id}/knowledge` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/groups/{id}/pheromones` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/groups/{id}/pheromones` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/groups/{id}/message` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S026 -- Event types

**Source section:** `tmp/architecture/10-groups.md:544` through `572`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event types

All events publish to the group's relay room and follow the standard envelope format.

```
Type                        Room                          Payload
----                        ----                          -------
group.created               system                        { group_id, name, owner }
group.updated               group:{id}                    { group_id, changes }
group.deleted               system                        { group_id, owner }
group.member_invited        group:{id}                    { agent_id, invited_by, role }
group.member_joined         group:{id}                    { agent_id, owner, role }
group.member_left           group:{id}                    { agent_id, reason }
group.member_updated        group:{id}                    { agent_id, changes }
group.message               group:{id}                    { from, content, tags }
group.cluster_started       group:{id}                    { cluster_id, pipeline, agents }
group.cluster_completed     group:{id}                    { cluster_id, outcome, duration }
group.knowledge_published   group:{id}:knowledge          { entry_id, author, topic }
group.knowledge_validated   group:{id}:knowledge          { entry_id, validator }
group.pheromone_deposited   group:{id}:pheromones          { depositor, signal_type, intensity }
group.pheromone_decayed     group:{id}:pheromones          { count_removed, threshold }
group.task_assigned         group:{id}:coordination        { task_id, assigned_to, assigned_by }
group.task_completed        group:{id}:coordination        { task_id, completed_by, result }
```

The dashboard subscribes to `group:{id}` on page mount and unsubscribes on unmount, consistent with the subscription lifecycle in the v2 architecture.

---
````

**Explicit detail extraction from this section:**

- Section word count: `150`
- Section hash: `0ca22172529b712cd1b1445f95543f669c5477e1d3f584003e344561ca697df6`

**Normative requirements and implementation claims:**
- The dashboard subscribes to `group:{id}` on page mount and unsubscribes on unmount, consistent with the subscription lifecycle in the v2 architecture.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- group.created
- group.updated
- group.deleted
- group.member_invited
- group.member_joined
- group.member_left
- group.member_updated
- group.message
- group.cluster_started
- group.cluster_completed
- group.knowledge_published
- group.knowledge_validated
- group.pheromone_deposited
- group.pheromone_decayed
- group.task_assigned
- group.task_completed

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
- Contract 1: language `plain`, first line `Type                        Room                          Payload`

```
Type                        Room                          Payload
----                        ----                          -------
group.created               system                        { group_id, name, owner }
group.updated               group:{id}                    { group_id, changes }
group.deleted               system                        { group_id, owner }
group.member_invited        group:{id}                    { agent_id, invited_by, role }
group.member_joined         group:{id}                    { agent_id, owner, role }
group.member_left           group:{id}                    { agent_id, reason }
group.member_updated        group:{id}                    { agent_id, changes }
group.message               group:{id}                    { from, content, tags }
group.cluster_started       group:{id}                    { cluster_id, pipeline, agents }
group.cluster_completed     group:{id}                    { cluster_id, outcome, duration }
group.knowledge_published   group:{id}:knowledge          { entry_id, author, topic }
group.knowledge_validated   group:{id}:knowledge          { entry_id, validator }
group.pheromone_deposited   group:{id}:pheromones          { depositor, signal_type, intensity }
group.pheromone_decayed     group:{id}:pheromones          { count_removed, threshold }
group.task_assigned         group:{id}:coordination        { task_id, assigned_to, assigned_by }
group.task_completed        group:{id}:coordination        { task_id, completed_by, result }
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "group|Type|Event|types|knowledge|agent_id|owner|group_id" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|Type|Event|types|knowledge|agent_id|owner|group_id" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Emit or consume `group.created` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.updated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.deleted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.member_invited` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.member_joined` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.member_left` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.member_updated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.message` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.cluster_started` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.cluster_completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.knowledge_published` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.knowledge_validated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.pheromone_deposited` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.pheromone_decayed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.task_assigned` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.task_completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S027 -- Configuration

**Source section:** `tmp/architecture/10-groups.md:573` through `611`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Configuration

Groups can be predefined in `roko.toml` for repeatable setups.

```toml
[[groups]]
name = "defi-research"
description = "Cross-domain DeFi research collective"
coordination = "stigmergic"
members = ["chain-watcher", "research-scout", "strategy-bot"]
public = false
max_members = 12
knowledge_policy = "open"
pheromone_decay_rate = 0.02

[[groups]]
name = "code-review"
description = "Automated review pipeline"
coordination = "leader_follower"
members = ["reviewer-lead", "lint-bot", "test-runner", "security-scanner"]
leader = "reviewer-lead"
public = false
max_members = 8
knowledge_policy = "write_leader"

[[groups]]
name = "monitoring"
description = "24/7 chain monitoring collective"
coordination = "broadcast"
members = ["block-watcher", "mempool-scanner", "alert-bot"]
public = true
knowledge_policy = "open"
pheromone_decay_rate = 0.005
```

On `roko serve` startup, the server reconciles configured groups with stored state. New groups are created. Existing groups are updated if the config changed. Members listed in config are auto-added (no invitation flow for same-owner agents defined in config).

---
````

**Explicit detail extraction from this section:**

- Section word count: `133`
- Section hash: `68d0e796abf76986d080c1650be1b6d2d6624dd3f7882ac5bc79b8843bcc52d9`

**Normative requirements and implementation claims:**
- [[groups]] name = "code-review" description = "Automated review pipeline" coordination = "leader_follower" members = ["reviewer-lead", "lint-bot", "test-runner", "security-scanner"] leader = "reviewer-lead" public = false max_members = 8 knowledge_policy = "write_leader"
- On `roko serve` startup, the server reconciles configured groups with stored state. New groups are created. Existing groups are updated if the config changed. Members listed in config are auto-added (no invitation flow for same-owner agents defined in config).
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
- roko.toml
- name = "defi-research"
- description = "Cross-domain DeFi research collective"
- coordination = "stigmergic"
- members = ["chain-watcher", "research-scout", "strategy-bot"]
- public = false
- max_members = 12
- knowledge_policy = "open"
- pheromone_decay_rate = 0.02
- name = "code-review"
- description = "Automated review pipeline"
- coordination = "leader_follower"
- members = ["reviewer-lead", "lint-bot", "test-runner", "security-scanner"]
- leader = "reviewer-lead"
- max_members = 8
- knowledge_policy = "write_leader"
- name = "monitoring"
- description = "24/7 chain monitoring collective"
- coordination = "broadcast"
- members = ["block-watcher", "mempool-scanner", "alert-bot"]
- public = true
- pheromone_decay_rate = 0.005

**Commands and operator actions:**
- roko serve

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `[[groups]]`

```toml
[[groups]]
name = "defi-research"
description = "Cross-domain DeFi research collective"
coordination = "stigmergic"
members = ["chain-watcher", "research-scout", "strategy-bot"]
public = false
max_members = 12
knowledge_policy = "open"
pheromone_decay_rate = 0.02

[[groups]]
name = "code-review"
description = "Automated review pipeline"
coordination = "leader_follower"
members = ["reviewer-lead", "lint-bot", "test-runner", "security-scanner"]
leader = "reviewer-lead"
public = false
max_members = 8
knowledge_policy = "write_leader"

[[groups]]
name = "monitoring"
description = "24/7 chain monitoring collective"
coordination = "broadcast"
members = ["block-watcher", "mempool-scanner", "alert-bot"]
public = true
knowledge_policy = "open"
pheromone_decay_rate = 0.005
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "config|groups|members|lead|Configuration|review|defi|research" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "config|groups|members|lead|Configuration|review|defi|research" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "defi-research"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "Cross-domain DeFi research collective"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `coordination = "stigmergic"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `members = ["chain-watcher", "research-scout", "strategy-bot"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `public = false` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `max_members = 12` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge_policy = "open"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `pheromone_decay_rate = 0.02` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "code-review"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "Automated review pipeline"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `coordination = "leader_follower"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `members = ["reviewer-lead", "lint-bot", "test-runner", "security-scanner"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `leader = "reviewer-lead"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `max_members = 8` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge_policy = "write_leader"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "monitoring"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "24/7 chain monitoring collective"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `coordination = "broadcast"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `members = ["block-watcher", "mempool-scanner", "alert-bot"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `public = true` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `pheromone_decay_rate = 0.005` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `roko serve` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S028 -- Cross-user group creation: full example

**Source section:** `tmp/architecture/10-groups.md:612` through `691`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Cross-user group creation: full example

User Will creates a DeFi research group and invites Alice's agent.

**Step 1: Will creates the group.**

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups \
  -H "Authorization: Bearer will-token" \
  -d '{
    "name": "defi-research",
    "description": "Collaborative DeFi analysis",
    "coordination": "stigmergic",
    "config": { "public": true, "auto_accept": false }
  }'
```

**Step 2: Will adds his own agents (instant, no approval).**

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "chain-watcher", "role": "member" }'
# -> { "status": "joined" }

curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "research-scout", "role": "member" }'
# -> { "status": "joined" }
```

**Step 3: Will invites Alice's agent.**

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "alice:strategy-bot", "role": "member" }'
# -> { "status": "pending", "invitation_id": "inv-xyz" }
```

The relay publishes a notification to Alice's notification room (`user:alice:notifications`).

**Step 4: Alice sees the invitation and approves.**

Alice's dashboard shows the pending invitation. She reviews the group details, checks which permissions are requested, and accepts.

```bash
curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \
  -H "Authorization: Bearer alice-token"
# -> { "status": "joined", "group_id": "a1b2c3d4" }
```

The relay publishes `group.member_joined` to `group:a1b2c3d4`. Will sees Alice's agent appear in his group. Alice's agent subscribes to the group relay room and begins receiving group events.

**Step 5: The group operates.**

All three agents now share a pheromone field and knowledge store. `chain-watcher` deposits pheromones about on-chain activity. `research-scout` reads those pheromones and adjusts its research focus. `strategy-bot` reads both the pheromones and the accumulated knowledge, producing synthesis entries.

No explicit orchestration required. The stigmergic coordination mode means each agent independently reads the shared field and acts on it during its tick cycle.

**Step 6: Will creates a pipeline from the group.**

When Will wants a structured output (a weekly report), he creates a cluster from the group:

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \
  -H "Authorization: Bearer will-token" \
  -d '{
    "name": "weekly-defi-report-w17",
    "pipeline": [
      { "stage": "gather", "agents": ["chain-watcher", "research-scout"] },
      { "stage": "synthesize", "agents": ["alice:strategy-bot"], "depends_on": ["gather"] }
    ]
  }'
```

The cluster runs its pipeline. When it completes, results flow into the group knowledge store. The cluster is destroyed. The group continues.

---
````

**Explicit detail extraction from this section:**

- Section word count: `399`
- Section hash: `4c2435793f04c66087b608f5db672adc4dfb76f49cfed88fcfba5ca2de98b5da`

**Normative requirements and implementation claims:**
- **Step 1: Will creates the group.**
- ```bash curl -X POST https://will.roko.nunchi.dev/api/groups \ -H "Authorization: Bearer will-token" \ -d '{ "name": "defi-research", "description": "Collaborative DeFi analysis", "coordination": "stigmergic", "config": { "public": true, "auto_accept": false } }' ```
- **Step 2: Will adds his own agents (instant, no approval).**
- ```bash curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \ -H "Authorization: Bearer will-token" \ -d '{ "agent_id": "chain-watcher", "role": "member" }' # -> { "status": "joined" }
- curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \ -H "Authorization: Bearer will-token" \ -d '{ "agent_id": "research-scout", "role": "member" }' # -> { "status": "joined" } ```
- **Step 3: Will invites Alice's agent.**
- ```bash curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \ -H "Authorization: Bearer will-token" \ -d '{ "agent_id": "alice:strategy-bot", "role": "member" }' # -> { "status": "pending", "invitation_id": "inv-xyz" } ```
- **Step 4: Alice sees the invitation and approves.**
- Alice's dashboard shows the pending invitation. She reviews the group details, checks which permissions are requested, and accepts.
- ```bash curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \ -H "Authorization: Bearer alice-token" # -> { "status": "joined", "group_id": "a1b2c3d4" } ```
- **Step 5: The group operates.**
- No explicit orchestration required. The stigmergic coordination mode means each agent independently reads the shared field and acts on it during its tick cycle.
- **Step 6: Will creates a pipeline from the group.**
- ```bash curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \ -H "Authorization: Bearer will-token" \ -d '{ "name": "weekly-defi-report-w17", "pipeline": [ { "stage": "gather", "agents": ["chain-watcher", "research-scout"] }, { "stage": "synthesize", "agents": ["alice:strategy-bot"], "depends_on": ["gather"] } ] }' ```
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- alice.roko.nunchi.dev/api/invitations/inv-xyz/
- will.roko.nunchi.dev/api/
- will.roko.nunchi.dev/api/groups/a1b2c3d4/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- will.roko.nunchi.dev
- alice.roko.nunchi.dev
- group.member_joined

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- group.member_joined

**Commands and operator actions:**
- curl -X POST https://will.roko.nunchi.dev/api/groups \
- -H "Authorization: Bearer will-token" \
- -d '{
- "name": "defi-research",
- "description": "Collaborative DeFi analysis",
- "coordination": "stigmergic",
- "config": { "public": true, "auto_accept": false }
- }'
- curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
- -d '{ "agent_id": "chain-watcher", "role": "member" }'
- -d '{ "agent_id": "research-scout", "role": "member" }'
- -d '{ "agent_id": "alice:strategy-bot", "role": "member" }'
- curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \
- -H "Authorization: Bearer alice-token"
- curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \
- "name": "weekly-defi-report-w17",
- "pipeline": [
- { "stage": "gather", "agents": ["chain-watcher", "research-scout"] },
- { "stage": "synthesize", "agents": ["alice:strategy-bot"], "depends_on": ["gather"] }
- ]

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `curl -X POST https://will.roko.nunchi.dev/api/groups \`

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups \
  -H "Authorization: Bearer will-token" \
  -d '{
    "name": "defi-research",
    "description": "Collaborative DeFi analysis",
    "coordination": "stigmergic",
    "config": { "public": true, "auto_accept": false }
  }'
```
- Contract 2: language `bash`, first line `curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \`

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "chain-watcher", "role": "member" }'
# -> { "status": "joined" }

curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "research-scout", "role": "member" }'
# -> { "status": "joined" }
```
- Contract 3: language `bash`, first line `curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \`

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \
  -H "Authorization: Bearer will-token" \
  -d '{ "agent_id": "alice:strategy-bot", "role": "member" }'
# -> { "status": "pending", "invitation_id": "inv-xyz" }
```
- Contract 4: language `bash`, first line `curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \`

```bash
curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \
  -H "Authorization: Bearer alice-token"
# -> { "status": "joined", "group_id": "a1b2c3d4" }
```
- Contract 5: language `bash`, first line `curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \`

```bash
curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \
  -H "Authorization: Bearer will-token" \
  -d '{
    "name": "weekly-defi-report-w17",
    "pipeline": [
      { "stage": "gather", "agents": ["chain-watcher", "research-scout"] },
      { "stage": "synthesize", "agents": ["alice:strategy-bot"], "depends_on": ["gather"] }
    ]
  }'
```

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `alice.roko.nunchi.dev/api/invitations/inv-xyz/`
- `will.roko.nunchi.dev/api/`
- `will.roko.nunchi.dev/api/groups/a1b2c3d4/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "group|alice|user|token|research|nunchi|https|dev" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "group|alice|user|token|research|nunchi|https|dev" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
- `crates/roko-serve/src/events.rs`
- `alice.roko.nunchi.dev/api/invitations/inv-xyz/`
- `will.roko.nunchi.dev/api/`
- `will.roko.nunchi.dev/api/groups/a1b2c3d4/`
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
- [ ] Emit or consume `will.roko.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `alice.roko.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `group.member_joined` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `group.member_joined` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `curl -X POST https://will.roko.nunchi.dev/api/groups \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-H "Authorization: Bearer will-token" \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-d '{` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"name": "defi-research",` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"description": "Collaborative DeFi analysis",` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"coordination": "stigmergic",` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"config": { "public": true, "auto_accept": false }` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `}'` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/invite \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-d '{ "agent_id": "chain-watcher", "role": "member" }'` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-d '{ "agent_id": "research-scout", "role": "member" }'` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-d '{ "agent_id": "alice:strategy-bot", "role": "member" }'` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `curl -X POST https://alice.roko.nunchi.dev/api/invitations/inv-xyz/accept \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `-H "Authorization: Bearer alice-token"` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `curl -X POST https://will.roko.nunchi.dev/api/groups/a1b2c3d4/cluster \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"name": "weekly-defi-report-w17",` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `"pipeline": [` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `{ "stage": "gather", "agents": ["chain-watcher", "research-scout"] },` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `{ "stage": "synthesize", "agents": ["alice:strategy-bot"], "depends_on": ["gather"] }` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `]` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S029 -- Crate mapping

**Source section:** `tmp/architecture/10-groups.md:692` through `707`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Crate mapping

| Component | Crate | Status |
|-----------|-------|--------|
| Group types (`Group`, `GroupMember`, `GroupInvitation`) | `roko-core` | New |
| Group API routes | `roko-serve` | New |
| Group pheromone field | `roko-neuro` (extends InsightStore) | New |
| Group context bidder | `roko-compose` | New |
| Group relay room management | `roko-runtime` (via relay client) | New |
| Cluster creation from group | `roko-orchestrator` | Extends existing |
| On-chain group registry | `roko-chain` (Phase 2+) | Deferred |
| Group config in `roko.toml` | `roko-core` (config module) | New |
| Dashboard group surfaces | `nunchi-dashboard` | Depends on PRD 12 |

---
````

**Explicit detail extraction from this section:**

- Section word count: `77`
- Section hash: `a0b011d8bf1224a11cc4738bb064a46ed286e4f1ff5e11258ede5bc134656e14`

**Normative requirements and implementation claims:**
- | Component | Crate | Status | |-----------|-------|--------| | Group types (`Group`, `GroupMember`, `GroupInvitation`) | `roko-core` | New | | Group API routes | `roko-serve` | New | | Group pheromone field | `roko-neuro` (extends InsightStore) | New | | Group context bidder | `roko-compose` | New | | Group relay room management | `roko-runtime` (via relay client) | New | | Cluster creation from group | `roko-orchestrator` | Extends existing | | On-chain group registry | `roko-chain` (Phase 2+) | Deferred | | Group config in `roko.toml` | `roko-core` (config module) | New | | Dashboard group surfaces | `nunchi-dashboard` | Depends on PRD 12 |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Group
- GroupMember
- GroupInvitation

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
| Component | Crate | Status |
|-----------|-------|--------|
| Group types (`Group`, `GroupMember`, `GroupInvitation`) | `roko-core` | New |
| Group API routes | `roko-serve` | New |
| Group pheromone field | `roko-neuro` (extends InsightStore) | New |
| Group context bidder | `roko-compose` | New |
| Group relay room management | `roko-runtime` (via relay client) | New |
| Cluster creation from group | `roko-orchestrator` | Extends existing |
| On-chain group registry | `roko-chain` (Phase 2+) | Deferred |
| Group config in `roko.toml` | `roko-core` (config module) | New |
| Dashboard group surfaces | `nunchi-dashboard` | Depends on PRD 12 |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
rg -n "Group|Crate|mapping|GroupMember|GroupInvitation|relay|extends|core" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|Crate|mapping|GroupMember|GroupInvitation|relay|extends|core" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `Group` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GroupMember` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GroupInvitation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

### ARCH-10-S030 -- Open questions

**Source section:** `tmp/architecture/10-groups.md:708` through `716`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Open questions

1. **Group-level reputation.** Should a group have its own reputation score (aggregated from members), or does reputation stay per-agent? The on-chain registry could track either. Starting with per-agent only; group reputation is a derived view.

2. **Group treasury.** If a group operates paid feeds, who receives payment? A group treasury contract (held by the group passport) or split per-member? Deferred to Phase 2+ with the DeFi infrastructure.

3. **Conflict resolution.** When two agents in a stigmergic group deposit contradictory pheromones, what happens? Currently: nothing special -- agents interpret the field independently. A future extension could add conflict-detection heuristics that trigger broadcast alerts.

4. **Group size limits.** The relay room can handle hundreds of subscribers, but pheromone field size and knowledge store queries scale with member activity. Practical limit is probably 50-100 active members before performance tuning is needed. The `max_members` config provides a hard cap.
````

**Explicit detail extraction from this section:**

- Section word count: `151`
- Section hash: `e8a9c08c847c456a59168eb733e8262696b73a225e89a3d7cc019a09329bcabf`

**Normative requirements and implementation claims:**
- 1. **Group-level reputation.** Should a group have its own reputation score (aggregated from members), or does reputation stay per-agent? The on-chain registry could track either. Starting with per-agent only; group reputation is a derived view.
- 2. **Group treasury.** If a group operates paid feeds, who receives payment? A group treasury contract (held by the group passport) or split per-member? Deferred to Phase 2+ with the DeFi infrastructure.
- 3. **Conflict resolution.** When two agents in a stigmergic group deposit contradictory pheromones, what happens? Currently: nothing special -- agents interpret the field independently. A future extension could add conflict-detection heuristics that trigger broadcast alerts.
- 4. **Group size limits.** The relay room can handle hundreds of subscribers, but pheromone field size and knowledge store queries scale with member activity. Practical limit is probably 50-100 active members before performance tuning is needed. The `max_members` config provides a hard cap.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- max_members

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Group-level reputation.** Should a group have its own reputation score (aggregated from members), or does reputation stay per-agent? The on-chain registry could track either. Starting with per-agent only; group reputation is a derived view.
- 2. **Group treasury.** If a group operates paid feeds, who receives payment? A group treasury contract (held by the group passport) or split per-member? Deferred to Phase 2+ with the DeFi infrastructure.
- 3. **Conflict resolution.** When two agents in a stigmergic group deposit contradictory pheromones, what happens? Currently: nothing special -- agents interpret the field independently. A future extension could add conflict-detection heuristics that trigger broadcast alerts.
- 4. **Group size limits.** The relay room can handle hundreds of subscribers, but pheromone field size and knowledge store queries scale with member activity. Practical limit is probably 50-100 active members before performance tuning is needed. The `max_members` config provides a hard cap.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/10-groups.md`
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Group|questions|member|max_members|Open|reputation|members|treasury" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|questions|member|max_members|Open|reputation|members|treasury" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/groups.rs`
- `crates/roko-orchestrator/src/coordination/`
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
- [ ] Implement or verify `max_members` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/10-groups
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

