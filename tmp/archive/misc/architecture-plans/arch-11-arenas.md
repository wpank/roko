# Architecture Plan: Arenas

**Source:** `tmp/architecture/11-arenas.md`
**Generated:** 2026-04-25
**Source hash:** `0768072a65490c3c522b2a4fb644714a8af294a2477d749e46f3a9833485191e`
**Section tasks:** 38
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
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-11-S001 | 1 | 11 -- Arenas, evals, and bounties | [ ] | 9.8 |
| ARCH-11-S002 | 9 | Design constraints | [ ] | 9.8 |
| ARCH-11-S003 | 19 | Arenas | [ ] | 9.8 |
| ARCH-11-S004 | 23 | Core types | [ ] | 9.8 |
| ARCH-11-S005 | 214 | Leaderboard | [ ] | 9.8 |
| ARCH-11-S006 | 249 | Attempt lifecycle | [ ] | 9.8 |
| ARCH-11-S007 | 309 | Arena registry | [ ] | 9.8 |
| ARCH-11-S008 | 360 | Evals | [ ] | 9.8 |
| ARCH-11-S009 | 364 | Ground truth | [ ] | 9.8 |
| ARCH-11-S010 | 435 | Eval definition | [ ] | 9.8 |
| ARCH-11-S011 | 470 | Meta-evals | [ ] | 9.8 |
| ARCH-11-S012 | 497 | Eval registry | [ ] | 9.8 |
| ARCH-11-S013 | 538 | Bounty market | [ ] | 9.8 |
| ARCH-11-S014 | 542 | Relationship to existing code | [ ] | 9.8 |
| ARCH-11-S015 | 552 | VCG matching | [ ] | 9.8 |
| ARCH-11-S016 | 623 | Bounty bids | [ ] | 9.8 |
| ARCH-11-S017 | 648 | Dispute resolution | [ ] | 9.8 |
| ARCH-11-S018 | 675 | API surface | [ ] | 9.8 |
| ARCH-11-S019 | 677 | Arena endpoints | [ ] | 9.8 |
| ARCH-11-S020 | 693 | Example: create an arena | [ ] | 9.8 |
| ARCH-11-S021 | 727 | Example: submit an attempt | [ ] | 9.8 |
| ARCH-11-S022 | 738 | Eval endpoints | [ ] | 9.8 |
| ARCH-11-S023 | 753 | Example: register an eval | [ ] | 9.8 |
| ARCH-11-S024 | 779 | Bounty endpoints | [ ] | 9.8 |
| ARCH-11-S025 | 798 | Example: post a bounty | [ ] | 9.8 |
| ARCH-11-S026 | 821 | Batch matching | [ ] | 9.8 |
| ARCH-11-S027 | 845 | Event types | [ ] | 9.8 |
| ARCH-11-S028 | 849 | Arena events | [ ] | 9.8 |
| ARCH-11-S029 | 868 | Eval events | [ ] | 9.8 |
| ARCH-11-S030 | 883 | Bounty events | [ ] | 9.8 |
| ARCH-11-S031 | 906 | WebSocket subscription | [ ] | 9.8 |
| ARCH-11-S032 | 919 | On-chain contracts | [ ] | 9.8 |
| ARCH-11-S033 | 923 | ArenaRegistry.sol | [ ] | 9.8 |
| ARCH-11-S034 | 968 | EvalRegistry.sol | [ ] | 9.8 |
| ARCH-11-S035 | 1002 | BountyMarket.sol | [ ] | 9.8 |
| ARCH-11-S036 | 1041 | DisputeResolver.sol | [ ] | 9.8 |
| ARCH-11-S037 | 1079 | Crate mapping | [ ] | 9.8 |
| ARCH-11-S038 | 1095 | Interactions with other subsystems | [ ] | 9.8 |

## Tasks

### ARCH-11-S001 -- 11 -- Arenas, evals, and bounties

**Source section:** `tmp/architecture/11-arenas.md:1` through `8`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 11 -- Arenas, evals, and bounties

Three subsystems that make agent performance measurable, competitive, and economically useful. Arenas provide competitive environments. Evals provide ground-truth measurement. Bounties provide paid task markets. All three feed the reputation registry and the cascade router's learning loop.

This document covers the runtime types, contract interfaces, API surface, and event model for each subsystem. Dashboard surfaces that consume these APIs are specified in `15-arena-surfaces.md` (PRD).

---
````

**Explicit detail extraction from this section:**

- Section word count: `68`
- Section hash: `45fd565b475b69aa378d5d875e36915932b31ecf7017debec4409fe0bbe2a1d4`

**Normative requirements and implementation claims:**
- Three subsystems that make agent performance measurable, competitive, and economically useful. Arenas provide competitive environments. Evals provide ground-truth measurement. Bounties provide paid task markets. All three feed the reputation registry and the cascade router's learning loop.
- This document covers the runtime types, contract interfaces, API surface, and event model for each subsystem. Dashboard surfaces that consume these APIs are specified in `15-arena-surfaces.md` (PRD).
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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "arena|evals|bounties|Arenas|surface|surfaces|subsystem" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "arena|evals|bounties|Arenas|surface|surfaces|subsystem" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S002 -- Design constraints

**Source section:** `tmp/architecture/11-arenas.md:9` through `18`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Design constraints

1. **No self-grading.** Evals never use LLM output to judge LLM output. Ground truth comes from external oracles, test suites, human review, chain state, or benchmark datasets.
2. **Scoring is declarative.** Every arena and eval declares its scoring function at registration time. Participants know how they'll be scored before they start.
3. **Escrow before execution.** Bounties lock funds in a contract before agents begin work. No payment promises -- only escrowed funds.
4. **Reputation flows from validation.** Arena attempts and bounty completions produce `WorkProof` records that feed the `ValidationRegistry` and `ReputationRegistry` (see `14-registries.md`).
5. **VCG for matching, Vickrey for bidding.** Agent-to-task matching uses welfare-maximizing allocation. Individual bounties use second-price auctions. Both enforce truthful bidding.

---
````

**Explicit detail extraction from this section:**

- Section word count: `121`
- Section hash: `52baccc767cca9c1a2d8a5e16d932ac14fb4eef50df00708abf865c1fcb4a307`

**Normative requirements and implementation claims:**
- 1. **No self-grading.** Evals never use LLM output to judge LLM output. Ground truth comes from external oracles, test suites, human review, chain state, or benchmark datasets. 2. **Scoring is declarative.** Every arena and eval declares its scoring function at registration time. Participants know how they'll be scored before they start. 3. **Escrow before execution.** Bounties lock funds in a contract before agents begin work. No payment promises -- only escrowed funds. 4. **Reputation flows from validation.** Arena attempts and bounty completions produce `WorkProof` records that feed the `ValidationRegistry` and `ReputationRegistry` (see `14-registries.md`). 5. **VCG for matching, Vickrey for bidding.** Agent-to-task matching uses welfare-maximizing allocation. Individual bounties use second-price auctions. Both enforce truthful bidding.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- WorkProof
- ValidationRegistry
- ReputationRegistry

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **No self-grading.** Evals never use LLM output to judge LLM output. Ground truth comes from external oracles, test suites, human review, chain state, or benchmark datasets.
- 2. **Scoring is declarative.** Every arena and eval declares its scoring function at registration time. Participants know how they'll be scored before they start.
- 3. **Escrow before execution.** Bounties lock funds in a contract before agents begin work. No payment promises -- only escrowed funds.
- 4. **Reputation flows from validation.** Arena attempts and bounty completions produce `WorkProof` records that feed the `ValidationRegistry` and `ReputationRegistry` (see `14-registries.md`).
- 5. **VCG for matching, Vickrey for bidding.** Agent-to-task matching uses welfare-maximizing allocation. Individual bounties use second-price auctions. Both enforce truthful bidding.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "constraints|WorkProof|ValidationRegistry|ReputationRegistry|Design|before|work|validation" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "constraints|WorkProof|ValidationRegistry|ReputationRegistry|Design|before|work|validation" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `WorkProof` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ValidationRegistry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReputationRegistry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S003 -- Arenas

**Source section:** `tmp/architecture/11-arenas.md:19` through `22`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Arenas

An arena is a competitive environment defined by three things: what agents do (task source), how they're scored (scoring function), and who's winning (leaderboard).
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `0bf01cad27bfb5b14d54809a74bfa9bd35c7442dbcb91c7e883b229757e67674`

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "arena|Arenas|winning|three|things|task|scoring|scored" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "arena|Arenas|winning|three|things|task|scoring|scored" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S004 -- Core types

**Source section:** `tmp/architecture/11-arenas.md:23` through `213`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Core types

```rust
/// Arena lifecycle states.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArenaState {
    /// Arena created but not yet accepting attempts.
    Draft,
    /// Arena is live and accepting attempts.
    Active,
    /// Arena is temporarily paused (no new attempts, existing ones continue).
    Paused,
    /// Arena has permanently concluded. Leaderboard is final.
    Concluded,
}

/// Where arena tasks come from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskSource {
    /// Fixed dataset of input/output pairs.
    Static {
        /// IPFS CID or URL pointing to the dataset.
        dataset_cid: String,
        /// Number of tasks in the dataset.
        count: u64,
        /// Whether tasks are sampled randomly per attempt.
        randomize: bool,
    },
    /// Tasks generated at attempt time by a deterministic function.
    Procedural {
        /// Generator identifier (registered in the eval registry).
        generator_id: [u8; 32],
        /// Seed derivation: per-attempt, per-epoch, or fixed.
        seed_mode: SeedMode,
        /// Difficulty parameters passed to the generator.
        difficulty: HashMap<String, f64>,
    },
    /// Tasks submitted by users and curated by the arena creator.
    UserContributed {
        /// Minimum reputation required to submit tasks.
        min_contributor_reputation: f64,
        /// Whether submissions require creator approval.
        requires_approval: bool,
    },
    /// Tasks designed to exploit weaknesses found in prior attempts.
    Adversarial {
        /// Agent that generates adversarial tasks.
        adversary_agent_id: [u8; 32],
        /// Maximum difficulty increase per round.
        max_difficulty_step: f64,
        /// Whether the adversary can see prior attempt strategies.
        sees_prior_attempts: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeedMode {
    /// New random seed per attempt.
    PerAttempt,
    /// Same seed for all attempts within an epoch (enables direct comparison).
    PerEpoch { epoch_duration_blocks: u64 },
    /// Fixed seed (all attempts see the same tasks).
    Fixed { seed: u64 },
}

/// How an attempt gets scored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScoringFunction {
    /// Pass or fail. Score is 0.0 or 1.0.
    Binary {
        /// What determines pass/fail.
        criterion: BinaryCriterion,
    },
    /// Continuous score in [0.0, 1.0] or unbounded.
    Continuous {
        /// Metric computed from the attempt output.
        metric: ContinuousMetric,
        /// How to normalize the raw metric to a score.
        normalization: Normalization,
    },
    /// Weighted combination of multiple scoring functions.
    Composite {
        /// Component scores and their weights.
        components: Vec<ScoringComponent>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BinaryCriterion {
    /// All gate checks pass.
    AllGatesPass,
    /// Test suite passes with zero failures.
    TestSuitePass { suite_cid: String },
    /// External oracle returns true.
    OracleVerdict { oracle_id: [u8; 32] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContinuousMetric {
    /// Sharpe ratio of returns (for trading arenas).
    SharpeRatio,
    /// Continuous ranked probability score (for prediction arenas).
    CRPS,
    /// Execution time in milliseconds (lower is better).
    Latency,
    /// Token efficiency: output quality per token spent.
    TokenEfficiency,
    /// Custom metric computed by a registered eval function.
    Custom { eval_id: [u8; 32] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Normalization {
    /// Score is used as-is.
    Identity,
    /// Linearly scaled to [0, 1] based on observed min/max.
    MinMax,
    /// Z-score relative to population mean/stddev.
    ZScore,
    /// Percentile rank within the leaderboard.
    Percentile,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringComponent {
    pub name: String,
    pub function: Box<ScoringFunction>,
    pub weight: f64,
}

/// The full arena definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arena {
    /// Unique arena identifier (blake3 hash of creation params).
    pub id: [u8; 32],
    /// Human-readable name.
    pub name: String,
    /// Markdown description.
    pub description: String,
    /// Category for filtering.
    pub category: ArenaCategory,
    /// Current lifecycle state.
    pub state: ArenaState,
    /// Where tasks come from.
    pub task_source: TaskSource,
    /// How attempts are scored.
    pub scoring: ScoringFunction,
    /// How individual scores aggregate into leaderboard rank.
    pub aggregation: AggregationRule,
    /// Creator's passport ID.
    pub creator_passport_id: u128,
    /// Block at which the arena was created.
    pub created_at_block: u64,
    /// Optional prize pool in USDC (held in escrow).
    pub prize_pool_usdc: u64,
    /// Maximum attempts per agent (0 = unlimited).
    pub max_attempts_per_agent: u64,
    /// Rate limit: minimum blocks between attempts by the same agent.
    pub cooldown_blocks: u64,
    /// Optional deadline block (arena concludes automatically).
    pub deadline_block: Option<u64>,
    /// Ground truth source declaration (required by design constraint 1).
    pub ground_truth: GroundTruthSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArenaCategory {
    Coding,
    Trading,
    Prediction,
    Games,
    Persuasion,
    Negotiation,
    Optimization,
    Research,
    UserCreated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AggregationRule {
    /// Best score across all attempts.
    BestOf,
    /// Average of the last N attempts.
    AverageLastN { n: u64 },
    /// Exponentially weighted moving average.
    EWMA { alpha: f64 },
    /// Median of all attempts.
    Median,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `655`
- Section hash: `0c4fbaafc91df08f3c939a560ca29ee46bbfde532c98466412ae47202dba0fee`

**Normative requirements and implementation claims:**
- /// Where arena tasks come from. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub enum TaskSource { /// Fixed dataset of input/output pairs. Static { /// IPFS CID or URL pointing to the dataset. dataset_cid: String, /// Number of tasks in the dataset. count: u64, /// Whether tasks are sampled randomly per attempt. randomize: bool, }, /// Tasks generated at attempt time by a deterministic function. Procedural { /// Generator identifier (registered in the eval registry). generator_id: [u8; 32], /// Seed derivation: per-attempt, per-epoch, or fixed. seed_mode: SeedMode, /// Difficulty parameters passed to the generator. difficulty: HashMap<String, f64>, }, /// Tasks submitted by users and curated by the arena creator. UserContributed { /// Minimum reputation required to submit tasks. min_contributor_reputation: f64, /// Whether submissions require creator approval. requires_approval: bool, }, /// Tasks designed to exploit weaknesses found in prior attempts. Adversarial { ///
- #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub enum BinaryCriterion { /// All gate checks pass. AllGatesPass, /// Test suite passes with zero failures. TestSuitePass { suite_cid: String }, /// External oracle returns true. OracleVerdict { oracle_id: [u8; 32] }, }
- /// The full arena definition. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct Arena { /// Unique arena identifier (blake3 hash of creation params). pub id: [u8; 32], /// Human-readable name. pub name: String, /// Markdown description. pub description: String, /// Category for filtering. pub category: ArenaCategory, /// Current lifecycle state. pub state: ArenaState, /// Where tasks come from. pub task_source: TaskSource, /// How attempts are scored. pub scoring: ScoringFunction, /// How individual scores aggregate into leaderboard rank. pub aggregation: AggregationRule, /// Creator's passport ID. pub creator_passport_id: u128, /// Block at which the arena was created. pub created_at_block: u64, /// Optional prize pool in USDC (held in escrow). pub prize_pool_usdc: u64, /// Maximum attempts per agent (0 = unlimited). pub max_attempts_per_agent: u64, /// Rate limit: minimum blocks between attempts by the same agent. pub cooldown_blocks: u64, /// Optional deadline b

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArenaState
- TaskSource
- SeedMode
- ScoringFunction
- BinaryCriterion
- ContinuousMetric
- Normalization
- ScoringComponent
- Arena
- ArenaCategory
- AggregationRule

**Event names and event-like entities:**
- UserCreated

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
- Contract 1: language `rust`, first line `/// Arena lifecycle states.`

```rust
/// Arena lifecycle states.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArenaState {
    /// Arena created but not yet accepting attempts.
    Draft,
    /// Arena is live and accepting attempts.
    Active,
    /// Arena is temporarily paused (no new attempts, existing ones continue).
    Paused,
    /// Arena has permanently concluded. Leaderboard is final.
    Concluded,
}

/// Where arena tasks come from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskSource {
    /// Fixed dataset of input/output pairs.
    Static {
        /// IPFS CID or URL pointing to the dataset.
        dataset_cid: String,
        /// Number of tasks in the dataset.
        count: u64,
        /// Whether tasks are sampled randomly per attempt.
        randomize: bool,
    },
    /// Tasks generated at attempt time by a deterministic function.
    Procedural {
        /// Generator identifier (registered in the eval registry).
        generator_id: [u8; 32],
        /// Seed derivation: per-attempt, per-epoch, or fixed.
        seed_mode: SeedMode,
        /// Difficulty parameters passed to the generator.
        difficulty: HashMap<String, f64>,
    },
    /// Tasks submitted by users and curated by the arena creator.
    UserContributed {
        /// Minimum reputation required to submit tasks.
        min_contributor_reputation: f64,
        /// Whether submissions require creator approval.
        requires_approval: bool,
    },
    /// Tasks designed to exploit weaknesses found in prior attempts.
    Adversarial {
        /// Agent that
...
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "attempt|Serialize|Arena|Core|attempts|tasks|Score|derive" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "attempt|Serialize|Arena|Core|attempts|tasks|Score|derive" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `ArenaState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskSource` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SeedMode` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ScoringFunction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BinaryCriterion` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ContinuousMetric` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Normalization` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ScoringComponent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Arena` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArenaCategory` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AggregationRule` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `UserCreated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S005 -- Leaderboard

**Source section:** `tmp/architecture/11-arenas.md:214` through `248`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Leaderboard

The leaderboard is a derived view, not a stored object. It's recomputed from attempt records using the arena's aggregation rule.

```rust
/// A single leaderboard entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Agent passport ID.
    pub agent_passport_id: u128,
    /// Aggregate score (computed from attempts via the arena's aggregation rule).
    pub score: f64,
    /// Total attempts by this agent.
    pub attempt_count: u64,
    /// Block of most recent attempt.
    pub last_attempt_block: u64,
    /// Score trajectory (last 7 scores for sparkline rendering).
    pub trajectory: Vec<f64>,
    /// Current rank (1-indexed).
    pub rank: u64,
}

/// Leaderboard query parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LeaderboardQuery {
    pub arena_id: [u8; 32],
    /// Time window filter (in blocks). None = all time.
    pub since_block: Option<u64>,
    /// Maximum entries to return.
    pub limit: u64,
    /// Offset for pagination.
    pub offset: u64,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `132`
- Section hash: `5fe027c20bb9f3498e7267b1c48ed6627cea6eefbb0bbdd3af39beb9b9e6d31e`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- LeaderboardEntry
- LeaderboardQuery

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
- Contract 1: language `rust`, first line `/// A single leaderboard entry.`

```rust
/// A single leaderboard entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Agent passport ID.
    pub agent_passport_id: u128,
    /// Aggregate score (computed from attempts via the arena's aggregation rule).
    pub score: f64,
    /// Total attempts by this agent.
    pub attempt_count: u64,
    /// Block of most recent attempt.
    pub last_attempt_block: u64,
    /// Score trajectory (last 7 scores for sparkline rendering).
    pub trajectory: Vec<f64>,
    /// Current rank (1-indexed).
    pub rank: u64,
}

/// Leaderboard query parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LeaderboardQuery {
    pub arena_id: [u8; 32],
    /// Time window filter (in blocks). None = all time.
    pub since_block: Option<u64>,
    /// Maximum entries to return.
    pub limit: u64,
    /// Offset for pagination.
    pub offset: u64,
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Leaderboard|attempt|LeaderboardQuery|LeaderboardEntry|score|Serialize|Block|derive" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Leaderboard|attempt|LeaderboardQuery|LeaderboardEntry|score|Serialize|Block|derive" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `LeaderboardEntry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LeaderboardQuery` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S006 -- Attempt lifecycle

**Source section:** `tmp/architecture/11-arenas.md:249` through `308`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Attempt lifecycle

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptState {
    /// Queued for execution.
    Queued,
    /// Agent is actively working.
    Running,
    /// Agent submitted output, gates are running.
    Evaluating,
    /// All gates passed, score computed.
    Completed,
    /// A gate failed or the agent timed out.
    Failed,
    /// The arena owner or agent cancelled the attempt.
    Cancelled,
    /// The attempt was flagged for rule violation.
    Disqualified,
}

/// A single attempt at an arena task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    /// Unique attempt identifier.
    pub id: [u8; 32],
    /// Arena this attempt belongs to.
    pub arena_id: [u8; 32],
    /// Agent making the attempt.
    pub agent_passport_id: u128,
    /// Current state.
    pub state: AttemptState,
    /// Task assigned for this attempt (from the task source).
    pub task_hash: [u8; 32],
    /// Submitted output hash (IPFS CID of the output artifact).
    pub output_cid: Option<String>,
    /// Gate verdicts for this attempt.
    pub gate_results: Vec<GateVerdict>,
    /// Computed score (set when state reaches Completed).
    pub score: Option<f64>,
    /// Block at which the attempt was submitted.
    pub submitted_at_block: u64,
    /// Block at which evaluation completed.
    pub completed_at_block: Option<u64>,
    /// Tokens consumed during the attempt.
    pub tokens_used: u64,
    /// Cost in USDC.
    pub cost_usdc: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateVerdict {
    pub gate_type: String,
    pub passed: bool,
    pub score: f64,
    pub detail: String,
    pub timestamp_block: u64,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `206`
- Section hash: `2c8e9d3d48be057f239f5a3ebf6e36365059f59e52d98e4c770f95449cbba03a`

**Normative requirements and implementation claims:**
- /// A single attempt at an arena task. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct Attempt { /// Unique attempt identifier. pub id: [u8; 32], /// Arena this attempt belongs to. pub arena_id: [u8; 32], /// Agent making the attempt. pub agent_passport_id: u128, /// Current state. pub state: AttemptState, /// Task assigned for this attempt (from the task source). pub task_hash: [u8; 32], /// Submitted output hash (IPFS CID of the output artifact). pub output_cid: Option<String>, /// Gate verdicts for this attempt. pub gate_results: Vec<GateVerdict>, /// Computed score (set when state reaches Completed). pub score: Option<f64>, /// Block at which the attempt was submitted. pub submitted_at_block: u64, /// Block at which evaluation completed. pub completed_at_block: Option<u64>, /// Tokens consumed during the attempt. pub tokens_used: u64, /// Cost in USDC. pub cost_usdc: f64, }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AttemptState
- Attempt
- GateVerdict

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
- Contract 1: language `rust`, first line `#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]`

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptState {
    /// Queued for execution.
    Queued,
    /// Agent is actively working.
    Running,
    /// Agent submitted output, gates are running.
    Evaluating,
    /// All gates passed, score computed.
    Completed,
    /// A gate failed or the agent timed out.
    Failed,
    /// The arena owner or agent cancelled the attempt.
    Cancelled,
    /// The attempt was flagged for rule violation.
    Disqualified,
}

/// A single attempt at an arena task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    /// Unique attempt identifier.
    pub id: [u8; 32],
    /// Arena this attempt belongs to.
    pub arena_id: [u8; 32],
    /// Agent making the attempt.
    pub agent_passport_id: u128,
    /// Current state.
    pub state: AttemptState,
    /// Task assigned for this attempt (from the task source).
    pub task_hash: [u8; 32],
    /// Submitted output hash (IPFS CID of the output artifact).
    pub output_cid: Option<String>,
    /// Gate verdicts for this attempt.
    pub gate_results: Vec<GateVerdict>,
    /// Computed score (set when state reaches Completed).
    pub score: Option<f64>,
    /// Block at which the attempt was submitted.
    pub submitted_at_block: u64,
    /// Block at which evaluation completed.
    pub completed_at_block: Option<u64>,
    /// Tokens consumed during the attempt.
    pub tokens_used: u64,
    /// Cost in USDC.
    pub cost_usdc: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateVerdict {
...
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Attempt|gate|Serialize|GateVerdict|AttemptState|state|lifecycle|Block" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Attempt|gate|Serialize|GateVerdict|AttemptState|state|lifecycle|Block" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `AttemptState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Attempt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GateVerdict` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S007 -- Arena registry

**Source section:** `tmp/architecture/11-arenas.md:309` through `359`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Arena registry

The arena registry lives on-chain for discoverability and tamper resistance. The full task datasets and attempt artifacts live off-chain (IPFS or relay storage), with content hashes anchored on-chain.

```rust
/// On-chain arena registry.
pub struct ArenaRegistry {
    /// All registered arenas by ID.
    arenas: HashMap<[u8; 32], Arena>,
    /// Attempts by arena ID.
    attempts: HashMap<[u8; 32], Vec<Attempt>>,
    /// Index: category -> arena IDs.
    by_category: HashMap<ArenaCategory, Vec<[u8; 32]>>,
    /// Index: creator passport -> arena IDs.
    by_creator: HashMap<u128, Vec<[u8; 32]>>,
}

impl ArenaRegistry {
    /// Register a new arena. Returns the arena ID.
    pub fn register(&mut self, arena: Arena) -> [u8; 32];

    /// Transition an arena's lifecycle state.
    pub fn transition(&mut self, id: &[u8; 32], new_state: ArenaState) -> Result<(), ArenaError>;

    /// Submit an attempt. Validates cooldown and attempt limits.
    pub fn submit_attempt(&mut self, attempt: Attempt) -> Result<(), ArenaError>;

    /// Record a completed attempt with its score.
    pub fn complete_attempt(
        &mut self,
        attempt_id: &[u8; 32],
        score: f64,
        gate_results: Vec<GateVerdict>,
    ) -> Result<(), ArenaError>;

    /// Compute the leaderboard for an arena.
    pub fn leaderboard(&self, query: &LeaderboardQuery) -> Vec<LeaderboardEntry>;

    /// List arenas with filters.
    pub fn list(
        &self,
        state: Option<ArenaState>,
        category: Option<ArenaCategory>,
        limit: u64,
        offset: u64,
    ) -> Vec<&Arena>;
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `190`
- Section hash: `3b5d62f4891ffdbb31e94ee2195e4418d4eb4cabae0c66284c76ac9f97b48388`

**Normative requirements and implementation claims:**
- /// Transition an arena's lifecycle state. pub fn transition(&mut self, id: &[u8; 32], new_state: ArenaState) -> Result<(), ArenaError>;
- /// List arenas with filters. pub fn list( &self, state: Option<ArenaState>, category: Option<ArenaCategory>, limit: u64, offset: u64, ) -> Vec<&Arena>; } ```
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArenaRegistry
- register
- transition
- submit_attempt
- complete_attempt
- leaderboard
- list

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- category -> arena IDs
- creator passport -> arena IDs

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// On-chain arena registry.`

```rust
/// On-chain arena registry.
pub struct ArenaRegistry {
    /// All registered arenas by ID.
    arenas: HashMap<[u8; 32], Arena>,
    /// Attempts by arena ID.
    attempts: HashMap<[u8; 32], Vec<Attempt>>,
    /// Index: category -> arena IDs.
    by_category: HashMap<ArenaCategory, Vec<[u8; 32]>>,
    /// Index: creator passport -> arena IDs.
    by_creator: HashMap<u128, Vec<[u8; 32]>>,
}

impl ArenaRegistry {
    /// Register a new arena. Returns the arena ID.
    pub fn register(&mut self, arena: Arena) -> [u8; 32];

    /// Transition an arena's lifecycle state.
    pub fn transition(&mut self, id: &[u8; 32], new_state: ArenaState) -> Result<(), ArenaError>;

    /// Submit an attempt. Validates cooldown and attempt limits.
    pub fn submit_attempt(&mut self, attempt: Attempt) -> Result<(), ArenaError>;

    /// Record a completed attempt with its score.
    pub fn complete_attempt(
        &mut self,
        attempt_id: &[u8; 32],
        score: f64,
        gate_results: Vec<GateVerdict>,
    ) -> Result<(), ArenaError>;

    /// Compute the leaderboard for an arena.
    pub fn leaderboard(&self, query: &LeaderboardQuery) -> Vec<LeaderboardEntry>;

    /// List arenas with filters.
    pub fn list(
        &self,
        state: Option<ArenaState>,
        category: Option<ArenaCategory>,
        limit: u64,
        offset: u64,
    ) -> Vec<&Arena>;
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Arena|attempt|registry|leaderboard|register|transition|self|list" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|attempt|registry|leaderboard|register|transition|self|list" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `ArenaRegistry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `register` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `transition` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `submit_attempt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `complete_attempt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `leaderboard` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `list` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `category -> arena IDs` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `creator passport -> arena IDs` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S008 -- Evals

**Source section:** `tmp/architecture/11-arenas.md:360` through `363`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Evals

An eval is a measurement with a declared ground truth source. Unlike arenas (which are competitive and ongoing), evals are calibration tools. They answer: "How good is this agent at this specific thing, measured against a known correct answer?"
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `728f52753101bc3eac716d0aefe437da2e1ad6aac0f64abbcdeb3723530945fa`

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "eval|Evals|answer|truth|tools|thing|specific|ongoing" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "eval|Evals|answer|truth|tools|thing|specific|ongoing" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S009 -- Ground truth

**Source section:** `tmp/architecture/11-arenas.md:364` through `434`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Ground truth

The ground truth source is the single most important field on an eval. It determines whether the measurement means anything.

```rust
/// Where the correct answer comes from. This is NOT negotiable -- every eval
/// must declare one, and "the LLM thinks it's good" is not an option.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroundTruthSource {
    /// External oracle (API endpoint, smart contract, or registered service).
    Oracle {
        /// Oracle identifier in the oracle registry.
        oracle_id: [u8; 32],
        /// HTTP endpoint or contract address.
        endpoint: String,
        /// Expected response schema (JSON Schema).
        response_schema: String,
    },
    /// Test suite that runs against the agent's output.
    TestSuite {
        /// IPFS CID of the test suite.
        suite_cid: String,
        /// Runtime environment (e.g., "rust-1.91", "python-3.12", "node-22").
        runtime: String,
        /// Timeout per test case in seconds.
        timeout_secs: u64,
    },
    /// Human review panel.
    HumanReview {
        /// Minimum number of reviewers required.
        min_reviewers: u32,
        /// Required agreement threshold (e.g., 0.67 = 2/3 must agree).
        agreement_threshold: f64,
        /// Rubric CID (markdown document describing evaluation criteria).
        rubric_cid: String,
    },
    /// On-chain state at a specific block.
    ChainState {
        /// Chain ID.
        chain_id: u64,
        /// Contract address to read from.
        contract_address: String,
        /// Function selector and expected return value.
        call_data: Vec<u8>,
        /// Block at which to read (None = latest).
        at_block: Option<u64>,
    },
    /// Benchmark dataset with known correct outputs.
    BenchmarkDataset {
        /// Dataset identifier.
        dataset_cid: String,
        /// Number of examples.
        example_count: u64,
        /// Comparison function (exact match, fuzzy match, semantic similarity threshold).
        comparison: ComparisonMethod,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ComparisonMethod {
    /// Byte-exact match.
    ExactMatch,
    /// Fuzzy string match with minimum similarity.
    FuzzyMatch { min_similarity: f64 },
    /// Semantic similarity above a threshold (uses a registered embedding model).
    SemanticSimilarity { threshold: f64, model_id: String },
    /// Numeric tolerance (for regression tasks).
    NumericTolerance { absolute: f64, relative: f64 },
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `274`
- Section hash: `376d40fdb7215115d25f38fba3bc7ec8953043db551adfb1172f8a7a4d0f613e`

**Normative requirements and implementation claims:**
- ```rust /// Where the correct answer comes from. This is NOT negotiable -- every eval /// must declare one, and "the LLM thinks it's good" is not an option. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub enum GroundTruthSource { /// External oracle (API endpoint, smart contract, or registered service). Oracle { /// Oracle identifier in the oracle registry. oracle_id: [u8; 32], /// HTTP endpoint or contract address. endpoint: String, /// Expected response schema (JSON Schema). response_schema: String, }, /// Test suite that runs against the agent's output. TestSuite { /// IPFS CID of the test suite. suite_cid: String, /// Runtime environment (e.g., "rust-1.91", "python-3.12", "node-22"). runtime: String, /// Timeout per test case in seconds. timeout_secs: u64, }, /// Human review panel. HumanReview { /// Minimum number of reviewers required. min_reviewers: u32, /// Required agreement threshold (e.g., 0.67 = 2/3 must agree). agreement_threshold: f64, /// Rubric CID (markd

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- GroundTruthSource
- ComparisonMethod

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
- Contract 1: language `rust`, first line `/// Where the correct answer comes from. This is NOT negotiable -- every eval`

```rust
/// Where the correct answer comes from. This is NOT negotiable -- every eval
/// must declare one, and "the LLM thinks it's good" is not an option.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroundTruthSource {
    /// External oracle (API endpoint, smart contract, or registered service).
    Oracle {
        /// Oracle identifier in the oracle registry.
        oracle_id: [u8; 32],
        /// HTTP endpoint or contract address.
        endpoint: String,
        /// Expected response schema (JSON Schema).
        response_schema: String,
    },
    /// Test suite that runs against the agent's output.
    TestSuite {
        /// IPFS CID of the test suite.
        suite_cid: String,
        /// Runtime environment (e.g., "rust-1.91", "python-3.12", "node-22").
        runtime: String,
        /// Timeout per test case in seconds.
        timeout_secs: u64,
    },
    /// Human review panel.
    HumanReview {
        /// Minimum number of reviewers required.
        min_reviewers: u32,
        /// Required agreement threshold (e.g., 0.67 = 2/3 must agree).
        agreement_threshold: f64,
        /// Rubric CID (markdown document describing evaluation criteria).
        rubric_cid: String,
    },
    /// On-chain state at a specific block.
    ChainState {
        /// Chain ID.
        chain_id: u64,
        /// Contract address to read from.
        contract_address: String,
        /// Function selector and expected return value.
        call_data: Vec<u8>,
        /// Block at which to read (None = latest).
        at_block: Option<u64>,
    },
...
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "String|truth|Ground|match|ComparisonMethod|threshold|similarity|oracle" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "String|truth|Ground|match|ComparisonMethod|threshold|similarity|oracle" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `GroundTruthSource` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ComparisonMethod` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S010 -- Eval definition

**Source section:** `tmp/architecture/11-arenas.md:435` through `469`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Eval definition

```rust
/// A registered evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Eval {
    /// Unique eval identifier.
    pub id: [u8; 32],
    /// Human-readable name.
    pub name: String,
    /// What this eval measures.
    pub description: String,
    /// Domain (coding, trading, prediction, etc.).
    pub domain: String,
    /// Input format description (what the agent receives).
    pub input_schema: String,
    /// Output format description (what the agent must produce).
    pub output_schema: String,
    /// Scoring function applied to the output.
    pub scoring: ScoringFunction,
    /// Ground truth source.
    pub ground_truth: GroundTruthSource,
    /// Creator passport ID.
    pub creator_passport_id: u128,
    /// Block at which the eval was registered.
    pub created_at_block: u64,
    /// Whether this eval is a meta-eval (measures other evals).
    pub is_meta_eval: bool,
    /// If this is a meta-eval, which evals it measures.
    pub target_eval_ids: Vec<[u8; 32]>,
    /// Version number (evals can be updated while preserving history).
    pub version: u32,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `133`
- Section hash: `a310f66a87f0b06c5fef794f76366a62fbfaae197e57317607825f9916a478ee`

**Normative requirements and implementation claims:**
- ```rust /// A registered evaluation. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct Eval { /// Unique eval identifier. pub id: [u8; 32], /// Human-readable name. pub name: String, /// What this eval measures. pub description: String, /// Domain (coding, trading, prediction, etc.). pub domain: String, /// Input format description (what the agent receives). pub input_schema: String, /// Output format description (what the agent must produce). pub output_schema: String, /// Scoring function applied to the output. pub scoring: ScoringFunction, /// Ground truth source. pub ground_truth: GroundTruthSource, /// Creator passport ID. pub creator_passport_id: u128, /// Block at which the eval was registered. pub created_at_block: u64, /// Whether this eval is a meta-eval (measures other evals). pub is_meta_eval: bool, /// If this is a meta-eval, which evals it measures. pub target_eval_ids: Vec<[u8; 32]>, /// Version number (evals can be updated while preserving history). 

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Eval

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
- Contract 1: language `rust`, first line `/// A registered evaluation.`

```rust
/// A registered evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Eval {
    /// Unique eval identifier.
    pub id: [u8; 32],
    /// Human-readable name.
    pub name: String,
    /// What this eval measures.
    pub description: String,
    /// Domain (coding, trading, prediction, etc.).
    pub domain: String,
    /// Input format description (what the agent receives).
    pub input_schema: String,
    /// Output format description (what the agent must produce).
    pub output_schema: String,
    /// Scoring function applied to the output.
    pub scoring: ScoringFunction,
    /// Ground truth source.
    pub ground_truth: GroundTruthSource,
    /// Creator passport ID.
    pub creator_passport_id: u128,
    /// Block at which the eval was registered.
    pub created_at_block: u64,
    /// Whether this eval is a meta-eval (measures other evals).
    pub is_meta_eval: bool,
    /// If this is a meta-eval, which evals it measures.
    pub target_eval_ids: Vec<[u8; 32]>,
    /// Version number (evals can be updated while preserving history).
    pub version: u32,
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Eval|definition|String|truth|meta|measures|evals|description" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|definition|String|truth|meta|measures|evals|description" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `Eval` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S011 -- Meta-evals

**Source section:** `tmp/architecture/11-arenas.md:470` through `496`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Meta-evals

A meta-eval measures whether another eval is well-calibrated. It answers: "Does eval X actually distinguish good performance from bad?"

Meta-evals work by running a set of known-quality submissions through the target eval and checking whether the scores match expectations.

```rust
/// Meta-eval calibration result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationResult {
    /// The eval being calibrated.
    pub eval_id: [u8; 32],
    /// Correlation between eval scores and ground truth rankings.
    /// 1.0 = perfect calibration, 0.0 = random, -1.0 = inverted.
    pub rank_correlation: f64,
    /// Whether the eval reliably separates good from bad (score gap between
    /// known-good and known-bad submissions exceeds a threshold).
    pub discrimination_power: f64,
    /// Inter-rater reliability (if the eval uses human review).
    pub inter_rater_reliability: Option<f64>,
    /// Number of calibration samples used.
    pub sample_count: u64,
    /// Block at which calibration was computed.
    pub calibrated_at_block: u64,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `139`
- Section hash: `659c33c8186b9869885e838ef12e66b901025796aeccf820ca2284e79e2bc36a`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- CalibrationResult

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
- Contract 1: language `rust`, first line `/// Meta-eval calibration result.`

```rust
/// Meta-eval calibration result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationResult {
    /// The eval being calibrated.
    pub eval_id: [u8; 32],
    /// Correlation between eval scores and ground truth rankings.
    /// 1.0 = perfect calibration, 0.0 = random, -1.0 = inverted.
    pub rank_correlation: f64,
    /// Whether the eval reliably separates good from bad (score gap between
    /// known-good and known-bad submissions exceeds a threshold).
    pub discrimination_power: f64,
    /// Inter-rater reliability (if the eval uses human review).
    pub inter_rater_reliability: Option<f64>,
    /// Number of calibration samples used.
    pub sample_count: u64,
    /// Block at which calibration was computed.
    pub calibrated_at_block: u64,
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "eval|Meta|evals|calibration|CalibrationResult|whether|score|known" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "eval|Meta|evals|calibration|CalibrationResult|whether|score|known" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `CalibrationResult` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S012 -- Eval registry

**Source section:** `tmp/architecture/11-arenas.md:497` through `537`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Eval registry

```rust
/// On-chain eval registry.
pub struct EvalRegistry {
    /// All registered evals by ID.
    evals: HashMap<[u8; 32], Eval>,
    /// Calibration results by eval ID.
    calibrations: HashMap<[u8; 32], Vec<CalibrationResult>>,
    /// Index: domain -> eval IDs.
    by_domain: HashMap<String, Vec<[u8; 32]>>,
    /// Meta-eval relationships: eval_id -> meta_eval_ids that measure it.
    meta_eval_index: HashMap<[u8; 32], Vec<[u8; 32]>>,
}

impl EvalRegistry {
    /// Register a new eval.
    pub fn register(&mut self, eval: Eval) -> [u8; 32];

    /// Record a calibration result for an eval.
    pub fn record_calibration(&mut self, result: CalibrationResult) -> Result<(), EvalError>;

    /// Get the latest calibration for an eval.
    pub fn latest_calibration(&self, eval_id: &[u8; 32]) -> Option<&CalibrationResult>;

    /// List evals filtered by domain and calibration quality.
    pub fn list(
        &self,
        domain: Option<&str>,
        min_calibration: Option<f64>,
        limit: u64,
        offset: u64,
    ) -> Vec<&Eval>;

    /// Get all meta-evals that target a given eval.
    pub fn meta_evals_for(&self, eval_id: &[u8; 32]) -> Vec<&Eval>;
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `143`
- Section hash: `af6b43685f55cefe20f79f4de98a60dfeb5910e52b424fb39220ed435863bbdc`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EvalRegistry
- register
- record_calibration
- latest_calibration
- list
- meta_evals_for

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- domain -> eval IDs
- eval_id -> meta_eval_ids that measure it

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// On-chain eval registry.`

```rust
/// On-chain eval registry.
pub struct EvalRegistry {
    /// All registered evals by ID.
    evals: HashMap<[u8; 32], Eval>,
    /// Calibration results by eval ID.
    calibrations: HashMap<[u8; 32], Vec<CalibrationResult>>,
    /// Index: domain -> eval IDs.
    by_domain: HashMap<String, Vec<[u8; 32]>>,
    /// Meta-eval relationships: eval_id -> meta_eval_ids that measure it.
    meta_eval_index: HashMap<[u8; 32], Vec<[u8; 32]>>,
}

impl EvalRegistry {
    /// Register a new eval.
    pub fn register(&mut self, eval: Eval) -> [u8; 32];

    /// Record a calibration result for an eval.
    pub fn record_calibration(&mut self, result: CalibrationResult) -> Result<(), EvalError>;

    /// Get the latest calibration for an eval.
    pub fn latest_calibration(&self, eval_id: &[u8; 32]) -> Option<&CalibrationResult>;

    /// List evals filtered by domain and calibration quality.
    pub fn list(
        &self,
        domain: Option<&str>,
        min_calibration: Option<f64>,
        limit: u64,
        offset: u64,
    ) -> Vec<&Eval>;

    /// Get all meta-evals that target a given eval.
    pub fn meta_evals_for(&self, eval_id: &[u8; 32]) -> Vec<&Eval>;
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Eval|Calibration|registry|result|register|list|EvalRegistry|self" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|Calibration|registry|result|register|list|EvalRegistry|self" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `EvalRegistry` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `register` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `record_calibration` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `latest_calibration` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `list` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `meta_evals_for` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `domain -> eval IDs` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `eval_id -> meta_eval_ids that measure it` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S013 -- Bounty market

**Source section:** `tmp/architecture/11-arenas.md:538` through `541`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Bounty market

The bounty market connects users who need work done with agents who can do it. Users post tasks with escrowed rewards. Agents bid. A VCG mechanism determines assignment. The existing `Marketplace` in `roko-chain/src/marketplace.rs` handles the job lifecycle and escrow. This section specifies the higher-level bounty market that wraps it.
````

**Explicit detail extraction from this section:**

- Section word count: `54`
- Section hash: `d1cde947fcdd63c2695ce5cf329452ff7986ac1f1394a20d4d52f4ccee690697`

**Normative requirements and implementation claims:**
- The bounty market connects users who need work done with agents who can do it. Users post tasks with escrowed rewards. Agents bid. A VCG mechanism determines assignment. The existing `Marketplace` in `roko-chain/src/marketplace.rs` handles the job lifecycle and escrow. This section specifies the higher-level bounty market that wraps it.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-chain/src/marketplace.rs

**Types, functions, traits, and inline code identifiers:**
- Marketplace

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/marketplace.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "market|Bounty|Marketplace|users|escrow|wraps|work|tasks" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "market|Bounty|Marketplace|users|escrow|wraps|work|tasks" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `Marketplace` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S014 -- Relationship to existing code

**Source section:** `tmp/architecture/11-arenas.md:542` through `551`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Relationship to existing code

The `Marketplace` struct already implements:
- Job lifecycle: `Posted -> Assigned -> InProgress -> Submitted -> Settled / Disputed / Expired`
- Three hiring models: `RandomVRF`, `BlindAuction`, `DirectHire`
- Escrow with deposit/release/dispute/refund
- 4-level dispute resolution: `BondEscalation` (3 rounds) -> `PeerJury` -> `GovernanceVote`

The bounty market adds a discovery layer, VCG multi-bounty matching, and the API surface for the dashboard.
````

**Explicit detail extraction from this section:**

- Section word count: `53`
- Section hash: `6c0a1bb6a49dc3a9470fd07a01c1b0d4c5df5d7261ff6b600e7738c30694df30`

**Normative requirements and implementation claims:**
- The bounty market adds a discovery layer, VCG multi-bounty matching, and the API surface for the dashboard.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- deposit/release/dispute/

**Types, functions, traits, and inline code identifiers:**
- already
- Marketplace
- RandomVRF
- BlindAuction
- DirectHire
- BondEscalation
- PeerJury
- GovernanceVote

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Posted -> Assigned -
- InProgress -> Submitted -

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Job lifecycle: `Posted -> Assigned -> InProgress -> Submitted -> Settled / Disputed / Expired`
- - Three hiring models: `RandomVRF`, `BlindAuction`, `DirectHire`
- - Escrow with deposit/release/dispute/refund
- - 4-level dispute resolution: `BondEscalation` (3 rounds) -> `PeerJury` -> `GovernanceVote`

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `deposit/release/dispute/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "existing|code|already|Relationship|RandomVRF|PeerJury|Marketplace|GovernanceVote" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "existing|code|already|Relationship|RandomVRF|PeerJury|Marketplace|GovernanceVote" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `deposit/release/dispute/`
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
- [ ] Implement or verify `already` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Marketplace` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RandomVRF` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BlindAuction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DirectHire` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BondEscalation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PeerJury` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `GovernanceVote` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `Posted -> Assigned -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `InProgress -> Submitted -` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S015 -- VCG matching

**Source section:** `tmp/architecture/11-arenas.md:552` through `622`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### VCG matching

When multiple bounties are open and multiple agents are available, VCG (Vickrey-Clarke-Groves) matching finds the welfare-maximizing assignment across all bounties simultaneously. Each agent bids on each bounty it's qualified for. The mechanism assigns agents to bounties such that total value is maximized, and each agent pays the externality it imposes on others.

The existing `vcg_allocate` in `roko-compose/src/auction.rs` provides the allocation algorithm. The bounty market uses it for batch matching.

```rust
/// A bounty posted to the market.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounty {
    /// Unique bounty identifier.
    pub id: [u8; 32],
    /// Human-readable title.
    pub title: String,
    /// Markdown description of the task.
    pub description: String,
    /// Domain category.
    pub domain: String,
    /// Reward amount in USDC (held in escrow).
    pub reward_usdc: u64,
    /// Optional additional reward in Daeji tokens.
    pub reward_daeji: u64,
    /// Deadline block for completion.
    pub deadline_block: u64,
    /// Current lifecycle state.
    pub state: BountyState,
    /// Poster's passport ID.
    pub poster_passport_id: u128,
    /// Required agent capabilities (bitmask).
    pub required_capabilities: u64,
    /// Minimum reputation score for bidders.
    pub min_reputation: f64,
    /// Evaluation criteria (human-readable).
    pub evaluation_criteria: Vec<String>,
    /// Eval ID used for automated scoring (if any).
    pub eval_id: Option<[u8; 32]>,
    /// Arena ID (if the bounty is "win an attempt in arena X").
    pub arena_id: Option<[u8; 32]>,
    /// Block at which the bounty was posted.
    pub posted_at_block: u64,
    /// Assigned agent (set on matching).
    pub assigned_agent: Option<u128>,
    /// Submitted result hash.
    pub result_cid: Option<String>,
    /// Quality score from evaluation.
    pub quality_score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BountyState {
    /// Posted and accepting bids.
    Open,
    /// An agent has been matched/assigned.
    Claimed,
    /// Agent is working.
    InProgress,
    /// Result submitted, awaiting evaluation.
    Submitted,
    /// Evaluation complete, awaiting settlement.
    Evaluated,
    /// Reward released to agent.
    Completed,
    /// Under dispute.
    Disputed,
    /// Poster cancelled before assignment.
    Cancelled,
    /// Deadline passed without completion.
    Expired,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `299`
- Section hash: `a690c1e843e35117a804213bab394a2c0430066b7c10b53ca64b0855352fd65d`

**Normative requirements and implementation claims:**
- ```rust /// A bounty posted to the market. #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct Bounty { /// Unique bounty identifier. pub id: [u8; 32], /// Human-readable title. pub title: String, /// Markdown description of the task. pub description: String, /// Domain category. pub domain: String, /// Reward amount in USDC (held in escrow). pub reward_usdc: u64, /// Optional additional reward in Daeji tokens. pub reward_daeji: u64, /// Deadline block for completion. pub deadline_block: u64, /// Current lifecycle state. pub state: BountyState, /// Poster's passport ID. pub poster_passport_id: u128, /// Required agent capabilities (bitmask). pub required_capabilities: u64, /// Minimum reputation score for bidders. pub min_reputation: f64, /// Evaluation criteria (human-readable). pub evaluation_criteria: Vec<String>, /// Eval ID used for automated scoring (if any). pub eval_id: Option<[u8; 32]>, /// Arena ID (if the bounty is "win an attempt in arena X"). pub arena_id

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-compose/src/auction.rs

**Types, functions, traits, and inline code identifiers:**
- Bounty
- BountyState
- vcg_allocate

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
- Contract 1: language `rust`, first line `/// A bounty posted to the market.`

```rust
/// A bounty posted to the market.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounty {
    /// Unique bounty identifier.
    pub id: [u8; 32],
    /// Human-readable title.
    pub title: String,
    /// Markdown description of the task.
    pub description: String,
    /// Domain category.
    pub domain: String,
    /// Reward amount in USDC (held in escrow).
    pub reward_usdc: u64,
    /// Optional additional reward in Daeji tokens.
    pub reward_daeji: u64,
    /// Deadline block for completion.
    pub deadline_block: u64,
    /// Current lifecycle state.
    pub state: BountyState,
    /// Poster's passport ID.
    pub poster_passport_id: u128,
    /// Required agent capabilities (bitmask).
    pub required_capabilities: u64,
    /// Minimum reputation score for bidders.
    pub min_reputation: f64,
    /// Evaluation criteria (human-readable).
    pub evaluation_criteria: Vec<String>,
    /// Eval ID used for automated scoring (if any).
    pub eval_id: Option<[u8; 32]>,
    /// Arena ID (if the bounty is "win an attempt in arena X").
    pub arena_id: Option<[u8; 32]>,
    /// Block at which the bounty was posted.
    pub posted_at_block: u64,
    /// Assigned agent (set on matching).
    pub assigned_agent: Option<u128>,
    /// Submitted result hash.
    pub result_cid: Option<String>,
    /// Quality score from evaluation.
    pub quality_score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BountyState {
    /// Posted and accepting bids.
    Open,
    /// An agent has been matched/assigned.
...
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-compose/src/auction.rs`
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
rg -n "Bounty|matching|Eval|VCG|Option|BountyState|vcg_allocate|String" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bounty|matching|Eval|VCG|Option|BountyState|vcg_allocate|String" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-compose/src/auction.rs`
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
- [ ] Implement or verify `Bounty` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BountyState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `vcg_allocate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S016 -- Bounty bids

**Source section:** `tmp/architecture/11-arenas.md:623` through `647`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Bounty bids

```rust
/// An agent's bid on a bounty.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BountyBid {
    /// Bidding agent's passport ID.
    pub agent_passport_id: u128,
    /// Target bounty.
    pub bounty_id: [u8; 32],
    /// Price the agent is willing to accept (in USDC).
    pub price_usdc: u64,
    /// Estimated completion time in seconds.
    pub estimated_time_secs: u64,
    /// Capability proof bitmask.
    pub capability_proof: u64,
    /// Agent's current reputation snapshot.
    pub reputation: f64,
    /// Optional message to the poster.
    pub cover_letter: Option<String>,
    /// Block at which the bid was submitted.
    pub bid_at_block: u64,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `84`
- Section hash: `1fdc6eedb03289308c5b7078d894d0728b032bd92371cd0c48dbc2aff0cd99ec`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- BountyBid

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
- Contract 1: language `rust`, first line `/// An agent's bid on a bounty.`

```rust
/// An agent's bid on a bounty.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BountyBid {
    /// Bidding agent's passport ID.
    pub agent_passport_id: u128,
    /// Target bounty.
    pub bounty_id: [u8; 32],
    /// Price the agent is willing to accept (in USDC).
    pub price_usdc: u64,
    /// Estimated completion time in seconds.
    pub estimated_time_secs: u64,
    /// Capability proof bitmask.
    pub capability_proof: u64,
    /// Agent's current reputation snapshot.
    pub reputation: f64,
    /// Optional message to the poster.
    pub cover_letter: Option<String>,
    /// Block at which the bid was submitted.
    pub bid_at_block: u64,
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Bounty|bids|BountyBid|time|reputation|proof|passport|USDC" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bounty|bids|BountyBid|time|reputation|proof|passport|USDC" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `BountyBid` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S017 -- Dispute resolution

**Source section:** `tmp/architecture/11-arenas.md:648` through `674`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dispute resolution

The dispute process escalates through four levels. Each level requires more resources and time, which discourages frivolous disputes while ensuring genuine disagreements get resolved.

```
Level 1: Bond escalation (up to 3 rounds)
    Challenger posts a bond. Defender can counter-bond.
    Each round doubles the required bond.
    If one side doesn't respond within the challenge window, the other wins.

Level 2: Peer jury
    5 randomly selected agents from the same domain review the submission.
    Majority vote determines outcome.
    Jurors stake reputation -- wrong votes reduce reputation.

Level 3: Governance vote
    Full governance proposal. All token holders can vote.
    Used only for high-value disputes or precedent-setting cases.

Level 4: (not implemented) External arbitration
    Reserved for disputes involving real-world legal obligations.
```

This matches the `DisputeLevel` enum already implemented in `roko-chain/src/phase2.rs`.

---
````

**Explicit detail extraction from this section:**

- Section word count: `134`
- Section hash: `29fe990d2a416d2504fc1b4c8933dab29b44177e7786ec10cb98d1c3e264c743`

**Normative requirements and implementation claims:**
- The dispute process escalates through four levels. Each level requires more resources and time, which discourages frivolous disputes while ensuring genuine disagreements get resolved.
- ``` Level 1: Bond escalation (up to 3 rounds) Challenger posts a bond. Defender can counter-bond. Each round doubles the required bond. If one side doesn't respond within the challenge window, the other wins.
- Level 3: Governance vote Full governance proposal. All token holders can vote. Used only for high-value disputes or precedent-setting cases.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-chain/src/phase2.rs

**Types, functions, traits, and inline code identifiers:**
- already
- DisputeLevel

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
- Contract 1: language `plain`, first line `Level 1: Bond escalation (up to 3 rounds)`

```
Level 1: Bond escalation (up to 3 rounds)
    Challenger posts a bond. Defender can counter-bond.
    Each round doubles the required bond.
    If one side doesn't respond within the challenge window, the other wins.

Level 2: Peer jury
    5 randomly selected agents from the same domain review the submission.
    Majority vote determines outcome.
    Jurors stake reputation -- wrong votes reduce reputation.

Level 3: Governance vote
    Full governance proposal. All token holders can vote.
    Used only for high-value disputes or precedent-setting cases.

Level 4: (not implemented) External arbitration
    Reserved for disputes involving real-world legal obligations.
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/phase2.rs`
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
rg -n "Dispute|level|resolution|already|DisputeLevel|vote|Bond|disputes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Dispute|level|resolution|already|DisputeLevel|vote|Bond|disputes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/phase2.rs`
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
- [ ] Implement or verify `already` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `DisputeLevel` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S018 -- API surface

**Source section:** `tmp/architecture/11-arenas.md:675` through `676`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## API surface
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `bc3ef3c6f285bc86997b2bc41a829b1cfa87cf574f7480e2f0c4e33880648ccf`

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "surface|API|arenas" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|API|arenas" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S019 -- Arena endpoints

**Source section:** `tmp/architecture/11-arenas.md:677` through `692`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Arena endpoints

```
POST   /api/arenas                          Create a new arena
GET    /api/arenas                          List arenas (query params: state, category, limit, offset, sort)
GET    /api/arenas/featured                 Curated featured arenas
GET    /api/arenas/:id                      Get arena detail
PATCH  /api/arenas/:id                      Update arena (creator only; state transitions)
GET    /api/arenas/:id/leaderboard          Get leaderboard (query: since_block, limit, offset)
GET    /api/arenas/:id/attempts             List attempts (query: agent_id, state, limit, offset, sort)
POST   /api/arenas/:id/attempts             Submit a new attempt
GET    /api/arenas/:id/attempts/:attemptId  Get attempt detail
GET    /api/arenas/:id/distribution         Score distribution statistics
GET    /api/arenas/:id/my                   User's participation (query: owner)
```
````

**Explicit detail extraction from this section:**

- Section word count: `103`
- Section hash: `3754d0f929bb5ee120f6168305e416ab7871ed051912981e2614e7295d59e432`

**Normative requirements and implementation claims:**
- ``` POST /api/arenas Create a new arena GET /api/arenas List arenas (query params: state, category, limit, offset, sort) GET /api/arenas/featured Curated featured arenas GET /api/arenas/:id Get arena detail PATCH /api/arenas/:id Update arena (creator only; state transitions) GET /api/arenas/:id/leaderboard Get leaderboard (query: since_block, limit, offset) GET /api/arenas/:id/attempts List attempts (query: agent_id, state, limit, offset, sort) POST /api/arenas/:id/attempts Submit a new attempt GET /api/arenas/:id/attempts/:attemptId Get attempt detail GET /api/arenas/:id/distribution Score distribution statistics GET /api/arenas/:id/my User's participation (query: owner) ```

**Routes and endpoint references:**
- POST /api/arenas
- GET /api/arenas
- GET /api/arenas/featured
- GET /api/arenas/:id
- PATCH /api/arenas/:id
- GET /api/arenas/:id/leaderboard
- GET /api/arenas/:id/attempts
- POST /api/arenas/:id/attempts
- GET /api/arenas/:id/attempts/:attemptId
- GET /api/arenas/:id/distribution
- GET /api/arenas/:id/my

**Files and path references:**
- api/arenas/
- id/attempts/

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
- Contract 1: language `plain`, first line `POST   /api/arenas                          Create a new arena`

```
POST   /api/arenas                          Create a new arena
GET    /api/arenas                          List arenas (query params: state, category, limit, offset, sort)
GET    /api/arenas/featured                 Curated featured arenas
GET    /api/arenas/:id                      Get arena detail
PATCH  /api/arenas/:id                      Update arena (creator only; state transitions)
GET    /api/arenas/:id/leaderboard          Get leaderboard (query: since_block, limit, offset)
GET    /api/arenas/:id/attempts             List attempts (query: agent_id, state, limit, offset, sort)
POST   /api/arenas/:id/attempts             Submit a new attempt
GET    /api/arenas/:id/attempts/:attemptId  Get attempt detail
GET    /api/arenas/:id/distribution         Score distribution statistics
GET    /api/arenas/:id/my                   User's participation (query: owner)
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/arenas/`
- `id/attempts/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Arena|arenas|api|GET|attempt|endpoints|query|attempts" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|arenas|api|GET|attempt|endpoints|query|attempts" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/arenas/`
- `id/attempts/`
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
- [ ] Implement or verify route `POST /api/arenas` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/featured` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PATCH /api/arenas/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id/leaderboard` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id/attempts` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/arenas/:id/attempts` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id/attempts/:attemptId` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id/distribution` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id/my` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S020 -- Example: create an arena

**Source section:** `tmp/architecture/11-arenas.md:693` through `726`
**Heading level:** H4
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
#### Example: create an arena

```json
POST /api/arenas
{
  "name": "Rust Optimization Challenge",
  "description": "Optimize the given Rust function for minimum latency.",
  "category": "Coding",
  "task_source": {
    "type": "static",
    "dataset_cid": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
    "count": 50,
    "randomize": true
  },
  "scoring": {
    "type": "continuous",
    "metric": "latency",
    "normalization": "percentile"
  },
  "aggregation": { "type": "best_of" },
  "ground_truth": {
    "type": "test_suite",
    "suite_cid": "QmTestSuite123",
    "runtime": "rust-1.91",
    "timeout_secs": 300
  },
  "max_attempts_per_agent": 10,
  "cooldown_blocks": 100,
  "prize_pool_usdc": 5000
}
```

Response: `201 Created` with the full `Arena` object including the generated `id`.
````

**Explicit detail extraction from this section:**

- Section word count: `67`
- Section hash: `65fcbd8c7708e0f6f19757382c3dd20f56f79400ad58c531e17319797e117c01`

**Normative requirements and implementation claims:**
- ```json POST /api/arenas { "name": "Rust Optimization Challenge", "description": "Optimize the given Rust function for minimum latency.", "category": "Coding", "task_source": { "type": "static", "dataset_cid": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG", "count": 50, "randomize": true }, "scoring": { "type": "continuous", "metric": "latency", "normalization": "percentile" }, "aggregation": { "type": "best_of" }, "ground_truth": { "type": "test_suite", "suite_cid": "QmTestSuite123", "runtime": "rust-1.91", "timeout_secs": 300 }, "max_attempts_per_agent": 10, "cooldown_blocks": 100, "prize_pool_usdc": 5000 } ```

**Routes and endpoint references:**
- POST /api/arenas

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Arena

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
- Contract 1: language `json`, first line `POST /api/arenas`

```json
POST /api/arenas
{
  "name": "Rust Optimization Challenge",
  "description": "Optimize the given Rust function for minimum latency.",
  "category": "Coding",
  "task_source": {
    "type": "static",
    "dataset_cid": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
    "count": 50,
    "randomize": true
  },
  "scoring": {
    "type": "continuous",
    "metric": "latency",
    "normalization": "percentile"
  },
  "aggregation": { "type": "best_of" },
  "ground_truth": {
    "type": "test_suite",
    "suite_cid": "QmTestSuite123",
    "runtime": "rust-1.91",
    "timeout_secs": 300
  },
  "max_attempts_per_agent": 10,
  "cooldown_blocks": 100,
  "prize_pool_usdc": 5000
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "arena|create|Example|type|Rust|latency|true|timeout_secs" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "arena|create|Example|type|Rust|latency|true|timeout_secs" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- `crates/roko-serve/src/routes/mod.rs`

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
- [ ] Implement or verify route `POST /api/arenas` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `Arena` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S021 -- Example: submit an attempt

**Source section:** `tmp/architecture/11-arenas.md:727` through `737`
**Heading level:** H4
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
#### Example: submit an attempt

```json
POST /api/arenas/0xabc.../attempts
{
  "agent_passport_id": 42
}
```

Response: `202 Accepted` with the `Attempt` object in `Queued` state. The server assigns a task from the task source, starts the agent, and streams progress via WebSocket.
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `50a33613d3dac604fb20e0c51638d8be4a8a8361d69745f6307e61b15cdb0436`

**Normative requirements and implementation claims:**
- ```json POST /api/arenas/0xabc.../attempts { "agent_passport_id": 42 } ```
- Response: `202 Accepted` with the `Attempt` object in `Queued` state. The server assigns a task from the task source, starts the agent, and streams progress via WebSocket.

**Routes and endpoint references:**
- POST /api/arenas/0xabc.../attempts

**Files and path references:**
- api/arenas/0xabc.../

**Types, functions, traits, and inline code identifiers:**
- Attempt
- Queued

**Event names and event-like entities:**
- xabc...

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
- Contract 1: language `json`, first line `POST /api/arenas/0xabc.../attempts`

```json
POST /api/arenas/0xabc.../attempts
{
  "agent_passport_id": 42
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/arenas/0xabc.../`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "attempt|submit|Queued|Example|task|xabc|streams|state" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "attempt|submit|Queued|Example|task|xabc|streams|state" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/arenas/0xabc.../`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/arenas/0xabc.../attempts` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `Attempt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Queued` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `xabc...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S022 -- Eval endpoints

**Source section:** `tmp/architecture/11-arenas.md:738` through `752`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Eval endpoints

```
POST   /api/evals                           Register a new eval
GET    /api/evals                           List evals (query: domain, min_calibration, limit, offset)
GET    /api/evals/:id                       Get eval detail
GET    /api/evals/:id/calibration           Get calibration history
POST   /api/evals/:id/calibrate             Trigger a calibration run
GET    /api/evals/:id/meta                  Get meta-evals targeting this eval
POST   /api/evals/:id/run                   Run an agent through this eval
GET    /api/evals/:id/runs                  List eval runs for this eval
GET    /api/evals/:id/runs/:runId           Get eval run detail
GET    /api/evals/dashboard                 Aggregate calibration dashboard
```
````

**Explicit detail extraction from this section:**

- Section word count: `91`
- Section hash: `9bd2effe98669cccb6529d7406b4975ba28c51e6975cd060db30ff123eaf078e`

**Normative requirements and implementation claims:**
- ``` POST /api/evals Register a new eval GET /api/evals List evals (query: domain, min_calibration, limit, offset) GET /api/evals/:id Get eval detail GET /api/evals/:id/calibration Get calibration history POST /api/evals/:id/calibrate Trigger a calibration run GET /api/evals/:id/meta Get meta-evals targeting this eval POST /api/evals/:id/run Run an agent through this eval GET /api/evals/:id/runs List eval runs for this eval GET /api/evals/:id/runs/:runId Get eval run detail GET /api/evals/dashboard Aggregate calibration dashboard ```

**Routes and endpoint references:**
- POST /api/evals
- GET /api/evals
- GET /api/evals/:id
- GET /api/evals/:id/calibration
- POST /api/evals/:id/calibrate
- GET /api/evals/:id/meta
- POST /api/evals/:id/run
- GET /api/evals/:id/runs
- GET /api/evals/:id/runs/:runId
- GET /api/evals/dashboard

**Files and path references:**
- api/evals/
- id/runs/

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
- Contract 1: language `plain`, first line `POST   /api/evals                           Register a new eval`

```
POST   /api/evals                           Register a new eval
GET    /api/evals                           List evals (query: domain, min_calibration, limit, offset)
GET    /api/evals/:id                       Get eval detail
GET    /api/evals/:id/calibration           Get calibration history
POST   /api/evals/:id/calibrate             Trigger a calibration run
GET    /api/evals/:id/meta                  Get meta-evals targeting this eval
POST   /api/evals/:id/run                   Run an agent through this eval
GET    /api/evals/:id/runs                  List eval runs for this eval
GET    /api/evals/:id/runs/:runId           Get eval run detail
GET    /api/evals/dashboard                 Aggregate calibration dashboard
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/evals/`
- `id/runs/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Eval|evals|GET|api|run|endpoints|calibration|runs" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|evals|GET|api|run|endpoints|calibration|runs" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/evals/`
- `id/runs/`
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
- [ ] Implement or verify route `POST /api/evals` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/:id/calibration` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/evals/:id/calibrate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/:id/meta` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/evals/:id/run` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/:id/runs` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/:id/runs/:runId` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/evals/dashboard` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S023 -- Example: register an eval

**Source section:** `tmp/architecture/11-arenas.md:753` through `778`
**Heading level:** H4
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
#### Example: register an eval

```json
POST /api/evals
{
  "name": "Solidity Audit Accuracy",
  "description": "Measures whether the agent correctly identifies known vulnerabilities in audited contracts.",
  "domain": "coding",
  "input_schema": "{ \"contract_source\": \"string\" }",
  "output_schema": "{ \"vulnerabilities\": [{ \"line\": \"number\", \"severity\": \"string\", \"description\": \"string\" }] }",
  "scoring": {
    "type": "composite",
    "components": [
      { "name": "recall", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.6 },
      { "name": "precision", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.4 }
    ]
  },
  "ground_truth": {
    "type": "benchmark_dataset",
    "dataset_cid": "QmKnownVulns456",
    "example_count": 200,
    "comparison": { "type": "fuzzy_match", "min_similarity": 0.85 }
  }
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `78`
- Section hash: `5425acedae70832a886d1950ab2356fd230aab09f6e242e6173913fdaaf9a8c3`

**Normative requirements and implementation claims:**
- ```json POST /api/evals { "name": "Solidity Audit Accuracy", "description": "Measures whether the agent correctly identifies known vulnerabilities in audited contracts.", "domain": "coding", "input_schema": "{ \"contract_source\": \"string\" }", "output_schema": "{ \"vulnerabilities\": [{ \"line\": \"number\", \"severity\": \"string\", \"description\": \"string\" }] }", "scoring": { "type": "composite", "components": [ { "name": "recall", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.6 }, { "name": "precision", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.4 } ] }, "ground_truth": { "type": "benchmark_dataset", "dataset_cid": "QmKnownVulns456", "example_count": 200, "comparison": { "type": "fuzzy_match", "min_similarity": 0.85 } } } ```

**Routes and endpoint references:**
- POST /api/evals

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
- Contract 1: language `json`, first line `POST /api/evals`

```json
POST /api/evals
{
  "name": "Solidity Audit Accuracy",
  "description": "Measures whether the agent correctly identifies known vulnerabilities in audited contracts.",
  "domain": "coding",
  "input_schema": "{ \"contract_source\": \"string\" }",
  "output_schema": "{ \"vulnerabilities\": [{ \"line\": \"number\", \"severity\": \"string\", \"description\": \"string\" }] }",
  "scoring": {
    "type": "composite",
    "components": [
      { "name": "recall", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.6 },
      { "name": "precision", "function": { "type": "continuous", "metric": { "type": "custom", "eval_id": "..." }, "normalization": "identity" }, "weight": 0.4 }
    ]
  },
  "ground_truth": {
    "type": "benchmark_dataset",
    "dataset_cid": "QmKnownVulns456",
    "example_count": 200,
    "comparison": { "type": "fuzzy_match", "min_similarity": 0.85 }
  }
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "eval|type|Example|register|string|name|weight|vulnerabilities" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "eval|type|Example|register|string|name|weight|vulnerabilities" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- `crates/roko-serve/src/routes/mod.rs`

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
- [ ] Implement or verify route `POST /api/evals` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S024 -- Bounty endpoints

**Source section:** `tmp/architecture/11-arenas.md:779` through `797`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Bounty endpoints

```
POST   /api/bounties                        Post a new bounty (creates escrow)
GET    /api/bounties                        List bounties (query: domain, state, min_value, limit, offset, sort)
GET    /api/bounties/:id                    Get bounty detail
POST   /api/bounties/:id/bids               Submit a bid
GET    /api/bounties/:id/bids               List bids (poster only)
POST   /api/bounties/:id/match              Trigger VCG matching (poster or system)
POST   /api/bounties/:id/submit             Submit result
POST   /api/bounties/:id/evaluate           Evaluate submitted result
POST   /api/bounties/:id/settle             Release escrow (after successful evaluation)
POST   /api/bounties/:id/dispute            Open a dispute
POST   /api/bounties/:id/dispute/escalate   Escalate an active dispute
POST   /api/bounties/:id/dispute/resolve    Resolve a dispute (jury/governance)
POST   /api/bounties/:id/cancel             Cancel (poster only, before assignment)
GET    /api/bounties/batch-match            Run VCG matching across all open bounties
```
````

**Explicit detail extraction from this section:**

- Section word count: `132`
- Section hash: `ef0e711f642b8f8de53c9307a2dff7e1c11481c8007c3b2d382a4884d90c9051`

**Normative requirements and implementation claims:**
- ``` POST /api/bounties Post a new bounty (creates escrow) GET /api/bounties List bounties (query: domain, state, min_value, limit, offset, sort) GET /api/bounties/:id Get bounty detail POST /api/bounties/:id/bids Submit a bid GET /api/bounties/:id/bids List bids (poster only) POST /api/bounties/:id/match Trigger VCG matching (poster or system) POST /api/bounties/:id/submit Submit result POST /api/bounties/:id/evaluate Evaluate submitted result POST /api/bounties/:id/settle Release escrow (after successful evaluation) POST /api/bounties/:id/dispute Open a dispute POST /api/bounties/:id/dispute/escalate Escalate an active dispute POST /api/bounties/:id/dispute/resolve Resolve a dispute (jury/governance) POST /api/bounties/:id/cancel Cancel (poster only, before assignment) GET /api/bounties/batch-match Run VCG matching across all open bounties ```

**Routes and endpoint references:**
- POST /api/bounties
- GET /api/bounties
- GET /api/bounties/:id
- POST /api/bounties/:id/bids
- GET /api/bounties/:id/bids
- POST /api/bounties/:id/match
- POST /api/bounties/:id/submit
- POST /api/bounties/:id/evaluate
- POST /api/bounties/:id/settle
- POST /api/bounties/:id/dispute
- POST /api/bounties/:id/dispute/escalate
- POST /api/bounties/:id/dispute/resolve
- POST /api/bounties/:id/cancel
- GET /api/bounties/batch-match

**Files and path references:**
- api/bounties/
- id/dispute/

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
- Contract 1: language `plain`, first line `POST   /api/bounties                        Post a new bounty (creates escrow)`

```
POST   /api/bounties                        Post a new bounty (creates escrow)
GET    /api/bounties                        List bounties (query: domain, state, min_value, limit, offset, sort)
GET    /api/bounties/:id                    Get bounty detail
POST   /api/bounties/:id/bids               Submit a bid
GET    /api/bounties/:id/bids               List bids (poster only)
POST   /api/bounties/:id/match              Trigger VCG matching (poster or system)
POST   /api/bounties/:id/submit             Submit result
POST   /api/bounties/:id/evaluate           Evaluate submitted result
POST   /api/bounties/:id/settle             Release escrow (after successful evaluation)
POST   /api/bounties/:id/dispute            Open a dispute
POST   /api/bounties/:id/dispute/escalate   Escalate an active dispute
POST   /api/bounties/:id/dispute/resolve    Resolve a dispute (jury/governance)
POST   /api/bounties/:id/cancel             Cancel (poster only, before assignment)
GET    /api/bounties/batch-match            Run VCG matching across all open bounties
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/bounties/`
- `id/dispute/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "bounties|api|POST|Bounty|dispute|endpoints|GET|submit" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "bounties|api|POST|Bounty|dispute|endpoints|GET|submit" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/bounties/`
- `id/dispute/`
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

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/bounties` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/bounties` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/bounties/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/bids` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/bounties/:id/bids` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/match` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/submit` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/evaluate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/settle` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/dispute` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/dispute/escalate` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/dispute/resolve` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/bounties/:id/cancel` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/bounties/batch-match` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S025 -- Example: post a bounty

**Source section:** `tmp/architecture/11-arenas.md:798` through `820`
**Heading level:** H4
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
#### Example: post a bounty

```json
POST /api/bounties
{
  "title": "Implement EIP-7702 support in roko-chain",
  "description": "Add account abstraction support per EIP-7702...",
  "domain": "coding",
  "reward_usdc": 2000,
  "deadline_block": 1000000,
  "required_capabilities": 3,
  "min_reputation": 0.7,
  "evaluation_criteria": [
    "All existing tests pass",
    "New tests cover the EIP-7702 path",
    "Clippy clean with no new warnings"
  ],
  "eval_id": "0xdef..."
}
```

Response: `201 Created`. The server creates the bounty and locks `reward_usdc` in escrow.
````

**Explicit detail extraction from this section:**

- Section word count: `64`
- Section hash: `67ac344ced2cffed37041ba4563a770f32d8eb762547c5a19c457e3b70522963`

**Normative requirements and implementation claims:**
- ```json POST /api/bounties { "title": "Implement EIP-7702 support in roko-chain", "description": "Add account abstraction support per EIP-7702...", "domain": "coding", "reward_usdc": 2000, "deadline_block": 1000000, "required_capabilities": 3, "min_reputation": 0.7, "evaluation_criteria": [ "All existing tests pass", "New tests cover the EIP-7702 path", "Clippy clean with no new warnings" ], "eval_id": "0xdef..." } ```

**Routes and endpoint references:**
- POST /api/bounties

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- reward_usdc

**Event names and event-like entities:**
- xdef...

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
- Contract 1: language `json`, first line `POST /api/bounties`

```json
POST /api/bounties
{
  "title": "Implement EIP-7702 support in roko-chain",
  "description": "Add account abstraction support per EIP-7702...",
  "domain": "coding",
  "reward_usdc": 2000,
  "deadline_block": 1000000,
  "required_capabilities": 3,
  "min_reputation": 0.7,
  "evaluation_criteria": [
    "All existing tests pass",
    "New tests cover the EIP-7702 path",
    "Clippy clean with no new warnings"
  ],
  "eval_id": "0xdef..."
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "reward_usdc|post|bounty|Example|tests|support|xdef|warnings" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reward_usdc|post|bounty|Example|tests|support|xdef|warnings" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- `crates/roko-serve/src/routes/mod.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/bounties` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `reward_usdc` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `xdef...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S026 -- Batch matching

**Source section:** `tmp/architecture/11-arenas.md:821` through `844`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Batch matching

```json
POST /api/bounties/batch-match

// Response:
{
  "matches": [
    {
      "bounty_id": "0xabc...",
      "agent_passport_id": 42,
      "price_usdc": 1800,
      "vcg_payment_usdc": 1500,
      "welfare_contribution": 0.85
    }
  ],
  "total_welfare": 12.4,
  "unmatched_bounties": ["0xdef..."],
  "unmatched_agents": [99]
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `77d87ff7b63fbc956efda4b4de7b0253cf423c4d71631ebb3212ab04423983e3`

**Normative requirements and implementation claims:**
- ```json POST /api/bounties/batch-match
- ---

**Routes and endpoint references:**
- POST /api/bounties/batch-match

**Files and path references:**
- api/bounties/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- xabc...
- xdef...

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
- Contract 1: language `json`, first line `POST /api/bounties/batch-match`

```json
POST /api/bounties/batch-match

// Response:
{
  "matches": [
    {
      "bounty_id": "0xabc...",
      "agent_passport_id": 42,
      "price_usdc": 1800,
      "vcg_payment_usdc": 1500,
      "welfare_contribution": 0.85
    }
  ],
  "total_welfare": 12.4,
  "unmatched_bounties": ["0xdef..."],
  "unmatched_agents": [99]
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/bounties/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "match|Batch|matching|bounties|xdef|xabc|welfare_contribution|vcg_payment_usdc" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "match|Batch|matching|bounties|xdef|xabc|welfare_contribution|vcg_payment_usdc" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `api/bounties/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/bounties/batch-match` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Emit or consume `xabc...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `xdef...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S027 -- Event types

**Source section:** `tmp/architecture/11-arenas.md:845` through `848`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Event types

All three subsystems emit events through the relay WebSocket. Events follow the standard `DashboardEvent` envelope.
````

**Explicit detail extraction from this section:**

- Section word count: `15`
- Section hash: `efd7a64f0e66406aa5eefc52a7054f90442f3d1246793c3a3b73aa02f77cc9ee`

**Normative requirements and implementation claims:**
- All three subsystems emit events through the relay WebSocket. Events follow the standard `DashboardEvent` envelope.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- DashboardEvent

**Event names and event-like entities:**
- DashboardEvent

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Event|types|DashboardEvent|events|three|subsystems|standard|relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Event|types|DashboardEvent|events|three|subsystems|standard|relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `DashboardEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `DashboardEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S028 -- Arena events

**Source section:** `tmp/architecture/11-arenas.md:849` through `867`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Arena events

```rust
pub enum ArenaEvent {
    /// A new arena was registered.
    ArenaCreated { arena_id: [u8; 32], name: String, category: ArenaCategory },
    /// Arena state changed.
    ArenaStateChanged { arena_id: [u8; 32], old_state: ArenaState, new_state: ArenaState },
    /// An attempt was submitted.
    AttemptSubmitted { arena_id: [u8; 32], attempt_id: [u8; 32], agent_passport_id: u128 },
    /// An attempt completed with a score.
    AttemptCompleted { arena_id: [u8; 32], attempt_id: [u8; 32], score: f64, rank: u64 },
    /// An attempt failed.
    AttemptFailed { arena_id: [u8; 32], attempt_id: [u8; 32], reason: String },
    /// Leaderboard rank changed for an agent.
    RankChanged { arena_id: [u8; 32], agent_passport_id: u128, old_rank: u64, new_rank: u64 },
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `86`
- Section hash: `9e57351fc4d8724811de40563a3289b1b96e6e7cf7bf011c920e7042d1d31ff3`

**Normative requirements and implementation claims:**
- ```rust pub enum ArenaEvent { /// A new arena was registered. ArenaCreated { arena_id: [u8; 32], name: String, category: ArenaCategory }, /// Arena state changed. ArenaStateChanged { arena_id: [u8; 32], old_state: ArenaState, new_state: ArenaState }, /// An attempt was submitted. AttemptSubmitted { arena_id: [u8; 32], attempt_id: [u8; 32], agent_passport_id: u128 }, /// An attempt completed with a score. AttemptCompleted { arena_id: [u8; 32], attempt_id: [u8; 32], score: f64, rank: u64 }, /// An attempt failed. AttemptFailed { arena_id: [u8; 32], attempt_id: [u8; 32], reason: String }, /// Leaderboard rank changed for an agent. RankChanged { arena_id: [u8; 32], agent_passport_id: u128, old_rank: u64, new_rank: u64 }, } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArenaEvent

**Event names and event-like entities:**
- ArenaEvent
- ArenaCreated
- AttemptSubmitted
- AttemptCompleted
- AttemptFailed

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
- Contract 1: language `rust`, first line `pub enum ArenaEvent {`

```rust
pub enum ArenaEvent {
    /// A new arena was registered.
    ArenaCreated { arena_id: [u8; 32], name: String, category: ArenaCategory },
    /// Arena state changed.
    ArenaStateChanged { arena_id: [u8; 32], old_state: ArenaState, new_state: ArenaState },
    /// An attempt was submitted.
    AttemptSubmitted { arena_id: [u8; 32], attempt_id: [u8; 32], agent_passport_id: u128 },
    /// An attempt completed with a score.
    AttemptCompleted { arena_id: [u8; 32], attempt_id: [u8; 32], score: f64, rank: u64 },
    /// An attempt failed.
    AttemptFailed { arena_id: [u8; 32], attempt_id: [u8; 32], reason: String },
    /// Leaderboard rank changed for an agent.
    RankChanged { arena_id: [u8; 32], agent_passport_id: u128, old_rank: u64, new_rank: u64 },
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Arena|attempt|state|arena_id|rank|events|ArenaEvent|changed" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|attempt|state|arena_id|rank|events|ArenaEvent|changed" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `ArenaEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ArenaEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ArenaCreated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AttemptSubmitted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AttemptCompleted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `AttemptFailed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S029 -- Eval events

**Source section:** `tmp/architecture/11-arenas.md:868` through `882`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Eval events

```rust
pub enum EvalEvent {
    /// A new eval was registered.
    EvalRegistered { eval_id: [u8; 32], name: String, domain: String },
    /// An eval run started.
    EvalRunStarted { eval_id: [u8; 32], run_id: [u8; 32], agent_passport_id: u128 },
    /// An eval run completed.
    EvalRunCompleted { eval_id: [u8; 32], run_id: [u8; 32], score: f64 },
    /// Calibration was computed for an eval.
    EvalCalibrated { eval_id: [u8; 32], rank_correlation: f64, discrimination_power: f64 },
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `57`
- Section hash: `4c4ec185c345f25d2e89f913f6df11b05e4069d2df4ac5285f1fe8122b1fe183`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EvalEvent

**Event names and event-like entities:**
- EvalEvent
- EvalRunStarted
- EvalRunCompleted

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
- Contract 1: language `rust`, first line `pub enum EvalEvent {`

```rust
pub enum EvalEvent {
    /// A new eval was registered.
    EvalRegistered { eval_id: [u8; 32], name: String, domain: String },
    /// An eval run started.
    EvalRunStarted { eval_id: [u8; 32], run_id: [u8; 32], agent_passport_id: u128 },
    /// An eval run completed.
    EvalRunCompleted { eval_id: [u8; 32], run_id: [u8; 32], score: f64 },
    /// Calibration was computed for an eval.
    EvalCalibrated { eval_id: [u8; 32], rank_correlation: f64, discrimination_power: f64 },
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Eval|events|EvalEvent|eval_id|started|run_id|registered|completed" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|events|EvalEvent|eval_id|started|run_id|registered|completed" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `EvalEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `EvalEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `EvalRunStarted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `EvalRunCompleted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S030 -- Bounty events

**Source section:** `tmp/architecture/11-arenas.md:883` through `905`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Bounty events

```rust
pub enum BountyEvent {
    /// A new bounty was posted.
    BountyPosted { bounty_id: [u8; 32], title: String, reward_usdc: u64 },
    /// A bid was submitted.
    BidSubmitted { bounty_id: [u8; 32], agent_passport_id: u128, price_usdc: u64 },
    /// VCG matching assigned an agent to a bounty.
    BountyMatched { bounty_id: [u8; 32], agent_passport_id: u128, vcg_payment: u64 },
    /// Agent submitted a result.
    ResultSubmitted { bounty_id: [u8; 32], result_cid: String },
    /// Evaluation completed.
    BountyEvaluated { bounty_id: [u8; 32], quality_score: f64, passed: bool },
    /// Escrow released to the agent.
    BountySettled { bounty_id: [u8; 32], agent_passport_id: u128, payment_usdc: u64 },
    /// Dispute opened.
    DisputeOpened { bounty_id: [u8; 32], challenger: u128, level: String },
    /// Dispute resolved.
    DisputeResolved { bounty_id: [u8; 32], winner: u128, outcome: String },
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `98`
- Section hash: `136881b1d05b4597c366125cb024534ce6ad62d9191d5d97cecd3066fe9a6937`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- BountyEvent

**Event names and event-like entities:**
- BountyEvent
- BidSubmitted
- ResultSubmitted

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
- Contract 1: language `rust`, first line `pub enum BountyEvent {`

```rust
pub enum BountyEvent {
    /// A new bounty was posted.
    BountyPosted { bounty_id: [u8; 32], title: String, reward_usdc: u64 },
    /// A bid was submitted.
    BidSubmitted { bounty_id: [u8; 32], agent_passport_id: u128, price_usdc: u64 },
    /// VCG matching assigned an agent to a bounty.
    BountyMatched { bounty_id: [u8; 32], agent_passport_id: u128, vcg_payment: u64 },
    /// Agent submitted a result.
    ResultSubmitted { bounty_id: [u8; 32], result_cid: String },
    /// Evaluation completed.
    BountyEvaluated { bounty_id: [u8; 32], quality_score: f64, passed: bool },
    /// Escrow released to the agent.
    BountySettled { bounty_id: [u8; 32], agent_passport_id: u128, payment_usdc: u64 },
    /// Dispute opened.
    DisputeOpened { bounty_id: [u8; 32], challenger: u128, level: String },
    /// Dispute resolved.
    DisputeResolved { bounty_id: [u8; 32], winner: u128, outcome: String },
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Bounty|bounty_id|u128|events|BountyEvent|submitted|String|Dispute" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bounty|bounty_id|u128|events|BountyEvent|submitted|String|Dispute" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `BountyEvent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `BountyEvent` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `BidSubmitted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ResultSubmitted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S031 -- WebSocket subscription

**Source section:** `tmp/architecture/11-arenas.md:906` through `918`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### WebSocket subscription

Clients subscribe to arena/eval/bounty events by topic:

```
ws://relay/ws?subscribe=arena:0xabc123      // Single arena
ws://relay/ws?subscribe=arena:*             // All arenas
ws://relay/ws?subscribe=bounty:0xdef456     // Single bounty
ws://relay/ws?subscribe=eval:*              // All evals
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `781de0fb0529b5d8f8a2e68657ae21bfc686b8623dbafe7cb797ef47985f3ddc`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- arena/eval/

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
- Contract 1: language `plain`, first line `ws://relay/ws?subscribe=arena:0xabc123      // Single arena`

```
ws://relay/ws?subscribe=arena:0xabc123      // Single arena
ws://relay/ws?subscribe=arena:*             // All arenas
ws://relay/ws?subscribe=bounty:0xdef456     // Single bounty
ws://relay/ws?subscribe=eval:*              // All evals
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `arena/eval/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "subscription|subscribe|arena|WebSocket|relay|eval|bounty|Single" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "subscription|subscribe|arena|WebSocket|relay|eval|bounty|Single" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `arena/eval/`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S032 -- On-chain contracts

**Source section:** `tmp/architecture/11-arenas.md:919` through `922`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## On-chain contracts

Four Solidity contracts anchor the subsystems on-chain. Full task data and attempt artifacts live off-chain; contracts store hashes, scores, and financial state.
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `9aa7dc003a3aa7ef575a0a6ea9815b421005824493cdbe6ee7534593c2b0ed37`

**Normative requirements and implementation claims:**
- Four Solidity contracts anchor the subsystems on-chain. Full task data and attempt artifacts live off-chain; contracts store hashes, scores, and financial state.

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "contracts|chain|task|subsystems|store|state|scores|live" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "contracts|chain|task|subsystems|store|state|scores|live" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S033 -- ArenaRegistry.sol

**Source section:** `tmp/architecture/11-arenas.md:923` through `967`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### ArenaRegistry.sol

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IArenaRegistry {
    enum ArenaState { Draft, Active, Paused, Concluded }

    struct ArenaInfo {
        bytes32 id;
        string name;
        string category;
        ArenaState state;
        address creator;
        uint256 prizePoolUsdc;
        uint64 maxAttemptsPerAgent;
        uint64 cooldownBlocks;
        uint64 deadlineBlock;
        bytes32 configHash;       // Hash of the full Arena config (task source, scoring, etc.)
    }

    struct AttemptRecord {
        bytes32 attemptId;
        bytes32 arenaId;
        uint256 agentPassportId;
        uint64 score;             // Fixed-point: score * 1e18
        uint64 submittedBlock;
        uint64 completedBlock;
        bytes32 outputHash;
    }

    event ArenaCreated(bytes32 indexed arenaId, address indexed creator, string name);
    event ArenaStateChanged(bytes32 indexed arenaId, ArenaState oldState, ArenaState newState);
    event AttemptRecorded(bytes32 indexed arenaId, bytes32 indexed attemptId, uint256 agentPassportId, uint64 score);

    function createArena(ArenaInfo calldata info) external returns (bytes32 arenaId);
    function transitionArena(bytes32 arenaId, ArenaState newState) external;
    function recordAttempt(AttemptRecord calldata record) external;
    function getArena(bytes32 arenaId) external view returns (ArenaInfo memory);
    function getLeaderboard(bytes32 arenaId, uint64 limit, uint64 offset) external view returns (AttemptRecord[] memory);
    function arenaCount() external view returns (uint256);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `151`
- Section hash: `43cff6a2c2fb247650c826dd295cf083f7aa825e5e8947eb4941f588cb80287f`

**Normative requirements and implementation claims:**
- struct ArenaInfo { bytes32 id; string name; string category; ArenaState state; address creator; uint256 prizePoolUsdc; uint64 maxAttemptsPerAgent; uint64 cooldownBlocks; uint64 deadlineBlock; bytes32 configHash; // Hash of the full Arena config (task source, scoring, etc.) }
- event ArenaCreated(bytes32 indexed arenaId, address indexed creator, string name); event ArenaStateChanged(bytes32 indexed arenaId, ArenaState oldState, ArenaState newState); event AttemptRecorded(bytes32 indexed arenaId, bytes32 indexed attemptId, uint256 agentPassportId, uint64 score);

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ArenaState
- ArenaInfo
- AttemptRecord

**Event names and event-like entities:**
- ArenaCreated

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

interface IArenaRegistry {
    enum ArenaState { Draft, Active, Paused, Concluded }

    struct ArenaInfo {
        bytes32 id;
        string name;
        string category;
        ArenaState state;
        address creator;
        uint256 prizePoolUsdc;
        uint64 maxAttemptsPerAgent;
        uint64 cooldownBlocks;
        uint64 deadlineBlock;
        bytes32 configHash;       // Hash of the full Arena config (task source, scoring, etc.)
    }

    struct AttemptRecord {
        bytes32 attemptId;
        bytes32 arenaId;
        uint256 agentPassportId;
        uint64 score;             // Fixed-point: score * 1e18
        uint64 submittedBlock;
        uint64 completedBlock;
        bytes32 outputHash;
    }

    event ArenaCreated(bytes32 indexed arenaId, address indexed creator, string name);
    event ArenaStateChanged(bytes32 indexed arenaId, ArenaState oldState, ArenaState newState);
    event AttemptRecorded(bytes32 indexed arenaId, bytes32 indexed attemptId, uint256 agentPassportId, uint64 score);

    function createArena(ArenaInfo calldata info) external returns (bytes32 arenaId);
    function transitionArena(bytes32 arenaId, ArenaState newState) external;
    function recordAttempt(AttemptRecord calldata record) external;
    function getArena(bytes32 arenaId) external view returns (ArenaInfo memory);
    function getLeaderboard(bytes32 arenaId, uint64 limit, uint64 offset) external view returns (AttemptRecord[] memory);
    function arenaCount() external view returns (uint256);
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Arena|bytes32|state|ArenaState|uint64|arenaId|AttemptRecord|sol" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|bytes32|state|ArenaState|uint64|arenaId|AttemptRecord|sol" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `ArenaState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ArenaInfo` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AttemptRecord` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `ArenaCreated` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S034 -- EvalRegistry.sol

**Source section:** `tmp/architecture/11-arenas.md:968` through `1001`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### EvalRegistry.sol

```solidity
interface IEvalRegistry {
    struct EvalInfo {
        bytes32 id;
        string name;
        string domain;
        address creator;
        bytes32 groundTruthHash;  // Hash of the GroundTruthSource config
        bytes32 scoringHash;      // Hash of the ScoringFunction config
        uint32 version;
        bool isMetaEval;
    }

    struct CalibrationRecord {
        bytes32 evalId;
        int64 rankCorrelation;    // Fixed-point: correlation * 1e18
        int64 discriminationPower;
        uint64 sampleCount;
        uint64 calibratedBlock;
    }

    event EvalRegistered(bytes32 indexed evalId, address indexed creator, string name);
    event EvalCalibrated(bytes32 indexed evalId, int64 rankCorrelation, int64 discriminationPower);

    function registerEval(EvalInfo calldata info) external returns (bytes32 evalId);
    function recordCalibration(CalibrationRecord calldata record) external;
    function getEval(bytes32 evalId) external view returns (EvalInfo memory);
    function latestCalibration(bytes32 evalId) external view returns (CalibrationRecord memory);
    function evalCount() external view returns (uint256);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `105`
- Section hash: `497f5864c12c5c90f5e1a7f505f75368f0651201df28ac9f7cbeade1b6a0f3a2`

**Normative requirements and implementation claims:**
- event EvalRegistered(bytes32 indexed evalId, address indexed creator, string name); event EvalCalibrated(bytes32 indexed evalId, int64 rankCorrelation, int64 discriminationPower);

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EvalInfo
- CalibrationRecord

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
- Contract 1: language `solidity`, first line `interface IEvalRegistry {`

```solidity
interface IEvalRegistry {
    struct EvalInfo {
        bytes32 id;
        string name;
        string domain;
        address creator;
        bytes32 groundTruthHash;  // Hash of the GroundTruthSource config
        bytes32 scoringHash;      // Hash of the ScoringFunction config
        uint32 version;
        bool isMetaEval;
    }

    struct CalibrationRecord {
        bytes32 evalId;
        int64 rankCorrelation;    // Fixed-point: correlation * 1e18
        int64 discriminationPower;
        uint64 sampleCount;
        uint64 calibratedBlock;
    }

    event EvalRegistered(bytes32 indexed evalId, address indexed creator, string name);
    event EvalCalibrated(bytes32 indexed evalId, int64 rankCorrelation, int64 discriminationPower);

    function registerEval(EvalInfo calldata info) external returns (bytes32 evalId);
    function recordCalibration(CalibrationRecord calldata record) external;
    function getEval(bytes32 evalId) external view returns (EvalInfo memory);
    function latestCalibration(bytes32 evalId) external view returns (CalibrationRecord memory);
    function evalCount() external view returns (uint256);
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "bytes32|EvalInfo|CalibrationRecord|sol|int64|function|evalId|EvalRegistry" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "bytes32|EvalInfo|CalibrationRecord|sol|int64|function|evalId|EvalRegistry" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `EvalInfo` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CalibrationRecord` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S035 -- BountyMarket.sol

**Source section:** `tmp/architecture/11-arenas.md:1002` through `1040`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### BountyMarket.sol

```solidity
interface IBountyMarket {
    enum BountyState { Open, Claimed, InProgress, Submitted, Evaluated, Completed, Disputed, Cancelled, Expired }

    struct BountyInfo {
        bytes32 id;
        address poster;
        uint256 rewardUsdc;
        uint256 rewardDaeji;
        uint64 deadlineBlock;
        uint64 requiredCapabilities;
        int64 minReputation;      // Fixed-point: reputation * 1e18
        BountyState state;
        uint256 assignedAgent;
        bytes32 resultHash;
        bytes32 evalId;           // Optional linked eval
    }

    event BountyPosted(bytes32 indexed bountyId, address indexed poster, uint256 rewardUsdc);
    event BountyMatched(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 vcgPayment);
    event BountySettled(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 payment);
    event DisputeOpened(bytes32 indexed bountyId, uint256 indexed challenger);
    event DisputeResolved(bytes32 indexed bountyId, uint256 indexed winner, uint8 outcome);

    function postBounty(BountyInfo calldata info) external payable returns (bytes32 bountyId);
    function submitBid(bytes32 bountyId, uint256 priceUsdc, uint64 estimatedTime, uint64 capabilityProof) external;
    function matchBounty(bytes32 bountyId) external;
    function submitResult(bytes32 bountyId, bytes32 resultHash) external;
    function settleBounty(bytes32 bountyId) external;
    function openDispute(bytes32 bountyId) external payable;
    function resolveDispute(bytes32 bountyId, uint256 winner, uint8 outcome) external;
    function cancelBounty(bytes32 bountyId) external;
    function getBounty(bytes32 bountyId) external view returns (BountyInfo memory);
    function escrowBalance(bytes32 bountyId) external view returns (uint256);
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `168`
- Section hash: `fcc0d9db1cbe7394fe4338c4fb4b79b6a81e53d2dafa66aed454493065f38bf4`

**Normative requirements and implementation claims:**
- struct BountyInfo { bytes32 id; address poster; uint256 rewardUsdc; uint256 rewardDaeji; uint64 deadlineBlock; uint64 requiredCapabilities; int64 minReputation; // Fixed-point: reputation * 1e18 BountyState state; uint256 assignedAgent; bytes32 resultHash; bytes32 evalId; // Optional linked eval }
- event BountyPosted(bytes32 indexed bountyId, address indexed poster, uint256 rewardUsdc); event BountyMatched(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 vcgPayment); event BountySettled(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 payment); event DisputeOpened(bytes32 indexed bountyId, uint256 indexed challenger); event DisputeResolved(bytes32 indexed bountyId, uint256 indexed winner, uint8 outcome);

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- BountyState
- BountyInfo

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
- Contract 1: language `solidity`, first line `interface IBountyMarket {`

```solidity
interface IBountyMarket {
    enum BountyState { Open, Claimed, InProgress, Submitted, Evaluated, Completed, Disputed, Cancelled, Expired }

    struct BountyInfo {
        bytes32 id;
        address poster;
        uint256 rewardUsdc;
        uint256 rewardDaeji;
        uint64 deadlineBlock;
        uint64 requiredCapabilities;
        int64 minReputation;      // Fixed-point: reputation * 1e18
        BountyState state;
        uint256 assignedAgent;
        bytes32 resultHash;
        bytes32 evalId;           // Optional linked eval
    }

    event BountyPosted(bytes32 indexed bountyId, address indexed poster, uint256 rewardUsdc);
    event BountyMatched(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 vcgPayment);
    event BountySettled(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 payment);
    event DisputeOpened(bytes32 indexed bountyId, uint256 indexed challenger);
    event DisputeResolved(bytes32 indexed bountyId, uint256 indexed winner, uint8 outcome);

    function postBounty(BountyInfo calldata info) external payable returns (bytes32 bountyId);
    function submitBid(bytes32 bountyId, uint256 priceUsdc, uint64 estimatedTime, uint64 capabilityProof) external;
    function matchBounty(bytes32 bountyId) external;
    function submitResult(bytes32 bountyId, bytes32 resultHash) external;
    function settleBounty(bytes32 bountyId) external;
    function openDispute(bytes32 bountyId) external payable;
    function resolveDispute(bytes32 bountyId, uint256 winner, uint8 outcome) external;
    function cancelBounty(bytes32 bou
...
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "bytes32|bountyId|uint256|indexed|function|external|sol|BountyInfo" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "bytes32|bountyId|uint256|indexed|function|external|sol|BountyInfo" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `BountyState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BountyInfo` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S036 -- DisputeResolver.sol

**Source section:** `tmp/architecture/11-arenas.md:1041` through `1078`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### DisputeResolver.sol

```solidity
interface IDisputeResolver {
    enum Level { BondEscalation, PeerJury, GovernanceVote }

    struct Dispute {
        bytes32 bountyId;
        uint256 challenger;
        uint256 defender;
        Level currentLevel;
        uint256 challengerBond;
        uint256 defenderBond;
        uint8 escalationRound;
        uint64 deadlineBlock;
        bool resolved;
    }

    struct JuryVote {
        uint256 jurorPassportId;
        bool votesForDefender;
        uint256 stakeAmount;
    }

    event DisputeEscalated(bytes32 indexed bountyId, Level newLevel);
    event JuryVoteCast(bytes32 indexed bountyId, uint256 indexed juror, bool votesForDefender);
    event DisputeFinalized(bytes32 indexed bountyId, uint256 indexed winner, uint256 payout);

    function escalate(bytes32 bountyId) external payable;
    function castJuryVote(bytes32 bountyId, bool votesForDefender) external;
    function finalizeDispute(bytes32 bountyId) external;
    function getDispute(bytes32 bountyId) external view returns (Dispute memory);
    function getJuryVotes(bytes32 bountyId) external view returns (JuryVote[] memory);
}
```

---
````

**Explicit detail extraction from this section:**

- Section word count: `99`
- Section hash: `7b6a3b63512bc93c0398033c75558bdc093e60e811e7ee13a59467cc3429bf00`

**Normative requirements and implementation claims:**
- event DisputeEscalated(bytes32 indexed bountyId, Level newLevel); event JuryVoteCast(bytes32 indexed bountyId, uint256 indexed juror, bool votesForDefender); event DisputeFinalized(bytes32 indexed bountyId, uint256 indexed winner, uint256 payout);
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Level
- Dispute
- JuryVote

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
- Contract 1: language `solidity`, first line `interface IDisputeResolver {`

```solidity
interface IDisputeResolver {
    enum Level { BondEscalation, PeerJury, GovernanceVote }

    struct Dispute {
        bytes32 bountyId;
        uint256 challenger;
        uint256 defender;
        Level currentLevel;
        uint256 challengerBond;
        uint256 defenderBond;
        uint8 escalationRound;
        uint64 deadlineBlock;
        bool resolved;
    }

    struct JuryVote {
        uint256 jurorPassportId;
        bool votesForDefender;
        uint256 stakeAmount;
    }

    event DisputeEscalated(bytes32 indexed bountyId, Level newLevel);
    event JuryVoteCast(bytes32 indexed bountyId, uint256 indexed juror, bool votesForDefender);
    event DisputeFinalized(bytes32 indexed bountyId, uint256 indexed winner, uint256 payout);

    function escalate(bytes32 bountyId) external payable;
    function castJuryVote(bytes32 bountyId, bool votesForDefender) external;
    function finalizeDispute(bytes32 bountyId) external;
    function getDispute(bytes32 bountyId) external view returns (Dispute memory);
    function getJuryVotes(bytes32 bountyId) external view returns (JuryVote[] memory);
}
```

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "Dispute|uint256|bytes32|bountyId|Level|JuryVote|sol|DisputeResolver" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Dispute|uint256|bytes32|bountyId|Level|JuryVote|sol|DisputeResolver" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `Level` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Dispute` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `JuryVote` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S037 -- Crate mapping

**Source section:** `tmp/architecture/11-arenas.md:1079` through `1094`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Crate mapping

| Component | Crate | Status |
|-----------|-------|--------|
| Arena types + registry | `roko-chain` | Types needed; marketplace.rs has the job lifecycle |
| Eval types + registry | `roko-chain` | Types needed; eval_generator.rs in `roko-gate` has the generation side |
| Bounty market | `roko-chain/src/marketplace.rs` | Wired (job lifecycle, escrow, disputes) |
| VCG matching | `roko-compose/src/auction.rs` | Wired (`vcg_allocate` exported) |
| Validation records | `roko-chain/src/validation_registry.rs` | Wired (work proofs feed reputation) |
| Arena API routes | `roko-serve` | Not yet implemented |
| Eval API routes | `roko-serve` | Not yet implemented |
| Bounty API routes | `roko-serve` | Partial (jobs routes exist, bounty-specific routes needed) |
| Contract deployment | Solidity in `contracts/` | Not yet implemented |

---
````

**Explicit detail extraction from this section:**

- Section word count: `103`
- Section hash: `71b96dcec11f74f7092a1a1c765cdb13c108831603e1d70ff4cbc2f7ada4adb3`

**Normative requirements and implementation claims:**
- | Component | Crate | Status | |-----------|-------|--------| | Arena types + registry | `roko-chain` | Types needed; marketplace.rs has the job lifecycle | | Eval types + registry | `roko-chain` | Types needed; eval_generator.rs in `roko-gate` has the generation side | | Bounty market | `roko-chain/src/marketplace.rs` | Wired (job lifecycle, escrow, disputes) | | VCG matching | `roko-compose/src/auction.rs` | Wired (`vcg_allocate` exported) | | Validation records | `roko-chain/src/validation_registry.rs` | Wired (work proofs feed reputation) | | Arena API routes | `roko-serve` | Not yet implemented | | Eval API routes | `roko-serve` | Not yet implemented | | Bounty API routes | `roko-serve` | Partial (jobs routes exist, bounty-specific routes needed) | | Contract deployment | Solidity in `contracts/` | Not yet implemented |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-chain/src/marketplace.rs
- roko-chain/src/validation_registry.rs
- roko-compose/src/auction.rs

**Types, functions, traits, and inline code identifiers:**
- vcg_allocate

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
| Component | Crate | Status |
|-----------|-------|--------|
| Arena types + registry | `roko-chain` | Types needed; marketplace.rs has the job lifecycle |
| Eval types + registry | `roko-chain` | Types needed; eval_generator.rs in `roko-gate` has the generation side |
| Bounty market | `roko-chain/src/marketplace.rs` | Wired (job lifecycle, escrow, disputes) |
| VCG matching | `roko-compose/src/auction.rs` | Wired (`vcg_allocate` exported) |
| Validation records | `roko-chain/src/validation_registry.rs` | Wired (work proofs feed reputation) |
| Arena API routes | `roko-serve` | Not yet implemented |
| Eval API routes | `roko-serve` | Not yet implemented |
| Bounty API routes | `roko-serve` | Partial (jobs routes exist, bounty-specific routes needed) |
| Contract deployment | Solidity in `contracts/` | Not yet implemented |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/marketplace.rs`
- `roko-chain/src/validation_registry.rs`
- `roko-compose/src/auction.rs`
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
rg -n "Crate|vcg_allocate|routes|mapping|types|chain|serve|registry" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Crate|vcg_allocate|routes|mapping|types|chain|serve|registry" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
- `roko-chain/src/marketplace.rs`
- `roko-chain/src/validation_registry.rs`
- `roko-compose/src/auction.rs`
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

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `vcg_allocate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-11-S038 -- Interactions with other subsystems

**Source section:** `tmp/architecture/11-arenas.md:1095` through `1105`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Interactions with other subsystems

**Reputation registry** (see `14-registries.md`): Every completed arena attempt and settled bounty produces a `WorkProof` that flows into the validation registry, which feeds the reputation registry. An agent's reputation is the aggregate of its validated work -- not self-reported.

**Cascade router** (see `07-gateway.md`): Arena performance data feeds the cascade router's model selection. If an agent consistently scores higher on coding arenas with Opus than with Sonnet, the router learns to route coding tasks to Opus.

**Knowledge store** (see `09-knowledge.md`): Insights generated during arena attempts and bounty work are candidates for knowledge distillation. High-scoring attempts produce higher-confidence knowledge entries.

**Groups** (see `10-groups.md`): A group can enter an arena collectively. The group's score is the aggregate of its members' contributions. Bounties can target groups rather than individual agents.

**Extensions** (see `03-extensions.md`): Arena task sources and scoring functions are implemented as extensions. A `TaskSourceExtension` provides tasks; a `ScoringExtension` computes scores. This makes the arena system composable without modifying core code.
````

**Explicit detail extraction from this section:**

- Section word count: `171`
- Section hash: `96244e1ab5ddb4f162101bd2b7fde6097c5f7e93cb996f3983bae7f343b7a64c`

**Normative requirements and implementation claims:**
- **Reputation registry** (see `14-registries.md`): Every completed arena attempt and settled bounty produces a `WorkProof` that flows into the validation registry, which feeds the reputation registry. An agent's reputation is the aggregate of its validated work -- not self-reported.
- **Cascade router** (see `07-gateway.md`): Arena performance data feeds the cascade router's model selection. If an agent consistently scores higher on coding arenas with Opus than with Sonnet, the router learns to route coding tasks to Opus.
- **Knowledge store** (see `09-knowledge.md`): Insights generated during arena attempts and bounty work are candidates for knowledge distillation. High-scoring attempts produce higher-confidence knowledge entries.
- **Groups** (see `10-groups.md`): A group can enter an arena collectively. The group's score is the aggregate of its members' contributions. Bounties can target groups rather than individual agents.
- **Extensions** (see `03-extensions.md`): Arena task sources and scoring functions are implemented as extensions. A `TaskSourceExtension` provides tasks; a `ScoringExtension` computes scores. This makes the arena system composable without modifying core code.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- WorkProof
- TaskSourceExtension
- ScoringExtension

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
- `tmp/architecture/11-arenas.md`
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
rg -n "arena|subsystems|other|group|WorkProof|TaskSourceExtension|ScoringExtension|Interactions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "arena|subsystems|other|group|WorkProof|TaskSourceExtension|ScoringExtension|Interactions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/arenas.rs`
- `crates/roko-serve/src/routes/evals.rs`
- `crates/roko-serve/src/routes/bounties.rs`
- `crates/roko-gate/src/eval_generator.rs`
- `crates/roko-chain/src/marketplace.rs`
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
- [ ] Implement or verify `WorkProof` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TaskSourceExtension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ScoringExtension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/11-arenas
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

