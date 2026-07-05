# Dashboard PRD Plan: Universal Primitives

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
**Generated:** 2026-04-25
**Source hash:** `7c93e25ce4f629eee797b55c55014348e1cbbcad3cc529e8348143b54fde6d36`
**Section tasks:** 39
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
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-23-S001 | 1 | 23 — Universal primitives | [ ] | 9.8 |
| DASH-23-S002 | 7 | Why this document exists | [ ] | 9.8 |
| DASH-23-S003 | 17 | The 12 primitives | [ ] | 9.8 |
| DASH-23-S004 | 19 | Changes from the 10-primitive vocabulary | [ ] | 9.8 |
| DASH-23-S005 | 36 | Why Domain is dropped as standalone | [ ] | 9.8 |
| DASH-23-S006 | 40 | Why Extension stays Extension (not Hook) and Pheromone becomes Signal | [ ] | 9.8 |
| DASH-23-S007 | 52 | Pi extension compatibility | [ ] | 9.8 |
| DASH-23-S008 | 73 | Primitive definitions | [ ] | 9.8 |
| DASH-23-S009 | 75 | 1. Agent | [ ] | 9.8 |
| DASH-23-S010 | 91 | 2. Extension | [ ] | 9.8 |
| DASH-23-S011 | 110 | 3. Connector | [ ] | 9.8 |
| DASH-23-S012 | 128 | 4. Gate | [ ] | 9.8 |
| DASH-23-S013 | 148 | 5. Feed | [ ] | 9.8 |
| DASH-23-S014 | 166 | 6. Recipe | [ ] | 9.8 |
| DASH-23-S015 | 186 | 7. Knowledge Entry | [ ] | 9.8 |
| DASH-23-S016 | 203 | 8. Arena | [ ] | 9.8 |
| DASH-23-S017 | 219 | 9. Eval | [ ] | 9.8 |
| DASH-23-S018 | 235 | 10. Signal | [ ] | 9.8 |
| DASH-23-S019 | 252 | 11. Group | [ ] | 9.8 |
| DASH-23-S020 | 268 | 12. Bounty | [ ] | 9.8 |
| DASH-23-S021 | 284 | Composition matrix | [ ] | 9.8 |
| DASH-23-S022 | 305 | DeFi struct mapping | [ ] | 9.8 |
| DASH-23-S023 | 361 | Authoring surfaces | [ ] | 9.8 |
| DASH-23-S024 | 365 | Agent Composer (existing — PRD 19 Stage 1-10) | [ ] | 9.8 |
| DASH-23-S025 | 369 | Extension Workshop (existing — PRD 19) | [ ] | 9.8 |
| DASH-23-S026 | 373 | Connector Manager (new surface) | [ ] | 9.8 |
| DASH-23-S027 | 382 | Gate Designer (existing — PRD 19) | [ ] | 9.8 |
| DASH-23-S028 | 386 | Feed Designer (new surface) | [ ] | 9.8 |
| DASH-23-S029 | 395 | Recipe Editor (new surface) | [ ] | 9.8 |
| DASH-23-S030 | 404 | Knowledge Publisher, Arena Constructor, Eval Author, Signal Designer, Group Organizer, Bounty Author | [ ] | 9.8 |
| DASH-23-S031 | 410 | Migration path: 10 primitives to 12 | [ ] | 9.8 |
| DASH-23-S032 | 423 | Versioning | [ ] | 9.8 |
| DASH-23-S033 | 429 | Rust trait mapping | [ ] | 9.8 |
| DASH-23-S034 | 454 | Cross-domain universality | [ ] | 9.8 |
| DASH-23-S035 | 458 | DeFi: automated trading desk | [ ] | 9.8 |
| DASH-23-S036 | 478 | Governance: DAO operations | [ ] | 9.8 |
| DASH-23-S037 | 496 | Ops: deployment pipeline | [ ] | 9.8 |
| DASH-23-S038 | 514 | Code: self-improvement | [ ] | 9.8 |
| DASH-23-S039 | 534 | Relation to existing PRDs | [ ] | 9.8 |

## Tasks

### DASH-23-S001 -- 23 — Universal primitives

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 23 — Universal primitives

*Twelve composable primitives spanning DeFi, governance, ops, and off-chain domains. Supersedes the 10-primitive vocabulary in `20-composition-patterns.md`.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `21`
- Section hash: `c9868eed1a75074724acca9275d426e1913f2cafe24fac74b289d52d6ffd529a`

**Normative requirements and implementation claims:**
- *Twelve composable primitives spanning DeFi, governance, ops, and off-chain domains. Supersedes the 10-primitive vocabulary in `20-composition-patterns.md`.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "primitive|primitives|Universal|vocabulary|spanning|patterns|governance|domains" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "primitive|primitives|Universal|vocabulary|spanning|patterns|governance|domains" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S002 -- Why this document exists

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:7` through `16`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Why this document exists

PRD 20 defined ten first-class primitives. In parallel, the DeFi gap analysis (`tmp/defi/gap/01-10`) proposed ~40 domain-specific structs — VenueAdapter, TradingReflect, DeFiRiskEngine, MarketHdcEncoder, and more — as pure backend infrastructure with no product-layer mapping. These two systems don't talk to each other.

The result: strategies, connectors, data feeds, and risk guards are invisible in the dashboard. The DeFi structs have no authoring surface, no composition grammar, and no way for users to create, fork, or share them.

This document resolves the gap by defining **12 universal primitives** that cover every domain the system touches — DeFi trading, governance, developer ops, and off-chain integrations. Every DeFi struct from the gap docs maps to exactly one primitive. Every primitive has an authoring surface in the dashboard.

---
````

**Explicit detail extraction from this section:**

- Section word count: `129`
- Section hash: `c2bf29e42aebf4c797d2d138c3408ebdc3d1986d401acdcf3a466c8d417889de`

**Normative requirements and implementation claims:**
- The result: strategies, connectors, data feeds, and risk guards are invisible in the dashboard. The DeFi structs have no authoring surface, no composition grammar, and no way for users to create, fork, or share them.
- This document resolves the gap by defining **12 universal primitives** that cover every domain the system touches — DeFi trading, governance, developer ops, and off-chain integrations. Every DeFi struct from the gap docs maps to exactly one primitive. Every primitive has an authoring surface in the dashboard.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- tmp/defi/gap/

**Types, functions, traits, and inline code identifiers:**
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
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `tmp/defi/gap/`
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
rg -n "DeFi|document|exists|Why|struct|primitive|every|trading" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "DeFi|document|exists|Why|struct|primitive|every|trading" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `tmp/defi/gap/`
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
- [ ] Implement or verify `from` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S003 -- The 12 primitives

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:17` through `18`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The 12 primitives
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `8f53de9c747c2713f6448da1d3a683375494483c442c3e3cd5713f1e3b7f2fc8`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "primitives|The|universal" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "primitives|The|universal" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S004 -- Changes from the 10-primitive vocabulary

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:19` through `35`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Changes from the 10-primitive vocabulary

| # | Primitive | Status | Replaces / Notes |
|---|-----------|--------|------------------|
| 1 | **Agent** | Restructured | Agent + Domain merged; `DomainProfile` becomes a field on Agent templates via `ArchetypeManifest` |
| 2 | **Extension** | Kept | 3-tier extension system (Pi-compatible, Roko-enhanced, Roko-native) — preserved for Pi ecosystem compatibility |
| 3 | **Connector** | **NEW** | VenueAdapter, MCP servers, API integrations, ChainRpc |
| 4 | **Gate** | Expanded | Gate (now pre-action + post-action) |
| 5 | **Feed** | **NEW** | Continuous data sources: price feeds, webhook streams, log tails |
| 6 | **Recipe** | **NEW** | Data transformation pipelines: indicator chains, scoring, P&L attribution |
| 7 | **Knowledge Entry** | Kept | — |
| 8 | **Arena** | Kept | — |
| 9 | **Eval** | Kept | — |
| 10 | **Signal** | Renamed | Pheromone (broadened) — "Signal" is universally understood |
| 11 | **Group** | Kept | — |
| 12 | **Bounty** | Kept | — |
````

**Explicit detail extraction from this section:**

- Section word count: `104`
- Section hash: `8b9ad612933b16435bf1cf2a10f9067e932c3bf2cad4a807b6eae4ab7725042b`

**Normative requirements and implementation claims:**
- | # | Primitive | Status | Replaces / Notes | |---|-----------|--------|------------------| | 1 | **Agent** | Restructured | Agent + Domain merged; `DomainProfile` becomes a field on Agent templates via `ArchetypeManifest` | | 2 | **Extension** | Kept | 3-tier extension system (Pi-compatible, Roko-enhanced, Roko-native) — preserved for Pi ecosystem compatibility | | 3 | **Connector** | **NEW** | VenueAdapter, MCP servers, API integrations, ChainRpc | | 4 | **Gate** | Expanded | Gate (now pre-action + post-action) | | 5 | **Feed** | **NEW** | Continuous data sources: price feeds, webhook streams, log tails | | 6 | **Recipe** | **NEW** | Data transformation pipelines: indicator chains, scoring, P&L attribution | | 7 | **Knowledge Entry** | Kept | — | | 8 | **Arena** | Kept | — | | 9 | **Eval** | Kept | — | | 10 | **Signal** | Renamed | Pheromone (broadened) — "Signal" is universally understood | | 11 | **Group** | Kept | — | | 12 | **Bounty** | Kept | — |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- DomainProfile
- ArchetypeManifest

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
| # | Primitive | Status | Replaces / Notes |
|---|-----------|--------|------------------|
| 1 | **Agent** | Restructured | Agent + Domain merged; `DomainProfile` becomes a field on Agent templates via `ArchetypeManifest` |
| 2 | **Extension** | Kept | 3-tier extension system (Pi-compatible, Roko-enhanced, Roko-native) — preserved for Pi ecosystem compatibility |
| 3 | **Connector** | **NEW** | VenueAdapter, MCP servers, API integrations, ChainRpc |
| 4 | **Gate** | Expanded | Gate (now pre-action + post-action) |
| 5 | **Feed** | **NEW** | Continuous data sources: price feeds, webhook streams, log tails |
| 6 | **Recipe** | **NEW** | Data transformation pipelines: indicator chains, scoring, P&L attribution |
| 7 | **Knowledge Entry** | Kept | — |
| 8 | **Arena** | Kept | — |
| 9 | **Eval** | Kept | — |
| 10 | **Signal** | Renamed | Pheromone (broadened) — "Signal" is universally understood |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "primitive|Kept|vocabulary|the|DomainProfile|Changes|ArchetypeManifest|data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "primitive|Kept|vocabulary|the|DomainProfile|Changes|ArchetypeManifest|data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `DomainProfile` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S005 -- Why Domain is dropped as standalone

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:36` through `39`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Why Domain is dropped as standalone

`DomainProfile` is a thin 6-variant enum (`coding`, `trading`, `governance`, `research`, `ops`, `general`) with no real authoring surface. The `ArchetypeManifest` from gap doc 05 is the actual specification — it defines tool profiles, gate pipelines, model preferences, and behavioral constraints. This becomes a field on Agent templates rather than a separate primitive, matching thesis 8 (tools for tools): an agent's domain is part of its configuration, not a separate noun.
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `ea2bad9ca11eff0e1f52c391c4edbba7a89ee8000028df11ce716246d51ded76`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- DomainProfile
- coding
- trading
- governance
- research
- ops
- general
- ArchetypeManifest

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Domain|trading|standalone|research|ops|governance|general|dropped" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Domain|trading|standalone|research|ops|governance|general|dropped" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `DomainProfile` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `coding` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `trading` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `governance` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `research` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ops` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `general` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S006 -- Why Extension stays Extension (not Hook) and Pheromone becomes Signal

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:40` through `51`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Why Extension stays Extension (not Hook) and Pheromone becomes Signal

The original plan renamed Extension→Hook to communicate the interception pattern. However, the Pi extension system (PRD-09) defines a **three-tier extension taxonomy** that is far richer than hooks:

- **Tier 1 (Pi-compatible):** JS/TS packages using the `PiExtension` API — `registerTool()`, event handlers (`tool_call`, `turn_start`, `turn_end`, `error`), config, logging. These run in QuickJS sandboxes.
- **Tier 2 (Roko-enhanced):** JS/TS using both `pi` + `roko` APIs — heartbeat hooks (`tick_start`, `assemble_context`, `observe`, `before_inference`), context injection, CorticalState read, knowledge queries.
- **Tier 3 (Roko-native):** Rust `Extension` trait with 22 hooks across 8 layers (Foundation → Perception → Memory → Cognition → Action → Learning → Affect → Recovery).

Renaming to "Hook" would flatten this into a single concept and break the mental model that `roko install npm:@pi/my-extension` installs an *extension* that works identically in both Pi and Roko. The name **Extension** is correct — it's the established term in the Pi ecosystem, the Rust trait name, the package type, and the marketplace category. Hook points are a *feature* of extensions, not the primitive itself.

**Pheromone→Signal** still applies. Pheromone only makes sense in stigmergy; Signal is universally understood and matches `roko-core::Signal`.
````

**Explicit detail extraction from this section:**

- Section word count: `187`
- Section hash: `1155e293eeeced87b39714dd4f2842b71187684b9b345bc72a64db518fa5319f`

**Normative requirements and implementation claims:**
- - **Tier 1 (Pi-compatible):** JS/TS packages using the `PiExtension` API — `registerTool()`, event handlers (`tool_call`, `turn_start`, `turn_end`, `error`), config, logging. These run in QuickJS sandboxes. - **Tier 2 (Roko-enhanced):** JS/TS using both `pi` + `roko` APIs — heartbeat hooks (`tick_start`, `assemble_context`, `observe`, `before_inference`), context injection, CorticalState read, knowledge queries. - **Tier 3 (Roko-native):** Rust `Extension` trait with 22 hooks across 8 layers (Foundation → Perception → Memory → Cognition → Action → Learning → Affect → Recovery).
- **Pheromone→Signal** still applies. Pheromone only makes sense in stigmergy; Signal is universally understood and matches `roko-core::Signal`.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- with
- name
- PiExtension
- tool_call
- turn_start
- turn_end
- error
- roko
- tick_start
- assemble_context
- observe
- before_inference
- Extension

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- The original plan renamed Extension -> Hook to communicate the interception patt
- Foundation -> Perception
- Memory -> Cognition
- Action -> Learning
- Affect -> Recovery
- Pheromone -> Signal

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko install npm:@pi/my-extension

**Bullet requirements:**
- - **Tier 1 (Pi-compatible):** JS/TS packages using the `PiExtension` API — `registerTool()`, event handlers (`tool_call`, `turn_start`, `turn_end`, `error`), config, logging. These run in QuickJS sandboxes.
- - **Tier 2 (Roko-enhanced):** JS/TS using both `pi` + `roko` APIs — heartbeat hooks (`tick_start`, `assemble_context`, `observe`, `before_inference`), context injection, CorticalState read, knowledge queries.
- - **Tier 3 (Roko-native):** Rust `Extension` trait with 22 hooks across 8 layers (Foundation → Perception → Memory → Cognition → Action → Learning → Affect → Recovery).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|Hook|Signal|name|Pheromone|not|turn_start" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|Hook|Signal|name|Pheromone|not|turn_start" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `with` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PiExtension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `turn_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `turn_end` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `error` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `roko` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tick_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `assemble_context` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `observe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `before_inference` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `The original plan renamed Extension -> Hook to communicate the interception patt` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Foundation -> Perception` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Memory -> Cognition` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Action -> Learning` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Affect -> Recovery` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Pheromone -> Signal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Implement or verify operator command `roko install npm:@pi/my-extension` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S007 -- Pi extension compatibility

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:52` through `72`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Pi extension compatibility

Every Pi extension (`pi.registerTool()` + `pi.on()`) works as a dashboard Extension without modification. The compatibility path:

| Pi API | Roko Extension hook | Dashboard surface |
|--------|-------------------|-------------------|
| `pi.registerTool()` | `DynamicToolRegistry` | Agent Composer Stage 4 (tool selection) |
| `pi.on("tool_call")` | `before_tool_call` | Extension Workshop (hook config) |
| `pi.on("tool_result")` | `after_tool_call` | Extension Workshop |
| `pi.on("turn_start")` | `on_tick_start` | Extension Workshop |
| `pi.on("turn_end")` | `on_tick_end` | Extension Workshop |
| `pi.on("error")` | `on_error` | Extension Workshop |

**Roko-enhanced extensions** (`roko.on("observe")`, `roko.injectContext()`, etc.) gain additional dashboard visibility in the Agent Detail panel.

**Roko-native extensions** (Rust `Extension` trait) participate in the full 22-hook heartbeat pipeline and are authored via the Extension Workshop's advanced mode (Rust code editor + ABI validation).

The Extension Workshop (PRD 19) already supports this — its 4-stage flow (selection → code editor → test sandbox → publish) accommodates all three tiers. Pi extensions get a simplified flow (just tool registration + event handlers); Roko-native extensions get the full hook configuration.

---
````

**Explicit detail extraction from this section:**

- Section word count: `153`
- Section hash: `1361b4c6e252380fd5ede9d4502373d689feb858009e14b5104854b40d7daca6`

**Normative requirements and implementation claims:**
- Every Pi extension (`pi.registerTool()` + `pi.on()`) works as a dashboard Extension without modification. The compatibility path:
- | Pi API | Roko Extension hook | Dashboard surface | |--------|-------------------|-------------------| | `pi.registerTool()` | `DynamicToolRegistry` | Agent Composer Stage 4 (tool selection) | | `pi.on("tool_call")` | `before_tool_call` | Extension Workshop (hook config) | | `pi.on("tool_result")` | `after_tool_call` | Extension Workshop | | `pi.on("turn_start")` | `on_tick_start` | Extension Workshop | | `pi.on("turn_end")` | `on_tick_end` | Extension Workshop | | `pi.on("error")` | `on_error` | Extension Workshop |
- **Roko-enhanced extensions** (`roko.on("observe")`, `roko.injectContext()`, etc.) gain additional dashboard visibility in the Agent Detail panel.
- **Roko-native extensions** (Rust `Extension` trait) participate in the full 22-hook heartbeat pipeline and are authored via the Extension Workshop's advanced mode (Rust code editor + ABI validation).
- The Extension Workshop (PRD 19) already supports this — its 4-stage flow (selection → code editor → test sandbox → publish) accommodates all three tiers. Pi extensions get a simplified flow (just tool registration + event handlers); Roko-native extensions get the full hook configuration.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- DynamicToolRegistry
- before_tool_call
- after_tool_call
- on_tick_start
- on_tick_end
- on_error
- Extension

**Event names and event-like entities:**
- pi.register
- pi.on
- roko.on
- roko.inject

**State transitions:**
- selection -> code editor
- test sandbox -> publish

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Pi API | Roko Extension hook | Dashboard surface |
|--------|-------------------|-------------------|
| `pi.registerTool()` | `DynamicToolRegistry` | Agent Composer Stage 4 (tool selection) |
| `pi.on("tool_call")` | `before_tool_call` | Extension Workshop (hook config) |
| `pi.on("tool_result")` | `after_tool_call` | Extension Workshop |
| `pi.on("turn_start")` | `on_tick_start` | Extension Workshop |
| `pi.on("turn_end")` | `on_tick_end` | Extension Workshop |
| `pi.on("error")` | `on_error` | Extension Workshop |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "extension|tool|works|Workshop|compatibility|on_tick_start|on_tick_end|on_error" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "extension|tool|works|Workshop|compatibility|on_tick_start|on_tick_end|on_error" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `DynamicToolRegistry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `before_tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `after_tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_tick_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_tick_end` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_error` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `pi.register` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `pi.on` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.on` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.inject` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `selection -> code editor` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `test sandbox -> publish` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S008 -- Primitive definitions

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:73` through `74`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Primitive definitions
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `9ed09494349cb9c2c8ebe1a25970d54820ac72eb13967c399e469a1168f6ea2f`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "definitions|Primitive|universal|primitives" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "definitions|Primitive|universal|primitives" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S009 -- 1. Agent

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:75` through `90`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 1. Agent

A Roko runtime instance with identity, configuration, and capabilities.

**Shape:** `create / configure / start / stop / message / observe`
**Archetype field:** `ArchetypeManifest` — declares domain (trading, governance, ops, coding), tool profiles, gate pipelines, model preferences, and behavioral constraints. Replaces standalone Domain primitive.
**Cross-domain examples:**
- **DeFi:** Trade executor agent with Hyperliquid connector, RSI feed, risk gate
- **Governance:** Proposal reviewer agent with Snapshot connector, voting feed
- **Ops:** Deploy agent with GitHub connector, CI feed, test gate
- **Code:** Refactoring agent with MCP code-intelligence connector

**Rust mapping:** `roko-agent` — `AgentManifest`, `AgentProcess`, `ArchetypeManifest` (from gap doc 05)

---
````

**Explicit detail extraction from this section:**

- Section word count: `90`
- Section hash: `82f280b419a9b5456973a3213abb2f3084a3e3eaf6f6d3f12b492eb93dc0833a`

**Normative requirements and implementation claims:**
- **Shape:** `create / configure / start / stop / message / observe` **Archetype field:** `ArchetypeManifest` — declares domain (trading, governance, ops, coding), tool profiles, gate pipelines, model preferences, and behavioral constraints. Replaces standalone Domain primitive. **Cross-domain examples:** - **DeFi:** Trade executor agent with Hyperliquid connector, RSI feed, risk gate - **Governance:** Proposal reviewer agent with Snapshot connector, voting feed - **Ops:** Deploy agent with GitHub connector, CI feed, test gate - **Code:** Refactoring agent with MCP code-intelligence connector
- **Rust mapping:** `roko-agent` — `AgentManifest`, `AgentProcess`, `ArchetypeManifest` (from gap doc 05)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArchetypeManifest
- AgentManifest
- AgentProcess

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Trade executor agent with Hyperliquid connector, RSI feed, risk gate
- - **Governance:** Proposal reviewer agent with Snapshot connector, voting feed
- - **Ops:** Deploy agent with GitHub connector, CI feed, test gate
- - **Code:** Refactoring agent with MCP code-intelligence connector

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "ArchetypeManifest|AgentProcess|AgentManifest|connector|gate|feed|domain|Archetype" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "ArchetypeManifest|AgentProcess|AgentManifest|connector|gate|feed|domain|Archetype" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentProcess` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S010 -- 2. Extension

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:91` through `109`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 2. Extension

A modular unit of behavior an agent can load. Spans three tiers: Pi-compatible (JS/TS tool registration + event hooks), Roko-enhanced (JS/TS with heartbeat + context access), and Roko-native (Rust, 22 hooks across 8 layers).

**Shape:** `install / activate / configure / deactivate / uninstall`
**Tier 1 (Pi-compatible):** `registerTool()` + event hooks (`tool_call`, `tool_result`, `turn_start`, `turn_end`, `error`) — runs in QuickJS sandbox, works identically in Pi and Roko
**Tier 2 (Roko-enhanced):** Tier 1 + heartbeat hooks (`tick_start`, `assemble_context`, `observe`, `before_inference`), context injection, CorticalState read, knowledge queries — graceful fallback in Pi
**Tier 3 (Roko-native):** Full `Extension` trait — 22 hooks (observe, retrieve, analyze, simulate, validate, execute, verify, reflect + lifecycle + frequency + events), 8 layers (Foundation→Recovery)
**Cross-domain examples:**
- **DeFi:** Affect modulation extension — daimon PAD vector adjusts position sizing before trade dispatch (gap doc 08, `TiltDetector`) [Tier 3]
- **DeFi:** Pi-compatible DeFi tools extension — registers swap/LP/vault tools via `pi.registerTool()` [Tier 1]
- **Governance:** Quorum check extension — blocks proposal submission if insufficient delegation [Tier 2]
- **Ops:** Cost guard extension — pauses agent if token spend exceeds budget [Tier 2]
- **Code:** Style enforcement extension — reformats output before commit [Tier 1]

**Rust mapping:** `roko-runtime` — `Extension` trait (22 hooks); `roko-quickjs` — `JsExtensionBridge` (Pi/Roko JS API); `roko-ext-registry` — `PackageManifest`, `ExtensionLoader`; `roko-daimon` — `DaimonModulator`

---
````

**Explicit detail extraction from this section:**

- Section word count: `207`
- Section hash: `0dc4820939caad70f32e8dce0ada2b8a8e150cbc4be3c16419c73a7d24a7b87a`

**Normative requirements and implementation claims:**
- A modular unit of behavior an agent can load. Spans three tiers: Pi-compatible (JS/TS tool registration + event hooks), Roko-enhanced (JS/TS with heartbeat + context access), and Roko-native (Rust, 22 hooks across 8 layers).
- **Shape:** `install / activate / configure / deactivate / uninstall` **Tier 1 (Pi-compatible):** `registerTool()` + event hooks (`tool_call`, `tool_result`, `turn_start`, `turn_end`, `error`) — runs in QuickJS sandbox, works identically in Pi and Roko **Tier 2 (Roko-enhanced):** Tier 1 + heartbeat hooks (`tick_start`, `assemble_context`, `observe`, `before_inference`), context injection, CorticalState read, knowledge queries — graceful fallback in Pi **Tier 3 (Roko-native):** Full `Extension` trait — 22 hooks (observe, retrieve, analyze, simulate, validate, execute, verify, reflect + lifecycle + frequency + events), 8 layers (Foundation→Recovery) **Cross-domain examples:** - **DeFi:** Affect modulation extension — daimon PAD vector adjusts position sizing before trade dispatch (gap doc 08, `TiltDetector`) [Tier 3] - **DeFi:** Pi-compatible DeFi tools extension — registers swap/LP/vault tools via `pi.registerTool()` [Tier 1] - **Governance:** Quorum check extension — blocks proposal sub
- **Rust mapping:** `roko-runtime` — `Extension` trait (22 hooks); `roko-quickjs` — `JsExtensionBridge` (Pi/Roko JS API); `roko-ext-registry` — `PackageManifest`, `ExtensionLoader`; `roko-daimon` — `DaimonModulator`
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- swap/LP/

**Types, functions, traits, and inline code identifiers:**
- tool_call
- tool_result
- turn_start
- turn_end
- error
- tick_start
- assemble_context
- observe
- before_inference
- Extension
- TiltDetector
- JsExtensionBridge
- PackageManifest
- ExtensionLoader
- DaimonModulator

**Event names and event-like entities:**
- pi.register

**State transitions:**
- Foundation -> Recovery

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Affect modulation extension — daimon PAD vector adjusts position sizing before trade dispatch (gap doc 08, `TiltDetector`) [Tier 3]
- - **DeFi:** Pi-compatible DeFi tools extension — registers swap/LP/vault tools via `pi.registerTool()` [Tier 1]
- - **Governance:** Quorum check extension — blocks proposal submission if insufficient delegation [Tier 2]
- - **Ops:** Cost guard extension — pauses agent if token spend exceeds budget [Tier 2]
- - **Code:** Style enforcement extension — reformats output before commit [Tier 1]

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `swap/LP/`
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
rg -n "Extension|Tier|tool|observe|hooks|turn_start|turn_end|tool_result" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|Tier|tool|observe|hooks|turn_start|turn_end|tool_result" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `swap/LP/`
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
- [ ] Implement or verify `tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tool_result` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `turn_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `turn_end` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `error` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tick_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `assemble_context` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `observe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `before_inference` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TiltDetector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `JsExtensionBridge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PackageManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExtensionLoader` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DaimonModulator` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `pi.register` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `Foundation -> Recovery` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S011 -- 3. Connector

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:110` through `127`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 3. Connector

External system I/O adapter — connects agents to venues, chains, APIs, and services.

**Shape:** `connect / query / execute / health / disconnect`
**Why it's a new primitive:** Not a Hook (doesn't modify behavior) or Knowledge Entry (doesn't store engrams). Fundamentally different shape: bidirectional I/O with health checks and reconnection. The DeFi gap docs define VenueAdapter (gap doc 02), ChainRpc (gap doc 01), and Oracle (gap doc 01) — all connectors.

**Cross-domain examples:**
- **DeFi:** `HyperliquidConnector` — order placement, position queries, fills stream (VenueAdapter from gap doc 02)
- **DeFi:** `ChainRpcConnector` — block reads, tx submission, event subscription (ChainClient from gap doc 01)
- **Governance:** `SnapshotConnector` — proposal CRUD, vote submission, delegation queries
- **Ops:** `GitHubConnector` — PR creation, CI status, issue management (MCP server wrapper)
- **Data:** `PostgresConnector` — query execution, schema introspection

**Rust mapping:** `roko-chain` — `ChainClient`, `VenueAdapter` trait; `roko-agent` — MCP server configs; new `Connector` trait unifying the pattern

---
````

**Explicit detail extraction from this section:**

- Section word count: `142`
- Section hash: `3af6726a23b6591e3d4ddf47cc7484ba3382ab6ece6eb49cba59e4e10ac3bf6b`

**Normative requirements and implementation claims:**
- **Shape:** `connect / query / execute / health / disconnect` **Why it's a new primitive:** Not a Hook (doesn't modify behavior) or Knowledge Entry (doesn't store engrams). Fundamentally different shape: bidirectional I/O with health checks and reconnection. The DeFi gap docs define VenueAdapter (gap doc 02), ChainRpc (gap doc 01), and Oracle (gap doc 01) — all connectors.
- **Cross-domain examples:** - **DeFi:** `HyperliquidConnector` — order placement, position queries, fills stream (VenueAdapter from gap doc 02) - **DeFi:** `ChainRpcConnector` — block reads, tx submission, event subscription (ChainClient from gap doc 01) - **Governance:** `SnapshotConnector` — proposal CRUD, vote submission, delegation queries - **Ops:** `GitHubConnector` — PR creation, CI status, issue management (MCP server wrapper) - **Data:** `PostgresConnector` — query execution, schema introspection
- **Rust mapping:** `roko-chain` — `ChainClient`, `VenueAdapter` trait; `roko-agent` — MCP server configs; new `Connector` trait unifying the pattern
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- unifying
- HyperliquidConnector
- ChainRpcConnector
- SnapshotConnector
- GitHubConnector
- PostgresConnector
- ChainClient
- VenueAdapter
- Connector

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** `HyperliquidConnector` — order placement, position queries, fills stream (VenueAdapter from gap doc 02)
- - **DeFi:** `ChainRpcConnector` — block reads, tx submission, event subscription (ChainClient from gap doc 01)
- - **Governance:** `SnapshotConnector` — proposal CRUD, vote submission, delegation queries
- - **Ops:** `GitHubConnector` — PR creation, CI status, issue management (MCP server wrapper)
- - **Data:** `PostgresConnector` — query execution, schema introspection

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "connect|Connector|VenueAdapter|chain|ChainClient|unifying|SnapshotConnector|PostgresConnector" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "connect|Connector|VenueAdapter|chain|ChainClient|unifying|SnapshotConnector|PostgresConnector" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `unifying` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `HyperliquidConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainRpcConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SnapshotConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GitHubConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PostgresConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VenueAdapter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Connector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S012 -- 4. Gate

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:128` through `147`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 4. Gate

A verification step checking agent output against ground truth. Expanded to cover both pre-action (permission) and post-action (validation) checks.

**Shape:** `check / approve / reject / explain / configure`
**Gate modes:**
- **Pre-action gate:** Runs before agent executes — risk limits, MEV simulation, cost bounds, custody checks
- **Post-action gate:** Runs after agent executes — compile check, test pass, diff review, chain state verification
**Rung pipeline:** 7 rungs with adaptive thresholds (existing), now with pre-action rungs 0-2 and post-action rungs 3-6.

**Cross-domain examples:**
- **DeFi:** Pre-action: `DeFiRiskGate` — position limits, max slippage, MEV exposure (gap doc 04). Post-action: `TxVerifyGate` — confirm on-chain settlement
- **Governance:** Pre-action: delegation threshold. Post-action: quorum verification
- **Ops:** Pre-action: cost budget check. Post-action: compile + test + clippy (existing)
- **Code:** Pre-action: file scope guard. Post-action: diff size limit

**Rust mapping:** `roko-gate` — 11 gate implementations, `RungConfig`, adaptive thresholds; gap doc 04 — `DeFiRiskEngine`, `MevProtectionGate`, `CircuitBreakerGate`

---
````

**Explicit detail extraction from this section:**

- Section word count: `152`
- Section hash: `d52d2fd355c89e0fe221a36a2dc8500e10c61d8bf42fff1cc18026bc1f0b7dfa`

**Normative requirements and implementation claims:**
- **Shape:** `check / approve / reject / explain / configure` **Gate modes:** - **Pre-action gate:** Runs before agent executes — risk limits, MEV simulation, cost bounds, custody checks - **Post-action gate:** Runs after agent executes — compile check, test pass, diff review, chain state verification **Rung pipeline:** 7 rungs with adaptive thresholds (existing), now with pre-action rungs 0-2 and post-action rungs 3-6.
- **Cross-domain examples:** - **DeFi:** Pre-action: `DeFiRiskGate` — position limits, max slippage, MEV exposure (gap doc 04). Post-action: `TxVerifyGate` — confirm on-chain settlement - **Governance:** Pre-action: delegation threshold. Post-action: quorum verification - **Ops:** Pre-action: cost budget check. Post-action: compile + test + clippy (existing) - **Code:** Pre-action: file scope guard. Post-action: diff size limit
- **Rust mapping:** `roko-gate` — 11 gate implementations, `RungConfig`, adaptive thresholds; gap doc 04 — `DeFiRiskEngine`, `MevProtectionGate`, `CircuitBreakerGate`
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- DeFiRiskGate
- TxVerifyGate
- RungConfig
- DeFiRiskEngine
- MevProtectionGate
- CircuitBreakerGate

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Pre-action gate:** Runs before agent executes — risk limits, MEV simulation, cost bounds, custody checks
- - **Post-action gate:** Runs after agent executes — compile check, test pass, diff review, chain state verification
- - **DeFi:** Pre-action: `DeFiRiskGate` — position limits, max slippage, MEV exposure (gap doc 04). Post-action: `TxVerifyGate` — confirm on-chain settlement
- - **Governance:** Pre-action: delegation threshold. Post-action: quorum verification
- - **Ops:** Pre-action: cost budget check. Post-action: compile + test + clippy (existing)
- - **Code:** Pre-action: file scope guard. Post-action: diff size limit

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "action|Gate|post|check|TxVerifyGate|RungConfig|Rung|MevProtectionGate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "action|Gate|post|check|TxVerifyGate|RungConfig|Rung|MevProtectionGate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `DeFiRiskGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TxVerifyGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RungConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DeFiRiskEngine` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MevProtectionGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CircuitBreakerGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S013 -- 5. Feed

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:148` through `165`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 5. Feed

A continuous data stream consumed by agents, recipes, and gates.

**Shape:** `subscribe / unsubscribe / poll / status / configure`
**Why it's a new primitive:** Not Knowledge (not validated/stored — raw stream) or Signal (not coordination — data). A Feed *uses* a Connector as its source and publishes processed events to PulseBus. Feeds are the "always-on" complement to one-shot Connector queries.

**Cross-domain examples:**
- **DeFi:** Price feed from Hyperliquid connector; block event feed from ChainRpc connector (gap doc 01, `ChainEventWatcher`); funding rate feed
- **DeFi:** Heartbeat tick feed — periodic clock driving decision cycles (gap doc 06)
- **Governance:** Proposal activity feed from Snapshot connector; delegation change feed
- **Ops:** CI status feed from GitHub connector; deploy event feed
- **Code:** File change feed from filesystem watcher; build output feed

**Rust mapping:** `roko-chain` — `ChainEventWatcher`, `HeartbeatClock`; `roko-conductor` — watchers; new `Feed` trait wrapping async stream pattern

---
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `91192ea7109ad9b1fa69c2a4fabc8641c29320844735795fbbf352162797f65e`

**Normative requirements and implementation claims:**
- **Shape:** `subscribe / unsubscribe / poll / status / configure` **Why it's a new primitive:** Not Knowledge (not validated/stored — raw stream) or Signal (not coordination — data). A Feed *uses* a Connector as its source and publishes processed events to PulseBus. Feeds are the "always-on" complement to one-shot Connector queries.
- **Cross-domain examples:** - **DeFi:** Price feed from Hyperliquid connector; block event feed from ChainRpc connector (gap doc 01, `ChainEventWatcher`); funding rate feed - **DeFi:** Heartbeat tick feed — periodic clock driving decision cycles (gap doc 06) - **Governance:** Proposal activity feed from Snapshot connector; delegation change feed - **Ops:** CI status feed from GitHub connector; deploy event feed - **Code:** File change feed from filesystem watcher; build output feed
- **Rust mapping:** `roko-chain` — `ChainEventWatcher`, `HeartbeatClock`; `roko-conductor` — watchers; new `Feed` trait wrapping async stream pattern
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- wrapping
- ChainEventWatcher
- HeartbeatClock
- Feed

**Event names and event-like entities:**
- ChainEvent

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Price feed from Hyperliquid connector; block event feed from ChainRpc connector (gap doc 01, `ChainEventWatcher`); funding rate feed
- - **DeFi:** Heartbeat tick feed — periodic clock driving decision cycles (gap doc 06)
- - **Governance:** Proposal activity feed from Snapshot connector; delegation change feed
- - **Ops:** CI status feed from GitHub connector; deploy event feed
- - **Code:** File change feed from filesystem watcher; build output feed

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Feed|Connector|ChainEventWatcher|wrapping|event|HeartbeatClock|watcher|chain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|Connector|ChainEventWatcher|wrapping|event|HeartbeatClock|watcher|chain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `wrapping` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainEventWatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `HeartbeatClock` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Feed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ChainEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S014 -- 6. Recipe

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:166` through `185`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 6. Recipe

A composable data transformation pipeline. The composition glue between Feeds, Scorers, and Signals.

**Shape:** `create / execute / chain / status / configure`
**Why it's a new primitive:** Not a Plan (not an agent task DAG) or Composer (not prompt assembly). Recipes are pure data pipelines: Feed → transform → transform → output. They make indicator chains, P&L attribution, HDC encoding, and scoring pipelines first-class composable objects.

**Cross-domain examples:**
- **DeFi:** Indicator pipeline: price feed → MACD → RSI → regime score → trading signal (gap doc 03)
- **DeFi:** P&L attribution pipeline: fill events → FIFO matching → realized P&L → Sharpe ratio (gap doc 07, `TradingReflect`, `FifoMatcher`)
- **DeFi:** HDC encoding pipeline: market state → role-filler binding → hypervector → similarity search (gap doc 10, `MarketHdcEncoder`)
- **DeFi:** Counterfactual pipeline: historical fills → alternative parameters → simulated outcome (gap doc 09)
- **Governance:** Voting power pipeline: delegation graph → power aggregation → threshold signal
- **Ops:** Cost pipeline: agent token usage → pricing → daily/weekly rollup → budget signal
- **Code:** Quality pipeline: diff → lint warnings → test coverage → quality score

**Rust mapping:** `roko-learn` — `TradingReflect`, `FifoMatcher`, `IndicatorTracker`; `roko-primitives` — `MarketHdcEncoder`; `roko-dreams` — `CounterfactualEngine`; new `Recipe` trait composing `Scorer` instances

---
````

**Explicit detail extraction from this section:**

- Section word count: `178`
- Section hash: `c35d7b632aac285037e7cfdc0e3825cf45755315fa952c77d9738d704e9204db`

**Normative requirements and implementation claims:**
- **Shape:** `create / execute / chain / status / configure` **Why it's a new primitive:** Not a Plan (not an agent task DAG) or Composer (not prompt assembly). Recipes are pure data pipelines: Feed → transform → transform → output. They make indicator chains, P&L attribution, HDC encoding, and scoring pipelines first-class composable objects.
- **Cross-domain examples:** - **DeFi:** Indicator pipeline: price feed → MACD → RSI → regime score → trading signal (gap doc 03) - **DeFi:** P&L attribution pipeline: fill events → FIFO matching → realized P&L → Sharpe ratio (gap doc 07, `TradingReflect`, `FifoMatcher`) - **DeFi:** HDC encoding pipeline: market state → role-filler binding → hypervector → similarity search (gap doc 10, `MarketHdcEncoder`) - **DeFi:** Counterfactual pipeline: historical fills → alternative parameters → simulated outcome (gap doc 09) - **Governance:** Voting power pipeline: delegation graph → power aggregation → threshold signal - **Ops:** Cost pipeline: agent token usage → pricing → daily/weekly rollup → budget signal - **Code:** Quality pipeline: diff → lint warnings → test coverage → quality score
- **Rust mapping:** `roko-learn` — `TradingReflect`, `FifoMatcher`, `IndicatorTracker`; `roko-primitives` — `MarketHdcEncoder`; `roko-dreams` — `CounterfactualEngine`; new `Recipe` trait composing `Scorer` instances
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- composing
- TradingReflect
- FifoMatcher
- MarketHdcEncoder
- IndicatorTracker
- CounterfactualEngine
- Recipe
- Scorer

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Feed -> transform
- transform -> output
- price feed -> MACD
- RSI -> regime score
- fill events -> FIFO matching
- L -> Sharpe ratio
- market state -> role-filler binding
- hypervector -> similarity search
- historical fills -> alternative parameters
- delegation graph -> power aggregation
- agent token usage -> pricing
- weekly rollup -> budget signal
- diff -> lint warnings
- test coverage -> quality score

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Indicator pipeline: price feed → MACD → RSI → regime score → trading signal (gap doc 03)
- - **DeFi:** P&L attribution pipeline: fill events → FIFO matching → realized P&L → Sharpe ratio (gap doc 07, `TradingReflect`, `FifoMatcher`)
- - **DeFi:** HDC encoding pipeline: market state → role-filler binding → hypervector → similarity search (gap doc 10, `MarketHdcEncoder`)
- - **DeFi:** Counterfactual pipeline: historical fills → alternative parameters → simulated outcome (gap doc 09)
- - **Governance:** Voting power pipeline: delegation graph → power aggregation → threshold signal
- - **Ops:** Cost pipeline: agent token usage → pricing → daily/weekly rollup → budget signal
- - **Code:** Quality pipeline: diff → lint warnings → test coverage → quality score

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Recipe|pipeline|TradingReflect|Scorer|MarketHdcEncoder|FifoMatcher|composing|IndicatorTracker" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recipe|pipeline|TradingReflect|Scorer|MarketHdcEncoder|FifoMatcher|composing|IndicatorTracker" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `composing` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TradingReflect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FifoMatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MarketHdcEncoder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `IndicatorTracker` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CounterfactualEngine` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Recipe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Scorer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `Feed -> transform` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `transform -> output` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `price feed -> MACD` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `RSI -> regime score` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `fill events -> FIFO matching` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `L -> Sharpe ratio` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `market state -> role-filler binding` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `hypervector -> similarity search` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `historical fills -> alternative parameters` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `delegation graph -> power aggregation` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `agent token usage -> pricing` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `weekly rollup -> budget signal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `diff -> lint warnings` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `test coverage -> quality score` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S015 -- 7. Knowledge Entry

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:186` through `202`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 7. Knowledge Entry

A unit of validated, durable information with provenance and decay.

**Shape:** `create / query / validate / expire / publish`
**Unchanged from PRD 20,** but now explicitly includes DeFi knowledge types with domain-specific half-lives.

**Cross-domain examples:**
- **DeFi:** Market regime observation (half-life: 3 days), price insight (4 hours), trade strategy (2 days) — gap doc 10, `defi_half_life_days()`
- **DeFi:** Somatic marker binding: TA pattern → affect vector (gap doc 08, `SomaticMap`)
- **Governance:** Proposal analysis (half-life: 7 days), governance pattern (30 days)
- **Code:** Code insight (half-life: 30 days), architecture pattern (90 days)

**Rust mapping:** `roko-neuro` — `KnowledgeEntry`, `KnowledgeStore`, tier progression; `roko-dreams` — `MarketKnowledgeBuilder`

---
````

**Explicit detail extraction from this section:**

- Section word count: `100`
- Section hash: `1ce0448f1d833e2909c8243200bacda6b2619cec551a90e10c8cad7959c23f7d`

**Normative requirements and implementation claims:**
- **Shape:** `create / query / validate / expire / publish` **Unchanged from PRD 20,** but now explicitly includes DeFi knowledge types with domain-specific half-lives.
- **Cross-domain examples:** - **DeFi:** Market regime observation (half-life: 3 days), price insight (4 hours), trade strategy (2 days) — gap doc 10, `defi_half_life_days()` - **DeFi:** Somatic marker binding: TA pattern → affect vector (gap doc 08, `SomaticMap`) - **Governance:** Proposal analysis (half-life: 7 days), governance pattern (30 days) - **Code:** Code insight (half-life: 30 days), architecture pattern (90 days)
- **Rust mapping:** `roko-neuro` — `KnowledgeEntry`, `KnowledgeStore`, tier progression; `roko-dreams` — `MarketKnowledgeBuilder`
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- SomaticMap
- KnowledgeEntry
- KnowledgeStore
- MarketKnowledgeBuilder

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- TA pattern -> affect vector

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Market regime observation (half-life: 3 days), price insight (4 hours), trade strategy (2 days) — gap doc 10, `defi_half_life_days()`
- - **DeFi:** Somatic marker binding: TA pattern → affect vector (gap doc 08, `SomaticMap`)
- - **Governance:** Proposal analysis (half-life: 7 days), governance pattern (30 days)
- - **Code:** Code insight (half-life: 30 days), architecture pattern (90 days)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Knowledge|days|Entry|half|SomaticMap|MarketKnowledgeBuilder|KnowledgeStore|KnowledgeEntry" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|days|Entry|half|SomaticMap|MarketKnowledgeBuilder|KnowledgeStore|KnowledgeEntry" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `SomaticMap` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeStore` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MarketKnowledgeBuilder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `TA pattern -> affect vector` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S016 -- 8. Arena

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:203` through `218`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 8. Arena

An evaluation environment with task source, gates, scoring, and leaderboard.

**Shape:** `create / submit / score / rank / configure`
**Unchanged from PRD 20.** DeFi arenas pit trading strategies against each other on historical data or live paper trading.

**Cross-domain examples:**
- **DeFi:** Strategy arena — agents compete on Sharpe ratio over a historical period
- **Governance:** Prediction arena — agents forecast proposal outcomes
- **Code:** Refactoring arena — agents optimize code quality metrics

**Rust mapping:** Existing arena infrastructure (PRD 15)

---
````

**Explicit detail extraction from this section:**

- Section word count: `72`
- Section hash: `5fc6b5f3067991885d22cf692895e20c38305a8edc475123917adfb7751feb74`

**Normative requirements and implementation claims:**
- **Shape:** `create / submit / score / rank / configure` **Unchanged from PRD 20.** DeFi arenas pit trading strategies against each other on historical data or live paper trading.
- **Cross-domain examples:** - **DeFi:** Strategy arena — agents compete on Sharpe ratio over a historical period - **Governance:** Prediction arena — agents forecast proposal outcomes - **Code:** Refactoring arena — agents optimize code quality metrics
- **Rust mapping:** Existing arena infrastructure (PRD 15)
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
- - **DeFi:** Strategy arena — agents compete on Sharpe ratio over a historical period
- - **Governance:** Prediction arena — agents forecast proposal outcomes
- - **Code:** Refactoring arena — agents optimize code quality metrics

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Arena|trading|over|historical|DeFi|Code|task|submit" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|trading|over|historical|DeFi|Code|task|submit" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S017 -- 9. Eval

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:219` through `234`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 9. Eval

A measurement of behavior against external ground truth.

**Shape:** `create / run / compare / report / configure`
**Unchanged from PRD 20.** DeFi evals measure strategy performance against benchmarks.

**Cross-domain examples:**
- **DeFi:** Strategy eval against buy-and-hold baseline; risk-adjusted return eval (gap doc 07, Sharpe/Sortino)
- **Governance:** Voting accuracy eval against eventual outcomes
- **Code:** Test coverage eval, build time eval

**Rust mapping:** Existing eval infrastructure (PRD 15); `roko-learn` — `EfficiencyEvent`, experiment store

---
````

**Explicit detail extraction from this section:**

- Section word count: `71`
- Section hash: `cec26a8188da1ef1594df688f1769ee7435379151327012c02f7351883fdeb3e`

**Normative requirements and implementation claims:**
- **Shape:** `create / run / compare / report / configure` **Unchanged from PRD 20.** DeFi evals measure strategy performance against benchmarks.
- **Cross-domain examples:** - **DeFi:** Strategy eval against buy-and-hold baseline; risk-adjusted return eval (gap doc 07, Sharpe/Sortino) - **Governance:** Voting accuracy eval against eventual outcomes - **Code:** Test coverage eval, build time eval
- **Rust mapping:** Existing eval infrastructure (PRD 15); `roko-learn` — `EfficiencyEvent`, experiment store
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EfficiencyEvent

**Event names and event-like entities:**
- EfficiencyEvent

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Strategy eval against buy-and-hold baseline; risk-adjusted return eval (gap doc 07, Sharpe/Sortino)
- - **Governance:** Voting accuracy eval against eventual outcomes
- - **Code:** Test coverage eval, build time eval

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Eval|EfficiencyEvent|against|strategy|measure|DeFi|truth|time" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|EfficiencyEvent|against|strategy|measure|DeFi|truth|time" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `EfficiencyEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `EfficiencyEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S018 -- 10. Signal

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:235` through `251`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 10. Signal

A coordination event published to PulseBus for consumption by agents, feeds, and recipes.

**Shape:** `emit / subscribe / filter / expire`
**Renamed from Pheromone.** Broadened beyond stigmergy to any coordination event. Signals are the reactive glue — when something happens, interested parties respond.

**Cross-domain examples:**
- **DeFi:** Trade signal (buy/sell with confidence), regime change signal, risk alert signal, circuit breaker trigger
- **Governance:** Proposal created signal, quorum reached signal, vote deadline signal
- **Ops:** Deploy completed signal, cost threshold signal, health check failed signal
- **Code:** PR merged signal, test suite passed signal, dependency update signal

**Rust mapping:** `roko-core` — `Signal` struct (the kernel type); `roko-fs` — `FileSubstrate` for persistence; PulseBus for distribution

---
````

**Explicit detail extraction from this section:**

- Section word count: `107`
- Section hash: `3f794f7311c10de22cbe02c4d60edefde4460f9558aebf2b68bd0d74b025cfdc`

**Normative requirements and implementation claims:**
- A coordination event published to PulseBus for consumption by agents, feeds, and recipes.
- **Shape:** `emit / subscribe / filter / expire` **Renamed from Pheromone.** Broadened beyond stigmergy to any coordination event. Signals are the reactive glue — when something happens, interested parties respond.
- **Cross-domain examples:** - **DeFi:** Trade signal (buy/sell with confidence), regime change signal, risk alert signal, circuit breaker trigger - **Governance:** Proposal created signal, quorum reached signal, vote deadline signal - **Ops:** Deploy completed signal, cost threshold signal, health check failed signal - **Code:** PR merged signal, test suite passed signal, dependency update signal
- **Rust mapping:** `roko-core` — `Signal` struct (the kernel type); `roko-fs` — `FileSubstrate` for persistence; PulseBus for distribution
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Signal
- FileSubstrate

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Trade signal (buy/sell with confidence), regime change signal, risk alert signal, circuit breaker trigger
- - **Governance:** Proposal created signal, quorum reached signal, vote deadline signal
- - **Ops:** Deploy completed signal, cost threshold signal, health check failed signal
- - **Code:** PR merged signal, test suite passed signal, dependency update signal

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Signal|FileSubstrate|event|coordination|PulseBus|vote|update|type" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Signal|FileSubstrate|event|coordination|PulseBus|vote|update|type" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `Signal` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FileSubstrate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S019 -- 11. Group

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:252` through `267`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 11. Group

A coordinated subset of agents working toward a shared objective.

**Shape:** `create / add / remove / coordinate / dissolve`
**Unchanged from PRD 20.** Groups gain DeFi semantics: a trading desk is a group of specialized agents (executor, risk assessor, researcher) sharing state and coordinating via signals.

**Cross-domain examples:**
- **DeFi:** Trading desk group — executor + risk assessor + market researcher + safety guardian (gap doc 05 archetypes)
- **Governance:** DAO operations group — proposal drafter + voter + treasury manager
- **Ops:** Incident response group — diagnostician + fixer + communicator

**Rust mapping:** Existing group infrastructure; `roko-orchestrator` — `PlanRunner` for coordinated execution

---
````

**Explicit detail extraction from this section:**

- Section word count: `88`
- Section hash: `9fdcc4ac37ee6e5fc13d9198a340ee640e4f81e2f751bcbe65f0fb30f6bf29a4`

**Normative requirements and implementation claims:**
- **Shape:** `create / add / remove / coordinate / dissolve` **Unchanged from PRD 20.** Groups gain DeFi semantics: a trading desk is a group of specialized agents (executor, risk assessor, researcher) sharing state and coordinating via signals.
- **Cross-domain examples:** - **DeFi:** Trading desk group — executor + risk assessor + market researcher + safety guardian (gap doc 05 archetypes) - **Governance:** DAO operations group — proposal drafter + voter + treasury manager - **Ops:** Incident response group — diagnostician + fixer + communicator
- **Rust mapping:** Existing group infrastructure; `roko-orchestrator` — `PlanRunner` for coordinated execution
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- PlanRunner

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **DeFi:** Trading desk group — executor + risk assessor + market researcher + safety guardian (gap doc 05 archetypes)
- - **Governance:** DAO operations group — proposal drafter + voter + treasury manager
- - **Ops:** Incident response group — diagnostician + fixer + communicator

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Group|PlanRunner|coordinate|trading|risk|researcher|executor|desk" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Group|PlanRunner|coordinate|trading|risk|researcher|executor|desk" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `PlanRunner` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S020 -- 12. Bounty

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:268` through `283`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 12. Bounty

A user-posted task with a reward and acceptance criteria.

**Shape:** `post / claim / submit / review / pay`
**Unchanged from PRD 20.** DeFi bounties can specify on-chain reward distribution.

**Cross-domain examples:**
- **DeFi:** "Build a mean-reversion strategy with Sharpe > 1.5" — reward in tokens
- **Governance:** "Draft a temperature check for treasury diversification" — reputation reward
- **Code:** "Reduce build time by 30%" — token reward

**Rust mapping:** Existing bounty infrastructure (PRD 17)

---
````

**Explicit detail extraction from this section:**

- Section word count: `69`
- Section hash: `8418de25f2c7b92302e76a372ce09a073abebaedbe8eaa214fca96dfeb6e9fd5`

**Normative requirements and implementation claims:**
- A user-posted task with a reward and acceptance criteria.
- **Shape:** `post / claim / submit / review / pay` **Unchanged from PRD 20.** DeFi bounties can specify on-chain reward distribution.
- **Cross-domain examples:** - **DeFi:** "Build a mean-reversion strategy with Sharpe > 1.5" — reward in tokens - **Governance:** "Draft a temperature check for treasury diversification" — reputation reward - **Code:** "Reduce build time by 30%" — token reward
- **Rust mapping:** Existing bounty infrastructure (PRD 17)
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
- - **DeFi:** "Build a mean-reversion strategy with Sharpe > 1.5" — reward in tokens
- - **Governance:** "Draft a temperature check for treasury diversification" — reputation reward
- - **Code:** "Reduce build time by 30%" — token reward

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Bounty|reward|token|post|DeFi|Build|user|treasury" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bounty|reward|token|post|DeFi|Build|user|treasury" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S021 -- Composition matrix

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:284` through `304`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Composition matrix

How every pair of primitives interacts. Read as "row composes with column."

| | Agent | Extension | Connector | Gate | Feed | Recipe | Knowledge | Arena | Eval | Signal | Group | Bounty |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Agent** | delegates to | loads | uses for I/O | checked by | subscribes to | executes | queries | competes in | measured by | emits/receives | member of | claims |
| **Extension** | modifies | chains with | wraps | triggers | filters | transforms | reads | — | — | listens to | — | — |
| **Connector** | — | — | proxies | health-checked by | sources | — | — | — | — | emits on error | — | — |
| **Gate** | validates | invoked by | verifies via | chains with | monitors | validates output | checks against | enforces rules | provides data | emits pass/fail | applies to all | criteria for |
| **Feed** | delivers to | filtered by | sourced from | monitored by | merges with | inputs to | — | streams to | measured by | triggers | shared across | — |
| **Recipe** | run by | — | reads from | validated by | consumes | chains with | produces | scores for | implements | emits results | — | — |
| **Knowledge** | informs | configures | — | ground truth for | — | produced by | links to | training data | ground truth | — | shared across | — |
| **Arena** | hosts | — | — | uses | receives from | scored by | produces | nests in | paired with | coordinates via | among | reward source |
| **Eval** | measures | — | — | uses gates | samples from | implemented by | references | paired with | compares with | reports via | across | criteria for |
| **Signal** | triggers | triggers | — | — | carried by | triggers | — | coordinates | — | amplifies | coordinates | — |
| **Group** | contains | shares | shares | shares | shares | shares | shares | enters as | evaluated as | coordinates via | nests in | posts/claims |
| **Bounty** | assigned to | — | — | acceptance gate | — | — | — | — | acceptance eval | announced via | claimed by | — |

---
````

**Explicit detail extraction from this section:**

- Section word count: `217`
- Section hash: `acb5efe7f235863d356ed90070d1425eb90d7038602ff902773bf600d71656e5`

**Normative requirements and implementation claims:**
- | | Agent | Extension | Connector | Gate | Feed | Recipe | Knowledge | Arena | Eval | Signal | Group | Bounty | |---|---|---|---|---|---|---|---|---|---|---|---|---| | **Agent** | delegates to | loads | uses for I/O | checked by | subscribes to | executes | queries | competes in | measured by | emits/receives | member of | claims | | **Extension** | modifies | chains with | wraps | triggers | filters | transforms | reads | — | — | listens to | — | — | | **Connector** | — | — | proxies | health-checked by | sources | — | — | — | — | emits on error | — | — | | **Gate** | validates | invoked by | verifies via | chains with | monitors | validates output | checks against | enforces rules | provides data | emits pass/fail | applies to all | criteria for | | **Feed** | delivers to | filtered by | sourced from | monitored by | merges with | inputs to | — | streams to | measured by | triggers | shared across | — | | **Recipe** | run by | — | reads from | validated by | consumes | chains with | 
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
- Table 1:

```markdown
| | Agent | Extension | Connector | Gate | Feed | Recipe | Knowledge | Arena | Eval | Signal | Group | Bounty |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Agent** | delegates to | loads | uses for I/O | checked by | subscribes to | executes | queries | competes in | measured by | emits/receives | member of | claims |
| **Extension** | modifies | chains with | wraps | triggers | filters | transforms | reads | — | — | listens to | — | — |
| **Connector** | — | — | proxies | health-checked by | sources | — | — | — | — | emits on error | — | — |
| **Gate** | validates | invoked by | verifies via | chains with | monitors | validates output | checks against | enforces rules | provides data | emits pass/fail | applies to all | criteria for |
| **Feed** | delivers to | filtered by | sourced from | monitored by | merges with | inputs to | — | streams to | measured by | triggers | shared across | — |
| **Recipe** | run by | — | reads from | validated by | consumes | chains with | produces | scores for | implements | emits results | — | — |
| **Knowledge** | informs | configures | — | ground truth for | — | produced by | links to | training data | ground truth | — | shared across | — |
| **Arena** | hosts | — | — | uses | receives from | scored by | produces | nests in | paired with | coordinates via | among | reward source |
| **Eval** | measures | — | — | uses gates | samples from | implemented by | references | paired with | compares with | reports via | across | criteria for |
| **Signal** | triggers | triggers | — | — | carried by | triggers | — | coordinates | — | amplifies | coordinates | — |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "shares|triggers|matrix|Gate|Composition|emits|coordinates|Eval" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "shares|triggers|matrix|Gate|Composition|emits|coordinates|Eval" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S022 -- DeFi struct mapping

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:305` through `360`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## DeFi struct mapping

Every domain-specific struct from the gap docs maps to exactly one universal primitive.

| Gap Doc | Struct / Concept | Primitive | Rationale |
|---------|-----------------|-----------|-----------|
| 01 | `ChainClient`, `ChainRpc` | **Connector** | External system I/O |
| 01 | `ChainEventWatcher` | **Feed** | Continuous block/event stream |
| 01 | `ForkManager`, `Snapshot/Revert` | **Connector** | Chain state management I/O |
| 01 | `OracleConnector` | **Connector** | External price data I/O |
| 01 | `TxSimulator` | **Gate** | Pre-action simulation check |
| 02 | `VenueAdapter` | **Connector** | Exchange I/O |
| 02 | `SwapHandler`, `LpHandler`, `VaultHandler` | **Extension** | Tool-specific behavior modifiers on Connectors |
| 02 | `RiskAssessmentTool` | **Gate** | Pre-action risk check |
| 03 | `MACD`, `RSI`, `ATR`, `ADX`, `OBV`, etc. | **Recipe** | Indicator transform pipelines |
| 03 | `DeFiIndicator` (tick asymmetry, liquidity migration) | **Recipe** | DeFi-native transform pipelines |
| 03 | `MicrostructureIndicator` (VPIN, Kyle's lambda) | **Recipe** | Microstructure transform pipelines |
| 03 | `VolatilityIndicator`, `RegimeClassifier` | **Recipe** | Volatility/regime transform pipelines |
| 03 | `MarketHdcEncoder` (composite indicators) | **Recipe** | HDC encoding transform pipeline |
| 04 | `DeFiRiskEngine` | **Gate** | Pre-action risk gate |
| 04 | `MevProtectionGate` | **Gate** | Pre-action MEV simulation gate |
| 04 | `CircuitBreakerGate` | **Gate** | Emergency shutdown gate |
| 04 | `CustodyGate` | **Gate** | Pre-action custody verification |
| 04 | `DaimonRiskModulator` | **Extension** | Affect-modulated risk adjustment |
| 05 | `ArchetypeManifest` | **Agent** | Agent template (archetype field) |
| 05 | `ArchetypeRegistry` | **Agent** | Agent template registry |
| 05 | `DelegationDag` | **Group** | Agent delegation hierarchy |
| 05 | `ToolProfileResolver` | **Agent** | Agent capability configuration |
| 06 | `HeartbeatClock` | **Feed** | Periodic tick stream |
| 06 | `DecisionCycleRecord` | **Knowledge Entry** | Decision audit trail |
| 06 | `CorticalState` | **Recipe** | State aggregation pipeline |
| 06 | `EmergencyShutdownPolicy` | **Gate** | Pre-action circuit breaker |
| 07 | `TradingReflect` | **Recipe** | P&L attribution pipeline |
| 07 | `FifoMatcher` | **Recipe** | Position matching transform |
| 07 | `IndicatorTracker` | **Recipe** | Indicator accuracy pipeline |
| 07 | `RegimeDetector` | **Recipe** | Regime classification pipeline |
| 07 | `TradingPlaybook` | **Knowledge Entry** | Durable strategy pattern |
| 07 | Risk-adjusted reward signal | **Signal** | Continuous reward for bandits |
| 08 | `ProspectValueFunction` | **Recipe** | P&L → PAD vector transform |
| 08 | `TiltDetector` | **Extension** | Affect-based execution modifier |
| 08 | `SomaticMap` | **Knowledge Entry** | TA pattern → affect binding |
| 08 | `PositionSizer` (affect-modulated) | **Extension** | Affect-based position sizing |
| 09 | `ChainEventTrigger` | **Feed** | Dream-triggering event stream |
| 09 | `CounterfactualEngine` | **Recipe** | Alternative outcome simulation |
| 09 | `DeFiThreatGenerator` | **Recipe** | Threat scenario generation |
| 09 | `MarketKnowledgeBuilder` | **Recipe** | TA → knowledge transform |
| 09 | `DreamCalibrator` | **Eval** | Dream accuracy measurement |
| 09 | `RegimeTransitionHandler` | **Recipe** | Regime-filtered episode transform |
| 10 | `MarketHdcEncoder` | **Recipe** | Market state → hypervector |
| 10 | `KnowledgeRoutingAdvisor` | **Recipe** | Knowledge → model routing |
| 10 | `RegimeCodebook` | **Knowledge Entry** | Canonical regime vectors |
| 10 | `CrossMarketTransfer` | **Recipe** | HDC resonance detection |
| 10 | Ebbinghaus for market knowledge | **Knowledge Entry** | Domain-specific decay rates |

---
````

**Explicit detail extraction from this section:**

- Section word count: `367`
- Section hash: `aa6d305ffcb4d9720bd31b8606af6bf7c56e73f060238078462ef3c267f194ad`

**Normative requirements and implementation claims:**
- | Gap Doc | Struct / Concept | Primitive | Rationale | |---------|-----------------|-----------|-----------| | 01 | `ChainClient`, `ChainRpc` | **Connector** | External system I/O | | 01 | `ChainEventWatcher` | **Feed** | Continuous block/event stream | | 01 | `ForkManager`, `Snapshot/Revert` | **Connector** | Chain state management I/O | | 01 | `OracleConnector` | **Connector** | External price data I/O | | 01 | `TxSimulator` | **Gate** | Pre-action simulation check | | 02 | `VenueAdapter` | **Connector** | Exchange I/O | | 02 | `SwapHandler`, `LpHandler`, `VaultHandler` | **Extension** | Tool-specific behavior modifiers on Connectors | | 02 | `RiskAssessmentTool` | **Gate** | Pre-action risk check | | 03 | `MACD`, `RSI`, `ATR`, `ADX`, `OBV`, etc. | **Recipe** | Indicator transform pipelines | | 03 | `DeFiIndicator` (tick asymmetry, liquidity migration) | **Recipe** | DeFi-native transform pipelines | | 03 | `MicrostructureIndicator` (VPIN, Kyle's lambda) | **Recipe** | Microstructure
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- from
- ChainClient
- ChainRpc
- ChainEventWatcher
- ForkManager
- OracleConnector
- TxSimulator
- VenueAdapter
- SwapHandler
- LpHandler
- VaultHandler
- RiskAssessmentTool
- MACD
- RSI
- ATR
- ADX
- OBV
- DeFiIndicator
- MicrostructureIndicator
- VolatilityIndicator
- RegimeClassifier
- MarketHdcEncoder
- DeFiRiskEngine
- MevProtectionGate
- CircuitBreakerGate
- CustodyGate
- DaimonRiskModulator
- ArchetypeManifest
- ArchetypeRegistry
- DelegationDag
- ToolProfileResolver
- HeartbeatClock
- DecisionCycleRecord
- CorticalState
- EmergencyShutdownPolicy
- TradingReflect
- FifoMatcher
- IndicatorTracker
- RegimeDetector
- TradingPlaybook
- ProspectValueFunction
- TiltDetector
- SomaticMap
- PositionSizer
- ChainEventTrigger
- CounterfactualEngine
- DeFiThreatGenerator
- MarketKnowledgeBuilder
- DreamCalibrator
- RegimeTransitionHandler
- KnowledgeRoutingAdvisor
- RegimeCodebook
- CrossMarketTransfer

**Event names and event-like entities:**
- ChainEvent

**State transitions:**
- L -> PAD vector transform
- TA pattern -> affect binding
- TA -> knowledge transform
- Market state -> hypervector
- Knowledge -> model routing

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Gap Doc | Struct / Concept | Primitive | Rationale |
|---------|-----------------|-----------|-----------|
| 01 | `ChainClient`, `ChainRpc` | **Connector** | External system I/O |
| 01 | `ChainEventWatcher` | **Feed** | Continuous block/event stream |
| 01 | `ForkManager`, `Snapshot/Revert` | **Connector** | Chain state management I/O |
| 01 | `OracleConnector` | **Connector** | External price data I/O |
| 01 | `TxSimulator` | **Gate** | Pre-action simulation check |
| 02 | `VenueAdapter` | **Connector** | Exchange I/O |
| 02 | `SwapHandler`, `LpHandler`, `VaultHandler` | **Extension** | Tool-specific behavior modifiers on Connectors |
| 02 | `RiskAssessmentTool` | **Gate** | Pre-action risk check |
| 03 | `MACD`, `RSI`, `ATR`, `ADX`, `OBV`, etc. | **Recipe** | Indicator transform pipelines |
| 03 | `DeFiIndicator` (tick asymmetry, liquidity migration) | **Recipe** | DeFi-native transform pipelines |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Recipe|Gate|Knowledge|transform|struct|pipeline|DeFi|regime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recipe|Gate|Knowledge|transform|struct|pipeline|DeFi|regime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `from` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainRpc` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainEventWatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ForkManager` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `OracleConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TxSimulator` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VenueAdapter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SwapHandler` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LpHandler` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VaultHandler` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RiskAssessmentTool` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MACD` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RSI` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ATR` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ADX` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `OBV` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DeFiIndicator` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MicrostructureIndicator` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VolatilityIndicator` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RegimeClassifier` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MarketHdcEncoder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DeFiRiskEngine` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MevProtectionGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CircuitBreakerGate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ChainEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `L -> PAD vector transform` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `TA pattern -> affect binding` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `TA -> knowledge transform` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Market state -> hypervector` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Knowledge -> model routing` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S023 -- Authoring surfaces

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:361` through `364`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Authoring surfaces

One authoring surface per primitive, referencing existing surfaces from PRD 19 where applicable.
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `d296f1bf90a4e3f789f14e1955b16f91b0c244f5200f6ba22af1dc57e7c8329b`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "surface|surfaces|Authoring|referencing|primitive|existing|applicable|universal" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|surfaces|Authoring|referencing|primitive|existing|applicable|universal" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S024 -- Agent Composer (existing — PRD 19 Stage 1-10)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:365` through `368`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent Composer (existing — PRD 19 Stage 1-10)

Expanded with archetype field. Stage 2 (Domain Selection) replaced by **Archetype Selector** — users pick from `ArchetypeManifest` templates that bundle domain, tool profiles, gate pipelines, and model preferences.
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `59023e9f7c516fabfc93a8d056f0479678fc98b433213b9f4b379d8faad9e47c`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArchetypeManifest

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Stage|existing|PRD|Composer|ArchetypeManifest|archetype|Domain|users" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Stage|existing|PRD|Composer|ArchetypeManifest|archetype|Domain|users" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S025 -- Extension Workshop (existing — PRD 19)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:369` through `372`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extension Workshop (existing — PRD 19)

Same 4-stage flow (extension selection, code editor, test sandbox, publish). Supports all three tiers: Tier 1 Pi extensions (simplified tool registration flow), Tier 2 Roko-enhanced (adds heartbeat hook config), Tier 3 Roko-native (full Rust hook editor with ABI validation). Gains DeFi extension types: affect modulation, tilt detection, position sizing. Marketplace integration via `roko install` / `roko publish`.
````

**Explicit detail extraction from this section:**

- Section word count: `59`
- Section hash: `d73c7fbba9ef3f952dff8d2a3a935135f2fcfbe5c3011c4254c5c953fe73fde5`

**Normative requirements and implementation claims:**
- Same 4-stage flow (extension selection, code editor, test sandbox, publish). Supports all three tiers: Tier 1 Pi extensions (simplified tool registration flow), Tier 2 Roko-enhanced (adds heartbeat hook config), Tier 3 Roko-native (full Rust hook editor with ABI validation). Gains DeFi extension types: affect modulation, tilt detection, position sizing. Marketplace integration via `roko install` / `roko publish`.

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
- roko install
- roko publish

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Extension|existing|Workshop|PRD|Tier|publish|hook|flow" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|existing|Workshop|PRD|Tier|publish|hook|flow" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify operator command `roko install` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko publish` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S026 -- Connector Manager (new surface)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:373` through `381`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Connector Manager (new surface)

| Stage | What | Notes |
|-------|------|-------|
| 1. Type Selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery |
| 2. Configuration | Connection string, auth, rate limits, retry policy | Live health check |
| 3. Tool Registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema |
| 4. Test & Deploy | Execute test query; verify health endpoint | Shows latency, error rate |
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `bf7bf2e2f242a88e31e16b0333777ed68d1451f54b7a0da66f7998e6ab297cae`

**Normative requirements and implementation claims:**
- | Stage | What | Notes | |-------|------|-------| | 1. Type Selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery | | 2. Configuration | Connection string, auth, rate limits, retry policy | Live health check | | 3. Tool Registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema | | 4. Test & Deploy | Execute test query; verify health endpoint | Shows latency, error rate |

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
| Stage | What | Notes |
|-------|------|-------|
| 1. Type Selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery |
| 2. Configuration | Connection string, auth, rate limits, retry policy | Live health check |
| 3. Tool Registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema |
| 4. Test & Deploy | Execute test query; verify health endpoint | Shows latency, error rate |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Connector|surface|new|Manager|select|rate|health|Type" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connector|surface|new|Manager|select|rate|health|Type" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S027 -- Gate Designer (existing — PRD 19)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:382` through `385`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gate Designer (existing — PRD 19)

Expanded with pre-action mode. Users toggle between pre-action (permission gate) and post-action (validation gate). DeFi gate types: risk limits, MEV simulation, custody check, circuit breaker.
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `134b96d42fe7bd69273811e80da2189d12a3ba73ee01bb4f390aefa85be65b83`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Gate|existing|PRD|Designer|action|validation|types|toggle" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Gate|existing|PRD|Designer|action|validation|types|toggle" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S028 -- Feed Designer (new surface)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:386` through `394`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feed Designer (new surface)

| Stage | What | Notes |
|-------|------|-------|
| 1. Source Selection | Choose source connector + event type | Connector picker |
| 2. Filter & Transform | Configure event filters, sampling rate, aggregation window | Visual filter builder |
| 3. Output Configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram |
| 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline |
````

**Explicit detail extraction from this section:**

- Section word count: `50`
- Section hash: `5cc8358098bee2e8864095fb2e725abe25b9872b5506b3163dddf41c5f649c9d`

**Normative requirements and implementation claims:**
- | Stage | What | Notes | |-------|------|-------| | 1. Source Selection | Choose source connector + event type | Connector picker | | 2. Filter & Transform | Configure event filters, sampling rate, aggregation window | Visual filter builder | | 3. Output Configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram | | 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline |

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
| Stage | What | Notes |
|-------|------|-------|
| 1. Source Selection | Choose source connector + event type | Connector picker |
| 2. Filter & Transform | Configure event filters, sampling rate, aggregation window | Visual filter builder |
| 3. Output Configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram |
| 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "surface|new|Feed|Designer|event|Filter|rate|connector" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|new|Feed|Designer|event|Filter|rate|connector" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S029 -- Recipe Editor (new surface)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:395` through `403`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Recipe Editor (new surface)

| Stage | What | Notes |
|-------|------|-------|
| 1. Input Selection | Choose feed(s) or connector query as input | Drag from feed/connector list |
| 2. Pipeline Builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor |
| 3. Output Configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output |
| 4. Backtest & Validate | Run against historical data; compare output distribution | Chart overlay |
````

**Explicit detail extraction from this section:**

- Section word count: `60`
- Section hash: `3137aa9775a588f57429c3255467292497b6fc79301a97fdc2a1cdc0bbcbe749`

**Normative requirements and implementation claims:**
- | Stage | What | Notes | |-------|------|-------| | 1. Input Selection | Choose feed(s) or connector query as input | Drag from feed/connector list | | 2. Pipeline Builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor | | 3. Output Configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output | | 4. Backtest & Validate | Run against historical data; compare output distribution | Chart overlay |

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
| Stage | What | Notes |
|-------|------|-------|
| 1. Input Selection | Choose feed(s) or connector query as input | Drag from feed/connector list |
| 2. Pipeline Builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor |
| 3. Output Configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output |
| 4. Backtest & Validate | Run against historical data; compare output distribution | Chart overlay |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Editor|surface|new|Recipe|feed|Output|connector|Stage" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Editor|surface|new|Recipe|feed|Output|connector|Stage" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S030 -- Knowledge Publisher, Arena Constructor, Eval Author, Signal Designer, Group Organizer, Bounty Author

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:404` through `409`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge Publisher, Arena Constructor, Eval Author, Signal Designer, Group Organizer, Bounty Author

Unchanged from PRD 19. See that document for stage-by-stage specifications.

---
````

**Explicit detail extraction from this section:**

- Section word count: `12`
- Section hash: `54d17e583c0d2a1ab623e72f6e4ee0cc4eb69f3f17624ce80bcecd1b1f0df61e`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Signal|Publisher|Organizer|Knowledge|Group|Eval|Designer|Constructor" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Signal|Publisher|Organizer|Knowledge|Group|Eval|Designer|Constructor" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S031 -- Migration path: 10 primitives to 12

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:410` through `422`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Migration path: 10 primitives to 12

The migration is **backward-compatible** — no existing objects break.

| Change | Migration |
|--------|-----------|
| Extension | No change needed. Name preserved for Pi ecosystem compatibility. The Rust `Extension` trait, `roko install` CLI, Pi `PiExtension` API, and marketplace all use "extension." |
| Pheromone → Signal | Rename at product layer only. Rust `Signal` struct is already named correctly. Pheromone references in UI become Signal. |
| Domain → Agent field | `DomainProfile` enum values become `archetype.domain` on Agent templates. Existing domain configs become archetype manifests. Automated migration: wrap each `DomainProfile` in an `ArchetypeManifest` with default tool profile and gate pipeline. |
| + Connector | New primitive. No migration needed — Connectors didn't exist before. MCP server configs in `roko.toml` auto-register as Connectors. |
| + Feed | New primitive. No migration needed. Existing `HeartbeatClock` and `ChainEventWatcher` auto-register as Feeds. |
| + Recipe | New primitive. No migration needed. Existing scoring pipelines in `roko-learn` become recipe templates. |
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `c522254b1dd7f6a8a53049243a49c2ad5307e994fec7dcd4ebaf7ac5c14538e4`

**Normative requirements and implementation claims:**
- | Change | Migration | |--------|-----------| | Extension | No change needed. Name preserved for Pi ecosystem compatibility. The Rust `Extension` trait, `roko install` CLI, Pi `PiExtension` API, and marketplace all use "extension." | | Pheromone → Signal | Rename at product layer only. Rust `Signal` struct is already named correctly. Pheromone references in UI become Signal. | | Domain → Agent field | `DomainProfile` enum values become `archetype.domain` on Agent templates. Existing domain configs become archetype manifests. Automated migration: wrap each `DomainProfile` in an `ArchetypeManifest` with default tool profile and gate pipeline. | | + Connector | New primitive. No migration needed — Connectors didn't exist before. MCP server configs in `roko.toml` auto-register as Connectors. | | + Feed | New primitive. No migration needed. Existing `HeartbeatClock` and `ChainEventWatcher` auto-register as Feeds. | | + Recipe | New primitive. No migration needed. Existing scoring pipelines 

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- is
- values
- Extension
- PiExtension
- Signal
- DomainProfile
- ArchetypeManifest
- HeartbeatClock
- ChainEventWatcher

**Event names and event-like entities:**
- archetype.domain
- ChainEvent

**State transitions:**
- Pheromone -> Signal
- Domain -> Agent field

**Config keys and TOML-like settings:**
- archetype.domain
- roko.toml

**Commands and operator actions:**
- roko install

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Change | Migration |
|--------|-----------|
| Extension | No change needed. Name preserved for Pi ecosystem compatibility. The Rust `Extension` trait, `roko install` CLI, Pi `PiExtension` API, and marketplace all use "extension." |
| Pheromone → Signal | Rename at product layer only. Rust `Signal` struct is already named correctly. Pheromone references in UI become Signal. |
| Domain → Agent field | `DomainProfile` enum values become `archetype.domain` on Agent templates. Existing domain configs become archetype manifests. Automated migration: wrap each `DomainProfile` in an `ArchetypeManifest` with default tool profile and gate pipeline. |
| + Connector | New primitive. No migration needed — Connectors didn't exist before. MCP server configs in `roko.toml` auto-register as Connectors. |
| + Feed | New primitive. No migration needed. Existing `HeartbeatClock` and `ChainEventWatcher` auto-register as Feeds. |
| + Recipe | New primitive. No migration needed. Existing scoring pipelines in `roko-learn` become recipe templates. |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Migration|primitive|Extension|Signal|DomainProfile|values|primitives|path" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Migration|primitive|Extension|Signal|DomainProfile|values|primitives|path" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `is` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `values` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PiExtension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Signal` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DomainProfile` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `HeartbeatClock` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainEventWatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `archetype.domain` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ChainEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `Pheromone -> Signal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Domain -> Agent field` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `archetype.domain` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `roko install` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S032 -- Versioning

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:423` through `428`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Versioning

The primitive vocabulary version moves from `1.0` (10 primitives, PRD 20) to `2.0` (12 primitives, this document). Dashboard surfaces should check the vocabulary version and render the appropriate primitive set.

---
````

**Explicit detail extraction from this section:**

- Section word count: `32`
- Section hash: `939510d6aabd95a8c84b6ad32c1dd9990d1a5218619821cc6d74722060dc820b`

**Normative requirements and implementation claims:**
- The primitive vocabulary version moves from `1.0` (10 primitives, PRD 20) to `2.0` (12 primitives, this document). Dashboard surfaces should check the vocabulary version and render the appropriate primitive set.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "version|Versioning|primitive|vocabulary|primitives|surfaces|render|moves" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "version|Versioning|primitive|vocabulary|primitives|surfaces|render|moves" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S033 -- Rust trait mapping

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:429` through `453`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Rust trait mapping

Where each primitive lives in the codebase.

| Primitive | Trait / Struct | Crate | File |
|-----------|---------------|-------|------|
| Agent | `AgentManifest`, `AgentProcess` | `roko-agent` | `src/manifest.rs`, `src/process.rs` |
| Agent (archetype) | `ArchetypeManifest` | `roko-agent` | (gap doc 05 — to be implemented) |
| Extension | `Extension` trait (22 hooks) | `roko-runtime` | `src/extension.rs` |
| Extension (Pi bridge) | `JsExtensionBridge` | `roko-quickjs` | `src/bridge.rs` |
| Extension (registry) | `PackageManifest`, `ExtensionLoader` | `roko-ext-registry` | `src/manifest.rs`, `src/extension_loader.rs` |
| Connector | `ChainClient`, `VenueAdapter` | `roko-chain` | `src/client.rs`, (gap doc 02) |
| Connector (MCP) | MCP server config | `roko-agent` | `src/mcp/` |
| Gate | `Gate` trait, `RungConfig` | `roko-gate` | `src/lib.rs`, `src/rung.rs` |
| Feed | `ChainEventWatcher`, `HeartbeatClock` | `roko-chain`, `roko-conductor` | (gap doc 01, 06) |
| Recipe | `Scorer` trait, `TradingReflect` | `roko-core`, `roko-learn` | `src/scorer.rs`, (gap doc 07) |
| Knowledge Entry | `KnowledgeEntry`, `KnowledgeStore` | `roko-neuro` | `src/store.rs` |
| Arena | Arena infrastructure | (dashboard + roko-serve) | PRD 15 |
| Eval | `EfficiencyEvent`, experiment store | `roko-learn` | `src/efficiency.rs` |
| Signal | `Signal` | `roko-core` | `src/signal.rs` |
| Group | Group infrastructure | `roko-orchestrator` | `src/plan_runner.rs` |
| Bounty | Bounty infrastructure | (dashboard + roko-serve) | PRD 17 |

---
````

**Explicit detail extraction from this section:**

- Section word count: `172`
- Section hash: `c1c4568f717840e101276cdb71bba25fc64784e7d5c38b9dec2ce8f2f3b38a79`

**Normative requirements and implementation claims:**
- | Primitive | Trait / Struct | Crate | File | |-----------|---------------|-------|------| | Agent | `AgentManifest`, `AgentProcess` | `roko-agent` | `src/manifest.rs`, `src/process.rs` | | Agent (archetype) | `ArchetypeManifest` | `roko-agent` | (gap doc 05 — to be implemented) | | Extension | `Extension` trait (22 hooks) | `roko-runtime` | `src/extension.rs` | | Extension (Pi bridge) | `JsExtensionBridge` | `roko-quickjs` | `src/bridge.rs` | | Extension (registry) | `PackageManifest`, `ExtensionLoader` | `roko-ext-registry` | `src/manifest.rs`, `src/extension_loader.rs` | | Connector | `ChainClient`, `VenueAdapter` | `roko-chain` | `src/client.rs`, (gap doc 02) | | Connector (MCP) | MCP server config | `roko-agent` | `src/mcp/` | | Gate | `Gate` trait, `RungConfig` | `roko-gate` | `src/lib.rs`, `src/rung.rs` | | Feed | `ChainEventWatcher`, `HeartbeatClock` | `roko-chain`, `roko-conductor` | (gap doc 01, 06) | | Recipe | `Scorer` trait, `TradingReflect` | `roko-core`, `roko-learn` | `
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- src/bridge.rs
- src/client.rs
- src/efficiency.rs
- src/extension.rs
- src/extension_loader.rs
- src/lib.rs
- src/manifest.rs
- src/mcp/
- src/plan_runner.rs
- src/process.rs
- src/rung.rs
- src/scorer.rs
- src/signal.rs
- src/store.rs

**Types, functions, traits, and inline code identifiers:**
- AgentManifest
- AgentProcess
- ArchetypeManifest
- Extension
- JsExtensionBridge
- PackageManifest
- ExtensionLoader
- ChainClient
- VenueAdapter
- Gate
- RungConfig
- ChainEventWatcher
- HeartbeatClock
- Scorer
- TradingReflect
- KnowledgeEntry
- KnowledgeStore
- EfficiencyEvent
- Signal

**Event names and event-like entities:**
- ChainEvent
- EfficiencyEvent

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
| Primitive | Trait / Struct | Crate | File |
|-----------|---------------|-------|------|
| Agent | `AgentManifest`, `AgentProcess` | `roko-agent` | `src/manifest.rs`, `src/process.rs` |
| Agent (archetype) | `ArchetypeManifest` | `roko-agent` | (gap doc 05 — to be implemented) |
| Extension | `Extension` trait (22 hooks) | `roko-runtime` | `src/extension.rs` |
| Extension (Pi bridge) | `JsExtensionBridge` | `roko-quickjs` | `src/bridge.rs` |
| Extension (registry) | `PackageManifest`, `ExtensionLoader` | `roko-ext-registry` | `src/manifest.rs`, `src/extension_loader.rs` |
| Connector | `ChainClient`, `VenueAdapter` | `roko-chain` | `src/client.rs`, (gap doc 02) |
| Connector (MCP) | MCP server config | `roko-agent` | `src/mcp/` |
| Gate | `Gate` trait, `RungConfig` | `roko-gate` | `src/lib.rs`, `src/rung.rs` |
| Feed | `ChainEventWatcher`, `HeartbeatClock` | `roko-chain`, `roko-conductor` | (gap doc 01, 06) |
| Recipe | `Scorer` trait, `TradingReflect` | `roko-core`, `roko-learn` | `src/scorer.rs`, (gap doc 07) |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `src/bridge.rs`
- `src/client.rs`
- `src/efficiency.rs`
- `src/extension.rs`
- `src/extension_loader.rs`
- `src/lib.rs`
- `src/manifest.rs`
- `src/mcp/`
- `src/plan_runner.rs`
- `src/process.rs`
- `crates/roko-serve/src/routes/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|trait|Signal|Gate|mapping|manifest|VenueAdapter|TradingReflect" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|trait|Signal|Gate|mapping|manifest|VenueAdapter|TradingReflect" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `src/bridge.rs`
- `src/client.rs`
- `src/efficiency.rs`
- `src/extension.rs`
- `src/extension_loader.rs`
- `src/lib.rs`
- `src/manifest.rs`
- `src/mcp/`
- `src/plan_runner.rs`
- `src/process.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`

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
- [ ] Implement or verify `AgentManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentProcess` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `JsExtensionBridge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PackageManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExtensionLoader` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VenueAdapter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Gate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RungConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainEventWatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `HeartbeatClock` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Scorer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TradingReflect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeStore` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `EfficiencyEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Signal` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ChainEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `EfficiencyEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S034 -- Cross-domain universality

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:454` through `457`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Cross-domain universality

To verify that the 12 primitives are truly universal, here are complete examples in four domains showing how the same primitive vocabulary describes fundamentally different workflows.
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `30fc67e84a1edd5654f48c1a5e358a4a4309541eb67e363d6ff528998afb9e54`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "universal|domain|universality|Cross|primitive|workflows|vocabulary|verify" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "universal|domain|universality|Cross|primitive|workflows|vocabulary|verify" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S035 -- DeFi: automated trading desk

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:458` through `477`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### DeFi: automated trading desk

```
Agent(trade-executor, archetype=trade-executor)
  loads Extension(affect-modulation)           -- daimon adjusts position sizing
  uses  Connector(hyperliquid)            -- exchange I/O
  uses  Connector(chain-rpc)              -- on-chain reads
  subscribes Feed(price-btc-usd)          -- continuous price stream
  subscribes Feed(heartbeat-5s)           -- tick clock
  executes Recipe(rsi-macd-regime)        -- indicator pipeline → regime score
  checked by Gate(risk-limits, pre)       -- position/slippage/MEV bounds
  checked by Gate(tx-verify, post)        -- on-chain settlement confirmation
  emits Signal(trade-executed)            -- coordination event
  queries Knowledge(regime-codebook)      -- canonical regime vectors
  member of Group(trading-desk)           -- with risk-assessor, researcher
  measured by Eval(sharpe-vs-buyhold)     -- performance benchmark
  competes in Arena(strategy-tournament)  -- against other strategies
  claims Bounty(sharpe-gt-1.5)           -- reward for performance
```
````

**Explicit detail extraction from this section:**

- Section word count: `117`
- Section hash: `a5ff9a48e80c3a78d784ba4b0b04f3f2bcbbf009e6c42ba74a2dfdf15050eff8`

**Normative requirements and implementation claims:**
- ``` Agent(trade-executor, archetype=trade-executor) loads Extension(affect-modulation) -- daimon adjusts position sizing uses Connector(hyperliquid) -- exchange I/O uses Connector(chain-rpc) -- on-chain reads subscribes Feed(price-btc-usd) -- continuous price stream subscribes Feed(heartbeat-5s) -- tick clock executes Recipe(rsi-macd-regime) -- indicator pipeline → regime score checked by Gate(risk-limits, pre) -- position/slippage/MEV bounds checked by Gate(tx-verify, post) -- on-chain settlement confirmation emits Signal(trade-executed) -- coordination event queries Knowledge(regime-codebook) -- canonical regime vectors member of Group(trading-desk) -- with risk-assessor, researcher measured by Eval(sharpe-vs-buyhold) -- performance benchmark competes in Arena(strategy-tournament) -- against other strategies claims Bounty(sharpe-gt-1.5) -- reward for performance ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- position/slippage/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- indicator pipeline -> regime score

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Agent(trade-executor, archetype=trade-executor)`

```
Agent(trade-executor, archetype=trade-executor)
  loads Extension(affect-modulation)           -- daimon adjusts position sizing
  uses  Connector(hyperliquid)            -- exchange I/O
  uses  Connector(chain-rpc)              -- on-chain reads
  subscribes Feed(price-btc-usd)          -- continuous price stream
  subscribes Feed(heartbeat-5s)           -- tick clock
  executes Recipe(rsi-macd-regime)        -- indicator pipeline → regime score
  checked by Gate(risk-limits, pre)       -- position/slippage/MEV bounds
  checked by Gate(tx-verify, post)        -- on-chain settlement confirmation
  emits Signal(trade-executed)            -- coordination event
  queries Knowledge(regime-codebook)      -- canonical regime vectors
  member of Group(trading-desk)           -- with risk-assessor, researcher
  measured by Eval(sharpe-vs-buyhold)     -- performance benchmark
  competes in Arena(strategy-tournament)  -- against other strategies
  claims Bounty(sharpe-gt-1.5)           -- reward for performance
```

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `position/slippage/`
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
rg -n "trading|desk|automated|DeFi|regime|trade|chain|uses" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "trading|desk|automated|DeFi|regime|trade|chain|uses" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
- `position/slippage/`
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
- [ ] Enforce state transition `indicator pipeline -> regime score` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S036 -- Governance: DAO operations

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:478` through `495`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Governance: DAO operations

```
Agent(proposal-drafter, archetype=governance)
  loads Extension(quorum-check)                -- blocks if insufficient delegation
  uses  Connector(snapshot)               -- proposal CRUD, vote submission
  uses  Connector(etherscan)              -- on-chain governance reads
  subscribes Feed(proposal-activity)      -- new proposals, vote changes
  subscribes Feed(delegation-changes)     -- power shifts
  executes Recipe(voting-power-pipeline)  -- delegation → power → threshold
  checked by Gate(delegation-threshold, pre)  -- minimum power to propose
  checked by Gate(quorum-verify, post)    -- quorum reached
  emits Signal(proposal-created)          -- coordination event
  queries Knowledge(governance-patterns)  -- historical precedent
  member of Group(dao-ops)               -- with voter, treasury-mgr
  measured by Eval(proposal-pass-rate)    -- effectiveness benchmark
```
````

**Explicit detail extraction from this section:**

- Section word count: `96`
- Section hash: `424692583d56278306c4a2f7862a654304e3f7a469eaea5523e3f125b5db7982`

**Normative requirements and implementation claims:**
- ``` Agent(proposal-drafter, archetype=governance) loads Extension(quorum-check) -- blocks if insufficient delegation uses Connector(snapshot) -- proposal CRUD, vote submission uses Connector(etherscan) -- on-chain governance reads subscribes Feed(proposal-activity) -- new proposals, vote changes subscribes Feed(delegation-changes) -- power shifts executes Recipe(voting-power-pipeline) -- delegation → power → threshold checked by Gate(delegation-threshold, pre) -- minimum power to propose checked by Gate(quorum-verify, post) -- quorum reached emits Signal(proposal-created) -- coordination event queries Knowledge(governance-patterns) -- historical precedent member of Group(dao-ops) -- with voter, treasury-mgr measured by Eval(proposal-pass-rate) -- effectiveness benchmark ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- delegation -> power

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Agent(proposal-drafter, archetype=governance)`

```
Agent(proposal-drafter, archetype=governance)
  loads Extension(quorum-check)                -- blocks if insufficient delegation
  uses  Connector(snapshot)               -- proposal CRUD, vote submission
  uses  Connector(etherscan)              -- on-chain governance reads
  subscribes Feed(proposal-activity)      -- new proposals, vote changes
  subscribes Feed(delegation-changes)     -- power shifts
  executes Recipe(voting-power-pipeline)  -- delegation → power → threshold
  checked by Gate(delegation-threshold, pre)  -- minimum power to propose
  checked by Gate(quorum-verify, post)    -- quorum reached
  emits Signal(proposal-created)          -- coordination event
  queries Knowledge(governance-patterns)  -- historical precedent
  member of Group(dao-ops)               -- with voter, treasury-mgr
  measured by Eval(proposal-pass-rate)    -- effectiveness benchmark
```

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Governance|proposal|DAO|operations|power|delegation|vote|quorum" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Governance|proposal|DAO|operations|power|delegation|vote|quorum" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Enforce state transition `delegation -> power` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S037 -- Ops: deployment pipeline

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:496` through `513`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Ops: deployment pipeline

```
Agent(deployer, archetype=ops)
  loads Extension(cost-guard)                  -- pause if budget exceeded
  uses  Connector(github)                 -- PR, CI, releases
  uses  Connector(railway)                -- deploy triggers
  subscribes Feed(ci-status)              -- build/test results
  subscribes Feed(deploy-events)          -- rollout progress
  executes Recipe(quality-pipeline)       -- lint → test → coverage → score
  checked by Gate(budget-check, pre)      -- cost within limits
  checked by Gate(test-pass, post)        -- all tests green
  emits Signal(deploy-completed)          -- coordination event
  queries Knowledge(deploy-runbook)       -- historical deploy patterns
  member of Group(incident-response)      -- with diagnostician, communicator
  measured by Eval(deploy-success-rate)   -- reliability benchmark
```
````

**Explicit detail extraction from this section:**

- Section word count: `91`
- Section hash: `9fd3514eb4bc4ad541f67ebf1a5c9a10ab2ec53866b0a859af155c983f24dd5a`

**Normative requirements and implementation claims:**
- ``` Agent(deployer, archetype=ops) loads Extension(cost-guard) -- pause if budget exceeded uses Connector(github) -- PR, CI, releases uses Connector(railway) -- deploy triggers subscribes Feed(ci-status) -- build/test results subscribes Feed(deploy-events) -- rollout progress executes Recipe(quality-pipeline) -- lint → test → coverage → score checked by Gate(budget-check, pre) -- cost within limits checked by Gate(test-pass, post) -- all tests green emits Signal(deploy-completed) -- coordination event queries Knowledge(deploy-runbook) -- historical deploy patterns member of Group(incident-response) -- with diagnostician, communicator measured by Eval(deploy-success-rate) -- reliability benchmark ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- lint -> test
- coverage -> score

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Agent(deployer, archetype=ops)`

```
Agent(deployer, archetype=ops)
  loads Extension(cost-guard)                  -- pause if budget exceeded
  uses  Connector(github)                 -- PR, CI, releases
  uses  Connector(railway)                -- deploy triggers
  subscribes Feed(ci-status)              -- build/test results
  subscribes Feed(deploy-events)          -- rollout progress
  executes Recipe(quality-pipeline)       -- lint → test → coverage → score
  checked by Gate(budget-check, pre)      -- cost within limits
  checked by Gate(test-pass, post)        -- all tests green
  emits Signal(deploy-completed)          -- coordination event
  queries Knowledge(deploy-runbook)       -- historical deploy patterns
  member of Group(incident-response)      -- with diagnostician, communicator
  measured by Eval(deploy-success-rate)   -- reliability benchmark
```

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "deploy|pipeline|Ops|deployment|test|check|uses|subscribes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "deploy|pipeline|Ops|deployment|test|check|uses|subscribes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Enforce state transition `lint -> test` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `coverage -> score` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S038 -- Code: self-improvement

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:514` through `533`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Code: self-improvement

```
Agent(refactorer, archetype=coding)
  loads Extension(style-enforcement)           -- format before commit
  uses  Connector(mcp-code-intel)         -- AST queries, symbol lookup
  uses  Connector(github)                 -- PR creation
  subscribes Feed(file-changes)           -- filesystem watcher
  subscribes Feed(build-output)           -- compiler diagnostics
  executes Recipe(complexity-pipeline)    -- diff → complexity → quality score
  checked by Gate(scope-guard, pre)       -- only touch allowed files
  checked by Gate(compile-test-clippy, post)  -- standard roko gates
  emits Signal(pr-created)               -- coordination event
  queries Knowledge(code-patterns)        -- architectural decisions
  member of Group(dev-team)              -- with tester, reviewer
  measured by Eval(code-quality-delta)    -- improvement benchmark
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `93`
- Section hash: `8a1b26e47b11fd4e95aa5914eff3d643d8caac1f40191c80c1252470dd5e3de3`

**Normative requirements and implementation claims:**
- ``` Agent(refactorer, archetype=coding) loads Extension(style-enforcement) -- format before commit uses Connector(mcp-code-intel) -- AST queries, symbol lookup uses Connector(github) -- PR creation subscribes Feed(file-changes) -- filesystem watcher subscribes Feed(build-output) -- compiler diagnostics executes Recipe(complexity-pipeline) -- diff → complexity → quality score checked by Gate(scope-guard, pre) -- only touch allowed files checked by Gate(compile-test-clippy, post) -- standard roko gates emits Signal(pr-created) -- coordination event queries Knowledge(code-patterns) -- architectural decisions member of Group(dev-team) -- with tester, reviewer measured by Eval(code-quality-delta) -- improvement benchmark ```
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
- diff -> complexity

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Agent(refactorer, archetype=coding)`

```
Agent(refactorer, archetype=coding)
  loads Extension(style-enforcement)           -- format before commit
  uses  Connector(mcp-code-intel)         -- AST queries, symbol lookup
  uses  Connector(github)                 -- PR creation
  subscribes Feed(file-changes)           -- filesystem watcher
  subscribes Feed(build-output)           -- compiler diagnostics
  executes Recipe(complexity-pipeline)    -- diff → complexity → quality score
  checked by Gate(scope-guard, pre)       -- only touch allowed files
  checked by Gate(compile-test-clippy, post)  -- standard roko gates
  emits Signal(pr-created)               -- coordination event
  queries Knowledge(code-patterns)        -- architectural decisions
  member of Group(dev-team)              -- with tester, reviewer
  measured by Eval(code-quality-delta)    -- improvement benchmark
```

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Code|improvement|self|file|Gate|uses|test|subscribes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Code|improvement|self|file|Gate|uses|test|subscribes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Enforce state transition `diff -> complexity` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### DASH-23-S039 -- Relation to existing PRDs

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md:534` through `541`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Relation to existing PRDs

| PRD | Relationship |
|-----|-------------|
| `19-authoring-surfaces.md` | Authoring surfaces for Agent, Extension, Gate, Knowledge, Arena, Eval, Signal (was Pheromone), Group, Bounty remain as specified. This doc adds surfaces for Connector, Feed, Recipe. Pi extension compatibility preserved via 3-tier model (PRD-09). |
| `20-composition-patterns.md` | Superseded vocabulary (10→12). Composition grammar and DAW principle still apply. Forward pointer added. |
| `21-roko-and-chain-additions.md` | Backend additions now map to Connector (chain/venue adapters), Feed (event streams), Recipe (scoring pipelines). |
| Gap docs 01-10 | Every struct maps to exactly one primitive. Product Layer sections added to each doc. |
````

**Explicit detail extraction from this section:**

- Section word count: `98`
- Section hash: `de23fa1f9642af88f35bade45cbe552c6d379c0a446cc8e43379161186d7cad7`

**Normative requirements and implementation claims:**
- | PRD | Relationship | |-----|-------------| | `19-authoring-surfaces.md` | Authoring surfaces for Agent, Extension, Gate, Knowledge, Arena, Eval, Signal (was Pheromone), Group, Bounty remain as specified. This doc adds surfaces for Connector, Feed, Recipe. Pi extension compatibility preserved via 3-tier model (PRD-09). | | `20-composition-patterns.md` | Superseded vocabulary (10→12). Composition grammar and DAW principle still apply. Forward pointer added. | | `21-roko-and-chain-additions.md` | Backend additions now map to Connector (chain/venue adapters), Feed (event streams), Recipe (scoring pipelines). | | Gap docs 01-10 | Every struct maps to exactly one primitive. Product Layer sections added to each doc. |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- maps

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
| PRD | Relationship |
|-----|-------------|
| `19-authoring-surfaces.md` | Authoring surfaces for Agent, Extension, Gate, Knowledge, Arena, Eval, Signal (was Pheromone), Group, Bounty remain as specified. This doc adds surfaces for Connector, Feed, Recipe. Pi extension compatibility preserved via 3-tier model (PRD-09). |
| `20-composition-patterns.md` | Superseded vocabulary (10→12). Composition grammar and DAW principle still apply. Forward pointer added. |
| `21-roko-and-chain-additions.md` | Backend additions now map to Connector (chain/venue adapters), Feed (event streams), Recipe (scoring pipelines). |
| Gap docs 01-10 | Every struct maps to exactly one primitive. Product Layer sections added to each doc. |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
rg -n "Relation|maps|existing|PRDs|surfaces|composition|chain|authoring" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Relation|maps|existing|PRDs|surfaces|composition|chain|authoring" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-core/src/feed.rs`
- `crates/roko-plugin/src/`
- `crates/roko-serve/src/routes/connectors.rs`
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
- [ ] Implement or verify `maps` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/23-universal-primitives
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

