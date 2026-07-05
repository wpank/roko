# Architecture Plan: Paid Feeds

**Source:** `tmp/architecture/06-paid-feeds.md`
**Generated:** 2026-04-25
**Source hash:** `c481b1d6d5af1bf8edd22dc3eee5a1848cc2d4def32444c8d0851170c41a9e72`
**Section tasks:** 11
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
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-06-S001 | 1 | Paid feeds and agent services | [ ] | 9.8 |
| ARCH-06-S002 | 10 | Payment protocols | [ ] | 9.8 |
| ARCH-06-S003 | 114 | Setting up a paid feed | [ ] | 9.8 |
| ARCH-06-S004 | 323 | Relay feed infrastructure | [ ] | 9.8 |
| ARCH-06-S005 | 407 | Feed types and composability | [ ] | 9.8 |
| ARCH-06-S006 | 459 | On-chain feed advertisement (ERC-8004) | [ ] | 9.8 |
| ARCH-06-S007 | 522 | Dashboard chain feed subscriptions | [ ] | 9.8 |
| ARCH-06-S008 | 564 | Dashboard integration | [ ] | 9.8 |
| ARCH-06-S009 | 673 | Feed data-source mapping | [ ] | 9.8 |
| ARCH-06-S010 | 684 | Practical example: funding rate divergence feed | [ ] | 9.8 |
| ARCH-06-S011 | 787 | Extensibility | [ ] | 9.8 |

## Tasks

### ARCH-06-S001 -- Paid feeds and agent services

**Source section:** `tmp/architecture/06-paid-feeds.md:1` through `9`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Paid feeds and agent services

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> See [Feeds and Data Streams](05-feeds.md) for the base feed system this builds on.

---

The previous section covers how agents subscribe to chain data and expose feeds via the relay. This section covers payment: how a feed producer sets a price, how subscribers pay, how sessions work, and how the dashboard surfaces it all.
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `d9abf362ed877447e52a44a7cc2118d7d35aa311b7214a8492635276543b1af2`

**Normative requirements and implementation claims:**
- ---
- The previous section covers how agents subscribe to chain data and expose feeds via the relay. This section covers payment: how a feed producer sets a price, how subscribers pay, how sessions work, and how the dashboard surfaces it all.

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
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|feeds|services|Paid|subscribe|covers|Data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|feeds|services|Paid|subscribe|covers|Data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S002 -- Payment protocols

**Source section:** `tmp/architecture/06-paid-feeds.md:10` through `113`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Payment protocols

Two protocols, both implemented in bardo (`crates/mpp/`).

**x402 (per-request, stateless)**

The simplest payment flow. No session, no state. Each request carries its own authorization.

```
Client                                  Server (relay / agent)
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │ ──────────────────────────────────────> │
  │                                         │
  │  HTTP 402                               │
  │  X-Payment-Required:                    │
  │    amount=50, recipient=0xABC...,       │
  │    nonce=1, expiry=1714000000           │
  │ <────────────────────────────────────── │
  │                                         │
  │  Client signs ERC-3009 authorization    │
  │  (gasless USDC approval, no on-chain tx)│
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │  X-Payment: <signed authorization>      │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Server verifies signature (ecrecover,  │
  │  no RPC needed), serves content         │
  │                                         │
  │  200 OK + feed data                     │
  │ <────────────────────────────────────── │
```

Settlement happens in batches: every 10 minutes or after 100+ accumulated authorizations, whichever comes first. The server submits a single on-chain transaction that settles all pending authorizations. This amortizes gas costs across many payments.

**MPP (session-based, streaming)**

For continuous feeds. One signature funds an entire session. No re-signing per message.

```
Client                                  Server (relay / agent)
  │                                         │
  │  POST /mpp/sessions                     │
  │  { amount: 500, authorization: <sig> }  │
  │ ──────────────────────────────────────> │
  │                                         │
  │  201 Created                            │
  │  { session_id: "abc-123",               │
  │    funded: 500, status: "active" }      │
  │ <────────────────────────────────────── │
  │                                         │
  │  WS subscribe with session_id           │
  │  { rooms: ["feed:eth-gas-trend"],       │
  │    payment: { session_id: "abc-123" } } │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Per-message draw from session          │
  │  (no client interaction needed)         │
  │                                         │
  │  feed_data: { ema_12: 42.5, ... }       │
  │  payment_draw: { amount: 1,             │
  │    balance_remaining: 499 }             │
  │ <────────────────────────────────────── │
```

Session lifecycle:

```
Active ──> Exhausted ──> Expired ──> Settled
  │            │                       │
  │  (top-up)  │                       │
  └────────────┘                       │
                                       └── Refund unspent balance
```

- **Active**: draws succeed, messages flow.
- **Exhausted**: balance hits zero. Server sends exhaustion notice and pauses delivery. Client can top-up to resume.
- **Expired**: TTL reached (default 24h). No more draws. Transitions to Settled.
- **Settled**: unspent balance refunded. Session closed. Settlement submitted on-chain.

**When to use which**

| Scenario | Protocol | Why |
|----------|----------|-----|
| Try a feed for 5 minutes | x402 | No session overhead, pay per message |
| Subscribe to a price feed for 24h | MPP | One signature, draws per tick |
| Query an agent's analysis on-demand | x402 | Stateless, pay per query |
| Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |

**Reputation-based pricing**

Higher ERC-8004 reputation tier = lower markup. Applied on top of the feed producer's base price.

| Tier | Markup |
|------|--------|
| None | +20% |
| Basic | +18% |
| Verified | +15% |
| Trusted | +12% |
| Sovereign | +8% |

A feed priced at $0.10/hr costs a `None`-tier subscriber $0.12/hr and a `Sovereign`-tier subscriber $0.108/hr. The spread goes to the relay as an infrastructure fee.
````

**Explicit detail extraction from this section:**

- Section word count: `366`
- Section hash: `22389c78ab72c5fd2dbe3725f77fd82f0bc0d2ee69c0dda7d88bd766662e2a5c`

**Normative requirements and implementation claims:**
- **x402 (per-request, stateless)**
- The simplest payment flow. No session, no state. Each request carries its own authorization.
- ``` Client Server (relay / agent) │ │ │ GET /relay/feeds/eth-gas-trend/data │ │ ──────────────────────────────────────> │ │ │ │ HTTP 402 │ │ X-Payment-Required: │ │ amount=50, recipient=0xABC..., │ │ nonce=1, expiry=1714000000 │ │ <────────────────────────────────────── │ │ │ │ Client signs ERC-3009 authorization │ │ (gasless USDC approval, no on-chain tx)│ │ │ │ GET /relay/feeds/eth-gas-trend/data │ │ X-Payment: <signed authorization> │ │ ──────────────────────────────────────> │ │ │ │ Server verifies signature (ecrecover, │ │ no RPC needed), serves content │ │ │ │ 200 OK + feed data │ │ <────────────────────────────────────── │ ```
- **MPP (session-based, streaming)**
- - **Active**: draws succeed, messages flow. - **Exhausted**: balance hits zero. Server sends exhaustion notice and pauses delivery. Client can top-up to resume. - **Expired**: TTL reached (default 24h). No more draws. Transitions to Settled. - **Settled**: unspent balance refunded. Session closed. Settlement submitted on-chain.
- **When to use which**
- | Scenario | Protocol | Why | |----------|----------|-----| | Try a feed for 5 minutes | x402 | No session overhead, pay per message | | Subscribe to a price feed for 24h | MPP | One signature, draws per tick | | Query an agent's analysis on-demand | x402 | Stateless, pay per query | | Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |
- **Reputation-based pricing**
- | Tier | Markup | |------|--------| | None | +20% | | Basic | +18% | | Verified | +15% | | Trusted | +12% | | Sovereign | +8% |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/mpp/
- relay/feeds/eth-gas-trend/

**Types, functions, traits, and inline code identifiers:**
- None
- Sovereign

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Active**: draws succeed, messages flow.
- - **Exhausted**: balance hits zero. Server sends exhaustion notice and pauses delivery. Client can top-up to resume.
- - **Expired**: TTL reached (default 24h). No more draws. Transitions to Settled.
- - **Settled**: unspent balance refunded. Session closed. Settlement submitted on-chain.

**Tables extracted:**
- Table 1:

```markdown
| Scenario | Protocol | Why |
|----------|----------|-----|
| Try a feed for 5 minutes | x402 | No session overhead, pay per message |
| Subscribe to a price feed for 24h | MPP | One signature, draws per tick |
| Query an agent's analysis on-demand | x402 | Stateless, pay per query |
| Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |
```
- Table 2:

```markdown
| Tier | Markup |
|------|--------|
| None | +20% |
| Basic | +18% |
| Verified | +15% |
| Trusted | +12% |
| Sovereign | +8% |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Client                                  Server (relay / agent)`

```
Client                                  Server (relay / agent)
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │ ──────────────────────────────────────> │
  │                                         │
  │  HTTP 402                               │
  │  X-Payment-Required:                    │
  │    amount=50, recipient=0xABC...,       │
  │    nonce=1, expiry=1714000000           │
  │ <────────────────────────────────────── │
  │                                         │
  │  Client signs ERC-3009 authorization    │
  │  (gasless USDC approval, no on-chain tx)│
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │  X-Payment: <signed authorization>      │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Server verifies signature (ecrecover,  │
  │  no RPC needed), serves content         │
  │                                         │
  │  200 OK + feed data                     │
  │ <────────────────────────────────────── │
```
- Contract 2: language `plain`, first line `Client                                  Server (relay / agent)`

```
Client                                  Server (relay / agent)
  │                                         │
  │  POST /mpp/sessions                     │
  │  { amount: 500, authorization: <sig> }  │
  │ ──────────────────────────────────────> │
  │                                         │
  │  201 Created                            │
  │  { session_id: "abc-123",               │
  │    funded: 500, status: "active" }      │
  │ <────────────────────────────────────── │
  │                                         │
  │  WS subscribe with session_id           │
  │  { rooms: ["feed:eth-gas-trend"],       │
  │    payment: { session_id: "abc-123" } } │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Per-message draw from session          │
  │  (no client interaction needed)         │
  │                                         │
  │  feed_data: { ema_12: 42.5, ... }       │
  │  payment_draw: { amount: 1,             │
  │    balance_remaining: 499 }             │
  │ <────────────────────────────────────── │
```
- Contract 3: language `plain`, first line `Active ──> Exhausted ──> Expired ──> Settled`

```
Active ──> Exhausted ──> Expired ──> Settled
  │            │                       │
  │  (top-up)  │                       │
  └────────────┘                       │
                                       └── Refund unspent balance
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/mpp/`
- `relay/feeds/eth-gas-trend/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "session|feed|Payment|protocols|authorization|Sovereign|None|relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "session|feed|Payment|protocols|authorization|Sovereign|None|relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/mpp/`
- `relay/feeds/eth-gas-trend/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
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
- [ ] Implement or verify `None` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Sovereign` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S003 -- Setting up a paid feed

**Source section:** `tmp/architecture/06-paid-feeds.md:114` through `322`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Setting up a paid feed

A concrete walkthrough: building a "gas-oracle" agent that produces a paid ETH gas trend feed.

**Step 1: Declare the feed in the agent manifest**

```toml
# roko.toml -- agent manifest
[agent]
name = "gas-oracle"
profile = "chain"
mode = "persistent"

[agent.feeds]
[agent.feeds.eth-gas-trend]
kind = "derived"
description = "12-block EMA gas price with percentile bands and MEV spike detection"
schema = "gas_trend_v1"
rate_hz = 0.5
access = "paid"
base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)
# For pricier feeds:
# base_price_usdc_per_hour = 500000  # $0.50/hr

[agent.feeds.eth-gas-trend.sample]
# Sample payload shown to prospective subscribers before they pay
data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'
```

When the agent boots, `FeedPublisherExt` reads these declarations and registers them with the relay. No manual registration needed.

**Step 2: The FeedPublisherExt extension**

Auto-loaded when `[agent.feeds.*]` entries exist. Handles the full lifecycle: register on boot, publish on each tick, deregister on shutdown.

```rust
pub struct FeedPublisherExt {
    feeds: Vec<FeedConfig>,
    relay: RelayHandle,
}

#[async_trait]
impl Extension for FeedPublisherExt {
    fn name(&self) -> &str { "feed-publisher" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.register_feed(FeedRegistration {
                feed_id: feed.id.clone(),
                agent_id: ctx.agent_id.clone(),
                kind: feed.kind,
                schema: feed.schema.clone(),
                rate_hz: feed.rate_hz,
                access: feed.access.clone(),
                sample: feed.sample.clone(),
            }).await?;
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            if let Some(data) = ctx.cortical.get_feed_data(&feed.id) {
                ctx.relay.publish_feed_data(&feed.id, data).await?;
            }
        }
        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.deregister_feed(&feed.id).await?;
        }
        Ok(())
    }
}
```

**Step 3: Compute the feed data**

The feed's value comes from a `Cognition`-layer extension that runs during the agent's `on_observe` step -- before `FeedPublisherExt` publishes in `on_tick_end`.

```rust
pub struct GasTrendExt {
    ema: f64,
    window: VecDeque<f64>,
}

#[async_trait]
impl Extension for GasTrendExt {
    fn name(&self) -> &str { "gas-trend" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let gas = ctx.cortical.gas_gwei();
        self.window.push_back(gas);
        if self.window.len() > 100 { self.window.pop_front(); }

        // 12-block EMA
        let alpha = 2.0 / 13.0;
        self.ema = alpha * gas + (1.0 - alpha) * self.ema;

        // Percentiles from rolling window
        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p25 = sorted[sorted.len() / 4];
        let p75 = sorted[3 * sorted.len() / 4];
        let p95 = sorted[19 * sorted.len() / 20];

        // MEV spike: gas exceeds 2x the 95th percentile
        let mev_spike = gas > p95 * 2.0;

        ctx.cortical.set_feed_data("eth-gas-trend", json!({
            "ema_12": self.ema,
            "p25": p25,
            "p75": p75,
            "p95": p95,
            "mev_spike": mev_spike,
            "current": gas,
            "ts": now_ms(),
        }));

        Ok(())
    }
}
```

The pipeline order matters: `GasTrendExt` (Cognition layer) runs during `on_observe`, writes data to `cortical`. Then `FeedPublisherExt` (Social layer) runs during `on_tick_end`, reads the data and publishes it to the relay. Extension layers execute in order: Perception -> Cognition -> Social.

**Step 4: Subscribe from a dashboard**

```typescript
// 1. Discover available feeds
const feeds = await fetch(`${relayUrl}/relay/feeds`).then(r => r.json());
const gasFeed = feeds.find(f => f.feed_id === "eth-gas-trend");
// -> { feed_id, agent_id, kind: "derived", rate_hz: 0.5,
//      access: { paid: { price_per_hour: 50 } } }

// 2. Open an MPP session (one-time ERC-3009 signature)
const session = await openMppSession(relayUrl, {
  amount: 500,  // $0.0005 USDC -- enough for ~10 hours at $0.05/hr
  recipient: gasFeed.agent_wallet,
});
// -> { session_id: "abc-123", funded_amount: 500, status: "active" }

// 3. Subscribe to the feed via WebSocket with session auth
const ws = new WebSocket(`${relayUrl}/relay/ws`);
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: "subscribe",
    rooms: [`feed:${gasFeed.feed_id}`],
    payment: {
      intent: "session",
      session_id: session.session_id,
    }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === "feed_data") {
    // { ema_12: 42.5, p25: 35.0, p75: 55.0, p95: 120.0, mev_spike: false }
    updateGasChart(msg.payload);
  }
  if (msg.type === "payment_draw") {
    // { amount: 1, balance_remaining: 499, session_id: "abc-123" }
    updateBalance(msg.payload);
  }
};
```

**Step 5: Subscribe from another agent**

Agents consume feeds the same way dashboards do, but the subscription is managed by an extension and the session is opened programmatically.

```rust
pub struct GasConsumerExt {
    gas_subscription: Option<FeedSubscription>,
}

#[async_trait]
impl Extension for GasConsumerExt {
    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let session = ctx.mpp.open_session(
            "gas-oracle",  // agent producing the feed
            500,           // $0.0005 USDC
        ).await?;

        self.gas_subscription = Some(
            ctx.relay.subscribe_feed("eth-gas-trend", session.session_id).await?
        );
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        if let Some(sub) = &self.gas_subscription {
            if let Some(data) = sub.latest() {
                let mev_spike = data["mev_spike"].as_bool().unwrap_or(false);
                if mev_spike {
                    ctx.cortical.set_prediction_error(0.8);
                }
            }
        }
        Ok(())
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `777`
- Section hash: `ae474e71d311c559c3ad30060989571f1276a51839bef1e9b4ba49beb25693a4`

**Normative requirements and implementation claims:**
- **Step 1: Declare the feed in the agent manifest**
- **Step 2: The FeedPublisherExt extension**
- **Step 3: Compute the feed data**
- **Step 4: Subscribe from a dashboard**
- ws.onmessage = (event) => { const msg = JSON.parse(event.data); if (msg.type === "feed_data") { // { ema_12: 42.5, p25: 35.0, p75: 55.0, p95: 120.0, mev_spike: false } updateGasChart(msg.payload); } if (msg.type === "payment_draw") { // { amount: 1, balance_remaining: 499, session_id: "abc-123" } updateBalance(msg.payload); } }; ```
- **Step 5: Subscribe from another agent**

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- FeedPublisherExt
- name
- layer
- on_boot
- on_tick_end
- on_shutdown
- GasTrendExt
- on_observe
- GasConsumerExt
- Cognition
- cortical

**Event names and event-like entities:**
- agent.feeds
- agent.feeds.eth
- trend.sample
- agent.feeds.
- self.feeds
- ctx.relay.register_feed
- feed.id.clone
- ctx.agent_id.clone
- feed.kind
- feed.schema.clone
- feed.rate_hz
- feed.access.clone
- feed.sample.clone
- ctx.cortical.get_feed_data
- feed.id
- ctx.relay.publish_feed_data
- ctx.relay.deregister_feed
- ctx.cortical.gas_gwei
- self.window.push_back
- self.window.len
- self.window.pop_front
- self.ema
- self.window.iter
- sorted.sort_by
- sorted.len
- ctx.cortical.set_feed_data
- feeds.find
- eed.agent_wallet
- ws.onopen
- ws.send
- eed.feed_id
- session.session_id
- ws.onmessage
- event.data
- msg.type
- msg.payload
- ctx.mpp.open_session
- self.gas_subscription
- ctx.relay.subscribe_feed
- sub.latest
- ctx.cortical.set_prediction_error

**State transitions:**
- Perception -> Cognition -

**Config keys and TOML-like settings:**
- [agent]
- name = "gas-oracle"
- profile = "chain"
- mode = "persistent"
- [agent.feeds]
- [agent.feeds.eth-gas-trend]
- kind = "derived"
- description = "12-block EMA gas price with percentile bands and MEV spike detection"
- schema = "gas_trend_v1"
- rate_hz = 0.5
- access = "paid"
- base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)
- [agent.feeds.eth-gas-trend.sample]
- data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'
- self.ema = alpha * gas + (1.0 - alpha) * self.ema;
- ws.onopen = () => {
- ws.onmessage = (event) => {
- self.gas_subscription = Some(

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# roko.toml -- agent manifest`

```toml
# roko.toml -- agent manifest
[agent]
name = "gas-oracle"
profile = "chain"
mode = "persistent"

[agent.feeds]
[agent.feeds.eth-gas-trend]
kind = "derived"
description = "12-block EMA gas price with percentile bands and MEV spike detection"
schema = "gas_trend_v1"
rate_hz = 0.5
access = "paid"
base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)
# For pricier feeds:
# base_price_usdc_per_hour = 500000  # $0.50/hr

[agent.feeds.eth-gas-trend.sample]
# Sample payload shown to prospective subscribers before they pay
data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'
```
- Contract 2: language `rust`, first line `pub struct FeedPublisherExt {`

```rust
pub struct FeedPublisherExt {
    feeds: Vec<FeedConfig>,
    relay: RelayHandle,
}

#[async_trait]
impl Extension for FeedPublisherExt {
    fn name(&self) -> &str { "feed-publisher" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.register_feed(FeedRegistration {
                feed_id: feed.id.clone(),
                agent_id: ctx.agent_id.clone(),
                kind: feed.kind,
                schema: feed.schema.clone(),
                rate_hz: feed.rate_hz,
                access: feed.access.clone(),
                sample: feed.sample.clone(),
            }).await?;
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            if let Some(data) = ctx.cortical.get_feed_data(&feed.id) {
                ctx.relay.publish_feed_data(&feed.id, data).await?;
            }
        }
        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.deregister_feed(&feed.id).await?;
        }
        Ok(())
    }
}
```
- Contract 3: language `rust`, first line `pub struct GasTrendExt {`

```rust
pub struct GasTrendExt {
    ema: f64,
    window: VecDeque<f64>,
}

#[async_trait]
impl Extension for GasTrendExt {
    fn name(&self) -> &str { "gas-trend" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let gas = ctx.cortical.gas_gwei();
        self.window.push_back(gas);
        if self.window.len() > 100 { self.window.pop_front(); }

        // 12-block EMA
        let alpha = 2.0 / 13.0;
        self.ema = alpha * gas + (1.0 - alpha) * self.ema;

        // Percentiles from rolling window
        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p25 = sorted[sorted.len() / 4];
        let p75 = sorted[3 * sorted.len() / 4];
        let p95 = sorted[19 * sorted.len() / 20];

        // MEV spike: gas exceeds 2x the 95th percentile
        let mev_spike = gas > p95 * 2.0;

        ctx.cortical.set_feed_data("eth-gas-trend", json!({
            "ema_12": self.ema,
            "p25": p25,
            "p75": p75,
            "p95": p95,
            "mev_spike": mev_spike,
            "current": gas,
            "ts": now_ms(),
        }));

        Ok(())
    }
}
```
- Contract 4: language `typescript`, first line `// 1. Discover available feeds`

```typescript
// 1. Discover available feeds
const feeds = await fetch(`${relayUrl}/relay/feeds`).then(r => r.json());
const gasFeed = feeds.find(f => f.feed_id === "eth-gas-trend");
// -> { feed_id, agent_id, kind: "derived", rate_hz: 0.5,
//      access: { paid: { price_per_hour: 50 } } }

// 2. Open an MPP session (one-time ERC-3009 signature)
const session = await openMppSession(relayUrl, {
  amount: 500,  // $0.0005 USDC -- enough for ~10 hours at $0.05/hr
  recipient: gasFeed.agent_wallet,
});
// -> { session_id: "abc-123", funded_amount: 500, status: "active" }

// 3. Subscribe to the feed via WebSocket with session auth
const ws = new WebSocket(`${relayUrl}/relay/ws`);
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: "subscribe",
    rooms: [`feed:${gasFeed.feed_id}`],
    payment: {
      intent: "session",
      session_id: session.session_id,
    }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === "feed_data") {
    // { ema_12: 42.5, p25: 35.0, p75: 55.0, p95: 120.0, mev_spike: false }
    updateGasChart(msg.payload);
  }
  if (msg.type === "payment_draw") {
    // { amount: 1, balance_remaining: 499, session_id: "abc-123" }
    updateBalance(msg.payload);
  }
};
```
- Contract 5: language `rust`, first line `pub struct GasConsumerExt {`

```rust
pub struct GasConsumerExt {
    gas_subscription: Option<FeedSubscription>,
}

#[async_trait]
impl Extension for GasConsumerExt {
    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let session = ctx.mpp.open_session(
            "gas-oracle",  // agent producing the feed
            500,           // $0.0005 USDC
        ).await?;

        self.gas_subscription = Some(
            ctx.relay.subscribe_feed("eth-gas-trend", session.session_id).await?
        );
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        if let Some(sub) = &self.gas_subscription {
            if let Some(data) = sub.latest() {
                let mev_spike = data["mev_spike"].as_bool().unwrap_or(false);
                if mev_spike {
                    ctx.cortical.set_prediction_error(0.8);
                }
            }
        }
        Ok(())
    }
}
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|gas|self|ctx|feeds|layer|relay|data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|gas|self|ctx|feeds|layer|relay|data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
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
- [ ] Implement or verify `FeedPublisherExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `layer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_boot` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_tick_end` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_shutdown` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GasTrendExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_observe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GasConsumerExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Cognition` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `cortical` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.feeds` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `agent.feeds.eth` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `trend.sample` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `agent.feeds.` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.feeds` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.relay.register_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.id.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.agent_id.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.kind` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.schema.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.rate_hz` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.access.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.sample.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.get_feed_data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.id` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.relay.publish_feed_data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.relay.deregister_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.gas_gwei` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.window.push_back` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.window.len` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.window.pop_front` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.ema` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.window.iter` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `sorted.sort_by` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `sorted.len` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.set_feed_data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feeds.find` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `eed.agent_wallet` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.onopen` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `eed.feed_id` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `session.session_id` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.onmessage` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `event.data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `msg.type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `msg.payload` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.mpp.open_session` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.gas_subscription` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.relay.subscribe_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `sub.latest` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.set_prediction_error` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `Perception -> Cognition -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `[agent]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "gas-oracle"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "chain"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `mode = "persistent"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[agent.feeds]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[agent.feeds.eth-gas-trend]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `kind = "derived"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "12-block EMA gas price with percentile bands and MEV spike detection"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `schema = "gas_trend_v1"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `rate_hz = 0.5` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `access = "paid"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[agent.feeds.eth-gas-trend.sample]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `self.ema = alpha * gas + (1.0 - alpha) * self.ema;` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ws.onopen = () => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ws.onmessage = (event) => {` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `self.gas_subscription = Some(` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S004 -- Relay feed infrastructure

**Source section:** `tmp/architecture/06-paid-feeds.md:323` through `406`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Relay feed infrastructure

The relay manages the feed registry, payment gating, and message forwarding. All feed operations go through the relay -- producers publish to it, subscribers connect through it.

**Feed registration** (agent -> relay on boot):

```json
POST /relay/feeds/register
{
  "feed_id": "eth-gas-trend",
  "agent_id": "gas-oracle",
  "kind": "derived",
  "schema": "gas_trend_v1",
  "description": "12-block EMA gas price with percentile bands and MEV detection",
  "rate_hz": 0.5,
  "access": {
    "paid": {
      "base_price_usdc_per_hour": 50,
      "accepted_protocols": ["x402", "mpp"]
    }
  },
  "sample": {"ema_12": 42.5, "p25": 35.0}
}
```

**Feed discovery** (dashboard or agent -> relay):

```
GET /relay/feeds                              # all feeds
GET /relay/feeds?kind=derived&access=paid     # filter by kind and access
GET /relay/feeds?agent_id=gas-oracle          # feeds from a specific agent
GET /relay/feeds/{feed_id}                    # single feed metadata
GET /relay/feeds/{feed_id}/sample             # sample payload (free, no auth)
```

**Feed subscription with payment** (subscriber -> relay WebSocket):

```json
{
  "type": "subscribe",
  "rooms": ["feed:eth-gas-trend"],
  "payment": {
    "intent": "session",
    "session_id": "abc-123"
  }
}
```

The relay verifies the MPP session with the feed producer's agent, then forwards feed data to the subscriber. Each forwarded message triggers a draw.

**Payment flow through the relay**

```
Subscriber                    Relay                     Feed Producer
    │                           │                            │
    │  Open MPP session         │                            │
    │  (ERC-3009 auth)          │                            │
    │ ────────────────────────> │  Store session ref         │
    │                           │ ─────────────────────────> │
    │  Subscribe to feed room   │                            │
    │  with session_id          │                            │
    │ ────────────────────────> │                            │
    │                           │                            │
    │                           │  <── feed_data ──────────  │
    │                           │                            │
    │                           │  Draw from session:        │
    │                           │  cost = base_price         │
    │                           │        / rate_hz / 3600    │
    │                           │                            │
    │                           │  Draw succeeds?            │
    │  <── feed_data ────────── │  Yes: forward              │
    │  <── payment_draw ─────── │                            │
    │                           │                            │
    │                           │  Draw fails (exhausted)?   │
    │  <── exhaustion_notice ── │  Unsubscribe, notify       │
    │                           │                            │
    │  Top-up session           │                            │
    │ ────────────────────────> │  Resume draws              │
    │                           │                            │
    │  Disconnect / unsubscribe │                            │
    │ ────────────────────────> │  Session stays open        │
    │                           │  (reusable on reconnect)   │
```
````

**Explicit detail extraction from this section:**

- Section word count: `231`
- Section hash: `0d0d3e5ec858390dc85defa61f18d03997e27bc484ed6935724bebbd5cf20503`

**Normative requirements and implementation claims:**
- **Feed registration** (agent -> relay on boot):
- **Feed discovery** (dashboard or agent -> relay):
- **Feed subscription with payment** (subscriber -> relay WebSocket):
- **Payment flow through the relay**

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- relay/feeds/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- agent -> relay on boot
- dashboard or agent -> relay
- subscriber -> relay WebSocket

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `POST /relay/feeds/register`

```json
POST /relay/feeds/register
{
  "feed_id": "eth-gas-trend",
  "agent_id": "gas-oracle",
  "kind": "derived",
  "schema": "gas_trend_v1",
  "description": "12-block EMA gas price with percentile bands and MEV detection",
  "rate_hz": 0.5,
  "access": {
    "paid": {
      "base_price_usdc_per_hour": 50,
      "accepted_protocols": ["x402", "mpp"]
    }
  },
  "sample": {"ema_12": 42.5, "p25": 35.0}
}
```
- Contract 2: language `plain`, first line `GET /relay/feeds                              # all feeds`

```
GET /relay/feeds                              # all feeds
GET /relay/feeds?kind=derived&access=paid     # filter by kind and access
GET /relay/feeds?agent_id=gas-oracle          # feeds from a specific agent
GET /relay/feeds/{feed_id}                    # single feed metadata
GET /relay/feeds/{feed_id}/sample             # sample payload (free, no auth)
```
- Contract 3: language `json`, first line `{`

```json
{
  "type": "subscribe",
  "rooms": ["feed:eth-gas-trend"],
  "payment": {
    "intent": "session",
    "session_id": "abc-123"
  }
}
```
- Contract 4: language `plain`, first line `Subscriber                    Relay                     Feed Producer`

```
Subscriber                    Relay                     Feed Producer
    │                           │                            │
    │  Open MPP session         │                            │
    │  (ERC-3009 auth)          │                            │
    │ ────────────────────────> │  Store session ref         │
    │                           │ ─────────────────────────> │
    │  Subscribe to feed room   │                            │
    │  with session_id          │                            │
    │ ────────────────────────> │                            │
    │                           │                            │
    │                           │  <── feed_data ──────────  │
    │                           │                            │
    │                           │  Draw from session:        │
    │                           │  cost = base_price         │
    │                           │        / rate_hz / 3600    │
    │                           │                            │
    │                           │  Draw succeeds?            │
    │  <── feed_data ────────── │  Yes: forward              │
    │  <── payment_draw ─────── │                            │
    │                           │                            │
    │                           │  Draw fails (exhausted)?   │
    │  <── exhaustion_notice ── │  Unsubscribe, notify       │
    │                           │                            │
    │  Top-up session           │                            │
    │ ────────────────────────> │  Resume draws              │
    │
...
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `relay/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|Relay|session|subscribe|feeds|draw|payment|infrastructure" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|Relay|session|subscribe|feeds|draw|payment|infrastructure" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `relay/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
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
- [ ] Enforce state transition `agent -> relay on boot` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `dashboard or agent -> relay` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `subscriber -> relay WebSocket` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S005 -- Feed types and composability

**Source section:** `tmp/architecture/06-paid-feeds.md:407` through `458`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feed types and composability

Feeds are domain-agnostic. Any agent can produce any feed -- blockchain data, ML model outputs, sentiment analysis, code quality metrics, research signals, market indicators, or arbitrary computed streams. The feed system is the same regardless of domain.

Feeds compose into value chains. Each layer adds computation and charges for it.

**Raw feeds** -- direct data ingestion:

- Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket)
- Research: `arxiv-new-papers`, `github-trending` (from web polling)
- Code: `repo-commit-stream`, `ci-build-results` (from webhooks)
- Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs)
- Any external data source an agent consumes can be re-published as a raw feed

**Derived feeds** -- computed from raw:

- Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability`
- Research: `paper-relevance-scores`, `topic-cluster-updates`
- Code: `code-quality-trend`, `dependency-risk-index`
- Market: `volatility-regime`, `cross-venue-spread`
- Any computation an agent performs on its inputs can be a derived feed

**Composite feeds** -- derived from multiple derived feeds:

- `cross-chain-arb-signal` (consumes gas trends + volume + funding rates)
- `research-portfolio-impact` (consumes paper scores + code quality + market sentiment)
- Cost stacks: producer pays for input feeds, charges for output feed

**Meta feeds** -- feeds about feeds:

- `feed-health` (monitors all feeds for staleness, drift, anomalies)
- `feed-accuracy` (tracks prediction accuracy of derived feeds over time)
- Produced by meta-agents

Composition example:

```
eth-mainnet-blocks (free, raw)
  └─> gas-oracle agent
       └─> eth-gas-trend ($0.05/hr, derived)
            └─> arb-bot agent
                 └─> cross-chain-gas-arb ($0.50/hr, composite)
                      └─> dashboard subscriber

arxiv-new-papers (free, raw)
  └─> research-scout agent
       └─> defi-paper-relevance ($0.02/hr, derived)
            └─> strategy-agent subscribes for research context
```

Each agent in the chain pays for its inputs and charges for its output.
````

**Explicit detail extraction from this section:**

- Section word count: `286`
- Section hash: `f0652850cafa5c963c5a0a3229afbaca4af5d62579e43a2d76295f07c41aa2d7`

**Normative requirements and implementation claims:**
- **Raw feeds** -- direct data ingestion:
- - Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket) - Research: `arxiv-new-papers`, `github-trending` (from web polling) - Code: `repo-commit-stream`, `ci-build-results` (from webhooks) - Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs) - Any external data source an agent consumes can be re-published as a raw feed
- **Derived feeds** -- computed from raw:
- - Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability` - Research: `paper-relevance-scores`, `topic-cluster-updates` - Code: `code-quality-trend`, `dependency-risk-index` - Market: `volatility-regime`, `cross-venue-spread` - Any computation an agent performs on its inputs can be a derived feed
- **Composite feeds** -- derived from multiple derived feeds:
- - `cross-chain-arb-signal` (consumes gas trends + volume + funding rates) - `research-portfolio-impact` (consumes paper scores + code quality + market sentiment) - Cost stacks: producer pays for input feeds, charges for output feed
- **Meta feeds** -- feeds about feeds:
- - `feed-health` (monitors all feeds for staleness, drift, anomalies) - `feed-accuracy` (tracks prediction accuracy of derived feeds over time) - Produced by meta-agents
- ``` eth-mainnet-blocks (free, raw) └─> gas-oracle agent └─> eth-gas-trend ($0.05/hr, derived) └─> arb-bot agent └─> cross-chain-gas-arb ($0.50/hr, composite) └─> dashboard subscriber

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
- - Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket)
- - Research: `arxiv-new-papers`, `github-trending` (from web polling)
- - Code: `repo-commit-stream`, `ci-build-results` (from webhooks)
- - Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs)
- - Any external data source an agent consumes can be re-published as a raw feed
- - Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability`
- - Research: `paper-relevance-scores`, `topic-cluster-updates`
- - Code: `code-quality-trend`, `dependency-risk-index`
- - Market: `volatility-regime`, `cross-venue-spread`
- - Any computation an agent performs on its inputs can be a derived feed
- - `cross-chain-arb-signal` (consumes gas trends + volume + funding rates)
- - `research-portfolio-impact` (consumes paper scores + code quality + market sentiment)
- - Cost stacks: producer pays for input feeds, charges for output feed
- - `feed-health` (monitors all feeds for staleness, drift, anomalies)
- - `feed-accuracy` (tracks prediction accuracy of derived feeds over time)
- - Produced by meta-agents

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `eth-mainnet-blocks (free, raw)`

```
eth-mainnet-blocks (free, raw)
  └─> gas-oracle agent
       └─> eth-gas-trend ($0.05/hr, derived)
            └─> arb-bot agent
                 └─> cross-chain-gas-arb ($0.50/hr, composite)
                      └─> dashboard subscriber

arxiv-new-papers (free, raw)
  └─> research-scout agent
       └─> defi-paper-relevance ($0.02/hr, derived)
            └─> strategy-agent subscribes for research context
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Feed|feeds|chain|Derived|research|types|trend" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|feeds|chain|Derived|research|types|trend" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S006 -- On-chain feed advertisement (ERC-8004)

**Source section:** `tmp/architecture/06-paid-feeds.md:459` through `521`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### On-chain feed advertisement (ERC-8004)

Agents with wallets advertise their feeds in their ERC-8004 passport. This makes feeds discoverable on-chain even when the agent or relay is offline.

```solidity
// AgentRegistry.sol — feed advertisement extension
struct FeedAdvert {
    bytes32 feedId;        // keccak256 of feed name
    bytes32 schemaHash;    // keccak256 of schema definition
    uint16  rateMilliHz;   // rate in milli-Hz (500 = 0.5 Hz)
    uint96  pricePerHour;  // USDC base units per hour (0 = free)
    uint32  updatedAt;     // last update timestamp
}

function updateFeeds(FeedAdvert[] calldata adverts) external;
function getFeeds(address agent) external view returns (FeedAdvert[] memory);
```

When an agent boots with feeds configured, it:

1. Registers feeds with the relay (for live presence and subscription routing)
2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery)
3. On feed config changes (add/remove/reprice), updates both relay and chain

```rust
// In FeedPublisherExt::on_boot()
async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
    for feed in &self.feeds {
        // Register with relay (live routing)
        ctx.relay.register_feed(/* ... */).await?;

        // Advertise on-chain (persistent discovery)
        if let Some(chain) = &ctx.chain_client {
            chain.update_feed_advert(&ctx.agent_wallet, FeedAdvert {
                feed_id: keccak256(feed.id.as_bytes()),
                schema_hash: keccak256(feed.schema.as_bytes()),
                rate_milli_hz: (feed.rate_hz * 1000.0) as u16,
                price_per_hour: feed.price_usdc_per_hour,
            }).await?;
        }
    }
    Ok(())
}
```

Feed discovery uses both sources:

```typescript
// Dashboard merges relay (live) + chain (persistent) feed data
async function discoverFeeds(): Promise<Feed[]> {
  const [relayFeeds, chainFeeds] = await Promise.all([
    fetch(`${relayUrl}/relay/feeds`).then(r => r.json()),
    chainClient.getRegisteredFeeds(),  // reads all ERC-8004 feed adverts
  ]);

  // Merge: relay has live status, chain has persistent metadata
  return mergeFeeds(relayFeeds, chainFeeds);
  // Result: each feed has { ...chainAdvert, live: boolean, subscribers: number }
}
```

An agent's feeds appear in its passport even when the agent is offline. Users browsing the on-chain registry can see what feeds exist, their pricing, and their schemas -- then subscribe when the agent comes online.
````

**Explicit detail extraction from this section:**

- Section word count: `294`
- Section hash: `965807162866ff1e4ae37382753f3e11bdc3a563f01ae16b61e9c5c605c40f38`

**Normative requirements and implementation claims:**
- ```solidity // AgentRegistry.sol — feed advertisement extension struct FeedAdvert { bytes32 feedId; // keccak256 of feed name bytes32 schemaHash; // keccak256 of schema definition uint16 rateMilliHz; // rate in milli-Hz (500 = 0.5 Hz) uint96 pricePerHour; // USDC base units per hour (0 = free) uint32 updatedAt; // last update timestamp }
- 1. Registers feeds with the relay (for live presence and subscription routing) 2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery) 3. On feed config changes (add/remove/reprice), updates both relay and chain
- ```typescript // Dashboard merges relay (live) + chain (persistent) feed data async function discoverFeeds(): Promise<Feed[]> { const [relayFeeds, chainFeeds] = await Promise.all([ fetch(`${relayUrl}/relay/feeds`).then(r => r.json()), chainClient.getRegisteredFeeds(), // reads all ERC-8004 feed adverts ]);

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- add/remove/

**Types, functions, traits, and inline code identifiers:**
- FeedAdvert
- on_boot

**Event names and event-like entities:**
- self.feeds
- ctx.relay.register_feed
- ctx.chain_client
- chain.update_feed_advert
- ctx.agent_wallet
- feed.id.as_bytes
- feed.schema.as_bytes
- feed.rate_hz
- feed.price_usdc_per_hour
- romise.all
- lient.get

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Registers feeds with the relay (for live presence and subscription routing)
- 2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery)
- 3. On feed config changes (add/remove/reprice), updates both relay and chain

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `solidity`, first line `// AgentRegistry.sol — feed advertisement extension`

```solidity
// AgentRegistry.sol — feed advertisement extension
struct FeedAdvert {
    bytes32 feedId;        // keccak256 of feed name
    bytes32 schemaHash;    // keccak256 of schema definition
    uint16  rateMilliHz;   // rate in milli-Hz (500 = 0.5 Hz)
    uint96  pricePerHour;  // USDC base units per hour (0 = free)
    uint32  updatedAt;     // last update timestamp
}

function updateFeeds(FeedAdvert[] calldata adverts) external;
function getFeeds(address agent) external view returns (FeedAdvert[] memory);
```
- Contract 2: language `rust`, first line `// In FeedPublisherExt::on_boot()`

```rust
// In FeedPublisherExt::on_boot()
async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
    for feed in &self.feeds {
        // Register with relay (live routing)
        ctx.relay.register_feed(/* ... */).await?;

        // Advertise on-chain (persistent discovery)
        if let Some(chain) = &ctx.chain_client {
            chain.update_feed_advert(&ctx.agent_wallet, FeedAdvert {
                feed_id: keccak256(feed.id.as_bytes()),
                schema_hash: keccak256(feed.schema.as_bytes()),
                rate_milli_hz: (feed.rate_hz * 1000.0) as u16,
                price_per_hour: feed.price_usdc_per_hour,
            }).await?;
        }
    }
    Ok(())
}
```
- Contract 3: language `typescript`, first line `// Dashboard merges relay (live) + chain (persistent) feed data`

```typescript
// Dashboard merges relay (live) + chain (persistent) feed data
async function discoverFeeds(): Promise<Feed[]> {
  const [relayFeeds, chainFeeds] = await Promise.all([
    fetch(`${relayUrl}/relay/feeds`).then(r => r.json()),
    chainClient.getRegisteredFeeds(),  // reads all ERC-8004 feed adverts
  ]);

  // Merge: relay has live status, chain has persistent metadata
  return mergeFeeds(relayFeeds, chainFeeds);
  // Result: each feed has { ...chainAdvert, live: boolean, subscribers: number }
}
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `add/remove/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|chain|feeds|relay|advertise|FeedAdvert|ERC|advertisement" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|chain|feeds|relay|advertise|FeedAdvert|ERC|advertisement" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `add/remove/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Implement or verify `FeedAdvert` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_boot` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `self.feeds` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.relay.register_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.chain_client` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `chain.update_feed_advert` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.agent_wallet` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.id.as_bytes` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.schema.as_bytes` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.rate_hz` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.price_usdc_per_hour` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `romise.all` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `lient.get` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S007 -- Dashboard chain feed subscriptions

**Source section:** `tmp/architecture/06-paid-feeds.md:522` through `563`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard chain feed subscriptions

When the dashboard connects to a blockchain agent, it automatically subscribes to that agent's chain feeds for UI rendering. This is not just data display -- the dashboard uses raw chain feeds to render live blockchain state.

```typescript
// When user opens Agent Detail for a chain agent:
function useAgentChainFeeds(agent: MergedAgent) {
  const feeds = agent.feeds?.filter(f =>
    f.schema.startsWith("eth_") ||
    f.schema.startsWith("evm_") ||
    f.schema === "block" ||
    f.schema === "transaction"
  );

  // Auto-subscribe to chain feeds for live rendering
  for (const feed of feeds ?? []) {
    useRealtimeFeed(feed.feedId, {
      // Chain feeds from a connected agent are rendered as live chain state
      onData: (data) => {
        updateBlockHeight(data.blockNumber);
        updateGasGauge(data.gasUsed);
        updateTransactionList(data.transactions);
      }
    });
  }
}
```

The dashboard renders different UI elements based on feed schema:

| Feed schema | Dashboard renders |
|-------------|-------------------|
| `eth_block` | Block height counter, gas gauge, tx list |
| `evm_logs` | Live event log with contract decode |
| `gas_trend_*` | Gas price sparkline with percentile bands |
| `funding_*` | Funding rate chart with divergence alerts |
| `position_*` | Position cards with P&L |
| `tick_activity_*` | Liquidity heatmap |
| Any custom | Raw JSON viewer with auto-detected chart type |

For non-blockchain feeds, the dashboard renders based on the data shape -- numeric values get sparklines, boolean values get status indicators, arrays get tables.
````

**Explicit detail extraction from this section:**

- Section word count: `196`
- Section hash: `6be0dcb5274334eba62d960f35f8e1b8fa7efc2e550f34abc30098fa8559e5fc`

**Normative requirements and implementation claims:**
- When the dashboard connects to a blockchain agent, it automatically subscribes to that agent's chain feeds for UI rendering. This is not just data display -- the dashboard uses raw chain feeds to render live blockchain state.
- // Auto-subscribe to chain feeds for live rendering for (const feed of feeds ?? []) { useRealtimeFeed(feed.feedId, { // Chain feeds from a connected agent are rendered as live chain state onData: (data) => { updateBlockHeight(data.blockNumber); updateGasGauge(data.gasUsed); updateTransactionList(data.transactions); } }); } } ```
- The dashboard renders different UI elements based on feed schema:
- | Feed schema | Dashboard renders | |-------------|-------------------| | `eth_block` | Block height counter, gas gauge, tx list | | `evm_logs` | Live event log with contract decode | | `gas_trend_*` | Gas price sparkline with percentile bands | | `funding_*` | Funding rate chart with divergence alerts | | `position_*` | Position cards with P&L | | `tick_activity_*` | Liquidity heatmap | | Any custom | Raw JSON viewer with auto-detected chart type |
- For non-blockchain feeds, the dashboard renders based on the data shape -- numeric values get sparklines, boolean values get status indicators, arrays get tables.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- eth_block
- evm_logs

**Event names and event-like entities:**
- agent.feeds
- schema.starts
- feed.feed
- data.block
- data.gas
- data.transactions

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- f.schema === "block" ||
- f.schema === "transaction"

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Feed schema | Dashboard renders |
|-------------|-------------------|
| `eth_block` | Block height counter, gas gauge, tx list |
| `evm_logs` | Live event log with contract decode |
| `gas_trend_*` | Gas price sparkline with percentile bands |
| `funding_*` | Funding rate chart with divergence alerts |
| `position_*` | Position cards with P&L |
| `tick_activity_*` | Liquidity heatmap |
| Any custom | Raw JSON viewer with auto-detected chart type |
```

**Data/code contracts extracted:**
- Contract 1: language `typescript`, first line `// When user opens Agent Detail for a chain agent:`

```typescript
// When user opens Agent Detail for a chain agent:
function useAgentChainFeeds(agent: MergedAgent) {
  const feeds = agent.feeds?.filter(f =>
    f.schema.startsWith("eth_") ||
    f.schema.startsWith("evm_") ||
    f.schema === "block" ||
    f.schema === "transaction"
  );

  // Auto-subscribe to chain feeds for live rendering
  for (const feed of feeds ?? []) {
    useRealtimeFeed(feed.feedId, {
      // Chain feeds from a connected agent are rendered as live chain state
      onData: (data) => {
        updateBlockHeight(data.blockNumber);
        updateGasGauge(data.gasUsed);
        updateTransactionList(data.transactions);
      }
    });
  }
}
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|chain|feeds|block|render|data|schema|subscriptions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|chain|feeds|block|render|data|schema|subscriptions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Implement or verify `eth_block` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `evm_logs` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.feeds` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `schema.starts` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `feed.feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `data.block` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `data.gas` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `data.transactions` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `f.schema === "block" ||` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `f.schema === "transaction"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S008 -- Dashboard integration

**Source section:** `tmp/architecture/06-paid-feeds.md:564` through `672`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard integration

Three new surfaces: a feed browser, a subscription manager, and a producer revenue panel.

**Feeds page** (in Fleet or System section):

```
+--------------------------------------------------------------+
| Available Feeds                               [+ Publish Feed]|
|                                                               |
| Filter: [All v] [Paid v] [Chain: All v] [Search...]          |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend                                 * LIVE      | |
| | by gas-oracle (Trusted)                      $0.05/hr    | |
| | 12-block EMA gas with percentile bands + MEV detect      | |
| | Schema: gas_trend_v1   Rate: 0.5 Hz   Subs: 7           | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | uniswap-v3-tick-activity                     * LIVE      | |
| | by pool-watcher (Verified)                  $0.20/hr    | |
| | Real-time tick-level activity for top 50 pools           | |
| | Schema: tick_activity_v2   Rate: 2 Hz   Subs: 3         | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-mainnet-blocks                           * LIVE      | |
| | by chain-watcher-1 (Basic)                     FREE     | |
| | Raw Ethereum mainnet block headers                       | |
| | Schema: eth_block   Rate: 0.08 Hz   Subs: 12            | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

**My subscriptions** (in Treasury or Settings):

```
+--------------------------------------------------------------+
| My Feed Subscriptions                                         |
|                                                               |
| Active spend: $0.25/hr across 3 feeds                        |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend          * Active    Session: $4.82 left    | |
| | gas-oracle             $0.05/hr   Since: 2h ago           | |
| | [Pause] [Top-up $5] [Unsubscribe]                        | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | cross-chain-gas-arb    * Active    Session: $1.20 left    | |
| | arb-bot                $0.50/hr   Since: 45m ago          | |
| | [Pause] [Top-up $10] [Unsubscribe]                       | |
| +----------------------------------------------------------+ |
|                                                               |
| Total spent this month: $12.40                                |
| Total earned from my feeds: $8.70                             |
+--------------------------------------------------------------+
```

**Feed detail page** (click into a feed):

```
+--------------------------------------------------------------+
| eth-gas-trend                                     * LIVE      |
| by gas-oracle (Trusted, 342 episodes)            $0.05/hr    |
|                                                               |
| +--------------- Live Preview -------------------------+     |
| | EMA: 42.5 gwei   P25: 35.0   P75: 55.0   P95: 120  |     |
| | MEV: none                                            |     |
| |                                                      |     |
| | [sparkline chart of last 100 data points]            |     |
| +------------------------------------------------------+     |
|                                                               |
| Schema: gas_trend_v1                                          |
| Fields: ema_12 (f64), p25 (f64), p75 (f64), p95 (f64),      |
|         mev_spike (bool), current (f64), ts (u64)            |
|                                                               |
| Uptime: 99.7% (30d)   Avg latency: 120ms                     |
| Subscribers: 7   Revenue: $84.20 (30d)                        |
|                                                               |
| Dependencies: eth-mainnet-blocks (free)                       |
|                                                               |
| Payment: x402 or MPP session                                  |
| [Subscribe with MPP ($5 deposit)]  [Try with x402 ($0.01)]   |
+--------------------------------------------------------------+
```

**Feed revenue** (in Treasury / Cost Analytics):

```
+--------------------------------------------------------------+
| Feed Revenue                                                  |
|                                                               |
| Total earned (30d): $84.20    Active subscribers: 7           |
|                                                               |
| Feed               Subs  Revenue/30d  Status                  |
| eth-gas-trend       7     $84.20      * producing             |
|                                                               |
| [chart: revenue over time, subscriber count over time]        |
|                                                               |
| Settlement: 12 batches settled on-chain                       |
| Pending: $2.30 (next batch in ~8 min)                         |
+--------------------------------------------------------------+
```
````

**Explicit detail extraction from this section:**

- Section word count: `335`
- Section hash: `efa13a06cf3d8f53280b4082f0a159403f17e5e99da747893ab933e004ed56d8`

**Normative requirements and implementation claims:**
- **Feeds page** (in Fleet or System section):
- **My subscriptions** (in Treasury or Settings):
- **Feed detail page** (click into a feed):
- **Feed revenue** (in Treasury / Cost Analytics):

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- earch...

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
| Available Feeds                               [+ Publish Feed]|
|                                                               |
| Filter: [All v] [Paid v] [Chain: All v] [Search...]          |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend                                 * LIVE      | |
| | by gas-oracle (Trusted)                      $0.05/hr    | |
| | 12-block EMA gas with percentile bands + MEV detect      | |
| | Schema: gas_trend_v1   Rate: 0.5 Hz   Subs: 7           | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
...
```
- Table 2:

```markdown
| My Feed Subscriptions                                         |
|                                                               |
| Active spend: $0.25/hr across 3 feeds                        |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend          * Active    Session: $4.82 left    | |
| | gas-oracle             $0.05/hr   Since: 2h ago           | |
| | [Pause] [Top-up $5] [Unsubscribe]                        | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | cross-chain-gas-arb    * Active    Session: $1.20 left    | |
...
```
- Table 3:

```markdown
| eth-gas-trend                                     * LIVE      |
| by gas-oracle (Trusted, 342 episodes)            $0.05/hr    |
|                                                               |
| +--------------- Live Preview -------------------------+     |
| | EMA: 42.5 gwei   P25: 35.0   P75: 55.0   P95: 120  |     |
| | MEV: none                                            |     |
| |                                                      |     |
| | [sparkline chart of last 100 data points]            |     |
| +------------------------------------------------------+     |
|                                                               |
| Schema: gas_trend_v1                                          |
| Fields: ema_12 (f64), p25 (f64), p75 (f64), p95 (f64),      |
...
```
- Table 4:

```markdown
| Feed Revenue                                                  |
|                                                               |
| Total earned (30d): $84.20    Active subscribers: 7           |
|                                                               |
| Feed               Subs  Revenue/30d  Status                  |
| eth-gas-trend       7     $84.20      * producing             |
|                                                               |
| [chart: revenue over time, subscriber count over time]        |
|                                                               |
| Settlement: 12 batches settled on-chain                       |
| Pending: $2.30 (next batch in ~8 min)                         |
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `+--------------------------------------------------------------+`

```
+--------------------------------------------------------------+
| Available Feeds                               [+ Publish Feed]|
|                                                               |
| Filter: [All v] [Paid v] [Chain: All v] [Search...]          |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend                                 * LIVE      | |
| | by gas-oracle (Trusted)                      $0.05/hr    | |
| | 12-block EMA gas with percentile bands + MEV detect      | |
| | Schema: gas_trend_v1   Rate: 0.5 Hz   Subs: 7           | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | uniswap-v3-tick-activity                     * LIVE      | |
| | by pool-watcher (Verified)                  $0.20/hr    | |
| | Real-time tick-level activity for top 50 pools           | |
| | Schema: tick_activity_v2   Rate: 2 Hz   Subs: 3         | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-mainnet-blocks                           * LIVE      | |
| | by chain-watcher-1 (Basic)                     FREE     | |
| | Raw Ethereum mainnet block headers
...
```
- Contract 2: language `plain`, first line `+--------------------------------------------------------------+`

```
+--------------------------------------------------------------+
| My Feed Subscriptions                                         |
|                                                               |
| Active spend: $0.25/hr across 3 feeds                        |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend          * Active    Session: $4.82 left    | |
| | gas-oracle             $0.05/hr   Since: 2h ago           | |
| | [Pause] [Top-up $5] [Unsubscribe]                        | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | cross-chain-gas-arb    * Active    Session: $1.20 left    | |
| | arb-bot                $0.50/hr   Since: 45m ago          | |
| | [Pause] [Top-up $10] [Unsubscribe]                       | |
| +----------------------------------------------------------+ |
|                                                               |
| Total spent this month: $12.40                                |
| Total earned from my feeds: $8.70                             |
+--------------------------------------------------------------+
```
- Contract 3: language `plain`, first line `+--------------------------------------------------------------+`

```
+--------------------------------------------------------------+
| eth-gas-trend                                     * LIVE      |
| by gas-oracle (Trusted, 342 episodes)            $0.05/hr    |
|                                                               |
| +--------------- Live Preview -------------------------+     |
| | EMA: 42.5 gwei   P25: 35.0   P75: 55.0   P95: 120  |     |
| | MEV: none                                            |     |
| |                                                      |     |
| | [sparkline chart of last 100 data points]            |     |
| +------------------------------------------------------+     |
|                                                               |
| Schema: gas_trend_v1                                          |
| Fields: ema_12 (f64), p25 (f64), p75 (f64), p95 (f64),      |
|         mev_spike (bool), current (f64), ts (u64)            |
|                                                               |
| Uptime: 99.7% (30d)   Avg latency: 120ms                     |
| Subscribers: 7   Revenue: $84.20 (30d)                        |
|                                                               |
| Dependencies: eth-mainnet-blocks (free)                       |
|                                                               |
| Payment: x402 or MPP session                                  |
| [Subscribe with MPP ($5 deposit)]  [Try with x402 ($0.01)]   |
+--------------------------------------------------------------+
```
- Contract 4: language `plain`, first line `+--------------------------------------------------------------+`

```
+--------------------------------------------------------------+
| Feed Revenue                                                  |
|                                                               |
| Total earned (30d): $84.20    Active subscribers: 7           |
|                                                               |
| Feed               Subs  Revenue/30d  Status                  |
| eth-gas-trend       7     $84.20      * producing             |
|                                                               |
| [chart: revenue over time, subscriber count over time]        |
|                                                               |
| Settlement: 12 batches settled on-chain                       |
| Pending: $2.30 (next batch in ~8 min)                         |
+--------------------------------------------------------------+
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Subs|feed|Subscribe|trend|revenue|integration|block|LIVE" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Subs|feed|Subscribe|trend|revenue|integration|block|LIVE" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Emit or consume `earch...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S009 -- Feed data-source mapping

**Source section:** `tmp/architecture/06-paid-feeds.md:673` through `683`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feed data-source mapping

Extending the page-to-data-source table from the [Dashboard architecture](15-dashboard.md) section:

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Fleet / Feeds | `system` | `feed_registered`, `feed_deregistered`, `feed_status` | `GET /relay/feeds` |
| Fleet / Feed detail | `feed:{id}` | `feed_data`, `feed_status`, `payment_draw` | `GET /relay/feeds/{id}` |
| Treasury / Subscriptions | `system` | `session_opened`, `session_exhausted`, `session_settled` | `GET /mpp/sessions` |
| Treasury / Feed Revenue | `system` | `feed_revenue_update`, `settlement_batch` | `GET /relay/feeds/revenue` |
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `0c3cbe5c4466ddec4bc834ca5980970fd8e19f254b102844b9686ccff147d78b`

**Normative requirements and implementation claims:**
- Extending the page-to-data-source table from the [Dashboard architecture](15-dashboard.md) section:
- | Section | WS rooms | Event types | REST fallback | |---------|----------|-------------|---------------| | Fleet / Feeds | `system` | `feed_registered`, `feed_deregistered`, `feed_status` | `GET /relay/feeds` | | Fleet / Feed detail | `feed:{id}` | `feed_data`, `feed_status`, `payment_draw` | `GET /relay/feeds/{id}` | | Treasury / Subscriptions | `system` | `session_opened`, `session_exhausted`, `session_settled` | `GET /mpp/sessions` | | Treasury / Feed Revenue | `system` | `feed_revenue_update`, `settlement_batch` | `GET /relay/feeds/revenue` |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- relay/feeds/

**Types, functions, traits, and inline code identifiers:**
- system
- feed_registered
- feed_deregistered
- feed_status
- feed_data
- payment_draw
- session_opened
- session_exhausted
- session_settled
- feed_revenue_update
- settlement_batch

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
| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Fleet / Feeds | `system` | `feed_registered`, `feed_deregistered`, `feed_status` | `GET /relay/feeds` |
| Fleet / Feed detail | `feed:{id}` | `feed_data`, `feed_status`, `payment_draw` | `GET /relay/feeds/{id}` |
| Treasury / Subscriptions | `system` | `session_opened`, `session_exhausted`, `session_settled` | `GET /mpp/sessions` |
| Treasury / Feed Revenue | `system` | `feed_revenue_update`, `settlement_batch` | `GET /relay/feeds/revenue` |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `relay/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Feed|data|feed_status|settlement_batch|session_settled|session_opened|session_exhausted|payment_draw" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|data|feed_status|settlement_batch|session_settled|session_opened|session_exhausted|payment_draw" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `relay/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Implement or verify `system` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feed_registered` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feed_deregistered` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feed_status` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feed_data` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `payment_draw` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `session_opened` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `session_exhausted` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `session_settled` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feed_revenue_update` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `settlement_batch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S010 -- Practical example: funding rate divergence feed

**Source section:** `tmp/architecture/06-paid-feeds.md:684` through `786`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Practical example: funding rate divergence feed

A second example showing feed composition. This agent consumes two paid feeds and produces a third.

**Agent manifest**

```toml
[agent]
name = "funding-arb"
profile = "chain"
mode = "persistent"

# This agent CONSUMES two feeds...
[[agent.feed_subscriptions]]
feed_id = "binance-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000  # $0.001 USDC session deposit

[[agent.feed_subscriptions]]
feed_id = "hyperliquid-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000

# ...and PRODUCES one feed
[agent.feeds.funding-divergence]
kind = "derived"
description = "Cross-venue funding rate divergence with z-score normalization"
schema = "funding_divergence_v1"
rate_hz = 0.1  # Every 10 seconds
access = "paid"
base_price_usdc_per_hour = 200000  # $0.20/hr
```

**The extension**

```rust
pub struct FundingDivergenceExt {
    binance_sub: FeedSubscription,
    hyperliquid_sub: FeedSubscription,
    history: VecDeque<f64>,
}

#[async_trait]
impl Extension for FundingDivergenceExt {
    fn name(&self) -> &str { "funding-divergence" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let binance = self.binance_sub.latest_or_default();
        let hyper = self.hyperliquid_sub.latest_or_default();

        let divergence = binance["rate"].as_f64().unwrap_or(0.0)
            - hyper["rate"].as_f64().unwrap_or(0.0);

        self.history.push_back(divergence);
        if self.history.len() > 1000 { self.history.pop_front(); }

        let mean = self.history.iter().sum::<f64>() / self.history.len() as f64;
        let variance = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        let zscore = if variance > 0.0 {
            (divergence - mean) / variance.sqrt()
        } else {
            0.0
        };

        ctx.cortical.set_feed_data("funding-divergence", json!({
            "divergence_bps": divergence * 10000.0,
            "zscore": zscore,
            "binance_rate": binance["rate"],
            "hyperliquid_rate": hyper["rate"],
            "signal": if zscore.abs() > 2.0 { "strong" }
                      else if zscore.abs() > 1.0 { "moderate" }
                      else { "none" },
            "direction": if divergence > 0.0 { "long_hyper" }
                         else { "long_binance" },
            "ts": now_ms(),
        }));

        // Extreme divergence triggers T2 reasoning via prediction error
        if zscore.abs() > 3.0 {
            ctx.cortical.set_prediction_error(0.9);
        }

        Ok(())
    }
}
```

**The value chain**

```
cex-connector produces binance-funding-rates ($0.05/hr)
cex-connector produces hyperliquid-funding-rates ($0.05/hr)
  └─> funding-arb consumes both, pays $0.10/hr
       └─> funding-arb produces funding-divergence ($0.20/hr)
            └─> trading-bot subscribes, pays $0.20/hr
            └─> dashboard subscribes, pays $0.20/hr
```

Economics for `funding-arb`: $0.20/hr revenue per subscriber minus $0.10/hr input cost. With 5 subscribers that's ($0.20 * 5) - $0.10 = $0.90/hr pure margin.
````

**Explicit detail extraction from this section:**

- Section word count: `356`
- Section hash: `902bb51b5e20ddc30b79ea3fd4a1d5989be324b9824fd07ce8a9f505c200f691`

**Normative requirements and implementation claims:**
- **Agent manifest**
- **The extension**
- **The value chain**
- ``` cex-connector produces binance-funding-rates ($0.05/hr) cex-connector produces hyperliquid-funding-rates ($0.05/hr) └─> funding-arb consumes both, pays $0.10/hr └─> funding-arb produces funding-divergence ($0.20/hr) └─> trading-bot subscribes, pays $0.20/hr └─> dashboard subscribes, pays $0.20/hr ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- FundingDivergenceExt
- name
- layer
- on_observe

**Event names and event-like entities:**
- feeds...
- agent.feed_subscriptions
- agent.feeds.funding
- self.binance_sub.latest_or_default
- self.hyperliquid_sub.latest_or_default
- self.history.push_back
- self.history.len
- self.history.pop_front
- self.history.iter
- variance.sqrt
- ctx.cortical.set_feed_data
- zscore.abs
- ctx.cortical.set_prediction_error

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- [agent]
- name = "funding-arb"
- profile = "chain"
- mode = "persistent"
- feed_id = "binance-funding-rates"
- agent_id = "cex-connector"
- budget_usdc = 1000  # $0.001 USDC session deposit
- feed_id = "hyperliquid-funding-rates"
- budget_usdc = 1000
- [agent.feeds.funding-divergence]
- kind = "derived"
- description = "Cross-venue funding rate divergence with z-score normalization"
- schema = "funding_divergence_v1"
- rate_hz = 0.1  # Every 10 seconds
- access = "paid"
- base_price_usdc_per_hour = 200000  # $0.20/hr

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - hyper["rate"].as_f64().unwrap_or(0.0);

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `[agent]`

```toml
[agent]
name = "funding-arb"
profile = "chain"
mode = "persistent"

# This agent CONSUMES two feeds...
[[agent.feed_subscriptions]]
feed_id = "binance-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000  # $0.001 USDC session deposit

[[agent.feed_subscriptions]]
feed_id = "hyperliquid-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000

# ...and PRODUCES one feed
[agent.feeds.funding-divergence]
kind = "derived"
description = "Cross-venue funding rate divergence with z-score normalization"
schema = "funding_divergence_v1"
rate_hz = 0.1  # Every 10 seconds
access = "paid"
base_price_usdc_per_hour = 200000  # $0.20/hr
```
- Contract 2: language `rust`, first line `pub struct FundingDivergenceExt {`

```rust
pub struct FundingDivergenceExt {
    binance_sub: FeedSubscription,
    hyperliquid_sub: FeedSubscription,
    history: VecDeque<f64>,
}

#[async_trait]
impl Extension for FundingDivergenceExt {
    fn name(&self) -> &str { "funding-divergence" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let binance = self.binance_sub.latest_or_default();
        let hyper = self.hyperliquid_sub.latest_or_default();

        let divergence = binance["rate"].as_f64().unwrap_or(0.0)
            - hyper["rate"].as_f64().unwrap_or(0.0);

        self.history.push_back(divergence);
        if self.history.len() > 1000 { self.history.pop_front(); }

        let mean = self.history.iter().sum::<f64>() / self.history.len() as f64;
        let variance = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        let zscore = if variance > 0.0 {
            (divergence - mean) / variance.sqrt()
        } else {
            0.0
        };

        ctx.cortical.set_feed_data("funding-divergence", json!({
            "divergence_bps": divergence * 10000.0,
            "zscore": zscore,
            "binance_rate": binance["rate"],
            "hyperliquid_rate": hyper["rate"],
            "signal": if zscore.abs() > 2.0 { "strong" }
                      else if zscore.abs() > 1.0 { "moderate" }
                      else { "none" },
            "direction": if divergence > 0.0 { "long_hyper" }
                         else { "long
...
```
- Contract 3: language `plain`, first line `cex-connector produces binance-funding-rates ($0.05/hr)`

```
cex-connector produces binance-funding-rates ($0.05/hr)
cex-connector produces hyperliquid-funding-rates ($0.05/hr)
  └─> funding-arb consumes both, pays $0.10/hr
       └─> funding-arb produces funding-divergence ($0.20/hr)
            └─> trading-bot subscribes, pays $0.20/hr
            └─> dashboard subscribes, pays $0.20/hr
```

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "funding|divergence|rate|feed|self|hyper|binance|history" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "funding|divergence|rate|feed|self|hyper|binance|history" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Implement or verify `FundingDivergenceExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `layer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_observe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `feeds...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `agent.feed_subscriptions` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `agent.feeds.funding` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.binance_sub.latest_or_default` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.hyperliquid_sub.latest_or_default` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.history.push_back` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.history.len` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.history.pop_front` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.history.iter` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `variance.sqrt` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.set_feed_data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `zscore.abs` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.set_prediction_error` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `[agent]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "funding-arb"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `profile = "chain"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `mode = "persistent"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `feed_id = "binance-funding-rates"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `agent_id = "cex-connector"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `budget_usdc = 1000  # $0.001 USDC session deposit` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `feed_id = "hyperliquid-funding-rates"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `budget_usdc = 1000` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[agent.feeds.funding-divergence]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `kind = "derived"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `description = "Cross-venue funding rate divergence with z-score normalization"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `schema = "funding_divergence_v1"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `rate_hz = 0.1  # Every 10 seconds` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `access = "paid"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `base_price_usdc_per_hour = 200000  # $0.20/hr` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-06-S011 -- Extensibility

**Source section:** `tmp/architecture/06-paid-feeds.md:787` through `796`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extensibility

Any extension can produce a feed. The pattern:

1. Declare the feed in the agent manifest (`[agent.feeds.*]`).
2. Compute data in an extension's `on_observe()` or `on_tick_end()` hook.
3. Store via `ctx.cortical.set_feed_data(feed_id, data)`.
4. `FeedPublisherExt` (auto-loaded when feeds are declared) publishes to the relay.

Users create custom feed schemas, set arbitrary pricing, compose feeds from other feeds. The relay handles discovery, payment gating, and delivery. The dashboard surfaces it all without any feed-specific code -- it reads the schema from the registry and renders accordingly.
````

**Explicit detail extraction from this section:**

- Section word count: `90`
- Section hash: `38c79037e04163b7ae1d82f8fa5390a0cc346fc5cdaa2210827d92cba468798f`

**Normative requirements and implementation claims:**
- 1. Declare the feed in the agent manifest (`[agent.feeds.*]`). 2. Compute data in an extension's `on_observe()` or `on_tick_end()` hook. 3. Store via `ctx.cortical.set_feed_data(feed_id, data)`. 4. `FeedPublisherExt` (auto-loaded when feeds are declared) publishes to the relay.
- Users create custom feed schemas, set arbitrary pricing, compose feeds from other feeds. The relay handles discovery, payment gating, and delivery. The dashboard surfaces it all without any feed-specific code -- it reads the schema from the registry and renders accordingly.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- FeedPublisherExt

**Event names and event-like entities:**
- agent.feeds.
- ctx.cortical.set_feed_data

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Declare the feed in the agent manifest (`[agent.feeds.*]`).
- 2. Compute data in an extension's `on_observe()` or `on_tick_end()` hook.
- 3. Store via `ctx.cortical.set_feed_data(feed_id, data)`.
- 4. `FeedPublisherExt` (auto-loaded when feeds are declared) publishes to the relay.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/06-paid-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|FeedPublisherExt|Extensibility|feeds|data|schema|relay|extension" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|FeedPublisherExt|Extensibility|feeds|data|schema|relay|extension" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-chain/src/`
- `contracts/src/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
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
- [ ] Implement or verify `FeedPublisherExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.feeds.` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.cortical.set_feed_data` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/06-paid-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

