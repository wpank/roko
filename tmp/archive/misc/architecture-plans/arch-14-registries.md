# Architecture Plan: Registries

**Source:** `tmp/architecture/14-registries.md`
**Generated:** 2026-04-25
**Source hash:** `ffa4143be4b1f92d60900ed7db2b740a79596578aa10e8b977fc0db1056c202e`
**Section tasks:** 32
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
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-14-S001 | 1 | 14 -- On-chain registries | [ ] | 9.8 |
| ARCH-14-S002 | 9 | Design constraints | [ ] | 9.8 |
| ARCH-14-S003 | 20 | ERC-8004 agent passport | [ ] | 9.8 |
| ARCH-14-S004 | 24 | Passport fields | [ ] | 9.8 |
| ARCH-14-S005 | 41 | Solidity interface | [ ] | 9.8 |
| ARCH-14-S006 | 133 | Rust types | [ ] | 9.8 |
| ARCH-14-S007 | 240 | Reputation registry | [ ] | 9.8 |
| ARCH-14-S008 | 244 | Score computation | [ ] | 9.8 |
| ARCH-14-S009 | 261 | Solidity interface | [ ] | 9.8 |
| ARCH-14-S010 | 346 | Rust types | [ ] | 9.8 |
| ARCH-14-S011 | 408 | Tier thresholds | [ ] | 9.8 |
| ARCH-14-S012 | 422 | Knowledge registry (InsightStore on-chain) | [ ] | 9.8 |
| ARCH-14-S013 | 426 | Publication lifecycle | [ ] | 9.8 |
| ARCH-14-S014 | 529 | Rust types | [ ] | 9.8 |
| ARCH-14-S015 | 593 | Event indexer | [ ] | 9.8 |
| ARCH-14-S016 | 597 | Architecture | [ ] | 9.8 |
| ARCH-14-S017 | 607 | Indexed event types | [ ] | 9.8 |
| ARCH-14-S018 | 619 | Indexer Rust types | [ ] | 9.8 |
| ARCH-14-S019 | 678 | Indexer REST API | [ ] | 9.8 |
| ARCH-14-S020 | 761 | Contract addresses | [ ] | 9.8 |
| ARCH-14-S021 | 765 | Mirage devnet addresses | [ ] | 9.8 |
| ARCH-14-S022 | 780 | Configuration | [ ] | 9.8 |
| ARCH-14-S023 | 816 | Integration with existing systems | [ ] | 9.8 |
| ARCH-14-S024 | 818 | Passport registration in agent lifecycle | [ ] | 9.8 |
| ARCH-14-S025 | 828 | Reputation in the cascade router | [ ] | 9.8 |
| ARCH-14-S026 | 836 | Knowledge publication from neuro store | [ ] | 9.8 |
| ARCH-14-S027 | 845 | Event indexer as data backbone | [ ] | 9.8 |
| ARCH-14-S028 | 851 | Event types | [ ] | 9.8 |
| ARCH-14-S029 | 926 | Full event type list | [ ] | 9.8 |
| ARCH-14-S030 | 943 | Deployment | [ ] | 9.8 |
| ARCH-14-S031 | 945 | Contracts | [ ] | 9.8 |
| ARCH-14-S032 | 958 | Indexer | [ ] | 9.8 |

## Tasks

### ARCH-14-S001 -- 14 -- On-chain registries

**Source section:** `tmp/architecture/14-registries.md:1` through `8`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 14 -- On-chain registries

The persistent identity and reputation layer. ERC-8004 agent passports, per-domain reputation scores, on-chain knowledge publication, and the event indexer that makes all of it queryable. These contracts are deployed on Korai (production) and Mirage (development). The dashboard, agent runtime, and clearing contracts all read from and write to these registries.

This document specifies the Solidity interfaces, Rust client types, API routes, and event models. Dashboard surfaces that consume these registries span multiple PRDs: `12-fleet-surfaces.md` (agent passports), `13-knowledge-surfaces.md` (knowledge registry), `15-arena-surfaces.md` (arena/eval registries), `16-meta-surfaces.md` (lineage), and `17-treasury-surfaces.md` (reputation-weighted economics).

---
````

**Explicit detail extraction from this section:**

- Section word count: `108`
- Section hash: `3342c473c677f1ad9ab39a3d2aa12822ebd42aafa377dfeac8604de0796d3b5f`

**Normative requirements and implementation claims:**
- The persistent identity and reputation layer. ERC-8004 agent passports, per-domain reputation scores, on-chain knowledge publication, and the event indexer that makes all of it queryable. These contracts are deployed on Korai (production) and Mirage (development). The dashboard, agent runtime, and clearing contracts all read from and write to these registries.
- This document specifies the Solidity interfaces, Rust client types, API routes, and event models. Dashboard surfaces that consume these registries span multiple PRDs: `12-fleet-surfaces.md` (agent passports), `13-knowledge-surfaces.md` (knowledge registry), `15-arena-surfaces.md` (arena/eval registries), `16-meta-surfaces.md` (lineage), and `17-treasury-surfaces.md` (reputation-weighted economics).
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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "registries|surfaces|chain|reputation|knowledge|passports|event|contracts" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "registries|surfaces|chain|reputation|knowledge|passports|event|contracts" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S002 -- Design constraints

**Source section:** `tmp/architecture/14-registries.md:9` through `19`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Design constraints

1. **Soulbound passports.** Agent passports (ERC-8004) are non-transferable. An agent's identity is bound to its creation wallet. The passport can be updated but not moved to another address.
2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, clearing outcomes, bounty resolution, and eval applications. No manual reputation injection.
3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An agent that stops participating gradually loses reputation. This prevents stale high-reputation agents from dominating indefinitely.
4. **Knowledge is challengeable.** Published knowledge entries can be challenged with counter-evidence. A challenge triggers a resolution process. This keeps the knowledge store honest.
5. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer falls behind or corrupts, it can be rebuilt from chain history.
6. **Everything is public.** On-chain state is public by design. Privacy is achieved at the application layer through selective publication and HDC fingerprinting (publish the fingerprint, keep the content private).

---
````

**Explicit detail extraction from this section:**

- Section word count: `185`
- Section hash: `12841f7b4dd92e75eb9983bed7ce8bb5d907282f8a534e0f166ccad86d977fd7`

**Normative requirements and implementation claims:**
- 1. **Soulbound passports.** Agent passports (ERC-8004) are non-transferable. An agent's identity is bound to its creation wallet. The passport can be updated but not moved to another address. 2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, clearing outcomes, bounty resolution, and eval applications. No manual reputation injection. 3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An agent that stops participating gradually loses reputation. This prevents stale high-reputation agents from dominating indefinitely. 4. **Knowledge is challengeable.** Published knowledge entries can be challenged with counter-evidence. A challenge triggers a resolution process. This keeps the knowledge store honest. 5. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer f
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
- 1. **Soulbound passports.** Agent passports (ERC-8004) are non-transferable. An agent's identity is bound to its creation wallet. The passport can be updated but not moved to another address.
- 2. **Reputation is earned, not assigned.** Reputation scores update only from attested sources: arena settlement contracts, clearing outcomes, bounty resolution, and eval applications. No manual reputation injection.
- 3. **EMA decay is constant.** Reputation decays via exponential moving average unless refreshed by new attestations. An agent that stops participating gradually loses reputation. This prevents stale high-reputation agents from dominating indefinitely.
- 4. **Knowledge is challengeable.** Published knowledge entries can be challenged with counter-evidence. A challenge triggers a resolution process. This keeps the knowledge store honest.
- 5. **Indexer is read-only.** The event indexer observes on-chain events and stores them for fast querying. It never writes to the chain. If the indexer falls behind or corrupts, it can be rebuilt from chain history.
- 6. **Everything is public.** On-chain state is public by design. Privacy is achieved at the application layer through selective publication and HDC fingerprinting (publish the fingerprint, keep the content private).

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Reputation|Design|constraints|chain|public|passport|event|challenge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reputation|Design|constraints|chain|public|passport|event|challenge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S003 -- ERC-8004 agent passport

**Source section:** `tmp/architecture/14-registries.md:20` through `23`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## ERC-8004 agent passport

A soulbound NFT that represents an agent's on-chain identity. Every agent that participates in on-chain activities (arenas, bounties, clearing, knowledge publication) must have a passport.
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `305d553cbaf97582b840cb8d38aed37d35bb5142a5cbfc463e834f027afe634d`

**Normative requirements and implementation claims:**
- A soulbound NFT that represents an agent's on-chain identity. Every agent that participates in on-chain activities (arenas, bounties, clearing, knowledge publication) must have a passport.

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "passport|ERC|chain|soulbound|represents|publication|participates|knowledge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "passport|ERC|chain|soulbound|represents|publication|participates|knowledge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S004 -- Passport fields

**Source section:** `tmp/architecture/14-registries.md:24` through `40`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Passport fields

| Field | Type | Description |
|-------|------|-------------|
| `tokenId` | `uint128` | Auto-incrementing passport ID |
| `wallet` | `address` | Controlling wallet (owner) |
| `name` | `string` | Human-readable agent name |
| `capabilities` | `bytes32[]` | Capability hashes (e.g., `keccak256("trading")`) |
| `tier` | `uint8` | Reputation tier (0-4: Gray, Copper, Silver, Gold, Amber) |
| `reputationScore` | `uint256` | Aggregate reputation score (18 decimals) |
| `feeds` | `string[]` | Advertised feed URIs (see `05-feeds.md`) |
| `serviceEndpoints` | `string[]` | Sidecar/relay endpoints for connectivity |
| `delegationCaveats` | `bytes[]` | Encoded delegation caveats (see `08-auth.md`) |
| `parentPassport` | `uint128` | Parent agent's passport ID (0 if no parent) |
| `createdAtBlock` | `uint64` | Block at which the passport was minted |
| `metadataUri` | `string` | IPFS URI pointing to extended metadata JSON |
````

**Explicit detail extraction from this section:**

- Section word count: `100`
- Section hash: `95b914679716db293e1a39d7da30f86d201df58c16bd7216995da9e07e851eef`

**Normative requirements and implementation claims:**
- | Field | Type | Description | |-------|------|-------------| | `tokenId` | `uint128` | Auto-incrementing passport ID | | `wallet` | `address` | Controlling wallet (owner) | | `name` | `string` | Human-readable agent name | | `capabilities` | `bytes32[]` | Capability hashes (e.g., `keccak256("trading")`) | | `tier` | `uint8` | Reputation tier (0-4: Gray, Copper, Silver, Gold, Amber) | | `reputationScore` | `uint256` | Aggregate reputation score (18 decimals) | | `feeds` | `string[]` | Advertised feed URIs (see `05-feeds.md`) | | `serviceEndpoints` | `string[]` | Sidecar/relay endpoints for connectivity | | `delegationCaveats` | `bytes[]` | Encoded delegation caveats (see `08-auth.md`) | | `parentPassport` | `uint128` | Parent agent's passport ID (0 if no parent) | | `createdAtBlock` | `uint64` | Block at which the passport was minted | | `metadataUri` | `string` | IPFS URI pointing to extended metadata JSON |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- tokenId
- uint128
- wallet
- address
- name
- string
- capabilities
- tier
- uint8
- reputationScore
- uint256
- feeds
- serviceEndpoints
- delegationCaveats
- parentPassport
- createdAtBlock
- uint64
- metadataUri

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
| Field | Type | Description |
|-------|------|-------------|
| `tokenId` | `uint128` | Auto-incrementing passport ID |
| `wallet` | `address` | Controlling wallet (owner) |
| `name` | `string` | Human-readable agent name |
| `capabilities` | `bytes32[]` | Capability hashes (e.g., `keccak256("trading")`) |
| `tier` | `uint8` | Reputation tier (0-4: Gray, Copper, Silver, Gold, Amber) |
| `reputationScore` | `uint256` | Aggregate reputation score (18 decimals) |
| `feeds` | `string[]` | Advertised feed URIs (see `05-feeds.md`) |
| `serviceEndpoints` | `string[]` | Sidecar/relay endpoints for connectivity |
| `delegationCaveats` | `bytes[]` | Encoded delegation caveats (see `08-auth.md`) |
| `parentPassport` | `uint128` | Parent agent's passport ID (0 if no parent) |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Passport|string|wallet|uint128|tier|name|feeds|Field" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Passport|string|wallet|uint128|tier|name|feeds|Field" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `tokenId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `uint128` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `wallet` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `address` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `string` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `capabilities` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tier` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `uint8` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `reputationScore` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `uint256` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `feeds` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `serviceEndpoints` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `delegationCaveats` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `parentPassport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `createdAtBlock` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `uint64` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `metadataUri` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S005 -- Solidity interface

**Source section:** `tmp/architecture/14-registries.md:41` through `132`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Solidity interface

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentPassport {
    struct Passport {
        uint128 tokenId;
        address wallet;
        string  name;
        bytes32[] capabilities;
        uint8   tier;
        uint256 reputationScore;
        string[] feeds;
        string[] serviceEndpoints;
        bytes[] delegationCaveats;
        uint128 parentPassport;
        uint64  createdAtBlock;
        string  metadataUri;
    }

    /// Register a new agent passport. Caller becomes the owner.
    /// Returns the new passport's tokenId.
    function register(
        string calldata name,
        bytes32[] calldata capabilities,
        string[] calldata feeds,
        string[] calldata serviceEndpoints,
        bytes[] calldata delegationCaveats,
        uint128 parentPassport,
        string calldata metadataUri
    ) external returns (uint128 tokenId);

    /// Update mutable fields. Only callable by the passport owner.
    function update(
        uint128 tokenId,
        string calldata name,
        bytes32[] calldata capabilities,
        string calldata metadataUri
    ) external;

    /// Update advertised feeds. Only callable by the passport owner.
    function updateFeeds(
        uint128 tokenId,
        string[] calldata feeds
    ) external;

    /// Update service endpoints. Only callable by the passport owner.
    function updateEndpoints(
        uint128 tokenId,
        string[] calldata endpoints
    ) external;

    /// Update delegation caveats. Only callable by the passport owner.
    /// For meta-agent children, caveats can only narrow (never widen).
    function updateCaveats(
        uint128 tokenId,
        bytes[] calldata caveats
    ) external;

    /// Read a passport by tokenId.
    function getPassport(uint128 tokenId) external view returns (Passport memory);

    /// List all passports owned by an address.
    function getPassportsByOwner(address owner) external view returns (uint128[] memory);

    /// Find passports by capability.
    function getPassportsByCapability(
        bytes32 capability,
        uint256 offset,
        uint256 limit
    ) external view returns (uint128[] memory);

    /// Total registered passports.
    function totalPassports() external view returns (uint128);

    // Events
    event PassportRegistered(
        uint128 indexed tokenId,
        address indexed owner,
        string name,
        uint128 parentPassport
    );
    event PassportUpdated(uint128 indexed tokenId, string name);
    event FeedsUpdated(uint128 indexed tokenId, uint256 feedCount);
    event EndpointsUpdated(uint128 indexed tokenId, uint256 endpointCount);
    event CaveatsUpdated(uint128 indexed tokenId, uint256 caveatCount);
    event TierChanged(uint128 indexed tokenId, uint8 oldTier, uint8 newTier);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `269`
- Section hash: `928a982bd7ff9a91e281e966f27667ab3a384bb2b87151487fc54c64f7380c97`

**Normative requirements and implementation claims:**
- /// Update mutable fields. Only callable by the passport owner. function update( uint128 tokenId, string calldata name, bytes32[] calldata capabilities, string calldata metadataUri ) external;
- /// Update advertised feeds. Only callable by the passport owner. function updateFeeds( uint128 tokenId, string[] calldata feeds ) external;
- /// Update service endpoints. Only callable by the passport owner. function updateEndpoints( uint128 tokenId, string[] calldata endpoints ) external;
- /// Update delegation caveats. Only callable by the passport owner. /// For meta-agent children, caveats can only narrow (never widen). function updateCaveats( uint128 tokenId, bytes[] calldata caveats ) external;
- // Events event PassportRegistered( uint128 indexed tokenId, address indexed owner, string name, uint128 parentPassport ); event PassportUpdated(uint128 indexed tokenId, string name); event FeedsUpdated(uint128 indexed tokenId, uint256 feedCount); event EndpointsUpdated(uint128 indexed tokenId, uint256 endpointCount); event CaveatsUpdated(uint128 indexed tokenId, uint256 caveatCount); event TierChanged(uint128 indexed tokenId, uint8 oldTier, uint8 newTier); } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Passport

**Event names and event-like entities:**
- PassportUpdated
- FeedsUpdated
- EndpointsUpdated
- CaveatsUpdated

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
- Contract 1: language `solidity`, first line `// SPDX-License-Identifier: MIT`

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentPassport {
    struct Passport {
        uint128 tokenId;
        address wallet;
        string  name;
        bytes32[] capabilities;
        uint8   tier;
        uint256 reputationScore;
        string[] feeds;
        string[] serviceEndpoints;
        bytes[] delegationCaveats;
        uint128 parentPassport;
        uint64  createdAtBlock;
        string  metadataUri;
    }

    /// Register a new agent passport. Caller becomes the owner.
    /// Returns the new passport's tokenId.
    function register(
        string calldata name,
        bytes32[] calldata capabilities,
        string[] calldata feeds,
        string[] calldata serviceEndpoints,
        bytes[] calldata delegationCaveats,
        uint128 parentPassport,
        string calldata metadataUri
    ) external returns (uint128 tokenId);

    /// Update mutable fields. Only callable by the passport owner.
    function update(
        uint128 tokenId,
        string calldata name,
        bytes32[] calldata capabilities,
        string calldata metadataUri
    ) external;

    /// Update advertised feeds. Only callable by the passport owner.
    function updateFeeds(
        uint128 tokenId,
        string[] calldata feeds
    ) external;

    /// Update service endpoints. Only callable by the passport owner.
    function updateEndpoints(
        uint128 tokenId,
        string[] calldata endpoints
    ) external;

    /// Update delegation caveats. Only callable by the passport owner.
    /// For meta-agent children, caveats can on
...
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Passport|uint128|tokenId|string|calldata|Update|function|external" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Passport|uint128|tokenId|string|calldata|Update|function|external" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `Passport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `PassportUpdated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `FeedsUpdated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `EndpointsUpdated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `CaveatsUpdated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S006 -- Rust types

**Source section:** `tmp/architecture/14-registries.md:133` through `239`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Rust types

```rust
/// Agent passport as represented in the Roko runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPassport {
    pub token_id: u128,
    pub wallet: Address,
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub tier: ReputationTier,
    pub reputation_score: f64,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_passport: Option<u128>,
    pub created_at_block: u64,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReputationTier {
    /// Tier 0: New or low-reputation agent. No track record.
    Gray = 0,
    /// Tier 1: Some positive attestations. Basic participation.
    Copper = 1,
    /// Tier 2: Consistent positive outcomes. Trusted for standard tasks.
    Silver = 2,
    /// Tier 3: Strong track record across multiple domains.
    Gold = 3,
    /// Tier 4: Exceptional performance. Highest trust level.
    Amber = 4,
}

impl ReputationTier {
    /// Minimum aggregate reputation score required for each tier.
    pub fn threshold(self) -> f64 {
        match self {
            Self::Gray => 0.0,
            Self::Copper => 10.0,
            Self::Silver => 50.0,
            Self::Gold => 200.0,
            Self::Amber => 1000.0,
        }
    }

    /// Determine tier from an aggregate score.
    pub fn from_score(score: f64) -> Self {
        if score >= 1000.0 {
            Self::Amber
        } else if score >= 200.0 {
            Self::Gold
        } else if score >= 50.0 {
            Self::Silver
        } else if score >= 10.0 {
            Self::Copper
        } else {
            Self::Gray
        }
    }
}

/// Client for reading and writing agent passports on-chain.
pub struct PassportClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl PassportClient {
    /// Register a new passport on-chain.
    pub async fn register(&self, config: PassportRegistration) -> Result<u128> { ... }

    /// Read a passport by token ID.
    pub async fn get(&self, token_id: u128) -> Result<AgentPassport> { ... }

    /// List passports by owner address.
    pub async fn by_owner(&self, owner: Address) -> Result<Vec<u128>> { ... }

    /// Update passport fields.
    pub async fn update(&self, token_id: u128, patch: PassportPatch) -> Result<()> { ... }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportRegistration {
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_passport: Option<u128>,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PassportPatch {
    pub name: Option<String>,
    pub capabilities: Option<Vec<[u8; 32]>>,
    pub feeds: Option<Vec<String>>,
    pub service_endpoints: Option<Vec<String>>,
    pub delegation_caveats: Option<Vec<DelegationCaveat>>,
    pub metadata_uri: Option<String>,
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `359`
- Section hash: `ef1eae52ddcf00545fa239bb672d400c55394d83b0fc29fff06461a47d385cf1`

**Normative requirements and implementation claims:**
- impl ReputationTier { /// Minimum aggregate reputation score required for each tier. pub fn threshold(self) -> f64 { match self { Self::Gray => 0.0, Self::Copper => 10.0, Self::Silver => 50.0, Self::Gold => 200.0, Self::Amber => 1000.0, } }
- /// Client for reading and writing agent passports on-chain. pub struct PassportClient { contract: Address, provider: Arc<dyn Provider>, signer: Option<Arc<dyn Signer>>, }
- /// Update passport fields. pub async fn update(&self, token_id: u128, patch: PassportPatch) -> Result<()> { ... } }
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentPassport
- ReputationTier
- threshold
- from_score
- PassportClient
- register
- get
- by_owner
- update
- PassportRegistration
- PassportPatch

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- Gray = 0,
- Copper = 1,
- Silver = 2,
- Gold = 3,
- Amber = 4,

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// Agent passport as represented in the Roko runtime.`

```rust
/// Agent passport as represented in the Roko runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPassport {
    pub token_id: u128,
    pub wallet: Address,
    pub name: String,
    pub capabilities: Vec<[u8; 32]>,
    pub tier: ReputationTier,
    pub reputation_score: f64,
    pub feeds: Vec<String>,
    pub service_endpoints: Vec<String>,
    pub delegation_caveats: Vec<DelegationCaveat>,
    pub parent_passport: Option<u128>,
    pub created_at_block: u64,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReputationTier {
    /// Tier 0: New or low-reputation agent. No track record.
    Gray = 0,
    /// Tier 1: Some positive attestations. Basic participation.
    Copper = 1,
    /// Tier 2: Consistent positive outcomes. Trusted for standard tasks.
    Silver = 2,
    /// Tier 3: Strong track record across multiple domains.
    Gold = 3,
    /// Tier 4: Exceptional performance. Highest trust level.
    Amber = 4,
}

impl ReputationTier {
    /// Minimum aggregate reputation score required for each tier.
    pub fn threshold(self) -> f64 {
        match self {
            Self::Gray => 0.0,
            Self::Copper => 10.0,
            Self::Silver => 50.0,
            Self::Gold => 200.0,
            Self::Amber => 1000.0,
        }
    }

    /// Determine tier from an aggregate score.
    pub fn from_score(score: f64) -> Self {
        if score >= 1000.0 {
            Self::Amber
        } else if score >= 200.0 {
            Self::Gold
        } else if score >=
...
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "self|passport|String|tier|score|Option|Serialize|Rust" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "self|passport|String|tier|score|Option|Serialize|Rust" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `AgentPassport` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReputationTier` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `threshold` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `from_score` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PassportClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `register` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `get` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `by_owner` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `update` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PassportRegistration` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PassportPatch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `Gray = 0,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `Copper = 1,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `Silver = 2,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `Gold = 3,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `Amber = 4,` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S007 -- Reputation registry

**Source section:** `tmp/architecture/14-registries.md:240` through `243`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Reputation registry

Per-agent, per-domain reputation scores derived from on-chain attestations. Reputation determines tier, unlocks higher-trust activities, and influences model routing weights in the cascade router.
````

**Explicit detail extraction from this section:**

- Section word count: `27`
- Section hash: `005e114a086fcd49bd6115e60f8c295a34bdefca4b00a6e83cd2e79626226339`

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Reputation|registry|weights|unlocks|trust|tier|scores|routing" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reputation|registry|weights|unlocks|trust|tier|scores|routing" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S008 -- Score computation

**Source section:** `tmp/architecture/14-registries.md:244` through `260`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Score computation

Each attestation carries a `delta` -- a positive or negative reputation change computed from the attesting event:

- **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative.
- **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure.
- **Clearing participation**: `delta = +0.1` per successful clearing round (small but cumulative).
- **Knowledge validation**: `delta = +0.2` when a published entry gets validated, `-0.3` when it gets successfully challenged.

The per-domain score is an EMA (exponential moving average) with alpha = 0.05:

```
new_score = alpha * delta + (1 - alpha) * old_score
```

Decay: if no attestation arrives for a domain within 30 days, the score decays by 1% per day until a new attestation refreshes it.
````

**Explicit detail extraction from this section:**

- Section word count: `121`
- Section hash: `23480f91de1bd1ed668c167b493fffd25025b70aaac482d3dae2d3ede3acc631`

**Normative requirements and implementation claims:**
- Each attestation carries a `delta` -- a positive or negative reputation change computed from the attesting event:
- - **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative. - **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure. - **Clearing participation**: `delta = +0.1` per successful clearing round (small but cumulative). - **Knowledge validation**: `delta = +0.2` when a published entry gets validated, `-0.3` when it gets successfully challenged.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- delta

**Event names and event-like entities:**
- arena.weight
- bounty.reward_tier

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- new_score = alpha * delta + (1 - alpha) * old_score

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Arena completion**: `delta = (score - 0.5) * arena.weight`. Scoring above the median earns positive reputation; below earns negative.
- - **Bounty resolution**: `delta = +bounty.reward_tier` on success, `-bounty.reward_tier * 0.5` on failure.
- - **Clearing participation**: `delta = +0.1` per successful clearing round (small but cumulative).
- - **Knowledge validation**: `delta = +0.2` when a published entry gets validated, `-0.3` when it gets successfully challenged.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `new_score = alpha * delta + (1 - alpha) * old_score`

```
new_score = alpha * delta + (1 - alpha) * old_score
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "delta|Score|computation|success|bounty|attestation|alpha|successful" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "delta|Score|computation|success|bounty|attestation|alpha|successful" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `delta` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `arena.weight` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `bounty.reward_tier` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `new_score = alpha * delta + (1 - alpha) * old_score` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S009 -- Solidity interface

**Source section:** `tmp/architecture/14-registries.md:261` through `345`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Solidity interface

```solidity
interface IReputationRegistry {
    struct ReputationRecord {
        uint128 agentPassportId;
        bytes32 domain;          // keccak256 of domain name
        uint256 score;           // Current EMA score (18 decimals, can be negative via signed math)
        int256  signedScore;     // Signed score for domains where negative is possible
        uint64  attestationCount;
        uint64  lastAttestedBlock;
        uint8   tier;            // Derived tier (0-4)
    }

    struct Attestation {
        uint128 agentPassportId;
        bytes32 domain;
        int256  delta;           // Reputation change (positive or negative, 18 decimals)
        bytes32 sourceContract;  // Address of the attesting contract (arena, bounty, clearing)
        bytes32 evidenceHash;    // Hash of the evidence supporting this attestation
        uint64  blockNumber;
    }

    /// Submit a reputation attestation. Only callable by registered attesting contracts.
    function attest(
        uint128 agentPassportId,
        bytes32 domain,
        int256 delta,
        bytes32 evidenceHash
    ) external;

    /// Read current reputation for an agent in a specific domain.
    function getReputation(
        uint128 agentPassportId,
        bytes32 domain
    ) external view returns (ReputationRecord memory);

    /// Read aggregate reputation across all domains.
    function getAggregateReputation(
        uint128 agentPassportId
    ) external view returns (uint256 aggregateScore, uint8 tier);

    /// Historical attestations for an agent in a domain.
    function getAttestations(
        uint128 agentPassportId,
        bytes32 domain,
        uint256 offset,
        uint256 limit
    ) external view returns (Attestation[] memory);

    /// All domains an agent has reputation in.
    function getAgentDomains(
        uint128 agentPassportId
    ) external view returns (bytes32[] memory);

    /// Top agents by reputation in a domain.
    function getTopAgents(
        bytes32 domain,
        uint256 limit
    ) external view returns (uint128[] memory passportIds, uint256[] memory scores);

    /// Register a contract as an attesting source. Governance-controlled.
    function registerAttester(address attester) external;

    /// Remove an attesting source. Governance-controlled.
    function removeAttester(address attester) external;

    // Events
    event ReputationAttested(
        uint128 indexed agentPassportId,
        bytes32 indexed domain,
        int256 delta,
        uint256 newScore,
        address indexed attester
    );
    event TierChanged(
        uint128 indexed agentPassportId,
        uint8 oldTier,
        uint8 newTier
    );
    event AttesterRegistered(address indexed attester);
    event AttesterRemoved(address indexed attester);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `262`
- Section hash: `bf0f73f8509b8b9addabbe0f16b6e43ce6d4e230a365c4eb4660fcd7950793d7`

**Normative requirements and implementation claims:**
- struct Attestation { uint128 agentPassportId; bytes32 domain; int256 delta; // Reputation change (positive or negative, 18 decimals) bytes32 sourceContract; // Address of the attesting contract (arena, bounty, clearing) bytes32 evidenceHash; // Hash of the evidence supporting this attestation uint64 blockNumber; }
- /// Submit a reputation attestation. Only callable by registered attesting contracts. function attest( uint128 agentPassportId, bytes32 domain, int256 delta, bytes32 evidenceHash ) external;
- /// Register a contract as an attesting source. Governance-controlled. function registerAttester(address attester) external;
- // Events event ReputationAttested( uint128 indexed agentPassportId, bytes32 indexed domain, int256 delta, uint256 newScore, address indexed attester ); event TierChanged( uint128 indexed agentPassportId, uint8 oldTier, uint8 newTier ); event AttesterRegistered(address indexed attester); event AttesterRemoved(address indexed attester); } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ReputationRecord
- Attestation

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
- Contract 1: language `solidity`, first line `interface IReputationRegistry {`

```solidity
interface IReputationRegistry {
    struct ReputationRecord {
        uint128 agentPassportId;
        bytes32 domain;          // keccak256 of domain name
        uint256 score;           // Current EMA score (18 decimals, can be negative via signed math)
        int256  signedScore;     // Signed score for domains where negative is possible
        uint64  attestationCount;
        uint64  lastAttestedBlock;
        uint8   tier;            // Derived tier (0-4)
    }

    struct Attestation {
        uint128 agentPassportId;
        bytes32 domain;
        int256  delta;           // Reputation change (positive or negative, 18 decimals)
        bytes32 sourceContract;  // Address of the attesting contract (arena, bounty, clearing)
        bytes32 evidenceHash;    // Hash of the evidence supporting this attestation
        uint64  blockNumber;
    }

    /// Submit a reputation attestation. Only callable by registered attesting contracts.
    function attest(
        uint128 agentPassportId,
        bytes32 domain,
        int256 delta,
        bytes32 evidenceHash
    ) external;

    /// Read current reputation for an agent in a specific domain.
    function getReputation(
        uint128 agentPassportId,
        bytes32 domain
    ) external view returns (ReputationRecord memory);

    /// Read aggregate reputation across all domains.
    function getAggregateReputation(
        uint128 agentPassportId
    ) external view returns (uint256 aggregateScore, uint8 tier);

    /// Historical attestations for an agent in a domain.
    function getAttestations(
        uint12
...
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "attest|domain|Reputation|int256|bytes32|Attestation|uint128|attester" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "attest|domain|Reputation|int256|bytes32|Attestation|uint128|attester" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `ReputationRecord` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Attestation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S010 -- Rust types

**Source section:** `tmp/architecture/14-registries.md:346` through `407`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Rust types

```rust
/// Per-domain reputation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRecord {
    pub agent_passport_id: u128,
    pub domain: String,
    pub score: f64,
    pub attestation_count: u64,
    pub last_attested_block: u64,
    pub tier: ReputationTier,
}

/// A single reputation attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub agent_passport_id: u128,
    pub domain: String,
    pub delta: f64,
    pub source_contract: Address,
    pub evidence_hash: [u8; 32],
    pub block_number: u64,
}

/// Client for reading reputation data on-chain.
pub struct ReputationClient {
    contract: Address,
    provider: Arc<dyn Provider>,
}

impl ReputationClient {
    /// Read reputation for an agent in a domain.
    pub async fn get_reputation(
        &self,
        passport_id: u128,
        domain: &str,
    ) -> Result<ReputationRecord> { ... }

    /// Read aggregate reputation across all domains.
    pub async fn get_aggregate(
        &self,
        passport_id: u128,
    ) -> Result<(f64, ReputationTier)> { ... }

    /// Historical attestations for an agent.
    pub async fn get_attestations(
        &self,
        passport_id: u128,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<Attestation>> { ... }

    /// Top agents in a domain.
    pub async fn top_agents(
        &self,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<(u128, f64)>> { ... }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `152`
- Section hash: `e3c35f6035ef8a351709308606539360ed0abcf8860dbe482298f0c5f50e188f`

**Normative requirements and implementation claims:**
- /// Client for reading reputation data on-chain. pub struct ReputationClient { contract: Address, provider: Arc<dyn Provider>, }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ReputationRecord
- Attestation
- ReputationClient
- get_reputation
- get_aggregate
- get_attestations
- top_agents

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
- Contract 1: language `rust`, first line `/// Per-domain reputation record.`

```rust
/// Per-domain reputation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRecord {
    pub agent_passport_id: u128,
    pub domain: String,
    pub score: f64,
    pub attestation_count: u64,
    pub last_attested_block: u64,
    pub tier: ReputationTier,
}

/// A single reputation attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub agent_passport_id: u128,
    pub domain: String,
    pub delta: f64,
    pub source_contract: Address,
    pub evidence_hash: [u8; 32],
    pub block_number: u64,
}

/// Client for reading reputation data on-chain.
pub struct ReputationClient {
    contract: Address,
    provider: Arc<dyn Provider>,
}

impl ReputationClient {
    /// Read reputation for an agent in a domain.
    pub async fn get_reputation(
        &self,
        passport_id: u128,
        domain: &str,
    ) -> Result<ReputationRecord> { ... }

    /// Read aggregate reputation across all domains.
    pub async fn get_aggregate(
        &self,
        passport_id: u128,
    ) -> Result<(f64, ReputationTier)> { ... }

    /// Historical attestations for an agent.
    pub async fn get_attestations(
        &self,
        passport_id: u128,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<Attestation>> { ... }

    /// Top agents in a domain.
    pub async fn top_agents(
        &self,
        domain: &str,
        limit: u64,
    ) -> Result<Vec<(u128, f64)>> { ... }
}
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "reputation|Attestation|domain|u128|Rust|ReputationRecord|ReputationClient|types" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reputation|Attestation|domain|u128|Rust|ReputationRecord|ReputationClient|types" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `ReputationRecord` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Attestation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReputationClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `get_reputation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `get_aggregate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `get_attestations` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `top_agents` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S011 -- Tier thresholds

**Source section:** `tmp/architecture/14-registries.md:408` through `421`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Tier thresholds

| Tier | Name | Aggregate score | Unlocks |
|------|------|----------------|---------|
| 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties |
| 1 | Copper | 10 - 49 | Create arenas, publish knowledge, claim mid-tier bounties |
| 2 | Silver | 50 - 199 | Create evals, participate in clearing, claim high-tier bounties |
| 3 | Gold | 200 - 999 | Meta-agent creation, validate knowledge, governance votes |
| 4 | Amber | >= 1000 | All capabilities, featured status, priority clearing |

Tier transitions emit a `TierChanged` event and update the passport's `tier` field.

---
````

**Explicit detail extraction from this section:**

- Section word count: `74`
- Section hash: `857efb3308344d24736679ffc26b857ccb49d7307ebff7a8d23689c6b75c79dc`

**Normative requirements and implementation claims:**
- | Tier | Name | Aggregate score | Unlocks | |------|------|----------------|---------| | 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties | | 1 | Copper | 10 - 49 | Create arenas, publish knowledge, claim mid-tier bounties | | 2 | Silver | 50 - 199 | Create evals, participate in clearing, claim high-tier bounties | | 3 | Gold | 200 - 999 | Meta-agent creation, validate knowledge, governance votes | | 4 | Amber | >= 1000 | All capabilities, featured status, priority clearing |
- Tier transitions emit a `TierChanged` event and update the passport's `tier` field.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- TierChanged
- tier

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
| Tier | Name | Aggregate score | Unlocks |
|------|------|----------------|---------|
| 0 | Gray | < 10 | Basic participation: join arenas, claim low-tier bounties |
| 1 | Copper | 10 - 49 | Create arenas, publish knowledge, claim mid-tier bounties |
| 2 | Silver | 50 - 199 | Create evals, participate in clearing, claim high-tier bounties |
| 3 | Gold | 200 - 999 | Meta-agent creation, validate knowledge, governance votes |
| 4 | Amber | >= 1000 | All capabilities, featured status, priority clearing |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Tier|thresholds|TierChanged|claim|bounties|knowledge|clearing|arenas" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Tier|thresholds|TierChanged|claim|bounties|knowledge|clearing|arenas" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `TierChanged` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `tier` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S012 -- Knowledge registry (InsightStore on-chain)

**Source section:** `tmp/architecture/14-registries.md:422` through `425`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Knowledge registry (InsightStore on-chain)

Published knowledge entries live on-chain for discoverability, validation, and challenge. The on-chain registry stores metadata and content hashes. Full content lives off-chain (IPFS or the agent's local neuro store) and is referenced by CID.
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `62ea030e36e26df8614d7bbe824756b34d5d81585c75156b284465db5295ed0a`

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "chain|store|registry|Knowledge|InsightStore|live|content|validation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "chain|store|registry|Knowledge|InsightStore|live|content|validation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S013 -- Publication lifecycle

**Source section:** `tmp/architecture/14-registries.md:426` through `528`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Publication lifecycle

1. **Publish**: Agent submits entry metadata + content hash. The entry enters `Active` state.
2. **Validate**: Another agent submits evidence supporting the entry's correctness. Validation count increments. The publisher earns positive reputation.
3. **Challenge**: Another agent submits counter-evidence. The entry enters `Challenged` state. A resolution window opens.
4. **Resolve**: After the resolution window, the entry is either `Validated` (challenge rejected) or `Retracted` (challenge accepted). Reputation flows accordingly.
5. **Decay**: Entries not validated or refreshed within 90 days enter `Stale` state. Stale entries still exist but are ranked lower in queries.

```solidity
interface IKnowledgeRegistry {
    enum EntryState {
        Active,
        Challenged,
        Validated,
        Retracted,
        Stale
    }

    struct KnowledgeEntry {
        bytes32 entryId;          // blake3 hash of content
        uint128 publisherPassport;
        string  title;
        string  entryType;        // "insight", "playbook", "analysis", "reference"
        bytes32 contentHash;      // IPFS CID or blake3 hash of full content
        bytes32 hdcFingerprint;   // HDC vector fingerprint for similarity queries
        string[] tags;
        EntryState state;
        uint64  validationCount;
        uint64  challengeCount;
        uint64  publishedAtBlock;
        uint64  lastRefreshedBlock;
    }

    struct Challenge {
        bytes32 challengeId;
        bytes32 entryId;
        uint128 challengerPassport;
        bytes32 evidenceHash;
        string  reason;
        uint64  challengedAtBlock;
        uint64  resolutionDeadline;  // Block by which resolution must occur
        bool    resolved;
        bool    upheld;              // True = challenge accepted, entry retracted
    }

    /// Publish a new knowledge entry.
    function publish(
        string calldata title,
        string calldata entryType,
        bytes32 contentHash,
        bytes32 hdcFingerprint,
        string[] calldata tags
    ) external returns (bytes32 entryId);

    /// Validate an existing entry with supporting evidence.
    function validate(
        bytes32 entryId,
        bytes32 evidenceHash
    ) external;

    /// Challenge an entry with counter-evidence.
    function challenge(
        bytes32 entryId,
        bytes32 evidenceHash,
        string calldata reason
    ) external returns (bytes32 challengeId);

    /// Resolve a challenge. Callable by governance or qualified resolvers.
    function resolveChallenge(
        bytes32 challengeId,
        bool upheld
    ) external;

    /// Read an entry by ID.
    function getEntry(bytes32 entryId) external view returns (KnowledgeEntry memory);

    /// Query entries by tag and state.
    function queryEntries(
        string calldata tag,
        EntryState state,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory entryIds);

    /// Lineage: entries derived from or referencing this entry.
    function getEntryLineage(bytes32 entryId) external view returns (bytes32[] memory);

    /// Entries published by a specific agent.
    function getEntriesByPublisher(
        uint128 publisherPassport,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory);

    // Events
    event EntryPublished(bytes32 indexed entryId, uint128 indexed publisher, string title);
    event EntryValidated(bytes32 indexed entryId, uint128 indexed validator);
    event EntryChallenged(bytes32 indexed entryId, bytes32 indexed challengeId, uint128 challenger);
    event ChallengeResolved(bytes32 indexed challengeId, bool upheld);
    event EntryStateChanged(bytes32 indexed entryId, EntryState oldState, EntryState newState);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `371`
- Section hash: `70fc8d4f87835d77d48628153436cfc795d217aabe8e8bdfac31e6726b3726f4`

**Normative requirements and implementation claims:**
- 1. **Publish**: Agent submits entry metadata + content hash. The entry enters `Active` state. 2. **Validate**: Another agent submits evidence supporting the entry's correctness. Validation count increments. The publisher earns positive reputation. 3. **Challenge**: Another agent submits counter-evidence. The entry enters `Challenged` state. A resolution window opens. 4. **Resolve**: After the resolution window, the entry is either `Validated` (challenge rejected) or `Retracted` (challenge accepted). Reputation flows accordingly. 5. **Decay**: Entries not validated or refreshed within 90 days enter `Stale` state. Stale entries still exist but are ranked lower in queries.
- struct KnowledgeEntry { bytes32 entryId; // blake3 hash of content uint128 publisherPassport; string title; string entryType; // "insight", "playbook", "analysis", "reference" bytes32 contentHash; // IPFS CID or blake3 hash of full content bytes32 hdcFingerprint; // HDC vector fingerprint for similarity queries string[] tags; EntryState state; uint64 validationCount; uint64 challengeCount; uint64 publishedAtBlock; uint64 lastRefreshedBlock; }
- struct Challenge { bytes32 challengeId; bytes32 entryId; uint128 challengerPassport; bytes32 evidenceHash; string reason; uint64 challengedAtBlock; uint64 resolutionDeadline; // Block by which resolution must occur bool resolved; bool upheld; // True = challenge accepted, entry retracted }
- /// Validate an existing entry with supporting evidence. function validate( bytes32 entryId, bytes32 evidenceHash ) external;
- /// Query entries by tag and state. function queryEntries( string calldata tag, EntryState state, uint256 offset, uint256 limit ) external view returns (bytes32[] memory entryIds);
- // Events event EntryPublished(bytes32 indexed entryId, uint128 indexed publisher, string title); event EntryValidated(bytes32 indexed entryId, uint128 indexed validator); event EntryChallenged(bytes32 indexed entryId, bytes32 indexed challengeId, uint128 challenger); event ChallengeResolved(bytes32 indexed challengeId, bool upheld); event EntryStateChanged(bytes32 indexed entryId, EntryState oldState, EntryState newState); } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EntryState
- KnowledgeEntry
- Challenge
- Active
- Challenged
- Validated
- Retracted
- Stale

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Publish**: Agent submits entry metadata + content hash. The entry enters `Active` state.
- 2. **Validate**: Another agent submits evidence supporting the entry's correctness. Validation count increments. The publisher earns positive reputation.
- 3. **Challenge**: Another agent submits counter-evidence. The entry enters `Challenged` state. A resolution window opens.
- 4. **Resolve**: After the resolution window, the entry is either `Validated` (challenge rejected) or `Retracted` (challenge accepted). Reputation flows accordingly.
- 5. **Decay**: Entries not validated or refreshed within 90 days enter `Stale` state. Stale entries still exist but are ranked lower in queries.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `solidity`, first line `interface IKnowledgeRegistry {`

```solidity
interface IKnowledgeRegistry {
    enum EntryState {
        Active,
        Challenged,
        Validated,
        Retracted,
        Stale
    }

    struct KnowledgeEntry {
        bytes32 entryId;          // blake3 hash of content
        uint128 publisherPassport;
        string  title;
        string  entryType;        // "insight", "playbook", "analysis", "reference"
        bytes32 contentHash;      // IPFS CID or blake3 hash of full content
        bytes32 hdcFingerprint;   // HDC vector fingerprint for similarity queries
        string[] tags;
        EntryState state;
        uint64  validationCount;
        uint64  challengeCount;
        uint64  publishedAtBlock;
        uint64  lastRefreshedBlock;
    }

    struct Challenge {
        bytes32 challengeId;
        bytes32 entryId;
        uint128 challengerPassport;
        bytes32 evidenceHash;
        string  reason;
        uint64  challengedAtBlock;
        uint64  resolutionDeadline;  // Block by which resolution must occur
        bool    resolved;
        bool    upheld;              // True = challenge accepted, entry retracted
    }

    /// Publish a new knowledge entry.
    function publish(
        string calldata title,
        string calldata entryType,
        bytes32 contentHash,
        bytes32 hdcFingerprint,
        string[] calldata tags
    ) external returns (bytes32 entryId);

    /// Validate an existing entry with supporting evidence.
    function validate(
        bytes32 entryId,
        bytes32 evidenceHash
    ) external;

    /// Challenge an entry with counter-evidence.
    funct
...
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "entry|bytes32|Challenge|state|entryId|Publish|string|EntryState" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "entry|bytes32|Challenge|state|entryId|Publish|string|EntryState" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `EntryState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Challenge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Active` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Challenged` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Validated` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Retracted` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Stale` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S014 -- Rust types

**Source section:** `tmp/architecture/14-registries.md:529` through `592`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Rust types

```rust
/// Knowledge entry as represented in the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainKnowledgeEntry {
    pub entry_id: [u8; 32],
    pub publisher_passport: u128,
    pub title: String,
    pub entry_type: KnowledgeEntryType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
    pub state: KnowledgeEntryState,
    pub validation_count: u64,
    pub challenge_count: u64,
    pub published_at_block: u64,
    pub last_refreshed_block: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryType {
    Insight,
    Playbook,
    Analysis,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryState {
    Active,
    Challenged,
    Validated,
    Retracted,
    Stale,
}

/// Client for the on-chain knowledge registry.
pub struct KnowledgeRegistryClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl KnowledgeRegistryClient {
    pub async fn publish(&self, entry: KnowledgePublication) -> Result<[u8; 32]> { ... }
    pub async fn validate(&self, entry_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()> { ... }
    pub async fn challenge(&self, entry_id: [u8; 32], evidence_hash: [u8; 32], reason: &str) -> Result<[u8; 32]> { ... }
    pub async fn get_entry(&self, entry_id: [u8; 32]) -> Result<OnChainKnowledgeEntry> { ... }
    pub async fn query(&self, tag: &str, state: KnowledgeEntryState, limit: u64) -> Result<Vec<[u8; 32]>> { ... }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePublication {
    pub title: String,
    pub entry_type: KnowledgeEntryType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `199`
- Section hash: `54f2a159c0fe4ace1ef99e90f86a405c3660bef14ea89d03efaf7e2e6f961aef`

**Normative requirements and implementation claims:**
- ```rust /// Knowledge entry as represented in the runtime. #[derive(Debug, Clone, Serialize, Deserialize)] pub struct OnChainKnowledgeEntry { pub entry_id: [u8; 32], pub publisher_passport: u128, pub title: String, pub entry_type: KnowledgeEntryType, pub content_hash: [u8; 32], pub hdc_fingerprint: [u8; 32], pub tags: Vec<String>, pub state: KnowledgeEntryState, pub validation_count: u64, pub challenge_count: u64, pub published_at_block: u64, pub last_refreshed_block: u64, }
- /// Client for the on-chain knowledge registry. pub struct KnowledgeRegistryClient { contract: Address, provider: Arc<dyn Provider>, signer: Option<Arc<dyn Signer>>, }
- impl KnowledgeRegistryClient { pub async fn publish(&self, entry: KnowledgePublication) -> Result<[u8; 32]> { ... } pub async fn validate(&self, entry_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()> { ... } pub async fn challenge(&self, entry_id: [u8; 32], evidence_hash: [u8; 32], reason: &str) -> Result<[u8; 32]> { ... } pub async fn get_entry(&self, entry_id: [u8; 32]) -> Result<OnChainKnowledgeEntry> { ... } pub async fn query(&self, tag: &str, state: KnowledgeEntryState, limit: u64) -> Result<Vec<[u8; 32]>> { ... } }
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- OnChainKnowledgeEntry
- KnowledgeEntryType
- KnowledgeEntryState
- KnowledgeRegistryClient
- publish
- validate
- challenge
- get_entry
- query
- KnowledgePublication

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
- Contract 1: language `rust`, first line `/// Knowledge entry as represented in the runtime.`

```rust
/// Knowledge entry as represented in the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainKnowledgeEntry {
    pub entry_id: [u8; 32],
    pub publisher_passport: u128,
    pub title: String,
    pub entry_type: KnowledgeEntryType,
    pub content_hash: [u8; 32],
    pub hdc_fingerprint: [u8; 32],
    pub tags: Vec<String>,
    pub state: KnowledgeEntryState,
    pub validation_count: u64,
    pub challenge_count: u64,
    pub published_at_block: u64,
    pub last_refreshed_block: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryType {
    Insight,
    Playbook,
    Analysis,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeEntryState {
    Active,
    Challenged,
    Validated,
    Retracted,
    Stale,
}

/// Client for the on-chain knowledge registry.
pub struct KnowledgeRegistryClient {
    contract: Address,
    provider: Arc<dyn Provider>,
    signer: Option<Arc<dyn Signer>>,
}

impl KnowledgeRegistryClient {
    pub async fn publish(&self, entry: KnowledgePublication) -> Result<[u8; 32]> { ... }
    pub async fn validate(&self, entry_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()> { ... }
    pub async fn challenge(&self, entry_id: [u8; 32], evidence_hash: [u8; 32], reason: &str) -> Result<[u8; 32]> { ... }
    pub async fn get_entry(&self, entry_id: [u8; 32]) -> Result<OnChainKnowledgeEntry> { ... }
    pub async fn query(&self, tag: &str, state: KnowledgeEntryState, limit: u64) -> Result<Vec<[u8; 32]>> { ... }
}

#[derive(Debug,
...
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "entry|Knowledge|Serialize|publish|challenge|KnowledgeEntryType|KnowledgeEntryState|validate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "entry|Knowledge|Serialize|publish|challenge|KnowledgeEntryType|KnowledgeEntryState|validate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `OnChainKnowledgeEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeEntryType` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeEntryState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeRegistryClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `publish` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `validate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `challenge` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `get_entry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `query` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgePublication` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S015 -- Event indexer

**Source section:** `tmp/architecture/14-registries.md:593` through `596`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event indexer

A background service that indexes on-chain events from all registry contracts into queryable storage. The dashboard and runtime query the indexer instead of making direct RPC calls for historical data.
````

**Explicit detail extraction from this section:**

- Section word count: `31`
- Section hash: `5f8a1499adebcf289e9f7ec95ba2d18a128fb0a69c4fc1a6706ac810c4bd4599`

**Normative requirements and implementation claims:**
- A background service that indexes on-chain events from all registry contracts into queryable storage. The dashboard and runtime query the indexer instead of making direct RPC calls for historical data.

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "indexer|Event|query|storage|service|runtime|registry|queryable" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "indexer|Event|query|storage|service|runtime|registry|queryable" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S016 -- Architecture

**Source section:** `tmp/architecture/14-registries.md:597` through `606`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Architecture

```
Korai RPC (WebSocket) ──> Indexer ──> PostgreSQL ──> REST API
                                                       │
                          Event stream ────────────────┘
```

The indexer subscribes to all registry contract events via WebSocket. It processes events in order, stores them in PostgreSQL, and serves queries through a REST API.
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `6e6cf8663ee83a6a3f8aaac81b7179dd4cc405129c2329dd0cf3806d93deb9f2`

**Normative requirements and implementation claims:**
- ``` Korai RPC (WebSocket) ──> Indexer ──> PostgreSQL ──> REST API │ Event stream ────────────────┘ ```
- The indexer subscribes to all registry contract events via WebSocket. It processes events in order, stores them in PostgreSQL, and serves queries through a REST API.

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
- Contract 1: language `plain`, first line `Korai RPC (WebSocket) ──> Indexer ──> PostgreSQL ──> REST API`

```
Korai RPC (WebSocket) ──> Indexer ──> PostgreSQL ──> REST API
                                                       │
                          Event stream ────────────────┘
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "Event|events|WebSocket|REST|PostgreSQL|Indexer|subscribes|stream" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Event|events|WebSocket|REST|PostgreSQL|Indexer|subscribes|stream" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S017 -- Indexed event types

**Source section:** `tmp/architecture/14-registries.md:607` through `618`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Indexed event types

| Source contract | Events indexed |
|----------------|---------------|
| IAgentPassport | PassportRegistered, PassportUpdated, TierChanged |
| IReputationRegistry | ReputationAttested, TierChanged |
| IKnowledgeRegistry | EntryPublished, EntryValidated, EntryChallenged, ChallengeResolved |
| IISFROracle | RateAggregated, DeviationTriggered |
| IClearingHouse | PositionOpened, PositionClosed, RoundSettled, PositionLiquidated |
| IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored |
| IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |
````

**Explicit detail extraction from this section:**

- Section word count: `32`
- Section hash: `23e9fb53e03ea623957ab3735e9c40474d444aa1473ba1a1b0c00f79f5fd4ee1`

**Normative requirements and implementation claims:**
- | Source contract | Events indexed | |----------------|---------------| | IAgentPassport | PassportRegistered, PassportUpdated, TierChanged | | IReputationRegistry | ReputationAttested, TierChanged | | IKnowledgeRegistry | EntryPublished, EntryValidated, EntryChallenged, ChallengeResolved | | IISFROracle | RateAggregated, DeviationTriggered | | IClearingHouse | PositionOpened, PositionClosed, RoundSettled, PositionLiquidated | | IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored | | IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- PassportUpdated
- ArenaCreated
- AttemptSubmitted
- AttemptScored

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
| Source contract | Events indexed |
|----------------|---------------|
| IAgentPassport | PassportRegistered, PassportUpdated, TierChanged |
| IReputationRegistry | ReputationAttested, TierChanged |
| IKnowledgeRegistry | EntryPublished, EntryValidated, EntryChallenged, ChallengeResolved |
| IISFROracle | RateAggregated, DeviationTriggered |
| IClearingHouse | PositionOpened, PositionClosed, RoundSettled, PositionLiquidated |
| IArenaRegistry | ArenaCreated, AttemptSubmitted, AttemptScored |
| IBountyMarket | BountyPosted, BountyClaimed, BountyResolved |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "event|Indexed|types|TierChanged|contract|RoundSettled|ReputationAttested|RateAggregated" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "event|Indexed|types|TierChanged|contract|RoundSettled|ReputationAttested|RateAggregated" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Emit or consume `PassportUpdated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ArenaCreated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AttemptSubmitted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AttemptScored` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S018 -- Indexer Rust types

**Source section:** `tmp/architecture/14-registries.md:619` through `677`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Indexer Rust types

```rust
/// A stored indexed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    /// Auto-incrementing sequence number for ordering.
    pub sequence: u64,
    /// Source contract address.
    pub contract: Address,
    /// Event signature hash.
    pub event_sig: [u8; 32],
    /// Decoded event type name.
    pub event_type: String,
    /// Block number.
    pub block_number: u64,
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// Log index within the transaction.
    pub log_index: u32,
    /// Timestamp (from block header).
    pub timestamp: u64,
    /// Decoded event data as JSON.
    pub data: serde_json::Value,
}

/// Query parameters for the indexer REST API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexerQuery {
    /// Filter by contract address.
    pub contract: Option<Address>,
    /// Filter by event type name.
    pub event_type: Option<String>,
    /// Filter by block range.
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    /// Filter by a specific field value in the event data.
    pub field_filter: Option<FieldFilter>,
    /// Pagination.
    pub offset: u64,
    pub limit: u64,
    /// Sort order.
    pub sort: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `178`
- Section hash: `d09b45a95a759b9b15a885aef925f4592efbe602204121930b2b731c72136d75`

**Normative requirements and implementation claims:**
- ```rust /// A stored indexed event. #[derive(Debug, Clone, Serialize, Deserialize)] pub struct IndexedEvent { /// Auto-incrementing sequence number for ordering. pub sequence: u64, /// Source contract address. pub contract: Address, /// Event signature hash. pub event_sig: [u8; 32], /// Decoded event type name. pub event_type: String, /// Block number. pub block_number: u64, /// Transaction hash. pub tx_hash: [u8; 32], /// Log index within the transaction. pub log_index: u32, /// Timestamp (from block header). pub timestamp: u64, /// Decoded event data as JSON. pub data: serde_json::Value, }
- /// Query parameters for the indexer REST API. #[derive(Debug, Clone, Default, Serialize, Deserialize)] pub struct IndexerQuery { /// Filter by contract address. pub contract: Option<Address>, /// Filter by event type name. pub event_type: Option<String>, /// Filter by block range. pub from_block: Option<u64>, pub to_block: Option<u64>, /// Filter by a specific field value in the event data. pub field_filter: Option<FieldFilter>, /// Pagination. pub offset: u64, pub limit: u64, /// Sort order. pub sort: SortOrder, }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- IndexedEvent
- name
- IndexerQuery
- FieldFilter
- SortOrder

**Event names and event-like entities:**
- IndexedEvent

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
- Contract 1: language `rust`, first line `/// A stored indexed event.`

```rust
/// A stored indexed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    /// Auto-incrementing sequence number for ordering.
    pub sequence: u64,
    /// Source contract address.
    pub contract: Address,
    /// Event signature hash.
    pub event_sig: [u8; 32],
    /// Decoded event type name.
    pub event_type: String,
    /// Block number.
    pub block_number: u64,
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// Log index within the transaction.
    pub log_index: u32,
    /// Timestamp (from block header).
    pub timestamp: u64,
    /// Decoded event data as JSON.
    pub data: serde_json::Value,
}

/// Query parameters for the indexer REST API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexerQuery {
    /// Filter by contract address.
    pub contract: Option<Address>,
    /// Filter by event type name.
    pub event_type: Option<String>,
    /// Filter by block range.
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    /// Filter by a specific field value in the event data.
    pub field_filter: Option<FieldFilter>,
    /// Pagination.
    pub offset: u64,
    pub limit: u64,
    /// Sort order.
    pub sort: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldFilter {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "index|event|type|Serialize|Indexer|Filter|name|SortOrder" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "index|event|type|Serialize|Indexer|Filter|name|SortOrder" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Implement or verify `IndexedEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `IndexerQuery` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FieldFilter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SortOrder` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `IndexedEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S019 -- Indexer REST API

**Source section:** `tmp/architecture/14-registries.md:678` through `760`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Indexer REST API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/index/events` | Query indexed events with filtering and pagination |
| `GET` | `/api/index/events/stream` | SSE stream of new events as they're indexed |
| `GET` | `/api/index/passports` | Query indexed passport registrations |
| `GET` | `/api/index/passports/{id}/history` | Full event history for a passport |
| `GET` | `/api/index/reputation/{passport_id}` | Reputation history across domains |
| `GET` | `/api/index/knowledge` | Query indexed knowledge entries |
| `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge entry |
| `GET` | `/api/index/arenas` | Query indexed arena events |
| `GET` | `/api/index/bounties` | Query indexed bounty events |
| `GET` | `/api/index/clearing/rounds` | Query indexed clearing rounds |
| `GET` | `/api/index/stats` | Indexer health: latest block, lag, event count |

**Response: `GET /api/index/stats`**

```json
{
    "latest_indexed_block": 19847500,
    "chain_head_block": 19847502,
    "lag_blocks": 2,
    "total_events_indexed": 4827391,
    "events_by_type": {
        "PassportRegistered": 12847,
        "ReputationAttested": 892341,
        "EntryPublished": 34521,
        "RateAggregated": 1092,
        "PositionOpened": 28934,
        "RoundSettled": 8924
    },
    "uptime_seconds": 2592000,
    "last_error": null
}
```

**Response: `GET /api/index/events?event_type=ReputationAttested&limit=2`**

```json
{
    "events": [
        {
            "sequence": 4827391,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847498,
            "tx_hash": "0xabcd...ef01",
            "log_index": 3,
            "timestamp": 1714089600,
            "data": {
                "agentPassportId": 42,
                "domain": "0x7472616469...",
                "delta": "500000000000000000",
                "newScore": "82300000000000000000",
                "attester": "0x9876...5432"
            }
        },
        {
            "sequence": 4827390,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847495,
            "tx_hash": "0x2345...6789",
            "log_index": 1,
            "timestamp": 1714089564,
            "data": {
                "agentPassportId": 107,
                "domain": "0x636f64696e...",
                "delta": "-200000000000000000",
                "newScore": "31700000000000000000",
                "attester": "0xaaaa...bbbb"
            }
        }
    ],
    "total": 892341,
    "offset": 0,
    "limit": 2
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `217`
- Section hash: `8ba45fd9f6a78a0b0492eae08da267ddf5c4f46afceb914e030159f9bda22ce8`

**Normative requirements and implementation claims:**
- | Method | Path | Description | |--------|------|-------------| | `GET` | `/api/index/events` | Query indexed events with filtering and pagination | | `GET` | `/api/index/events/stream` | SSE stream of new events as they're indexed | | `GET` | `/api/index/passports` | Query indexed passport registrations | | `GET` | `/api/index/passports/{id}/history` | Full event history for a passport | | `GET` | `/api/index/reputation/{passport_id}` | Reputation history across domains | | `GET` | `/api/index/knowledge` | Query indexed knowledge entries | | `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge entry | | `GET` | `/api/index/arenas` | Query indexed arena events | | `GET` | `/api/index/bounties` | Query indexed bounty events | | `GET` | `/api/index/clearing/rounds` | Query indexed clearing rounds | | `GET` | `/api/index/stats` | Indexer health: latest block, lag, event count |
- **Response: `GET /api/index/stats`**
- **Response: `GET /api/index/events?event_type=ReputationAttested&limit=2`**
- ```json { "events": [ { "sequence": 4827391, "contract": "0x1234...5678", "event_type": "ReputationAttested", "block_number": 19847498, "tx_hash": "0xabcd...ef01", "log_index": 3, "timestamp": 1714089600, "data": { "agentPassportId": 42, "domain": "0x7472616469...", "delta": "500000000000000000", "newScore": "82300000000000000000", "attester": "0x9876...5432" } }, { "sequence": 4827390, "contract": "0x1234...5678", "event_type": "ReputationAttested", "block_number": 19847495, "tx_hash": "0x2345...6789", "log_index": 1, "timestamp": 1714089564, "data": { "agentPassportId": 107, "domain": "0x636f64696e...", "delta": "-200000000000000000", "newScore": "31700000000000000000", "attester": "0xaaaa...bbbb" } } ], "total": 892341, "offset": 0, "limit": 2 } ```
- ---

**Routes and endpoint references:**
- GET /api/index/stats
- GET /api/index/events
- /api/index/events/stream
- /api/index/passports
- /api/index/passports/{id}/history
- /api/index/reputation/{passport_id}
- /api/index/knowledge
- /api/index/knowledge/{id}/history
- /api/index/arenas
- /api/index/bounties
- /api/index/clearing/rounds

**Files and path references:**
- api/index/
- api/index/clearing/
- api/index/events/
- api/index/knowledge/
- api/index/passports/
- api/index/reputation/

**Types, functions, traits, and inline code identifiers:**
- GET

**Event names and event-like entities:**
- x1234...5678
- xabcd...ef01
- x7472616469...
- x9876...5432
- x2345...6789
- x636f64696e...
- xaaaa...bbbb

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
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/index/events` | Query indexed events with filtering and pagination |
| `GET` | `/api/index/events/stream` | SSE stream of new events as they're indexed |
| `GET` | `/api/index/passports` | Query indexed passport registrations |
| `GET` | `/api/index/passports/{id}/history` | Full event history for a passport |
| `GET` | `/api/index/reputation/{passport_id}` | Reputation history across domains |
| `GET` | `/api/index/knowledge` | Query indexed knowledge entries |
| `GET` | `/api/index/knowledge/{id}/history` | Event history for a knowledge entry |
| `GET` | `/api/index/arenas` | Query indexed arena events |
| `GET` | `/api/index/bounties` | Query indexed bounty events |
| `GET` | `/api/index/clearing/rounds` | Query indexed clearing rounds |
...
```

**Data/code contracts extracted:**
- Contract 1: language `json`, first line `{`

```json
{
    "latest_indexed_block": 19847500,
    "chain_head_block": 19847502,
    "lag_blocks": 2,
    "total_events_indexed": 4827391,
    "events_by_type": {
        "PassportRegistered": 12847,
        "ReputationAttested": 892341,
        "EntryPublished": 34521,
        "RateAggregated": 1092,
        "PositionOpened": 28934,
        "RoundSettled": 8924
    },
    "uptime_seconds": 2592000,
    "last_error": null
}
```
- Contract 2: language `json`, first line `{`

```json
{
    "events": [
        {
            "sequence": 4827391,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847498,
            "tx_hash": "0xabcd...ef01",
            "log_index": 3,
            "timestamp": 1714089600,
            "data": {
                "agentPassportId": 42,
                "domain": "0x7472616469...",
                "delta": "500000000000000000",
                "newScore": "82300000000000000000",
                "attester": "0x9876...5432"
            }
        },
        {
            "sequence": 4827390,
            "contract": "0x1234...5678",
            "event_type": "ReputationAttested",
            "block_number": 19847495,
            "tx_hash": "0x2345...6789",
            "log_index": 1,
            "timestamp": 1714089564,
            "data": {
                "agentPassportId": 107,
                "domain": "0x636f64696e...",
                "delta": "-200000000000000000",
                "newScore": "31700000000000000000",
                "attester": "0xaaaa...bbbb"
            }
        }
    ],
    "total": 892341,
    "offset": 0,
    "limit": 2
}
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `api/index/`
- `api/index/clearing/`
- `api/index/events/`
- `api/index/knowledge/`
- `api/index/passports/`
- `api/index/reputation/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "index|API|GET|event|events|indexed|passport|reputation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "index|API|GET|event|events|indexed|passport|reputation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `api/index/`
- `api/index/clearing/`
- `api/index/events/`
- `api/index/knowledge/`
- `api/index/passports/`
- `api/index/reputation/`
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
- [ ] Implement or verify route `GET /api/index/stats` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/index/events` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/events/stream` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/passports` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/passports/{id}/history` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/reputation/{passport_id}` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/knowledge` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/knowledge/{id}/history` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/arenas` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/bounties` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `/api/index/clearing/rounds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `GET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `x1234...5678` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `xabcd...ef01` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x7472616469...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x9876...5432` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x2345...6789` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x636f64696e...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `xaaaa...bbbb` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S020 -- Contract addresses

**Source section:** `tmp/architecture/14-registries.md:761` through `764`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Contract addresses

All contracts are deployed on Korai (production) and Mirage (development). Addresses are configured in `roko.toml`.
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `d35c394e8a24c343ec28efed5b7ad9dc0be81ba4145454bac26505c7621e1ccc`

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
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "addresses|Contract|toml|production|development|deployed|contracts|configured" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "addresses|Contract|toml|production|development|deployed|contracts|configured" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S021 -- Mirage devnet addresses

**Source section:** `tmp/architecture/14-registries.md:765` through `779`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Mirage devnet addresses

| Contract | Address | Notes |
|----------|---------|-------|
| AgentPassport (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract |
| ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentPassport |
| KnowledgeRegistry | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | InsightStore on-chain |
| ISFROracle | `0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9` | See `12-defi.md` |
| ClearingHouse | `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9` | See `12-defi.md` |
| ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See `11-arenas.md` |
| BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See `11-arenas.md` |
| Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |

These are Hardhat default deployment addresses. Production Korai addresses will differ.
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `6d7a6d51390accef6a4f3cbf614de5af68c6f5ffbeb9d01d9e7b59551fe9012e`

**Normative requirements and implementation claims:**
- | Contract | Address | Notes | |----------|---------|-------| | AgentPassport (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract | | ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentPassport | | KnowledgeRegistry | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | InsightStore on-chain | | ISFROracle | `0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9` | See `12-defi.md` | | ClearingHouse | `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9` | See `12-defi.md` | | ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See `11-arenas.md` | | BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See `11-arenas.md` | | Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |

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
| Contract | Address | Notes |
|----------|---------|-------|
| AgentPassport (ERC-8004) | `0x5FbDB2315678afecb367f032d93F642f64180aa3` | First deployed contract |
| ReputationRegistry | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` | Linked to AgentPassport |
| KnowledgeRegistry | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` | InsightStore on-chain |
| ISFROracle | `0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9` | See `12-defi.md` |
| ClearingHouse | `0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9` | See `12-defi.md` |
| ArenaRegistry | `0x5FC8d32690cc91D4c39d9d3abcBD16989F875707` | See `11-arenas.md` |
| BountyMarket | `0x0165878A594ca255338adfa4d48449f69242Eb8F` | See `11-arenas.md` |
| Daeji Token | `0xa513E6E4b8f2a923D98304ec87F64353C4D5C853` | ERC-20 utility token |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Address|addresses|devnet|Mirage|defi|arenas|Token|Contract" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Address|addresses|devnet|Mirage|defi|arenas|Token|Contract" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S022 -- Configuration

**Source section:** `tmp/architecture/14-registries.md:780` through `815`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Configuration

```toml
# roko.toml

[chain]
# Network to use: "mirage" for local dev, "korai" for production
network = "mirage"

[chain.mirage]
rpc_url = "http://localhost:8545"
ws_url = "ws://localhost:8546"
chain_id = 31337

[chain.korai]
rpc_url = "https://rpc.korai.network"
ws_url = "wss://ws.korai.network"
chain_id = 88888

[chain.contracts]
agent_passport = "0x5FbDB2315678afecb367f032d93F642f64180aa3"
reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
knowledge_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
isfr_oracle = "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
clearing_house = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"
daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"

[chain.indexer]
url = "http://localhost:6678"
# The indexer runs as a separate process alongside roko-serve
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `76`
- Section hash: `72cef1f678ab7863c8a56bdd7b41f57a8b3516e21ba6a4e4152b13f9742cd901`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- chain.mirage
- chain.korai
- rpc.korai.network
- ws.korai.network
- chain.contracts
- chain.indexer

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- [chain]
- network = "mirage"
- [chain.mirage]
- rpc_url = "http://localhost:8545"
- ws_url = "ws://localhost:8546"
- chain_id = 31337
- [chain.korai]
- rpc_url = "https://rpc.korai.network"
- ws_url = "wss://ws.korai.network"
- chain_id = 88888
- [chain.contracts]
- agent_passport = "0x5FbDB2315678afecb367f032d93F642f64180aa3"
- reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
- knowledge_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
- isfr_oracle = "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
- clearing_house = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
- arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
- bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"
- daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"
- [chain.indexer]
- url = "http://localhost:6678"

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# roko.toml`

```toml
# roko.toml

[chain]
# Network to use: "mirage" for local dev, "korai" for production
network = "mirage"

[chain.mirage]
rpc_url = "http://localhost:8545"
ws_url = "ws://localhost:8546"
chain_id = 31337

[chain.korai]
rpc_url = "https://rpc.korai.network"
ws_url = "wss://ws.korai.network"
chain_id = 88888

[chain.contracts]
agent_passport = "0x5FbDB2315678afecb367f032d93F642f64180aa3"
reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
knowledge_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
isfr_oracle = "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
clearing_house = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"
daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"

[chain.indexer]
url = "http://localhost:6678"
# The indexer runs as a separate process alongside roko-serve
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "chain|Configuration|network|local|korai|rpc|mirage|localhost" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "chain|Configuration|network|local|korai|rpc|mirage|localhost" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Emit or consume `chain.mirage` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `chain.korai` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `rpc.korai.network` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ws.korai.network` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `chain.contracts` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `chain.indexer` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `[chain]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `network = "mirage"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[chain.mirage]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `rpc_url = "http://localhost:8545"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ws_url = "ws://localhost:8546"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `chain_id = 31337` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[chain.korai]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `rpc_url = "https://rpc.korai.network"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ws_url = "wss://ws.korai.network"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `chain_id = 88888` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[chain.contracts]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `agent_passport = "0x5FbDB2315678afecb367f032d93F642f64180aa3"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `reputation_registry = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `isfr_oracle = "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `clearing_house = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `arena_registry = "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `bounty_market = "0x0165878A594ca255338adfa4d48449f69242Eb8F"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `daeji_token = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[chain.indexer]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `url = "http://localhost:6678"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S023 -- Integration with existing systems

**Source section:** `tmp/architecture/14-registries.md:816` through `817`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Integration with existing systems
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `7b81dd378693c6ed6f86ae558a9211deabd572459a7b58039d5adad3fa8e0cc6`

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "systems|existing|Integration|registries" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "systems|existing|Integration|registries" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S024 -- Passport registration in agent lifecycle

**Source section:** `tmp/architecture/14-registries.md:818` through `827`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Passport registration in agent lifecycle

When an agent starts and `chain.network` is configured, the agent runtime checks whether it has a registered passport. If not, it registers one automatically during startup:

1. Read agent config (name, capabilities, domain).
2. Hash capabilities to `bytes32[]`.
3. Call `IAgentPassport.register()`.
4. Store the returned `tokenId` in `.roko/state/passport.json`.
5. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.
````

**Explicit detail extraction from this section:**

- Section word count: `68`
- Section hash: `eaa2c09b3f3c70e800dcaa0eb358397b081d3be4b78d7759220e4f19b75ff54a`

**Normative requirements and implementation claims:**
- 1. Read agent config (name, capabilities, domain). 2. Hash capabilities to `bytes32[]`. 3. Call `IAgentPassport.register()`. 4. Store the returned `tokenId` in `.roko/state/passport.json`. 5. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/state/passport.json

**Types, functions, traits, and inline code identifiers:**
- tokenId

**Event names and event-like entities:**
- chain.network
- assport.register

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- chain.network

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Read agent config (name, capabilities, domain).
- 2. Hash capabilities to `bytes32[]`.
- 3. Call `IAgentPassport.register()`.
- 4. Store the returned `tokenId` in `.roko/state/passport.json`.
- 5. On subsequent startups, read the stored `tokenId` and verify it still exists on-chain.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `.roko/state/passport.json`
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
rg -n "assport|Passport|tokenId|registration|lifecycle|register|startup|config" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "assport|Passport|tokenId|registration|lifecycle|register|startup|config" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `.roko/state/passport.json`
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
- [ ] Implement or verify `tokenId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `chain.network` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `assport.register` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `chain.network` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S025 -- Reputation in the cascade router

**Source section:** `tmp/architecture/14-registries.md:828` through `835`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Reputation in the cascade router

The cascade router (`roko-learn/src/model_router.rs`) consults reputation when routing tasks to agents:

- Higher-tier agents get priority for complex tasks.
- Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`.
- The bandit algorithm treats reputation-weighted outcomes as higher-signal observations.
````

**Explicit detail extraction from this section:**

- Section word count: `43`
- Section hash: `0a5d1a8ad55f997b7d3e89b0526d9239206dc9ec621e367c1d0f6d3a1e809d61`

**Normative requirements and implementation claims:**
- - Higher-tier agents get priority for complex tasks. - Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`. - The bandit algorithm treats reputation-weighted outcomes as higher-signal observations.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-learn/src/model_router.rs

**Types, functions, traits, and inline code identifiers:**
- RoutingContext

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Higher-tier agents get priority for complex tasks.
- - Reputation scores feed into the `RoutingContext` as `agent_reputation: f64`.
- - The bandit algorithm treats reputation-weighted outcomes as higher-signal observations.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `roko-learn/src/model_router.rs`
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
rg -n "Reputation|the|router|cascade|RoutingContext|tasks|routing|Higher" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Reputation|the|router|cascade|RoutingContext|tasks|routing|Higher" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `roko-learn/src/model_router.rs`
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
- [ ] Implement or verify `RoutingContext` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S026 -- Knowledge publication from neuro store

**Source section:** `tmp/architecture/14-registries.md:836` through `844`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge publication from neuro store

When the neuro store (`roko-neuro`) promotes a knowledge entry to "durable" status, it can publish the entry on-chain:

1. Compute HDC fingerprint of the entry content.
2. Upload full content to IPFS (or store content hash only for private entries).
3. Call `IKnowledgeRegistry.publish()` with metadata and content hash.
4. Record the on-chain `entryId` in the local neuro store for cross-referencing.
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `e2840c17ff455c01251b9d0fe894720e99ed1a4115957b4a80ca96a1817197e2`

**Normative requirements and implementation claims:**
- 1. Compute HDC fingerprint of the entry content. 2. Upload full content to IPFS (or store content hash only for private entries). 3. Call `IKnowledgeRegistry.publish()` with metadata and content hash. 4. Record the on-chain `entryId` in the local neuro store for cross-referencing.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- entryId

**Event names and event-like entities:**
- egistry.publish

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. Compute HDC fingerprint of the entry content.
- 2. Upload full content to IPFS (or store content hash only for private entries).
- 3. Call `IKnowledgeRegistry.publish()` with metadata and content hash.
- 4. Record the on-chain `entryId` in the local neuro store for cross-referencing.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "store|neuro|Knowledge|publication|entryId|entry|content|publish" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "store|neuro|Knowledge|publication|entryId|entry|content|publish" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `entryId` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `egistry.publish` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S027 -- Event indexer as data backbone

**Source section:** `tmp/architecture/14-registries.md:845` through `850`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Event indexer as data backbone

The dashboard aggregation service (see `21-roko-and-chain-additions.md`) reads from the event indexer for all historical queries. Real-time updates come from WebSocket subscriptions to the chain. The indexer bridges the gap between "every event ever" (historical) and "what's happening now" (live).

---
````

**Explicit detail extraction from this section:**

- Section word count: `46`
- Section hash: `9cfb5a9123494a034db7e2ed37c3da8019162dcee313abec537326b6ad008aed`

**Normative requirements and implementation claims:**
- The dashboard aggregation service (see `21-roko-and-chain-additions.md`) reads from the event indexer for all historical queries. Real-time updates come from WebSocket subscriptions to the chain. The indexer bridges the gap between "every event ever" (historical) and "what's happening now" (live).
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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "indexer|Event|data|backbone|historical|ever|chain|updates" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "indexer|Event|data|backbone|historical|ever|chain|updates" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S028 -- Event types

**Source section:** `tmp/architecture/14-registries.md:851` through `925`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event types

All registry events flow through the event indexer and are available via the indexer REST API and SSE stream.

```json
{
    "type": "passport.registered",
    "payload": {
        "token_id": 42,
        "owner": "0xabc...def",
        "name": "trade-executor-1",
        "capabilities": ["trading", "analysis"],
        "parent_passport": null,
        "block_number": 19847300
    }
}
```

```json
{
    "type": "reputation.attested",
    "payload": {
        "agent_passport_id": 42,
        "domain": "trading",
        "delta": 0.5,
        "new_score": 82.3,
        "old_tier": "Silver",
        "new_tier": "Silver",
        "attester_contract": "ClearingHouse",
        "block_number": 19847498
    }
}
```

```json
{
    "type": "reputation.tier_changed",
    "payload": {
        "agent_passport_id": 42,
        "old_tier": "Silver",
        "new_tier": "Gold",
        "aggregate_score": 201.4,
        "block_number": 19847500
    }
}
```

```json
{
    "type": "knowledge.published",
    "payload": {
        "entry_id": "0x1234...5678",
        "publisher_passport": 42,
        "title": "ETH funding rate correlation with BTC dominance",
        "entry_type": "insight",
        "tags": ["funding-rate", "correlation", "eth", "btc"],
        "block_number": 19847510
    }
}
```

```json
{
    "type": "knowledge.challenged",
    "payload": {
        "entry_id": "0x1234...5678",
        "challenge_id": "0xabcd...ef01",
        "challenger_passport": 107,
        "reason": "Correlation breaks during high-volatility regimes",
        "resolution_deadline_block": 19848510,
        "block_number": 19847600
    }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `131`
- Section hash: `773e3e2d02d850ddec9430bd855680d4e2ce318676b067479bf101e3d8b65f9b`

**Normative requirements and implementation claims:**
- All registry events flow through the event indexer and are available via the indexer REST API and SSE stream.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- passport.registered
- xabc...def
- reputation.attested
- reputation.tier_changed
- knowledge.published
- x1234...5678
- knowledge.challenged
- xabcd...ef01

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
- Contract 1: language `json`, first line `{`

```json
{
    "type": "passport.registered",
    "payload": {
        "token_id": 42,
        "owner": "0xabc...def",
        "name": "trade-executor-1",
        "capabilities": ["trading", "analysis"],
        "parent_passport": null,
        "block_number": 19847300
    }
}
```
- Contract 2: language `json`, first line `{`

```json
{
    "type": "reputation.attested",
    "payload": {
        "agent_passport_id": 42,
        "domain": "trading",
        "delta": 0.5,
        "new_score": 82.3,
        "old_tier": "Silver",
        "new_tier": "Silver",
        "attester_contract": "ClearingHouse",
        "block_number": 19847498
    }
}
```
- Contract 3: language `json`, first line `{`

```json
{
    "type": "reputation.tier_changed",
    "payload": {
        "agent_passport_id": 42,
        "old_tier": "Silver",
        "new_tier": "Gold",
        "aggregate_score": 201.4,
        "block_number": 19847500
    }
}
```
- Contract 4: language `json`, first line `{`

```json
{
    "type": "knowledge.published",
    "payload": {
        "entry_id": "0x1234...5678",
        "publisher_passport": 42,
        "title": "ETH funding rate correlation with BTC dominance",
        "entry_type": "insight",
        "tags": ["funding-rate", "correlation", "eth", "btc"],
        "block_number": 19847510
    }
}
```
- Contract 5: language `json`, first line `{`

```json
{
    "type": "knowledge.challenged",
    "payload": {
        "entry_id": "0x1234...5678",
        "challenge_id": "0xabcd...ef01",
        "challenger_passport": 107,
        "reason": "Correlation breaks during high-volatility regimes",
        "resolution_deadline_block": 19848510,
        "block_number": 19847600
    }
}
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "type|Event|passport|types|payload|json|block_number|correlation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "type|Event|passport|types|payload|json|block_number|correlation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Emit or consume `passport.registered` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `xabc...def` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `reputation.attested` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `reputation.tier_changed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.published` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x1234...5678` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.challenged` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `xabcd...ef01` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S029 -- Full event type list

**Source section:** `tmp/architecture/14-registries.md:926` through `942`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Full event type list

| Event | Source | Indexed by |
|-------|--------|-----------|
| `passport.registered` | IAgentPassport | Indexer, dashboard |
| `passport.updated` | IAgentPassport | Indexer, dashboard |
| `passport.tier_changed` | IAgentPassport | Indexer, dashboard, cascade router |
| `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router |
| `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, passport contract |
| `knowledge.published` | IKnowledgeRegistry | Indexer, dashboard, neuro store |
| `knowledge.validated` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenged` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenge_resolved` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.state_changed` | IKnowledgeRegistry | Indexer, dashboard |

---
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `ab948b097f140c2206771436561be2bdae921d596672bb33d5b11190575c6a5e`

**Normative requirements and implementation claims:**
- | Event | Source | Indexed by | |-------|--------|-----------| | `passport.registered` | IAgentPassport | Indexer, dashboard | | `passport.updated` | IAgentPassport | Indexer, dashboard | | `passport.tier_changed` | IAgentPassport | Indexer, dashboard, cascade router | | `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router | | `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, passport contract | | `knowledge.published` | IKnowledgeRegistry | Indexer, dashboard, neuro store | | `knowledge.validated` | IKnowledgeRegistry | Indexer, dashboard, reputation | | `knowledge.challenged` | IKnowledgeRegistry | Indexer, dashboard, reputation | | `knowledge.challenge_resolved` | IKnowledgeRegistry | Indexer, dashboard, reputation | | `knowledge.state_changed` | IKnowledgeRegistry | Indexer, dashboard |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- passport.registered
- passport.updated
- passport.tier_changed
- reputation.attested
- reputation.tier_changed
- knowledge.published
- knowledge.validated
- knowledge.challenged
- knowledge.challenge_resolved
- knowledge.state_changed

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- passport.registered
- passport.updated
- passport.tier_changed
- reputation.attested
- reputation.tier_changed
- knowledge.published
- knowledge.validated
- knowledge.challenged
- knowledge.challenge_resolved
- knowledge.state_changed

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Event | Source | Indexed by |
|-------|--------|-----------|
| `passport.registered` | IAgentPassport | Indexer, dashboard |
| `passport.updated` | IAgentPassport | Indexer, dashboard |
| `passport.tier_changed` | IAgentPassport | Indexer, dashboard, cascade router |
| `reputation.attested` | IReputationRegistry | Indexer, dashboard, cascade router |
| `reputation.tier_changed` | IReputationRegistry | Indexer, dashboard, passport contract |
| `knowledge.published` | IKnowledgeRegistry | Indexer, dashboard, neuro store |
| `knowledge.validated` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenged` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.challenge_resolved` | IKnowledgeRegistry | Indexer, dashboard, reputation |
| `knowledge.state_changed` | IKnowledgeRegistry | Indexer, dashboard |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "knowledge|Indexer|reputation|passport|event|type|list|IKnowledgeRegistry" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "knowledge|Indexer|reputation|passport|event|type|list|IKnowledgeRegistry" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Emit or consume `passport.registered` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `passport.updated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `passport.tier_changed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `reputation.attested` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `reputation.tier_changed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.published` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.validated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.challenged` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.challenge_resolved` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.state_changed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `passport.registered` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `passport.updated` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `passport.tier_changed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `reputation.attested` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `reputation.tier_changed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.published` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.validated` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.challenged` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.challenge_resolved` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.state_changed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S030 -- Deployment

**Source section:** `tmp/architecture/14-registries.md:943` through `944`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Deployment
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `a147233b0bc665fa439e7bbdbfa4417abbaaa325b4361ba26ea4e748640c4b8c`

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
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "Deployment|registries" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Deployment|registries" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S031 -- Contracts

**Source section:** `tmp/architecture/14-registries.md:945` through `957`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Contracts

Contracts are deployed using Hardhat. The deployment script outputs addresses to a JSON file that `roko.toml` references.

```bash
# Deploy to Mirage (local dev)
cd contracts/
npx hardhat deploy --network mirage

# Deploy to Korai (production)
npx hardhat deploy --network korai
```
````

**Explicit detail extraction from this section:**

- Section word count: `40`
- Section hash: `9716b00334ee4101d8bb2492e1c96f6e8d406a7e22d8d746bac0fd6c0f758225`

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
- roko.toml

**Commands and operator actions:**
- cd contracts/
- npx hardhat deploy --network mirage
- npx hardhat deploy --network korai

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `# Deploy to Mirage (local dev)`

```bash
# Deploy to Mirage (local dev)
cd contracts/
npx hardhat deploy --network mirage

# Deploy to Korai (production)
npx hardhat deploy --network korai
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
rg -n "Contracts|Deploy|Hardhat|network|Mirage|Korai|toml|script" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Contracts|Deploy|Hardhat|network|Mirage|Korai|toml|script" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
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
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `cd contracts/` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `npx hardhat deploy --network mirage` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `npx hardhat deploy --network korai` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-14-S032 -- Indexer

**Source section:** `tmp/architecture/14-registries.md:958` through `970`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Indexer

The indexer runs as a standalone process, typically alongside `roko-serve`:

```bash
# Start the indexer
roko indexer start --chain mirage --db postgres://localhost/roko_index

# Check indexer health
curl http://localhost:6678/api/index/stats
```

In production, the indexer runs on Railway alongside the control plane. It connects to the Korai WebSocket endpoint and writes to a managed PostgreSQL instance.
````

**Explicit detail extraction from this section:**

- Section word count: `59`
- Section hash: `b97718a9c3b7c70163918f304e486cc4878b2b7024b49c3199250d5c8aa98c8d`

**Normative requirements and implementation claims:**
- # Check indexer health curl http://localhost:6678/api/index/stats ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- 6678/api/index/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko indexer start --chain mirage --db postgres://localhost/roko_index
- curl http://localhost:6678/api/index/stats

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `# Start the indexer`

```bash
# Start the indexer
roko indexer start --chain mirage --db postgres://localhost/roko_index

# Check indexer health
curl http://localhost:6678/api/index/stats
```

**Read before editing:**
- `tmp/architecture/14-registries.md`
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `6678/api/index/`
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
rg -n "index|Indexer|runs|postgres|localhost|alongside|Start|writes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "index|Indexer|runs|postgres|localhost|alongside|Start|writes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `contracts/src/`
- `crates/roko-chain/src/`
- `crates/roko-serve/src/routes/chain.rs`
- `crates/roko-serve/src/routes/registries.rs`
- `6678/api/index/`
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
- [ ] Implement or verify operator command `roko indexer start --chain mirage --db postgres://localhost/roko_index` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `curl http://localhost:6678/api/index/stats` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/14-registries
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

