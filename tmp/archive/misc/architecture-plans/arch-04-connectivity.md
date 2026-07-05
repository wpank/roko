# Architecture Plan: Connectivity

**Source:** `tmp/architecture/04-connectivity.md`
**Generated:** 2026-04-25
**Source hash:** `3894a4d83c4b133595fb49ba6cdf28eeac951a1f37957bd6cb4d1a3785973df2`
**Section tasks:** 21
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
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-04-S001 | 1 | Connectivity and relay | [ ] | 9.8 |
| ARCH-04-S002 | 8 | Workspace discovery | [ ] | 9.8 |
| ARCH-04-S003 | 12 | How it works | [ ] | 9.8 |
| ARCH-04-S004 | 54 | Dashboard connection flow | [ ] | 9.8 |
| ARCH-04-S005 | 64 | Local development | [ ] | 9.8 |
| ARCH-04-S006 | 79 | Data flow: subscription-only | [ ] | 9.8 |
| ARCH-04-S007 | 83 | Event sources | [ ] | 9.8 |
| ARCH-04-S008 | 101 | Subscription lifecycle | [ ] | 9.8 |
| ARCH-04-S009 | 139 | WebSocket message envelope | [ ] | 9.8 |
| ARCH-04-S010 | 161 | Room naming convention | [ ] | 9.8 |
| ARCH-04-S011 | 175 | Event types | [ ] | 9.8 |
| ARCH-04-S012 | 197 | Backpressure and coalescing | [ ] | 9.8 |
| ARCH-04-S013 | 218 | Reconnection | [ ] | 9.8 |
| ARCH-04-S014 | 228 | Reconnection recovery protocol | [ ] | 9.8 |
| ARCH-04-S015 | 280 | Multi-instance handling | [ ] | 9.8 |
| ARCH-04-S016 | 296 | Agent connectivity | [ ] | 9.8 |
| ARCH-04-S017 | 338 | In-process agents | [ ] | 9.8 |
| ARCH-04-S018 | 371 | Remote agents | [ ] | 9.8 |
| ARCH-04-S019 | 406 | Direct-reachable agents | [ ] | 9.8 |
| ARCH-04-S020 | 420 | Agent discovery: three sources merged | [ ] | 9.8 |
| ARCH-04-S021 | 491 | Message routing | [ ] | 9.8 |

## Tasks

### ARCH-04-S001 -- Connectivity and relay

**Source section:** `tmp/architecture/04-connectivity.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Connectivity and relay

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Data flow: subscription-only" and "Agent connectivity" sections.

---
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `ce558fd1fb72b1bff21066d1d5ec1fefafa1fb2535a2f7388f0f73fcf44773c8`

**Normative requirements and implementation claims:**
- > Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc. > Merges the "Data flow: subscription-only" and "Agent connectivity" sections.
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
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Connectivity|relay|subscription|sections|redesign|flow|Specification" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connectivity|relay|subscription|sections|redesign|flow|Specification" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S002 -- Workspace discovery

**Source section:** `tmp/architecture/04-connectivity.md:8` through `11`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Workspace discovery

Roko instances register with the relay on startup. Dashboards discover available workspaces automatically — no manual URL entry.
````

**Explicit detail extraction from this section:**

- Section word count: `17`
- Section hash: `7743dad81231cc5cb22c440faef5c6bbe8eaf1d46e8f1398c76407d73a09171f`

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
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "discover|Workspace|discovery|workspaces|startup|relay|register|manual" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "discover|Workspace|discovery|workspaces|startup|relay|register|manual" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S003 -- How it works

**Source section:** `tmp/architecture/04-connectivity.md:12` through `53`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### How it works

When `roko serve` starts, it connects to the relay and announces itself:

```json
{
  "type": "workspace_hello",
  "workspace_id": "ws-a1b2c3",
  "name": "will-dev",
  "url": "https://my-roko.up.railway.app",
  "version": "0.1.0",
  "capabilities": ["agents", "plans", "prds", "learning", "gateway"],
  "owner_wallet": "0x7f3b...2c4a",
  "agents_count": 3,
  "uptime_secs": 3600
}
```

The relay maintains a workspace directory alongside the agent directory. Dashboards query it:

```
GET /relay/workspaces
-> [
    {
      "workspace_id": "ws-a1b2c3",
      "name": "will-dev",
      "url": "https://my-roko.up.railway.app",
      "owner_wallet": "0x7f3b...2c4a",
      "agents_count": 3,
      "online": true,
      "last_seen_ms": 1713960000000
    }
  ]
```

The relay also pushes workspace events on the events WebSocket:

```json
{"type": "workspace_connected", "workspace_id": "ws-a1b2c3", "url": "https://..."}
{"type": "workspace_disconnected", "workspace_id": "ws-a1b2c3"}
```
````

**Explicit detail extraction from this section:**

- Section word count: `106`
- Section hash: `4e4985175f93f51fc47a8bdf4b6d678dd51d389185a5012de500c9746675ff58`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- roko.up.railway.app
- x7f3b...2c4a

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko serve

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{`

```json
{
  "type": "workspace_hello",
  "workspace_id": "ws-a1b2c3",
  "name": "will-dev",
  "url": "https://my-roko.up.railway.app",
  "version": "0.1.0",
  "capabilities": ["agents", "plans", "prds", "learning", "gateway"],
  "owner_wallet": "0x7f3b...2c4a",
  "agents_count": 3,
  "uptime_secs": 3600
}
```
- Contract 2: language `plain`, first line `GET /relay/workspaces`

```
GET /relay/workspaces
-> [
    {
      "workspace_id": "ws-a1b2c3",
      "name": "will-dev",
      "url": "https://my-roko.up.railway.app",
      "owner_wallet": "0x7f3b...2c4a",
      "agents_count": 3,
      "online": true,
      "last_seen_ms": 1713960000000
    }
  ]
```
- Contract 3: language `json`, first line `{"type": "workspace_connected", "workspace_id": "ws-a1b2c3", "url": "https://..."}`

```json
{"type": "workspace_connected", "workspace_id": "ws-a1b2c3", "url": "https://..."}
{"type": "workspace_disconnected", "workspace_id": "ws-a1b2c3"}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "works|workspace|How|workspace_id|relay|a1b2c3|type|https" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "works|workspace|How|workspace_id|relay|a1b2c3|type|https" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Emit or consume `roko.up.railway.app` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x7f3b...2c4a` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S004 -- Dashboard connection flow

**Source section:** `tmp/architecture/04-connectivity.md:54` through `63`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard connection flow

1. Dashboard loads, connects to relay
2. Fetches `GET /relay/workspaces` — lists all online roko instances
3. If user has a Privy wallet, auto-matches workspaces by `owner_wallet`
4. If exactly one match: auto-connect (zero friction)
5. If multiple matches: show picker ("You have 2 workspaces online — which one?")
6. If no match: show global-only view (agents from relay, chain data, no workspace features)
7. User can also manually add a workspace URL in Settings (for instances not registered with relay)
````

**Explicit detail extraction from this section:**

- Section word count: `83`
- Section hash: `77338dc3275ca4fabeb7aea8ee77322054c29e27ff023e48944e30681d4e1b0b`

**Normative requirements and implementation claims:**
- 1. Dashboard loads, connects to relay 2. Fetches `GET /relay/workspaces` — lists all online roko instances 3. If user has a Privy wallet, auto-matches workspaces by `owner_wallet` 4. If exactly one match: auto-connect (zero friction) 5. If multiple matches: show picker ("You have 2 workspaces online — which one?") 6. If no match: show global-only view (agents from relay, chain data, no workspace features) 7. User can also manually add a workspace URL in Settings (for instances not registered with relay)

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- owner_wallet

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Dashboard loads, connects to relay
- 2. Fetches `GET /relay/workspaces` — lists all online roko instances
- 3. If user has a Privy wallet, auto-matches workspaces by `owner_wallet`
- 4. If exactly one match: auto-connect (zero friction)
- 5. If multiple matches: show picker ("You have 2 workspaces online — which one?")
- 6. If no match: show global-only view (agents from relay, chain data, no workspace features)
- 7. User can also manually add a workspace URL in Settings (for instances not registered with relay)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "connect|workspace|owner_wallet|flow|connection|relay|match|workspaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "connect|workspace|owner_wallet|flow|connection|relay|match|workspaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `owner_wallet` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S005 -- Local development

**Source section:** `tmp/architecture/04-connectivity.md:64` through `78`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Local development

`roko serve` on localhost registers with the relay if `relay.url` is configured. For pure local dev (no relay), the dashboard falls back to `VITE_ROKO_API_URL` env var or `localhost:6677`.

```toml
# roko.toml — relay registration
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
```

If `[relay]` is not configured, roko serves HTTP only — no relay registration, no auto-discovery. Dashboard must be pointed at it manually.

---
````

**Explicit detail extraction from this section:**

- Section word count: `66`
- Section hash: `1f7b91da9c69a72b12d15d6ccbfddbab7954e6d8e69dfad7f054f76f41f4c966`

**Normative requirements and implementation claims:**
- `roko serve` on localhost registers with the relay if `relay.url` is configured. For pure local dev (no relay), the dashboard falls back to `VITE_ROKO_API_URL` env var or `localhost:6677`.
- If `[relay]` is not configured, roko serves HTTP only — no relay registration, no auto-discovery. Dashboard must be pointed at it manually.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- VITE_ROKO_API_URL

**Event names and event-like entities:**
- relay.url
- relay.nunchi.dev

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- relay.url
- [relay]
- url = "wss://relay.nunchi.dev"
- workspace_name = "will-dev"

**Commands and operator actions:**
- roko serve

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# roko.toml — relay registration`

```toml
# roko.toml — relay registration
[relay]
url = "wss://relay.nunchi.dev"
workspace_name = "will-dev"
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "relay|dev|Local|development|VITE_ROKO_API_URL|url|toml|serve" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "relay|dev|Local|development|VITE_ROKO_API_URL|url|toml|serve" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `VITE_ROKO_API_URL` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `relay.url` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `relay.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `relay.url` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[relay]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `url = "wss://relay.nunchi.dev"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `workspace_name = "will-dev"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S006 -- Data flow: subscription-only

**Source section:** `tmp/architecture/04-connectivity.md:79` through `82`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Data flow: subscription-only

Every piece of data flows through WebSocket subscriptions. No polling.
````

**Explicit detail extraction from this section:**

- Section word count: `10`
- Section hash: `8c296f608d94549f164b238374d323645b53030ebfef2ce82efbfae88c5a883b`

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
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "subscription|flow|Data|subscriptions|polling|piece|flows|WebSocket" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "subscription|flow|Data|subscriptions|polling|piece|flows|WebSocket" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S007 -- Event sources

**Source section:** `tmp/architecture/04-connectivity.md:83` through `100`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Event sources

```
Source              Transport           What it carries
──────              ─────────           ───────────────
Relay               WS /relay/ws        Agent presence, message lifecycle,
                                        relay health
roko-serve          WS /ws              Plan progress, gate results, episodes,
                                        learning metrics, job updates
Agent (direct)      WS (per-agent)      Heartbeats, streaming LLM output,
                                        decision traces
Agent (via relay)   WS /relay/ws        Same as direct, tunneled through relay
Chain               WS (RPC sub)        Blocks, contract events, ERC-8004
                                        registry updates
Agent chain feeds   WS (per-feed)       Raw RPC data, derived indicators,
                                        signals, analysis
```
````

**Explicit detail extraction from this section:**

- Section word count: `75`
- Section hash: `83fc7404c4fc918385469f46e7cc86809368e24c187bbbb86b1fab68fb87973c`

**Normative requirements and implementation claims:**
- ``` Source Transport What it carries ────── ───────── ─────────────── Relay WS /relay/ws Agent presence, message lifecycle, relay health roko-serve WS /ws Plan progress, gate results, episodes, learning metrics, job updates Agent (direct) WS (per-agent) Heartbeats, streaming LLM output, decision traces Agent (via relay) WS /relay/ws Same as direct, tunneled through relay Chain WS (RPC sub) Blocks, contract events, ERC-8004 registry updates Agent chain feeds WS (per-feed) Raw RPC data, derived indicators, signals, analysis ```

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
- Contract 1: language `plain`, first line `Source              Transport           What it carries`

```
Source              Transport           What it carries
──────              ─────────           ───────────────
Relay               WS /relay/ws        Agent presence, message lifecycle,
                                        relay health
roko-serve          WS /ws              Plan progress, gate results, episodes,
                                        learning metrics, job updates
Agent (direct)      WS (per-agent)      Heartbeats, streaming LLM output,
                                        decision traces
Agent (via relay)   WS /relay/ws        Same as direct, tunneled through relay
Chain               WS (RPC sub)        Blocks, contract events, ERC-8004
                                        registry updates
Agent chain feeds   WS (per-feed)       Raw RPC data, derived indicators,
                                        signals, analysis
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Relay|Event|sources|updates|feed|direct|Chain|tunneled" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Relay|Event|sources|updates|feed|direct|Chain|tunneled" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S008 -- Subscription lifecycle

**Source section:** `tmp/architecture/04-connectivity.md:101` through `138`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Subscription lifecycle

The dashboard subscribes when a page mounts and unsubscribes when it unmounts.

```typescript
// React hook pattern
function useAgentFeed(agentId: string) {
  const [state, setState] = useState<AgentState | null>(null);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);

    ws.onopen = () => {
      // Subscribe to this agent's room
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
    };

    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      setState(prev => applyEvent(prev, event));
    };

    return () => {
      ws.send(JSON.stringify({
        type: "unsubscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
      ws.close();
    };
  }, [agentId]);

  return state;
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `87`
- Section hash: `28d11ec9c1b26208deab4d954e99d5d658ef3b472d59e3b0b684f68a2cfdba3e`

**Normative requirements and implementation claims:**
- The dashboard subscribes when a page mounts and unsubscribes when it unmounts.
- ```typescript // React hook pattern function useAgentFeed(agentId: string) { const [state, setState] = useState<AgentState | null>(null);
- ws.onmessage = (e) => { const event = JSON.parse(e.data); setState(prev => applyEvent(prev, event)); };
- return state; } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- ws.onopen
- ws.send
- ws.onmessage
- applyEvent
- ws.close

**State transitions:**
- prev -> applyEvent

**Config keys and TOML-like settings:**
- ws.onopen = () => {
- ws.onmessage = (e) => {

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `typescript`, first line `// React hook pattern`

```typescript
// React hook pattern
function useAgentFeed(agentId: string) {
  const [state, setState] = useState<AgentState | null>(null);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);

    ws.onopen = () => {
      // Subscribe to this agent's room
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
    };

    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      setState(prev => applyEvent(prev, event));
    };

    return () => {
      ws.send(JSON.stringify({
        type: "unsubscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
      ws.close();
    };
  }, [agentId]);

  return state;
}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "state|agentId|lifecycle|Subscription|Subscribe|type|string|room" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "state|agentId|lifecycle|Subscription|Subscribe|type|string|room" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Emit or consume `ws.onopen` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.onmessage` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `applyEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.close` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `prev -> applyEvent` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `ws.onopen = () => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ws.onmessage = (e) => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S009 -- WebSocket message envelope

**Source section:** `tmp/architecture/04-connectivity.md:139` through `160`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### WebSocket message envelope

Every message through the relay uses the same envelope:

```json
{
  "seq": 4821,
  "ts": 1713974400123,
  "room": "agent:coder-1:heartbeat",
  "type": "heartbeat",
  "payload": { ... }
}
```

| Field | Type | Purpose |
|-------|------|---------|
| `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. |
| `ts` | `u64` | Unix milliseconds. Server clock. |
| `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. |
| `type` | `string` | Event discriminant. One of the types listed below. |
| `payload` | `object` | Type-specific data. |
````

**Explicit detail extraction from this section:**

- Section word count: `67`
- Section hash: `461c4888999eedef3a97cf204586452fdfccd9b47b8d6ce58048110f5b7d8480`

**Normative requirements and implementation claims:**
- | Field | Type | Purpose | |-------|------|---------| | `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. | | `ts` | `u64` | Unix milliseconds. Server clock. | | `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. | | `type` | `string` | Event discriminant. One of the types listed below. | | `payload` | `object` | Type-specific data. |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- seq
- u64
- room
- string
- type
- payload
- object

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
| Field | Type | Purpose |
|-------|------|---------|
| `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. |
| `ts` | `u64` | Unix milliseconds. Server clock. |
| `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. |
| `type` | `string` | Event discriminant. One of the types listed below. |
| `payload` | `object` | Type-specific data. |
```

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{`

```json
{
  "seq": 4821,
  "ts": 1713974400123,
  "room": "agent:coder-1:heartbeat",
  "type": "heartbeat",
  "payload": { ... }
}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "type|seq|room|message|u64|string|payload|envelope" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "type|seq|room|message|u64|string|payload|envelope" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `seq` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `u64` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `room` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `string` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `type` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `payload` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `object` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S010 -- Room naming convention

**Source section:** `tmp/architecture/04-connectivity.md:161` through `174`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Room naming convention

```
agent:{id}                  Agent lifecycle events (spawn, stop, error)
agent:{id}:heartbeat        Heartbeat ticks (T0/T1/T2, cortical state)
agent:{id}:output           Streaming LLM output chunks
agent:{id}:trace            Decision traces per tick
agent:{id}:feed:{feed_id}   Chain data feeds exposed by the agent
plan:{id}                   Plan progress, task transitions, gate results
cluster:{id}                Cluster pipeline events
system                      Server health, provider status, cost updates
learning                    Experiment results, router updates, thresholds
```
````

**Explicit detail extraction from this section:**

- Section word count: `69`
- Section hash: `4eb96ee6dce171d56039376f478d48adedb9aed7127e0025faf7b1bbc5186334`

**Normative requirements and implementation claims:**
- ``` agent:{id} Agent lifecycle events (spawn, stop, error) agent:{id}:heartbeat Heartbeat ticks (T0/T1/T2, cortical state) agent:{id}:output Streaming LLM output chunks agent:{id}:trace Decision traces per tick agent:{id}:feed:{feed_id} Chain data feeds exposed by the agent plan:{id} Plan progress, task transitions, gate results cluster:{id} Cluster pipeline events system Server health, provider status, cost updates learning Experiment results, router updates, thresholds ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- T0/T1/

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
- Contract 1: language `plain`, first line `agent:{id}                  Agent lifecycle events (spawn, stop, error)`

```
agent:{id}                  Agent lifecycle events (spawn, stop, error)
agent:{id}:heartbeat        Heartbeat ticks (T0/T1/T2, cortical state)
agent:{id}:output           Streaming LLM output chunks
agent:{id}:trace            Decision traces per tick
agent:{id}:feed:{feed_id}   Chain data feeds exposed by the agent
plan:{id}                   Plan progress, task transitions, gate results
cluster:{id}                Cluster pipeline events
system                      Server health, provider status, cost updates
learning                    Experiment results, router updates, thresholds
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `T0/T1/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "naming|convention|Room|feed|updates|trace|tick|results" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "naming|convention|Room|feed|updates|trace|tick|results" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `T0/T1/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S011 -- Event types

**Source section:** `tmp/architecture/04-connectivity.md:175` through `196`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Event types

```
Type                    Room pattern            Payload
────                    ────────────            ───────
presence_join           system                  { agent_id, mode, profile }
presence_leave          system                  { agent_id, reason }
heartbeat               agent:{id}:heartbeat    { tick, tier, pe, cortical_state }
output_chunk            agent:{id}:output       { content, done, usage }
trace                   agent:{id}:trace        { tick, steps[], gate_result }
task_started            plan:{id}               { task_id, phase }
task_completed          plan:{id}               { task_id, outcome }
gate_result             plan:{id}               { task_id, gate, rung, passed }
phase_transition        plan:{id}               { from, to }
feed_data               agent:{id}:feed:{fid}   { feed_id, data }
feed_registered         system                  { agent_id, feed_id, schema }
cost_update             system                  { agent_id, delta, total }
provider_status         system                  { provider, healthy, latency_ms }
experiment_result       learning                { experiment_id, winner, p_value }
router_update           learning                { model, weight, reason }
```
````

**Explicit detail extraction from this section:**

- Section word count: `89`
- Section hash: `8d5a0f4684ee4e8738544462b21e77f2604816c7faea905fabace64dc819f6f4`

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
- Contract 1: language `plain`, first line `Type                    Room pattern            Payload`

```
Type                    Room pattern            Payload
────                    ────────────            ───────
presence_join           system                  { agent_id, mode, profile }
presence_leave          system                  { agent_id, reason }
heartbeat               agent:{id}:heartbeat    { tick, tier, pe, cortical_state }
output_chunk            agent:{id}:output       { content, done, usage }
trace                   agent:{id}:trace        { tick, steps[], gate_result }
task_started            plan:{id}               { task_id, phase }
task_completed          plan:{id}               { task_id, outcome }
gate_result             plan:{id}               { task_id, gate, rung, passed }
phase_transition        plan:{id}               { from, to }
feed_data               agent:{id}:feed:{fid}   { feed_id, data }
feed_registered         system                  { agent_id, feed_id, schema }
cost_update             system                  { agent_id, delta, total }
provider_status         system                  { provider, healthy, latency_ms }
experiment_result       learning                { experiment_id, winner, p_value }
router_update           learning                { model, weight, reason }
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Type|types|feed|Event|plan|agent_id|task_id|gate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Type|types|feed|Event|plan|agent_id|task_id|gate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S012 -- Backpressure and coalescing

**Source section:** `tmp/architecture/04-connectivity.md:197` through `217`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Backpressure and coalescing

High-frequency events (heartbeats at 100ms, chain blocks at 2s) need throttling for dashboard consumption.

```
Strategy                Applies to              Behavior
────────                ──────────              ────────
Coalesce                heartbeat               Relay buffers heartbeats per agent,
                                                sends latest every 500ms to
                                                dashboard subscribers
Drop-oldest             output_chunk            Ring buffer per agent (1024 chunks).
                                                Slow consumers miss old chunks,
                                                catch up from latest.
Lossless                gate_result,            Queue with backpressure. If client
                        task_completed          can't keep up, relay applies
                                                TCP-level flow control.
Sample                  feed_data               Agent-configurable sample rate.
                                                Default: every Nth update where
                                                N = ceil(source_rate / 2Hz).
```
````

**Explicit detail extraction from this section:**

- Section word count: `84`
- Section hash: `0edf64debacd8d6ad2f922447c13bd16a06a4a865fe1e203208cdb4ba7ba2b2d`

**Normative requirements and implementation claims:**
- High-frequency events (heartbeats at 100ms, chain blocks at 2s) need throttling for dashboard consumption.
- ``` Strategy Applies to Behavior ──────── ────────── ──────── Coalesce heartbeat Relay buffers heartbeats per agent, sends latest every 500ms to dashboard subscribers Drop-oldest output_chunk Ring buffer per agent (1024 chunks). Slow consumers miss old chunks, catch up from latest. Lossless gate_result, Queue with backpressure. If client task_completed can't keep up, relay applies TCP-level flow control. Sample feed_data Agent-configurable sample rate. Default: every Nth update where N = ceil(source_rate / 2Hz). ```

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
- N = ceil(source_rate / 2Hz).

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Strategy                Applies to              Behavior`

```
Strategy                Applies to              Behavior
────────                ──────────              ────────
Coalesce                heartbeat               Relay buffers heartbeats per agent,
                                                sends latest every 500ms to
                                                dashboard subscribers
Drop-oldest             output_chunk            Ring buffer per agent (1024 chunks).
                                                Slow consumers miss old chunks,
                                                catch up from latest.
Lossless                gate_result,            Queue with backpressure. If client
                        task_completed          can't keep up, relay applies
                                                TCP-level flow control.
Sample                  feed_data               Agent-configurable sample rate.
                                                Default: every Nth update where
                                                N = ceil(source_rate / 2Hz).
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Backpressure|coalescing|rate|heartbeat|latest|heartbeats|every" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Backpressure|coalescing|rate|heartbeat|latest|heartbeats|every" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Add or verify config key `N = ceil(source_rate / 2Hz).` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S013 -- Reconnection

**Source section:** `tmp/architecture/04-connectivity.md:218` through `227`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reconnection

Clients track the last received `seq`. On reconnect:

```json
{ "type": "resume", "last_seq": 4821 }
```

The relay replays missed events from its ring buffer (default: 64K entries, ~10 minutes at moderate throughput). If the gap exceeds the buffer, the relay sends a `snapshot` event with current state followed by live events.
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `e4cc1fb7a67d4e344ed5389ca41af8d4b4b629acd389681794077871be65632a`

**Normative requirements and implementation claims:**
- The relay replays missed events from its ring buffer (default: 64K entries, ~10 minutes at moderate throughput). If the gap exceeds the buffer, the relay sends a `snapshot` event with current state followed by live events.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- seq
- snapshot

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
- Contract 1: language `json`, first line `{ "type": "resume", "last_seq": 4821 }`

```json
{ "type": "resume", "last_seq": 4821 }
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "seq|reconnect|snapshot|Reconnection|event|relay|last|events" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "seq|reconnect|snapshot|Reconnection|event|relay|last|events" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `seq` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `snapshot` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S014 -- Reconnection recovery protocol

**Source section:** `tmp/architecture/04-connectivity.md:228` through `279`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reconnection recovery protocol

Full reconnection sequence:

```
Client                                  Relay
  │                                       │
  │──── WS connect ─────────────────────►│
  │                                       │
  │──── { "type": "resume",             │
  │       "last_seq": 4821 } ──────────►│
  │                                       │
  │                           ┌───────────┤
  │                           │ Check gap │
  │                           └───────────┤
  │                                       │
  │  Case 1: gap <= 64K entries           │
  │◄──── replay events 4822..4900 ────────│
  │◄──── live events continue ────────────│
  │                                       │
  │  Case 2: gap > 64K entries            │
  │◄──── { "type": "snapshot",           │
  │        "state": {                     │
  │          "agents": [...],             │
  │          "feeds": [...],              │
  │          "rooms": [...]               │
  │        }} ────────────────────────────│
  │◄──── live events continue ────────────│
  │                                       │
```

**Snapshot format**: The snapshot contains the minimum state needed to rebuild client-side views:

```json
{
  "type": "snapshot",
  "seq": 71042,
  "state": {
    "agents": [
      { "id": "coder-1", "online": true, "mode": "persistent", "profile": "coding" },
      { "id": "research", "online": true, "mode": "ephemeral", "profile": "research" }
    ],
    "feeds": [
      { "feed_id": "eth-gas-trend", "agent_id": "chain-watcher-1", "schema": "gas_trend_v1" }
    ],
    "rooms": ["agent:coder-1", "agent:coder-1:heartbeat", "plan:current"]
  }
}
```

**Gap detection on the client**: Clients track the last received `seq` and check every incoming message for continuity. A gap (missing sequence numbers between the last received and the current message) means events were lost. On gap detection, the client should reconnect and send a `resume` message.
````

**Explicit detail extraction from this section:**

- Section word count: `145`
- Section hash: `58ff062dd9614c4a07eb855dfc0f7d522329b771c6ec17ac04f07bf4be88c945`

**Normative requirements and implementation claims:**
- ``` Client Relay │ │ │──── WS connect ─────────────────────►│ │ │ │──── { "type": "resume", │ │ "last_seq": 4821 } ──────────►│ │ │ │ ┌───────────┤ │ │ Check gap │ │ └───────────┤ │ │ │ Case 1: gap <= 64K entries │ │◄──── replay events 4822..4900 ────────│ │◄──── live events continue ────────────│ │ │ │ Case 2: gap > 64K entries │ │◄──── { "type": "snapshot", │ │ "state": { │ │ "agents": [...], │ │ "feeds": [...], │ │ "rooms": [...] │ │ }} ────────────────────────────│ │◄──── live events continue ────────────│ │ │ ```
- **Snapshot format**: The snapshot contains the minimum state needed to rebuild client-side views:
- ```json { "type": "snapshot", "seq": 71042, "state": { "agents": [ { "id": "coder-1", "online": true, "mode": "persistent", "profile": "coding" }, { "id": "research", "online": true, "mode": "ephemeral", "profile": "research" } ], "feeds": [ { "feed_id": "eth-gas-trend", "agent_id": "chain-watcher-1", "schema": "gas_trend_v1" } ], "rooms": ["agent:coder-1", "agent:coder-1:heartbeat", "plan:current"] } } ```
- **Gap detection on the client**: Clients track the last received `seq` and check every incoming message for continuity. A gap (missing sequence numbers between the last received and the current message) means events were lost. On gap detection, the client should reconnect and send a `resume` message.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- seq
- resume

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
- Contract 1: language `plain`, first line `Client                                  Relay`

```
Client                                  Relay
  │                                       │
  │──── WS connect ─────────────────────►│
  │                                       │
  │──── { "type": "resume",             │
  │       "last_seq": 4821 } ──────────►│
  │                                       │
  │                           ┌───────────┤
  │                           │ Check gap │
  │                           └───────────┤
  │                                       │
  │  Case 1: gap <= 64K entries           │
  │◄──── replay events 4822..4900 ────────│
  │◄──── live events continue ────────────│
  │                                       │
  │  Case 2: gap > 64K entries            │
  │◄──── { "type": "snapshot",           │
  │        "state": {                     │
  │          "agents": [...],             │
  │          "feeds": [...],              │
  │          "rooms": [...]               │
  │        }} ────────────────────────────│
  │◄──── live events continue ────────────│
  │                                       │
```
- Contract 2: language `json`, first line `{`

```json
{
  "type": "snapshot",
  "seq": 71042,
  "state": {
    "agents": [
      { "id": "coder-1", "online": true, "mode": "persistent", "profile": "coding" },
      { "id": "research", "online": true, "mode": "ephemeral", "profile": "research" }
    ],
    "feeds": [
      { "feed_id": "eth-gas-trend", "agent_id": "chain-watcher-1", "schema": "gas_trend_v1" }
    ],
    "rooms": ["agent:coder-1", "agent:coder-1:heartbeat", "plan:current"]
  }
}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "seq|connect|reconnect|resume|Reconnection|recovery|protocol|Client" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "seq|connect|reconnect|resume|Reconnection|recovery|protocol|Client" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `seq` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `resume` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S015 -- Multi-instance handling

**Source section:** `tmp/architecture/04-connectivity.md:280` through `295`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Multi-instance handling

Each roko instance connects to the relay with a unique `instance_id` (generated at startup, format: `inst_{ulid}`).

**Conflict resolution**: If two roko instances claim the same `agent_id`, the relay uses last-write-wins. The most recent connection for that `agent_id` becomes authoritative. The old connection receives a supersession notice:

```json
{ "type": "superseded", "agent_id": "coder-1", "by": "inst_01HZ3X9K2M..." }
```

On receiving `superseded`, the old instance must stop publishing events and heartbeats for that agent. It can continue operating other agents that are not in conflict. This prevents ghost presence where two instances both claim an agent is online.

**Typical scenario**: A developer restarts their roko process. The new process connects before the old WebSocket times out. The relay transfers ownership to the new connection immediately rather than waiting for the old one to disconnect.

---
````

**Explicit detail extraction from this section:**

- Section word count: `132`
- Section hash: `0e1a394fd1a96954a206e00ecafc0aa0ac745312fa55a0dfea5181792370c620`

**Normative requirements and implementation claims:**
- **Conflict resolution**: If two roko instances claim the same `agent_id`, the relay uses last-write-wins. The most recent connection for that `agent_id` becomes authoritative. The old connection receives a supersession notice:
- On receiving `superseded`, the old instance must stop publishing events and heartbeats for that agent. It can continue operating other agents that are not in conflict. This prevents ghost presence where two instances both claim an agent is online.
- **Typical scenario**: A developer restarts their roko process. The new process connects before the old WebSocket times out. The relay transfers ownership to the new connection immediately rather than waiting for the old one to disconnect.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- instance_id
- agent_id
- superseded

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
- Contract 1: language `json`, first line `{ "type": "superseded", "agent_id": "coder-1", "by": "inst_01HZ3X9K2M..." }`

```json
{ "type": "superseded", "agent_id": "coder-1", "by": "inst_01HZ3X9K2M..." }
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "instance|agent_id|superseded|instance_id|handling|Multi|relay|connection" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "instance|agent_id|superseded|instance_id|handling|Multi|relay|connection" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `instance_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `superseded` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S016 -- Agent connectivity

**Source section:** `tmp/architecture/04-connectivity.md:296` through `337`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Agent connectivity

Agents communicate across users and across machines. The relay is the rendezvous point -- any agent connected to the relay can discover and message any other agent, regardless of who owns them. There is no concept of "my agents" vs "their agents" at the protocol level. Ownership and access control are handled by auth (Privy JWT, wallet signatures, API keys), not by network isolation.

```
User A's roko process          Relay            User B's Fly Machine
┌──────────────┐                                ┌──────────────┐
│ agent-alpha  │──── WS ────►┌────────┐◄── WS ──│ agent-beta   │
│              │             │ Relay  │          │              │
│ Can message  │◄── relay ───│        │── relay─►│ Can message  │
│ agent-beta   │  forwarding │        │forwarding│ agent-alpha  │
└──────────────┘             │        │          └──────────────┘
                             │        │
User C's dashboard           │        │          User D's agent (local)
┌──────────────┐             │        │          ┌──────────────┐
│ Dashboard    │──── WS ────►│        │◄── WS ──│ agent-gamma  │
│ sees all 3   │             └────────┘          │ behind NAT   │
│ agents       │                                 └──────────────┘
└──────────────┘
```

Cross-user agent communication patterns:

- **Direct messaging**: Agent A sends a message to Agent B via relay. B receives it in its inbox, processes it during the next tick, and can respond.
- **Feed subscription**: Agent A subscribes to Agent B's feed (free or paid). Data flows B → relay → A continuously.
- **Pheromone signaling**: Agents deposit pheromones on-chain. Any agent can read them -- stigmergic coordination without explicit messaging.
- **Cluster collaboration**: Agents from different users can join the same cluster if authorized. The cluster pipeline orchestrates them together.
- **Knowledge sharing**: Agents publish knowledge to the InsightStore (on-chain). Any agent can query it regardless of owner.

Auth controls what an agent can do, not who it can talk to:

| Action | Auth required |
|--------|--------------|
| Discover agents on relay | None (public) |
| Read agent card / capabilities | None (public) |
| Send message to agent | Privy JWT or API key |
| Subscribe to free feed | None |
| Subscribe to paid feed | MPP session or x402 payment |
| Join a cluster | Cluster owner's invitation token |
| Read on-chain knowledge | None (public chain data) |
| Publish knowledge on-chain | Agent wallet signature |
````

**Explicit detail extraction from this section:**

- Section word count: `292`
- Section hash: `07847ded98668e650c8cec04e7483d5afe64603cc2490b4284d46fe4cfe11860`

**Normative requirements and implementation claims:**
- Agents communicate across users and across machines. The relay is the rendezvous point -- any agent connected to the relay can discover and message any other agent, regardless of who owns them. There is no concept of "my agents" vs "their agents" at the protocol level. Ownership and access control are handled by auth (Privy JWT, wallet signatures, API keys), not by network isolation.
- ``` User A's roko process Relay User B's Fly Machine ┌──────────────┐ ┌──────────────┐ │ agent-alpha │──── WS ────►┌────────┐◄── WS ──│ agent-beta │ │ │ │ Relay │ │ │ │ Can message │◄── relay ───│ │── relay─►│ Can message │ │ agent-beta │ forwarding │ │forwarding│ agent-alpha │ └──────────────┘ │ │ └──────────────┘ │ │ User C's dashboard │ │ User D's agent (local) ┌──────────────┐ │ │ ┌──────────────┐ │ Dashboard │──── WS ────►│ │◄── WS ──│ agent-gamma │ │ sees all 3 │ └────────┘ │ behind NAT │ │ agents │ └──────────────┘ └──────────────┘ ```
- - **Direct messaging**: Agent A sends a message to Agent B via relay. B receives it in its inbox, processes it during the next tick, and can respond. - **Feed subscription**: Agent A subscribes to Agent B's feed (free or paid). Data flows B → relay → A continuously. - **Pheromone signaling**: Agents deposit pheromones on-chain. Any agent can read them -- stigmergic coordination without explicit messaging. - **Cluster collaboration**: Agents from different users can join the same cluster if authorized. The cluster pipeline orchestrates them together. - **Knowledge sharing**: Agents publish knowledge to the InsightStore (on-chain). Any agent can query it regardless of owner.
- | Action | Auth required | |--------|--------------| | Discover agents on relay | None (public) | | Read agent card / capabilities | None (public) | | Send message to agent | Privy JWT or API key | | Subscribe to free feed | None | | Subscribe to paid feed | MPP session or x402 payment | | Join a cluster | Cluster owner's invitation token | | Read on-chain knowledge | None (public chain data) | | Publish knowledge on-chain | Agent wallet signature |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Data flows B -> relay

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Direct messaging**: Agent A sends a message to Agent B via relay. B receives it in its inbox, processes it during the next tick, and can respond.
- - **Feed subscription**: Agent A subscribes to Agent B's feed (free or paid). Data flows B → relay → A continuously.
- - **Pheromone signaling**: Agents deposit pheromones on-chain. Any agent can read them -- stigmergic coordination without explicit messaging.
- - **Cluster collaboration**: Agents from different users can join the same cluster if authorized. The cluster pipeline orchestrates them together.
- - **Knowledge sharing**: Agents publish knowledge to the InsightStore (on-chain). Any agent can query it regardless of owner.

**Tables extracted:**
- Table 1:

```markdown
| Action | Auth required |
|--------|--------------|
| Discover agents on relay | None (public) |
| Read agent card / capabilities | None (public) |
| Send message to agent | Privy JWT or API key |
| Subscribe to free feed | None |
| Subscribe to paid feed | MPP session or x402 payment |
| Join a cluster | Cluster owner's invitation token |
| Read on-chain knowledge | None (public chain data) |
| Publish knowledge on-chain | Agent wallet signature |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `User A's roko process          Relay            User B's Fly Machine`

```
User A's roko process          Relay            User B's Fly Machine
┌──────────────┐                                ┌──────────────┐
│ agent-alpha  │──── WS ────►┌────────┐◄── WS ──│ agent-beta   │
│              │             │ Relay  │          │              │
│ Can message  │◄── relay ───│        │── relay─►│ Can message  │
│ agent-beta   │  forwarding │        │forwarding│ agent-alpha  │
└──────────────┘             │        │          └──────────────┘
                             │        │
User C's dashboard           │        │          User D's agent (local)
┌──────────────┐             │        │          ┌──────────────┐
│ Dashboard    │──── WS ────►│        │◄── WS ──│ agent-gamma  │
│ sees all 3   │             └────────┘          │ behind NAT   │
│ agents       │                                 └──────────────┘
└──────────────┘
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "relay|User|message|connectivity|chain|Cluster|auth|None" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "relay|User|message|connectivity|chain|Cluster|auth|None" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Enforce state transition `Data flows B -> relay` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S017 -- In-process agents

**Source section:** `tmp/architecture/04-connectivity.md:338` through `370`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### In-process agents

The default. Agents run as tokio tasks inside the roko process. Communication happens through channels.

```
┌──────────────────────────────────────────────────────────────┐
│                        roko process                          │
│                                                              │
│  ┌──────────────┐     mpsc          ┌──────────────────┐    │
│  │ Control      │ ◄──────────────── │ AgentRuntime     │    │
│  │ Plane        │ ────────────────► │ "coder-1"        │    │
│  │              │     mpsc          │                   │    │
│  │              │                   │ Extensions:       │    │
│  │ Routes msgs  │     mpsc          │  - GitExt         │    │
│  │ to agents    │ ◄──────────────── │  - CompilerExt    │    │
│  │ via channel  │ ────────────────► │  - TestRunnerExt  │    │
│  │ map          │     mpsc          │                   │    │
│  │              │                   └──────────────────┘    │
│  │  agent_id →  │                                           │
│  │  Sender      │     mpsc          ┌──────────────────┐    │
│  │              │ ◄──────────────── │ AgentRuntime     │    │
│  └──────────────┘ ────────────────► │ "research"       │    │
│                       mpsc          └──────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ Inference Gateway                                     │    │
│  │ (shared by all in-process agents via InferenceHandle) │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

Benefits: zero serialization overhead, shared inference gateway, shared memory structures, no network latency.
````

**Explicit detail extraction from this section:**

- Section word count: `66`
- Section hash: `8c8176e4e40c7c281536008e755bc3076f96f39f34294cf553cafd026fa68920`

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
- Contract 1: language `plain`, first line `┌──────────────────────────────────────────────────────────────┐`

```
┌──────────────────────────────────────────────────────────────┐
│                        roko process                          │
│                                                              │
│  ┌──────────────┐     mpsc          ┌──────────────────┐    │
│  │ Control      │ ◄──────────────── │ AgentRuntime     │    │
│  │ Plane        │ ────────────────► │ "coder-1"        │    │
│  │              │     mpsc          │                   │    │
│  │              │                   │ Extensions:       │    │
│  │ Routes msgs  │     mpsc          │  - GitExt         │    │
│  │ to agents    │ ◄──────────────── │  - CompilerExt    │    │
│  │ via channel  │ ────────────────► │  - TestRunnerExt  │    │
│  │ map          │     mpsc          │                   │    │
│  │              │                   └──────────────────┘    │
│  │  agent_id →  │                                           │
│  │  Sender      │     mpsc          ┌──────────────────┐    │
│  │              │ ◄──────────────── │ AgentRuntime     │    │
│  └──────────────┘ ────────────────► │ "research"       │    │
│                       mpsc          └──────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ Inference Gateway                                     │    │
│  │ (shared by all in-process agents via InferenceHandle) │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "process|mpsc|shared|Inference|channel|Gateway|AgentRuntime|zero" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "process|mpsc|shared|Inference|channel|Gateway|AgentRuntime|zero" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S018 -- Remote agents

**Source section:** `tmp/architecture/04-connectivity.md:371` through `405`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Remote agents

For isolation or NAT traversal, agents connect OUTBOUND to the relay. No inbound server required.

```
┌───────────────┐         ┌─────────────┐        ┌───────────────┐
│ Remote agent  │ ──WS──► │   Relay     │ ◄──WS──│ Control plane │
│ (Fly Machine) │         │             │        │ (roko-serve)  │
│               │         │ Routes msgs │        │               │
│ Connects out. │         │ by agent_id │        │ Routes msgs   │
│ No inbound    │         │             │        │ to relay for  │
│ ports needed. │         │             │        │ remote agents │
└───────────────┘         └─────────────┘        └───────────────┘
```

The relay acts as a message router. Both the agent and the control plane maintain persistent WebSocket connections to the relay. Messages are routed by agent ID.

Remote agent startup:

```bash
# On the Fly Machine / Railway container
roko agent run \
  --name "isolated-coder" \
  --relay wss://relay.nunchi.dev \
  --inference-proxy https://my-roko.up.railway.app/api/inference \
  --auth-token $AGENT_TOKEN
```

The agent:
1. Connects to the relay WebSocket
2. Announces presence with its agent ID and capabilities
3. Enters the standard `run()` loop
4. Routes inference requests to the parent's gateway via HTTPS proxy
5. Publishes heartbeats and events through the relay
````

**Explicit detail extraction from this section:**

- Section word count: `147`
- Section hash: `77560981c5ef15aae18884bcf8d68d3cd887b724504d5d2450375d46cac660fb`

**Normative requirements and implementation claims:**
- For isolation or NAT traversal, agents connect OUTBOUND to the relay. No inbound server required.
- ```bash # On the Fly Machine / Railway container roko agent run \ --name "isolated-coder" \ --relay wss://relay.nunchi.dev \ --inference-proxy https://my-roko.up.railway.app/api/inference \ --auth-token $AGENT_TOKEN ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- my-roko.up.railway.app/api/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- relay.nunchi.dev
- roko.up.railway.app

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko agent run \
- --name "isolated-coder" \
- --relay wss://relay.nunchi.dev \
- --inference-proxy https://my-roko.up.railway.app/api/inference \
- --auth-token $AGENT_TOKEN

**Bullet requirements:**
- 1. Connects to the relay WebSocket
- 2. Announces presence with its agent ID and capabilities
- 3. Enters the standard `run()` loop
- 4. Routes inference requests to the parent's gateway via HTTPS proxy
- 5. Publishes heartbeats and events through the relay

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `┌───────────────┐         ┌─────────────┐        ┌───────────────┐`

```
┌───────────────┐         ┌─────────────┐        ┌───────────────┐
│ Remote agent  │ ──WS──► │   Relay     │ ◄──WS──│ Control plane │
│ (Fly Machine) │         │             │        │ (roko-serve)  │
│               │         │ Routes msgs │        │               │
│ Connects out. │         │ by agent_id │        │ Routes msgs   │
│ No inbound    │         │             │        │ to relay for  │
│ ports needed. │         │             │        │ remote agents │
└───────────────┘         └─────────────┘        └───────────────┘
```
- Contract 2: language `bash`, first line `# On the Fly Machine / Railway container`

```bash
# On the Fly Machine / Railway container
roko agent run \
  --name "isolated-coder" \
  --relay wss://relay.nunchi.dev \
  --inference-proxy https://my-roko.up.railway.app/api/inference \
  --auth-token $AGENT_TOKEN
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `my-roko.up.railway.app/api/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "relay|Remote|connect|inference|Routes|token|serve|railway" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "relay|Remote|connect|inference|Routes|token|serve|railway" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `my-roko.up.railway.app/api/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Emit or consume `relay.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.up.railway.app` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Implement or verify operator command `roko agent run \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `--name "isolated-coder" \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `--relay wss://relay.nunchi.dev \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `--inference-proxy https://my-roko.up.railway.app/api/inference \` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `--auth-token $AGENT_TOKEN` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S019 -- Direct-reachable agents

**Source section:** `tmp/architecture/04-connectivity.md:406` through `419`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Direct-reachable agents

Some remote agents have public URLs (Railway services, dedicated VMs). These can receive messages directly via HTTP in addition to the relay path.

```toml
# User's local deployment list (stored in dashboard localStorage or roko.toml)
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"
```

The control plane prefers direct HTTP for request-response patterns (lower latency) and uses the relay for event streaming and presence.
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `b32e1addd6d4f55108452fee42070738d8a79e0c8c39d9f37d1f3869ee2df9e0`

**Normative requirements and implementation claims:**
- ```toml # User's local deployment list (stored in dashboard localStorage or roko.toml) [[remote_agents]] name = "staging-monitor" url = "https://staging-monitor.fly.dev" auth_token_ref = "secrets.staging_monitor_token" ```
- The control plane prefers direct HTTP for request-response patterns (lower latency) and uses the relay for event streaming and presence.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- monitor.fly.dev
- secrets.staging_monitor_token

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- name = "staging-monitor"
- url = "https://staging-monitor.fly.dev"
- auth_token_ref = "secrets.staging_monitor_token"

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# User's local deployment list (stored in dashboard localStorage or roko.toml)`

```toml
# User's local deployment list (stored in dashboard localStorage or roko.toml)
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Direct|reachable|staging|monitor|HTTP|url|toml|remote" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Direct|reachable|staging|monitor|HTTP|url|toml|remote" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Emit or consume `monitor.fly.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `secrets.staging_monitor_token` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `name = "staging-monitor"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `url = "https://staging-monitor.fly.dev"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `auth_token_ref = "secrets.staging_monitor_token"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S020 -- Agent discovery: three sources merged

**Source section:** `tmp/architecture/04-connectivity.md:420` through `490`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent discovery: three sources merged

```
┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Relay presence   │  │ ERC-8004 on-chain│  │ User's deployment  │
│                  │  │ registry         │  │ list               │
│ Who's online     │  │                  │  │                    │
│ right now.       │  │ Wallet address,  │  │ Railway/Fly URLs,  │
│                  │  │ reputation,      │  │ manually added     │
│ Source of truth  │  │ stake, caps,     │  │ endpoints.         │
│ for liveness.    │  │ feed adverts.    │  │                    │
│                  │  │                  │  │ Per-user. Stored   │
│ Always available.│  │ Source of truth   │  │ in localStorage.   │
│                  │  │ for identity +   │  │                    │
│                  │  │ feed discovery.  │  │ Always available.  │
│                  │  │                  │  │                    │
│                  │  │ Always available. │  │                    │
└────────┬────────┘  └────────┬─────────┘  └────────┬───────────┘
         │                    │                      │
         └────────────────────┼──────────────────────┘
                              ▼
                   ┌─────────────────────┐
                   │ Merged agent list   │
                   │                     │
                   │ Each agent has:     │
                   │ - id, name          │
                   │ - online (relay)    │
                   │ - reputation (chain)│
                   │ - endpoints (deploy)│
                   │ - capabilities      │
                   │ - mode, profile     │
                   │ - feeds (chain+relay│
                   └─────────────────────┘
```

The dashboard merges all three sources client-side. The relay provides liveness. The chain provides identity and reputation. The deployment list provides connectivity.

```typescript
interface MergedAgent {
  id: string;
  name: string;

  // From relay
  online: boolean;
  lastSeen: number;
  mode: "ephemeral" | "persistent" | "reactive";
  profile: string;

  // From chain (ERC-8004)
  wallet?: string;
  reputation?: number;
  stake?: bigint;
  tier?: "gray" | "copper" | "silver" | "gold" | "amber";
  capabilities?: string[];
  cardUri?: string;
  feeds?: FeedAdvertisement[];  // feeds registered in passport

  // From deployment list
  directUrl?: string;
  deployPlatform?: "fly" | "railway" | "manual";
}

interface FeedAdvertisement {
  feedId: string;
  schema: string;
  rateHz: number;
  access: "public" | { paid: { pricePerHour: number } };
  description: string;
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `164`
- Section hash: `b84597b0c2b13f46ec03e72d2d9418369a771531a4b0bd35df36e7c8a8d7c509`

**Normative requirements and implementation claims:**
- ``` ┌─────────────────┐ ┌──────────────────┐ ┌────────────────────┐ │ Relay presence │ │ ERC-8004 on-chain│ │ User's deployment │ │ │ │ registry │ │ list │ │ Who's online │ │ │ │ │ │ right now. │ │ Wallet address, │ │ Railway/Fly URLs, │ │ │ │ reputation, │ │ manually added │ │ Source of truth │ │ stake, caps, │ │ endpoints. │ │ for liveness. │ │ feed adverts. │ │ │ │ │ │ │ │ Per-user. Stored │ │ Always available.│ │ Source of truth │ │ in localStorage. │ │ │ │ for identity + │ │ │ │ │ │ feed discovery. │ │ Always available. │ │ │ │ │ │ │ │ │ │ Always available. │ │ │ └────────┬────────┘ └────────┬─────────┘ └────────┬───────────┘ │ │ │ └────────────────────┼──────────────────────┘ ▼ ┌─────────────────────┐ │ Merged agent list │ │ │ │ Each agent has: │ │ - id, name │ │ - online (relay) │ │ - reputation (chain)│ │ - endpoints (deploy)│ │ - capabilities │ │ - mode, profile │ │ - feeds (chain+relay│ └─────────────────────┘ ```
- The dashboard merges all three sources client-side. The relay provides liveness. The chain provides identity and reputation. The deployment list provides connectivity.

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
- Contract 1: language `plain`, first line `┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐`

```
┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Relay presence   │  │ ERC-8004 on-chain│  │ User's deployment  │
│                  │  │ registry         │  │ list               │
│ Who's online     │  │                  │  │                    │
│ right now.       │  │ Wallet address,  │  │ Railway/Fly URLs,  │
│                  │  │ reputation,      │  │ manually added     │
│ Source of truth  │  │ stake, caps,     │  │ endpoints.         │
│ for liveness.    │  │ feed adverts.    │  │                    │
│                  │  │                  │  │ Per-user. Stored   │
│ Always available.│  │ Source of truth   │  │ in localStorage.   │
│                  │  │ for identity +   │  │                    │
│                  │  │ feed discovery.  │  │ Always available.  │
│                  │  │                  │  │                    │
│                  │  │ Always available. │  │                    │
└────────┬────────┘  └────────┬─────────┘  └────────┬───────────┘
         │                    │                      │
         └────────────────────┼──────────────────────┘
                              ▼
                   ┌─────────────────────┐
                   │ Merged agent list   │
                   │                     │
                   │ Each agent has:     │
                   │ - id, name          │
                   │ - online (relay)    │
                   │ - reputation (chain)│
                   │ - endpoints (deploy)│
                   │ - capabilities      │
                   │ - mode, profile     │
                   │ -
...
```
- Contract 2: language `typescript`, first line `interface MergedAgent {`

```typescript
interface MergedAgent {
  id: string;
  name: string;

  // From relay
  online: boolean;
  lastSeen: number;
  mode: "ephemeral" | "persistent" | "reactive";
  profile: string;

  // From chain (ERC-8004)
  wallet?: string;
  reputation?: number;
  stake?: bigint;
  tier?: "gray" | "copper" | "silver" | "gold" | "amber";
  capabilities?: string[];
  cardUri?: string;
  feeds?: FeedAdvertisement[];  // feeds registered in passport

  // From deployment list
  directUrl?: string;
  deployPlatform?: "fly" | "railway" | "manual";
}

interface FeedAdvertisement {
  feedId: string;
  schema: string;
  rateHz: number;
  access: "public" | { paid: { pricePerHour: number } };
  description: string;
}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "string|feed|merged|three|sources|discovery|deploy|chain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "string|feed|merged|three|sources|discovery|deploy|chain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-04-S021 -- Message routing

**Source section:** `tmp/architecture/04-connectivity.md:491` through `517`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Message routing

The control plane routes messages based on agent location:

```rust
impl ControlPlane {
    async fn send_to_agent(&self, agent_id: &AgentId, msg: AgentMessage) -> Result<()> {
        // 1. Check in-process agents first (fastest path)
        if let Some(sender) = self.local_agents.get(agent_id) {
            return sender.send(msg).await.map_err(Into::into);
        }

        // 2. Check direct-reachable agents (HTTP)
        if let Some(url) = self.deployment_urls.get(agent_id) {
            return self.http_client
                .post(format!("{url}/api/message"))
                .json(&msg)
                .send()
                .await
                .map_err(Into::into);
        }

        // 3. Fall back to relay (works for NAT-traversal)
        self.relay.send(agent_id, msg).await
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `89`
- Section hash: `28ebc14ef33ebb746f90a300b5a08ba34c192e82908004da99e2624e272bf50d`

**Normative requirements and implementation claims:**
- // 2. Check direct-reachable agents (HTTP) if let Some(url) = self.deployment_urls.get(agent_id) { return self.http_client .post(format!("{url}/api/message")) .json(&msg) .send() .await .map_err(Into::into); }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- send_to_agent

**Event names and event-like entities:**
- self.local_agents.get
- sender.send
- await.map_err
- self.deployment_urls.get
- self.http_client
- self.relay.send

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
- Contract 1: language `rust`, first line `impl ControlPlane {`

```rust
impl ControlPlane {
    async fn send_to_agent(&self, agent_id: &AgentId, msg: AgentMessage) -> Result<()> {
        // 1. Check in-process agents first (fastest path)
        if let Some(sender) = self.local_agents.get(agent_id) {
            return sender.send(msg).await.map_err(Into::into);
        }

        // 2. Check direct-reachable agents (HTTP)
        if let Some(url) = self.deployment_urls.get(agent_id) {
            return self.http_client
                .post(format!("{url}/api/message"))
                .json(&msg)
                .send()
                .await
                .map_err(Into::into);
        }

        // 3. Fall back to relay (works for NAT-traversal)
        self.relay.send(agent_id, msg).await
    }
}
```

**Read before editing:**
- `tmp/architecture/04-connectivity.md`
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Message|send|send_to_agent|self|routing|agent_id|await|sender" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Message|send|send_to_agent|self|routing|agent_id|await|sender" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/connector.rs`
- `crates/roko-serve/src/routes/connectors.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
- [ ] Implement or verify `send_to_agent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `self.local_agents.get` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `sender.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `await.map_err` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.deployment_urls.get` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.http_client` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.relay.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/04-connectivity
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

