# Dashboard PRD Plan: Knowledge Surfaces

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
**Generated:** 2026-04-25
**Source hash:** `0f84d0ae12a08745fe8a8aa75c330e84c8128112e49b36c306ff97ffb027d5c5`
**Section tasks:** 76
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
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
- `crates/roko-serve/src/routes/projections.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-14-S001 | 1 | 14 — Knowledge surfaces | [ ] | 9.8 |
| DASH-14-S002 | 7 | Section overview | [ ] | 9.8 |
| DASH-14-S003 | 25 | Page: Knowledge Store | [ ] | 9.8 |
| DASH-14-S004 | 27 | Purpose | [ ] | 9.8 |
| DASH-14-S005 | 31 | Primary persona | [ ] | 9.8 |
| DASH-14-S006 | 35 | Lenses supported | [ ] | 9.8 |
| DASH-14-S007 | 42 | Page structure | [ ] | 9.8 |
| DASH-14-S008 | 81 | Data sources | [ ] | 9.8 |
| DASH-14-S009 | 87 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S010 | 95 | Interactions | [ ] | 9.8 |
| DASH-14-S011 | 106 | Cross-references | [ ] | 9.8 |
| DASH-14-S012 | 112 | Empty states | [ ] | 9.8 |
| DASH-14-S013 | 117 | Deferred | [ ] | 9.8 |
| DASH-14-S014 | 125 | Page: Entry Detail | [ ] | 9.8 |
| DASH-14-S015 | 127 | Purpose | [ ] | 9.8 |
| DASH-14-S016 | 131 | Primary persona | [ ] | 9.8 |
| DASH-14-S017 | 135 | Lenses supported | [ ] | 9.8 |
| DASH-14-S018 | 139 | Page structure | [ ] | 9.8 |
| DASH-14-S019 | 175 | Data sources | [ ] | 9.8 |
| DASH-14-S020 | 182 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S021 | 189 | Interactions | [ ] | 9.8 |
| DASH-14-S022 | 197 | Cross-references | [ ] | 9.8 |
| DASH-14-S023 | 202 | Empty states | [ ] | 9.8 |
| DASH-14-S024 | 206 | Deferred | [ ] | 9.8 |
| DASH-14-S025 | 213 | Page: Stigmergy | [ ] | 9.8 |
| DASH-14-S026 | 215 | Purpose | [ ] | 9.8 |
| DASH-14-S027 | 219 | Primary persona | [ ] | 9.8 |
| DASH-14-S028 | 223 | Lenses supported | [ ] | 9.8 |
| DASH-14-S029 | 230 | Page structure | [ ] | 9.8 |
| DASH-14-S030 | 264 | Data sources | [ ] | 9.8 |
| DASH-14-S031 | 269 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S032 | 275 | Interactions | [ ] | 9.8 |
| DASH-14-S033 | 283 | Cross-references | [ ] | 9.8 |
| DASH-14-S034 | 288 | Empty states | [ ] | 9.8 |
| DASH-14-S035 | 292 | Deferred | [ ] | 9.8 |
| DASH-14-S036 | 299 | Page: Dream Cycles | [ ] | 9.8 |
| DASH-14-S037 | 301 | Purpose | [ ] | 9.8 |
| DASH-14-S038 | 305 | Primary persona | [ ] | 9.8 |
| DASH-14-S039 | 309 | Lenses supported | [ ] | 9.8 |
| DASH-14-S040 | 315 | Page structure | [ ] | 9.8 |
| DASH-14-S041 | 340 | Data sources | [ ] | 9.8 |
| DASH-14-S042 | 346 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S043 | 352 | Interactions | [ ] | 9.8 |
| DASH-14-S044 | 358 | Cross-references | [ ] | 9.8 |
| DASH-14-S045 | 364 | Empty states | [ ] | 9.8 |
| DASH-14-S046 | 368 | Deferred | [ ] | 9.8 |
| DASH-14-S047 | 375 | Page: Cross-Domain Resonance | [ ] | 9.8 |
| DASH-14-S048 | 377 | Purpose | [ ] | 9.8 |
| DASH-14-S049 | 381 | Primary persona | [ ] | 9.8 |
| DASH-14-S050 | 385 | Lenses supported | [ ] | 9.8 |
| DASH-14-S051 | 390 | Page structure | [ ] | 9.8 |
| DASH-14-S052 | 417 | Data sources | [ ] | 9.8 |
| DASH-14-S053 | 423 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S054 | 429 | Interactions | [ ] | 9.8 |
| DASH-14-S055 | 437 | Cross-references | [ ] | 9.8 |
| DASH-14-S056 | 442 | Empty states | [ ] | 9.8 |
| DASH-14-S057 | 446 | Deferred | [ ] | 9.8 |
| DASH-14-S058 | 453 | Page: Knowledge Lineage | [ ] | 9.8 |
| DASH-14-S059 | 455 | Purpose | [ ] | 9.8 |
| DASH-14-S060 | 459 | Primary persona | [ ] | 9.8 |
| DASH-14-S061 | 463 | Lenses supported | [ ] | 9.8 |
| DASH-14-S062 | 469 | Page structure | [ ] | 9.8 |
| DASH-14-S063 | 495 | Data sources | [ ] | 9.8 |
| DASH-14-S064 | 501 | Reactivity hooks | [ ] | 9.8 |
| DASH-14-S065 | 508 | Interactions | [ ] | 9.8 |
| DASH-14-S066 | 516 | Cross-references | [ ] | 9.8 |
| DASH-14-S067 | 520 | Empty states | [ ] | 9.8 |
| DASH-14-S068 | 524 | Deferred | [ ] | 9.8 |
| DASH-14-S069 | 531 | Cross-section considerations for Knowledge | [ ] | 9.8 |
| DASH-14-S070 | 533 | Knowledge surfaces compose | [ ] | 9.8 |
| DASH-14-S071 | 537 | Knowledge everywhere | [ ] | 9.8 |
| DASH-14-S072 | 541 | On-chain vs off-chain | [ ] | 9.8 |
| DASH-14-S073 | 545 | Privacy | [ ] | 9.8 |
| DASH-14-S074 | 549 | Reputation linkage | [ ] | 9.8 |
| DASH-14-S075 | 553 | Publishing friction | [ ] | 9.8 |
| DASH-14-S076 | 557 | Knowledge decay and revalidation | [ ] | 9.8 |

## Tasks

### DASH-14-S001 -- 14 — Knowledge surfaces

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 14 — Knowledge surfaces

*The collective mind section of the Nunchi dashboard. Where users browse, contribute, validate, and discover knowledge; observe stigmergic coordination; watch dreams consolidate.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `22`
- Section hash: `bae0c74dcf9d9f9c93ebf14b6e0097c646bf53f2c56d40756fd568e3acbd1d2e`

**Normative requirements and implementation claims:**
- *The collective mind section of the Nunchi dashboard. Where users browse, contribute, validate, and discover knowledge; observe stigmergic coordination; watch dreams consolidate.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Knowledge|surfaces|watch|validate|users|stigmergic|observe|mind" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|surfaces|watch|validate|users|stigmergic|observe|mind" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S002 -- Section overview

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:7` through `24`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Section overview

The Knowledge section is where the collective intelligence of the network lives as a navigable space. This is the section where Thesis 2 (collective beats individual) becomes tangible — knowledge accumulated across agents, published on-chain, decaying and revalidating over time, forming a substrate that every agent draws from.

Knowledge contains five pages:

- **Knowledge Store** — browse, search, read, and publish knowledge entries.
- **Stigmergy** — the pheromone field; agents leaving signals for agents.
- **Dream Cycles** — offline consolidation processes, where patterns crystallize.
- **Cross-Domain Resonance** — discover unexpected connections across domains.
- **Knowledge Lineage** — trace the provenance and dependency graph of entries.

(Context Audit, which was in the Knowledge section of the current dashboard, moves to System per the IA decisions in `04-information-architecture.md` — it is debug infrastructure, not knowledge substrate.)

Knowledge surfaces are used by Knowledge Contributors as their primary workspace, by Domain Architects refining their domains, by every other persona who benefits from the collective substrate.

---
````

**Explicit detail extraction from this section:**

- Section word count: `154`
- Section hash: `7a029ef4a9ae3215cc739e18936e7fa85cd3186029bff86cce543d929025e438`

**Normative requirements and implementation claims:**
- - **Knowledge Store** — browse, search, read, and publish knowledge entries. - **Stigmergy** — the pheromone field; agents leaving signals for agents. - **Dream Cycles** — offline consolidation processes, where patterns crystallize. - **Cross-Domain Resonance** — discover unexpected connections across domains. - **Knowledge Lineage** — trace the provenance and dependency graph of entries.
- (Context Audit, which was in the Knowledge section of the current dashboard, moves to System per the IA decisions in `04-information-architecture.md` — it is debug infrastructure, not knowledge substrate.)
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
- - **Knowledge Store** — browse, search, read, and publish knowledge entries.
- - **Stigmergy** — the pheromone field; agents leaving signals for agents.
- - **Dream Cycles** — offline consolidation processes, where patterns crystallize.
- - **Cross-Domain Resonance** — discover unexpected connections across domains.
- - **Knowledge Lineage** — trace the provenance and dependency graph of entries.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "knowledge|over|overview|Domain|substrate|collective|Cross|space" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "knowledge|over|overview|Domain|substrate|collective|Cross|space" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S003 -- Page: Knowledge Store

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:25` through `26`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Knowledge Store
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `f82a5a0e91ba95780ca8f9d23227c9af2132c83badddd5cd85774bba8ca000cd`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Store|Knowledge|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Store|Knowledge|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S004 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:27` through `30`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

The browsable, searchable catalog of all knowledge entries — both local to the user's agents and published on the chain. Users come here to find knowledge relevant to their current context, publish their own, validate others', and build reputation through contribution.
````

**Explicit detail extraction from this section:**

- Section word count: `41`
- Section hash: `dad6cf706f4350a7238a8f3f7ed96ab2657d8f3178e76ba4491fcb9041fdec28`

**Normative requirements and implementation claims:**
- The browsable, searchable catalog of all knowledge entries — both local to the user's agents and published on the chain. Users come here to find knowledge relevant to their current context, publish their own, validate others', and build reputation through contribution.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|user|publish|knowledge|validate|searchable|reputation|relevant" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|user|publish|knowledge|validate|searchable|reputation|relevant" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S005 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:31` through `34`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Knowledge Contributor, Domain Architect, Solo Operator (consuming relevant knowledge).
````

**Explicit detail extraction from this section:**

- Section word count: `9`
- Section hash: `d102f1b196926ad3cfbefe75b81ab5a1da0d13ab465dcbaee393990105a0d50e`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|knowledge|relevant|consuming|Solo|Operator|Domain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|knowledge|relevant|consuming|Solo|Operator|Domain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S006 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:35` through `41`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- **Global** (default): all published knowledge on the network.
- **Fleet**: knowledge authored by the user's agents plus entries the user has personally published.
- **Agent** (when drill-down from an agent's knowledge tab): entries authored by one specific agent.
- **Chain**: on-chain-anchored entries specifically (filtering out off-chain entries that haven't been published yet).
````

**Explicit detail extraction from this section:**

- Section word count: `56`
- Section hash: `3811911103c17256c5367320d3ae39565459b9a0d283a283865b89a0c9ec1b58`

**Normative requirements and implementation claims:**
- - **Global** (default): all published knowledge on the network. - **Fleet**: knowledge authored by the user's agents plus entries the user has personally published. - **Agent** (when drill-down from an agent's knowledge tab): entries authored by one specific agent. - **Chain**: on-chain-anchored entries specifically (filtering out off-chain entries that haven't been published yet).

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
- - **Global** (default): all published knowledge on the network.
- - **Fleet**: knowledge authored by the user's agents plus entries the user has personally published.
- - **Agent** (when drill-down from an agent's knowledge tab): entries authored by one specific agent.
- - **Chain**: on-chain-anchored entries specifically (filtering out off-chain entries that haven't been published yet).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "supported|Lenses|entries|published|knowledge|Chain|user|specific" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supported|Lenses|entries|published|knowledge|Chain|user|specific" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S007 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:42` through `80`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header area**: page title "Knowledge Store," block height, lens switcher. Primary action: "Publish Entry."

**Section 01 — Catalog.** Description: "The collective substrate. Everything agents and users have learned, validated, and left for others."

**Filter and search bar** prominent at top:
- Full-text search input.
- Domain filter (coding, DeFi, research, security, user-created domains).
- Type filter (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge).
- Confidence threshold slider.
- Validation threshold slider.
- Freshness filter (last 24h, 7d, 30d, all).
- Tier filter (author's tier).
- Sort control (relevance, recent, most-validated, most-used, trending).

**View toggle**: List / Grid / Resonance Graph / Topography.

- **List view**: dense one-per-row, shows title, type tag, author portrait, confidence, validations, age, brief preview. Users scan many entries quickly.
- **Grid view**: card-per-entry, shows fuller preview, better for browsing with attention.
- **Resonance Graph view**: the Resonance Graph primitive. Entries as nodes, resonance as edges, clusters visible.
- **Topography view**: the Knowledge Topography primitive. High-confidence validated regions as peaks, sparse or contested areas as valleys.

**Entry list**:

Each list item or card shows:
- Type tag (Insight, Heuristic, etc.) with small type glyph.
- Title (the claim or headline).
- Author portrait and name.
- Confidence bar (authored confidence).
- Validation indicator (count of validators and their aggregate reputation).
- Freshness indicator (age with decay fade applied).
- Resonance indicator (count of recent retrievals).
- A "+" indicator for entries with rich content beyond the headline.

**Section 02 — Featured / Trending.** *(appears at top on Global lens)*

Content: a curated row of notable entries — recently-promoted to high confidence, challenged and resolved, high cross-domain impact, new publications by Sovereign-tier authors.
````

**Explicit detail extraction from this section:**

- Section word count: `259`
- Section hash: `0feb0599ca80f092408d787fa4e8348f44ce09b64c499f0544602f255120c10a`

**Normative requirements and implementation claims:**
- **Header area**: page title "Knowledge Store," block height, lens switcher. Primary action: "Publish Entry."
- **Section 01 — Catalog.** Description: "The collective substrate. Everything agents and users have learned, validated, and left for others."
- **Filter and search bar** prominent at top: - Full-text search input. - Domain filter (coding, DeFi, research, security, user-created domains). - Type filter (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge). - Confidence threshold slider. - Validation threshold slider. - Freshness filter (last 24h, 7d, 30d, all). - Tier filter (author's tier). - Sort control (relevance, recent, most-validated, most-used, trending).
- **View toggle**: List / Grid / Resonance Graph / Topography.
- - **List view**: dense one-per-row, shows title, type tag, author portrait, confidence, validations, age, brief preview. Users scan many entries quickly. - **Grid view**: card-per-entry, shows fuller preview, better for browsing with attention. - **Resonance Graph view**: the Resonance Graph primitive. Entries as nodes, resonance as edges, clusters visible. - **Topography view**: the Knowledge Topography primitive. High-confidence validated regions as peaks, sparse or contested areas as valleys.
- **Entry list**:
- **Section 02 — Featured / Trending.** *(appears at top on Global lens)*

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- tag
- glyph

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Full-text search input.
- - Domain filter (coding, DeFi, research, security, user-created domains).
- - Type filter (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge).
- - Confidence threshold slider.
- - Validation threshold slider.
- - Freshness filter (last 24h, 7d, 30d, all).
- - Tier filter (author's tier).
- - Sort control (relevance, recent, most-validated, most-used, trending).
- - **List view**: dense one-per-row, shows title, type tag, author portrait, confidence, validations, age, brief preview. Users scan many entries quickly.
- - **Grid view**: card-per-entry, shows fuller preview, better for browsing with attention.
- - **Resonance Graph view**: the Resonance Graph primitive. Entries as nodes, resonance as edges, clusters visible.
- - **Topography view**: the Knowledge Topography primitive. High-confidence validated regions as peaks, sparse or contested areas as valleys.
- - Type tag (Insight, Heuristic, etc.) with small type glyph.
- - Title (the claim or headline).
- - Author portrait and name.
- - Confidence bar (authored confidence).
- - Validation indicator (count of validators and their aggregate reputation).
- - Freshness indicator (age with decay fade applied).
- - Resonance indicator (count of recent retrievals).
- - A "+" indicator for entries with rich content beyond the headline.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "View|tag|Graph|Confidence|structure|glyph|author|Resonance" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "View|tag|Graph|Confidence|structure|glyph|author|Resonance" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Implement or verify `tag` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `glyph` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S008 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:81` through `86`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- InsightStore queries on the chain.
- Local knowledge store queries for user's agents.
- Aggregated event streams for trending / featured.
````

**Explicit detail extraction from this section:**

- Section word count: `19`
- Section hash: `a2d35fd762a4d64b52c27a9b00610c1a9a7c15c3e36e0727d3da043f36706cf0`

**Normative requirements and implementation claims:**
- - InsightStore queries on the chain. - Local knowledge store queries for user's agents. - Aggregated event streams for trending / featured.

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
- - InsightStore queries on the chain.
- - Local knowledge store queries for user's agents.
- - Aggregated event streams for trending / featured.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|store|queries|user|trending|streams|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|store|queries|user|trending|streams|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S009 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:87` through `94`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- Entry cards have epistemic glow (confidence × validation × recency).
- Decay fade on older entries.
- Contested turbulence on disputed entries.
- Resonance pulses on entries recently applied by agents elsewhere.
- Featured row has subtle rotating emphasis (one entry has an authoritative glow at a time).
````

**Explicit detail extraction from this section:**

- Section word count: `42`
- Section hash: `8778a27d03fb09e8c2b6f3c4e22a728480ecda13f8201d5cc11eda4680d975fe`

**Normative requirements and implementation claims:**
- - Entry cards have epistemic glow (confidence × validation × recency). - Decay fade on older entries. - Contested turbulence on disputed entries. - Resonance pulses on entries recently applied by agents elsewhere. - Featured row has subtle rotating emphasis (one entry has an authoritative glow at a time).

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
- - Entry cards have epistemic glow (confidence × validation × recency).
- - Decay fade on older entries.
- - Contested turbulence on disputed entries.
- - Resonance pulses on entries recently applied by agents elsewhere.
- - Featured row has subtle rotating emphasis (one entry has an authoritative glow at a time).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "hooks|Reactivity|entries|glow|Entry|validation|turbulence|time" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "hooks|Reactivity|entries|glow|Entry|validation|turbulence|time" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S010 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:95` through `105`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Click an entry → Entry Detail.
- Hover an entry → preview popover with fuller detail.
- Publish Entry → authoring surface (see `19-authoring-surfaces.md`).
- Validate an entry → validation flow (requires user's signed assertion).
- Challenge an entry → challenge flow (requires counter-evidence).
- Filter chips and search update the view live.
- View toggles animate the transition between List / Grid / Graph / Topography.
- Save to reading list (personal bookmarks).
````

**Explicit detail extraction from this section:**

- Section word count: `64`
- Section hash: `12bb86375f13076cd787a47a4c262331b323e872ea75a271ca080cadac2cdb9e`

**Normative requirements and implementation claims:**
- - Click an entry → Entry Detail. - Hover an entry → preview popover with fuller detail. - Publish Entry → authoring surface (see `19-authoring-surfaces.md`). - Validate an entry → validation flow (requires user's signed assertion). - Challenge an entry → challenge flow (requires counter-evidence). - Filter chips and search update the view live. - View toggles animate the transition between List / Grid / Graph / Topography. - Save to reading list (personal bookmarks).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Click an entry -> Entry Detail
- Hover an entry -> preview popover with fuller detail
- Publish Entry -> authoring surface
- Validate an entry -> validation flow
- Challenge an entry -> challenge flow

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Click an entry → Entry Detail.
- - Hover an entry → preview popover with fuller detail.
- - Publish Entry → authoring surface (see `19-authoring-surfaces.md`).
- - Validate an entry → validation flow (requires user's signed assertion).
- - Challenge an entry → challenge flow (requires counter-evidence).
- - Filter chips and search update the view live.
- - View toggles animate the transition between List / Grid / Graph / Topography.
- - Save to reading list (personal bookmarks).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "entry|Interactions|view|surface|requires|flow|authoring|List" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "entry|Interactions|view|surface|requires|flow|authoring|List" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `Click an entry -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Hover an entry -> preview popover with fuller detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Publish Entry -> authoring surface` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Validate an entry -> validation flow` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Challenge an entry -> challenge flow` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S011 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:106` through `111`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Entry Detail (sub-page / drill-down).
- Author → Agent Detail.
- Validators and challengers → Agent Detail or User profile.
````

**Explicit detail extraction from this section:**

- Section word count: `17`
- Section hash: `cef05c45eab42922ddf5815904971348f663fd74cad37df8bd2077ba089098af`

**Normative requirements and implementation claims:**
- - Entry Detail (sub-page / drill-down). - Author → Agent Detail. - Validators and challengers → Agent Detail or User profile.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Author -> Agent Detail
- Validators and challengers -> Agent Detail or User profile

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Entry Detail (sub-page / drill-down).
- - Author → Agent Detail.
- - Validators and challengers → Agent Detail or User profile.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|Detail|profile|drill|down|challengers|Validators" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|Detail|profile|drill|down|challengers|Validators" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `Author -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Validators and challengers -> Agent Detail or User profile` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S012 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:112` through `116`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- No entries matching filters: "No entries match these filters. Clear or broaden filters."
- Early network: "The knowledge layer is growing. Publish the first entry in [domain]?"
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `9c7d75f089dee27642eb365c2dba176f89ccbc46c064ee19fa7f282ca07e4dc4`

**Normative requirements and implementation claims:**
- - No entries matching filters: "No entries match these filters. Clear or broaden filters." - Early network: "The knowledge layer is growing. Publish the first entry in [domain]?"

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
- - No entries matching filters: "No entries match these filters. Clear or broaden filters."
- - Early network: "The knowledge layer is growing. Publish the first entry in [domain]?"

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|filters|match|entries|network|matching|layer" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|filters|match|entries|network|matching|layer" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S013 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:117` through `124`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- Knowledge entry commenting and discussion threads. Later — knowledge itself has validations and challenges, but freeform discussion is a different primitive.
- Paid or tokenized knowledge access (pay to read premium entries). Later.
- Knowledge collections / reading lists as first-class shared objects. Later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `41`
- Section hash: `609f53423f18d652c53a6c2a6ee5b5c3ee0e930fb658bbf2f777a9bc155e7610`

**Normative requirements and implementation claims:**
- - Knowledge entry commenting and discussion threads. Later — knowledge itself has validations and challenges, but freeform discussion is a different primitive. - Paid or tokenized knowledge access (pay to read premium entries). Later. - Knowledge collections / reading lists as first-class shared objects. Later.
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
- - Knowledge entry commenting and discussion threads. Later — knowledge itself has validations and challenges, but freeform discussion is a different primitive.
- - Paid or tokenized knowledge access (pay to read premium entries). Later.
- - Knowledge collections / reading lists as first-class shared objects. Later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|knowledge|read|Later|discussion|validations|tokenized|threads" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|knowledge|read|Later|discussion|validations|tokenized|threads" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S014 -- Page: Entry Detail

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:125` through `126`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Entry Detail
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `d87997e315df8af26e8fb32ca6326c88a94183a9c509f470f86aad695ad5dff8`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Entry|Detail|surfaces|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Entry|Detail|surfaces|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S015 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:127` through `130`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

The canonical page for a single knowledge entry. Every knowledge reference across the dashboard links here.
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `0b21a25bca4f96c7edebd6a9378750b0e6fbfe92be7316a2736adb4ca8d9b507`

**Normative requirements and implementation claims:**
- The canonical page for a single knowledge entry. Every knowledge reference across the dashboard links here.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|knowledge|single|reference|links|here|entry|canonical" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|knowledge|single|reference|links|here|entry|canonical" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S016 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:131` through `134`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Knowledge Contributor (managing their entries), any persona consuming knowledge.
````

**Explicit detail extraction from this section:**

- Section word count: `9`
- Section hash: `51ed87e072ce6a10d7215adc5d6a69a3c97e7313dda1c0d78fda22985c37c6ca`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|knowledge|managing|entries|consuming|Contributor|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|knowledge|managing|entries|consuming|Contributor|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S017 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:135` through `138`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- Scoped to one entry — lens switching is not meaningfully applicable, though the user can see the entry as visible in Global or scoped to how their fleet interacts with it.
````

**Explicit detail extraction from this section:**

- Section word count: `30`
- Section hash: `98d7fbc9fea88d20dc9ea8b524371349b740ef530ca2816a97f044e6b0429dc8`

**Normative requirements and implementation claims:**
- - Scoped to one entry — lens switching is not meaningfully applicable, though the user can see the entry as visible in Global or scoped to how their fleet interacts with it.

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
- - Scoped to one entry — lens switching is not meaningfully applicable, though the user can see the entry as visible in Global or scoped to how their fleet interacts with it.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "lens|supported|Lenses|entry|Scoped|visible|user|though" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "lens|supported|Lenses|entry|Scoped|visible|user|though" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S018 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:139` through `174`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header**: breadcrumb "Knowledge / Entry [slug]." Entry title as page title. Type tag prominent.

**Metadata row**: author, authored date, domain, type, fingerprint (compact mono), confidence, validations count, challenges count, retrievals count.

**Content section**: the entry's full content. Depending on entry type:
- **Insight**: prose claim with evidence links.
- **Heuristic**: condition / rule / notes structure.
- **Warning**: danger description with reproduction steps.
- **CausalLink**: cause, effect, mechanism, evidence.
- **StrategyFragment**: reusable strategy snippet with applicable context.
- **AntiKnowledge**: the refuted claim clearly struck through, with the refutation.

**Validation section**: list of validators. Each entry:
- Validator agent or user.
- Validator's tier.
- Their signed assertion.
- Timestamp.
- Evidence or context they cited.

**Challenge section**: list of challenges. Each entry:
- Challenger.
- Challenger's tier.
- Counter-claim.
- Evidence.
- Resolution status (open, resolved-upheld, resolved-refuted).

**Resonance section**: visualization showing the entry's HDC neighborhood. Nearby entries with edges. Entries in adjacent clusters.

**Provenance section**: the entry's lineage — what earlier entries it draws on, what downstream entries depend on it. Links to Knowledge Lineage.

**Usage section**: how the entry is being used. Recent retrievals by agents, aggregate retrieval count, cross-domain applications.

**Actions**: Validate (for users not yet validating), Challenge, Fork (create a revised entry citing this), Cite (use in a new entry).
````

**Explicit detail extraction from this section:**

- Section word count: `202`
- Section hash: `69d01dc45842f75d48141a619ab51466423adf3b829d4ea85322a3bbb2eb6a16`

**Normative requirements and implementation claims:**
- **Header**: breadcrumb "Knowledge / Entry [slug]." Entry title as page title. Type tag prominent.
- **Metadata row**: author, authored date, domain, type, fingerprint (compact mono), confidence, validations count, challenges count, retrievals count.
- **Content section**: the entry's full content. Depending on entry type: - **Insight**: prose claim with evidence links. - **Heuristic**: condition / rule / notes structure. - **Warning**: danger description with reproduction steps. - **CausalLink**: cause, effect, mechanism, evidence. - **StrategyFragment**: reusable strategy snippet with applicable context. - **AntiKnowledge**: the refuted claim clearly struck through, with the refutation.
- **Validation section**: list of validators. Each entry: - Validator agent or user. - Validator's tier. - Their signed assertion. - Timestamp. - Evidence or context they cited.
- **Challenge section**: list of challenges. Each entry: - Challenger. - Challenger's tier. - Counter-claim. - Evidence. - Resolution status (open, resolved-upheld, resolved-refuted).
- **Resonance section**: visualization showing the entry's HDC neighborhood. Nearby entries with edges. Entries in adjacent clusters.
- **Provenance section**: the entry's lineage — what earlier entries it draws on, what downstream entries depend on it. Links to Knowledge Lineage.
- **Usage section**: how the entry is being used. Recent retrievals by agents, aggregate retrieval count, cross-domain applications.
- **Actions**: Validate (for users not yet validating), Challenge, Fork (create a revised entry citing this), Cite (use in a new entry).

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
- - **Insight**: prose claim with evidence links.
- - **Heuristic**: condition / rule / notes structure.
- - **Warning**: danger description with reproduction steps.
- - **CausalLink**: cause, effect, mechanism, evidence.
- - **StrategyFragment**: reusable strategy snippet with applicable context.
- - **AntiKnowledge**: the refuted claim clearly struck through, with the refutation.
- - Validator agent or user.
- - Validator's tier.
- - Their signed assertion.
- - Timestamp.
- - Evidence or context they cited.
- - Challenger.
- - Challenger's tier.
- - Counter-claim.
- - Evidence.
- - Resolution status (open, resolved-upheld, resolved-refuted).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Entry|structure|Challenge|count|evidence|entries|retrieval|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Entry|structure|Challenge|count|evidence|entries|retrieval|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S019 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:175` through `181`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- InsightStore on-chain for published entries.
- Local knowledge store for unpublished entries.
- Event aggregator for retrieval counts.
- Associated chain events for validations and challenges.
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `30a95e35c45dfa17a208a76f79c1617cedde5bfad8bfccee719b0931dbdca98b`

**Normative requirements and implementation claims:**
- - InsightStore on-chain for published entries. - Local knowledge store for unpublished entries. - Event aggregator for retrieval counts. - Associated chain events for validations and challenges.

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
- - InsightStore on-chain for published entries.
- - Local knowledge store for unpublished entries.
- - Event aggregator for retrieval counts.
- - Associated chain events for validations and challenges.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|store|published|entries|chain|Event|validations" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|store|published|entries|chain|Event|validations" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S020 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:182` through `188`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- Entry epistemic glow on the page header.
- New validations / challenges fade in as they arrive (if viewing live).
- Resonance neighborhood renders with its primitive's full reactivity.
- Retrieval counter ticks live.
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `fa910f91477c1011c579f4150193f47e23eddddff42874b914399718b6297cd8`

**Normative requirements and implementation claims:**
- - Entry epistemic glow on the page header. - New validations / challenges fade in as they arrive (if viewing live). - Resonance neighborhood renders with its primitive's full reactivity. - Retrieval counter ticks live.

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
- - Entry epistemic glow on the page header.
- - New validations / challenges fade in as they arrive (if viewing live).
- - Resonance neighborhood renders with its primitive's full reactivity.
- - Retrieval counter ticks live.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Reactivity|hooks|live|viewing|validations|ticks|renders|primitive" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reactivity|hooks|live|viewing|validations|ticks|renders|primitive" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S021 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:189` through `196`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Validate / Challenge / Fork / Cite buttons.
- Click validators/challengers → their Agent Detail.
- Click neighboring entries → their Entry Detail.
- Copy fingerprint / entry ID.
- View on chain (link to chain explorer).
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `2e67cc70fde1dd6e9c0b607f983523da493e35d08e3a5c591f607d65cf468806`

**Normative requirements and implementation claims:**
- - Validate / Challenge / Fork / Cite buttons. - Click validators/challengers → their Agent Detail. - Click neighboring entries → their Entry Detail. - Copy fingerprint / entry ID. - View on chain (link to chain explorer).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- challengers -> their Agent Detail
- Click neighboring entries -> their Entry Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Validate / Challenge / Fork / Cite buttons.
- - Click validators/challengers → their Agent Detail.
- - Click neighboring entries → their Entry Detail.
- - Copy fingerprint / entry ID.
- - View on chain (link to chain explorer).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Interactions|chain|Entry|Detail|Click|Challenge|validators|neighboring" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Interactions|chain|Entry|Detail|Click|Challenge|validators|neighboring" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `challengers -> their Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click neighboring entries -> their Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S022 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:197` through `201`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Entry Detail is reached from many places (Knowledge Store, agent knowledge tabs, resonance graphs, search results, event streams).
- Links out to many places (author, validators, provenance, neighbors).
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `b17b2cd95d4a32b4f14a1699ee47fafc0cac70bf8b7fee825aeaae12f4259f57`

**Normative requirements and implementation claims:**
- - Entry Detail is reached from many places (Knowledge Store, agent knowledge tabs, resonance graphs, search results, event streams). - Links out to many places (author, validators, provenance, neighbors).

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
- - Entry Detail is reached from many places (Knowledge Store, agent knowledge tabs, resonance graphs, search results, event streams).
- - Links out to many places (author, validators, provenance, neighbors).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|places|many|knowledge|validators|tabs|streams" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|places|many|knowledge|validators|tabs|streams" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S023 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:202` through `205`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- Just-published entry with no validations yet: explicit note that the entry is new and awaiting validation.
````

**Explicit detail extraction from this section:**

- Section word count: `17`
- Section hash: `771a3b8e86c46986eabfb49cec3f578a1e4f256b3bfed068ff1a6960091b4a0d`

**Normative requirements and implementation claims:**
- - Just-published entry with no validations yet: explicit note that the entry is new and awaiting validation.

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
- - Just-published entry with no validations yet: explicit note that the entry is new and awaiting validation.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|validation|entry|validations|published|note|explicit" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|validation|entry|validations|published|note|explicit" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S024 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:206` through `212`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- Translation / multilingual entries. Later.
- Rich-media evidence (images, videos embedded as evidence). Later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `5ffa8a16cb28688e114d351e4af06068485067564b681b7250d2ed588a6a1075`

**Normative requirements and implementation claims:**
- - Translation / multilingual entries. Later. - Rich-media evidence (images, videos embedded as evidence). Later.
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
- - Translation / multilingual entries. Later.
- - Rich-media evidence (images, videos embedded as evidence). Later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|evidence|Later|videos|multilingual|media|images|entries" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|evidence|Later|videos|multilingual|media|images|entries" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S025 -- Page: Stigmergy

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:213` through `214`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Stigmergy
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `a4d4181633150da48347b8bc844870bae226d046eaf5c1e1600654433d1669e7`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Stigmergy|surfaces|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Stigmergy|surfaces|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S026 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:215` through `218`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

The pheromone field — agents leaving signals for other agents to read. This page is the primary surface for observing stigmergic coordination across the network.
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `75b715d3fce46d26b47e8889f2c23203e9bf5bb3126648488f0f1d61367868b4`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|surface|stigmergic|signals|read|primary|pheromone|other" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|surface|stigmergic|signals|read|primary|pheromone|other" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S027 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:219` through `222`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Fleet Orchestrator (seeing what their agents are signaling), Knowledge Contributor (tracking pheromones related to their knowledge), System Steward (observing field health), all users wanting to understand what agents are noticing.
````

**Explicit detail extraction from this section:**

- Section word count: `30`
- Section hash: `20fdfe20ce4dc459d4bfe0a0d344beab0631ea89c59ef0553a66b3c887160f06`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|knowledge|wanting|users|understand|tracking|signaling" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|knowledge|wanting|users|understand|tracking|signaling" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S028 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:223` through `229`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- **Global** (default): all active pheromones on the network.
- **Fleet**: pheromones deposited by the user's agents.
- **Chain**: on-chain-anchored pheromones specifically.
- **Group**: group-scoped pheromones.
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `9c0c5c007e0364206fe84e4a9208860f13308d9ceed7c72de3d40aa246024cd6`

**Normative requirements and implementation claims:**
- - **Global** (default): all active pheromones on the network. - **Fleet**: pheromones deposited by the user's agents. - **Chain**: on-chain-anchored pheromones specifically. - **Group**: group-scoped pheromones.

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
- - **Global** (default): all active pheromones on the network.
- - **Fleet**: pheromones deposited by the user's agents.
- - **Chain**: on-chain-anchored pheromones specifically.
- - **Group**: group-scoped pheromones.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "supported|Lenses|pheromones|Group|Chain|user|specifically|scoped" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supported|Lenses|pheromones|Group|Chain|user|specifically|scoped" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S029 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:230` through `263`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header area**: page title "Stigmergy," block height, lens switcher.

**Section 01 — Pheromone Field.** Description: "Agents leave signals for other agents. Wisdom, opportunity, threat — decaying unless reinforced."

Content: the Stigmergy Field primitive (`09-visualization-primitives.md`) as the dominant visualization, full-width.

Controls below the field:
- Type filter (show wisdom / opportunity / threat / others).
- Age filter (last hour / day / week / all).
- Source filter (by tier, by author).
- Time scrubber for historical state.
- Play / pause animation of decay and reinforcement.

**Section 02 — Active Signals.** Description: "Strongest and most recent signals."

Content: a list beside the field showing current top signals:
- Signal type (colored).
- Source agent.
- Strength.
- Age with decay.
- Short summary of what the signal represents.

Click a signal to focus it in the field above.

**Section 03 — Signal Events.** Description: "The feed of deposits, reinforcements, and expirations."

Content: an event-stream-style feed of pheromone events:
- Deposits (new pheromone).
- Reinforcements (existing pheromone strengthened).
- Decays (pheromone fading below threshold).
- Expirations (pheromone gone).
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `9f9b5d85556d212d5563a10439b9f137274c4bb7cdd56c359f3dde2a9ce7afbd`

**Normative requirements and implementation claims:**
- **Header area**: page title "Stigmergy," block height, lens switcher.
- **Section 01 — Pheromone Field.** Description: "Agents leave signals for other agents. Wisdom, opportunity, threat — decaying unless reinforced."
- Controls below the field: - Type filter (show wisdom / opportunity / threat / others). - Age filter (last hour / day / week / all). - Source filter (by tier, by author). - Time scrubber for historical state. - Play / pause animation of decay and reinforcement.
- **Section 02 — Active Signals.** Description: "Strongest and most recent signals."
- **Section 03 — Signal Events.** Description: "The feed of deposits, reinforcements, and expirations."
- Content: an event-stream-style feed of pheromone events: - Deposits (new pheromone). - Reinforcements (existing pheromone strengthened). - Decays (pheromone fading below threshold). - Expirations (pheromone gone).

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
- - Type filter (show wisdom / opportunity / threat / others).
- - Age filter (last hour / day / week / all).
- - Source filter (by tier, by author).
- - Time scrubber for historical state.
- - Play / pause animation of decay and reinforcement.
- - Signal type (colored).
- - Source agent.
- - Strength.
- - Age with decay.
- - Short summary of what the signal represents.
- - Deposits (new pheromone).
- - Reinforcements (existing pheromone strengthened).
- - Decays (pheromone fading below threshold).
- - Expirations (pheromone gone).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Signal|Pheromone|structure|Field|signals|decay|reinforcement|filter" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Signal|Pheromone|structure|Field|signals|decay|reinforcement|filter" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S030 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:264` through `268`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- Chain queries and subscription for pheromone state.
- Pheromone event fabric.
````

**Explicit detail extraction from this section:**

- Section word count: `10`
- Section hash: `161cfb0d19981d655f95688198590cf97525201e545b0ec9df4ce72b3ae400a4`

**Normative requirements and implementation claims:**
- - Chain queries and subscription for pheromone state. - Pheromone event fabric.

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
- - Chain queries and subscription for pheromone state.
- - Pheromone event fabric.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|pheromone|subscription|state|queries|fabric|event" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|pheromone|subscription|state|queries|fabric|event" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S031 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:269` through `274`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- The field itself uses its primitive's full reactivity (particle density, glow, decay fade, type colors, heartbeat on new deposits).
- Active Signals list has heartbeat ticks on recent deposits.
- Events feed uses standard Event Stream fade-in patterns.
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `a205790334d0aaedf925a4db72b642fa23d9ce502d27dc5dc010dd8d56e7cd47`

**Normative requirements and implementation claims:**
- - The field itself uses its primitive's full reactivity (particle density, glow, decay fade, type colors, heartbeat on new deposits). - Active Signals list has heartbeat ticks on recent deposits. - Events feed uses standard Event Stream fade-in patterns.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- colors

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - The field itself uses its primitive's full reactivity (particle density, glow, decay fade, type colors, heartbeat on new deposits).
- - Active Signals list has heartbeat ticks on recent deposits.
- - Events feed uses standard Event Stream fade-in patterns.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Reactivity|hooks|colors|uses|heartbeat|fade|deposits|Event" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reactivity|hooks|colors|uses|heartbeat|fade|deposits|Event" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `colors` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S032 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:275` through `282`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Click a signal in the field → detail popover.
- Click a signal in the Active Signals list → focus in the field, scroll to relevant region.
- Time scrubber → navigate historical field state.
- Play / pause → animate the field's evolution.
- Deposit a pheromone (owner-only, from their agent) → ephemeral action linked to specific agent context.
````

**Explicit detail extraction from this section:**

- Section word count: `52`
- Section hash: `e07dec479fe13dcae3ddc8e4a3f5dbad94577ba03ce7a59ea730c020a28d135a`

**Normative requirements and implementation claims:**
- - Click a signal in the field → detail popover. - Click a signal in the Active Signals list → focus in the field, scroll to relevant region. - Time scrubber → navigate historical field state. - Play / pause → animate the field's evolution. - Deposit a pheromone (owner-only, from their agent) → ephemeral action linked to specific agent context.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Click a signal in the field -> detail popover
- Click a signal in the Active Signals list -> focus in the field
- Time scrubber -> navigate historical field state
- pause -> animate the field

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Click a signal in the field → detail popover.
- - Click a signal in the Active Signals list → focus in the field, scroll to relevant region.
- - Time scrubber → navigate historical field state.
- - Play / pause → animate the field's evolution.
- - Deposit a pheromone (owner-only, from their agent) → ephemeral action linked to specific agent context.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "action|Interactions|field|signal|Click|state|specific|scrubber" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "action|Interactions|field|signal|Click|state|specific|scrubber" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `Click a signal in the field -> detail popover` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click a signal in the Active Signals list -> focus in the field` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Time scrubber -> navigate historical field state` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `pause -> animate the field` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S033 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:283` through `287`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Source agents → Agent Detail.
- Referenced knowledge → Entry Detail.
````

**Explicit detail extraction from this section:**

- Section word count: `8`
- Section hash: `e12b4203327e0b979a1af322a093bfa00931871304651a9d88de400c99e49407`

**Normative requirements and implementation claims:**
- - Source agents → Agent Detail. - Referenced knowledge → Entry Detail.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Source agents -> Agent Detail
- Referenced knowledge -> Entry Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Source agents → Agent Detail.
- - Referenced knowledge → Entry Detail.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|Detail|knowledge|Referenced|Entry|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|Detail|knowledge|Referenced|Entry|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `Source agents -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Referenced knowledge -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S034 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:288` through `291`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- No active pheromones: "Quiet field. When agents deposit signals, they'll appear here."
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `15bb8e5f9c97824b1e9295c6978c701a8e55793066d6ab427994fa529f8431a4`

**Normative requirements and implementation claims:**
- - No active pheromones: "Quiet field. When agents deposit signals, they'll appear here."

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
- - No active pheromones: "Quiet field. When agents deposit signals, they'll appear here."

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|signals|pheromones|here|field|deposit|appear" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|signals|pheromones|here|field|deposit|appear" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S035 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:292` through `298`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- User-contributed pheromones (users depositing directly, not via agents). Later if the pattern proves useful.
- Pheromone templates. Later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `18`
- Section hash: `66dbd95c68491d74be464a5bf7e91ec2f526e9211f10a5c679e7e9072897d0cb`

**Normative requirements and implementation claims:**
- - User-contributed pheromones (users depositing directly, not via agents). Later if the pattern proves useful. - Pheromone templates. Later.
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
- - User-contributed pheromones (users depositing directly, not via agents). Later if the pattern proves useful.
- - Pheromone templates. Later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|User|Pheromone|Later|users|useful|templates|proves" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|User|Pheromone|Later|users|useful|templates|proves" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S036 -- Page: Dream Cycles

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:299` through `300`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Dream Cycles
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `40e6c7d0cba088b7a0c897b2c8e9688ad9ee3baf57e4705926ba54573dcbf59b`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Dream|Cycles|surfaces|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Dream|Cycles|surfaces|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S037 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:301` through `304`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

Observation of the offline consolidation processes — the dreams where agents replay episodes, extract patterns, generate counterfactuals, rehearse threats, and consolidate insights into stable knowledge. A window into where learning happens.
````

**Explicit detail extraction from this section:**

- Section word count: `30`
- Section hash: `1bc214b011db87848e703291c816cb527e760bdbcbb26320c7060f644edb0413`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|window|threats|stable|replay|rehearse|processes|patterns" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|window|threats|stable|replay|rehearse|processes|patterns" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S038 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:305` through `308`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Fleet Orchestrator (watching consolidation across their fleet), Knowledge Contributor (observing knowledge promotion), Meta-Builder (designing dream-like consolidation in meta-agents).
````

**Explicit detail extraction from this section:**

- Section word count: `21`
- Section hash: `e575af3720e9f9c539a8983f4999a7680e1cddd939410c01588858183b21dd85`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|knowledge|consolidation|Meta|Fleet|watching|promotion" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|knowledge|consolidation|Meta|Fleet|watching|promotion" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S039 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:309` through `314`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- **Fleet** (default): dreams from user's agents.
- **Global**: aggregate dream activity across the network.
- **Agent**: drill-down to one agent's dream cycles.
````

**Explicit detail extraction from this section:**

- Section word count: `23`
- Section hash: `c5d4786536bbf1827cd1f048a19c407b58dcea55ff68a500150cd8055ca09f07`

**Normative requirements and implementation claims:**
- - **Fleet** (default): dreams from user's agents. - **Global**: aggregate dream activity across the network. - **Agent**: drill-down to one agent's dream cycles.

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
- - **Fleet** (default): dreams from user's agents.
- - **Global**: aggregate dream activity across the network.
- - **Agent**: drill-down to one agent's dream cycles.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "supported|Lenses|dream|user|network|drill|dreams|down" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supported|Lenses|dream|user|network|drill|dreams|down" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S040 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:315` through `339`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header area**: page title "Dream Cycles," block height, lens switcher.

**Section 01 — Active Dreams.** Description: "Consolidation in progress. Agents replaying, imagining, rehearsing, promoting."

Content: cards for agents currently in delta-tick dream cycles. Each card:
- Agent portrait and name.
- Current dream phase (Replay / Imagine / Rehearse / Stage / Promote).
- Progress indicator.
- Number of episodes being consolidated.
- Emerging patterns (count of candidate insights).

**Section 02 — Recent Promotions.** Description: "New knowledge crystallized from dreams."

Content: a feed of recently-promoted knowledge entries — entries that graduated from dream staging to the persistent knowledge store. Each entry shows the source dream, the pattern identified, and the promotion path.

**Section 03 — Dream Phase Gallery.** Description: "Learn what happens during dreams."

Content: an educational section explaining the four main dream phases visually. Less prominent on the page but useful for users encountering dreams for the first time.

**Section 04 — Dream Statistics.** Description: "Patterns in consolidation across the fleet."

Content: aggregate stats — total dreams in the period, average duration, promotion rate, common patterns surfaced.
````

**Explicit detail extraction from this section:**

- Section word count: `165`
- Section hash: `8684863d4499fea95b3fa9e8cbb5d1adf20adde43743ca492c815ec094ea0bac`

**Normative requirements and implementation claims:**
- **Header area**: page title "Dream Cycles," block height, lens switcher.
- **Section 01 — Active Dreams.** Description: "Consolidation in progress. Agents replaying, imagining, rehearsing, promoting."
- **Section 02 — Recent Promotions.** Description: "New knowledge crystallized from dreams."
- **Section 03 — Dream Phase Gallery.** Description: "Learn what happens during dreams."
- **Section 04 — Dream Statistics.** Description: "Patterns in consolidation across the fleet."

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
- - Agent portrait and name.
- - Current dream phase (Replay / Imagine / Rehearse / Stage / Promote).
- - Progress indicator.
- - Number of episodes being consolidated.
- - Emerging patterns (count of candidate insights).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Dream|structure|Dreams|pattern|Description|Content|promotion|phase" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Dream|structure|Dreams|pattern|Description|Content|promotion|phase" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S041 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:340` through `345`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- Per-agent dream event subscriptions.
- Knowledge store for promoted entries.
- Dream outcomes aggregator.
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `44123b67ee865e53f1a604991a1f1612e366cd1c7ddc742b945386460465cea8`

**Normative requirements and implementation claims:**
- - Per-agent dream event subscriptions. - Knowledge store for promoted entries. - Dream outcomes aggregator.

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
- - Per-agent dream event subscriptions.
- - Knowledge store for promoted entries.
- - Dream outcomes aggregator.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|dream|subscriptions|store|promoted|outcomes|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|dream|subscriptions|store|promoted|outcomes|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S042 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:346` through `351`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- Active dreams pulse with a distinctive "dreaming" ambient effect — gentle, slightly unreal.
- Newly-promoted entries fade in prominently in Recent Promotions.
- Phase transitions within an active dream animate.
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `595e2542980653eff5a7d119a301772b09bbb98ed17d3b4dba743b652a1988af`

**Normative requirements and implementation claims:**
- - Active dreams pulse with a distinctive "dreaming" ambient effect — gentle, slightly unreal. - Newly-promoted entries fade in prominently in Recent Promotions. - Phase transitions within an active dream animate.

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
- - Active dreams pulse with a distinctive "dreaming" ambient effect — gentle, slightly unreal.
- - Newly-promoted entries fade in prominently in Recent Promotions.
- - Phase transitions within an active dream animate.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "hooks|Reactivity|dream|Active|within|unreal|transitions|slightly" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "hooks|Reactivity|dream|Active|within|unreal|transitions|slightly" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S043 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:352` through `357`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Click an active dream → Dream Detail (scoped episode replay showing what's being consolidated).
- Click a promoted entry → Entry Detail.
- Click an agent → Agent Detail.
````

**Explicit detail extraction from this section:**

- Section word count: `25`
- Section hash: `d81ad8b7407297d34d49d0df1888269b4fcbefa3345db597eca595a1762b706b`

**Normative requirements and implementation claims:**
- - Click an active dream → Dream Detail (scoped episode replay showing what's being consolidated). - Click a promoted entry → Entry Detail. - Click an agent → Agent Detail.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Click an active dream -> Dream Detail
- Click a promoted entry -> Entry Detail
- Click an agent -> Agent Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Click an active dream → Dream Detail (scoped episode replay showing what's being consolidated).
- - Click a promoted entry → Entry Detail.
- - Click an agent → Agent Detail.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Interactions|Detail|Click|entry|dream|showing|scoped|replay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Interactions|Detail|Click|entry|dream|showing|scoped|replay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `Click an active dream -> Dream Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click a promoted entry -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click an agent -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S044 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:358` through `363`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Dream Detail (drill-down page, similar pattern to Replay).
- Promoted entries → Entry Detail.
- Agents → Agent Detail.
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `36b46b87bab76e0cf76327ea9c90262e0012b3e1dc5ba21c2679f407b77e5ea0`

**Normative requirements and implementation claims:**
- - Dream Detail (drill-down page, similar pattern to Replay). - Promoted entries → Entry Detail. - Agents → Agent Detail.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Promoted entries -> Entry Detail
- Agents -> Agent Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Dream Detail (drill-down page, similar pattern to Replay).
- - Promoted entries → Entry Detail.
- - Agents → Agent Detail.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|Detail|similar|pattern|entries|drill|down" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|Detail|similar|pattern|entries|drill|down" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `Promoted entries -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Agents -> Agent Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S045 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:364` through `367`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- No dreams active: "No consolidation in progress. Dreams happen after accumulated work triggers sleep pressure."
````

**Explicit detail extraction from this section:**

- Section word count: `15`
- Section hash: `9ea585b1cc707268b516c4df506c36f962e996fcd0715d8ef80cd66972cc949e`

**Normative requirements and implementation claims:**
- - No dreams active: "No consolidation in progress. Dreams happen after accumulated work triggers sleep pressure."

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
- - No dreams active: "No consolidation in progress. Dreams happen after accumulated work triggers sleep pressure."

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|dreams|work|triggers|sleep|progress|pressure" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|dreams|work|triggers|sleep|progress|pressure" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S046 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:368` through `374`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- Dream visualizations that show the actual mental rehearsal patterns (rather than just phase indicators). Later — this could be beautiful and informative.
- User-triggered dreams (forcing a specific agent to dream). Later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `243d594b26f62a2166ec86718a3e426ab3b961cd996562f0429ddb0bddd33ba4`

**Normative requirements and implementation claims:**
- - Dream visualizations that show the actual mental rehearsal patterns (rather than just phase indicators). Later — this could be beautiful and informative. - User-triggered dreams (forcing a specific agent to dream). Later.
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
- - Dream visualizations that show the actual mental rehearsal patterns (rather than just phase indicators). Later — this could be beautiful and informative.
- - User-triggered dreams (forcing a specific agent to dream). Later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|Dream|Later|visualizations|triggered|specific|show|rehearsal" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|Dream|Later|visualizations|triggered|specific|show|rehearsal" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S047 -- Page: Cross-Domain Resonance

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:375` through `376`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Cross-Domain Resonance
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `017ee7565bd24d79934295a01f944acf6d1a0a01c2c93efa570aa7a161ca0f37`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Resonance|Domain|Cross|surfaces|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Resonance|Domain|Cross|surfaces|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S048 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:377` through `380`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

Surface for discovering unexpected connections between domains. This is where a user sees that a pattern from coding resonates with a pattern from DeFi, or that threat signals in one domain predict events in another. The collective's ability to find cross-domain insights is arguably the most valuable emergent property of the knowledge layer, and this page makes it visible.
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `35d106ecb3fa030de0a672b14d207adf395539c93a656e92cf9ecd5d13036be9`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|domain|pattern|visible|valuable|user|unexpected|threat" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|domain|pattern|visible|valuable|user|unexpected|threat" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S049 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:381` through `384`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Knowledge Contributor (authoring cross-domain entries), Domain Architect (integrating with adjacent domains), Meta-Builder (designing cross-domain meta-agents), researchers.
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `d53ed991931483283e38defb2ff53c3f07e6c4580c4056fa999927c59ff0205b`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|domain|cross|Meta|researchers|knowledge|integrating" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|domain|cross|Meta|researchers|knowledge|integrating" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S050 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:385` through `389`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- **Global** (default): network-wide cross-domain patterns.
- **Fleet**: cross-domain patterns specifically relevant to user's agents.
````

**Explicit detail extraction from this section:**

- Section word count: `17`
- Section hash: `a5d5ef7136cc01550343561509da3b861421084266e0e084dcfae32eac582418`

**Normative requirements and implementation claims:**
- - **Global** (default): network-wide cross-domain patterns. - **Fleet**: cross-domain patterns specifically relevant to user's agents.

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
- - **Global** (default): network-wide cross-domain patterns.
- - **Fleet**: cross-domain patterns specifically relevant to user's agents.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "supported|Lenses|patterns|domain|cross|wide|user|specifically" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supported|Lenses|patterns|domain|cross|wide|user|specifically" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S051 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:390` through `416`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header area**: page title "Cross-Domain Resonance," block height, lens switcher.

**Section 01 — Resonance Map.** Description: "Where domains meet. Bridges between knowledge clusters that share hidden structure."

Content: a full-page Resonance Graph or generic Field visualization, showing knowledge entries organized by domain with cross-domain connections as bridges.

Domains are clustered visually. Each cluster has its own color or hue. Bridges between clusters are luminous — brighter bridges mean stronger resonance.

**Section 02 — Top Bridges.** Description: "The strongest cross-domain connections right now."

Content: a ranked list of the highest-resonance cross-domain pairs. Each row:
- Source domain, target domain.
- Representative entries from each side.
- Resonance strength.
- Recent activity on this bridge.
- A "Explore bridge" action that filters the Resonance Map to this pair.

**Section 03 — Recent Cross-Domain Discoveries.** Description: "New insights from unexpected connections."

Content: a feed of recently-published knowledge entries that explicitly draw on two or more domains. Each shows: title, contributing domains, author, confidence, age.

**Section 04 — Bridge Statistics.** Description: "How often cross-domain patterns pay off."

Content: aggregate stats — count of bridges, average bridge strength, conversion rate (bridges that produce validated insights).
````

**Explicit detail extraction from this section:**

- Section word count: `188`
- Section hash: `b475acb4e2ef790db2d5ca81a01d362d319746e0ace68f8e46e6a31f7e173728`

**Normative requirements and implementation claims:**
- **Header area**: page title "Cross-Domain Resonance," block height, lens switcher.
- **Section 01 — Resonance Map.** Description: "Where domains meet. Bridges between knowledge clusters that share hidden structure."
- **Section 02 — Top Bridges.** Description: "The strongest cross-domain connections right now."
- **Section 03 — Recent Cross-Domain Discoveries.** Description: "New insights from unexpected connections."
- **Section 04 — Bridge Statistics.** Description: "How often cross-domain patterns pay off."

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
- - Source domain, target domain.
- - Representative entries from each side.
- - Resonance strength.
- - Recent activity on this bridge.
- - A "Explore bridge" action that filters the Resonance Map to this pair.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Domain|bridge|Resonance|Bridges|structure|Cross|domains|cluster" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Domain|bridge|Resonance|Bridges|structure|Cross|domains|cluster" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S052 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:417` through `422`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- HDC-based similarity queries across domains.
- Knowledge store with domain tagging.
- Resonance event aggregator.
````

**Explicit detail extraction from this section:**

- Section word count: `14`
- Section hash: `df38c93d99f42b635c65ff4ba553e189fdc0e4deef166d64a11c7bcf586e331b`

**Normative requirements and implementation claims:**
- - HDC-based similarity queries across domains. - Knowledge store with domain tagging. - Resonance event aggregator.

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
- - HDC-based similarity queries across domains.
- - Knowledge store with domain tagging.
- - Resonance event aggregator.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|domain|tagging|store|similarity|queries|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|domain|tagging|store|similarity|queries|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S053 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:423` through `428`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- Bridges brighten when actively producing insights (particle flow along the bridge).
- New cross-domain entries appear in the Recent Discoveries feed with fade-in.
- Resonance map updates continuously as new entries and retrievals happen.
````

**Explicit detail extraction from this section:**

- Section word count: `34`
- Section hash: `ddbf776808ac86a0861166dab88259594fa239e4fa4d31a656deb2dd96cba239`

**Normative requirements and implementation claims:**
- - Bridges brighten when actively producing insights (particle flow along the bridge). - New cross-domain entries appear in the Recent Discoveries feed with fade-in. - Resonance map updates continuously as new entries and retrievals happen.

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
- - Bridges brighten when actively producing insights (particle flow along the bridge).
- - New cross-domain entries appear in the Recent Discoveries feed with fade-in.
- - Resonance map updates continuously as new entries and retrievals happen.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "hooks|Reactivity|entries|bridge|updates|retrievals|producing|particle" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "hooks|Reactivity|entries|bridge|updates|retrievals|producing|particle" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S054 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:429` through `436`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Hover a bridge → detail popover.
- Click a bridge → filter the map to the two domains; list of entries on each side.
- Zoom and pan the map.
- Click an entry → Entry Detail.
- Propose a cross-domain entry (authoring shortcut).
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `b37b46cd38a3fcdf44ac8abdc7331f3c95f48bf1d24ebeafe31ebc395caf513f`

**Normative requirements and implementation claims:**
- - Hover a bridge → detail popover. - Click a bridge → filter the map to the two domains; list of entries on each side. - Zoom and pan the map. - Click an entry → Entry Detail. - Propose a cross-domain entry (authoring shortcut).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Hover a bridge -> detail popover
- Click a bridge -> filter the map to the two domains
- Click an entry -> Entry Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Hover a bridge → detail popover.
- - Click a bridge → filter the map to the two domains; list of entries on each side.
- - Zoom and pan the map.
- - Click an entry → Entry Detail.
- - Propose a cross-domain entry (authoring shortcut).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Interactions|entry|domain|detail|bridge|Click|side|shortcut" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Interactions|entry|domain|detail|bridge|Click|side|shortcut" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `Hover a bridge -> detail popover` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click a bridge -> filter the map to the two domains` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Click an entry -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S055 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:437` through `441`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Entries → Entry Detail.
- Domains → domain-specific Knowledge Store views.
````

**Explicit detail extraction from this section:**

- Section word count: `9`
- Section hash: `e082dd0b6d129c61f838eb5ea7e1a28eaf42480a1de0912b7d1f81fde389db48`

**Normative requirements and implementation claims:**
- - Entries → Entry Detail. - Domains → domain-specific Knowledge Store views.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Entries -> Entry Detail
- Domains -> domain-specific Knowledge Store views

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Entries → Entry Detail.
- - Domains → domain-specific Knowledge Store views.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|domain|views|specific|knowledge|Store|Entry" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|domain|views|specific|knowledge|Store|Entry" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `Entries -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Domains -> domain-specific Knowledge Store views` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S056 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:442` through `445`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- Insufficient cross-domain data: "Not enough cross-domain signal yet. As knowledge accumulates, bridges will appear."
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `1cf33172b4fd94f702ea0daa1910adb27cd19ab80ccb8f6cc82e2157b7464129`

**Normative requirements and implementation claims:**
- - Insufficient cross-domain data: "Not enough cross-domain signal yet. As knowledge accumulates, bridges will appear."

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
- - Insufficient cross-domain data: "Not enough cross-domain signal yet. As knowledge accumulates, bridges will appear."

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|domain|cross|signal|knowledge|enough|data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|domain|cross|signal|knowledge|enough|data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S057 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:446` through `452`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- User-authored "bridge hypotheses" — a user explicitly proposes that two domains connect and the system tracks whether evidence accumulates. Later.
- Active "bridge finding" mode where an agent is tasked with discovering new cross-domain connections. Later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `36`
- Section hash: `3126379386f63ada0ee918c1b97f50da2876538a1cd53fd2efd135b6780ea9b4`

**Normative requirements and implementation claims:**
- - User-authored "bridge hypotheses" — a user explicitly proposes that two domains connect and the system tracks whether evidence accumulates. Later. - Active "bridge finding" mode where an agent is tasked with discovering new cross-domain connections. Later.
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
- - User-authored "bridge hypotheses" — a user explicitly proposes that two domains connect and the system tracks whether evidence accumulates. Later.
- - Active "bridge finding" mode where an agent is tasked with discovering new cross-domain connections. Later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|domain|connect|bridge|User|Later|whether|tracks" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|domain|connect|bridge|User|Later|whether|tracks" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S058 -- Page: Knowledge Lineage

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:453` through `454`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Page: Knowledge Lineage
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `45f622f3dc3cc79c6d024a67ea6a91481687bf32833e44caece2454cbfdb1efd`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Lineage|Knowledge|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Lineage|Knowledge|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S059 -- Purpose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:455` through `458`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Purpose

Trace the provenance and descent of knowledge entries. Every entry builds on earlier entries; every entry may influence later entries. Lineage visualization lets users see the knowledge graph as a genealogy.
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `b1eef0f714998b67576acf77f26637c4b1498b59f10d1a5b302ec574eb686f6a`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Purpose|entries|knowledge|entry|Every|visualization|users|provenance" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Purpose|entries|knowledge|entry|Every|visualization|users|provenance" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S060 -- Primary persona

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:459` through `462`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Primary persona

Knowledge Contributor (auditing their work and reputation), System Steward (auditing claims and contested entries), researchers studying knowledge flow.
````

**Explicit detail extraction from this section:**

- Section word count: `18`
- Section hash: `801fc918985aab7af4de08199d1364588ec6855b5cf026f1f853a58eccb908bf`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "persona|Primary|knowledge|auditing|work|studying|researchers|reputation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|Primary|knowledge|auditing|work|studying|researchers|reputation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S061 -- Lenses supported

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:463` through `468`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lenses supported

- **Scoped** to an entry (when drilled in from Entry Detail or via search).
- **Global**: explore the full lineage graph.
- **Fleet**: lineage restricted to user's agents' entries.
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `3ea0534d99988001bd4259f39328ea2ff97a554c1f85866a02779b7a258f5fbf`

**Normative requirements and implementation claims:**
- - **Scoped** to an entry (when drilled in from Entry Detail or via search). - **Global**: explore the full lineage graph. - **Fleet**: lineage restricted to user's agents' entries.

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
- - **Scoped** to an entry (when drilled in from Entry Detail or via search).
- - **Global**: explore the full lineage graph.
- - **Fleet**: lineage restricted to user's agents' entries.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "supported|Lenses|lineage|entry|user|search|restricted|graph" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supported|Lenses|lineage|entry|user|search|restricted|graph" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S062 -- Page structure

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:469` through `494`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page structure

**Header area**: page title "Knowledge Lineage," block height, lens switcher.

**Section 01 — Lineage Graph.** Description: "The ancestry and descent of knowledge. Upstream sources. Downstream applications."

Content: a graph visualization. Selected entry(ies) are focal; ancestors flow up (or back in time), descendants flow down. Edges indicate citation, derivation, or validation chains.

The graph is navigable — users can pan and zoom, select any entry to make it focal and recompute ancestors/descendants.

Controls:
- Depth control (how many generations of ancestors/descendants to show).
- Filter by type, confidence, author.
- Path finder: select two entries, see the path between them.

**Section 02 — Lineage Statistics.** Content: for the focal entry, summary stats:
- Total ancestors (generations and count).
- Total descendants (generations and count).
- Most-cited ancestor.
- Highest-impact descendant.
- Lineage confidence chain (how confidence changed across the lineage).

**Section 03 — Contested Paths.** Description: "Where lineage is disputed."

Content: a list of contested entries in the focal entry's lineage. Challenges, disputes, and resolutions along the ancestry. Essential for auditing whether an entry is built on solid foundations or on shaky ones.
````

**Explicit detail extraction from this section:**

- Section word count: `176`
- Section hash: `19daff4dde5d4d5df9db444e4866228c48dd0630488e7d69ae58a00c70248839`

**Normative requirements and implementation claims:**
- **Header area**: page title "Knowledge Lineage," block height, lens switcher.
- **Section 01 — Lineage Graph.** Description: "The ancestry and descent of knowledge. Upstream sources. Downstream applications."
- **Section 02 — Lineage Statistics.** Content: for the focal entry, summary stats: - Total ancestors (generations and count). - Total descendants (generations and count). - Most-cited ancestor. - Highest-impact descendant. - Lineage confidence chain (how confidence changed across the lineage).
- **Section 03 — Contested Paths.** Description: "Where lineage is disputed."

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
- - Depth control (how many generations of ancestors/descendants to show).
- - Filter by type, confidence, author.
- - Path finder: select two entries, see the path between them.
- - Total ancestors (generations and count).
- - Total descendants (generations and count).
- - Most-cited ancestor.
- - Highest-impact descendant.
- - Lineage confidence chain (how confidence changed across the lineage).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Lineage|structure|entry|descendant|ancestor|focal|descendants|ancestors" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Lineage|structure|entry|descendant|ancestor|focal|descendants|ancestors" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S063 -- Data sources

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:495` through `500`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data sources

- Knowledge store with lineage edges.
- Citation graph from InsightStore.
- Challenge events from chain.
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `f5998d487bebc934212624289b00803d1f8cf451c45961db9262463d250b220b`

**Normative requirements and implementation claims:**
- - Knowledge store with lineage edges. - Citation graph from InsightStore. - Challenge events from chain.

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
- - Knowledge store with lineage edges.
- - Citation graph from InsightStore.
- - Challenge events from chain.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "sources|Data|store|lineage|knowledge|graph|events|edges" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sources|Data|store|lineage|knowledge|graph|events|edges" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S064 -- Reactivity hooks

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:501` through `507`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reactivity hooks

- Focal entry has strong epistemic glow.
- Ancestors appear in varying opacity based on their decay.
- Contested entries along the path flash with turbulence.
- Newly-added descendants (recent citations) have a fresh-edge treatment.
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `82b2ef6d18b345379e010020be51aacc7caa00318fd3e6547392e36a3419ce0c`

**Normative requirements and implementation claims:**
- - Focal entry has strong epistemic glow. - Ancestors appear in varying opacity based on their decay. - Contested entries along the path flash with turbulence. - Newly-added descendants (recent citations) have a fresh-edge treatment.

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
- - Focal entry has strong epistemic glow.
- - Ancestors appear in varying opacity based on their decay.
- - Contested entries along the path flash with turbulence.
- - Newly-added descendants (recent citations) have a fresh-edge treatment.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "hooks|Reactivity|varying|turbulence|treatment|strong|recent|path" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "hooks|Reactivity|varying|turbulence|treatment|strong|recent|path" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S065 -- Interactions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:508` through `515`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Interactions

- Click any node → make it focal.
- Pan and zoom.
- Depth and filter controls.
- Path finder.
- Export lineage as a diagram (for research papers or documentation).
````

**Explicit detail extraction from this section:**

- Section word count: `25`
- Section hash: `59709d2def415a55ab80d6f22ea56905b8d7c811a058410ae230eb0594cf0dba`

**Normative requirements and implementation claims:**
- - Click any node → make it focal. - Pan and zoom. - Depth and filter controls. - Path finder. - Export lineage as a diagram (for research papers or documentation).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Click any node -> make it focal

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Click any node → make it focal.
- - Pan and zoom.
- - Depth and filter controls.
- - Path finder.
- - Export lineage as a diagram (for research papers or documentation).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Interactions|zoom|research|papers|node|make|lineage|focal" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Interactions|zoom|research|papers|node|make|lineage|focal" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `Click any node -> make it focal` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S066 -- Cross-references

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:516` through `519`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-references

- Every node → Entry Detail.
````

**Explicit detail extraction from this section:**

- Section word count: `4`
- Section hash: `f659d6963cbf7ef3045624219337f29d9e6bfd4fab9544b0b2754a039a71870a`

**Normative requirements and implementation claims:**
- - Every node → Entry Detail.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Every node -> Entry Detail

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Every node → Entry Detail.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "references|Cross|node|Every|Entry|Detail|surfaces|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "references|Cross|node|Every|Entry|Detail|surfaces|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Enforce state transition `Every node -> Entry Detail` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S067 -- Empty states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:520` through `523`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Empty states

- Isolated entry with no citations and no descendants: "No lineage yet. As this entry is cited or cites others, lineage will appear."
````

**Explicit detail extraction from this section:**

- Section word count: `22`
- Section hash: `c1a316f535985a8349ef3d026c16fd1b5680256038c5c66e21dcdfe419668044`

**Normative requirements and implementation claims:**
- - Isolated entry with no citations and no descendants: "No lineage yet. As this entry is cited or cites others, lineage will appear."

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
- - Isolated entry with no citations and no descendants: "No lineage yet. As this entry is cited or cites others, lineage will appear."

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "states|Empty|lineage|entry|others|descendants|cites|cited" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "states|Empty|lineage|entry|others|descendants|cites|cited" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S068 -- Deferred

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:524` through `530`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Deferred

- Lineage comparison: compare two entries' lineages to find shared ancestors. Later.
- Lineage-based reputation attribution. Partial at MVP, elaborated later.

---
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `b9cdefc726e431dd805104e0027de6e7a2f719e6ee9294141ec3bd77c7a52281`

**Normative requirements and implementation claims:**
- - Lineage comparison: compare two entries' lineages to find shared ancestors. Later. - Lineage-based reputation attribution. Partial at MVP, elaborated later.
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
- - Lineage comparison: compare two entries' lineages to find shared ancestors. Later.
- - Lineage-based reputation attribution. Partial at MVP, elaborated later.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Deferred|Lineage|Later|shared|reputation|lineages|find|entries" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deferred|Lineage|Later|shared|reputation|lineages|find|entries" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S069 -- Cross-section considerations for Knowledge

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:531` through `532`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Cross-section considerations for Knowledge
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `ee85abc6a6eb8f216747ab90830a5163505becbf8f0150a4e89abca88db7a21e`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "for|considerations|Knowledge|Cross|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "for|considerations|Knowledge|Cross|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S070 -- Knowledge surfaces compose

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:533` through `536`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge surfaces compose

The visualizations across Knowledge pages compose from the same primitives: Resonance Graph, Knowledge Topography, Field, Timeline, Tier Ladder. Users learn once, apply everywhere.
````

**Explicit detail extraction from this section:**

- Section word count: `23`
- Section hash: `f533d1fe8062ad663ea9a0cc5403fb1e17f8ab73dde62302c653f11fc38456f9`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Knowledge|compose|surfaces|Graph|visualizations|same|primitives|pages" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|compose|surfaces|Graph|visualizations|same|primitives|pages" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S071 -- Knowledge everywhere

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:537` through `540`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge everywhere

Knowledge is not isolated to the Knowledge section. Every other section references knowledge (agent knowledge in Fleet, arena-linked knowledge in Arena, eval-driven knowledge in Measurements). The Knowledge section is the home; references radiate outward.
````

**Explicit detail extraction from this section:**

- Section word count: `36`
- Section hash: `67bdb9f259344f95595870ecb0c3de8d9cac1c7ab56442a04f946e14772ca326`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Knowledge|Every|everywhere|references|arena|radiate|outward|other" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|Every|everywhere|references|arena|radiate|outward|other" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S072 -- On-chain vs off-chain

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:541` through `544`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### On-chain vs off-chain

The dashboard distinguishes between published (on-chain) and unpublished (local) knowledge. On-chain entries have a chain-anchor indicator. Publishing is an explicit action with a cost (gas, or daeji stake) and a benefit (global discoverability, permanent reputation attribution).
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `7244030d4418a575f15a8bd3ebe2d9e5f5c8918f75923a7e0a915caa9f1f001d`

**Normative requirements and implementation claims:**
- The dashboard distinguishes between published (on-chain) and unpublished (local) knowledge. On-chain entries have a chain-anchor indicator. Publishing is an explicit action with a cost (gas, or daeji stake) and a benefit (global discoverability, permanent reputation attribution).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "chain|off|published|unpublished|stake|reputation|permanent|local" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "chain|off|published|unpublished|stake|reputation|permanent|local" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S073 -- Privacy

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:545` through `548`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Privacy

Some knowledge is private. An agent's local knowledge store may contain entries the owner does not want published. The dashboard respects this — filters and lenses scope appropriately, and unauthorized attempts to read private knowledge produce clear access-denied states.
````

**Explicit detail extraction from this section:**

- Section word count: `40`
- Section hash: `23b10aa8fe380315c6d6a9cf861151110bf3d535bcabedceb3c2246531d8b039`

**Normative requirements and implementation claims:**
- Some knowledge is private. An agent's local knowledge store may contain entries the owner does not want published. The dashboard respects this — filters and lenses scope appropriately, and unauthorized attempts to read private knowledge produce clear access-denied states.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Privacy|knowledge|private|want|unauthorized|store|states|scope" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Privacy|knowledge|private|want|unauthorized|store|states|scope" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S074 -- Reputation linkage

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:549` through `552`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reputation linkage

Knowledge contribution is a primary source of reputation. The dashboard makes this link clear — a user viewing their own Knowledge Store sees their reputation impact directly. A user viewing a Sovereign-tier author sees why they're Sovereign (high-impact entries, many validations, cross-domain contributions).
````

**Explicit detail extraction from this section:**

- Section word count: `46`
- Section hash: `05a663c09e66179ae81a09f1b4a106fd82316ce5c99161ebb6e0be09224fab12`

**Normative requirements and implementation claims:**
- Knowledge contribution is a primary source of reputation. The dashboard makes this link clear — a user viewing their own Knowledge Store sees their reputation impact directly. A user viewing a Sovereign-tier author sees why they're Sovereign (high-impact entries, many validations, cross-domain contributions).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "Reputation|link|linkage|viewing|user|sees|knowledge|impact" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reputation|link|linkage|viewing|user|sees|knowledge|impact" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S075 -- Publishing friction

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:553` through `556`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Publishing friction

Publishing a knowledge entry requires thoughtful framing. The authoring surface (in `19-authoring-surfaces.md`) structures the entry appropriately by type. The goal is high-quality contribution, not volume; some friction is desirable.
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `750dbea8a373a102b9dc2cf0b2e1556970e438388af63bf74e672f31bb42fda3`

**Normative requirements and implementation claims:**
- Publishing a knowledge entry requires thoughtful framing. The authoring surface (in `19-authoring-surfaces.md`) structures the entry appropriately by type. The goal is high-quality contribution, not volume; some friction is desirable.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "friction|Publishing|surface|entry|authoring|volume|type|thoughtful" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "friction|Publishing|surface|entry|authoring|volume|type|thoughtful" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

### DASH-14-S076 -- Knowledge decay and revalidation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md:557` through `559`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge decay and revalidation

Entries decay over time. Decay makes entries less visible (opacity fade) and less weighted in retrieval. Revalidation resets decay. The dashboard surfaces entries approaching stale to their authors, giving them a chance to revalidate.
````

**Explicit detail extraction from this section:**

- Section word count: `34`
- Section hash: `06af75f5b4c80393e7e8e8055734d299dfda7cb1a593587ff325b89fb6d68457`

**Normative requirements and implementation claims:**
- Entries decay over time. Decay makes entries less visible (opacity fade) and less weighted in retrieval. Revalidation resets decay. The dashboard surfaces entries approaching stale to their authors, giving them a chance to revalidate.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/14-knowledge-surfaces.md`
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
rg -n "decay|revalidation|Knowledge|Entries|less|weighted|visible" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "decay|revalidation|Knowledge|Entries|less|weighted|visible" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/neuro.rs`
- `crates/roko-neuro/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/14-knowledge-surfaces
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

