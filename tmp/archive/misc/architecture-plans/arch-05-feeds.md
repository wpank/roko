# Architecture Plan: Feeds

**Source:** `tmp/architecture/05-feeds.md`
**Generated:** 2026-04-25
**Source hash:** `1993e854f17de5c0a0e6ec670fe37ff0edf6a5aaaac754925a10177a4eb62382`
**Section tasks:** 14
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
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-05-S001 | 1 | Feeds and data streams | [ ] | 9.8 |
| ARCH-05-S002 | 12 | Blockchain: raw feed subscription | [ ] | 9.8 |
| ARCH-05-S003 | 14 | Raw feed subscription | [ ] | 9.8 |
| ARCH-05-S004 | 44 | Exposing feeds externally | [ ] | 9.8 |
| ARCH-05-S005 | 70 | Feed discovery | [ ] | 9.8 |
| ARCH-05-S006 | 98 | Feed registry pagination | [ ] | 9.8 |
| ARCH-05-S007 | 132 | Dashboard subscribes to agent chain feeds | [ ] | 9.8 |
| ARCH-05-S008 | 159 | Agent-to-agent feed marketplace | [ ] | 9.8 |
| ARCH-05-S009 | 174 | Dynamic endpoint registration | [ ] | 9.8 |
| ARCH-05-S010 | 196 | Recipes (universal primitive) | [ ] | 9.8 |
| ARCH-05-S011 | 202 | Why Recipe is distinct from Plan and Composer | [ ] | 9.8 |
| ARCH-05-S012 | 206 | Cross-domain examples | [ ] | 9.8 |
| ARCH-05-S013 | 217 | Rust mapping | [ ] | 9.8 |
| ARCH-05-S014 | 229 | Dashboard authoring surface | [ ] | 9.8 |

## Tasks

### ARCH-05-S001 -- Feeds and data streams

**Source section:** `tmp/architecture/05-feeds.md:1` through `11`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Feeds and data streams

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> See [Paid Feeds and MPP](06-paid-feeds.md) for payment protocols, pricing, and marketplace economics.

---

> **Universal primitive (PRD 23).** Feed is one of the 12 universal primitives in the canonical vocabulary. Its shape is `subscribe / unsubscribe / poll / status / configure`. A Feed uses a Connector as its source and publishes processed events to PulseBus. Feeds are the "always-on" complement to one-shot Connector queries. The architecture below already supports this universal definition -- blockchain feeds are the most data-intensive case, but the same pattern applies to CI status feeds, file change feeds, webhook streams, and any other continuous data source.

Any agent can produce and consume real-time data feeds. Blockchain agents subscribe to RPC WebSocket feeds; research agents ingest papers and web data; coding agents emit build status and metrics. The feed system is domain-agnostic -- the examples below use blockchain because that is the most data-intensive case, but the same extension pattern works for any data source.
````

**Explicit detail extraction from this section:**

- Section word count: `175`
- Section hash: `2579289c0ddedb6c41d2d26905230b7a503c43151ab88978f96292d42e199f32`

**Normative requirements and implementation claims:**
- ---
- > **Universal primitive (PRD 23).** Feed is one of the 12 universal primitives in the canonical vocabulary. Its shape is `subscribe / unsubscribe / poll / status / configure`. A Feed uses a Connector as its source and publishes processed events to PulseBus. Feeds are the "always-on" complement to one-shot Connector queries. The architecture below already supports this universal definition -- blockchain feeds are the most data-intensive case, but the same pattern applies to CI status feeds, file change feeds, webhook streams, and any other continuous data source.
- Any agent can produce and consume real-time data feeds. Blockchain agents subscribe to RPC WebSocket feeds; research agents ingest papers and web data; coding agents emit build status and metrics. The feed system is domain-agnostic -- the examples below use blockchain because that is the most data-intensive case, but the same extension pattern works for any data source.

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
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "Feed|Feeds|data|streams|subscribe|status|blockchain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|Feeds|data|streams|subscribe|status|blockchain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S002 -- Blockchain: raw feed subscription

**Source section:** `tmp/architecture/05-feeds.md:12` through `13`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Blockchain: raw feed subscription
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `dc905839d2537aaa5b86f568d5d00603b74692269f9b26fa792a0128d5422602`

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
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "subscription|raw|feed|Blockchain|feeds" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "subscription|raw|feed|Blockchain|feeds" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S003 -- Raw feed subscription

**Source section:** `tmp/architecture/05-feeds.md:14` through `43`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Raw feed subscription

```rust
// Inside a ChainReaderExt extension
impl ChainReaderExt {
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Subscribe to Ethereum mainnet new blocks
        let eth_blocks = ctx.chain_subscribe(
            "ethereum",
            "newHeads",
            json!({}),
        ).await?;

        // Subscribe to Uniswap V3 swap events on Base
        let base_swaps = ctx.chain_subscribe(
            "base",
            "logs",
            json!({
                "address": "0x...",  // Uniswap V3 factory
                "topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"]
            }),
        ).await?;

        self.feeds.push(eth_blocks);
        self.feeds.push(base_swaps);
        Ok(())
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `c5b9280cc5b0f8ce753a4b1941d911c385760bbeec439f14a990945feba6d97f`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- on_init

**Event names and event-like entities:**
- ctx.chain_subscribe
- self.feeds.push

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
- Contract 1: language `rust`, first line `// Inside a ChainReaderExt extension`

```rust
// Inside a ChainReaderExt extension
impl ChainReaderExt {
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Subscribe to Ethereum mainnet new blocks
        let eth_blocks = ctx.chain_subscribe(
            "ethereum",
            "newHeads",
            json!({}),
        ).await?;

        // Subscribe to Uniswap V3 swap events on Base
        let base_swaps = ctx.chain_subscribe(
            "base",
            "logs",
            json!({
                "address": "0x...",  // Uniswap V3 factory
                "topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"]
            }),
        ).await?;

        self.feeds.push(eth_blocks);
        self.feeds.push(base_swaps);
        Ok(())
    }
}
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "feed|swap|subscription|on_init|Raw|Subscribe|Base|self" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|swap|subscription|on_init|Raw|Subscribe|Base|self" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Implement or verify `on_init` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ctx.chain_subscribe` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `self.feeds.push` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S004 -- Exposing feeds externally

**Source section:** `tmp/architecture/05-feeds.md:44` through `69`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Exposing feeds externally

Any agent -- regardless of domain -- can register feeds with the relay. The relay handles discovery, subscription routing, and payment gating. The agent just publishes data; the relay does the rest.

```rust
// Agent publishes a raw feed
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-mainnet-blocks",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Raw,
    schema: FeedSchema::EthBlock,
    rate_hz: 0.08,  // ~1 block per 12s
    access: FeedAccess::Public,
})?;

// Agent publishes a derived feed (computed from raw data)
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-gas-trend",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("gas_trend_v1"),
    rate_hz: 0.5,
    access: FeedAccess::Paid { price_per_hour: 100 },  // 100 units/hr
})?;
```
````

**Explicit detail extraction from this section:**

- Section word count: `103`
- Section hash: `a41db03f20e5530a6f5df26639ad074c8c5009ae4fd28043ae2cd3e0bc3b772e`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- ctx.relay.register_feed
- ctx.agent_id.clone

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
- Contract 1: language `rust`, first line `// Agent publishes a raw feed`

```rust
// Agent publishes a raw feed
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-mainnet-blocks",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Raw,
    schema: FeedSchema::EthBlock,
    rate_hz: 0.08,  // ~1 block per 12s
    access: FeedAccess::Public,
})?;

// Agent publishes a derived feed (computed from raw data)
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-gas-trend",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("gas_trend_v1"),
    rate_hz: 0.5,
    access: FeedAccess::Paid { price_per_hour: 100 },  // 100 units/hr
})?;
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "feed|feeds|relay|externally|Exposing|schema|kind|ctx" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|feeds|relay|externally|Exposing|schema|kind|ctx" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Emit or consume `ctx.relay.register_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.agent_id.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S005 -- Feed discovery

**Source section:** `tmp/architecture/05-feeds.md:70` through `97`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feed discovery

The relay maintains a feed registry. Dashboards and agents discover feeds dynamically.

```
GET /relay/feeds
→ [
    {
      "feed_id": "eth-mainnet-blocks",
      "agent_id": "chain-watcher-1",
      "kind": "raw",
      "schema": "eth_block",
      "rate_hz": 0.08,
      "access": "public",
      "subscribers": 3
    },
    {
      "feed_id": "eth-gas-trend",
      "agent_id": "chain-watcher-1",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": { "paid": { "price_per_hour": 100 } },
      "subscribers": 1
    }
  ]
```
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `9eb86c65543ab45dfe20b1801e1c5526acaa16b7d28b95948ba9a29609981b47`

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
- Contract 1: language `plain`, first line `GET /relay/feeds`

```
GET /relay/feeds
→ [
    {
      "feed_id": "eth-mainnet-blocks",
      "agent_id": "chain-watcher-1",
      "kind": "raw",
      "schema": "eth_block",
      "rate_hz": 0.08,
      "access": "public",
      "subscribers": 3
    },
    {
      "feed_id": "eth-gas-trend",
      "agent_id": "chain-watcher-1",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": { "paid": { "price_per_hour": 100 } },
      "subscribers": 1
    }
  ]
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "Feed|discover|discovery|watcher|trend|subscribers|schema|relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|discover|discovery|watcher|trend|subscribers|schema|relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S006 -- Feed registry pagination

**Source section:** `tmp/architecture/05-feeds.md:98` through `131`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feed registry pagination

`GET /relay/feeds` supports cursor-based pagination and filtering.

**Query parameters**:

```
Parameter    Type      Default   Description
─────────    ────      ───────   ───────────
limit        u32       50        Results per page. Max: 200.
cursor       string    (none)    Opaque cursor from previous response's next_cursor.
kind         string    (none)    Filter by feed kind: "raw" or "derived".
access       string    (none)    Filter by access type: "public" or "paid".
agent_id     string    (none)    Filter to feeds from a specific agent.
schema       string    (none)    Filter by schema name (exact match).
search       string    (none)    Full-text search across feed_id and description.
```

**Example**: Get the second page of paid derived feeds:

```
GET /relay/feeds?kind=derived&access=paid&limit=20&cursor=eyJsYXN0IjoiZXRoLWdhcy10cmVuZCJ9

→ {
    "feeds": [ ... ],
    "next_cursor": "eyJsYXN0IjoiYnRjLXZvbC1pbmRleCJ9",
    "total": 47
  }
```

**Cursor format**: Opaque base64-encoded JSON. Clients must not parse or construct cursors -- treat them as opaque strings. The relay uses the last `feed_id` as the cursor anchor (keyset pagination, not offset). This stays stable when feeds are added or removed between pages.

When `next_cursor` is `null`, there are no more results.
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `196b986046304b6a6f976a78093ab66e64e29a498dce7af924a7a91c6129c59f`

**Normative requirements and implementation claims:**
- `GET /relay/feeds` supports cursor-based pagination and filtering.
- **Query parameters**:
- **Example**: Get the second page of paid derived feeds:
- **Cursor format**: Opaque base64-encoded JSON. Clients must not parse or construct cursors -- treat them as opaque strings. The relay uses the last `feed_id` as the cursor anchor (keyset pagination, not offset). This stays stable when feeds are added or removed between pages.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- feed_id
- next_cursor
- null

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
- Contract 1: language `plain`, first line `Parameter    Type      Default   Description`

```
Parameter    Type      Default   Description
─────────    ────      ───────   ───────────
limit        u32       50        Results per page. Max: 200.
cursor       string    (none)    Opaque cursor from previous response's next_cursor.
kind         string    (none)    Filter by feed kind: "raw" or "derived".
access       string    (none)    Filter by access type: "public" or "paid".
agent_id     string    (none)    Filter to feeds from a specific agent.
schema       string    (none)    Filter by schema name (exact match).
search       string    (none)    Full-text search across feed_id and description.
```
- Contract 2: language `plain`, first line `GET /relay/feeds?kind=derived&access=paid&limit=20&cursor=eyJsYXN0IjoiZXRoLWdhcy10cmVuZCJ9`

```
GET /relay/feeds?kind=derived&access=paid&limit=20&cursor=eyJsYXN0IjoiZXRoLWdhcy10cmVuZCJ9

→ {
    "feeds": [ ... ],
    "next_cursor": "eyJsYXN0IjoiYnRjLXZvbC1pbmRleCJ9",
    "total": 47
  }
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "Feed|cursor|string|pagination|next_cursor|none|feeds|feed_id" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|cursor|string|pagination|next_cursor|none|feeds|feed_id" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Implement or verify `feed_id` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `next_cursor` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `null` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S007 -- Dashboard subscribes to agent chain feeds

**Source section:** `tmp/architecture/05-feeds.md:132` through `158`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard subscribes to agent chain feeds

```typescript
function useChainFeed(agentId: string, feedId: string) {
  const [data, setData] = useState<FeedData[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);
    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}:feed:${feedId}`]
      }));
    };
    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      if (event.type === "feed_data") {
        setData(prev => [...prev.slice(-999), event.payload]);
      }
    };
    return () => ws.close();
  }, [agentId, feedId]);

  return data;
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `60`
- Section hash: `c751315027c9c1163c98714cad59bd980f72e059aec4556480a291ce5934e7a4`

**Normative requirements and implementation claims:**
- useEffect(() => { const ws = new WebSocket(`${relayUrl}/relay/ws`); ws.onopen = () => { ws.send(JSON.stringify({ type: "subscribe", rooms: [`agent:${agentId}:feed:${feedId}`] })); }; ws.onmessage = (e) => { const event = JSON.parse(e.data); if (event.type === "feed_data") { setData(prev => [...prev.slice(-999), event.payload]); } }; return () => ws.close(); }, [agentId, feedId]);

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
- event.type
- prev.slice
- event.payload
- ws.close

**State transitions:**
- None extracted from this section.

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
- Contract 1: language `typescript`, first line `function useChainFeed(agentId: string, feedId: string) {`

```typescript
function useChainFeed(agentId: string, feedId: string) {
  const [data, setData] = useState<FeedData[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);
    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}:feed:${feedId}`]
      }));
    };
    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      if (event.type === "feed_data") {
        setData(prev => [...prev.slice(-999), event.payload]);
      }
    };
    return () => ws.close();
  }, [agentId, feedId]);

  return data;
}
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "feed|data|subscribe|chain|subscribes|feeds|type|string" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|data|subscribe|chain|subscribes|feeds|type|string" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Emit or consume `ws.onopen` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.onmessage` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `event.type` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `prev.slice` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `event.payload` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.close` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S008 -- Agent-to-agent feed marketplace

**Source section:** `tmp/architecture/05-feeds.md:159` through `173`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent-to-agent feed marketplace

Agents can subscribe to each other's feeds, creating a data marketplace:

```rust
// Agent B subscribes to Agent A's derived feed
let subscription = ctx.relay.subscribe_feed(SubscribeFeedRequest {
    feed_id: "eth-gas-trend",
    source_agent_id: "chain-watcher-1",
})?;

// For paid feeds, payment is handled automatically via the inference gateway's
// cost tracking. The subscribing agent's budget is debited per hour.
```
````

**Explicit detail extraction from this section:**

- Section word count: `59`
- Section hash: `33d65170348224ba8228e1a77c2ecc6e13ee29a05bf1af983078e6da0ce37e9e`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- ctx.relay.subscribe_feed

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
- Contract 1: language `rust`, first line `// Agent B subscribes to Agent A's derived feed`

```rust
// Agent B subscribes to Agent A's derived feed
let subscription = ctx.relay.subscribe_feed(SubscribeFeedRequest {
    feed_id: "eth-gas-trend",
    source_agent_id: "chain-watcher-1",
})?;

// For paid feeds, payment is handled automatically via the inference gateway's
// cost tracking. The subscribing agent's budget is debited per hour.
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "feed|marketplace|subscribe|feeds|watcher|trend|tracking|subscription" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|marketplace|subscribe|feeds|watcher|trend|tracking|subscription" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Emit or consume `ctx.relay.subscribe_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S009 -- Dynamic endpoint registration

**Source section:** `tmp/architecture/05-feeds.md:174` through `195`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dynamic endpoint registration

Agents can register new endpoints at runtime. When an agent discovers a new data source or creates a derived feed, it announces it to the relay.

```rust
// Agent discovers a new DEX and creates a feed for it
ctx.relay.register_feed(FeedRegistration {
    feed_id: format!("dex-{}-swaps", dex_address),
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("dex_swap_v1"),
    rate_hz: 2.0,
    access: FeedAccess::Public,
})?;

// The dashboard discovers this feed dynamically
// because it subscribes to the "system" room and receives
// feed_registered events
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `81`
- Section hash: `51d660a18a94939022c324a9d78ec6f57a6e73fd749156109d067860199f80a6`

**Normative requirements and implementation claims:**
- // The dashboard discovers this feed dynamically // because it subscribes to the "system" room and receives // feed_registered events ```
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- ctx.relay.register_feed
- ctx.agent_id.clone

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
- Contract 1: language `rust`, first line `// Agent discovers a new DEX and creates a feed for it`

```rust
// Agent discovers a new DEX and creates a feed for it
ctx.relay.register_feed(FeedRegistration {
    feed_id: format!("dex-{}-swaps", dex_address),
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("dex_swap_v1"),
    rate_hz: 2.0,
    access: FeedAccess::Public,
})?;

// The dashboard discovers this feed dynamically
// because it subscribes to the "system" room and receives
// feed_registered events
```

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "feed|registration|endpoint|Dynamic|register|discovers|schema|relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|registration|endpoint|Dynamic|register|discovers|schema|relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Emit or consume `ctx.relay.register_feed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ctx.agent_id.clone` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S010 -- Recipes (universal primitive)

**Source section:** `tmp/architecture/05-feeds.md:196` through `201`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Recipes (universal primitive)

> Added 2026-04-24. Per dashboard PRD 23, Recipe is a first-class primitive in the 12-primitive vocabulary.

A Recipe is a composable data transformation pipeline. Its shape is `create / execute / chain / status / configure`. Recipes are the composition glue between Feeds, Scorers, and Signals -- they turn raw data streams into derived values.
````

**Explicit detail extraction from this section:**

- Section word count: `53`
- Section hash: `425c4d811117ee3ab65f5eb7495dddeb2ad845448bf1de9223402e3aec7043f6`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Per dashboard PRD 23, Recipe is a first-class primitive in the 12-primitive vocabulary.
- A Recipe is a composable data transformation pipeline. Its shape is `create / execute / chain / status / configure`. Recipes are the composition glue between Feeds, Scorers, and Signals -- they turn raw data streams into derived values.

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
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "Recipe|primitive|Recipes|universal|data|vocabulary|values|turn" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recipe|primitive|Recipes|universal|data|vocabulary|values|turn" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S011 -- Why Recipe is distinct from Plan and Composer

**Source section:** `tmp/architecture/05-feeds.md:202` through `205`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Why Recipe is distinct from Plan and Composer

A Plan is an agent task DAG (tasks with dependencies, assigned to agents). A Composer assembles prompts for LLM calls. A Recipe is a pure data pipeline: Feed -> transform -> transform -> output. No LLM inference, no agent dispatch. Recipes make indicator chains, P&L attribution, HDC encoding, and scoring pipelines into first-class composable objects that users can author, share, and backtest.
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `060e197413333c04c4b0429ec058403b70059ab00cbf1c9da6b3ad1c0d663843`

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
- Feed -> transform -
- transform -> output

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
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "Recipe|Plan|Composer|distinct|Why|transform|task" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Recipe|Plan|Composer|distinct|Why|transform|task" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Enforce state transition `Feed -> transform -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `transform -> output` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S012 -- Cross-domain examples

**Source section:** `tmp/architecture/05-feeds.md:206` through `216`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Cross-domain examples

| Domain | Recipe | Pipeline |
|--------|--------|----------|
| DeFi | Indicator pipeline | price feed -> MACD -> RSI -> regime score -> trading signal |
| DeFi | P&L attribution | fill events -> FIFO matching -> realized P&L -> Sharpe ratio |
| DeFi | HDC encoding | market state -> role-filler binding -> hypervector -> similarity search |
| Governance | Voting power | delegation graph -> power aggregation -> threshold signal |
| Ops | Cost tracking | agent token usage -> pricing -> daily/weekly rollup -> budget signal |
| Code | Quality scoring | diff -> lint warnings -> test coverage -> quality score |
````

**Explicit detail extraction from this section:**

- Section word count: `69`
- Section hash: `c858707bd3dd35c9e9c5eeb0de9326014088457a11487fa7f515958f5fad8b11`

**Normative requirements and implementation claims:**
- | Domain | Recipe | Pipeline | |--------|--------|----------| | DeFi | Indicator pipeline | price feed -> MACD -> RSI -> regime score -> trading signal | | DeFi | P&L attribution | fill events -> FIFO matching -> realized P&L -> Sharpe ratio | | DeFi | HDC encoding | market state -> role-filler binding -> hypervector -> similarity search | | Governance | Voting power | delegation graph -> power aggregation -> threshold signal | | Ops | Cost tracking | agent token usage -> pricing -> daily/weekly rollup -> budget signal | | Code | Quality scoring | diff -> lint warnings -> test coverage -> quality score |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- price feed -> MACD -
- RSI -> regime score -
- fill events -> FIFO matching -
- L -> Sharpe ratio
- market state -> role-filler binding -
- hypervector -> similarity search
- delegation graph -> power aggregation -
- agent token usage -> pricing -
- weekly rollup -> budget signal
- diff -> lint warnings -
- test coverage -> quality score

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Domain | Recipe | Pipeline |
|--------|--------|----------|
| DeFi | Indicator pipeline | price feed -> MACD -> RSI -> regime score -> trading signal |
| DeFi | P&L attribution | fill events -> FIFO matching -> realized P&L -> Sharpe ratio |
| DeFi | HDC encoding | market state -> role-filler binding -> hypervector -> similarity search |
| Governance | Voting power | delegation graph -> power aggregation -> threshold signal |
| Ops | Cost tracking | agent token usage -> pricing -> daily/weekly rollup -> budget signal |
| Code | Quality scoring | diff -> lint warnings -> test coverage -> quality score |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "domain|examples|Cross|signal|DeFi|score|power|fill" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "domain|examples|Cross|signal|DeFi|score|power|fill" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- [ ] Enforce state transition `price feed -> MACD -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `RSI -> regime score -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `fill events -> FIFO matching -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `L -> Sharpe ratio` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `market state -> role-filler binding -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `hypervector -> similarity search` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `delegation graph -> power aggregation -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `agent token usage -> pricing -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `weekly rollup -> budget signal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `diff -> lint warnings -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `test coverage -> quality score` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S013 -- Rust mapping

**Source section:** `tmp/architecture/05-feeds.md:217` through `228`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Rust mapping

Recipes compose `Scorer` trait instances from `roko-core`. Existing implementations that become recipe templates:

| Crate | Struct | Recipe type |
|-------|--------|-------------|
| `roko-learn` | `TradingReflect` | P&L attribution pipeline |
| `roko-learn` | `FifoMatcher` | Position matching transform |
| `roko-learn` | `IndicatorTracker` | Indicator accuracy pipeline |
| `roko-primitives` | `MarketHdcEncoder` | HDC encoding pipeline |
| `roko-dreams` | `CounterfactualEngine` | Alternative outcome simulation |
````

**Explicit detail extraction from this section:**

- Section word count: `49`
- Section hash: `d27c4447d1294bf034c9c213b8df93654de3b7528d5762a0963d701de6e1e779`

**Normative requirements and implementation claims:**
- | Crate | Struct | Recipe type | |-------|--------|-------------| | `roko-learn` | `TradingReflect` | P&L attribution pipeline | | `roko-learn` | `FifoMatcher` | Position matching transform | | `roko-learn` | `IndicatorTracker` | Indicator accuracy pipeline | | `roko-primitives` | `MarketHdcEncoder` | HDC encoding pipeline | | `roko-dreams` | `CounterfactualEngine` | Alternative outcome simulation |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- instances
- Scorer
- TradingReflect
- FifoMatcher
- IndicatorTracker
- MarketHdcEncoder
- CounterfactualEngine

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
| Crate | Struct | Recipe type |
|-------|--------|-------------|
| `roko-learn` | `TradingReflect` | P&L attribution pipeline |
| `roko-learn` | `FifoMatcher` | Position matching transform |
| `roko-learn` | `IndicatorTracker` | Indicator accuracy pipeline |
| `roko-primitives` | `MarketHdcEncoder` | HDC encoding pipeline |
| `roko-dreams` | `CounterfactualEngine` | Alternative outcome simulation |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "mapping|instances|TradingReflect|Scorer|Rust|MarketHdcEncoder|IndicatorTracker|FifoMatcher" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mapping|instances|TradingReflect|Scorer|Rust|MarketHdcEncoder|IndicatorTracker|FifoMatcher" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `instances` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Scorer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TradingReflect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FifoMatcher` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `IndicatorTracker` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `MarketHdcEncoder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CounterfactualEngine` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-05-S014 -- Dashboard authoring surface

**Source section:** `tmp/architecture/05-feeds.md:229` through `236`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard authoring surface

The Recipe Editor is a 4-stage authoring surface (per PRD 23):

1. **Input selection** -- choose feed(s) or connector query as input (drag from feed/connector list)
2. **Pipeline builder** -- chain transform stages: map, filter, window, aggregate, score (visual DAG editor)
3. **Output configuration** -- emit as: Signal, Knowledge Entry, Feed, or raw value (type-checked output)
4. **Backtest and validate** -- run against historical data, compare output distribution (chart overlay)
````

**Explicit detail extraction from this section:**

- Section word count: `70`
- Section hash: `cd3a75d6d1902307eebd0718b88c53f55250e42371555f213ffff1fb6bbae85b`

**Normative requirements and implementation claims:**
- 1. **Input selection** -- choose feed(s) or connector query as input (drag from feed/connector list) 2. **Pipeline builder** -- chain transform stages: map, filter, window, aggregate, score (visual DAG editor) 3. **Output configuration** -- emit as: Signal, Knowledge Entry, Feed, or raw value (type-checked output) 4. **Backtest and validate** -- run against historical data, compare output distribution (chart overlay)

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
- 1. **Input selection** -- choose feed(s) or connector query as input (drag from feed/connector list)
- 2. **Pipeline builder** -- chain transform stages: map, filter, window, aggregate, score (visual DAG editor)
- 3. **Output configuration** -- emit as: Signal, Knowledge Entry, Feed, or raw value (type-checked output)
- 4. **Backtest and validate** -- run against historical data, compare output distribution (chart overlay)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/05-feeds.md`
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
rg -n "surface|authoring|feed|Output|stage|connector|Input|Editor" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|authoring|feed|Output|stage|connector|Input|Editor" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/feed.rs`
- `crates/roko-serve/src/routes/feeds.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/05-feeds
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

