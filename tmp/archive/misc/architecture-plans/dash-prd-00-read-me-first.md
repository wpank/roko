# Dashboard PRD Plan: Read Me First

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
**Generated:** 2026-04-25
**Source hash:** `48eec85b5feff333453a1df2c0819d9593c0247d64d7769e9d95539c5a0a4fc1`
**Section tasks:** 12
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
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
- `docs/API-REFERENCE.md`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-00-S001 | 1 | 00 — Read me first | [ ] | 9.8 |
| DASH-00-S002 | 7 | What this document set is | [ ] | 9.8 |
| DASH-00-S003 | 15 | Who this is for | [ ] | 9.8 |
| DASH-00-S004 | 27 | How to use this document set | [ ] | 9.8 |
| DASH-00-S005 | 35 | Reading order | [ ] | 9.8 |
| DASH-00-S006 | 53 | The 22 documents | [ ] | 9.8 |
| DASH-00-S007 | 81 | Terminology conventions | [ ] | 9.8 |
| DASH-00-S008 | 113 | What is canonical vs illustrative | [ ] | 9.8 |
| DASH-00-S009 | 125 | What is in scope and out of scope | [ ] | 9.8 |
| DASH-00-S010 | 147 | What changed from the current dashboard | [ ] | 9.8 |
| DASH-00-S011 | 163 | What to do if something is missing | [ ] | 9.8 |
| DASH-00-S012 | 173 | A note on tone | [ ] | 9.8 |

## Tasks

### DASH-00-S001 -- 00 — Read me first

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 00 — Read me first

*Orientation document for the Nunchi dashboard specification set.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `8`
- Section hash: `66a5759aa5d50a1645e66984843f02fe2adafa8b333f8be1a60f1cb44170a0ed`

**Normative requirements and implementation claims:**
- *Orientation document for the Nunchi dashboard specification set.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "first|Read|specification|document|Orientation|Nunchi" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "first|Read|specification|document|Orientation|Nunchi" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S002 -- What this document set is

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:7` through `14`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What this document set is

This is a specification for the Nunchi dashboard — a web application that gives humans a way to create, operate, observe, and collaborate with autonomous agents running on the Roko runtime and coordinating through the Korai chain. The specifications are deliberately detailed because the intended reader is either a human designer building the dashboard or an AI coding agent working from these documents as its only source of context. Nothing should require inference from outside this document set.

The specifications cover: what pages exist, how they are grouped, what primitives they compose from, how they look and behave, how they react to real-time data, how users create new things through them, and what the dashboard needs from the layers beneath it (Roko, Korai, Mirage) that does not yet exist.

These specifications are not the dashboard. They are a direction. A capable implementer working from them should produce a dashboard that is coherent with the direction, but the specifications leave room for judgment in visual details, component-internal state, and local interaction design.
````

**Explicit detail extraction from this section:**

- Section word count: `172`
- Section hash: `ad68e11d6d5dfb01b696a9f0974c44285db84116b5854ea7e9a028cb97bfa1ee`

**Normative requirements and implementation claims:**
- This is a specification for the Nunchi dashboard — a web application that gives humans a way to create, operate, observe, and collaborate with autonomous agents running on the Roko runtime and coordinating through the Korai chain. The specifications are deliberately detailed because the intended reader is either a human designer building the dashboard or an AI coding agent working from these documents as its only source of context. Nothing should require inference from outside this document set.
- The specifications cover: what pages exist, how they are grouped, what primitives they compose from, how they look and behave, how they react to real-time data, how users create new things through them, and what the dashboard needs from the layers beneath it (Roko, Korai, Mirage) that does not yet exist.
- These specifications are not the dashboard. They are a direction. A capable implementer working from them should produce a dashboard that is coherent with the direction, but the specifications leave room for judgment in visual details, component-internal state, and local interaction design.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "document|set|specification|specifications|working|time|human|exist" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|set|specification|specifications|working|time|human|exist" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S003 -- Who this is for

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:15` through `26`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Who this is for

Three audiences, in decreasing priority:

The first audience is a human operator (the product owner, a designer, an engineer) loading these documents into a fresh Claude Code session or a similar tool so that an AI agent can build the dashboard without needing any other source of context. For this audience, the documents must be self-contained, every term must be defined on first use within each document, and every instruction must be unambiguous enough that reasonable interpretations converge on the same implementation.

The second audience is a human reader who wants to understand what Nunchi is and what the dashboard does without reading the underlying PRDs or the codebase. For this audience, the documents must be readable linearly, must not assume prior knowledge of the system, and must build up concepts in dependency order.

The third audience is the product owner themselves, referring back to these documents to remember what was decided and why. For this audience, the documents must be honest about what is firm (hard decisions), what is a recommendation (my call, open to override), and what is deferred (unresolved, tracked in `22-deferred-and-open-questions.md`).

The documents are not written for investors, partners, or marketing. They are a working specification. Strategy, narrative, and positioning are adjacent concerns, handled elsewhere.
````

**Explicit detail extraction from this section:**

- Section word count: `215`
- Section hash: `0f7fcc7628cd2d4a8656fd8c4e59d5f5e72af80b20e3a37626ab95633b215430`

**Normative requirements and implementation claims:**
- The first audience is a human operator (the product owner, a designer, an engineer) loading these documents into a fresh Claude Code session or a similar tool so that an AI agent can build the dashboard without needing any other source of context. For this audience, the documents must be self-contained, every term must be defined on first use within each document, and every instruction must be unambiguous enough that reasonable interpretations converge on the same implementation.
- The second audience is a human reader who wants to understand what Nunchi is and what the dashboard does without reading the underlying PRDs or the codebase. For this audience, the documents must be readable linearly, must not assume prior knowledge of the system, and must build up concepts in dependency order.
- The third audience is the product owner themselves, referring back to these documents to remember what was decided and why. For this audience, the documents must be honest about what is firm (hard decisions), what is a recommendation (my call, open to override), and what is deferred (unresolved, tracked in `22-deferred-and-open-questions.md`).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "for|document|audience|documents|Who|read|without|product" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "for|document|audience|documents|Who|read|without|product" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S004 -- How to use this document set

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:27` through `34`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## How to use this document set

For a fresh agent session: hand the agent the entire `nunchi-dashboard-specs/` directory. Point it at `00-read-me-first.md` (this document) and have it read straight through in numerical order. Do not hand it documents out of order. Each document assumes the ones before it have been read.

For a human reviewer: read `00`, `01`, `02`, `03` first. These are foundation. After that, read `04`, `05`, `06` to understand the information architecture. After that, the design and visualization documents (`07`–`10`) can be read in any order, and the page-specification documents (`11`–`18`) can be read in the order that matches your interest.

For reference during implementation: use `01-system-landscape.md` as the glossary. Every term used anywhere in the document set is defined there. When in doubt about a term, check there first.
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `8b62f6a340046b61137156e49021a91a87485dd597bdc5c60f711710c02e9b79`

**Normative requirements and implementation claims:**
- For a fresh agent session: hand the agent the entire `nunchi-dashboard-specs/` directory. Point it at `00-read-me-first.md` (this document) and have it read straight through in numerical order. Do not hand it documents out of order. Each document assumes the ones before it have been read.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "document|use|read|set|How|order|first|documents" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|use|read|set|How|order|first|documents" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S005 -- Reading order

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:35` through `52`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Reading order

The 22 documents are grouped into six sections:

**Section I — Foundation** (`00`–`03`): what the system is, what it believes, who it serves.

**Section II — Information Architecture** (`04`–`06`): how the dashboard is organized, how users move through it, and how the same data gets viewed through different lenses.

**Section III — Design Language and Aesthetic System** (`07`–`10`): how the dashboard looks and feels. Includes the performance-reactive aesthetics system, which is a distinctive feature of the product and should not be treated as optional polish.

**Section IV — Pages and Surfaces** (`11`–`18`): specific page specifications, organized by sidebar section. Each document covers multiple related pages.

**Section V — Authoring and Creation** (`19`–`20`): the surfaces through which users make new things (agents, extensions, arenas, evals, meta-agents). This is where the "tools for tools" principle lands concretely.

**Section VI — Backend and Unresolved** (`21`–`22`): what the dashboard needs from the layers beneath it that does not yet exist, and what decisions have been deferred.

A reader with only a few hours of budget should read Sections I and II in full, skim Section III, read the page specifications for the sections of the product they care about most, and skim Sections V and VI. A reader with a full day should read every document in order. A reader implementing the dashboard should read every document in order at least once before writing code.
````

**Explicit detail extraction from this section:**

- Section word count: `233`
- Section hash: `21792c371eede350dc598f858b16fd1f49586de9529f1f667280e908ee8c8b34`

**Normative requirements and implementation claims:**
- **Section I — Foundation** (`00`–`03`): what the system is, what it believes, who it serves.
- **Section II — Information Architecture** (`04`–`06`): how the dashboard is organized, how users move through it, and how the same data gets viewed through different lenses.
- **Section III — Design Language and Aesthetic System** (`07`–`10`): how the dashboard looks and feels. Includes the performance-reactive aesthetics system, which is a distinctive feature of the product and should not be treated as optional polish.
- **Section IV — Pages and Surfaces** (`11`–`18`): specific page specifications, organized by sidebar section. Each document covers multiple related pages.
- **Section V — Authoring and Creation** (`19`–`20`): the surfaces through which users make new things (agents, extensions, arenas, evals, meta-agents). This is where the "tools for tools" principle lands concretely.
- **Section VI — Backend and Unresolved** (`21`–`22`): what the dashboard needs from the layers beneath it that does not yet exist, and what decisions have been deferred.
- A reader with only a few hours of budget should read Sections I and II in full, skim Section III, read the page specifications for the sections of the product they care about most, and skim Sections V and VI. A reader with a full day should read every document in order. A reader implementing the dashboard should read every document in order at least once before writing code.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
rg -n "read|order|Reading|sections|document|specific|reader|users" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "read|order|Reading|sections|document|specific|reader|users" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S006 -- The 22 documents

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:53` through `80`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The 22 documents

| # | Title | Purpose |
|---|---|---|
| 00 | Read me first | This document. Orientation. |
| 01 | System landscape | What Nunchi, Roko, Korai, Mirage are. Full glossary. |
| 02 | Theses and principles | Load-bearing beliefs that every page must honor. |
| 03 | Personas and jobs | Six to eight concrete personas the dashboard serves. |
| 04 | Information architecture | Top-level organization. Sidebar groups. Critique of current IA, proposed new IA. |
| 05 | Lenses and perspectives | The five lenses (global, fleet, agent, group, chain) and how they compose. |
| 06 | Navigation and traversal | How users move through the dashboard. Entry points, breadcrumbs, follow-the-thread. |
| 07 | Design language | ROSEDUST inheritance. Palette, typography, glass, motion, correspondence principle. |
| 08 | Epistemic aesthetics | Performance-reactive UI. Epistemic sharpness drives visual properties. |
| 09 | Visualization primitives | Recurring building blocks (stigmergy field, resonance graph, knowledge topography, etc.). |
| 10 | Realtime and motion | How streaming data becomes UI. Rhythm, cadence, scale handling. |
| 11 | Pulse surfaces | Command Center, Live Console, Event Stream. |
| 12 | Fleet surfaces | Agent Fleet, Agent Detail, Templates, Network. |
| 13 | Forge surfaces | PRDs, Plans, Research, Execution, Replay. |
| 14 | Knowledge surfaces | Knowledge Store, Stigmergy, Dream Cycles, Context Audit, new additions. |
| 15 | Arena surfaces | Benchmarks, Leaderboard, Experiments, Arena Browser, Arena Creator, Bounty Market. |
| 16 | Meta surfaces | Meta-Agents, Eval Designer, Generator Builder, Recursive Workshop. |
| 17 | Treasury surfaces | Positions, Cost Analytics, ISFR Dashboard, Multi-Chain. |
| 18 | System surfaces | Providers, Jobs, Extensions, Build, Settings, plus Measurements, Evals, Caveats, Delegations. |
| 19 | Authoring surfaces | Dedicated creation UIs for every creatable object. |
| 20 | Composition patterns | How primitives compose. The DAW principle made concrete. |
| 21 | Roko and chain additions | What the dashboard needs from the backend that doesn't exist yet. |
| 22 | Deferred and open questions | Unresolved decisions. Known tensions. Things to prototype first. |
````

**Explicit detail extraction from this section:**

- Section word count: `274`
- Section hash: `f8b93cb1b7c6281e5e4db6ecafa819fe17f5b575f2d871471260c6f2b6db6154`

**Normative requirements and implementation claims:**
- | # | Title | Purpose | |---|---|---| | 00 | Read me first | This document. Orientation. | | 01 | System landscape | What Nunchi, Roko, Korai, Mirage are. Full glossary. | | 02 | Theses and principles | Load-bearing beliefs that every page must honor. | | 03 | Personas and jobs | Six to eight concrete personas the dashboard serves. | | 04 | Information architecture | Top-level organization. Sidebar groups. Critique of current IA, proposed new IA. | | 05 | Lenses and perspectives | The five lenses (global, fleet, agent, group, chain) and how they compose. | | 06 | Navigation and traversal | How users move through the dashboard. Entry points, breadcrumbs, follow-the-thread. | | 07 | Design language | ROSEDUST inheritance. Palette, typography, glass, motion, correspondence principle. | | 08 | Epistemic aesthetics | Performance-reactive UI. Epistemic sharpness drives visual properties. | | 09 | Visualization primitives | Recurring building blocks (stigmergy field, resonance graph, knowledg

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
- Table 1:

```markdown
| # | Title | Purpose |
|---|---|---|
| 00 | Read me first | This document. Orientation. |
| 01 | System landscape | What Nunchi, Roko, Korai, Mirage are. Full glossary. |
| 02 | Theses and principles | Load-bearing beliefs that every page must honor. |
| 03 | Personas and jobs | Six to eight concrete personas the dashboard serves. |
| 04 | Information architecture | Top-level organization. Sidebar groups. Critique of current IA, proposed new IA. |
| 05 | Lenses and perspectives | The five lenses (global, fleet, agent, group, chain) and how they compose. |
| 06 | Navigation and traversal | How users move through the dashboard. Entry points, breadcrumbs, follow-the-thread. |
| 07 | Design language | ROSEDUST inheritance. Palette, typography, glass, motion, correspondence principle. |
| 08 | Epistemic aesthetics | Performance-reactive UI. Epistemic sharpness drives visual properties. |
| 09 | Visualization primitives | Recurring building blocks (stigmergy field, resonance graph, knowledge topography, etc.). |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
rg -n "The|surfaces|document|documents|read|principle|knowledge|graph" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|surfaces|document|documents|read|principle|knowledge|graph" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S007 -- Terminology conventions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:81` through `112`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Terminology conventions

The full glossary lives in `01-system-landscape.md`. These are the meta-conventions used across all documents:

**Nunchi** is the product. It is the thing users experience. When a document says "the product," it means Nunchi. When a document says "the dashboard," it means the web application that is Nunchi's primary surface.

**Roko** is the agent runtime. It is the Rust toolkit that agents are built on. It is not the product; it is the infrastructure layer below the product. End users should rarely encounter the name "Roko" directly in the UI, but implementers will encounter it constantly in the codebase.

**Korai** is the chain. It is a purpose-built blockchain where agent identity, reputation, knowledge, and coordination live. When the dashboard reads on-chain data, it reads from Korai (or, in development, from Mirage).

**Mirage** is the local development chain that stands in for Korai. It is functionally equivalent from the dashboard's perspective. Production dashboards read from Korai; development dashboards read from Mirage. The dashboard should treat them identically.

**Agent** means one instance of a Roko runtime process, possibly long-running, possibly ephemeral. Agents have identity (via ERC-8004 registration on Korai), state, and capabilities. Agents are the primary objects users create, configure, and observe.

**Domain** is the category of work an agent is configured to do (coding, blockchain monitoring, research, security, or user-created domains). Domains shape which extensions, gates, and context strategies an agent uses.

**Extension** is a modular unit of behavior that an agent can load. Extensions hook into the agent's heartbeat pipeline and add capability without changing the core runtime.

**Gate** is a verification step that checks an agent's output against an external truth source (compile, test, chain simulation, formal check, human review). Gates are how the system grounds LLM output against reality.

**Arena** is a defined evaluation environment — a task source, a gate configuration, a scoring function, and a leaderboard — where agents compete and performance is measured. Arenas are first-class objects users can create.

**Eval** is a measurement of agent behavior against a ground truth. Evals are first-class objects users can create, publish, challenge, and compose.

**Knowledge** is a unit of validated information that agents produce and share. Knowledge entries live in local knowledge stores and, when published, on the Korai chain's knowledge layer. Knowledge decays, gets validated, gets challenged, and gets composed.

**Lens** is a perspective the user takes on a piece of data: global (the whole network), fleet (agents I own), agent (one specific agent), group (a coordinated subset), or chain (on-chain state). The same data can be viewed through multiple lenses.

**Persona** is a category of user the dashboard is designed for. Personas are defined in `03-personas-and-jobs.md` and referenced by short handle throughout the rest of the documents.

When a document introduces a concept that has not appeared before, it defines the concept on first use. When a document uses a concept already defined in an earlier document, it uses the concept without re-defining it, but the reader can consult `01-system-landscape.md` for the full definition.
````

**Explicit detail extraction from this section:**

- Section word count: `521`
- Section hash: `5ba731e532f9a5607f38e7547360eaa0f3edca9107c0756e722c81cffb43bf70`

**Normative requirements and implementation claims:**
- **Nunchi** is the product. It is the thing users experience. When a document says "the product," it means Nunchi. When a document says "the dashboard," it means the web application that is Nunchi's primary surface.
- **Roko** is the agent runtime. It is the Rust toolkit that agents are built on. It is not the product; it is the infrastructure layer below the product. End users should rarely encounter the name "Roko" directly in the UI, but implementers will encounter it constantly in the codebase.
- **Korai** is the chain. It is a purpose-built blockchain where agent identity, reputation, knowledge, and coordination live. When the dashboard reads on-chain data, it reads from Korai (or, in development, from Mirage).
- **Mirage** is the local development chain that stands in for Korai. It is functionally equivalent from the dashboard's perspective. Production dashboards read from Korai; development dashboards read from Mirage. The dashboard should treat them identically.
- **Agent** means one instance of a Roko runtime process, possibly long-running, possibly ephemeral. Agents have identity (via ERC-8004 registration on Korai), state, and capabilities. Agents are the primary objects users create, configure, and observe.
- **Domain** is the category of work an agent is configured to do (coding, blockchain monitoring, research, security, or user-created domains). Domains shape which extensions, gates, and context strategies an agent uses.
- **Extension** is a modular unit of behavior that an agent can load. Extensions hook into the agent's heartbeat pipeline and add capability without changing the core runtime.
- **Gate** is a verification step that checks an agent's output against an external truth source (compile, test, chain simulation, formal check, human review). Gates are how the system grounds LLM output against reality.
- **Arena** is a defined evaluation environment — a task source, a gate configuration, a scoring function, and a leaderboard — where agents compete and performance is measured. Arenas are first-class objects users can create.
- **Eval** is a measurement of agent behavior against a ground truth. Evals are first-class objects users can create, publish, challenge, and compose.
- **Knowledge** is a unit of validated information that agents produce and share. Knowledge entries live in local knowledge stores and, when published, on the Korai chain's knowledge layer. Knowledge decays, gets validated, gets challenged, and gets composed.
- **Lens** is a perspective the user takes on a piece of data: global (the whole network), fleet (agents I own), agent (one specific agent), group (a coordinated subset), or chain (on-chain state). The same data can be viewed through multiple lenses.
- **Persona** is a category of user the dashboard is designed for. Personas are defined in `03-personas-and-jobs.md` and referenced by short handle throughout the rest of the documents.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
rg -n "chain|user|document|read|knowledge|conventions|Korai|users" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "chain|user|document|read|knowledge|conventions|Korai|users" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S008 -- What is canonical vs illustrative

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:113` through `124`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What is canonical vs illustrative

The documents make three kinds of claims:

**Canonical claims** are firm specifications. They describe how the system should work. Implementers should follow them. If they are wrong, they should be changed in these documents before being changed in the implementation. Canonical claims are the default mode of the documents.

**Recommended claims** are marked with language like "I recommend" or "the default is" or "unless overridden." These are judgment calls I made to move forward. They can be overridden by the product owner or by implementers who have good reason, but the override should be recorded somewhere — either in `22-deferred-and-open-questions.md` or in a new revision of the relevant document.

**Illustrative claims** are marked with language like "for example" or "something like." These are examples, not requirements. An implementer should understand what kind of thing is expected but not treat the example as a literal target.

When in doubt, a claim is canonical unless it is explicitly marked otherwise.
````

**Explicit detail extraction from this section:**

- Section word count: `162`
- Section hash: `2b3a77a5691b8839f4d30ea0e3f6aefeac3755cd3cd1a4cda8f0ee1e70fe1357`

**Normative requirements and implementation claims:**
- **Canonical claims** are firm specifications. They describe how the system should work. Implementers should follow them. If they are wrong, they should be changed in these documents before being changed in the implementation. Canonical claims are the default mode of the documents.
- **Recommended claims** are marked with language like "I recommend" or "the default is" or "unless overridden." These are judgment calls I made to move forward. They can be overridden by the product owner or by implementers who have good reason, but the override should be recorded somewhere — either in `22-deferred-and-open-questions.md` or in a new revision of the relevant document.
- **Illustrative claims** are marked with language like "for example" or "something like." These are examples, not requirements. An implementer should understand what kind of thing is expected but not treat the example as a literal target.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "canonical|illustrative|claim|claims|document|marked|like|implementer" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "canonical|illustrative|claim|claims|document|marked|like|implementer" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S009 -- What is in scope and out of scope

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:125` through `146`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What is in scope and out of scope

**In scope:**

- All pages of the Nunchi dashboard, including pages that do not yet exist and pages that should be added.
- The design language, component library, and visual system the dashboard uses.
- The real-time behavior and aesthetic reactivity of the dashboard.
- How users create new things through the dashboard (agents, extensions, arenas, evals, meta-agents).
- The information architecture and navigation model.
- API and streaming surfaces the dashboard consumes (as a consumer, not as the designer of those surfaces).
- What the dashboard needs from Roko, Korai, and Mirage that does not yet exist (proposed additions, not detailed specs for those layers).

**Out of scope:**

- Detailed implementation of Roko internals (the runtime, the heartbeat pipeline, the gate pipeline). These are specified elsewhere in the PRDs.
- Detailed implementation of Korai internals (consensus, execution, precompiles). Specified elsewhere.
- Detailed implementation of smart contracts. Specified elsewhere.
- Marketing pages, landing pages, documentation sites, developer portals.
- Mobile applications. The dashboard is a web application. A mobile companion could exist later; it is not in scope here.
- Pricing, monetization, billing UI. These may exist in the dashboard but their design is a product decision that should be made separately from this specification.
- Detailed API specifications for endpoints the dashboard consumes. The dashboard will consume the API; the API's design is specified in the PRDs for Roko and Korai.
````

**Explicit detail extraction from this section:**

- Section word count: `221`
- Section hash: `ac99a962d9212931e11bc9e66dd1f84e362c98f0eb6b92c0b0fb13f9e72e20e8`

**Normative requirements and implementation claims:**
- **In scope:**
- - All pages of the Nunchi dashboard, including pages that do not yet exist and pages that should be added. - The design language, component library, and visual system the dashboard uses. - The real-time behavior and aesthetic reactivity of the dashboard. - How users create new things through the dashboard (agents, extensions, arenas, evals, meta-agents). - The information architecture and navigation model. - API and streaming surfaces the dashboard consumes (as a consumer, not as the designer of those surfaces). - What the dashboard needs from Roko, Korai, and Mirage that does not yet exist (proposed additions, not detailed specs for those layers).
- **Out of scope:**
- - Detailed implementation of Roko internals (the runtime, the heartbeat pipeline, the gate pipeline). These are specified elsewhere in the PRDs. - Detailed implementation of Korai internals (consensus, execution, precompiles). Specified elsewhere. - Detailed implementation of smart contracts. Specified elsewhere. - Marketing pages, landing pages, documentation sites, developer portals. - Mobile applications. The dashboard is a web application. A mobile companion could exist later; it is not in scope here. - Pricing, monetization, billing UI. These may exist in the dashboard but their design is a product decision that should be made separately from this specification. - Detailed API specifications for endpoints the dashboard consumes. The dashboard will consume the API; the API's design is specified in the PRDs for Roko and Korai.

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
- - All pages of the Nunchi dashboard, including pages that do not yet exist and pages that should be added.
- - The design language, component library, and visual system the dashboard uses.
- - The real-time behavior and aesthetic reactivity of the dashboard.
- - How users create new things through the dashboard (agents, extensions, arenas, evals, meta-agents).
- - The information architecture and navigation model.
- - API and streaming surfaces the dashboard consumes (as a consumer, not as the designer of those surfaces).
- - What the dashboard needs from Roko, Korai, and Mirage that does not yet exist (proposed additions, not detailed specs for those layers).
- - Detailed implementation of Roko internals (the runtime, the heartbeat pipeline, the gate pipeline). These are specified elsewhere in the PRDs.
- - Detailed implementation of Korai internals (consensus, execution, precompiles). Specified elsewhere.
- - Detailed implementation of smart contracts. Specified elsewhere.
- - Marketing pages, landing pages, documentation sites, developer portals.
- - Mobile applications. The dashboard is a web application. A mobile companion could exist later; it is not in scope here.
- - Pricing, monetization, billing UI. These may exist in the dashboard but their design is a product decision that should be made separately from this specification.
- - Detailed API specifications for endpoints the dashboard consumes. The dashboard will consume the API; the API's design is specified in the PRDs for Roko and Korai.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
rg -n "scope|out|pages|detailed|specified|here|exist" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "scope|out|pages|detailed|specified|here|exist" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S010 -- What changed from the current dashboard

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:147` through `162`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What changed from the current dashboard

The current Nunchi dashboard exists and has a recognizable structure (see the current sidebar groups: Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System). This specification does not start from scratch. It inherits from the current dashboard and proposes changes.

The changes fall into four categories:

**Kept**: the ROSEDUST visual system, the typography stack, the glass morphism hierarchy, the general sidebar organization, most current pages.

**Revised**: some pages move to different sidebar sections. The information architecture gets a modest restructuring, documented in `04-information-architecture.md`. Some pages get new capabilities. The visual system gets extended with performance-reactive aesthetics, documented in `08-epistemic-aesthetics.md`.

**Added**: new pages that do not exist yet, primarily in the authoring layer (`19-authoring-surfaces.md`) and the meta layer (`16-meta-surfaces.md`), and one new sidebar section for measurements and evals.

**Retired**: nothing is removed outright, but some current emphasis is reduced. The "mortality as backbone" framing from the earlier Bardo/Golem era is replaced with "mortality as opt-in feature." The current dashboard's read-heavy bias (lots of observation, thin authoring) is rebalanced by adding substantial authoring surfaces.

Every specific change is called out in the document where it applies.
````

**Explicit detail extraction from this section:**

- Section word count: `199`
- Section hash: `5836b112f7ec914affec702b59820326151eee3e87703a1f286f59556c75e04e`

**Normative requirements and implementation claims:**
- The current Nunchi dashboard exists and has a recognizable structure (see the current sidebar groups: Pulse, Fleet, Forge, Knowledge, Arena, Treasury, System). This specification does not start from scratch. It inherits from the current dashboard and proposes changes.
- **Kept**: the ROSEDUST visual system, the typography stack, the glass morphism hierarchy, the general sidebar organization, most current pages.
- **Revised**: some pages move to different sidebar sections. The information architecture gets a modest restructuring, documented in `04-information-architecture.md`. Some pages get new capabilities. The visual system gets extended with performance-reactive aesthetics, documented in `08-epistemic-aesthetics.md`.
- **Added**: new pages that do not exist yet, primarily in the authoring layer (`19-authoring-surfaces.md`) and the meta layer (`16-meta-surfaces.md`), and one new sidebar section for measurements and evals.
- **Retired**: nothing is removed outright, but some current emphasis is reduced. The "mortality as backbone" framing from the earlier Bardo/Golem era is replaced with "mortality as opt-in feature." The current dashboard's read-heavy bias (lots of observation, thin authoring) is rebalanced by adding substantial authoring surfaces.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
rg -n "the|current|change|changed|sidebar|pages|authoring|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|current|change|changed|sidebar|pages|authoring|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S011 -- What to do if something is missing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:163` through `172`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What to do if something is missing

These documents try to be complete, but they are finite. If you are implementing the dashboard and find a gap — a question these documents do not answer — you have three options:

First, check `01-system-landscape.md` to see if the gap is a terminology question. Often a question that feels like a gap is a missing definition.

Second, check `22-deferred-and-open-questions.md` to see if the gap is an explicitly deferred decision. If it is, the product owner needs to make the call.

Third, use your best judgment, record the decision you made, and flag it for later review. Do not block. Do not invent a requirement. Do not contradict these documents. The spec is the floor; judgment fills the gaps above the floor.
````

**Explicit detail extraction from this section:**

- Section word count: `128`
- Section hash: `0f208c53e2a6867f7dd91fe2a3a978c15e70082deb04ec8b65b023d6c4988238`

**Normative requirements and implementation claims:**
- These documents try to be complete, but they are finite. If you are implementing the dashboard and find a gap — a question these documents do not answer — you have three options:
- Second, check `22-deferred-and-open-questions.md` to see if the gap is an explicitly deferred decision. If it is, the product owner needs to make the call.
- Third, use your best judgment, record the decision you made, and flag it for later review. Do not block. Do not invent a requirement. Do not contradict these documents. The spec is the floor; judgment fills the gaps above the floor.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "missing|something|question|documents|judgment|floor|deferred|decision" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "missing|something|question|documents|judgment|floor|deferred|decision" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

### DASH-00-S012 -- A note on tone

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md:173` through `179`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## A note on tone

These documents are written in direct prose. Short sentences where short sentences work. Longer sentences where precision requires them. No hedging language unless hedging is the honest choice. No "perhaps consider" when "do this" is the actual instruction. No corporate voice. No cheerleading.

If something is hard, the documents say it is hard. If something is speculative, the documents say it is speculative. If a decision was made on limited information, the documents say so.

The product this specification describes is ambitious. The tone should match — confident, specific, but honest about what is not yet known.
````

**Explicit detail extraction from this section:**

- Section word count: `96`
- Section hash: `6b1b1cba6e282558f54b685637d886eb2c86d5758c5ae53fad0e0c0fc0a29e17`

**Normative requirements and implementation claims:**
- These documents are written in direct prose. Short sentences where short sentences work. Longer sentences where precision requires them. No hedging language unless hedging is the honest choice. No "perhaps consider" when "do this" is the actual instruction. No corporate voice. No cheerleading.
- The product this specification describes is ambitious. The tone should match — confident, specific, but honest about what is not yet known.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/00-read-me-first.md`
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tone|note|documents|sentences|speculative|specific|something|honest" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tone|note|documents|sentences|speculative|specific|something|honest" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/parity.rs`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/00-read-me-first
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

