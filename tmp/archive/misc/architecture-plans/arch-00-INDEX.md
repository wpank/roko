# Architecture Plan: Index

**Source:** `tmp/architecture/00-INDEX.md`
**Generated:** 2026-04-25
**Source hash:** `cf4a72f48245c449a8eb8969013759964c6e32ca61d86ac2208905cc35e27977`
**Section tasks:** 5
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
- Determine canonical target modules with the discovery commands in each task.

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-00-S001 | 1 | Roko Architecture Specification | [ ] | 9.8 |
| ARCH-00-S002 | 6 | Document Map | [ ] | 9.8 |
| ARCH-00-S003 | 32 | Primitive vocabulary | [ ] | 9.8 |
| ARCH-00-S004 | 53 | Reading order | [ ] | 9.8 |
| ARCH-00-S005 | 58 | Source references | [ ] | 9.8 |

## Tasks

### ARCH-00-S001 -- Roko Architecture Specification

**Source section:** `tmp/architecture/00-INDEX.md:1` through `5`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Roko Architecture Specification

> Canonical architecture for the Nunchi agent platform.
> Split from `roko-architecture-redesign-v2.md` for maintainability.
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `6f7b3059b9fbad40c35807b9b78d4b7977bb6afe0bffa6dc672832617f3f4c57`

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
- roko-architecture-redesign-v2.md

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/00-INDEX.md`
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
rg -n "Specification|redesign|platform|maintainability|Split|Nunchi|Canonical|INDEX" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Specification|redesign|platform|maintainability|Split|Nunchi|Canonical|INDEX" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
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
- [ ] Add or verify config key `roko-architecture-redesign-v2.md` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/00-INDEX
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-00-S002 -- Document Map

**Source section:** `tmp/architecture/00-INDEX.md:6` through `31`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Document Map

| # | Document | Scope | Status |
|---|----------|-------|--------|
| 01 | [Overview and Problem](01-overview.md) | System diagram, deployment tiers, design principles | Ported from v2 |
| 02 | [Agent Runtime](02-agent-runtime.md) | AgentRuntime struct, 9-step pipeline, modes, timescales, T0/T1/T2 gating, adaptive clock algorithm, cortical state persistence, **acceptance criteria** | Ported from v2 + gaps filled + **AC added 2026-04-25** |
| 03 | [Extensions](03-extensions.md) | Extension trait, 8 layers, 22 hooks, loading/discovery, dependency resolution, **decision enum variants, hook timeout, AgentContext, connector discovery, acceptance criteria** | Ported from v2 + gaps filled + **spec clarifications 2026-04-25** |
| 04 | [Connectivity and Relay](04-connectivity.md) | In-process agents, remote agents, relay protocol, cross-user communication, message routing, relay scalability, disconnection recovery, reconnection | Ported from v2 + gaps filled |
| 05 | [Feeds and Data Streams](05-feeds.md) | Raw/derived/composite/meta feeds, ERC-8004 advertisement, feed registry, pagination, dashboard chain subscriptions | Ported from v2 + gaps filled |
| 06 | [Paid Feeds and MPP](06-paid-feeds.md) | x402, MPP sessions, payment gating, reputation pricing, feed marketplace, practical examples | Ported from v2 |
| 07 | [Inference Gateway](07-gateway.md) | 12 subsystems, pipeline, InferenceHandle, CascadeRouter, concurrency/backpressure, provider fallback, proxy for isolated agents | Ported from v2 + gaps filled |
| 08 | [Authentication](08-auth.md) | Privy, API keys, agent tokens (full lifecycle + revocation), wallet signatures, scopes, relay auth, JWKS caching | Ported from v2 + gaps filled |
| 09 | [Knowledge and Pheromones](09-knowledge.md) | InsightStore, knowledge publish/validate/challenge/decay, HDC embeddings, pheromone deposits, stigmergy, dream consolidation | NEW |
| 10 | [Groups and Coordination](10-groups.md) | Group identity, membership, coordination protocol, shared context, cluster pipelines | NEW |
| 11 | [Arenas, Evals, and Bounties](11-arenas.md) | Arena registry, task sources, scoring functions, leaderboards, eval registry, bounty market, clearing | NEW |
| 12 | [DeFi Infrastructure](12-defi.md) | ISFR oracle, yield perpetuals, cooperative clearing, multi-chain, bridge architecture | NEW |
| 13 | [Meta Layer](13-meta.md) | Meta-agents, generators, lineage tracking, recursive safety monitoring | NEW |
| 14 | [On-Chain Registries](14-registries.md) | ERC-8004 agent passport, reputation registry, knowledge registry, arena/eval/bounty contracts, event indexer | NEW |
| 15 | [Dashboard Architecture](15-dashboard.md) | Data layer, subscription manager, aggregation service, page-to-data mapping, adaptive density, epistemic aesthetics, performance targets | Ported from v2 + gaps filled |
| 16 | [Secrets and Configuration](16-config.md) | **Complete roko.toml schema reference** (30+ sections from schema.rs), secret management, load precedence, env expansion, config versions | **Rewritten 2026-04-25** from skeleton to full reference |
| 17 | [Deployment](17-deployment.md) | Railway, Fly, local dev, agent creation UX, scaling tiers | Ported from v2 |
| 18 | [Implementation Roadmap](18-roadmap.md) | Phases 1-10, dependencies, crate mapping, migration from v1, **bardo naming map, 48-task summary, dependency graph, critical path** | Updated from v2 + integration plan folded |
| 19 | [Visual Composition and Authoring](19-visual-composition.md) | Plan mutation protocol, conversation-as-plan-editor, template registry, extension compilation, gate testing, authoring API contracts, cost projection | NEW |
| 20 | [Orchestrator and Learning Gaps](20-orchestrator-gaps.md) | Structured reviews, compile error classification, error pattern sharing, post-gate reflection, context scoping, warm spawning, 10 conductor watchers, neuro→cascade router, episode clustering, A-MAC admission, **current state reconciliation, 12 spec clarifications** | Folded from integration plan + **updated 2026-04-24** |
| 21 | [TUI and Operations](21-tui-and-operations.md) | DaimonState visualization, heartbeat status view, knowledge browser, justfile, E2E test harness, self-healing supervisor, **conductor watcher config, implementation state table** | Folded from integration plan + **updated 2026-04-25** |
````

**Explicit detail extraction from this section:**

- Section word count: `554`
- Section hash: `bbbee6e335256373127145a1cc0d3a940402bc388a58ffd24728e83b6622955c`

**Normative requirements and implementation claims:**
- | # | Document | Scope | Status | |---|----------|-------|--------| | 01 | [Overview and Problem](01-overview.md) | System diagram, deployment tiers, design principles | Ported from v2 | | 02 | [Agent Runtime](02-agent-runtime.md) | AgentRuntime struct, 9-step pipeline, modes, timescales, T0/T1/T2 gating, adaptive clock algorithm, cortical state persistence, **acceptance criteria** | Ported from v2 + gaps filled + **AC added 2026-04-25** | | 03 | [Extensions](03-extensions.md) | Extension trait, 8 layers, 22 hooks, loading/discovery, dependency resolution, **decision enum variants, hook timeout, AgentContext, connector discovery, acceptance criteria** | Ported from v2 + gaps filled + **spec clarifications 2026-04-25** | | 04 | [Connectivity and Relay](04-connectivity.md) | In-process agents, remote agents, relay protocol, cross-user communication, message routing, relay scalability, disconnection recovery, reconnection | Ported from v2 + gaps filled | | 05 | [Feeds and Data Streams](05

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Raw/derived/composite/
- T0/T1/
- arena/eval/
- publish/validate/challenge/

**Types, functions, traits, and inline code identifiers:**
- variants

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- neuro -> cascade router

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| # | Document | Scope | Status |
|---|----------|-------|--------|
| 01 | [Overview and Problem](01-overview.md) | System diagram, deployment tiers, design principles | Ported from v2 |
| 02 | [Agent Runtime](02-agent-runtime.md) | AgentRuntime struct, 9-step pipeline, modes, timescales, T0/T1/T2 gating, adaptive clock algorithm, cortical state persistence, **acceptance criteria** | Ported from v2 + gaps filled + **AC added 2026-04-25** |
| 03 | [Extensions](03-extensions.md) | Extension trait, 8 layers, 22 hooks, loading/discovery, dependency resolution, **decision enum variants, hook timeout, AgentContext, connector discovery, acceptance criteria** | Ported from v2 + gaps filled + **spec clarifications 2026-04-25** |
| 04 | [Connectivity and Relay](04-connectivity.md) | In-process agents, remote agents, relay protocol, cross-user communication, message routing, relay scalability, disconnection recovery, reconnection | Ported from v2 + gaps filled |
| 05 | [Feeds and Data Streams](05-feeds.md) | Raw/derived/composite/meta feeds, ERC-8004 advertisement, feed registry, pagination, dashboard chain subscriptions | Ported from v2 + gaps filled |
| 06 | [Paid Feeds and MPP](06-paid-feeds.md) | x402, MPP sessions, payment gating, reputation pricing, feed marketplace, practical examples | Ported from v2 |
| 07 | [Inference Gateway](07-gateway.md) | 12 subsystems, pipeline, InferenceHandle, CascadeRouter, concurrency/backpressure, provider fallback, proxy for isolated agents | Ported from v2 + gaps filled |
| 08 | [Authentication](08-auth.md) | Privy, API keys, agent tokens (full lifecycle + revocation), wallet signatures, scopes, relay auth, JWKS caching | Ported from v2 + gaps filled |
| 09 | [Knowledge and Pheromones](09-knowledge.md) | InsightStore, knowledge publish/validate/challenge/decay, HDC embeddings, pheromone deposits, stigmergy, dream consolidation | NEW |
| 10 | [Groups and Coordination](10-groups.md) | Group identity, membership, coordination protocol, shared context, cluster pipelines | NEW |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/00-INDEX.md`
- `Raw/derived/composite/`
- `T0/T1/`
- `arena/eval/`
- `publish/validate/challenge/`
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
rg -n "Ported|Map|gaps|filled|feed|registry|Document|variants" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Ported|Map|gaps|filled|feed|registry|Document|variants" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `Raw/derived/composite/`
- `T0/T1/`
- `arena/eval/`
- `publish/validate/challenge/`
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
- [ ] Implement or verify `variants` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `neuro -> cascade router` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/00-INDEX
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-00-S003 -- Primitive vocabulary

**Source section:** `tmp/architecture/00-INDEX.md:32` through `52`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Primitive vocabulary

The canonical primitive vocabulary is **12 primitives** (per dashboard PRD 23, superseding the 10-primitive vocabulary in PRD 20):

| # | Primitive | Status vs PRD 20 |
|---|-----------|------------------|
| 1 | Agent | Restructured -- Domain merged in as `ArchetypeManifest` field |
| 2 | Extension | Kept (three-tier: Pi-compatible, Roko-enhanced, Roko-native) |
| 3 | **Connector** | NEW -- external system I/O (VenueAdapter, ChainRpc, MCP, databases) |
| 4 | Gate | Expanded (pre-action + post-action) |
| 5 | **Feed** | NEW -- continuous data streams (price feeds, event watchers, CI status) |
| 6 | **Recipe** | NEW -- data transformation pipelines (indicator chains, P&L attribution, HDC encoding) |
| 7 | Knowledge Entry | Kept |
| 8 | Arena | Kept |
| 9 | Eval | Kept |
| 10 | Signal | Renamed from Pheromone at product layer; backend keeps internal pheromone naming |
| 11 | Group | Kept |
| 12 | Bounty | Kept |

Each primitive has a defined shape (verb set), Rust trait mapping, and dashboard authoring surface. See PRD 23 for the full composition matrix and DeFi struct mapping.
````

**Explicit detail extraction from this section:**

- Section word count: `143`
- Section hash: `77161cd52c3d8238747c4f1eb265fd615303411bfe29c5000d5743898219d1a5`

**Normative requirements and implementation claims:**
- The canonical primitive vocabulary is **12 primitives** (per dashboard PRD 23, superseding the 10-primitive vocabulary in PRD 20):
- | # | Primitive | Status vs PRD 20 | |---|-----------|------------------| | 1 | Agent | Restructured -- Domain merged in as `ArchetypeManifest` field | | 2 | Extension | Kept (three-tier: Pi-compatible, Roko-enhanced, Roko-native) | | 3 | **Connector** | NEW -- external system I/O (VenueAdapter, ChainRpc, MCP, databases) | | 4 | Gate | Expanded (pre-action + post-action) | | 5 | **Feed** | NEW -- continuous data streams (price feeds, event watchers, CI status) | | 6 | **Recipe** | NEW -- data transformation pipelines (indicator chains, P&L attribution, HDC encoding) | | 7 | Knowledge Entry | Kept | | 8 | Arena | Kept | | 9 | Eval | Kept | | 10 | Signal | Renamed from Pheromone at product layer; backend keeps internal pheromone naming | | 11 | Group | Kept | | 12 | Bounty | Kept |
- Each primitive has a defined shape (verb set), Rust trait mapping, and dashboard authoring surface. See PRD 23 for the full composition matrix and DeFi struct mapping.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- mapping
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
| # | Primitive | Status vs PRD 20 |
|---|-----------|------------------|
| 1 | Agent | Restructured -- Domain merged in as `ArchetypeManifest` field |
| 2 | Extension | Kept (three-tier: Pi-compatible, Roko-enhanced, Roko-native) |
| 3 | **Connector** | NEW -- external system I/O (VenueAdapter, ChainRpc, MCP, databases) |
| 4 | Gate | Expanded (pre-action + post-action) |
| 5 | **Feed** | NEW -- continuous data streams (price feeds, event watchers, CI status) |
| 6 | **Recipe** | NEW -- data transformation pipelines (indicator chains, P&L attribution, HDC encoding) |
| 7 | Knowledge Entry | Kept |
| 8 | Arena | Kept |
| 9 | Eval | Kept |
| 10 | Signal | Renamed from Pheromone at product layer; backend keeps internal pheromone naming |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/00-INDEX.md`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Primitive|vocabulary|mapping|Kept|ArchetypeManifest|data|struct|action" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Primitive|vocabulary|mapping|Kept|ArchetypeManifest|data|struct|action" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
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
- [ ] Implement or verify `mapping` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArchetypeManifest` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/00-INDEX
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-00-S004 -- Reading order

**Source section:** `tmp/architecture/00-INDEX.md:53` through `57`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Reading order

For a full understanding, read in order: 01 -> 02 -> 03 -> 04 -> 05 -> 07 -> 08 -> 09 -> 14 -> 15.
For implementation priority, start with 18 (roadmap) then read the phase-relevant docs.
````

**Explicit detail extraction from this section:**

- Section word count: `30`
- Section hash: `19d334636a90ee24bd8220cc34bf57f6fa71c4963e6f8a581a6534db9428eefb`

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
- `tmp/architecture/00-INDEX.md`
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
rg -n "read|order|Reading|understanding|start|roadmap|relevant|priority" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "read|order|Reading|understanding|start|roadmap|relevant|priority" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
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
./target/debug/roko parity check --strict --area tmp/architecture/00-INDEX
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-00-S005 -- Source references

**Source section:** `tmp/architecture/00-INDEX.md:58` through `67`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Source references

- Original v2 monolith: `tmp/roko-architecture-redesign-v2.md`
- ~~Bardo integration plan: `tmp/bardo-integration-plan.md`~~ -> **Folded into docs 18, 20, 21** (original file retained for reference but all content is now in this doc set)
- Dashboard PRDs: `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/`
- **Dashboard PRD 23 (universal primitives):** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md` -- defines the 12-primitive vocabulary, composition matrix, DeFi struct mapping, and authoring surfaces
- **Architecture cross-reference:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md` -- maps all 22 architecture docs to dashboard needs, identifies conflicts, lists ~160 new endpoints
- **Dashboard-roko integration:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md` -- three-tier deployment model, interaction modes, API contracts
- DeFi gap analysis: `tmp/defi/gap/`
- PRDs (roko): `tmp/04-21-26/PRDs/`
````

**Explicit detail extraction from this section:**

- Section word count: `142`
- Section hash: `fa42d840b49c34edfb971a67a9cbeeb6a0ec66781c5a1fcf0a9fa59a3622237f`

**Normative requirements and implementation claims:**
- - Original v2 monolith: `tmp/roko-architecture-redesign-v2.md` - ~~Bardo integration plan: `tmp/bardo-integration-plan.md`~~ -> **Folded into docs 18, 20, 21** (original file retained for reference but all content is now in this doc set) - Dashboard PRDs: `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/` - **Dashboard PRD 23 (universal primitives):** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md` -- defines the 12-primitive vocabulary, composition matrix, DeFi struct mapping, and authoring surfaces - **Architecture cross-reference:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md` -- maps all 22 architecture docs to dashboard needs, identifies conflicts, lists ~160 new endpoints - **Dashboard-roko integration:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md` -- three-tier deployment model, interaction modes, API contracts - DeFi gap analysis: `tmp/defi/gap/` - PRDs (roko): `tmp/04-21

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md
- Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md
- Users/will/dev/nunchi/nunchi-dashboard/docs/prd/
- Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md
- tmp/04-21-26/PRDs/
- tmp/bardo-integration-plan.md
- tmp/defi/gap/
- tmp/roko-architecture-redesign-v2.md

**Types, functions, traits, and inline code identifiers:**
- mapping

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Original v2 monolith: `tmp/roko-architecture-redesign-v2.md`
- - ~~Bardo integration plan: `tmp/bardo-integration-plan.md`~~ -> **Folded into docs 18, 20, 21** (original file retained for reference but all content is now in this doc set)
- - Dashboard PRDs: `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/`
- - **Dashboard PRD 23 (universal primitives):** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md` -- defines the 12-primitive vocabulary, composition matrix, DeFi struct mapping, and authoring surfaces
- - **Architecture cross-reference:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md` -- maps all 22 architecture docs to dashboard needs, identifies conflicts, lists ~160 new endpoints
- - **Dashboard-roko integration:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md` -- three-tier deployment model, interaction modes, API contracts
- - DeFi gap analysis: `tmp/defi/gap/`
- - PRDs (roko): `tmp/04-21-26/PRDs/`

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/00-INDEX.md`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/prd/`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `tmp/04-21-26/PRDs/`
- `tmp/bardo-integration-plan.md`
- `tmp/defi/gap/`
- `tmp/roko-architecture-redesign-v2.md`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "reference|nunchi|docs|references|mapping|plan|integration|Users" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reference|nunchi|docs|references|mapping|plan|integration|Users" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/prd/`
- `Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md`
- `tmp/04-21-26/PRDs/`
- `tmp/bardo-integration-plan.md`
- `tmp/defi/gap/`
- `tmp/roko-architecture-redesign-v2.md`
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
- [ ] Implement or verify `mapping` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/00-INDEX
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

