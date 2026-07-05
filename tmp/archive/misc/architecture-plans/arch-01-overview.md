# Architecture Plan: Overview

**Source:** `tmp/architecture/01-overview.md`
**Generated:** 2026-04-25
**Source hash:** `abd87f82c6cda5fd32a15ce3f95263997c52059e859e604d6f435e95cf325a45`
**Section tasks:** 3
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
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-01-S001 | 1 | Overview and problem | [ ] | 9.8 |
| ARCH-01-S002 | 7 | The problem | [ ] | 9.8 |
| ARCH-01-S003 | 29 | Architecture overview | [ ] | 9.8 |

## Tasks

### ARCH-01-S001 -- Overview and problem

**Source section:** `tmp/architecture/01-overview.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Overview and problem

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---
````

**Explicit detail extraction from this section:**

- Section word count: `14`
- Section hash: `7fbae02307bdd896122b1e6ff701a10efb7424281fddd678a0795dc02af2f746`

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
- `tmp/architecture/01-overview.md`
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "problem|Overview|redesign|Specification|Part|INDEX|Extracted" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "problem|Overview|redesign|Specification|Part|INDEX|Extracted" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/01-overview
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-01-S002 -- The problem

**Source section:** `tmp/architecture/01-overview.md:7` through `28`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The problem

Roko has four infrastructure components that evolved independently and never agreed on boundaries:

1. **Mirage** -- a devnet chain with a relay WebSocket. Always on, shared across users.
2. **roko-serve** -- an HTTP control plane with ~85 routes. Requires a workspace directory. Optional for users who only want agents.
3. **roko-agent-server** -- a per-agent HTTP sidecar (13 routes). One process per agent. Breaks behind NAT.
4. **Dashboard / TUI** -- consumes REST endpoints from all three. Falls over when any backend is unreachable.

This creates several concrete failures:

- **Per-agent sidecars don't traverse NAT.** An agent on a Fly Machine can't expose an HTTP server that the control plane can reach without proxy configuration. The sidecar model assumes a flat network.
- **Dashboard requires roko-serve.** If the control plane is down, the dashboard shows "Backend offline" even though agents may still be running and the relay still has presence data.
- **Polling everywhere.** The dashboard polls multiple endpoints on 1-5 second intervals. This wastes bandwidth, creates visual jitter, and scales poorly with agent count.
- **API keys scattered.** Each agent holds its own LLM API keys via environment variables. No central audit, no rotation, no cost attribution.
- **No agent lifecycle.** Agents are either ephemeral CLI processes or stateless HTTP workers. No heartbeat, no mode (persistent vs reactive), no graceful shutdown protocol.
- **Three discovery sources, zero merge.** Relay presence, ERC-8004 on-chain registry, and manually-added deployment URLs each live in separate UIs. No unified agent list.

This document specifies the architecture that resolves all six.

---
````

**Explicit detail extraction from this section:**

- Section word count: `256`
- Section hash: `d822ce59a894c33e6a340528f883409bd81d41e7825df7151297e1c766091f25`

**Normative requirements and implementation claims:**
- Roko has four infrastructure components that evolved independently and never agreed on boundaries:
- 1. **Mirage** -- a devnet chain with a relay WebSocket. Always on, shared across users. 2. **roko-serve** -- an HTTP control plane with ~85 routes. Requires a workspace directory. Optional for users who only want agents. 3. **roko-agent-server** -- a per-agent HTTP sidecar (13 routes). One process per agent. Breaks behind NAT. 4. **Dashboard / TUI** -- consumes REST endpoints from all three. Falls over when any backend is unreachable.
- - **Per-agent sidecars don't traverse NAT.** An agent on a Fly Machine can't expose an HTTP server that the control plane can reach without proxy configuration. The sidecar model assumes a flat network. - **Dashboard requires roko-serve.** If the control plane is down, the dashboard shows "Backend offline" even though agents may still be running and the relay still has presence data. - **Polling everywhere.** The dashboard polls multiple endpoints on 1-5 second intervals. This wastes bandwidth, creates visual jitter, and scales poorly with agent count. - **API keys scattered.** Each agent holds its own LLM API keys via environment variables. No central audit, no rotation, no cost attribution. - **No agent lifecycle.** Agents are either ephemeral CLI processes or stateless HTTP workers. No heartbeat, no mode (persistent vs reactive), no graceful shutdown protocol. - **Three discovery sources, zero merge.** Relay presence, ERC-8004 on-chain registry, and manually-added deployment URLs ea
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
- 1. **Mirage** -- a devnet chain with a relay WebSocket. Always on, shared across users.
- 2. **roko-serve** -- an HTTP control plane with ~85 routes. Requires a workspace directory. Optional for users who only want agents.
- 3. **roko-agent-server** -- a per-agent HTTP sidecar (13 routes). One process per agent. Breaks behind NAT.
- 4. **Dashboard / TUI** -- consumes REST endpoints from all three. Falls over when any backend is unreachable.
- - **Per-agent sidecars don't traverse NAT.** An agent on a Fly Machine can't expose an HTTP server that the control plane can reach without proxy configuration. The sidecar model assumes a flat network.
- - **Dashboard requires roko-serve.** If the control plane is down, the dashboard shows "Backend offline" even though agents may still be running and the relay still has presence data.
- - **Polling everywhere.** The dashboard polls multiple endpoints on 1-5 second intervals. This wastes bandwidth, creates visual jitter, and scales poorly with agent count.
- - **API keys scattered.** Each agent holds its own LLM API keys via environment variables. No central audit, no rotation, no cost attribution.
- - **No agent lifecycle.** Agents are either ephemeral CLI processes or stateless HTTP workers. No heartbeat, no mode (persistent vs reactive), no graceful shutdown protocol.
- - **Three discovery sources, zero merge.** Relay presence, ERC-8004 on-chain registry, and manually-added deployment URLs each live in separate UIs. No unified agent list.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/01-overview.md`
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "The|problem|serve|HTTP|sidecar|relay|plane|control" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|problem|serve|HTTP|sidecar|relay|plane|control" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/01-overview
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-01-S003 -- Architecture overview

**Source section:** `tmp/architecture/01-overview.md:29` through `75`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Architecture overview

```
                         ┌─────────────────────────┐
                         │   Mirage chain + Relay   │  Always on. Shared.
                         │   (mirage-devnet.fly.dev)│
                         │                          │
                         │  Chain: blocks, events,  │
                         │         ERC-8004 registry │
                         │  Relay: agent presence,  │
                         │         WS event routing  │
                         └────────┬─────────────────┘
                                  │ WebSocket
             ┌────────────────────┼─────────────────────────┐
             │                    │                          │
             ▼                    ▼                          ▼
  ┌──────────────────┐  ┌─────────────────────┐   ┌──────────────────┐
  │    Dashboard     │  │    roko process      │   │  Remote agent    │
  │   (web / TUI)    │  │   (optional)         │   │  (Fly / Railway) │
  │                  │  │                      │   │                  │
  │ Connects to:     │  │  ┌───────────────┐   │   │  Connects        │
  │ - Relay (always) │  │  │ Control plane │   │   │  OUTBOUND to     │
  │ - roko (if avail)│  │  │ (roko-serve)  │   │   │  relay via WS    │
  │ - Agent feeds    │  │  ├───────────────┤   │   │                  │
  │                  │  │  │ Agent runtime │   │   │  Gets inference  │
  │ Subscribes to WS │  │  │ (tokio tasks) │   │   │  via parent      │
  │ per page. No     │  │  ├───────────────┤   │   │  gateway proxy   │
  │ polling.         │  │  │ Inference     │   │   │                  │
  └──────────────────┘  │  │ Gateway       │   │   └──────────────────┘
                        │  └───────────────┘   │
                        │                      │
                        │  In-process agents:   │
                        │  ┌─────┐ ┌─────┐     │
                        │  │ A1  │ │ A2  │ ... │
                        │  └─────┘ └─────┘     │
                        └──────────────────────┘
```

Three deployment tiers:

| Tier | What runs | Who needs it |
|------|-----------|--------------|
| **Backbone** | Mirage chain + relay | Everyone. Always on. Shared infrastructure. |
| **Workspace** | roko process (control plane + agent runtime + inference gateway) | Users who want orchestration, plans, PRDs, learning. |
| **Remote agents** | Standalone processes on Fly/Railway | Users who need isolation or scale. |

The backbone is the only hard dependency. Everything else is additive.
````

**Explicit detail extraction from this section:**

- Section word count: `134`
- Section hash: `0c86263e88cd621ca58acd5d87b0b20f24330b2ac1f44381978666fffb02b52b`

**Normative requirements and implementation claims:**
- ``` ┌─────────────────────────┐ │ Mirage chain + Relay │ Always on. Shared. │ (mirage-devnet.fly.dev)│ │ │ │ Chain: blocks, events, │ │ ERC-8004 registry │ │ Relay: agent presence, │ │ WS event routing │ └────────┬─────────────────┘ │ WebSocket ┌────────────────────┼─────────────────────────┐ │ │ │ ▼ ▼ ▼ ┌──────────────────┐ ┌─────────────────────┐ ┌──────────────────┐ │ Dashboard │ │ roko process │ │ Remote agent │ │ (web / TUI) │ │ (optional) │ │ (Fly / Railway) │ │ │ │ │ │ │ │ Connects to: │ │ ┌───────────────┐ │ │ Connects │ │ - Relay (always) │ │ │ Control plane │ │ │ OUTBOUND to │ │ - roko (if avail)│ │ │ (roko-serve) │ │ │ relay via WS │ │ - Agent feeds │ │ ├───────────────┤ │ │ │ │ │ │ │ Agent runtime │ │ │ Gets inference │ │ Subscribes to WS │ │ │ (tokio tasks) │ │ │ via parent │ │ per page. No │ │ ├───────────────┤ │ │ gateway proxy │ │ polling. │ │ │ Inference │ │ │ │ └──────────────────┘ │ │ Gateway │ │ └──────────────────┘ │ └───────────────┘ │ │ │ │ In-process agents: │ │
- | Tier | What runs | Who needs it | |------|-----------|--------------| | **Backbone** | Mirage chain + relay | Everyone. Always on. Shared infrastructure. | | **Workspace** | roko process (control plane + agent runtime + inference gateway) | Users who want orchestration, plans, PRDs, learning. | | **Remote agents** | Standalone processes on Fly/Railway | Users who need isolation or scale. |
- The backbone is the only hard dependency. Everything else is additive.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- devnet.fly.dev

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
| Tier | What runs | Who needs it |
|------|-----------|--------------|
| **Backbone** | Mirage chain + relay | Everyone. Always on. Shared infrastructure. |
| **Workspace** | roko process (control plane + agent runtime + inference gateway) | Users who want orchestration, plans, PRDs, learning. |
| **Remote agents** | Standalone processes on Fly/Railway | Users who need isolation or scale. |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `┌─────────────────────────┐`

```
┌─────────────────────────┐
                         │   Mirage chain + Relay   │  Always on. Shared.
                         │   (mirage-devnet.fly.dev)│
                         │                          │
                         │  Chain: blocks, events,  │
                         │         ERC-8004 registry │
                         │  Relay: agent presence,  │
                         │         WS event routing  │
                         └────────┬─────────────────┘
                                  │ WebSocket
             ┌────────────────────┼─────────────────────────┐
             │                    │                          │
             ▼                    ▼                          ▼
  ┌──────────────────┐  ┌─────────────────────┐   ┌──────────────────┐
  │    Dashboard     │  │    roko process      │   │  Remote agent    │
  │   (web / TUI)    │  │   (optional)         │   │  (Fly / Railway) │
  │                  │  │                      │   │                  │
  │ Connects to:     │  │  ┌───────────────┐   │   │  Connects        │
  │ - Relay (always) │  │  │ Control plane │   │   │  OUTBOUND to     │
  │ - roko (if avail)│  │  │ (roko-serve)  │   │   │  relay via WS    │
  │ - Agent feeds    │  │  ├───────────────┤   │   │                  │
  │                  │  │  │ Agent runtime │   │   │  Gets inference  │
  │ Subscribes to WS │  │  │ (tokio tasks) │   │   │  via parent      │
  │ per page. No     │  │  ├───────────────┤   │   │  gateway proxy   │
  │ polling.         │  │  │ Inference     │   │   │                  │
  └──────────────────
...
```

**Read before editing:**
- `tmp/architecture/01-overview.md`
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
rg -n "overview|Relay|process|inference|gateway|fly|chain|Mirage" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "overview|Relay|process|inference|gateway|fly|chain|Mirage" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/parity.rs`
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
- [ ] Emit or consume `devnet.fly.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/01-overview
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

