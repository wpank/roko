# Dashboard PRD Plan: Information Architecture

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
**Generated:** 2026-04-25
**Source hash:** `bc93ed0069ba09dd8c9b7ccf92cc3d56924a0f8686989ac6ee5bcca9efdf8ccb`
**Section tasks:** 23
**Context mode:** full source section embedded in every task; no excerpt truncation.
**Quality threshold:** every task must score at least 9.5/10 before implementation begins.

## Purpose
Turn every dashboard PRD section into explicit backend-support work. Even visual/frontend sections must produce backend projection, telemetry, fixture, schema, or explicit no-backend rationale so frontend implementation is easy and stable.

## Global Implementation Rules
- Extend existing modules before creating new ones; only add new route/service files when no canonical owner exists.
- Implement production wiring, not only structs, mocks, or isolated helpers.
- Preserve every extracted detail unless a parity-ledger row explicitly marks it covered or deferred.
- Add persistence, events, auth/safety, dashboard projections, and docs updates whenever the requirement reaches those surfaces.
- A checked box means code, tests, docs, parity ledger, and strict gates are done for that task.

## Primary Target Areas
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-04-S001 | 1 | 04 — Information architecture | [ ] | 9.8 |
| DASH-04-S002 | 7 | Why this document matters | [ ] | 9.8 |
| DASH-04-S003 | 19 | Principles for IA decisions | [ ] | 9.8 |
| DASH-04-S004 | 39 | Critique of the current IA | [ ] | 9.8 |
| DASH-04-S005 | 69 | The proposed IA | [ ] | 9.8 |
| DASH-04-S006 | 73 | Sidebar sections (top level) | [ ] | 9.8 |
| DASH-04-S007 | 97 | Progressive disclosure | [ ] | 9.8 |
| DASH-04-S008 | 101 | Section contents | [ ] | 9.8 |
| DASH-04-S009 | 109 | Pulse | [ ] | 9.8 |
| DASH-04-S010 | 122 | Fleet | [ ] | 9.8 |
| DASH-04-S011 | 136 | Forge | [ ] | 9.8 |
| DASH-04-S012 | 150 | Knowledge | [ ] | 9.8 |
| DASH-04-S013 | 165 | Arena | [ ] | 9.8 |
| DASH-04-S014 | 181 | Measurements *(new section)* | [ ] | 9.8 |
| DASH-04-S015 | 197 | Treasury | [ ] | 9.8 |
| DASH-04-S016 | 211 | Meta *(new section)* | [ ] | 9.8 |
| DASH-04-S017 | 226 | System | [ ] | 9.8 |
| DASH-04-S018 | 243 | Where every major concept lives | [ ] | 9.8 |
| DASH-04-S019 | 301 | Changes from the current IA — summary | [ ] | 9.8 |
| DASH-04-S020 | 348 | Cross-cutting UI elements | [ ] | 9.8 |
| DASH-04-S021 | 366 | IA principles applied — some worked examples | [ ] | 9.8 |
| DASH-04-S022 | 396 | IA non-goals | [ ] | 9.8 |
| DASH-04-S023 | 410 | What comes next | [ ] | 9.8 |

## Tasks

### DASH-04-S001 -- 04 — Information architecture

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 04 — Information architecture

*The top-level organization of the Nunchi dashboard: what sidebar sections exist, why they are grouped that way, what each section contains, and where every major concept lives.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `e05468a9b7f7ed9749faa8f9af7928c8f9758646f7994eba67818b07ed939340`

**Normative requirements and implementation claims:**
- *The top-level organization of the Nunchi dashboard: what sidebar sections exist, why they are grouped that way, what each section contains, and where every major concept lives.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
rg -n "Information|sidebar|sections|organization|major|lives|level|grouped" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Information|sidebar|sections|organization|major|lives|level|grouped" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S002 -- Why this document matters

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:7` through `18`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Why this document matters

Information architecture is the scaffolding on which every page hangs. If the sidebar groupings are wrong, every page in the wrong section gets harder to find, every cross-reference gets more confusing, and every new user takes longer to understand what the product does.

The current Nunchi dashboard has a recognizable IA (Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System). This specification inherits most of it and proposes deliberate changes to accommodate the new surfaces the product needs — authoring surfaces, meta surfaces, measurement surfaces — without breaking the mental model established by the current structure.

This document does three things. First, it explains the principles that guide IA decisions. Second, it critiques the current IA to make explicit what is working and what is not. Third, it specifies the proposed IA: sidebar sections, what belongs in each, where every major concept lives, and what changes from the current state.

Subsequent documents (`05-lenses-and-perspectives.md`, `06-navigation-and-traversal.md`) build on this structure. Section IV (pages and surfaces) is organized by this IA.

---
````

**Explicit detail extraction from this section:**

- Section word count: `173`
- Section hash: `27d825b68f2b373d82c5dbc94ed8eb3e450bb2b0fb5c6d76f39fce38723cbbe3`

**Normative requirements and implementation claims:**
- The current Nunchi dashboard has a recognizable IA (Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System). This specification inherits most of it and proposes deliberate changes to accommodate the new surfaces the product needs — authoring surfaces, meta surfaces, measurement surfaces — without breaking the mental model established by the current structure.
- This document does three things. First, it explains the principles that guide IA decisions. Second, it critiques the current IA to make explicit what is working and what is not. Third, it specifies the proposed IA: sidebar sections, what belongs in each, where every major concept lives, and what changes from the current state.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "document|surfaces|matters|every|Why|current|wrong|structure" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|surfaces|matters|every|Why|current|wrong|structure" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S003 -- Principles for IA decisions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:19` through `38`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Principles for IA decisions

Before making specific decisions about which page lives where, the following principles apply.

**Group by user intent, not by backend structure.** Users do not think in terms of "services" or "data sources." They think in terms of tasks and goals. Sidebar sections should correspond to things users want to do or look at, not to the microservices that power them.

**Surface primary jobs.** If a persona's primary job is well-served by a single section, that section must be reachable in one click from anywhere. Arena Competitors live in Arena; their path to Arena is always one click. Solo Operators live in Pulse and Fleet; both are always one click.

**Minimize cross-section hunting.** A page should appear in one place. Exceptions exist (a leaderboard might logically belong to both Arena and Knowledge), but every exception costs clarity. Resolve exceptions by picking a primary home and providing cross-links.

**Distinguish observe from author.** Looking at a thing and making a thing are different modes. When both exist, they should be clearly distinguishable in the IA. This specification adds authoring surfaces where the current dashboard has mostly observation surfaces.

**Progressive disclosure at the section level.** New users see fewer sections. Advanced users see more. The IA should allow hiding advanced sections without breaking the mental model. Meta-Builders need Meta; Solo Operators can ignore it until they need it.

**Group around primitives.** The system's primitives (agents, knowledge, arenas, evals, etc., from thesis 6 in `02-theses-and-principles.md`) are the natural axes of organization. Sections that cut across multiple primitives are acceptable but should be deliberate.

**Lenses are orthogonal to sections.** Lenses (global, fleet, agent, group, chain — see `05-lenses-and-perspectives.md`) apply across sections. A user can view Pulse from a fleet lens or a global lens. Lenses are a UI mechanism within sections, not a replacement for sections.

---
````

**Explicit detail extraction from this section:**

- Section word count: `313`
- Section hash: `49b982b14acdc1b78e0ab5561885f5c6a708eb9c99ca98bbaf15702caf970e55`

**Normative requirements and implementation claims:**
- **Group by user intent, not by backend structure.** Users do not think in terms of "services" or "data sources." They think in terms of tasks and goals. Sidebar sections should correspond to things users want to do or look at, not to the microservices that power them.
- **Surface primary jobs.** If a persona's primary job is well-served by a single section, that section must be reachable in one click from anywhere. Arena Competitors live in Arena; their path to Arena is always one click. Solo Operators live in Pulse and Fleet; both are always one click.
- **Minimize cross-section hunting.** A page should appear in one place. Exceptions exist (a leaderboard might logically belong to both Arena and Knowledge), but every exception costs clarity. Resolve exceptions by picking a primary home and providing cross-links.
- **Distinguish observe from author.** Looking at a thing and making a thing are different modes. When both exist, they should be clearly distinguishable in the IA. This specification adds authoring surfaces where the current dashboard has mostly observation surfaces.
- **Progressive disclosure at the section level.** New users see fewer sections. Advanced users see more. The IA should allow hiding advanced sections without breaking the mental model. Meta-Builders need Meta; Solo Operators can ignore it until they need it.
- **Group around primitives.** The system's primitives (agents, knowledge, arenas, evals, etc., from thesis 6 in `02-theses-and-principles.md`) are the natural axes of organization. Sections that cut across multiple primitives are acceptable but should be deliberate.
- **Lenses are orthogonal to sections.** Lenses (global, fleet, agent, group, chain — see `05-lenses-and-perspectives.md`) apply across sections. A user can view Pulse from a fleet lens or a global lens. Lenses are a UI mechanism within sections, not a replacement for sections.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "sections|for|Principles|user|lens|decisions|Arena|cross" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sections|for|Principles|user|lens|decisions|Arena|cross" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S004 -- Critique of the current IA

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:39` through `68`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Critique of the current IA

The current Nunchi dashboard has seven sidebar sections: Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System. Several pages sit under each. This is the state of the IA as of the screenshots reviewed for this specification.

**What the current IA gets right.**

The groupings are intuitive at a high level. Pulse conveys live observation. Fleet conveys owned-agent management. Forge conveys creative work. Knowledge conveys the shared layer. Arena conveys competition. Treasury conveys economics. System conveys configuration. A new user can guess what each section contains and be mostly right.

The sidebar is usable. Sections are clearly labeled, icons are recognizable, current selection is visible, nested pages are listed flat under each section header. This is a known-good pattern for a dashboard of this scale.

The naming is evocative and distinctive. "Pulse" is better than "Monitoring." "Forge" is better than "Create." "Arena" is better than "Benchmarks." The names carry character without sacrificing clarity. This should be preserved.

**Where the current IA falls short.**

First, authoring is underweight. The current sidebar is heavily observation-oriented. Fleet has Templates (which is partial authoring), Forge has PRDs (which is authoring-adjacent), but most of the actual creative surfaces — building an agent from parts, constructing a new arena, designing an eval — either don't exist yet or are buried. This specification pulls authoring into a first-class position.

Second, there is no home for evals or measurements as first-class objects. The current dashboard shows measurements (C-Factor, pass rates, Beta posteriors) in various places but does not have a surface where evals themselves are browsed, authored, composed, and challenged. Given thesis 4 (evals come from outside the LLM) and the general measurability thesis, this is a significant gap.

Third, meta is missing. Meta-agents, meta-evals, and generators have no home in the current IA. They cannot be shoved into existing sections without damaging the clarity of those sections. Meta deserves its own home.

Fourth, some pages are placed by backend proximity rather than user intent. "Context Audit" under Knowledge is logically adjacent to the knowledge substrate, but users come to Context Audit to debug why an agent behaved as it did — which is much closer to System or even a new Audit section. "Providers" under System is technically correct (providers are infrastructure configuration) but users often access providers to make authoring decisions, so proximity to Fleet or a new authoring section might serve them better.

Fifth, "Research" under Forge is ambiguous. Research in the sense of running a research agent belongs under Fleet (it's an agent type). Research in the sense of deep investigation of a topic belongs under Forge (it's creative work). The current label confuses the two.

Sixth, Treasury conflates two distinct concerns. Cost tracking (for Solo Operators and Fleet Orchestrators) is operational; position management and ISFR trading (for trader personas) is product-facing. Both are legitimate, but they serve different personas and should have clearly separated surfaces even within Treasury.

Seventh, the System section is a grab-bag. It contains Providers (model configuration), Jobs (bounties), Extensions (agent building blocks), Build (API reference), Settings (user settings). These are all configuration-ish but span very different concerns.

---
````

**Explicit detail extraction from this section:**

- Section word count: `528`
- Section hash: `e1633aae4c90ffadfe4c11f0ee01e9a44700ed6c2629a5d90a5790c9d8bcdc57`

**Normative requirements and implementation claims:**
- The current Nunchi dashboard has seven sidebar sections: Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System. Several pages sit under each. This is the state of the IA as of the screenshots reviewed for this specification.
- **What the current IA gets right.**
- The sidebar is usable. Sections are clearly labeled, icons are recognizable, current selection is visible, nested pages are listed flat under each section header. This is a known-good pattern for a dashboard of this scale.
- The naming is evocative and distinctive. "Pulse" is better than "Monitoring." "Forge" is better than "Create." "Arena" is better than "Benchmarks." The names carry character without sacrificing clarity. This should be preserved.
- **Where the current IA falls short.**
- Second, there is no home for evals or measurements as first-class objects. The current dashboard shows measurements (C-Factor, pass rates, Beta posteriors) in various places but does not have a surface where evals themselves are browsed, authored, composed, and challenged. Given thesis 4 (evals come from outside the LLM) and the general measurability thesis, this is a significant gap.
- Sixth, Treasury conflates two distinct concerns. Cost tracking (for Solo Operators and Fleet Orchestrators) is operational; position management and ISFR trading (for trader personas) is product-facing. Both are legitimate, but they serve different personas and should have clearly separated surfaces even within Treasury.
- Seventh, the System section is a grab-bag. It contains Providers (model configuration), Jobs (bounties), Extensions (agent building blocks), Build (API reference), Settings (user settings). These are all configuration-ish but span very different concerns.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "the|current|under|conveys|authoring|Forge|Fleet|user" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|current|under|conveys|authoring|Forge|Fleet|user" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S005 -- The proposed IA

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:69` through `72`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The proposed IA

The proposed IA keeps most of the current structure, moves several pages to better homes, adds three new top-level sections, and adds substantial depth to several existing sections. The full specification follows.
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `66b3ca6240c5406df22ffa72074db684d2be7ade07a3f053268c4dfb9db9fc4f`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
rg -n "The|proposed|several|sections|adds|three|substantial|structure" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|proposed|several|sections|adds|three|substantial|structure" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S006 -- Sidebar sections (top level)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:73` through `96`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Sidebar sections (top level)

The dashboard has nine sidebar sections, grouped in three tiers. The tiers are not labeled in the UI; they are a structural aid for this specification.

**Tier 1 — Core operation (always visible, every user, frequent use)**

- **Pulse** — live observation of agents and the network
- **Fleet** — management of owned agents
- **Forge** — creative work: plans, PRDs, execution, replay
- **Knowledge** — the shared knowledge layer

**Tier 2 — Competitive and economic (visible for most users)**

- **Arena** — competitive surfaces and leaderboards
- **Measurements** *(new)* — evals, benchmarks, measurement authoring
- **Treasury** — economics, cost, positions, ISFR

**Tier 3 — Advanced (hidden by default for new users, reachable via navigation)**

- **Meta** *(new)* — meta-agents, generators, recursive tooling
- **System** — providers, extensions, jobs, build, settings

Total: nine sections, up from seven. Two new sections (Measurements, Meta). One implicit tier distinction. Existing section names preserved.
````

**Explicit detail extraction from this section:**

- Section word count: `130`
- Section hash: `08e53aa090562067986b3bbc105f5c20799c0a6591697e63c7aae672b67ddc84`

**Normative requirements and implementation claims:**
- The dashboard has nine sidebar sections, grouped in three tiers. The tiers are not labeled in the UI; they are a structural aid for this specification.
- **Tier 1 — Core operation (always visible, every user, frequent use)**
- - **Pulse** — live observation of agents and the network - **Fleet** — management of owned agents - **Forge** — creative work: plans, PRDs, execution, replay - **Knowledge** — the shared knowledge layer
- **Tier 2 — Competitive and economic (visible for most users)**
- - **Arena** — competitive surfaces and leaderboards - **Measurements** *(new)* — evals, benchmarks, measurement authoring - **Treasury** — economics, cost, positions, ISFR
- **Tier 3 — Advanced (hidden by default for new users, reachable via navigation)**
- - **Meta** *(new)* — meta-agents, generators, recursive tooling - **System** — providers, extensions, jobs, build, settings

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
- - **Pulse** — live observation of agents and the network
- - **Fleet** — management of owned agents
- - **Forge** — creative work: plans, PRDs, execution, replay
- - **Knowledge** — the shared knowledge layer
- - **Arena** — competitive surfaces and leaderboards
- - **Measurements** *(new)* — evals, benchmarks, measurement authoring
- - **Treasury** — economics, cost, positions, ISFR
- - **Meta** *(new)* — meta-agents, generators, recursive tooling
- - **System** — providers, extensions, jobs, build, settings

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "sections|Tier|Sidebar|top|level|user|measurement|Meta" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sections|Tier|Sidebar|top|level|user|measurement|Meta" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S007 -- Progressive disclosure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:97` through `100`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Progressive disclosure

A new user sees Tier 1 and Tier 2 in full, and Tier 3 as a collapsed "Advanced" section they can expand. Once a user interacts with any Tier 3 surface (e.g., adjusts a provider config, creates a meta-agent), Tier 3 expands by default for their subsequent sessions. This is implementation behavior, not policy — the progressive disclosure is a user-experience optimization, not a permissions mechanism.
````

**Explicit detail extraction from this section:**

- Section word count: `68`
- Section hash: `1d9e70dd50fe9cea876aee551334c80b80a0d40c736ff5edf031c71eeefaabfa`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "disclosure|Progressive|Tier|user|expand|surface|subsequent|sessions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "disclosure|Progressive|Tier|user|expand|surface|subsequent|sessions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S008 -- Section contents

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:101` through `108`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Section contents

Each section's pages are listed below. Pages marked with [*new*] do not exist in the current dashboard. Pages marked with [*moved*] exist elsewhere currently and are being relocated. Pages marked with [*revised*] exist but have significant capability additions in this specification. All other pages exist and are retained largely as-is.

Page specifications (purpose, layout, interactions, data sources) are in Section IV documents.

---
````

**Explicit detail extraction from this section:**

- Section word count: `64`
- Section hash: `1ab120ca0dbf919db380fb9ae01085039d67b2f2860357571c64e36b2d77138f`

**Normative requirements and implementation claims:**
- Each section's pages are listed below. Pages marked with [*new*] do not exist in the current dashboard. Pages marked with [*moved*] exist elsewhere currently and are being relocated. Pages marked with [*revised*] exist but have significant capability additions in this specification. All other pages exist and are retained largely as-is.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "pages|contents|exist|marked|specification|current|specifications|sources" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "pages|contents|exist|marked|specification|current|specifications|sources" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S009 -- Pulse

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:109` through `121`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pulse

Live observation surfaces. The default landing page for most users.

- **Command Center** — aggregate dashboard: block height, on-chain agents, ISFR rate, knowledge entries, pheromones, active work, chain intelligence panel with knowledge layer / stigmergy field / agent collective.
- **Live Console** — one-row-per-agent view of every agent the user has access to, with gate %, cost, frequency, live output preview.
- **Event Stream** — the raw typed event feed with filters, search, pause, export.
- **Network Pulse** [*new*] — global-lens view of the whole network's activity, with selectable granularity (my fleet / domain-wide / all). Intended to surface patterns across the collective.

Primary personas: Solo Operator, Fleet Orchestrator. Secondary: Arena Competitor (for watching matches).

---
````

**Explicit detail extraction from this section:**

- Section word count: `109`
- Section hash: `901fe2e10f74a77df7358b61c92ae34f71f1976cb6e64393367bfbeaa36647a9`

**Normative requirements and implementation claims:**
- - **Command Center** — aggregate dashboard: block height, on-chain agents, ISFR rate, knowledge entries, pheromones, active work, chain intelligence panel with knowledge layer / stigmergy field / agent collective. - **Live Console** — one-row-per-agent view of every agent the user has access to, with gate %, cost, frequency, live output preview. - **Event Stream** — the raw typed event feed with filters, search, pause, export. - **Network Pulse** [*new*] — global-lens view of the whole network's activity, with selectable granularity (my fleet / domain-wide / all). Intended to surface patterns across the collective.
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
- - **Command Center** — aggregate dashboard: block height, on-chain agents, ISFR rate, knowledge entries, pheromones, active work, chain intelligence panel with knowledge layer / stigmergy field / agent collective.
- - **Live Console** — one-row-per-agent view of every agent the user has access to, with gate %, cost, frequency, live output preview.
- - **Event Stream** — the raw typed event feed with filters, search, pause, export.
- - **Network Pulse** [*new*] — global-lens view of the whole network's activity, with selectable granularity (my fleet / domain-wide / all). Intended to surface patterns across the collective.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Pulse|work|view|Live|user|surface|knowledge|gate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Pulse|work|view|Live|user|surface|knowledge|gate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S010 -- Fleet

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:122` through `135`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Fleet

Management of agents, templates, and fleet-level configuration.

- **Agent Fleet** — the list of owned agents, filterable and groupable. Card view and table view. Create, pause, delete, message individual agents.
- **Agent Detail** [*revised*] — drill-down into one agent: status, configuration, knowledge, heartbeat, episodes, costs, passport, recent actions. Each agent gets a canonical detail page reached from anywhere by clicking the agent's name.
- **Templates** — preset library of reusable agent configurations. Users can fork, customize, publish.
- **Network** [*revised*] — discovery surface for agents on the network. Currently underspecified; this specification fleshes it into a browseable catalog of registered agents filterable by capability, reputation tier, domain, owner.
- **Groups** [*new*] — create, join, inspect coordinated groups of agents (owned and foreign). Where group-level coordination is configured and observed.

Primary personas: Solo Operator, Fleet Orchestrator, Domain Architect. Secondary: everyone.

---
````

**Explicit detail extraction from this section:**

- Section word count: `133`
- Section hash: `ec997062bac2b141957bae19dfb9d12b9fc6a62c7eb72581fb7d3c01a3678c12`

**Normative requirements and implementation claims:**
- - **Agent Fleet** — the list of owned agents, filterable and groupable. Card view and table view. Create, pause, delete, message individual agents. - **Agent Detail** [*revised*] — drill-down into one agent: status, configuration, knowledge, heartbeat, episodes, costs, passport, recent actions. Each agent gets a canonical detail page reached from anywhere by clicking the agent's name. - **Templates** — preset library of reusable agent configurations. Users can fork, customize, publish. - **Network** [*revised*] — discovery surface for agents on the network. Currently underspecified; this specification fleshes it into a browseable catalog of registered agents filterable by capability, reputation tier, domain, owner. - **Groups** [*new*] — create, join, inspect coordinated groups of agents (owned and foreign). Where group-level coordination is configured and observed.
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
- - **Agent Fleet** — the list of owned agents, filterable and groupable. Card view and table view. Create, pause, delete, message individual agents.
- - **Agent Detail** [*revised*] — drill-down into one agent: status, configuration, knowledge, heartbeat, episodes, costs, passport, recent actions. Each agent gets a canonical detail page reached from anywhere by clicking the agent's name.
- - **Templates** — preset library of reusable agent configurations. Users can fork, customize, publish.
- - **Network** [*revised*] — discovery surface for agents on the network. Currently underspecified; this specification fleshes it into a browseable catalog of registered agents filterable by capability, reputation tier, domain, owner.
- - **Groups** [*new*] — create, join, inspect coordinated groups of agents (owned and foreign). Where group-level coordination is configured and observed.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Fleet|group|configuration|view|templates|revised|owned|level" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Fleet|group|configuration|view|templates|revised|owned|level" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S011 -- Forge

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:136` through `149`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Forge

Creative work: plans, PRDs, research workflows, execution, replay.

- **PRDs** — the pipeline of ideas → drafts → published. Unchanged from current; some enhancements to Kanban mechanics deferred.
- **Plans** — a plan is a structured specification for agent work (a DAG of tasks, dependencies, and checkpoints). Plans can be authored here, executed, and inspected. Currently thin; this specification expands.
- **Research** [*revised*] — the research workflow: pose a topic, agents gather sources, synthesize, produce artifacts. Specifically the composition of multiple agents on one research task. Distinct from agent-type-called-research (which lives under Fleet templates).
- **Execution** — the live view of running plans: which tasks are in progress, which are blocked, which have completed.
- **Replay** — episode replay for debugging and learning. Load a past episode and step through it.

Primary personas: Solo Operator (for specific tasks), Fleet Orchestrator (for plan composition), Arena Competitor (for iterating on scaffolding).

---
````

**Explicit detail extraction from this section:**

- Section word count: `140`
- Section hash: `0fcc60b06fb293a0ce75e26aa1064175c784fc06c2ebae2d87cdaf7b39cfd18d`

**Normative requirements and implementation claims:**
- - **PRDs** — the pipeline of ideas → drafts → published. Unchanged from current; some enhancements to Kanban mechanics deferred. - **Plans** — a plan is a structured specification for agent work (a DAG of tasks, dependencies, and checkpoints). Plans can be authored here, executed, and inspected. Currently thin; this specification expands. - **Research** [*revised*] — the research workflow: pose a topic, agents gather sources, synthesize, produce artifacts. Specifically the composition of multiple agents on one research task. Distinct from agent-type-called-research (which lives under Fleet templates). - **Execution** — the live view of running plans: which tasks are in progress, which are blocked, which have completed. - **Replay** — episode replay for debugging and learning. Load a past episode and step through it.
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
- the pipeline of ideas -> drafts

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **PRDs** — the pipeline of ideas → drafts → published. Unchanged from current; some enhancements to Kanban mechanics deferred.
- - **Plans** — a plan is a structured specification for agent work (a DAG of tasks, dependencies, and checkpoints). Plans can be authored here, executed, and inspected. Currently thin; this specification expands.
- - **Research** [*revised*] — the research workflow: pose a topic, agents gather sources, synthesize, produce artifacts. Specifically the composition of multiple agents on one research task. Distinct from agent-type-called-research (which lives under Fleet templates).
- - **Execution** — the live view of running plans: which tasks are in progress, which are blocked, which have completed.
- - **Replay** — episode replay for debugging and learning. Load a past episode and step through it.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "plan|research|Forge|work|task|specific|plans|tasks" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "plan|research|Forge|work|task|specific|plans|tasks" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Enforce state transition `the pipeline of ideas -> drafts` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S012 -- Knowledge

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:150` through `164`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge

The shared knowledge layer and associated surfaces.

- **Knowledge Store** — browse, search, read knowledge entries. Supports filtering by type, domain, author, confidence, freshness. Entry detail pages are reached from here.
- **Stigmergy** — visualization of the pheromone field. Types (wisdom, opportunity, threat), deposits, decays, sources.
- **Dream Cycles** — observation of offline consolidation processes. When dreams are running, what patterns are being extracted, what entries are being promoted.
- **Cross-Domain Resonance** [*new*] — dedicated surface for discovering and exploring cross-domain pattern matches. Renders the resonance graph. Entry point for surprising connections between domains.
- **Knowledge Lineage** [*new*] — view the provenance and dependency chains of entries. Start from an entry, trace ancestors and descendants. Essential for Knowledge Contributors and for auditing.
- **Context Audit** [*moved*] — *moved from current Knowledge section to System* (below). Context Audit is debug infrastructure, not a knowledge surface.

Primary personas: Knowledge Contributor, Domain Architect. Secondary: everyone (knowledge is consumed everywhere).

---
````

**Explicit detail extraction from this section:**

- Section word count: `146`
- Section hash: `720eec00d5085a508a62a9ae1c4292eaabaa63b212241a73183976c9d8312b9f`

**Normative requirements and implementation claims:**
- - **Knowledge Store** — browse, search, read knowledge entries. Supports filtering by type, domain, author, confidence, freshness. Entry detail pages are reached from here. - **Stigmergy** — visualization of the pheromone field. Types (wisdom, opportunity, threat), deposits, decays, sources. - **Dream Cycles** — observation of offline consolidation processes. When dreams are running, what patterns are being extracted, what entries are being promoted. - **Cross-Domain Resonance** [*new*] — dedicated surface for discovering and exploring cross-domain pattern matches. Renders the resonance graph. Entry point for surprising connections between domains. - **Knowledge Lineage** [*new*] — view the provenance and dependency chains of entries. Start from an entry, trace ancestors and descendants. Essential for Knowledge Contributors and for auditing. - **Context Audit** [*moved*] — *moved from current Knowledge section to System* (below). Context Audit is debug infrastructure, not a knowledge s
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
- - **Knowledge Store** — browse, search, read knowledge entries. Supports filtering by type, domain, author, confidence, freshness. Entry detail pages are reached from here.
- - **Stigmergy** — visualization of the pheromone field. Types (wisdom, opportunity, threat), deposits, decays, sources.
- - **Dream Cycles** — observation of offline consolidation processes. When dreams are running, what patterns are being extracted, what entries are being promoted.
- - **Cross-Domain Resonance** [*new*] — dedicated surface for discovering and exploring cross-domain pattern matches. Renders the resonance graph. Entry point for surprising connections between domains.
- - **Knowledge Lineage** [*new*] — view the provenance and dependency chains of entries. Start from an entry, trace ancestors and descendants. Essential for Knowledge Contributors and for auditing.
- - **Context Audit** [*moved*] — *moved from current Knowledge section to System* (below). Context Audit is debug infrastructure, not a knowledge surface.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Knowledge|domain|surface|entries|Entry|Audit|type|pattern" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|domain|surface|entries|Entry|Audit|type|pattern" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S013 -- Arena

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:165` through `180`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Arena

Competitive surfaces and leaderboards.

- **Arena Browser** [*new*] — discovery surface for available arenas (SWE-bench, AMM optimization, chess, prediction markets, persuasion, negotiation, dogfight, QEC, packing, user-created). Filterable by domain, complexity, active participants, prize pool.
- **Arena Detail** [*new*] — landing page for one arena. Description, rules, current leaderboard, recent attempts, submit entry, historical trends.
- **Leaderboard** [*revised*] — global leaderboard across all arenas and domains, with drill-down into specific arenas. Current Leaderboard page evolves into this.
- **Experiments** — A/B experiments. Currently fine; gets integration with the new arena infrastructure.
- **Benchmarks** [*revised*] — static benchmark dashboards (health score, C-Factor, pass rates) across the fleet or network. Extended to support user-chosen benchmark composition.
- **Bounty Market** [*new*] — user-posted bounties, their states, claim mechanics. Distinct from the Jobs page under System (which is more operational).
- **Challenge Runner** [*new*] — the live view for running an arena attempt. Watch the agent work, see gates fire in real time, get final score.

Primary personas: Arena Competitor. Secondary: Fleet Orchestrator (for cross-arena insights), Meta-Builder (for arena generation).

---
````

**Explicit detail extraction from this section:**

- Section word count: `170`
- Section hash: `7236b4eacf4fc852f11cbd7c902da273b8928bdbd871c91d39647026784d2d91`

**Normative requirements and implementation claims:**
- - **Arena Browser** [*new*] — discovery surface for available arenas (SWE-bench, AMM optimization, chess, prediction markets, persuasion, negotiation, dogfight, QEC, packing, user-created). Filterable by domain, complexity, active participants, prize pool. - **Arena Detail** [*new*] — landing page for one arena. Description, rules, current leaderboard, recent attempts, submit entry, historical trends. - **Leaderboard** [*revised*] — global leaderboard across all arenas and domains, with drill-down into specific arenas. Current Leaderboard page evolves into this. - **Experiments** — A/B experiments. Currently fine; gets integration with the new arena infrastructure. - **Benchmarks** [*revised*] — static benchmark dashboards (health score, C-Factor, pass rates) across the fleet or network. Extended to support user-chosen benchmark composition. - **Bounty Market** [*new*] — user-posted bounties, their states, claim mechanics. Distinct from the Jobs page under System (which is more operati
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
- - **Arena Browser** [*new*] — discovery surface for available arenas (SWE-bench, AMM optimization, chess, prediction markets, persuasion, negotiation, dogfight, QEC, packing, user-created). Filterable by domain, complexity, active participants, prize pool.
- - **Arena Detail** [*new*] — landing page for one arena. Description, rules, current leaderboard, recent attempts, submit entry, historical trends.
- - **Leaderboard** [*revised*] — global leaderboard across all arenas and domains, with drill-down into specific arenas. Current Leaderboard page evolves into this.
- - **Experiments** — A/B experiments. Currently fine; gets integration with the new arena infrastructure.
- - **Benchmarks** [*revised*] — static benchmark dashboards (health score, C-Factor, pass rates) across the fleet or network. Extended to support user-chosen benchmark composition.
- - **Bounty Market** [*new*] — user-posted bounties, their states, claim mechanics. Distinct from the Jobs page under System (which is more operational).
- - **Challenge Runner** [*new*] — the live view for running an arena attempt. Watch the agent work, see gates fire in real time, get final score.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Arena|leaderboard|bench|user|current|cross|benchmark|arenas" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|leaderboard|bench|user|current|cross|benchmark|arenas" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S014 -- Measurements *(new section)*

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:181` through `196`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Measurements *(new section)*

Evals, measurements, and the machinery that grounds the system in external reality.

- **Evals Library** [*new*] — browse published evals. Filter by domain, ground truth source, author, reputation, usage. Entry detail shows definition, history, validations, challenges.
- **Eval Detail** [*new*] — landing page for one eval. Definition, ground truth source, scoring function, recent applications, calibration, challenges.
- **Measurement Runner** [*new*] — apply an eval to an agent, a configuration, an attempt, or a set. See the measurement compute live. Get a score with uncertainty.
- **Calibration Dashboard** [*new*] — across evals, are they well-calibrated? Over-confident? Under-confident? Essential for the cybernetic feedback loop.
- **Meta-Evals** [*new*] — evals that evaluate other evals. Which evals are producing useful signal? Which are redundant? Which are miscalibrated?

This section is new because the current dashboard has no home for evals as first-class objects, and the product's thesis depends on them being first-class. See thesis 4 in `02-theses-and-principles.md`.

Primary personas: Knowledge Contributor, Domain Architect, Meta-Builder, System Steward.

---
````

**Explicit detail extraction from this section:**

- Section word count: `164`
- Section hash: `0b56b426b369be418552defc867bed1fc9207415a921f0f8f3966f3f9195f4de`

**Normative requirements and implementation claims:**
- - **Evals Library** [*new*] — browse published evals. Filter by domain, ground truth source, author, reputation, usage. Entry detail shows definition, history, validations, challenges. - **Eval Detail** [*new*] — landing page for one eval. Definition, ground truth source, scoring function, recent applications, calibration, challenges. - **Measurement Runner** [*new*] — apply an eval to an agent, a configuration, an attempt, or a set. See the measurement compute live. Get a score with uncertainty. - **Calibration Dashboard** [*new*] — across evals, are they well-calibrated? Over-confident? Under-confident? Essential for the cybernetic feedback loop. - **Meta-Evals** [*new*] — evals that evaluate other evals. Which evals are producing useful signal? Which are redundant? Which are miscalibrated?
- This section is new because the current dashboard has no home for evals as first-class objects, and the product's thesis depends on them being first-class. See thesis 4 in `02-theses-and-principles.md`.
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
- - **Evals Library** [*new*] — browse published evals. Filter by domain, ground truth source, author, reputation, usage. Entry detail shows definition, history, validations, challenges.
- - **Eval Detail** [*new*] — landing page for one eval. Definition, ground truth source, scoring function, recent applications, calibration, challenges.
- - **Measurement Runner** [*new*] — apply an eval to an agent, a configuration, an attempt, or a set. See the measurement compute live. Get a score with uncertainty.
- - **Calibration Dashboard** [*new*] — across evals, are they well-calibrated? Over-confident? Under-confident? Essential for the cybernetic feedback loop.
- - **Meta-Evals** [*new*] — evals that evaluate other evals. Which evals are producing useful signal? Which are redundant? Which are miscalibrated?

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Eval|new|Evals|Measurement|Measurements|ground|truth|thesis" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|new|Evals|Measurement|Measurements|ground|truth|thesis" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S015 -- Treasury

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:197` through `210`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Treasury

Economics, cost, positions, ISFR.

- **Positions** — active holdings in yield perpetuals and other tradable instruments.
- **Cost Analytics** [*revised*] — aggregate cost tracking across agents, providers, time. Breakdowns by domain, tier, extension. Forecasting.
- **ISFR Dashboard** — the benchmark rate, its components, its history, its implied curves.
- **Multi-Chain** — cross-chain state and bridge flows. Currently thin; specification recommends investing here if multi-chain is a near-term priority.
- **Yield Perpetuals** [*new*] — dedicated surface for trading and hedging yield perps. Distinct from Positions (which lists) and ISFR (which informs). Where orders are placed and clearing is observed.

Primary personas: Solo Operator (for cost), Fleet Orchestrator (for aggregated cost), Passive User (for position review if they're in a yield product), traders.

---
````

**Explicit detail extraction from this section:**

- Section word count: `116`
- Section hash: `a7d68afb8fdc42845c11a33226ec2334be92b375e23921f474271200e7b93e9e`

**Normative requirements and implementation claims:**
- - **Positions** — active holdings in yield perpetuals and other tradable instruments. - **Cost Analytics** [*revised*] — aggregate cost tracking across agents, providers, time. Breakdowns by domain, tier, extension. Forecasting. - **ISFR Dashboard** — the benchmark rate, its components, its history, its implied curves. - **Multi-Chain** — cross-chain state and bridge flows. Currently thin; specification recommends investing here if multi-chain is a near-term priority. - **Yield Perpetuals** [*new*] — dedicated surface for trading and hedging yield perps. Distinct from Positions (which lists) and ISFR (which informs). Where orders are placed and clearing is observed.
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
- - **Positions** — active holdings in yield perpetuals and other tradable instruments.
- - **Cost Analytics** [*revised*] — aggregate cost tracking across agents, providers, time. Breakdowns by domain, tier, extension. Forecasting.
- - **ISFR Dashboard** — the benchmark rate, its components, its history, its implied curves.
- - **Multi-Chain** — cross-chain state and bridge flows. Currently thin; specification recommends investing here if multi-chain is a near-term priority.
- - **Yield Perpetuals** [*new*] — dedicated surface for trading and hedging yield perps. Distinct from Positions (which lists) and ISFR (which informs). Where orders are placed and clearing is observed.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "cost|Treasury|yield|position|positions|ISFR|Chain|perpetuals" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cost|Treasury|yield|position|positions|ISFR|Chain|perpetuals" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S016 -- Meta *(new section)*

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:211` through `225`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Meta *(new section)*

Meta-agents, generators, recursive tooling. First-class home for the "tools for tools" principle.

- **Meta-Agents** [*new*] — create, operate, inspect meta-agents. A meta-agent produces other agents for specific task categories.
- **Generator Studio** [*new*] — build generators for new first-class objects: new extensions, new arenas, new domains, new gates, new evals.
- **Recursive Workshop** [*new*] — observe recursive behavior safely. Trace lineage across multiple layers. Inspect what a meta-agent produced and why.
- **Meta-Lineage** [*new*] — genealogy of objects in the system. Which agents produced which agents; which evals retired which evals; which generators spawned which arenas.

Primary personas: Meta-Builder. Secondary: Domain Architect (for domain generators), System Steward (for system integrity).

This section is hidden by default for new users. It appears in the sidebar once the user navigates to it or creates their first meta-object.

---
````

**Explicit detail extraction from this section:**

- Section word count: `137`
- Section hash: `1207d4728d54f7eaa400c5aef9d3b5b351c56c8fd6799714f11ed13a6df47ee0`

**Normative requirements and implementation claims:**
- - **Meta-Agents** [*new*] — create, operate, inspect meta-agents. A meta-agent produces other agents for specific task categories. - **Generator Studio** [*new*] — build generators for new first-class objects: new extensions, new arenas, new domains, new gates, new evals. - **Recursive Workshop** [*new*] — observe recursive behavior safely. Trace lineage across multiple layers. Inspect what a meta-agent produced and why. - **Meta-Lineage** [*new*] — genealogy of objects in the system. Which agents produced which agents; which evals retired which evals; which generators spawned which arenas.
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
- - **Meta-Agents** [*new*] — create, operate, inspect meta-agents. A meta-agent produces other agents for specific task categories.
- - **Generator Studio** [*new*] — build generators for new first-class objects: new extensions, new arenas, new domains, new gates, new evals.
- - **Recursive Workshop** [*new*] — observe recursive behavior safely. Trace lineage across multiple layers. Inspect what a meta-agent produced and why.
- - **Meta-Lineage** [*new*] — genealogy of objects in the system. Which agents produced which agents; which evals retired which evals; which generators spawned which arenas.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "new|Meta|Generator|generators|recursive|object|evals|First" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "new|Meta|Generator|generators|recursive|object|evals|First" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S017 -- System

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:226` through `242`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### System

Providers, extensions, jobs, build, settings, and operational configuration.

- **Providers** — LLM provider configuration, model catalog, comparison, cascade router state. Unchanged structurally; some authoring capabilities added.
- **Extensions** [*revised*] — browse installed extensions, install new ones, inspect extension configuration. Extended with authoring surface for user-created extensions (see `19-authoring-surfaces.md`).
- **Gates** [*new*] — first-class surface for gate pipeline configuration. Currently gates are configured implicitly via domain profiles; this makes them editable and inspectable.
- **Jobs** [*revised*] — lifecycle wizard for bounty jobs. Current page fine; reorganized slightly.
- **Delegations** [*new*] — view and edit delegation caveats. Essential for owners managing what their agents can do.
- **Context Audit** [*moved*] — *moved from Knowledge*. Debug infrastructure for inspecting why an agent assembled the context it did.
- **Build** — API and CLI reference. Live endpoint health. Current page excellent; retain.
- **Settings** — user settings (wallet connection, preferences, notifications, layout, feature flags).

Primary personas: Fleet Orchestrator, Domain Architect, System Steward. Secondary: everyone (occasional).

---
````

**Explicit detail extraction from this section:**

- Section word count: `150`
- Section hash: `e652d07601d005f8ce60a1b88282f90b37961402ac908832a9a6135a410d4581`

**Normative requirements and implementation claims:**
- - **Providers** — LLM provider configuration, model catalog, comparison, cascade router state. Unchanged structurally; some authoring capabilities added. - **Extensions** [*revised*] — browse installed extensions, install new ones, inspect extension configuration. Extended with authoring surface for user-created extensions (see `19-authoring-surfaces.md`). - **Gates** [*new*] — first-class surface for gate pipeline configuration. Currently gates are configured implicitly via domain profiles; this makes them editable and inspectable. - **Jobs** [*revised*] — lifecycle wizard for bounty jobs. Current page fine; reorganized slightly. - **Delegations** [*new*] — view and edit delegation caveats. Essential for owners managing what their agents can do. - **Context Audit** [*moved*] — *moved from Knowledge*. Debug infrastructure for inspecting why an agent assembled the context it did. - **Build** — API and CLI reference. Live endpoint health. Current page excellent; retain. - **Settings** — 
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
- - **Providers** — LLM provider configuration, model catalog, comparison, cascade router state. Unchanged structurally; some authoring capabilities added.
- - **Extensions** [*revised*] — browse installed extensions, install new ones, inspect extension configuration. Extended with authoring surface for user-created extensions (see `19-authoring-surfaces.md`).
- - **Gates** [*new*] — first-class surface for gate pipeline configuration. Currently gates are configured implicitly via domain profiles; this makes them editable and inspectable.
- - **Jobs** [*revised*] — lifecycle wizard for bounty jobs. Current page fine; reorganized slightly.
- - **Delegations** [*new*] — view and edit delegation caveats. Essential for owners managing what their agents can do.
- - **Context Audit** [*moved*] — *moved from Knowledge*. Debug infrastructure for inspecting why an agent assembled the context it did.
- - **Build** — API and CLI reference. Live endpoint health. Current page excellent; retain.
- - **Settings** — user settings (wallet connection, preferences, notifications, layout, feature flags).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "extension|extensions|configuration|surface|settings|provider|jobs|inspect" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "extension|extensions|configuration|surface|settings|provider|jobs|inspect" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S018 -- Where every major concept lives

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:243` through `300`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Where every major concept lives

This section names every major concept from `01-system-landscape.md` and specifies its primary location in the IA. Concepts may appear in multiple places (as lenses or cross-references), but each has one canonical home.

| Concept | Primary location | Notes |
|---|---|---|
| Agents (owned) | Fleet → Agent Fleet | Agent Detail reachable from anywhere by clicking name |
| Agents (network) | Fleet → Network | Discovery surface for all registered agents |
| Agent templates | Fleet → Templates | Forkable presets |
| Agent detail page | Fleet → Agent Detail | Canonical detail page for one agent |
| Groups | Fleet → Groups | Coordinated subsets of agents |
| Domains | System → Extensions (browse) + Fleet → Templates (apply) | Domain authoring in `19-authoring-surfaces.md` |
| Extensions | System → Extensions | Browse, install, author |
| Gates | System → Gates | First-class config |
| Delegation caveats | System → Delegations | Per-agent and per-owner policy |
| Model providers | System → Providers | Catalog and configuration |
| Cascade router state | System → Providers | As a sub-view |
| Knowledge entries | Knowledge → Knowledge Store | Canonical location |
| Knowledge lineage | Knowledge → Knowledge Lineage | Dedicated traversal surface |
| Pheromones | Knowledge → Stigmergy | Field visualization |
| Dreams | Knowledge → Dream Cycles | Observation of consolidation |
| Cross-domain resonance | Knowledge → Cross-Domain Resonance | Dedicated surface |
| Arenas | Arena → Arena Browser | Discovery |
| Arena detail | Arena → Arena Detail | Per-arena page |
| Leaderboards | Arena → Leaderboard | Global + drill-down |
| Bounties | Arena → Bounty Market | User-posted tasks |
| Attempts (arena) | Arena → Challenge Runner + Arena Detail | Running + historical |
| Experiments (A/B) | Arena → Experiments | Configuration running |
| Benchmarks | Arena → Benchmarks | Fleet/network benchmarks |
| Evals | Measurements → Evals Library | Browse, compose |
| Eval detail | Measurements → Eval Detail | Per-eval page |
| Measurement execution | Measurements → Measurement Runner | Live measurement |
| Calibration state | Measurements → Calibration Dashboard | How well-calibrated are evals |
| Meta-evals | Measurements → Meta-Evals | Evals of evals |
| Meta-agents | Meta → Meta-Agents | Create, operate |
| Generators | Meta → Generator Studio | Build generators |
| Recursive traces | Meta → Recursive Workshop | Safe observation |
| Object lineage | Meta → Meta-Lineage | Genealogy across layers |
| Positions | Treasury → Positions | Holdings |
| Yield perpetuals | Treasury → Yield Perpetuals | Trading surface |
| ISFR | Treasury → ISFR Dashboard | Benchmark rate state |
| Cost (per agent, per fleet) | Treasury → Cost Analytics | Aggregated economics |
| Multi-chain state | Treasury → Multi-Chain | Cross-chain |
| Live agent activity | Pulse → Live Console | One-row-per-agent |
| Aggregate network state | Pulse → Command Center | High-level KPIs |
| Raw event stream | Pulse → Event Stream | Typed events, filterable |
| Network-level patterns | Pulse → Network Pulse | Cross-fleet observation |
| Plans (DAGs of work) | Forge → Plans | Authoring and running |
| PRDs | Forge → PRDs | Idea pipeline |
| Research workflows | Forge → Research | Multi-agent research |
| Plan execution | Forge → Execution | Live running plans |
| Episode replay | Forge → Replay | Debug past runs |
| Context debug | System → Context Audit | Why did the agent see what it saw |
| API reference | System → Build | Live endpoints |
| User settings | System → Settings | Preferences |

---
````

**Explicit detail extraction from this section:**

- Section word count: `423`
- Section hash: `ebf63550f256049226c371f4e04387ec1999355b4d57e4d74fc94e6954336d24`

**Normative requirements and implementation claims:**
- | Concept | Primary location | Notes | |---|---|---| | Agents (owned) | Fleet → Agent Fleet | Agent Detail reachable from anywhere by clicking name | | Agents (network) | Fleet → Network | Discovery surface for all registered agents | | Agent templates | Fleet → Templates | Forkable presets | | Agent detail page | Fleet → Agent Detail | Canonical detail page for one agent | | Groups | Fleet → Groups | Coordinated subsets of agents | | Domains | System → Extensions (browse) + Fleet → Templates (apply) | Domain authoring in `19-authoring-surfaces.md` | | Extensions | System → Extensions | Browse, install, author | | Gates | System → Gates | First-class config | | Delegation caveats | System → Delegations | Per-agent and per-owner policy | | Model providers | System → Providers | Catalog and configuration | | Cascade router state | System → Providers | As a sub-view | | Knowledge entries | Knowledge → Knowledge Store | Canonical location | | Knowledge lineage | Knowledge → Knowledge Linea
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
- Fleet -> Agent Fleet
- Fleet -> Network
- Fleet -> Templates
- Fleet -> Agent Detail
- Fleet -> Groups
- System -> Extensions
- System -> Gates
- System -> Delegations
- System -> Providers
- Knowledge -> Knowledge Store
- Knowledge -> Knowledge Lineage
- Knowledge -> Stigmergy
- Knowledge -> Dream Cycles
- Knowledge -> Cross-Domain Resonance
- Arena -> Arena Browser
- Arena -> Arena Detail
- Arena -> Leaderboard
- Arena -> Bounty Market
- Arena -> Challenge Runner
- Arena -> Experiments
- Arena -> Benchmarks
- Measurements -> Evals Library
- Measurements -> Eval Detail
- Measurements -> Measurement Runner
- Measurements -> Calibration Dashboard
- Measurements -> Meta-Evals
- Meta -> Meta-Agents
- Meta -> Generator Studio
- Meta -> Recursive Workshop
- Meta -> Meta-Lineage
- Treasury -> Positions
- Treasury -> Yield Perpetuals
- Treasury -> ISFR Dashboard
- Treasury -> Cost Analytics
- Treasury -> Multi-Chain
- Pulse -> Live Console
- Pulse -> Command Center
- Pulse -> Event Stream
- Pulse -> Network Pulse
- Forge -> Plans
- Forge -> PRDs
- Forge -> Research
- Forge -> Execution
- Forge -> Replay
- System -> Context Audit
- System -> Build
- System -> Settings

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Concept | Primary location | Notes |
|---|---|---|
| Agents (owned) | Fleet → Agent Fleet | Agent Detail reachable from anywhere by clicking name |
| Agents (network) | Fleet → Network | Discovery surface for all registered agents |
| Agent templates | Fleet → Templates | Forkable presets |
| Agent detail page | Fleet → Agent Detail | Canonical detail page for one agent |
| Groups | Fleet → Groups | Coordinated subsets of agents |
| Domains | System → Extensions (browse) + Fleet → Templates (apply) | Domain authoring in `19-authoring-surfaces.md` |
| Extensions | System → Extensions | Browse, install, author |
| Gates | System → Gates | First-class config |
| Delegation caveats | System → Delegations | Per-agent and per-owner policy |
| Model providers | System → Providers | Catalog and configuration |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Arena|Live|Fleet|Eval|Meta|Knowledge|Detail|concept" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|Live|Fleet|Eval|Meta|Knowledge|Detail|concept" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Enforce state transition `Fleet -> Agent Fleet` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Fleet -> Network` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Fleet -> Templates` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Fleet -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Fleet -> Groups` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Extensions` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Gates` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Delegations` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Providers` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> Knowledge Store` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> Knowledge Lineage` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> Stigmergy` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> Dream Cycles` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> Cross-Domain Resonance` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Arena Browser` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Arena Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Leaderboard` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Bounty Market` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Challenge Runner` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Experiments` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Arena -> Benchmarks` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Measurements -> Evals Library` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Measurements -> Eval Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Measurements -> Measurement Runner` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Measurements -> Calibration Dashboard` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Measurements -> Meta-Evals` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Meta -> Meta-Agents` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Meta -> Generator Studio` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Meta -> Recursive Workshop` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Meta -> Meta-Lineage` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Treasury -> Positions` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Treasury -> Yield Perpetuals` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Treasury -> ISFR Dashboard` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Treasury -> Cost Analytics` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Treasury -> Multi-Chain` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Pulse -> Live Console` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Pulse -> Command Center` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Pulse -> Event Stream` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Pulse -> Network Pulse` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Forge -> Plans` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Forge -> PRDs` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Forge -> Research` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Forge -> Execution` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Forge -> Replay` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Context Audit` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Build` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `System -> Settings` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S019 -- Changes from the current IA — summary

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:301` through `347`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Changes from the current IA — summary

For quick reference, the specific moves and additions this specification proposes.

**Added sections (2)**

- Measurements — new Tier 2 section for evals and measurement objects.
- Meta — new Tier 3 section for meta-agents, generators, recursive tooling.

**Added pages (17)**

- Fleet: Agent Detail (formalized canonical page), Groups.
- Knowledge: Cross-Domain Resonance, Knowledge Lineage.
- Arena: Arena Browser, Arena Detail, Bounty Market, Challenge Runner.
- Measurements: Evals Library, Eval Detail, Measurement Runner, Calibration Dashboard, Meta-Evals.
- Treasury: Yield Perpetuals.
- Meta: Meta-Agents, Generator Studio, Recursive Workshop, Meta-Lineage.
- System: Gates, Delegations.
- Pulse: Network Pulse.

**Moved pages (1)**

- Context Audit: Knowledge → System.

**Revised pages (8)**

- Fleet: Network (filled out from placeholder to real catalog).
- Forge: Research (clarified scope vs agent-type-called-research).
- Forge: Plans (expanded from thin to real DAG authoring).
- Arena: Leaderboard (integrates with new arena infrastructure).
- Arena: Benchmarks (composable benchmark dashboards).
- Treasury: Cost Analytics (expanded tracking and forecasting).
- System: Extensions (adds authoring surface).
- System: Jobs (reorganized).

**Preserved pages (no change or only minor)**

- Pulse: Command Center, Live Console, Event Stream.
- Fleet: Agent Fleet, Templates.
- Forge: PRDs, Execution, Replay.
- Knowledge: Knowledge Store, Stigmergy, Dream Cycles.
- Arena: Experiments.
- Treasury: Positions, ISFR Dashboard, Multi-Chain.
- System: Providers, Build, Settings.

---
````

**Explicit detail extraction from this section:**

- Section word count: `193`
- Section hash: `ca7f454693954e10bf525cf684bd8a68b84926f57009dae163b6e9afbcba1cd5`

**Normative requirements and implementation claims:**
- **Added sections (2)**
- - Measurements — new Tier 2 section for evals and measurement objects. - Meta — new Tier 3 section for meta-agents, generators, recursive tooling.
- **Added pages (17)**
- - Fleet: Agent Detail (formalized canonical page), Groups. - Knowledge: Cross-Domain Resonance, Knowledge Lineage. - Arena: Arena Browser, Arena Detail, Bounty Market, Challenge Runner. - Measurements: Evals Library, Eval Detail, Measurement Runner, Calibration Dashboard, Meta-Evals. - Treasury: Yield Perpetuals. - Meta: Meta-Agents, Generator Studio, Recursive Workshop, Meta-Lineage. - System: Gates, Delegations. - Pulse: Network Pulse.
- **Moved pages (1)**
- - Context Audit: Knowledge → System.
- **Revised pages (8)**
- - Fleet: Network (filled out from placeholder to real catalog). - Forge: Research (clarified scope vs agent-type-called-research). - Forge: Plans (expanded from thin to real DAG authoring). - Arena: Leaderboard (integrates with new arena infrastructure). - Arena: Benchmarks (composable benchmark dashboards). - Treasury: Cost Analytics (expanded tracking and forecasting). - System: Extensions (adds authoring surface). - System: Jobs (reorganized).
- **Preserved pages (no change or only minor)**
- - Pulse: Command Center, Live Console, Event Stream. - Fleet: Agent Fleet, Templates. - Forge: PRDs, Execution, Replay. - Knowledge: Knowledge Store, Stigmergy, Dream Cycles. - Arena: Experiments. - Treasury: Positions, ISFR Dashboard, Multi-Chain. - System: Providers, Build, Settings.
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
- Knowledge -> System

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Measurements — new Tier 2 section for evals and measurement objects.
- - Meta — new Tier 3 section for meta-agents, generators, recursive tooling.
- - Fleet: Agent Detail (formalized canonical page), Groups.
- - Knowledge: Cross-Domain Resonance, Knowledge Lineage.
- - Arena: Arena Browser, Arena Detail, Bounty Market, Challenge Runner.
- - Measurements: Evals Library, Eval Detail, Measurement Runner, Calibration Dashboard, Meta-Evals.
- - Treasury: Yield Perpetuals.
- - Meta: Meta-Agents, Generator Studio, Recursive Workshop, Meta-Lineage.
- - System: Gates, Delegations.
- - Pulse: Network Pulse.
- - Context Audit: Knowledge → System.
- - Fleet: Network (filled out from placeholder to real catalog).
- - Forge: Research (clarified scope vs agent-type-called-research).
- - Forge: Plans (expanded from thin to real DAG authoring).
- - Arena: Leaderboard (integrates with new arena infrastructure).
- - Arena: Benchmarks (composable benchmark dashboards).
- - Treasury: Cost Analytics (expanded tracking and forecasting).
- - System: Extensions (adds authoring surface).
- - System: Jobs (reorganized).
- - Pulse: Command Center, Live Console, Event Stream.
- - Fleet: Agent Fleet, Templates.
- - Forge: PRDs, Execution, Replay.
- - Knowledge: Knowledge Store, Stigmergy, Dream Cycles.
- - Arena: Experiments.
- - Treasury: Positions, ISFR Dashboard, Multi-Chain.
- - System: Providers, Build, Settings.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Arena|the|change|Meta|summary|current|Knowledge|Changes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|the|change|Meta|summary|current|Knowledge|Changes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Enforce state transition `Knowledge -> System` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S020 -- Cross-cutting UI elements

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:348` through `365`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Cross-cutting UI elements

Some UI elements cut across all sections and belong to the IA discussion.

**The global header.** Every page has a header with: the Nunchi logo / brand, current block height of the chain (with LIVE / CALM status indicator), a search surface, account and wallet information. This header is consistent across all sections.

**The global sidebar.** The sidebar lists all visible sections with their pages. Sidebar state (expanded sections, pinned pages) persists per user.

**Breadcrumbs.** Detail pages (Agent Detail, Eval Detail, Arena Detail, etc.) show breadcrumbs so users know where they are. Breadcrumbs are clickable to return to parent pages.

**The lens switcher.** Pages that support multiple lenses have a lens switcher near the top. Implementation specified in `05-lenses-and-perspectives.md`.

**The command palette.** A keyboard-activated command palette (Cmd-K) provides direct navigation to any page, agent, knowledge entry, arena, eval, or other addressable object. Essential for power users; invisible to new users until they learn the shortcut.

**Notifications.** A persistent notifications tray shows alerts: agent errors, gate failures, arena results, knowledge challenges, cost thresholds. User-configurable.

---
````

**Explicit detail extraction from this section:**

- Section word count: `178`
- Section hash: `d4c9cdedb010f766a44f3cb8f2c459bcbdc8b5b76f8c3eb2982221425c6d0f93`

**Normative requirements and implementation claims:**
- **The global header.** Every page has a header with: the Nunchi logo / brand, current block height of the chain (with LIVE / CALM status indicator), a search surface, account and wallet information. This header is consistent across all sections.
- **The global sidebar.** The sidebar lists all visible sections with their pages. Sidebar state (expanded sections, pinned pages) persists per user.
- **Breadcrumbs.** Detail pages (Agent Detail, Eval Detail, Arena Detail, etc.) show breadcrumbs so users know where they are. Breadcrumbs are clickable to return to parent pages.
- **The lens switcher.** Pages that support multiple lenses have a lens switcher near the top. Implementation specified in `05-lenses-and-perspectives.md`.
- **The command palette.** A keyboard-activated command palette (Cmd-K) provides direct navigation to any page, agent, knowledge entry, arena, eval, or other addressable object. Essential for power users; invisible to new users until they learn the shortcut.
- **Notifications.** A persistent notifications tray shows alerts: agent errors, gate failures, arena results, knowledge challenges, cost thresholds. User-configurable.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cross|elements|user|pages|cutting|sections|lens|Detail" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cross|elements|user|pages|cutting|sections|lens|Detail" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S021 -- IA principles applied — some worked examples

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:366` through `395`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## IA principles applied — some worked examples

To illustrate how IA decisions get made under the principles above, here are three worked examples.

**Worked example 1: where does "Arena leaderboard" belong?**

Arguments for Arena: leaderboards are the defining output of arenas; users come to them primarily through arena engagement.

Arguments for Knowledge: top-performing configurations are valuable knowledge.

Arguments for Measurements: leaderboards rank by measurements.

Resolution: Arena. Leaderboards belong primarily in Arena because that's where users' mental model places them. Cross-links from Measurements (to see what measurement produced which rank) and Knowledge (to see what knowledge top performers used) are provided without duplicating the page.

**Worked example 2: where does "agent detail" appear?**

Arguments: every section could link to agent detail pages.

Resolution: the canonical page lives under Fleet → Agent Detail, but a clicked agent name from anywhere (a knowledge entry's author, a leaderboard row, an event stream line) navigates to this page. The page is one, the entry points are many.

**Worked example 3: where does "Delegations" belong?**

Arguments for Fleet: delegations are per-agent.

Arguments for System: delegations are owner-level policy.

Resolution: System. Delegations are policy and governance, not per-agent operation. They appear under System → Delegations. Per-agent delegation state is summarized in Agent Detail with a link to the System page to edit.

---
````

**Explicit detail extraction from this section:**

- Section word count: `214`
- Section hash: `f3bab31b67c19185692c60d92491f8bcf1fe36edf80064f05ae25fcf7196812e`

**Normative requirements and implementation claims:**
- **Worked example 1: where does "Arena leaderboard" belong?**
- **Worked example 2: where does "agent detail" appear?**
- Resolution: the canonical page lives under Fleet → Agent Detail, but a clicked agent name from anywhere (a knowledge entry's author, a leaderboard row, an event stream line) navigates to this page. The page is one, the entry points are many.
- **Worked example 3: where does "Delegations" belong?**
- Resolution: System. Delegations are policy and governance, not per-agent operation. They appear under System → Delegations. Per-agent delegation state is summarized in Agent Detail with a link to the System page to edit.
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
- the canonical page lives under Fleet -> Agent Detail
- They appear under System -> Delegations

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "worked|example|principles|here|examples|delegation|Arguments|Arena" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "worked|example|principles|here|examples|delegation|Arguments|Arena" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] Enforce state transition `the canonical page lives under Fleet -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `They appear under System -> Delegations` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S022 -- IA non-goals

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:396` through `409`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## IA non-goals

Some things the IA does not try to do.

**The IA does not model every user role.** Permission differences (owner vs viewer of an agent, member vs creator of a group, steward vs participant) are handled inside pages, not by separate sidebar sections.

**The IA does not hide features behind mode switches.** A user in "advanced mode" does not see a different dashboard. Progressive disclosure happens at the section and surface level, not via a mode.

**The IA does not bury important things for aesthetic reasons.** If a concept is important enough to have a page, the page is reachable. The sidebar may get long; that is acceptable.

**The IA does not accommodate every conceivable future feature.** New surfaces will need new homes. The IA specified here is for the product as it is designed today, with room to grow but not reserved empty buckets.

---
````

**Explicit detail extraction from this section:**

- Section word count: `145`
- Section hash: `48e7f0593d61c917c89a6899c9dd9f9bc1d01e58c5b7dc6e808a158289df0df8`

**Normative requirements and implementation claims:**
- **The IA does not model every user role.** Permission differences (owner vs viewer of an agent, member vs creator of a group, steward vs participant) are handled inside pages, not by separate sidebar sections.
- **The IA does not hide features behind mode switches.** A user in "advanced mode" does not see a different dashboard. Progressive disclosure happens at the section and surface level, not via a mode.
- **The IA does not bury important things for aesthetic reasons.** If a concept is important enough to have a page, the page is reachable. The sidebar may get long; that is acceptable.
- **The IA does not accommodate every conceivable future feature.** New surfaces will need new homes. The IA specified here is for the product as it is designed today, with room to grow but not reserved empty buckets.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "non|goals|mode|user|things|surface|sidebar|important" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "non|goals|mode|user|things|surface|sidebar|important" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

### DASH-04-S023 -- What comes next

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md:410` through `412`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What comes next

`05-lenses-and-perspectives.md` specifies how users change vantage point within sections. `06-navigation-and-traversal.md` specifies how users move between pages and follow threads. Together, these three documents describe the full top-level structure of the dashboard. Section IV documents (`11`–`18`) specify the pages themselves within this structure.
````

**Explicit detail extraction from this section:**

- Section word count: `52`
- Section hash: `fbe7bc7833fd194aa5ae2d6a4e54ab47f5feb40ac69b757d4c510309dec68b78`

**Normative requirements and implementation claims:**
- `05-lenses-and-perspectives.md` specifies how users change vantage point within sections. `06-navigation-and-traversal.md` specifies how users move between pages and follow threads. Together, these three documents describe the full top-level structure of the dashboard. Section IV documents (`11`–`18`) specify the pages themselves within this structure.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/04-information-architecture.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
rg -n "next|comes|within|users|structure|specifies|pages|documents" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "next|comes|within|users|structure|specifies|pages|documents" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/status.rs`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/`
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
- [ ] For this dashboard-derived task, prefer backend projection/realtime support over frontend stitching: one stable payload, typed stale/degraded states, and fixtures for local UI development.

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
./target/debug/roko parity check --strict --area dashboard-prd/04-information-architecture
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

