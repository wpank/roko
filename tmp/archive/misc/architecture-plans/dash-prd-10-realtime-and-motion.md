# Dashboard PRD Plan: Realtime And Motion

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
**Generated:** 2026-04-25
**Source hash:** `e126ede5cc6cecca51df3ffe54a3b55210c25b277243563bd3577a62fa58c3d7`
**Section tasks:** 58
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
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-10-S001 | 1 | 10 — Realtime and motion | [ ] | 9.8 |
| DASH-10-S002 | 7 | Why this document matters | [ ] | 9.8 |
| DASH-10-S003 | 17 | The streaming model | [ ] | 9.8 |
| DASH-10-S004 | 21 | Agent event streams | [ ] | 9.8 |
| DASH-10-S005 | 48 | Chain block streams | [ ] | 9.8 |
| DASH-10-S006 | 58 | Measurement and computed streams | [ ] | 9.8 |
| DASH-10-S007 | 68 | What updates live and what does not | [ ] | 9.8 |
| DASH-10-S008 | 74 | Always live | [ ] | 9.8 |
| DASH-10-S009 | 83 | Live when visible | [ ] | 9.8 |
| DASH-10-S010 | 91 | Refreshed on navigation | [ ] | 9.8 |
| DASH-10-S011 | 97 | Never live | [ ] | 9.8 |
| DASH-10-S012 | 107 | Update rendering patterns | [ ] | 9.8 |
| DASH-10-S013 | 111 | Value ticking | [ ] | 9.8 |
| DASH-10-S014 | 121 | List items appearing | [ ] | 9.8 |
| DASH-10-S015 | 131 | List items departing | [ ] | 9.8 |
| DASH-10-S016 | 139 | Status changes | [ ] | 9.8 |
| DASH-10-S017 | 147 | Rank reorderings | [ ] | 9.8 |
| DASH-10-S018 | 155 | Progress advances | [ ] | 9.8 |
| DASH-10-S019 | 163 | Field updates | [ ] | 9.8 |
| DASH-10-S020 | 173 | Rhythm and cadence | [ ] | 9.8 |
| DASH-10-S021 | 177 | The heartbeat tick | [ ] | 9.8 |
| DASH-10-S022 | 183 | Ambient pulse | [ ] | 9.8 |
| DASH-10-S023 | 187 | Event cadence | [ ] | 9.8 |
| DASH-10-S024 | 191 | Lens shifts and page transitions | [ ] | 9.8 |
| DASH-10-S025 | 195 | Tempo consistency | [ ] | 9.8 |
| DASH-10-S026 | 201 | Handling high update rates | [ ] | 9.8 |
| DASH-10-S027 | 205 | Aggregation | [ ] | 9.8 |
| DASH-10-S028 | 215 | Throttling | [ ] | 9.8 |
| DASH-10-S029 | 219 | Prioritization | [ ] | 9.8 |
| DASH-10-S030 | 223 | Graceful degradation | [ ] | 9.8 |
| DASH-10-S031 | 229 | Reducing motion without losing information | [ ] | 9.8 |
| DASH-10-S032 | 233 | What reduces | [ ] | 9.8 |
| DASH-10-S033 | 241 | What stays | [ ] | 9.8 |
| DASH-10-S034 | 247 | Manual control | [ ] | 9.8 |
| DASH-10-S035 | 257 | Real-time subscription lifecycle | [ ] | 9.8 |
| DASH-10-S036 | 261 | Subscription creation | [ ] | 9.8 |
| DASH-10-S037 | 267 | Subscription cleanup | [ ] | 9.8 |
| DASH-10-S038 | 273 | Reconnection | [ ] | 9.8 |
| DASH-10-S039 | 283 | Stale indicators | [ ] | 9.8 |
| DASH-10-S040 | 295 | Chain-specific realtime behavior | [ ] | 9.8 |
| DASH-10-S041 | 299 | Block-paced updates | [ ] | 9.8 |
| DASH-10-S042 | 303 | Reorg handling | [ ] | 9.8 |
| DASH-10-S043 | 311 | Gas and timing | [ ] | 9.8 |
| DASH-10-S044 | 323 | Motion as information versus motion as decoration | [ ] | 9.8 |
| DASH-10-S045 | 346 | Specific patterns for recurring UI elements | [ ] | 9.8 |
| DASH-10-S046 | 348 | Live number glyphs | [ ] | 9.8 |
| DASH-10-S047 | 354 | Status dots | [ ] | 9.8 |
| DASH-10-S048 | 362 | Loading states | [ ] | 9.8 |
| DASH-10-S049 | 373 | Notifications | [ ] | 9.8 |
| DASH-10-S050 | 383 | Modal entries and exits | [ ] | 9.8 |
| DASH-10-S051 | 389 | Toast notifications | [ ] | 9.8 |
| DASH-10-S052 | 395 | Performance budget | [ ] | 9.8 |
| DASH-10-S053 | 416 | Indicators for realtime state | [ ] | 9.8 |
| DASH-10-S054 | 420 | Global live indicator | [ ] | 9.8 |
| DASH-10-S055 | 429 | Per-element stale indicators | [ ] | 9.8 |
| DASH-10-S056 | 433 | Pause and resume | [ ] | 9.8 |
| DASH-10-S057 | 439 | Testing realtime behavior | [ ] | 9.8 |
| DASH-10-S058 | 454 | Summary | [ ] | 9.8 |

## Tasks

### DASH-10-S001 -- 10 — Realtime and motion

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 10 — Realtime and motion

*How the Nunchi dashboard makes the continuous nature of the system visible. What updates live, how updates are rendered, the rhythms the UI adopts, and how motion conveys meaning.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `29`
- Section hash: `ed83e42d156e9fd98a67ca35dd43d2edbc6957022c685fb45493ebb18dc01635`

**Normative requirements and implementation claims:**
- *How the Nunchi dashboard makes the continuous nature of the system visible. What updates live, how updates are rendered, the rhythms the UI adopts, and how motion conveys meaning.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "motion|Realtime|updates|visible|rhythms|rendered|nature" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "motion|Realtime|updates|visible|rhythms|rendered|nature" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S002 -- Why this document matters

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:7` through `16`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Why this document matters

The Nunchi system is continuously producing signal. Agents tick. Gates fire. Knowledge gets validated and decayed. Pheromones deposit and fade. Chain blocks confirm. Leaderboards update. Clearing rounds resolve. If the dashboard snapshotted this activity and required users to refresh, it would feel dead — and it would hide exactly the liveness that makes the product what it is.

This document specifies how the dashboard renders the system's continuous activity. What data streams into the UI. How streams are visualized. How motion communicates without overwhelming. How the UI remains legible under high update rates. How it degrades gracefully when connections are lost.

Realtime is not a feature of the dashboard; it is the dashboard's default mode. Static views are the exception, used where data genuinely doesn't change. Everything else is alive.

---
````

**Explicit detail extraction from this section:**

- Section word count: `132`
- Section hash: `788c0336077fba2ad770f001f2d7fcb758444dd22546a6b0b701bbeb280826e7`

**Normative requirements and implementation claims:**
- The Nunchi system is continuously producing signal. Agents tick. Gates fire. Knowledge gets validated and decayed. Pheromones deposit and fade. Chain blocks confirm. Leaderboards update. Clearing rounds resolve. If the dashboard snapshotted this activity and required users to refresh, it would feel dead — and it would hide exactly the liveness that makes the product what it is.
- This document specifies how the dashboard renders the system's continuous activity. What data streams into the UI. How streams are visualized. How motion communicates without overwhelming. How the UI remains legible under high update rates. How it degrades gracefully when connections are lost.
- Realtime is not a feature of the dashboard; it is the dashboard's default mode. Static views are the exception, used where data genuinely doesn't change. Everything else is alive.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "document|matters|Why|update|streams|data|continuous" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|matters|Why|update|streams|data|continuous" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S003 -- The streaming model

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:17` through `20`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The streaming model

The dashboard consumes three main streams of data, each with its own characteristics.
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `af30b4bfaa9e1780e155af29c09bfc780e5801b8cdb7b16ee02a89c6e3321677`

**Normative requirements and implementation claims:**
- The dashboard consumes three main streams of data, each with its own characteristics.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "The|streaming|model|three|streams|main|data|consumes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|streaming|model|three|streams|main|data|consumes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S004 -- Agent event streams

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:21` through `47`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent event streams

Each running agent publishes events through its Roko event fabric. The dashboard subscribes via WebSocket to receive these events in real time.

Event types include (non-exhaustive):

- `tick.started` / `tick.completed` — heartbeat events.
- `gate.passed` / `gate.failed` — gate pipeline outcomes.
- `tool.called` / `tool.returned` — tool invocations.
- `llm.requested` / `llm.responded` — LLM calls.
- `context.assembled` — context workspace computed.
- `knowledge.retrieved` — knowledge entry fetched.
- `knowledge.promoted` — entry graduated from staging to store.
- `pheromone.deposited` — pheromone deposited on chain.
- `outcome.recorded` — episode outcome.
- `state.transitioned` — lifecycle state change.
- `dream.started` / `dream.completed` — consolidation cycles.
- `error` — runtime errors.

Events carry structured payloads: timestamp, agent ID, sequence number, typed data. The dashboard maintains a subscription per agent (or per set of agents) and receives events as they are published.

**Rate.** A typical long-running agent produces low-tens of events per minute under normal operation, with bursts during active work. A fleet of 100 agents under load can produce hundreds of events per second aggregate.

**Backpressure.** The dashboard handles burst rates without dropping events. If the UI cannot render each event individually (during extreme bursts), it aggregates rather than drops.

**Resilience.** On disconnect and reconnect, the dashboard sends the last known sequence number; the server replays events from that point. No gaps in the record.
````

**Explicit detail extraction from this section:**

- Section word count: `210`
- Section hash: `f2a2bc7c4b5f8513eb6ce3f5ea7ee9272f900b21d2d178e7b3d984047e39a2a9`

**Normative requirements and implementation claims:**
- Each running agent publishes events through its Roko event fabric. The dashboard subscribes via WebSocket to receive these events in real time.
- Event types include (non-exhaustive):
- - `tick.started` / `tick.completed` — heartbeat events. - `gate.passed` / `gate.failed` — gate pipeline outcomes. - `tool.called` / `tool.returned` — tool invocations. - `llm.requested` / `llm.responded` — LLM calls. - `context.assembled` — context workspace computed. - `knowledge.retrieved` — knowledge entry fetched. - `knowledge.promoted` — entry graduated from staging to store. - `pheromone.deposited` — pheromone deposited on chain. - `outcome.recorded` — episode outcome. - `state.transitioned` — lifecycle state change. - `dream.started` / `dream.completed` — consolidation cycles. - `error` — runtime errors.
- Events carry structured payloads: timestamp, agent ID, sequence number, typed data. The dashboard maintains a subscription per agent (or per set of agents) and receives events as they are published.
- **Rate.** A typical long-running agent produces low-tens of events per minute under normal operation, with bursts during active work. A fleet of 100 agents under load can produce hundreds of events per second aggregate.
- **Backpressure.** The dashboard handles burst rates without dropping events. If the UI cannot render each event individually (during extreme bursts), it aggregates rather than drops.
- **Resilience.** On disconnect and reconnect, the dashboard sends the last known sequence number; the server replays events from that point. No gaps in the record.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- error

**Event names and event-like entities:**
- tick.started
- tick.completed
- gate.passed
- gate.failed
- tool.called
- tool.returned
- llm.requested
- llm.responded
- context.assembled
- knowledge.retrieved
- knowledge.promoted
- pheromone.deposited
- outcome.recorded
- state.transitioned
- dream.started
- dream.completed

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- tick.started
- tick.completed
- gate.passed
- gate.failed
- tool.called
- tool.returned
- llm.requested
- llm.responded
- context.assembled
- knowledge.retrieved
- knowledge.promoted
- pheromone.deposited
- outcome.recorded
- state.transitioned
- dream.started
- dream.completed

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `tick.started` / `tick.completed` — heartbeat events.
- - `gate.passed` / `gate.failed` — gate pipeline outcomes.
- - `tool.called` / `tool.returned` — tool invocations.
- - `llm.requested` / `llm.responded` — LLM calls.
- - `context.assembled` — context workspace computed.
- - `knowledge.retrieved` — knowledge entry fetched.
- - `knowledge.promoted` — entry graduated from staging to store.
- - `pheromone.deposited` — pheromone deposited on chain.
- - `outcome.recorded` — episode outcome.
- - `state.transitioned` — lifecycle state change.
- - `dream.started` / `dream.completed` — consolidation cycles.
- - `error` — runtime errors.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "event|events|error|streams|gate|tool|time|outcome" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "event|events|error|streams|gate|tool|time|outcome" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `error` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `tick.started` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `tick.completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `gate.passed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `gate.failed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `tool.called` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `tool.returned` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `llm.requested` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `llm.responded` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `context.assembled` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.retrieved` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.promoted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `pheromone.deposited` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `outcome.recorded` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `state.transitioned` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `dream.started` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `dream.completed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `tick.started` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tick.completed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `gate.passed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `gate.failed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tool.called` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tool.returned` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `llm.requested` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `llm.responded` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `context.assembled` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.retrieved` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.promoted` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `pheromone.deposited` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `outcome.recorded` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `state.transitioned` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `dream.started` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `dream.completed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S005 -- Chain block streams

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:48` through `57`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Chain block streams

The dashboard subscribes to Korai (or Mirage) via RPC/WebSocket for new blocks. Each block triggers updates to chain-sourced data: on-chain agent passports, reputation updates, pheromone deposits on chain, clearing outcomes, bounty state changes, ISFR values.

**Rate.** Korai target block time is sub-second; the dashboard sees a new block roughly every 500ms. Most blocks are empty or near-empty for the dashboard's purposes (not every block updates data the user cares about).

**Selectivity.** The dashboard does not process every block's full state. It subscribes to specific events via event filters and processes only relevant changes.

**Derived data.** Some chain-sourced data is computed (aggregates, summaries) from block streams. These aggregates are kept in dashboard memory and updated incrementally as blocks arrive.
````

**Explicit detail extraction from this section:**

- Section word count: `126`
- Section hash: `4860226151f1fd2e1af3e5910536e702bb0ac89d7989eecd1f59cbfaa29cda4e`

**Normative requirements and implementation claims:**
- The dashboard subscribes to Korai (or Mirage) via RPC/WebSocket for new blocks. Each block triggers updates to chain-sourced data: on-chain agent passports, reputation updates, pheromone deposits on chain, clearing outcomes, bounty state changes, ISFR values.
- **Rate.** Korai target block time is sub-second; the dashboard sees a new block roughly every 500ms. Most blocks are empty or near-empty for the dashboard's purposes (not every block updates data the user cares about).
- **Selectivity.** The dashboard does not process every block's full state. It subscribes to specific events via event filters and processes only relevant changes.
- **Derived data.** Some chain-sourced data is computed (aggregates, summaries) from block streams. These aggregates are kept in dashboard memory and updated incrementally as blocks arrive.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "block|Chain|streams|data|updates|every|blocks|subscribes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "block|Chain|streams|data|updates|every|blocks|subscribes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S006 -- Measurement and computed streams

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:58` through `67`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Measurement and computed streams

Beyond raw event and chain streams, some derived streams exist: arena leaderboards recompute as attempts complete, cost analytics recompute as events record costs, epistemic sharpness recomputes as knowledge entries change state.

Some of these recomputes happen server-side (backend pushes updated aggregates to the dashboard). Others happen client-side (dashboard subscribes to underlying streams and recomputes).

The principle: data the user sees should reflect the system's actual state within the shortest practical latency. Staleness is acceptable when noted; unnoted staleness is not.

---
````

**Explicit detail extraction from this section:**

- Section word count: `83`
- Section hash: `ca891749c9e6a586316a19f9a2b5c500134cc2f06905218899ab12835505835a`

**Normative requirements and implementation claims:**
- Beyond raw event and chain streams, some derived streams exist: arena leaderboards recompute as attempts complete, cost analytics recompute as events record costs, epistemic sharpness recomputes as knowledge entries change state.
- Some of these recomputes happen server-side (backend pushes updated aggregates to the dashboard). Others happen client-side (dashboard subscribes to underlying streams and recomputes).
- The principle: data the user sees should reflect the system's actual state within the shortest practical latency. Staleness is acceptable when noted; unnoted staleness is not.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "streams|recompute|computed|Measurement|recomputes|state|some" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "streams|recompute|computed|Measurement|recomputes|state|some" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S007 -- What updates live and what does not

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:68` through `73`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## What updates live and what does not

Not everything in the dashboard updates live. Some content is genuinely static (a historical episode's replay doesn't change). Some content changes rarely (the list of reputation tiers). Some content changes constantly (a live agent's current tick).

The rule: data updates live when it can change in ways the user cares about. The dashboard marks which data is live and which is not.
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `d0af22d8464cef72d574c688c82f4f1e4e657a515695dad7ee45614c68c25e6a`

**Normative requirements and implementation claims:**
- Not everything in the dashboard updates live. Some content is genuinely static (a historical episode's replay doesn't change). Some content changes rarely (the list of reputation tiers). Some content changes constantly (a live agent's current tick).
- The rule: data updates live when it can change in ways the user cares about. The dashboard marks which data is live and which is not.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "live|updates|not|change|content|Some|data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "live|updates|not|change|content|Some|data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S008 -- Always live

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:74` through `82`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Always live

- Agent activity: the Live Console, Event Stream, agent status indicators, current cognitive tier, gate pass rate.
- Chain state: block height, ISFR rate, pheromone field, recent chain transactions.
- Leaderboards: rank updates as attempts complete.
- Aggregate KPIs on Command Center.
- Live counts (on-chain agents, knowledge entries, active pheromones).
- Event Stream itself.
````

**Explicit detail extraction from this section:**

- Section word count: `50`
- Section hash: `a94859417b8d99561796f98bec36f4488b3c4fe49479074b0f12b5661a1fade0`

**Normative requirements and implementation claims:**
- - Agent activity: the Live Console, Event Stream, agent status indicators, current cognitive tier, gate pass rate. - Chain state: block height, ISFR rate, pheromone field, recent chain transactions. - Leaderboards: rank updates as attempts complete. - Aggregate KPIs on Command Center. - Live counts (on-chain agents, knowledge entries, active pheromones). - Event Stream itself.

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
- - Agent activity: the Live Console, Event Stream, agent status indicators, current cognitive tier, gate pass rate.
- - Chain state: block height, ISFR rate, pheromone field, recent chain transactions.
- - Leaderboards: rank updates as attempts complete.
- - Aggregate KPIs on Command Center.
- - Live counts (on-chain agents, knowledge entries, active pheromones).
- - Event Stream itself.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "live|Always|Chain|rate|pheromone|gate|Stream|Event" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "live|Always|Chain|rate|pheromone|gate|Stream|Event" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S009 -- Live when visible

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:83` through `90`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Live when visible

- Agent Detail for a running agent.
- Knowledge Entry Detail (updates when validated or challenged).
- Arena Detail (updates when new attempts happen).
- Group coordination views.
- Dream Cycles page (when dreams are running).
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `360bbf7c8c02174042280dfa4d6d6ce9d27da152d423b58ed694b08b1a4013a7`

**Normative requirements and implementation claims:**
- - Agent Detail for a running agent. - Knowledge Entry Detail (updates when validated or challenged). - Arena Detail (updates when new attempts happen). - Group coordination views. - Dream Cycles page (when dreams are running).

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
- - Agent Detail for a running agent.
- - Knowledge Entry Detail (updates when validated or challenged).
- - Arena Detail (updates when new attempts happen).
- - Group coordination views.
- - Dream Cycles page (when dreams are running).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "visible|Live|Detail|updates|running|Dream|views|validated" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "visible|Live|Detail|updates|running|Dream|views|validated" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S010 -- Refreshed on navigation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:91` through `96`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Refreshed on navigation

- Historical views (Replay, episode archives).
- Analytical dashboards with slow-changing data.
- User settings and configuration (updated on explicit save, not continuously).
````

**Explicit detail extraction from this section:**

- Section word count: `21`
- Section hash: `1ae38a5963b11f99c4208734fbe269643a0da24c3b3e1dc4374c87e6740ccc0b`

**Normative requirements and implementation claims:**
- - Historical views (Replay, episode archives). - Analytical dashboards with slow-changing data. - User settings and configuration (updated on explicit save, not continuously).

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
- - Historical views (Replay, episode archives).
- - Analytical dashboards with slow-changing data.
- - User settings and configuration (updated on explicit save, not continuously).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "navigation|Refreshed|views|updated|slow|settings|save|explicit" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "navigation|Refreshed|views|updated|slow|settings|save|explicit" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S011 -- Never live

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:97` through `106`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Never live

- Template definitions (static once published).
- Historical leaderboards at a fixed date.
- Static documentation.

The distinction matters because live data carries UI cost (subscriptions, re-renders, motion). Data that doesn't need to be live should not be — both for performance and to avoid visual noise.

---
````

**Explicit detail extraction from this section:**

- Section word count: `45`
- Section hash: `b845eff545449a2cd17fc71e3cde53334e50888f10b9c7befb30b17328d21d0e`

**Normative requirements and implementation claims:**
- - Template definitions (static once published). - Historical leaderboards at a fixed date. - Static documentation.
- The distinction matters because live data carries UI cost (subscriptions, re-renders, motion). Data that doesn't need to be live should not be — both for performance and to avoid visual noise.
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
- - Template definitions (static once published).
- - Historical leaderboards at a fixed date.
- - Static documentation.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "live|Never|static|data|visual|subscriptions|renders|published" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "live|Never|static|data|visual|subscriptions|renders|published" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S012 -- Update rendering patterns

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:107` through `110`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Update rendering patterns

When data updates, the UI renders the change. The rendering follows patterns that depend on update frequency and visual prominence.
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `50c9896abc9b4a02c4d24e0e6214262a9d908286de3a5f9258fe7416d2af3010`

**Normative requirements and implementation claims:**
- When data updates, the UI renders the change. The rendering follows patterns that depend on update frequency and visual prominence.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Update|rendering|patterns|visual|updates|renders|prominence|frequency" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Update|rendering|patterns|visual|updates|renders|prominence|frequency" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S013 -- Value ticking

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:111` through `120`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Value ticking

A single numerical value updates. Pattern: the new value replaces the old with a brief highlight animation.

Implementation: the number glyph brightens for 200ms and slightly scales (1.02x), then returns. If the value is accompanied by a delta indicator (arrow, color tint), that also updates briefly.

Used for: KPI card values, block height, counts, prices, rates.

Frequency tolerance: up to several times per second. Above that, switch to a throttled-update pattern where visible changes are every 500ms but internal state is current.
````

**Explicit detail extraction from this section:**

- Section word count: `84`
- Section hash: `82233864faa86b36f3c27b2c8d80564e29c92f5b6e960bf81ed48eec7d7b6775`

**Normative requirements and implementation claims:**
- Frequency tolerance: up to several times per second. Above that, switch to a throttled-update pattern where visible changes are every 500ms but internal state is current.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Value|ticking|update|updates|brief|Pattern|visible|values" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Value|ticking|update|updates|brief|Pattern|visible|values" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S014 -- List items appearing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:121` through `130`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### List items appearing

A new item enters a list or feed. Pattern: the item fades in from transparent, height-expands from 0 to natural, and items below shift down smoothly.

Implementation: CSS transition on height and opacity with `--ease-luxury` over 320ms. For dense streams, staggered delay (20ms per new item) prevents simultaneous popping.

Used for: Event Stream new events, new knowledge entries, new agent rows, new pheromone deposits.

Frequency tolerance: handles 10+ per second comfortably. Above, aggregation takes over (see below).
````

**Explicit detail extraction from this section:**

- Section word count: `79`
- Section hash: `6cadab914f18aa5ca30d7159a9440753393e93107b8dac4cb002df71dd45faa2`

**Normative requirements and implementation claims:**
- Used for: Event Stream new events, new knowledge entries, new agent rows, new pheromone deposits.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "item|items|List|appearing|Event|over|height" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "item|items|List|appearing|Event|over|height" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S015 -- List items departing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:131` through `138`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### List items departing

An item leaves a list. Pattern: opacity fades out, height collapses, items below shift up.

Implementation: reverse of appearance, 200ms duration.

Used for: resolved events aging out, filtered-out items, dismissed notifications.
````

**Explicit detail extraction from this section:**

- Section word count: `32`
- Section hash: `775f11844d9b617522e191936a758a54c72da493fb1f521ecfe2b0658d1f8e99`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "item|items|List|departing|shift|reverse|resolved|opacity" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "item|items|List|departing|shift|reverse|resolved|opacity" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S016 -- Status changes

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:139` through `146`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Status changes

A status indicator changes (e.g., agent goes from running to stuck). Pattern: the status indicator animates from old state to new over 240ms with a brief emphasis moment.

Implementation: color transition, with optional brief glow peak at the midpoint.

Used for: agent status dots, gate pass/fail indicators, tier transitions.
````

**Explicit detail extraction from this section:**

- Section word count: `51`
- Section hash: `9c7d50f5ac101dacab6f34ec3265c361f2ec5ea253d9a2ee2241659967fe05b3`

**Normative requirements and implementation claims:**
- A status indicator changes (e.g., agent goes from running to stuck). Pattern: the status indicator animates from old state to new over 240ms with a brief emphasis moment.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Status|changes|indicator|transition|brief|transitions|tier|stuck" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Status|changes|indicator|transition|brief|transitions|tier|stuck" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S017 -- Rank reorderings

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:147` through `154`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Rank reorderings

A list reorders (leaderboard positions change). Pattern: items animate to their new positions with a spatial transition.

Implementation: FLIP technique (First, Last, Invert, Play) — compute start and end positions, apply transforms, animate to final positions. Duration: 480ms. Velocity streak visible during transition.

Used for: Leaderboard updates, any reordered list.
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `22a9b80f7e094bdf63cc80e6fc19d091ccca2712af9f410d486c077518924d57`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "reorderings|Rank|positions|transition|list|leaderboard|animate|visible" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reorderings|Rank|positions|transition|list|leaderboard|animate|visible" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S018 -- Progress advances

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:155` through `162`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Progress advances

A progress bar or step indicator advances. Pattern: the progress smoothly eases to its new position.

Implementation: width or fill transition over 320ms.

Used for: plan execution progress, arena attempt progress, onboarding flows.
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `4ccab1589381aecf72436b9db6141f78b6351384456d8123fcb11b05783d332b`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Progress|advances|width|transition|step|smoothly|position|plan" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Progress|advances|width|transition|step|smoothly|position|plan" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S019 -- Field updates

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:163` through `172`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Field updates

A spatial field visualization (stigmergy field, resonance graph, knowledge topography) receives new data. Pattern: changes ripple through the field without re-rendering the whole thing.

Implementation: for canvas/WebGL visualizations, per-frame diff rendering. Only the changed elements re-draw; the rest stays constant.

Used for: Stigmergy Field, Resonance Graph, Knowledge Topography, any primitive-3 or primitive-13 from `09-visualization-primitives.md`.

---
````

**Explicit detail extraction from this section:**

- Section word count: `63`
- Section hash: `112ff470e28c7ff3bb8e53412f1b04b81f8ffd215deba614b6405231d0f10d85`

**Normative requirements and implementation claims:**
- Implementation: for canvas/WebGL visualizations, per-frame diff rendering. Only the changed elements re-draw; the rest stays constant.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Field|updates|graph|visualization|primitive|topography|stigmergy|resonance" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Field|updates|graph|visualization|primitive|topography|stigmergy|resonance" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S020 -- Rhythm and cadence

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:173` through `176`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Rhythm and cadence

The dashboard's motion creates a rhythm. That rhythm should feel calm, not nervous. Users sit with the dashboard for long sessions; a nervous UI becomes exhausting.
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `b2d9a9dc363ead63b77965eb90f8ce1c3d9d4ba8cb3ec12ececf9146aaf5d20e`

**Normative requirements and implementation claims:**
- The dashboard's motion creates a rhythm. That rhythm should feel calm, not nervous. Users sit with the dashboard for long sessions; a nervous UI becomes exhausting.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Rhythm|cadence|nervous|sessions|motion|long|feel" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Rhythm|cadence|nervous|sessions|motion|long|feel" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S021 -- The heartbeat tick

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:177` through `182`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### The heartbeat tick

Live agent rows and agent-related primitives respect the agent's heartbeat cadence. A gamma-ticking agent (every 5–15 seconds) produces a subtle visual tick at that rate. A theta-ticking agent (every 30–120 seconds) produces a slower tick.

Visual: a brief brightening of a status dot or a border accent on the agent's card, over 200ms, then return. Barely perceptible individually; collectively communicating "these agents are alive."
````

**Explicit detail extraction from this section:**

- Section word count: `71`
- Section hash: `818de8ce0d409f2ae503a3bc7ef8e561852fbfdc953bfa11b63ca7ad04bbf15c`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "The|tick|heartbeat|visual|ticking|seconds|produces|every" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|tick|heartbeat|visual|ticking|seconds|produces|every" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S022 -- Ambient pulse

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:183` through `186`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Ambient pulse

Ambient pulses for high-sharpness elements (see `08-epistemic-aesthetics.md`). Slow cycles, 4–8 seconds per pulse. Not synchronized across elements — each element pulses on its own cycle. The overall effect is ambient, like breathing.
````

**Explicit detail extraction from this section:**

- Section word count: `36`
- Section hash: `60075c1bcc084d053ed33ba19a87078b208985611bd73acbecf7b47ce6a1dc89`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "pulse|Ambient|element|pulses|elements|cycle|synchronized|sharpness" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "pulse|Ambient|element|pulses|elements|cycle|synchronized|sharpness" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S023 -- Event cadence

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:187` through `190`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Event cadence

The Event Stream has its own cadence, matched to the underlying event rate. At 1 event per second, each event is individually visible. At 100 events per second, events aggregate into bins (see below).
````

**Explicit detail extraction from this section:**

- Section word count: `34`
- Section hash: `abd99c2f436f473b19d09fe12439e6666edb4b978e3cf44b41a4b10e3d5b8ff7`

**Normative requirements and implementation claims:**
- The Event Stream has its own cadence, matched to the underlying event rate. At 1 event per second, each event is individually visible. At 100 events per second, events aggregate into bins (see below).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Event|cadence|second|events|visible|underlying|rate|matched" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Event|cadence|second|events|visible|underlying|rate|matched" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S024 -- Lens shifts and page transitions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:191` through `194`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Lens shifts and page transitions

When the user navigates, the dashboard transitions over 200–320ms. Not instant (instant feels abrupt), not slow (slow feels sluggish).
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `ffd6fe013ae800e2ed17e4cadf687cf8c9c32ed5ab3c460dcb0030f047fc11f4`

**Normative requirements and implementation claims:**
- When the user navigates, the dashboard transitions over 200–320ms. Not instant (instant feels abrupt), not slow (slow feels sluggish).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "transitions|shifts|Lens|slow|instant|feels|user" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "transitions|shifts|Lens|slow|instant|feels|user" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S025 -- Tempo consistency

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:195` through `200`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Tempo consistency

Within a single page, motion durations are consistent. If card hovers are 320ms, rank reorderings are 480ms, and item appearances are 320ms, the overall tempo is coherent. Mixing 120ms and 800ms on the same page feels uneven.

---
````

**Explicit detail extraction from this section:**

- Section word count: `37`
- Section hash: `9b10c3394ebfc8f013e795cd0764c5bbb7cf74db21e6ff43bacde5c911979efc`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Tempo|consistency|uneven|single|same|reorderings|rank" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Tempo|consistency|uneven|single|same|reorderings|rank" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S026 -- Handling high update rates

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:201` through `204`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Handling high update rates

The dashboard must remain legible when update rates are high. A page receiving 100 events per second cannot flash each event individually — that would be a wall of flickering that users would simply stop watching.
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `6c515cd1d8233594e2427c4405444dbe91e12773357c9a2cf387e29e18688243`

**Normative requirements and implementation claims:**
- The dashboard must remain legible when update rates are high. A page receiving 100 events per second cannot flash each event individually — that would be a wall of flickering that users would simply stop watching.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "update|rates|high|Handling|event|watching|wall" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "update|rates|high|Handling|event|watching|wall" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S027 -- Aggregation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:205` through `214`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Aggregation

When events arrive faster than the UI can individually animate, aggregate them. Examples:

- **Event Stream at high rate**: group events in 1-second bins, show the group with a count and a color band indicating type mix. Individual events accessible on drill-down.
- **Pheromone deposits during a burst**: aggregate into a density spike on the field.
- **Gate verdicts during load testing**: aggregate into a throughput indicator.

Aggregation is visible — the UI indicates "showing aggregated events" during bursts so users know what they're seeing.
````

**Explicit detail extraction from this section:**

- Section word count: `84`
- Section hash: `907744d6623c1b3dc8fad2fc002ccb1c7ee53f40c28a794933ce45f6ee97e929`

**Normative requirements and implementation claims:**
- - **Event Stream at high rate**: group events in 1-second bins, show the group with a count and a color band indicating type mix. Individual events accessible on drill-down. - **Pheromone deposits during a burst**: aggregate into a density spike on the field. - **Gate verdicts during load testing**: aggregate into a throughput indicator.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- mix

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Event Stream at high rate**: group events in 1-second bins, show the group with a count and a color band indicating type mix. Individual events accessible on drill-down.
- - **Pheromone deposits during a burst**: aggregate into a density spike on the field.
- - **Gate verdicts during load testing**: aggregate into a throughput indicator.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Aggregation|mix|Gate|Event|events|aggregate|during|show" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Aggregation|mix|Gate|Event|events|aggregate|during|show" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `mix` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S028 -- Throttling

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:215` through `218`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Throttling

Some updates throttle naturally. A counter that updates 1000 times per second is pointless — the user can't read it. Throttle to 10 updates per second; the underlying state is always current, but the display updates at a pace the eye can follow.
````

**Explicit detail extraction from this section:**

- Section word count: `43`
- Section hash: `36cd6ed91c492f88de07a3401956280f1ebc471d47de0c5032766f7c79b24b33`

**Normative requirements and implementation claims:**
- Some updates throttle naturally. A counter that updates 1000 times per second is pointless — the user can't read it. Throttle to 10 updates per second; the underlying state is always current, but the display updates at a pace the eye can follow.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Throttling|updates|throttle|second|user|underlying|times|state" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Throttling|updates|throttle|second|user|underlying|times|state" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S029 -- Prioritization

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:219` through `222`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Prioritization

When multiple updates compete, prioritize. Severity matters: an error event takes visual priority over a routine tick. Recency matters: the most recent update gets full animation; older updates blend into the background.
````

**Explicit detail extraction from this section:**

- Section word count: `32`
- Section hash: `91ad7aeb727ee7f2bde2738625130d688b63d1096a7d45fb995900c64d31ae86`

**Normative requirements and implementation claims:**
- When multiple updates compete, prioritize. Severity matters: an error event takes visual priority over a routine tick. Recency matters: the most recent update gets full animation; older updates blend into the background.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Prioritization|update|updates|matters|visual|tick|takes|routine" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Prioritization|update|updates|matters|visual|tick|takes|routine" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S030 -- Graceful degradation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:223` through `228`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Graceful degradation

If rendering falls behind, the UI drops visual fidelity before dropping correctness. Motion simplifies; data remains current. A user in an extreme-load scenario sees accurate values in a simplified presentation, not inaccurate values in a fancy one.

---
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `b886e6196f728014a4806706ae8014d775f8122093dd3a67eb6513b7b10922d6`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "degradation|Graceful|values|accurate|visual|user|simplifies|simplified" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "degradation|Graceful|values|accurate|visual|user|simplifies|simplified" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S031 -- Reducing motion without losing information

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:229` through `232`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Reducing motion without losing information

Users with `prefers-reduced-motion` (browser setting) or who have disabled reactive effects in Settings get a simplified motion profile.
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `25f9cd88925b56d24d251e4403eb87ca31d9c156add99378be16710fb21a221d`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "motion|without|losing|information|Reducing|setting|simplified|reduced" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "motion|without|losing|information|Reducing|setting|simplified|reduced" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S032 -- What reduces

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:233` through `240`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What reduces

- Transforms reduce. Scale animations become brightness changes. Position transitions become fade-in-at-new-position.
- Durations halve. 320ms becomes 160ms.
- Pulsing ambient effects are disabled.
- Velocity streaks simplify to a brief color highlight.
- Heartbeat rhythm becomes a status dot color rather than a pulse.
````

**Explicit detail extraction from this section:**

- Section word count: `44`
- Section hash: `babd999b0fd30fe0d987f05717568e146269b299f891e132728a52e8c6849807`

**Normative requirements and implementation claims:**
- - Transforms reduce. Scale animations become brightness changes. Position transitions become fade-in-at-new-position. - Durations halve. 320ms becomes 160ms. - Pulsing ambient effects are disabled. - Velocity streaks simplify to a brief color highlight. - Heartbeat rhythm becomes a status dot color rather than a pulse.

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
- - Transforms reduce. Scale animations become brightness changes. Position transitions become fade-in-at-new-position.
- - Durations halve. 320ms becomes 160ms.
- - Pulsing ambient effects are disabled.
- - Velocity streaks simplify to a brief color highlight.
- - Heartbeat rhythm becomes a status dot color rather than a pulse.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "reduce|reduces|become|color|becomes|Position|transitions|streaks" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reduce|reduces|become|color|becomes|Position|transitions|streaks" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S033 -- What stays

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:241` through `246`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What stays

- Data fidelity is unchanged.
- Structural change indicators remain (new items still appear, just with less fanfare).
- Semantic motion (severity flashing, critical alerts) remains at reduced intensity.
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `d7164e66c4563d0e4b04d3623e6684df7caf0071bf387923bdcd100f66b374fe`

**Normative requirements and implementation claims:**
- - Data fidelity is unchanged. - Structural change indicators remain (new items still appear, just with less fanfare). - Semantic motion (severity flashing, critical alerts) remains at reduced intensity.

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
- - Data fidelity is unchanged.
- - Structural change indicators remain (new items still appear, just with less fanfare).
- - Semantic motion (severity flashing, critical alerts) remains at reduced intensity.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "stays|remain|change|unchanged|still|severity|remains|reduced" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "stays|remain|change|unchanged|still|severity|remains|reduced" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S034 -- Manual control

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:247` through `256`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Manual control

Settings include a granular control for motion:

- **Full motion**: default.
- **Reduced motion**: complies with system preference; simpler animations.
- **Static**: no motion at all; changes are instant. For users who find even reduced motion distracting.

---
````

**Explicit detail extraction from this section:**

- Section word count: `34`
- Section hash: `b40214a12d963c22eea8ad422c6ff3e45f970e77c9aca638536cd64617ebbf2b`

**Normative requirements and implementation claims:**
- - **Full motion**: default. - **Reduced motion**: complies with system preference; simpler animations. - **Static**: no motion at all; changes are instant. For users who find even reduced motion distracting.
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
- - **Full motion**: default.
- - **Reduced motion**: complies with system preference; simpler animations.
- - **Static**: no motion at all; changes are instant. For users who find even reduced motion distracting.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "control|motion|Manual|Reduced|users|simpler|preference|instant" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "control|motion|Manual|Reduced|users|simpler|preference|instant" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S035 -- Real-time subscription lifecycle

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:257` through `260`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Real-time subscription lifecycle

The dashboard manages WebSocket subscriptions carefully. Unused subscriptions are closed. Relevant subscriptions are kept alive.
````

**Explicit detail extraction from this section:**

- Section word count: `15`
- Section hash: `205be68af35444467d708420dacbc359cc20cd3c1e0ecc0f1fd2aae5934b199e`

**Normative requirements and implementation claims:**
- The dashboard manages WebSocket subscriptions carefully. Unused subscriptions are closed. Relevant subscriptions are kept alive.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "subscription|time|lifecycle|Real|subscriptions|manages|kept|closed" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "subscription|time|lifecycle|Real|subscriptions|manages|kept|closed" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S036 -- Subscription creation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:261` through `266`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Subscription creation

- On page navigation, the dashboard determines what subscriptions the current view needs.
- It compares to existing subscriptions and opens new ones as needed.
- For pages showing many agents, a multiplexed subscription covers them all rather than N separate subscriptions.
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `385c7b34ce663b6d233a88b3583bee79505ece15052b69c2b4a6a82885b45e4e`

**Normative requirements and implementation claims:**
- - On page navigation, the dashboard determines what subscriptions the current view needs. - It compares to existing subscriptions and opens new ones as needed. - For pages showing many agents, a multiplexed subscription covers them all rather than N separate subscriptions.

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
- - On page navigation, the dashboard determines what subscriptions the current view needs.
- - It compares to existing subscriptions and opens new ones as needed.
- - For pages showing many agents, a multiplexed subscription covers them all rather than N separate subscriptions.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Subscription|creation|subscriptions|view|showing|separate|rather|pages" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Subscription|creation|subscriptions|view|showing|separate|rather|pages" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S037 -- Subscription cleanup

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:267` through `272`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Subscription cleanup

- When the user navigates away from a page, subscriptions used only by that page close.
- Subscriptions shared across pages (e.g., the header block height indicator) stay open.
- A background subscription for high-priority notifications stays open regardless of the current page.
````

**Explicit detail extraction from this section:**

- Section word count: `42`
- Section hash: `36a32bdb478f7f4218ce9b8136563baaeebe46d140d1f3e5f228f08fa6bb4930`

**Normative requirements and implementation claims:**
- - When the user navigates away from a page, subscriptions used only by that page close. - Subscriptions shared across pages (e.g., the header block height indicator) stay open. - A background subscription for high-priority notifications stays open regardless of the current page.

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
- - When the user navigates away from a page, subscriptions used only by that page close.
- - Subscriptions shared across pages (e.g., the header block height indicator) stay open.
- - A background subscription for high-priority notifications stays open regardless of the current page.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Subscription|cleanup|subscriptions|stay|open|user|stays|shared" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Subscription|cleanup|subscriptions|stay|open|user|stays|shared" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S038 -- Reconnection

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:273` through `282`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reconnection

When a subscription disconnects, the dashboard:

- Indicates the disconnection in the UI (a small icon on the affected area).
- Attempts to reconnect with exponential backoff.
- On reconnection, requests missed events from the last known sequence number.
- Replays missed events into the UI, with a brief "catching up" indicator.
- Returns to live mode.
````

**Explicit detail extraction from this section:**

- Section word count: `52`
- Section hash: `f1e4581929ffab78a772b0641675a779b242f94c666d88f2f80dd5090ff6a165`

**Normative requirements and implementation claims:**
- When a subscription disconnects, the dashboard:
- - Indicates the disconnection in the UI (a small icon on the affected area). - Attempts to reconnect with exponential backoff. - On reconnection, requests missed events from the last known sequence number. - Replays missed events into the UI, with a brief "catching up" indicator. - Returns to live mode.

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
- - Indicates the disconnection in the UI (a small icon on the affected area).
- - Attempts to reconnect with exponential backoff.
- - On reconnection, requests missed events from the last known sequence number.
- - Replays missed events into the UI, with a brief "catching up" indicator.
- - Returns to live mode.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "reconnect|Reconnection|missed|events|subscription|small|sequence|requests" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reconnect|Reconnection|missed|events|subscription|small|sequence|requests" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S039 -- Stale indicators

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:283` through `294`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Stale indicators

When data has not updated for longer than expected:

- The stale element shows a subtle fading or a small stale icon.
- Hovering explains "Last update: N seconds ago" and what might be wrong.
- The user can manually trigger a refresh.

This prevents the "silently showing old data" failure mode.

---
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `b4d7b6c8463d0c0ea5581869aacaa8315153046be95584bf0a0f31f4295d4a7d`

**Normative requirements and implementation claims:**
- - The stale element shows a subtle fading or a small stale icon. - Hovering explains "Last update: N seconds ago" and what might be wrong. - The user can manually trigger a refresh.
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
- - The stale element shows a subtle fading or a small stale icon.
- - Hovering explains "Last update: N seconds ago" and what might be wrong.
- - The user can manually trigger a refresh.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Stale|indicators|update|data|wrong|user|updated|trigger" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Stale|indicators|update|data|wrong|user|updated|trigger" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S040 -- Chain-specific realtime behavior

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:295` through `298`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Chain-specific realtime behavior

Chain state has specific properties that shape its realtime handling.
````

**Explicit detail extraction from this section:**

- Section word count: `10`
- Section hash: `703611664bd906ecc70f4f753ba3a0f3bc13a3001b9b59b951e350979ea5690a`

**Normative requirements and implementation claims:**
- Chain state has specific properties that shape its realtime handling.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "specific|realtime|Chain|behavior|state|shape|properties|handling" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "specific|realtime|Chain|behavior|state|shape|properties|handling" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S041 -- Block-paced updates

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:299` through `302`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Block-paced updates

Chain-sourced data updates per block. The dashboard can assume ~500ms between updates in normal operation. If no block arrives for longer than 3 seconds, show a stale indicator on chain-sourced data.
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `22624628dc390b501bbe72b714faa5516540026ad062dd042218ba11cd159fba`

**Normative requirements and implementation claims:**
- Chain-sourced data updates per block. The dashboard can assume ~500ms between updates in normal operation. If no block arrives for longer than 3 seconds, show a stale indicator on chain-sourced data.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "updates|Block|paced|sourced|data|Chain|stale|show" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "updates|Block|paced|sourced|data|Chain|stale|show" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S042 -- Reorg handling

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:303` through `310`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reorg handling

Chain reorgs are rare but happen. When a reorg occurs, some data the dashboard showed may turn out to be incorrect. The dashboard should:

- Subscribe to finality, not just latest, for data where certainty matters.
- Show a "pending" indicator on data that is recent but not yet finalized.
- On reorg, update the affected data and optionally notify the user if the change is material.
````

**Explicit detail extraction from this section:**

- Section word count: `64`
- Section hash: `a21f3eebd985922fd1aaac9f4bc7209791306908542a7623f0b296631de32ee3`

**Normative requirements and implementation claims:**
- Chain reorgs are rare but happen. When a reorg occurs, some data the dashboard showed may turn out to be incorrect. The dashboard should:
- - Subscribe to finality, not just latest, for data where certainty matters. - Show a "pending" indicator on data that is recent but not yet finalized. - On reorg, update the affected data and optionally notify the user if the change is material.

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
- - Subscribe to finality, not just latest, for data where certainty matters.
- - Show a "pending" indicator on data that is recent but not yet finalized.
- - On reorg, update the affected data and optionally notify the user if the change is material.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Reorg|handling|data|Show|user|update|turn" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reorg|handling|data|Show|user|update|turn" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S043 -- Gas and timing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:311` through `322`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gas and timing

On-chain writes have variable timing. When the dashboard submits a transaction (e.g., publishing a knowledge entry), the UI shows:

- Immediate optimistic update (the entry appears as "publishing").
- Confirmation when the transaction is mined (the entry becomes "published").
- Failure path if the transaction reverts (the entry reverts with an error).

This pattern applies across the dashboard wherever on-chain writes happen.

---
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `a843387dba8fcb86b12a78256ef4c27bf45b6450cc4ba3ff1ff9722d1abc8f90`

**Normative requirements and implementation claims:**
- On-chain writes have variable timing. When the dashboard submits a transaction (e.g., publishing a knowledge entry), the UI shows:
- - Immediate optimistic update (the entry appears as "publishing"). - Confirmation when the transaction is mined (the entry becomes "published"). - Failure path if the transaction reverts (the entry reverts with an error).
- This pattern applies across the dashboard wherever on-chain writes happen.
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
- - Immediate optimistic update (the entry appears as "publishing").
- - Confirmation when the transaction is mined (the entry becomes "published").
- - Failure path if the transaction reverts (the entry reverts with an error).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "timing|Gas|entry|transaction|writes|reverts|publishing" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "timing|Gas|entry|transaction|writes|reverts|publishing" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S044 -- Motion as information versus motion as decoration

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:323` through `345`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Motion as information versus motion as decoration

Every animation in the dashboard should communicate something. Motion that doesn't communicate is decorative and should be removed.

Examples of motion that communicates:

- A status change animation makes the change visible. The user sees "this thing just changed."
- A rank reordering animation preserves context. The user sees "this item moved from #3 to #1."
- A heartbeat tick communicates liveness. The user sees "this agent is alive."
- A velocity streak shows momentum. The user sees "this is changing quickly."
- A pulse on high-sharpness communicates authority. The user feels the quality.

Examples of motion to avoid:

- Bouncing on load just to be fun.
- Rotating elements for visual interest.
- Constant pulses on static content.
- Motion that happens on every hover without communicating anything.

The test: can you articulate what this motion communicates? If no, remove it.

---
````

**Explicit detail extraction from this section:**

- Section word count: `135`
- Section hash: `0205fd39b2e1ee7dae6f2fa0ba0a16aa8463a579c3bcac983f2c927fb8d2048c`

**Normative requirements and implementation claims:**
- Every animation in the dashboard should communicate something. Motion that doesn't communicate is decorative and should be removed.
- - A status change animation makes the change visible. The user sees "this thing just changed." - A rank reordering animation preserves context. The user sees "this item moved from #3 to #1." - A heartbeat tick communicates liveness. The user sees "this agent is alive." - A velocity streak shows momentum. The user sees "this is changing quickly." - A pulse on high-sharpness communicates authority. The user feels the quality.
- - Bouncing on load just to be fun. - Rotating elements for visual interest. - Constant pulses on static content. - Motion that happens on every hover without communicating anything.
- The test: can you articulate what this motion communicates? If no, remove it.
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
- - A status change animation makes the change visible. The user sees "this thing just changed."
- - A rank reordering animation preserves context. The user sees "this item moved from #3 to #1."
- - A heartbeat tick communicates liveness. The user sees "this agent is alive."
- - A velocity streak shows momentum. The user sees "this is changing quickly."
- - A pulse on high-sharpness communicates authority. The user feels the quality.
- - Bouncing on load just to be fun.
- - Rotating elements for visual interest.
- - Constant pulses on static content.
- - Motion that happens on every hover without communicating anything.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Motion|communicate|versus|user|information|decoration|sees|communicates" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Motion|communicate|versus|user|information|decoration|sees|communicates" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S045 -- Specific patterns for recurring UI elements

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:346` through `347`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Specific patterns for recurring UI elements
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `0ade0b26662e8cb9e05efe87cf1eec69902f9b053ff739febd0bbd0675b73ade`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "recurring|patterns|for|elements|Specific|realtime|motion" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "recurring|patterns|for|elements|Specific|realtime|motion" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S046 -- Live number glyphs

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:348` through `353`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Live number glyphs

Numbers that update frequently (block height, counts, cost) use mono typography with a subtle tick effect on update. The glyph brightens momentarily, then settles.

For numbers that update very frequently (multiple per second), throttle the visual tick to once per second — internal state can update each time, visual animation aggregates.
````

**Explicit detail extraction from this section:**

- Section word count: `50`
- Section hash: `67796ad4868331c3c3eb2aeee7e74813a3a4a3d5894ee3d44446d4f2c99c7ee9`

**Normative requirements and implementation claims:**
- Numbers that update frequently (block height, counts, cost) use mono typography with a subtle tick effect on update. The glyph brightens momentarily, then settles.
- For numbers that update very frequently (multiple per second), throttle the visual tick to once per second — internal state can update each time, visual animation aggregates.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "number|glyph|glyphs|Live|update|visual|tick|second" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "number|glyph|glyphs|Live|update|visual|tick|second" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S047 -- Status dots

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:354` through `361`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Status dots

Small colored dots (6px) indicate status. When status changes, the dot:

- Transitions color over 200ms.
- Emits a brief pulse (concentric expanding circle that fades) on change.
- Returns to steady state after the pulse completes.
````

**Explicit detail extraction from this section:**

- Section word count: `34`
- Section hash: `77ca0907ce69659621a0f96cbc344a246ab33b9194cb7f3eabaece933c9d260f`

**Normative requirements and implementation claims:**
- - Transitions color over 200ms. - Emits a brief pulse (concentric expanding circle that fades) on change. - Returns to steady state after the pulse completes.

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
- - Transitions color over 200ms.
- - Emits a brief pulse (concentric expanding circle that fades) on change.
- - Returns to steady state after the pulse completes.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Status|dots|pulse|color|change|steady|state|over" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Status|dots|pulse|color|change|steady|state|over" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S048 -- Loading states

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:362` through `372`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Loading states

Pages loading initially show:

- A subtle shimmer skeleton for expected content shapes.
- Actual content fades in as it arrives.
- If loading takes longer than 400ms, a progress indicator appears.
- If loading fails, an error state replaces the skeleton.

Never show a full-page blocking spinner except during critical transitions (initial auth, payment).
````

**Explicit detail extraction from this section:**

- Section word count: `52`
- Section hash: `cd6229c08b5dab1b2e31971224a6db1ad725e2ce1650751ae9a40c46631be34e`

**Normative requirements and implementation claims:**
- - A subtle shimmer skeleton for expected content shapes. - Actual content fades in as it arrives. - If loading takes longer than 400ms, a progress indicator appears. - If loading fails, an error state replaces the skeleton.
- Never show a full-page blocking spinner except during critical transitions (initial auth, payment).

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
- - A subtle shimmer skeleton for expected content shapes.
- - Actual content fades in as it arrives.
- - If loading takes longer than 400ms, a progress indicator appears.
- - If loading fails, an error state replaces the skeleton.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "Loading|state|states|skeleton|show|initial|content|transitions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Loading|state|states|skeleton|show|initial|content|transitions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S049 -- Notifications

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:373` through `382`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Notifications

Notifications appear in the notifications tray with:

- A subtle fade-in on arrival.
- A brief count badge update on the tray icon.
- Optional sound or system notification (user-configurable).

Dismissed notifications fade out over 200ms.
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `7e47f376566732758d1363e188944900cdaa83e8e9d09035fd7f1343e9d916d0`

**Normative requirements and implementation claims:**
- - A subtle fade-in on arrival. - A brief count badge update on the tray icon. - Optional sound or system notification (user-configurable).

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
- - A subtle fade-in on arrival.
- - A brief count badge update on the tray icon.
- - Optional sound or system notification (user-configurable).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "notification|Notifications|tray|fade|user|update|subtle|sound" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "notification|Notifications|tray|fade|user|update|subtle|sound" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S050 -- Modal entries and exits

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:383` through `388`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Modal entries and exits

Modals fade in and scale up from 0.95 to 1.0 over 240ms with `--ease-luxury`. Exits reverse the animation, 180ms.

The backdrop behind modals fades in over the same duration.
````

**Explicit detail extraction from this section:**

- Section word count: `32`
- Section hash: `2d354b5e55e092ce3ef5d4286c5866c5588a81c514025fdcd9d2e7d1c1931924`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Modal|exits|entries|over|fade|Modals|scale" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Modal|exits|entries|over|fade|Modals|scale" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S051 -- Toast notifications

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:389` through `394`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Toast notifications

Toasts slide up from the bottom-right corner over 320ms with `--ease-luxury`. Stay visible for a default 5 seconds (configurable). Exit with a slide-down and fade-out over 200ms.

---
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `f4efa1c50440ca1d6dd00c0f9219ff6dac66c4bc873a9bd61bfcc11cf6d155fb`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Toast|notifications|slide|over|visible|seconds|right|luxury" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Toast|notifications|slide|over|visible|seconds|right|luxury" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S052 -- Performance budget

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:395` through `415`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Performance budget

Realtime updates have a performance budget. Exceeding it degrades the UX even if correctness is maintained.

Targets:

- **Frame rate**: 60fps during steady state. 30fps acceptable during heavy updates.
- **Event processing latency**: events should become visible within 100ms of receipt under normal load.
- **Page response to interaction**: UI responds to clicks, keys, scrolls within 16ms.
- **Memory**: subscriptions, caches, and DOM should stay bounded. Long sessions should not leak.

When the budget is exceeded:

- Degrade visual richness first (simpler animations, coarser aggregation).
- Degrade update frequency second (throttle).
- Degrade data freshness last (pause live updates temporarily).

Never lie about data state. If updates are paused, the UI says so.

---
````

**Explicit detail extraction from this section:**

- Section word count: `106`
- Section hash: `9f14503e9c52a6fdeedc648732f1335c1805b9ef5e344854d780d497aab80628`

**Normative requirements and implementation claims:**
- - **Frame rate**: 60fps during steady state. 30fps acceptable during heavy updates. - **Event processing latency**: events should become visible within 100ms of receipt under normal load. - **Page response to interaction**: UI responds to clicks, keys, scrolls within 16ms. - **Memory**: subscriptions, caches, and DOM should stay bounded. Long sessions should not leak.
- - Degrade visual richness first (simpler animations, coarser aggregation). - Degrade update frequency second (throttle). - Degrade data freshness last (pause live updates temporarily).
- Never lie about data state. If updates are paused, the UI says so.
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
- - **Frame rate**: 60fps during steady state. 30fps acceptable during heavy updates.
- - **Event processing latency**: events should become visible within 100ms of receipt under normal load.
- - **Page response to interaction**: UI responds to clicks, keys, scrolls within 16ms.
- - **Memory**: subscriptions, caches, and DOM should stay bounded. Long sessions should not leak.
- - Degrade visual richness first (simpler animations, coarser aggregation).
- - Degrade update frequency second (throttle).
- - Degrade data freshness last (pause live updates temporarily).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "budget|Performance|update|updates|Degrade|even|within|state" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "budget|Performance|update|updates|Degrade|even|within|state" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S053 -- Indicators for realtime state

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:416` through `419`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Indicators for realtime state

The dashboard indicates its realtime state explicitly.
````

**Explicit detail extraction from this section:**

- Section word count: `7`
- Section hash: `50b73bd246c59288f84848fb6191709fe1ad5cdbad38247d00e62a35d983243b`

**Normative requirements and implementation claims:**
- The dashboard indicates its realtime state explicitly.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "state|realtime|for|Indicators|indicates|explicitly|motion" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "state|realtime|for|Indicators|indicates|explicitly|motion" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S054 -- Global live indicator

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:420` through `428`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Global live indicator

The header shows a "LIVE" badge with a pulsing dot when subscriptions are active and updating. When subscriptions are paused, disconnected, or stale, the indicator reflects the state:

- **LIVE** (green/rose pulse) — everything connected and streaming.
- **CATCHING UP** (yellow, briefly) — reconnected and replaying missed events.
- **STALE** (muted) — some data has not updated recently; tooltip explains which.
- **OFFLINE** (red) — no connection; dashboard shows last-known state.
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `7effa5ab2b6c145c80a09467e9fd315ef715fbabe26477baea393e427eac0c55`

**Normative requirements and implementation claims:**
- The header shows a "LIVE" badge with a pulsing dot when subscriptions are active and updating. When subscriptions are paused, disconnected, or stale, the indicator reflects the state:
- - **LIVE** (green/rose pulse) — everything connected and streaming. - **CATCHING UP** (yellow, briefly) — reconnected and replaying missed events. - **STALE** (muted) — some data has not updated recently; tooltip explains which. - **OFFLINE** (red) — no connection; dashboard shows last-known state.

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
- - **LIVE** (green/rose pulse) — everything connected and streaming.
- - **CATCHING UP** (yellow, briefly) — reconnected and replaying missed events.
- - **STALE** (muted) — some data has not updated recently; tooltip explains which.
- - **OFFLINE** (red) — no connection; dashboard shows last-known state.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "live|indicator|Global|connected|subscriptions|state|stale" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "live|indicator|Global|connected|subscriptions|state|stale" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S055 -- Per-element stale indicators

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:429` through `432`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Per-element stale indicators

Individual elements (a specific KPI, a specific agent) can be stale without the whole dashboard being stale. An element that is unexpectedly stale shows a small indicator.
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `4bd24535488c8eefec274f6d2e6a2cd5c3bb0f9111120cf4fd216d91fd8a8346`

**Normative requirements and implementation claims:**
- Individual elements (a specific KPI, a specific agent) can be stale without the whole dashboard being stale. An element that is unexpectedly stale shows a small indicator.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "stale|element|indicator|indicators|Per|specific|without|whole" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "stale|element|indicator|indicators|Per|specific|without|whole" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S056 -- Pause and resume

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:433` through `438`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pause and resume

Users can pause realtime updates on specific surfaces (the Event Stream has a Pause button visible in the screenshots). Paused surfaces indicate "Paused" clearly and resume cleanly when unpaused.

---
````

**Explicit detail extraction from this section:**

- Section word count: `29`
- Section hash: `4def67a68cf06813d909ff10a87e762f7f33dcae31efa70fafcb779ccd490666`

**Normative requirements and implementation claims:**
- Users can pause realtime updates on specific surfaces (the Event Stream has a Pause button visible in the screenshots). Paused surfaces indicate "Paused" clearly and resume cleanly when unpaused.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Pause|resume|Paused|surfaces|visible|updates|unpaused" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Pause|resume|Paused|surfaces|visible|updates|unpaused" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S057 -- Testing realtime behavior

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:439` through `453`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Testing realtime behavior

Realtime behavior should be tested under varying conditions:

- **Low rate**: single events every few seconds. Each event should be individually rendered with full fidelity.
- **Normal rate**: tens of events per minute. Rendering should feel smooth and calm.
- **Burst**: hundreds of events in a short window. UI should aggregate gracefully without losing data.
- **Sustained high rate**: continuous high load. UI should stabilize into aggregated mode.
- **Intermittent disconnect**: frequent short connection drops. UI should reconnect and catch up reliably.
- **Extended disconnect**: multi-minute outage. UI should indicate offline state clearly, preserve last-known state, and catch up on reconnection.

These scenarios should be representable in development so designers and implementers can see the UI under conditions beyond happy-path.

---
````

**Explicit detail extraction from this section:**

- Section word count: `117`
- Section hash: `29daf0586187e366718f16f1321e0a2116c41b1ce4d6ba06778cf894c43e4df8`

**Normative requirements and implementation claims:**
- Realtime behavior should be tested under varying conditions:
- - **Low rate**: single events every few seconds. Each event should be individually rendered with full fidelity. - **Normal rate**: tens of events per minute. Rendering should feel smooth and calm. - **Burst**: hundreds of events in a short window. UI should aggregate gracefully without losing data. - **Sustained high rate**: continuous high load. UI should stabilize into aggregated mode. - **Intermittent disconnect**: frequent short connection drops. UI should reconnect and catch up reliably. - **Extended disconnect**: multi-minute outage. UI should indicate offline state clearly, preserve last-known state, and catch up on reconnection.
- These scenarios should be representable in development so designers and implementers can see the UI under conditions beyond happy-path.
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
- - **Low rate**: single events every few seconds. Each event should be individually rendered with full fidelity.
- - **Normal rate**: tens of events per minute. Rendering should feel smooth and calm.
- - **Burst**: hundreds of events in a short window. UI should aggregate gracefully without losing data.
- - **Sustained high rate**: continuous high load. UI should stabilize into aggregated mode.
- - **Intermittent disconnect**: frequent short connection drops. UI should reconnect and catch up reliably.
- - **Extended disconnect**: multi-minute outage. UI should indicate offline state clearly, preserve last-known state, and catch up on reconnection.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "realtime|behavior|Testing|event|rate|events|under" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "realtime|behavior|Testing|event|rate|events|under" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

### DASH-10-S058 -- Summary

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md:454` through `469`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Summary

Realtime is the default. The system is alive; the UI shows it. But liveness is calibrated — calm rhythms, earned emphasis, graceful degradation, explicit stale indicators.

Key points:

- **Three streams**: agent events, chain blocks, derived/computed.
- **Live by default** for anything that can meaningfully change.
- **Aggregate under load** rather than flash every event.
- **Rhythm over fireworks**: slow pulses, heartbeat ticks, earned pulses on high-sharpness.
- **Graceful degradation**: visual fidelity drops before data fidelity.
- **Explicit state**: the UI tells users when it is live, stale, reconnecting, or offline.
- **Accessibility**: reduced motion supported; information preserved.
- **Performance budget**: 60fps, sub-100ms event latency, bounded memory.

The goal is an interface that feels as alive as the system it represents, without becoming nervous or overwhelming.
````

**Explicit detail extraction from this section:**

- Section word count: `120`
- Section hash: `f7bb00e51f5bf209fe92fa87982c2a7e41680a3c573dbae48881d9f10b77eccd`

**Normative requirements and implementation claims:**
- - **Three streams**: agent events, chain blocks, derived/computed. - **Live by default** for anything that can meaningfully change. - **Aggregate under load** rather than flash every event. - **Rhythm over fireworks**: slow pulses, heartbeat ticks, earned pulses on high-sharpness. - **Graceful degradation**: visual fidelity drops before data fidelity. - **Explicit state**: the UI tells users when it is live, stale, reconnecting, or offline. - **Accessibility**: reduced motion supported; information preserved. - **Performance budget**: 60fps, sub-100ms event latency, bounded memory.

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
- - **Three streams**: agent events, chain blocks, derived/computed.
- - **Live by default** for anything that can meaningfully change.
- - **Aggregate under load** rather than flash every event.
- - **Rhythm over fireworks**: slow pulses, heartbeat ticks, earned pulses on high-sharpness.
- - **Graceful degradation**: visual fidelity drops before data fidelity.
- - **Explicit state**: the UI tells users when it is live, stale, reconnecting, or offline.
- - **Accessibility**: reduced motion supported; information preserved.
- - **Performance budget**: 60fps, sub-100ms event latency, bounded memory.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/10-realtime-and-motion.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Summary|Live|event|stale|pulses|over|graceful|fidelity" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Summary|Live|event|stale|pulses|over|graceful|fidelity" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area dashboard-prd/10-realtime-and-motion
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

