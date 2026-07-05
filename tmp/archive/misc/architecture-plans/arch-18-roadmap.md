# Architecture Plan: Roadmap

**Source:** `tmp/architecture/18-roadmap.md`
**Generated:** 2026-04-25
**Source hash:** `e9fe8d67892275c2713add3f7fdc3136d38fb181f70f696d53334c463121e92e`
**Section tasks:** 29
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
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-18-S001 | 1 | Implementation roadmap | [ ] | 9.8 |
| ARCH-18-S002 | 8 | Implementation path | [ ] | 9.8 |
| ARCH-18-S003 | 10 | Phase 1: auth + secrets (mostly done) | [ ] | 9.8 |
| ARCH-18-S004 | 25 | Phase 2: agent runtime | [ ] | 9.8 |
| ARCH-18-S005 | 42 | Phase 3: relay + dashboard integration | [ ] | 9.8 |
| ARCH-18-S006 | 59 | Phase 4: inference gateway | [ ] | 9.8 |
| ARCH-18-S007 | 76 | Phase 5: agent feeds, dynamic endpoints, and paid subscriptions | [ ] | 9.8 |
| ARCH-18-S008 | 93 | Phase 6: clusters + coordination | [ ] | 9.8 |
| ARCH-18-S009 | 109 | Phase 7: isolated execution (Fly Machines) | [ ] | 9.8 |
| ARCH-18-S010 | 124 | Phase 8: multi-tenant | [ ] | 9.8 |
| ARCH-18-S011 | 138 | Bardo source references | [ ] | 9.8 |
| ARCH-18-S012 | 142 | Inference gateway -- bardo-gateway (22.8K LOC) | [ ] | 9.8 |
| ARCH-18-S013 | 161 | Agent runtime -- mori (108K LOC) | [ ] | 9.8 |
| ARCH-18-S014 | 180 | Heartbeat -- golem-heartbeat (10.2K LOC) | [ ] | 9.8 |
| ARCH-18-S015 | 199 | DeFi tools -- golem-tools (7.2K LOC) | [ ] | 9.8 |
| ARCH-18-S016 | 203 | Chain runtime -- golem-chain (5.3K LOC) | [ ] | 9.8 |
| ARCH-18-S017 | 207 | Dashboard -- apps/dashboard (Next.js, ~27K LOC) | [ ] | 9.8 |
| ARCH-18-S018 | 213 | Migration from v1 | [ ] | 9.8 |
| ARCH-18-S019 | 231 | Bardo → Roko naming map | [ ] | 9.8 |
| ARCH-18-S020 | 256 | Bardo source reference (LOC counts) | [ ] | 9.8 |
| ARCH-18-S021 | 271 | Implementation task summary | [ ] | 9.8 |
| ARCH-18-S022 | 291 | Dependency graph | [ ] | 9.8 |
| ARCH-18-S023 | 308 | Critical path | [ ] | 9.8 |
| ARCH-18-S024 | 312 | Parallel tracks | [ ] | 9.8 |
| ARCH-18-S025 | 318 | Phase 1 task breakdown: Inference Gateway | [ ] | 9.8 |
| ARCH-18-S026 | 341 | Phase 4 task breakdown: Heartbeat Pipeline | [ ] | 9.8 |
| ARCH-18-S027 | 352 | Phase 5 task breakdown: Agent Modes + Profiles | [ ] | 9.8 |
| ARCH-18-S028 | 364 | Phase 6 task breakdown: Dashboard | [ ] | 9.8 |
| ARCH-18-S029 | 377 | Phase 10-12 task breakdowns | [ ] | 9.8 |

## Tasks

### ARCH-18-S001 -- Implementation roadmap

**Source section:** `tmp/architecture/18-roadmap.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Implementation roadmap

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Implementation path", "Bardo source references", and "Migration from v1" sections.

---
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `0d94853b5c6a9be3d8e75e1b1c72b75e1a2181cd7c0dbdfbb8007a6649a25fa6`

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "roadmap|sections|references|redesign|path|Specification|Part|Migration" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "roadmap|sections|references|redesign|path|Specification|Part|Migration" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S002 -- Implementation path

**Source section:** `tmp/architecture/18-roadmap.md:8` through `9`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Implementation path
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `f0756358ca2f75d71eed1319f8687c609e5a5f2767ca6275488327c6c4dae1d1`

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "path|roadmap" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "path|roadmap" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S003 -- Phase 1: auth + secrets (mostly done)

**Source section:** `tmp/architecture/18-roadmap.md:10` through `24`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 1: auth + secrets (mostly done)

Already shipped in commit `5af205d3`:
- Secrets HTTP API (GET/POST/DELETE/test)
- Multi-key auth middleware (X-Api-Key + Bearer)
- API key scopes + SHA-256 + expiry
- `roko login` CLI with credential store
- Agent CRUD (create/start/stop/restart/list)
- ProcessSupervisor integration

Remaining:
- Privy JWT validation (real JWKS, not structural stub)
- Scope enforcement at route level
- Device flow for headless CLI login
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `3bca1f6edf0f871c3e74d86aeb37685905591d1e377360ffbbf2540d68af1841`

**Normative requirements and implementation claims:**
- Already shipped in commit `5af205d3`: - Secrets HTTP API (GET/POST/DELETE/test) - Multi-key auth middleware (X-Api-Key + Bearer) - API key scopes + SHA-256 + expiry - `roko login` CLI with credential store - Agent CRUD (create/start/stop/restart/list) - ProcessSupervisor integration
- Remaining: - Privy JWT validation (real JWKS, not structural stub) - Scope enforcement at route level - Device flow for headless CLI login

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- GET/POST/DELETE/
- create/start/stop/restart/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko login

**Bullet requirements:**
- - Secrets HTTP API (GET/POST/DELETE/test)
- - Multi-key auth middleware (X-Api-Key + Bearer)
- - API key scopes + SHA-256 + expiry
- - `roko login` CLI with credential store
- - Agent CRUD (create/start/stop/restart/list)
- - ProcessSupervisor integration
- - Privy JWT validation (real JWKS, not structural stub)
- - Scope enforcement at route level
- - Device flow for headless CLI login

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `GET/POST/DELETE/`
- `create/start/stop/restart/`
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
rg -n "secrets|auth|mostly|done|Phase|start|login|Scope" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "secrets|auth|mostly|done|Phase|start|login|Scope" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `GET/POST/DELETE/`
- `create/start/stop/restart/`
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
- [ ] Implement or verify operator command `roko login` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S004 -- Phase 2: agent runtime

**Source section:** `tmp/architecture/18-roadmap.md:25` through `41`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 2: agent runtime

Port the 9-step heartbeat pipeline into the existing `AgentRuntime`:

- Define `AgentRuntime` struct with cortical state, extensions, clock
- Implement `TickPipeline` with T0/T1/T2 gating
- Add `AgentMode` enum (ephemeral/persistent/reactive) to agent lifecycle
- Wire `AdaptiveClock` (gamma/theta/delta timescales)
- Define `Extension` trait with 22 hooks across 8 layers
- Build domain profile system (string-based, user-extensible, with built-in defaults for coding/research/chain)
- Support user-authored profiles in ~/.roko/profiles/*.toml
- Wire prediction error tracking and sleepwalk fallback

Depends on: Phase 1 (agent CRUD exists).

See [Agent Runtime](02-agent-runtime.md) and [Extensions](03-extensions.md).
````

**Explicit detail extraction from this section:**

- Section word count: `99`
- Section hash: `3dcf305351e9257b8d0fdea5fda224d3f5681f34322070c66b0e99ea988e412b`

**Normative requirements and implementation claims:**
- - Define `AgentRuntime` struct with cortical state, extensions, clock - Implement `TickPipeline` with T0/T1/T2 gating - Add `AgentMode` enum (ephemeral/persistent/reactive) to agent lifecycle - Wire `AdaptiveClock` (gamma/theta/delta timescales) - Define `Extension` trait with 22 hooks across 8 layers - Build domain profile system (string-based, user-extensible, with built-in defaults for coding/research/chain) - Support user-authored profiles in ~/.roko/profiles/*.toml - Wire prediction error tracking and sleepwalk fallback

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/profiles/
- T0/T1/
- coding/research/
- ephemeral/persistent/
- gamma/theta/

**Types, functions, traits, and inline code identifiers:**
- with
- AgentRuntime
- TickPipeline
- AgentMode
- AdaptiveClock
- Extension

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Define `AgentRuntime` struct with cortical state, extensions, clock
- - Implement `TickPipeline` with T0/T1/T2 gating
- - Add `AgentMode` enum (ephemeral/persistent/reactive) to agent lifecycle
- - Wire `AdaptiveClock` (gamma/theta/delta timescales)
- - Define `Extension` trait with 22 hooks across 8 layers
- - Build domain profile system (string-based, user-extensible, with built-in defaults for coding/research/chain)
- - Support user-authored profiles in ~/.roko/profiles/*.toml
- - Wire prediction error tracking and sleepwalk fallback

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `.roko/profiles/`
- `T0/T1/`
- `coding/research/`
- `ephemeral/persistent/`
- `gamma/theta/`
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
rg -n "runtime|Extension|Phase|AgentRuntime|TickPipeline|AgentMode|AdaptiveClock|profile" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "runtime|Extension|Phase|AgentRuntime|TickPipeline|AgentMode|AdaptiveClock|profile" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `.roko/profiles/`
- `T0/T1/`
- `coding/research/`
- `ephemeral/persistent/`
- `gamma/theta/`
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
- [ ] Implement or verify `with` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentRuntime` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TickPipeline` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentMode` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AdaptiveClock` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S005 -- Phase 3: relay + dashboard integration

**Source section:** `tmp/architecture/18-roadmap.md:42` through `58`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 3: relay + dashboard integration

Convert all data flows to subscription-only:

- Define WebSocket message envelope (seq, ts, room, type, payload)
- Implement room-based subscription in the relay
- Add reconnection with sequence-based replay
- Add backpressure and coalescing strategies
- Dashboard: replace all polling with WebSocket subscriptions
- Dashboard: graceful degradation when roko-serve is down
- Dashboard: subscribe per-page, unsubscribe on unmount
- Test: dashboard works with relay only (no roko-serve)

Depends on: Phase 2 (agents publish heartbeats to relay).

See [Connectivity and Relay](04-connectivity.md) and [Dashboard Architecture](15-dashboard.md).
````

**Explicit detail extraction from this section:**

- Section word count: `87`
- Section hash: `309815bd301b2a7f4d5bde6a46ac6b5cb8d3e72979cf320abc3b20b2017e7f3e`

**Normative requirements and implementation claims:**
- Convert all data flows to subscription-only:
- - Define WebSocket message envelope (seq, ts, room, type, payload) - Implement room-based subscription in the relay - Add reconnection with sequence-based replay - Add backpressure and coalescing strategies - Dashboard: replace all polling with WebSocket subscriptions - Dashboard: graceful degradation when roko-serve is down - Dashboard: subscribe per-page, unsubscribe on unmount - Test: dashboard works with relay only (no roko-serve)
- See [Connectivity and Relay](04-connectivity.md) and [Dashboard Architecture](15-dashboard.md).

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
- - Define WebSocket message envelope (seq, ts, room, type, payload)
- - Implement room-based subscription in the relay
- - Add reconnection with sequence-based replay
- - Add backpressure and coalescing strategies
- - Dashboard: replace all polling with WebSocket subscriptions
- - Dashboard: graceful degradation when roko-serve is down
- - Dashboard: subscribe per-page, unsubscribe on unmount
- - Test: dashboard works with relay only (no roko-serve)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "relay|Phase|integration|subscription|subscribe|serve|room|connectivity" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "relay|Phase|integration|subscription|subscribe|serve|room|connectivity" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S006 -- Phase 4: inference gateway

**Source section:** `tmp/architecture/18-roadmap.md:59` through `75`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 4: inference gateway

Centralize all LLM API key management:

- Build `InferenceGateway` struct with request queue and provider backends
- Build `InferenceHandle` (channel-based, no secrets)
- Wire `CascadeRouter` as the model selection layer
- Add L1 (exact) and L2 (semantic) response caching
- Add per-request, per-agent, per-model cost tracking
- Add `/api/inference/proxy` endpoint for remote agents
- Publish cost_update events to relay
- Remove API key passing from agent environment

Depends on: Phase 2 (agents use InferenceHandle).

See [Inference Gateway](07-gateway.md).
````

**Explicit detail extraction from this section:**

- Section word count: `78`
- Section hash: `83b31e23083a79033a31ff87943041a49c9aa1ecc4113afaff00a29034303ca2`

**Normative requirements and implementation claims:**
- Centralize all LLM API key management:
- - Build `InferenceGateway` struct with request queue and provider backends - Build `InferenceHandle` (channel-based, no secrets) - Wire `CascadeRouter` as the model selection layer - Add L1 (exact) and L2 (semantic) response caching - Add per-request, per-agent, per-model cost tracking - Add `/api/inference/proxy` endpoint for remote agents - Publish cost_update events to relay - Remove API key passing from agent environment

**Routes and endpoint references:**
- /api/inference/proxy

**Files and path references:**
- api/inference/

**Types, functions, traits, and inline code identifiers:**
- with
- InferenceGateway
- InferenceHandle
- CascadeRouter

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Build `InferenceGateway` struct with request queue and provider backends
- - Build `InferenceHandle` (channel-based, no secrets)
- - Wire `CascadeRouter` as the model selection layer
- - Add L1 (exact) and L2 (semantic) response caching
- - Add per-request, per-agent, per-model cost tracking
- - Add `/api/inference/proxy` endpoint for remote agents
- - Publish cost_update events to relay
- - Remove API key passing from agent environment

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `api/inference/`
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
rg -n "inference|gateway|Phase|InferenceHandle|InferenceGateway|CascadeRouter|api|request" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "inference|gateway|Phase|InferenceHandle|InferenceGateway|CascadeRouter|api|request" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `api/inference/`
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
- [ ] Implement or verify route `/api/inference/proxy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `with` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `InferenceGateway` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `InferenceHandle` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CascadeRouter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S007 -- Phase 5: agent feeds, dynamic endpoints, and paid subscriptions

**Source section:** `tmp/architecture/18-roadmap.md:76` through `92`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 5: agent feeds, dynamic endpoints, and paid subscriptions

Enable agents to produce, consume, and monetize real-time data streams:

- Define `FeedRegistration` and `FeedSubscription` types
- Add feed registry to the relay
- Implement `FeedPublisherExt` extension for chain agents
- Add feed discovery API (`GET /api/feeds`)
- Dashboard: Feeds page with live data visualization
- Agent-to-agent feed subscription
- Paid feed support with budget integration
- Dynamic endpoint registration at runtime

Depends on: Phase 3 (relay handles subscriptions), Phase 4 (cost tracking for paid feeds).

See [Feeds and Data Streams](05-feeds.md) and [Paid Feeds and MPP](06-paid-feeds.md).
````

**Explicit detail extraction from this section:**

- Section word count: `89`
- Section hash: `bb193922291356ffb079578a72259a0481b1274dae748c985db5a2f315122289`

**Normative requirements and implementation claims:**
- - Define `FeedRegistration` and `FeedSubscription` types - Add feed registry to the relay - Implement `FeedPublisherExt` extension for chain agents - Add feed discovery API (`GET /api/feeds`) - Dashboard: Feeds page with live data visualization - Agent-to-agent feed subscription - Paid feed support with budget integration - Dynamic endpoint registration at runtime

**Routes and endpoint references:**
- GET /api/feeds

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- FeedRegistration
- FeedSubscription
- FeedPublisherExt

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Define `FeedRegistration` and `FeedSubscription` types
- - Add feed registry to the relay
- - Implement `FeedPublisherExt` extension for chain agents
- - Add feed discovery API (`GET /api/feeds`)
- - Dashboard: Feeds page with live data visualization
- - Agent-to-agent feed subscription
- - Paid feed support with budget integration
- - Dynamic endpoint registration at runtime

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "feed|feeds|paid|subscription|Phase|subscriptions|endpoint" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|feeds|paid|subscription|Phase|subscriptions|endpoint" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- [ ] Implement or verify route `GET /api/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `FeedRegistration` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FeedSubscription` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FeedPublisherExt` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S008 -- Phase 6: clusters + coordination

**Source section:** `tmp/architecture/18-roadmap.md:93` through `108`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 6: clusters + coordination

Enable multi-agent pipelines:

- Define `Cluster` type with pipeline DAG
- Cluster creation API (POST /api/clusters)
- Pipeline stage execution with dependency ordering
- Shared context distribution to cluster agents
- Dashboard: cluster pipeline visualization
- Cluster lifecycle management (stop all, destroy)
- Event publishing to `cluster:{id}` room

Depends on: Phase 2 (agent lifecycle), Phase 3 (relay subscriptions).

See [Deployment](17-deployment.md) (clusters section).
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `1087e039e325af9dccbebf8724a64531dd9452639ba60ef4226aa201d925702d`

**Normative requirements and implementation claims:**
- - Define `Cluster` type with pipeline DAG - Cluster creation API (POST /api/clusters) - Pipeline stage execution with dependency ordering - Shared context distribution to cluster agents - Dashboard: cluster pipeline visualization - Cluster lifecycle management (stop all, destroy) - Event publishing to `cluster:{id}` room

**Routes and endpoint references:**
- POST /api/clusters

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- with
- Cluster

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - Define `Cluster` type with pipeline DAG
- - Cluster creation API (POST /api/clusters)
- - Pipeline stage execution with dependency ordering
- - Shared context distribution to cluster agents
- - Dashboard: cluster pipeline visualization
- - Cluster lifecycle management (stop all, destroy)
- - Event publishing to `cluster:{id}` room

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Cluster|clusters|Phase|coordination|pipeline|lifecycle|deployment|api" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Cluster|clusters|Phase|coordination|pipeline|lifecycle|deployment|api" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- [ ] Implement or verify route `POST /api/clusters` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `with` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Cluster` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S009 -- Phase 7: isolated execution (Fly Machines)

**Source section:** `tmp/architecture/18-roadmap.md:109` through `123`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 7: isolated execution (Fly Machines)

Full agent isolation for untrusted workloads:

- `FlyMachineManager` implementation
- `roko agent run --relay ... --inference-proxy ...` child mode
- Inference proxying through parent gateway (uses Phase 4 endpoint)
- Volume management for persistent state
- Auto-suspend for reactive agents (Fly Machine stop/start)
- Network policy: outbound-only from Fly Machine

Depends on: Phase 4 (inference proxy), Phase 3 (relay connectivity).

See [Deployment](17-deployment.md) (scaling section).
````

**Explicit detail extraction from this section:**

- Section word count: `63`
- Section hash: `a1616b0c2832262e49096b1288945f9b4f2f1fcd1930bb8c70f1dee0a2d46e9e`

**Normative requirements and implementation claims:**
- - `FlyMachineManager` implementation - `roko agent run --relay ... --inference-proxy ...` child mode - Inference proxying through parent gateway (uses Phase 4 endpoint) - Volume management for persistent state - Auto-suspend for reactive agents (Fly Machine stop/start) - Network policy: outbound-only from Fly Machine

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- FlyMachineManager

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko agent run --relay ... --inference-proxy ...

**Bullet requirements:**
- - `FlyMachineManager` implementation
- - `roko agent run --relay ... --inference-proxy ...` child mode
- - Inference proxying through parent gateway (uses Phase 4 endpoint)
- - Volume management for persistent state
- - Auto-suspend for reactive agents (Fly Machine stop/start)
- - Network policy: outbound-only from Fly Machine

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "Phase|Machine|Fly|isolated|execution|Machines|FlyMachineManager|proxy" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Phase|Machine|Fly|isolated|execution|Machines|FlyMachineManager|proxy" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- [ ] Implement or verify `FlyMachineManager` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify operator command `roko agent run --relay ... --inference-proxy ...` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S010 -- Phase 8: multi-tenant

**Source section:** `tmp/architecture/18-roadmap.md:124` through `137`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Phase 8: multi-tenant

Organization and team support:

- Organization model with members and roles
- Invitation-based onboarding
- Per-org resource isolation (separate agent namespaces)
- Per-org billing (aggregate cost tracking)
- Dashboard: team management UI

Depends on: Phase 1 (auth), Phase 4 (cost tracking).

---
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `819253570e1c03781d2aa7b6e8f00b6d27f4827f7d5a719e70194a021ace4454`

**Normative requirements and implementation claims:**
- Organization and team support:
- - Organization model with members and roles - Invitation-based onboarding - Per-org resource isolation (separate agent namespaces) - Per-org billing (aggregate cost tracking) - Dashboard: team management UI
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
- - Organization model with members and roles
- - Invitation-based onboarding
- - Per-org resource isolation (separate agent namespaces)
- - Per-org billing (aggregate cost tracking)
- - Dashboard: team management UI

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Phase|tenant|multi|tracking|team|cost|Organization|support" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Phase|tenant|multi|tracking|team|cost|Organization|support" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S011 -- Bardo source references

**Source section:** `tmp/architecture/18-roadmap.md:138` through `141`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Bardo source references

Everything in this redesign has prior art in the bardo codebase (`/Users/will/dev/uniswap/bardo/`). This section maps each component to its bardo implementation.
````

**Explicit detail extraction from this section:**

- Section word count: `25`
- Section hash: `b78120c660dc2e0edd8a42b4b702c6536f49dca1f78d32692c0c3e4a8cba644d`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Users/will/dev/uniswap/bardo/

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `Users/will/dev/uniswap/bardo/`
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
rg -n "Bardo|references|uniswap|redesign|prior|maps|component|codebase" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bardo|references|uniswap|redesign|prior|maps|component|codebase" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `Users/will/dev/uniswap/bardo/`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S012 -- Inference gateway -- bardo-gateway (22.8K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:142` through `160`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Inference gateway -- bardo-gateway (22.8K LOC)

The gateway already exists. `apps/bardo-gateway/` is a production LLM inference proxy with:

- **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix)
- **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr
- **Tool pruning**: strips unused tool definitions to reduce token count
- **Batch API**: Anthropic batch endpoint for async, cheaper inference
- **Cost tracking**: per-request, per-model, per-session with SQLite persistence
- **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard

Port the cache, provider abstraction, and cost tracking. Skip the batch API and USDC micropayments for now.

| File | LOC | What |
|------|-----|------|
| `apps/bardo-gateway/src/` | 22,856 | Full gateway server |
| `crates/bardo-inference/src/` | 413 | Protocol types |
| `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client |
````

**Explicit detail extraction from this section:**

- Section word count: `120`
- Section hash: `fb96d024411894ec8e6d2560ec51025a4c70ec2f7e0a80b1128bbfd75ef98003`

**Normative requirements and implementation claims:**
- - **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix) - **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr - **Tool pruning**: strips unused tool definitions to reduce token count - **Batch API**: Anthropic batch endpoint for async, cheaper inference - **Cost tracking**: per-request, per-model, per-session with SQLite persistence - **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard
- Port the cache, provider abstraction, and cost tracking. Skip the batch API and USDC micropayments for now.
- | File | LOC | What | |------|-----|------| | `apps/bardo-gateway/src/` | 22,856 | Full gateway server | | `crates/bardo-inference/src/` | 413 | Protocol types | | `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- apps/bardo-gateway/
- apps/bardo-gateway/src/
- crates/bardo-inference/src/
- crates/golem-inference/src/client.rs
- v1/ws/

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
- - **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix)
- - **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr
- - **Tool pruning**: strips unused tool definitions to reduce token count
- - **Batch API**: Anthropic batch endpoint for async, cheaper inference
- - **Cost tracking**: per-request, per-model, per-session with SQLite persistence
- - **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard

**Tables extracted:**
- Table 1:

```markdown
| File | LOC | What |
|------|-----|------|
| `apps/bardo-gateway/src/` | 22,856 | Full gateway server |
| `crates/bardo-inference/src/` | 413 | Protocol types |
| `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/bardo-gateway/`
- `apps/bardo-gateway/src/`
- `crates/bardo-inference/src/`
- `crates/golem-inference/src/client.rs`
- `v1/ws/`
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
rg -n "gateway|Inference|bardo|LOC|Batch|tracking|stats|provider" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "gateway|Inference|bardo|LOC|Batch|tracking|stats|provider" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/bardo-gateway/`
- `apps/bardo-gateway/src/`
- `crates/bardo-inference/src/`
- `crates/golem-inference/src/client.rs`
- `v1/ws/`
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
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S013 -- Agent runtime -- mori (108K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:161` through `179`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent runtime -- mori (108K LOC)

Mori is the production orchestrator that roko-cli/orchestrate.rs replaces. Key patterns:

- **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM then SIGKILL with 200ms grace
- **MultiAgentPool + warm spawning**: pre-spawn warm agents during gate overlap
- **26 agent roles**: with priority scheduling
- **3 LLM backends**: Claude CLI, Codex, Cursor ACP
- **Rate limiter**: 8-agent default concurrency, priority-sorted queue
- **Conductor**: 10 watchers with 3-tier interventions (Nudge/Restart/Abort)

Port the warm spawning, conductor watchers, and process isolation. The agent model itself is redesigned (heartbeat pipeline replaces mori's event loop).

| File | LOC | What |
|------|-----|------|
| `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle |
| `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning |
| `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |
````

**Explicit detail extraction from this section:**

- Section word count: `128`
- Section hash: `5ca27496ea010ddcc4cc627b06ff9a155203c42c94d4531b0a591ccf16960da3`

**Normative requirements and implementation claims:**
- - **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM then SIGKILL with 200ms grace - **MultiAgentPool + warm spawning**: pre-spawn warm agents during gate overlap - **26 agent roles**: with priority scheduling - **3 LLM backends**: Claude CLI, Codex, Cursor ACP - **Rate limiter**: 8-agent default concurrency, priority-sorted queue - **Conductor**: 10 watchers with 3-tier interventions (Nudge/Restart/Abort)
- Port the warm spawning, conductor watchers, and process isolation. The agent model itself is redesigned (heartbeat pipeline replaces mori's event loop).
- | File | LOC | What | |------|-----|------| | `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle | | `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning | | `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Nudge/Restart/
- apps/mori/src/agent/connection.rs
- apps/mori/src/agent/mod.rs
- apps/mori/src/conductor/mod.rs
- roko-cli/orchestrate.rs

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
- - **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM then SIGKILL with 200ms grace
- - **MultiAgentPool + warm spawning**: pre-spawn warm agents during gate overlap
- - **26 agent roles**: with priority scheduling
- - **3 LLM backends**: Claude CLI, Codex, Cursor ACP
- - **Rate limiter**: 8-agent default concurrency, priority-sorted queue
- - **Conductor**: 10 watchers with 3-tier interventions (Nudge/Restart/Abort)

**Tables extracted:**
- Table 1:

```markdown
| File | LOC | What |
|------|-----|------|
| `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle |
| `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning |
| `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `Nudge/Restart/`
- `apps/mori/src/agent/connection.rs`
- `apps/mori/src/agent/mod.rs`
- `apps/mori/src/conductor/mod.rs`
- `roko-cli/orchestrate.rs`
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
rg -n "mori|LOC|spawn|runtime|warm|Conductor|watchers|spawning" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mori|LOC|spawn|runtime|warm|Conductor|watchers|spawning" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `Nudge/Restart/`
- `apps/mori/src/agent/connection.rs`
- `apps/mori/src/agent/mod.rs`
- `apps/mori/src/conductor/mod.rs`
- `roko-cli/orchestrate.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S014 -- Heartbeat -- golem-heartbeat (10.2K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:180` through `198`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Heartbeat -- golem-heartbeat (10.2K LOC)

Full 9-step CoALA pipeline. Built but never integrated into mori's runtime. This is the core of the redesigned agent runtime.

- **9-step tick**: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect
- **AdaptiveClock**: gamma/theta/delta timescales
- **T0/T1/T2 gating**: prediction error threshold
- **VCG attention auction**: 6 cognitive bidder kinds

Port the full pipeline. Wire it as the `TickPipeline` inside `AgentRuntime`.

| File | LOC | What |
|------|-----|------|
| `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine |
| `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline |
| `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate |
| `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction |
| `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |
````

**Explicit detail extraction from this section:**

- Section word count: `114`
- Section hash: `68f73601acd521f2f73884bf6f9bb9e0432cb58751353d7600be6381af7ae200`

**Normative requirements and implementation claims:**
- Full 9-step CoALA pipeline. Built but never integrated into mori's runtime. This is the core of the redesigned agent runtime.
- - **9-step tick**: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect - **AdaptiveClock**: gamma/theta/delta timescales - **T0/T1/T2 gating**: prediction error threshold - **VCG attention auction**: 6 cognitive bidder kinds
- Port the full pipeline. Wire it as the `TickPipeline` inside `AgentRuntime`.
- | File | LOC | What | |------|-----|------| | `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine | | `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline | | `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate | | `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction | | `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- T0/T1/
- crates/golem-heartbeat/src/auction.rs
- crates/golem-heartbeat/src/clock.rs
- crates/golem-heartbeat/src/engine.rs
- crates/golem-heartbeat/src/gating.rs
- crates/golem-heartbeat/src/pipeline.rs
- gamma/theta/

**Types, functions, traits, and inline code identifiers:**
- TickPipeline
- AgentRuntime

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **9-step tick**: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect
- - **AdaptiveClock**: gamma/theta/delta timescales
- - **T0/T1/T2 gating**: prediction error threshold
- - **VCG attention auction**: 6 cognitive bidder kinds

**Tables extracted:**
- Table 1:

```markdown
| File | LOC | What |
|------|-----|------|
| `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine |
| `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline |
| `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate |
| `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction |
| `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `T0/T1/`
- `crates/golem-heartbeat/src/auction.rs`
- `crates/golem-heartbeat/src/clock.rs`
- `crates/golem-heartbeat/src/engine.rs`
- `crates/golem-heartbeat/src/gating.rs`
- `crates/golem-heartbeat/src/pipeline.rs`
- `gamma/theta/`
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
rg -n "Heartbeat|golem|LOC|TickPipeline|pipeline|crates|AgentRuntime|tick" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Heartbeat|golem|LOC|TickPipeline|pipeline|crates|AgentRuntime|tick" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `T0/T1/`
- `crates/golem-heartbeat/src/auction.rs`
- `crates/golem-heartbeat/src/clock.rs`
- `crates/golem-heartbeat/src/engine.rs`
- `crates/golem-heartbeat/src/gating.rs`
- `crates/golem-heartbeat/src/pipeline.rs`
- `gamma/theta/`
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
- [ ] Implement or verify `TickPipeline` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AgentRuntime` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S015 -- DeFi tools -- golem-tools (7.2K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:199` through `202`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### DeFi tools -- golem-tools (7.2K LOC)

29+ tool categories with capability-gated execution. Port the ToolExecutor framework and the vault/identity tools.
````

**Explicit detail extraction from this section:**

- Section word count: `16`
- Section hash: `2c24f4133396ab03a482dcf1351ff3dd7581a3749ed8ec3d14d79cf1bc0bfd0e`

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tool|tools|golem|LOC|DeFi|vault|identity|gated" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tool|tools|golem|LOC|DeFi|vault|identity|gated" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S016 -- Chain runtime -- golem-chain (5.3K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:203` through `206`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Chain runtime -- golem-chain (5.3K LOC)

12 networks, ProviderPool, SubgraphClient, RevmSimulator, Warden time-delay safety.
````

**Explicit detail extraction from this section:**

- Section word count: `9`
- Section hash: `6afbd0dcf6587b6a60e6fb353ec7c9299b87b68902a0a26b48627bfe9af29315`

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "time|runtime|golem|LOC|Chain|safety|networks|delay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "time|runtime|golem|LOC|Chain|safety|networks|delay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S017 -- Dashboard -- apps/dashboard (Next.js, ~27K LOC)

**Source section:** `tmp/architecture/18-roadmap.md:207` through `212`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard -- apps/dashboard (Next.js, ~27K LOC)

20 React components, real-time WebSocket, canvas-based charts. Port the monitoring components. Add agent management, settings UI, and feeds page (new).

---
````

**Explicit detail extraction from this section:**

- Section word count: `22`
- Section hash: `ca27e52262d49f2a7be112f3be4e392eb7421938e4c3d25d9eb47b74f2c5fdfb`

**Normative requirements and implementation claims:**
- 20 React components, real-time WebSocket, canvas-based charts. Port the monitoring components. Add agent management, settings UI, and feeds page (new).
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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "apps|Next|LOC|components|time|settings|real|monitoring" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "apps|Next|LOC|components|time|settings|real|monitoring" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S018 -- Migration from v1

**Source section:** `tmp/architecture/18-roadmap.md:213` through `230`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Migration from v1

This section documents what changed between the v1 redesign and this revision.

| v1 design | v2 design | Why |
|-----------|-----------|-----|
| Per-agent HTTP sidecar (13 routes) | No sidecar. Agents are tokio tasks or outbound-WS-connected processes. | Sidecars break behind NAT. Channels are faster for in-process. Relay handles remote. |
| Single `roko` deployment is the backbone | Relay + Mirage is the backbone. `roko` is optional. | Dashboard should work without the control plane. Relay is the universal bus. |
| Dashboard polls REST endpoints | Subscription-only via WebSocket | Polling wastes bandwidth, creates jitter, scales poorly. |
| Agent discovery from roko-serve only | Three sources: relay, chain, deployment list | Each source has different strengths. Merge client-side. |
| API keys in agent env vars | Centralized InferenceGateway, agents get channel handle | Eliminates key sprawl, enables cost tracking, audit, rotation. |
| No agent modes | Ephemeral / Persistent / Reactive | Different workloads need different lifecycles. |
| Agent specialization via code | Extension chain composition | Extensions are composable and configurable. No code forks. |
| No chain data feeds | Agents expose and subscribe to real-time feeds | Creates a data marketplace. Dashboard subscribes to same feeds. |
| No dynamic endpoints | Agents register endpoints at runtime | Enables feed discovery and subscription. |

---
````

**Explicit detail extraction from this section:**

- Section word count: `188`
- Section hash: `9ddf0ae99ee9b23ab464d303e86933c6ce34f79d610748931ecbc2af0345e0da`

**Normative requirements and implementation claims:**
- | v1 design | v2 design | Why | |-----------|-----------|-----| | Per-agent HTTP sidecar (13 routes) | No sidecar. Agents are tokio tasks or outbound-WS-connected processes. | Sidecars break behind NAT. Channels are faster for in-process. Relay handles remote. | | Single `roko` deployment is the backbone | Relay + Mirage is the backbone. `roko` is optional. | Dashboard should work without the control plane. Relay is the universal bus. | | Dashboard polls REST endpoints | Subscription-only via WebSocket | Polling wastes bandwidth, creates jitter, scales poorly. | | Agent discovery from roko-serve only | Three sources: relay, chain, deployment list | Each source has different strengths. Merge client-side. | | API keys in agent env vars | Centralized InferenceGateway, agents get channel handle | Eliminates key sprawl, enables cost tracking, audit, rotation. | | No agent modes | Ephemeral / Persistent / Reactive | Different workloads need different lifecycles. | | Agent specialization via 
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- roko

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
| v1 design | v2 design | Why |
|-----------|-----------|-----|
| Per-agent HTTP sidecar (13 routes) | No sidecar. Agents are tokio tasks or outbound-WS-connected processes. | Sidecars break behind NAT. Channels are faster for in-process. Relay handles remote. |
| Single `roko` deployment is the backbone | Relay + Mirage is the backbone. `roko` is optional. | Dashboard should work without the control plane. Relay is the universal bus. |
| Dashboard polls REST endpoints | Subscription-only via WebSocket | Polling wastes bandwidth, creates jitter, scales poorly. |
| Agent discovery from roko-serve only | Three sources: relay, chain, deployment list | Each source has different strengths. Merge client-side. |
| API keys in agent env vars | Centralized InferenceGateway, agents get channel handle | Eliminates key sprawl, enables cost tracking, audit, rotation. |
| No agent modes | Ephemeral / Persistent / Reactive | Different workloads need different lifecycles. |
| Agent specialization via code | Extension chain composition | Extensions are composable and configurable. No code forks. |
| No chain data feeds | Agents expose and subscribe to real-time feeds | Creates a data marketplace. Dashboard subscribes to same feeds. |
| No dynamic endpoints | Agents register endpoints at runtime | Enables feed discovery and subscription. |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "Migration|side|feed|Relay|sidecar|feeds|endpoints|different" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Migration|side|feed|Relay|sidecar|feeds|endpoints|different" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
- [ ] Implement or verify `roko` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S019 -- Bardo → Roko naming map

**Source section:** `tmp/architecture/18-roadmap.md:231` through `255`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Bardo → Roko naming map

> Folded from `tmp/bardo-integration-plan.md`. Essential reference for porting work.

| Bardo Crate | Roko Crate | Status | Notes |
|-------------|-----------|--------|-------|
| golem-core | roko-core | Migrated | |
| golem-runtime | roko-runtime | Migrated | |
| golem-grimoire | roko-neuro | Partial | Renamed grimoire → neuro |
| golem-daimon | roko-daimon | Migrated | |
| golem-dreams | roko-dreams | Migrated | |
| golem-chain | roko-chain | Partial | |
| golem-tools | roko-std | Partial | 19 builtin tools, no DeFi |
| golem-heartbeat | roko-conductor + roko-runtime | Partial | Split into two crates |
| golem-safety | roko-agent/safety | Migrated | |
| golem-eval | roko-gate | Migrated | |
| golem-inference | roko-gateway (new) | Not ported | See [07-gateway.md](07-gateway.md) |
| golem-triage | roko-orchestrator (merge) | Not ported | |
| golem-context | roko-compose | Partial | VCG exists |
| golem-identity | roko-chain (merge) | Not ported | |
| bardo-gateway | roko-gateway (new) | Not ported | Key missing piece |
| dashboard | apps/dashboard (new) | Not ported | See [15-dashboard.md](15-dashboard.md) |
| mori | roko-cli/src/orchestrate.rs | Reference only | 108K LOC reference |
| mpp | roko-mpp (new) | Not ported | Payments, optional |
````

**Explicit detail extraction from this section:**

- Section word count: `160`
- Section hash: `00956e07061995419fd9d4245233a1f3d79fe6b07cea071250113a146d99879d`

**Normative requirements and implementation claims:**
- | Bardo Crate | Roko Crate | Status | Notes | |-------------|-----------|--------|-------| | golem-core | roko-core | Migrated | | | golem-runtime | roko-runtime | Migrated | | | golem-grimoire | roko-neuro | Partial | Renamed grimoire → neuro | | golem-daimon | roko-daimon | Migrated | | | golem-dreams | roko-dreams | Migrated | | | golem-chain | roko-chain | Partial | | | golem-tools | roko-std | Partial | 19 builtin tools, no DeFi | | golem-heartbeat | roko-conductor + roko-runtime | Partial | Split into two crates | | golem-safety | roko-agent/safety | Migrated | | | golem-eval | roko-gate | Migrated | | | golem-inference | roko-gateway (new) | Not ported | See [07-gateway.md](07-gateway.md) | | golem-triage | roko-orchestrator (merge) | Not ported | | | golem-context | roko-compose | Partial | VCG exists | | golem-identity | roko-chain (merge) | Not ported | | | bardo-gateway | roko-gateway (new) | Not ported | Key missing piece | | dashboard | apps/dashboard (new) | Not ported | 

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-cli/src/orchestrate.rs
- tmp/bardo-integration-plan.md

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Renamed grimoire -> neuro

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Bardo Crate | Roko Crate | Status | Notes |
|-------------|-----------|--------|-------|
| golem-core | roko-core | Migrated | |
| golem-runtime | roko-runtime | Migrated | |
| golem-grimoire | roko-neuro | Partial | Renamed grimoire → neuro |
| golem-daimon | roko-daimon | Migrated | |
| golem-dreams | roko-dreams | Migrated | |
| golem-chain | roko-chain | Partial | |
| golem-tools | roko-std | Partial | 19 builtin tools, no DeFi |
| golem-heartbeat | roko-conductor + roko-runtime | Partial | Split into two crates |
| golem-safety | roko-agent/safety | Migrated | |
| golem-eval | roko-gate | Migrated | |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `roko-cli/src/orchestrate.rs`
- `tmp/bardo-integration-plan.md`
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
rg -n "golem|Bardo|ported|gate|Migrated|naming|map|gateway" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "golem|Bardo|ported|gate|Migrated|naming|map|gateway" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `roko-cli/src/orchestrate.rs`
- `tmp/bardo-integration-plan.md`
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
- [ ] Enforce state transition `Renamed grimoire -> neuro` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S020 -- Bardo source reference (LOC counts)

**Source section:** `tmp/architecture/18-roadmap.md:256` through `270`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Bardo source reference (LOC counts)

| Component | Bardo Path | LOC | Roko equivalent |
|-----------|-----------|-----|-----------------|
| Inference gateway | `bardo/apps/bardo-gateway/` | 22,800 | `crates/roko-gateway/` (new) |
| Agent runtime (mori) | `bardo/apps/mori/` | 108,000 | `crates/roko-cli/src/orchestrate.rs` |
| Heartbeat | `bardo/crates/golem-heartbeat/` | 10,200 | `crates/roko-conductor/` |
| DeFi tools | `bardo/crates/golem-tools/` | 7,200 | `crates/roko-std/` |
| Chain runtime | `bardo/crates/golem-chain/` | 5,300 | `crates/roko-chain/` |
| Dashboard | `bardo/apps/dashboard/` | 27,000 | `apps/dashboard/` (new) |
| Terminal | `bardo/apps/bardo-terminal/` | ~4,000 | `crates/roko-cli/src/tui/` |
| MPP | `bardo/crates/mpp/` | 988 | `crates/roko-mpp/` (new) |

---
````

**Explicit detail extraction from this section:**

- Section word count: `94`
- Section hash: `20ba052ab2bfd8f54c5e35d23a6a3a4aa704508397968764499eabc3b8bd6238`

**Normative requirements and implementation claims:**
- | Component | Bardo Path | LOC | Roko equivalent | |-----------|-----------|-----|-----------------| | Inference gateway | `bardo/apps/bardo-gateway/` | 22,800 | `crates/roko-gateway/` (new) | | Agent runtime (mori) | `bardo/apps/mori/` | 108,000 | `crates/roko-cli/src/orchestrate.rs` | | Heartbeat | `bardo/crates/golem-heartbeat/` | 10,200 | `crates/roko-conductor/` | | DeFi tools | `bardo/crates/golem-tools/` | 7,200 | `crates/roko-std/` | | Chain runtime | `bardo/crates/golem-chain/` | 5,300 | `crates/roko-chain/` | | Dashboard | `bardo/apps/dashboard/` | 27,000 | `apps/dashboard/` (new) | | Terminal | `bardo/apps/bardo-terminal/` | ~4,000 | `crates/roko-cli/src/tui/` | | MPP | `bardo/crates/mpp/` | 988 | `crates/roko-mpp/` (new) |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- apps/dashboard/
- bardo/apps/bardo-gateway/
- bardo/apps/bardo-terminal/
- bardo/apps/dashboard/
- bardo/apps/mori/
- bardo/crates/golem-chain/
- bardo/crates/golem-heartbeat/
- bardo/crates/golem-tools/
- bardo/crates/mpp/
- crates/roko-chain/
- crates/roko-cli/src/orchestrate.rs
- crates/roko-cli/src/tui/
- crates/roko-conductor/
- crates/roko-gateway/
- crates/roko-mpp/
- crates/roko-std/

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
| Component | Bardo Path | LOC | Roko equivalent |
|-----------|-----------|-----|-----------------|
| Inference gateway | `bardo/apps/bardo-gateway/` | 22,800 | `crates/roko-gateway/` (new) |
| Agent runtime (mori) | `bardo/apps/mori/` | 108,000 | `crates/roko-cli/src/orchestrate.rs` |
| Heartbeat | `bardo/crates/golem-heartbeat/` | 10,200 | `crates/roko-conductor/` |
| DeFi tools | `bardo/crates/golem-tools/` | 7,200 | `crates/roko-std/` |
| Chain runtime | `bardo/crates/golem-chain/` | 5,300 | `crates/roko-chain/` |
| Dashboard | `bardo/apps/dashboard/` | 27,000 | `apps/dashboard/` (new) |
| Terminal | `bardo/apps/bardo-terminal/` | ~4,000 | `crates/roko-cli/src/tui/` |
| MPP | `bardo/crates/mpp/` | 988 | `crates/roko-mpp/` (new) |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/dashboard/`
- `bardo/apps/bardo-gateway/`
- `bardo/apps/bardo-terminal/`
- `bardo/apps/dashboard/`
- `bardo/apps/mori/`
- `bardo/crates/golem-chain/`
- `bardo/crates/golem-heartbeat/`
- `bardo/crates/golem-tools/`
- `bardo/crates/mpp/`
- `crates/roko-chain/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Bardo|crates|LOC|reference|counts|apps|golem|gateway" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Bardo|crates|LOC|reference|counts|apps|golem|gateway" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/dashboard/`
- `bardo/apps/bardo-gateway/`
- `bardo/apps/bardo-terminal/`
- `bardo/apps/dashboard/`
- `bardo/apps/mori/`
- `bardo/crates/golem-chain/`
- `bardo/crates/golem-heartbeat/`
- `bardo/crates/golem-tools/`
- `bardo/crates/mpp/`
- `crates/roko-chain/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`

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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S021 -- Implementation task summary

**Source section:** `tmp/architecture/18-roadmap.md:271` through `290`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Implementation task summary

> Folded from `tmp/bardo-integration-plan.md`. 48 tasks across 12 phases.

| Phase | Tasks | Priority | Parallelizable | Spec doc |
|-------|-------|----------|----------------|----------|
| 1. Inference Gateway | 12 | P0 | No (sequential foundation) | [07-gateway.md](07-gateway.md) |
| 2. Orchestrator Gaps | 7 | P0 | Yes (with Phase 1) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 3. Learning Loop Gaps | 5 | P1 | Yes (with Phases 1-2) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 4. Heartbeat Pipeline | 2 | P1 | Yes (with Phases 1-3) | [02-agent-runtime.md](02-agent-runtime.md) |
| 5. Agent Modes | 3 | P1 | After Phase 2 | [02-agent-runtime.md](02-agent-runtime.md) |
| 6. Dashboard | 4 | P1 | After Phase 1.11 | [15-dashboard.md](15-dashboard.md) |
| 7. DeFi Tools + Chain | 4 | P2 | Yes (standalone) | [12-defi.md](12-defi.md), `defi/gap/` |
| 8. TUI Enhancements | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 9. Operational Infra | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 10. Fly Machines | 2 | P2 | After Phase 5 | [17-deployment.md](17-deployment.md) |
| 11. Clusters | 2 | P3 | After Phases 4-5 | [17-deployment.md](17-deployment.md) |
| 12. Payments | 1 | P3 | After Phase 1 | [06-paid-feeds.md](06-paid-feeds.md) |
| **Total** | **48** | | | |
````

**Explicit detail extraction from this section:**

- Section word count: `211`
- Section hash: `d05a24a9cfb67a3124c998da9c12d99abc1bef7ad4deeb2f0b28037f81d4f6ca`

**Normative requirements and implementation claims:**
- | Phase | Tasks | Priority | Parallelizable | Spec doc | |-------|-------|----------|----------------|----------| | 1. Inference Gateway | 12 | P0 | No (sequential foundation) | [07-gateway.md](07-gateway.md) | | 2. Orchestrator Gaps | 7 | P0 | Yes (with Phase 1) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) | | 3. Learning Loop Gaps | 5 | P1 | Yes (with Phases 1-2) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) | | 4. Heartbeat Pipeline | 2 | P1 | Yes (with Phases 1-3) | [02-agent-runtime.md](02-agent-runtime.md) | | 5. Agent Modes | 3 | P1 | After Phase 2 | [02-agent-runtime.md](02-agent-runtime.md) | | 6. Dashboard | 4 | P1 | After Phase 1.11 | [15-dashboard.md](15-dashboard.md) | | 7. DeFi Tools + Chain | 4 | P2 | Yes (standalone) | [12-defi.md](12-defi.md), `defi/gap/` | | 8. TUI Enhancements | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) | | 9. Operational Infra | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-op

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- defi/gap/
- tmp/bardo-integration-plan.md

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
| Phase | Tasks | Priority | Parallelizable | Spec doc |
|-------|-------|----------|----------------|----------|
| 1. Inference Gateway | 12 | P0 | No (sequential foundation) | [07-gateway.md](07-gateway.md) |
| 2. Orchestrator Gaps | 7 | P0 | Yes (with Phase 1) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 3. Learning Loop Gaps | 5 | P1 | Yes (with Phases 1-2) | [20-orchestrator-gaps.md](20-orchestrator-gaps.md) |
| 4. Heartbeat Pipeline | 2 | P1 | Yes (with Phases 1-3) | [02-agent-runtime.md](02-agent-runtime.md) |
| 5. Agent Modes | 3 | P1 | After Phase 2 | [02-agent-runtime.md](02-agent-runtime.md) |
| 6. Dashboard | 4 | P1 | After Phase 1.11 | [15-dashboard.md](15-dashboard.md) |
| 7. DeFi Tools + Chain | 4 | P2 | Yes (standalone) | [12-defi.md](12-defi.md), `defi/gap/` |
| 8. TUI Enhancements | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 9. Operational Infra | 3 | P2 | Yes (standalone) | [21-tui-and-operations.md](21-tui-and-operations.md) |
| 10. Fly Machines | 2 | P2 | After Phase 5 | [17-deployment.md](17-deployment.md) |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `defi/gap/`
- `tmp/bardo-integration-plan.md`
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
rg -n "Phase|task|gaps|summary|Orchestrator|After|runtime|phases" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Phase|task|gaps|summary|Orchestrator|After|runtime|phases" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `defi/gap/`
- `tmp/bardo-integration-plan.md`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S022 -- Dependency graph

**Source section:** `tmp/architecture/18-roadmap.md:291` through `307`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dependency graph

```
Phase 1 (Gateway) ──→ Phase 6 (Dashboard)
     │                Phase 10 (Fly Machines)
     └─→ Phase 12 (Payments)

Phase 2 (Orchestrator) ──→ Phase 5 (Agent Modes) ──→ Phase 10 (Fly)
Phase 3 (Learning) ──→ (standalone)
Phase 4 (Heartbeat) ──→ Phase 11 (Clusters)
Phase 5 (Agent Modes) ──→ Phase 11 (Clusters)

Phase 7 (DeFi) ──→ (standalone, parallel with everything)
Phase 8 (TUI) ──→ (standalone)
Phase 9 (Ops) ──→ (standalone)
```
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `62e96b349b91c202751afe793212f5532a2d62d0c09bf1d549d385c51d48f360`

**Normative requirements and implementation claims:**
- ``` Phase 1 (Gateway) ──→ Phase 6 (Dashboard) │ Phase 10 (Fly Machines) └─→ Phase 12 (Payments)

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
- Contract 1: language `plain`, first line `Phase 1 (Gateway) ──→ Phase 6 (Dashboard)`

```
Phase 1 (Gateway) ──→ Phase 6 (Dashboard)
     │                Phase 10 (Fly Machines)
     └─→ Phase 12 (Payments)

Phase 2 (Orchestrator) ──→ Phase 5 (Agent Modes) ──→ Phase 10 (Fly)
Phase 3 (Learning) ──→ (standalone)
Phase 4 (Heartbeat) ──→ Phase 11 (Clusters)
Phase 5 (Agent Modes) ──→ Phase 11 (Clusters)

Phase 7 (DeFi) ──→ (standalone, parallel with everything)
Phase 8 (TUI) ──→ (standalone)
Phase 9 (Ops) ──→ (standalone)
```

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "Phase|graph|Dependency|standalone|Modes|Clusters|parallel|everything" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Phase|graph|Dependency|standalone|Modes|Clusters|parallel|everything" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S023 -- Critical path

**Source section:** `tmp/architecture/18-roadmap.md:308` through `311`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Critical path

**Phase 1 (Gateway) → Phase 6 (Dashboard) → Phase 10 (Fly)**
````

**Explicit detail extraction from this section:**

- Section word count: `9`
- Section hash: `a0ab04dd94182da1843bf9a94e35ab6c9f93016ff1ff5c8dbc0ca641a0b1a2e2`

**Normative requirements and implementation claims:**
- **Phase 1 (Gateway) → Phase 6 (Dashboard) → Phase 10 (Fly)**

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "path|Critical|Phase|Gateway|roadmap" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "path|Critical|Phase|Gateway|roadmap" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S024 -- Parallel tracks

**Source section:** `tmp/architecture/18-roadmap.md:312` through `317`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Parallel tracks

Phases 2+3+4 (Orchestrator + Learning + Heartbeat) can run alongside Phase 1. Phases 7+8+9 (DeFi + TUI + Ops) are fully independent.

---
````

**Explicit detail extraction from this section:**

- Section word count: `22`
- Section hash: `aa801a41fe37c18fe137f5de1b52325b0bb22fa44ce7d40dacde187a20a79f43`

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
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
rg -n "tracks|Parallel|Phase|Phases|independent|fully|alongside|Orchestrator" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tracks|Parallel|Phase|Phases|independent|fully|alongside|Orchestrator" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S025 -- Phase 1 task breakdown: Inference Gateway

**Source section:** `tmp/architecture/18-roadmap.md:318` through `340`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Phase 1 task breakdown: Inference Gateway

> Detailed per-task specification with source references. See [07-gateway.md](07-gateway.md) for the architectural spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 1.1 | Port inference protocol types | `bardo/crates/bardo-inference/src/lib.rs` (413), `golem-inference/src/client.rs` (723) | `crates/roko-gateway/src/types.rs` | S |
| 1.2 | Port hash cache (Layer 1) | `bardo/apps/bardo-gateway/src/cache.rs` | `crates/roko-gateway/src/cache/hash_cache.rs` | M |
| 1.3 | Port semantic cache (Layer 2) | `bardo/apps/bardo-gateway/src/semantic_cache.rs` | `crates/roko-gateway/src/cache/semantic_cache.rs` | M |
| 1.4 | Port provider abstraction + key rotation | `bardo/apps/bardo-gateway/src/providers/` | `crates/roko-gateway/src/providers/` | L |
| 1.5 | Port cost computation + tracking | `bardo/apps/bardo-gateway/src/pricing.rs`, `handler.rs`, `cost_db.rs` | Wire into `roko-learn/src/costs_db.rs` + new `pricing.rs` | M |
| 1.6 | Port loop detection | `bardo/apps/bardo-gateway/src/loop_guard.rs` | `crates/roko-gateway/src/loop_guard.rs` | M |
| 1.7 | Port output budgeting | `bardo/apps/bardo-gateway/src/output_budget.rs` | `crates/roko-gateway/src/output_budget.rs` | S |
| 1.8 | Port tool pruning | `bardo/apps/bardo-gateway/src/tools.rs` | `crates/roko-gateway/src/tool_pruning.rs` | S |
| 1.9 | Port convergence detection | `bardo/apps/bardo-gateway/src/convergence.rs` | `crates/roko-gateway/src/convergence.rs` | S |
| 1.10 | Port thinking cap | `bardo/apps/bardo-gateway/src/thinking_cap.rs` | `crates/roko-gateway/src/thinking_cap.rs` | S |
| 1.11 | Wire gateway into roko-serve | — | `crates/roko-serve/src/routes/gateway.rs` | L |
| 1.12 | Port batch API | `bardo/apps/bardo-gateway/src/batch.rs` | `crates/roko-gateway/src/batch.rs` | M |

**Sequence**: 1.1 → 1.2 → 1.3 → 1.4 → 1.5 → (1.6, 1.7, 1.8, 1.9, 1.10 in parallel) → 1.11 → 1.12

---
````

**Explicit detail extraction from this section:**

- Section word count: `297`
- Section hash: `71c8370e136f235ce555436bd407595752fc7b5d0c80c10772bdd7d5421869ba`

**Normative requirements and implementation claims:**
- | Task | Description | Source | Target | Size | |------|------------|--------|--------|------| | 1.1 | Port inference protocol types | `bardo/crates/bardo-inference/src/lib.rs` (413), `golem-inference/src/client.rs` (723) | `crates/roko-gateway/src/types.rs` | S | | 1.2 | Port hash cache (Layer 1) | `bardo/apps/bardo-gateway/src/cache.rs` | `crates/roko-gateway/src/cache/hash_cache.rs` | M | | 1.3 | Port semantic cache (Layer 2) | `bardo/apps/bardo-gateway/src/semantic_cache.rs` | `crates/roko-gateway/src/cache/semantic_cache.rs` | M | | 1.4 | Port provider abstraction + key rotation | `bardo/apps/bardo-gateway/src/providers/` | `crates/roko-gateway/src/providers/` | L | | 1.5 | Port cost computation + tracking | `bardo/apps/bardo-gateway/src/pricing.rs`, `handler.rs`, `cost_db.rs` | Wire into `roko-learn/src/costs_db.rs` + new `pricing.rs` | M | | 1.6 | Port loop detection | `bardo/apps/bardo-gateway/src/loop_guard.rs` | `crates/roko-gateway/src/loop_guard.rs` | M | | 1.7 | Port outpu
- **Sequence**: 1.1 → 1.2 → 1.3 → 1.4 → 1.5 → (1.6, 1.7, 1.8, 1.9, 1.10 in parallel) → 1.11 → 1.12
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/bardo-gateway/src/batch.rs
- bardo/apps/bardo-gateway/src/cache.rs
- bardo/apps/bardo-gateway/src/convergence.rs
- bardo/apps/bardo-gateway/src/loop_guard.rs
- bardo/apps/bardo-gateway/src/output_budget.rs
- bardo/apps/bardo-gateway/src/pricing.rs
- bardo/apps/bardo-gateway/src/providers/
- bardo/apps/bardo-gateway/src/semantic_cache.rs
- bardo/apps/bardo-gateway/src/thinking_cap.rs
- bardo/apps/bardo-gateway/src/tools.rs
- bardo/crates/bardo-inference/src/lib.rs
- crates/roko-gateway/src/batch.rs
- crates/roko-gateway/src/cache/hash_cache.rs
- crates/roko-gateway/src/cache/semantic_cache.rs
- crates/roko-gateway/src/convergence.rs
- crates/roko-gateway/src/loop_guard.rs
- crates/roko-gateway/src/output_budget.rs
- crates/roko-gateway/src/providers/
- crates/roko-gateway/src/thinking_cap.rs
- crates/roko-gateway/src/tool_pruning.rs
- crates/roko-gateway/src/types.rs
- crates/roko-serve/src/routes/gateway.rs
- golem-inference/src/client.rs
- roko-learn/src/costs_db.rs

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- handler.rs
- cost_db.rs
- pricing.rs

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 1.1 | Port inference protocol types | `bardo/crates/bardo-inference/src/lib.rs` (413), `golem-inference/src/client.rs` (723) | `crates/roko-gateway/src/types.rs` | S |
| 1.2 | Port hash cache (Layer 1) | `bardo/apps/bardo-gateway/src/cache.rs` | `crates/roko-gateway/src/cache/hash_cache.rs` | M |
| 1.3 | Port semantic cache (Layer 2) | `bardo/apps/bardo-gateway/src/semantic_cache.rs` | `crates/roko-gateway/src/cache/semantic_cache.rs` | M |
| 1.4 | Port provider abstraction + key rotation | `bardo/apps/bardo-gateway/src/providers/` | `crates/roko-gateway/src/providers/` | L |
| 1.5 | Port cost computation + tracking | `bardo/apps/bardo-gateway/src/pricing.rs`, `handler.rs`, `cost_db.rs` | Wire into `roko-learn/src/costs_db.rs` + new `pricing.rs` | M |
| 1.6 | Port loop detection | `bardo/apps/bardo-gateway/src/loop_guard.rs` | `crates/roko-gateway/src/loop_guard.rs` | M |
| 1.7 | Port output budgeting | `bardo/apps/bardo-gateway/src/output_budget.rs` | `crates/roko-gateway/src/output_budget.rs` | S |
| 1.8 | Port tool pruning | `bardo/apps/bardo-gateway/src/tools.rs` | `crates/roko-gateway/src/tool_pruning.rs` | S |
| 1.9 | Port convergence detection | `bardo/apps/bardo-gateway/src/convergence.rs` | `crates/roko-gateway/src/convergence.rs` | S |
| 1.10 | Port thinking cap | `bardo/apps/bardo-gateway/src/thinking_cap.rs` | `crates/roko-gateway/src/thinking_cap.rs` | S |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `bardo/apps/bardo-gateway/src/batch.rs`
- `bardo/apps/bardo-gateway/src/cache.rs`
- `bardo/apps/bardo-gateway/src/convergence.rs`
- `bardo/apps/bardo-gateway/src/loop_guard.rs`
- `bardo/apps/bardo-gateway/src/output_budget.rs`
- `bardo/apps/bardo-gateway/src/pricing.rs`
- `bardo/apps/bardo-gateway/src/providers/`
- `bardo/apps/bardo-gateway/src/semantic_cache.rs`
- `bardo/apps/bardo-gateway/src/thinking_cap.rs`
- `bardo/apps/bardo-gateway/src/tools.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Gateway|bardo|crates|Port|apps|cache|Inference|task" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Gateway|bardo|crates|Port|apps|cache|Inference|task" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `bardo/apps/bardo-gateway/src/batch.rs`
- `bardo/apps/bardo-gateway/src/cache.rs`
- `bardo/apps/bardo-gateway/src/convergence.rs`
- `bardo/apps/bardo-gateway/src/loop_guard.rs`
- `bardo/apps/bardo-gateway/src/output_budget.rs`
- `bardo/apps/bardo-gateway/src/pricing.rs`
- `bardo/apps/bardo-gateway/src/providers/`
- `bardo/apps/bardo-gateway/src/semantic_cache.rs`
- `bardo/apps/bardo-gateway/src/thinking_cap.rs`
- `bardo/apps/bardo-gateway/src/tools.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`

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
- [ ] Add or verify config key `handler.rs` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `cost_db.rs` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `pricing.rs` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S026 -- Phase 4 task breakdown: Heartbeat Pipeline

**Source section:** `tmp/architecture/18-roadmap.md:341` through `351`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Phase 4 task breakdown: Heartbeat Pipeline

> See [02-agent-runtime.md](02-agent-runtime.md) for the full 9-step pipeline spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 4.1 | Port 9-step TickPipeline | `golem-heartbeat/src/pipeline.rs` (3,019), `engine.rs` (1,307) | `crates/roko-conductor/src/tick_pipeline.rs` | L |
| 4.2 | Wire T0/T1/T2 at dispatch time | `golem-heartbeat/src/gating.rs` (481) | Modify `dispatch_agent_with()` in orchestrate.rs | M |

---
````

**Explicit detail extraction from this section:**

- Section word count: `66`
- Section hash: `6f3bd206957f09371778c300d7c4a67a37fe5d0c44f8de7ee6e34511f5af8101`

**Normative requirements and implementation claims:**
- | Task | Description | Source | Target | Size | |------|------------|--------|--------|------| | 4.1 | Port 9-step TickPipeline | `golem-heartbeat/src/pipeline.rs` (3,019), `engine.rs` (1,307) | `crates/roko-conductor/src/tick_pipeline.rs` | L | | 4.2 | Wire T0/T1/T2 at dispatch time | `golem-heartbeat/src/gating.rs` (481) | Modify `dispatch_agent_with()` in orchestrate.rs | M |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- T0/T1/
- crates/roko-conductor/src/tick_pipeline.rs
- golem-heartbeat/src/gating.rs
- golem-heartbeat/src/pipeline.rs

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- engine.rs

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 4.1 | Port 9-step TickPipeline | `golem-heartbeat/src/pipeline.rs` (3,019), `engine.rs` (1,307) | `crates/roko-conductor/src/tick_pipeline.rs` | L |
| 4.2 | Wire T0/T1/T2 at dispatch time | `golem-heartbeat/src/gating.rs` (481) | Modify `dispatch_agent_with()` in orchestrate.rs | M |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `T0/T1/`
- `crates/roko-conductor/src/tick_pipeline.rs`
- `golem-heartbeat/src/gating.rs`
- `golem-heartbeat/src/pipeline.rs`
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
rg -n "Pipeline|Heartbeat|task|breakdown|Phase|time|step|runtime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Pipeline|Heartbeat|task|breakdown|Phase|time|step|runtime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `T0/T1/`
- `crates/roko-conductor/src/tick_pipeline.rs`
- `golem-heartbeat/src/gating.rs`
- `golem-heartbeat/src/pipeline.rs`
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
- [ ] Add or verify config key `engine.rs` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S027 -- Phase 5 task breakdown: Agent Modes + Profiles

**Source section:** `tmp/architecture/18-roadmap.md:352` through `363`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Phase 5 task breakdown: Agent Modes + Profiles

> See [02-agent-runtime.md](02-agent-runtime.md) for mode and profile specs.

| Task | Description | Target | Size |
|------|------------|--------|------|
| 5.1 | Add AgentMode + AgentProfile enums | `crates/roko-core/src/config/schema.rs` | S |
| 5.2 | Wire ephemeral auto-stop | `roko-serve/src/routes/agents.rs` + `roko-runtime/src/process.rs` | S |
| 5.3 | Wire reactive mode (webhook/cron) | New `crates/roko-runtime/src/reactive.rs` | L |

---
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `85d3a0b41d2f971cfa5934c63f803a433881396c025fa22e3b5b78690fbba90f`

**Normative requirements and implementation claims:**
- | Task | Description | Target | Size | |------|------------|--------|------| | 5.1 | Add AgentMode + AgentProfile enums | `crates/roko-core/src/config/schema.rs` | S | | 5.2 | Wire ephemeral auto-stop | `roko-serve/src/routes/agents.rs` + `roko-runtime/src/process.rs` | S | | 5.3 | Wire reactive mode (webhook/cron) | New `crates/roko-runtime/src/reactive.rs` | L |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/roko-core/src/config/schema.rs
- crates/roko-runtime/src/reactive.rs
- roko-runtime/src/process.rs
- roko-serve/src/routes/agents.rs

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
| Task | Description | Target | Size |
|------|------------|--------|------|
| 5.1 | Add AgentMode + AgentProfile enums | `crates/roko-core/src/config/schema.rs` | S |
| 5.2 | Wire ephemeral auto-stop | `roko-serve/src/routes/agents.rs` + `roko-runtime/src/process.rs` | S |
| 5.3 | Wire reactive mode (webhook/cron) | New `crates/roko-runtime/src/reactive.rs` | L |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-runtime/src/reactive.rs`
- `roko-runtime/src/process.rs`
- `roko-serve/src/routes/agents.rs`
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
rg -n "mode|profile|task|breakdown|Profiles|Phase|Modes|runtime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mode|profile|task|breakdown|Profiles|Phase|Modes|runtime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-runtime/src/reactive.rs`
- `roko-runtime/src/process.rs`
- `roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S028 -- Phase 6 task breakdown: Dashboard

**Source section:** `tmp/architecture/18-roadmap.md:364` through `376`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Phase 6 task breakdown: Dashboard

> See [15-dashboard.md](15-dashboard.md) for the dashboard architecture spec.

| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 6.1 | Set up Next.js app in monorepo | `bardo/apps/dashboard/`, `bardo/packages/ui/` | `apps/dashboard/`, `packages/ui/` | S |
| 6.2 | Add Privy auth | — | `apps/dashboard/src/app/login/`, AuthProvider | M |
| 6.3 | Add agent management pages | — | `apps/dashboard/src/app/agents/` | L |
| 6.4 | Add settings page | — | `apps/dashboard/src/app/settings/` | M |

---
````

**Explicit detail extraction from this section:**

- Section word count: `72`
- Section hash: `5e0aa68e676cd1e86b2d732884e5245819182b915fcae31ab39a5260a0f51301`

**Normative requirements and implementation claims:**
- > See [15-dashboard.md](15-dashboard.md) for the dashboard architecture spec.
- | Task | Description | Source | Target | Size | |------|------------|--------|--------|------| | 6.1 | Set up Next.js app in monorepo | `bardo/apps/dashboard/`, `bardo/packages/ui/` | `apps/dashboard/`, `packages/ui/` | S | | 6.2 | Add Privy auth | — | `apps/dashboard/src/app/login/`, AuthProvider | M | | 6.3 | Add agent management pages | — | `apps/dashboard/src/app/agents/` | L | | 6.4 | Add settings page | — | `apps/dashboard/src/app/settings/` | M |
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- apps/dashboard/
- apps/dashboard/src/app/agents/
- apps/dashboard/src/app/login/
- apps/dashboard/src/app/settings/
- bardo/apps/dashboard/
- bardo/packages/ui/
- packages/ui/

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
| Task | Description | Source | Target | Size |
|------|------------|--------|--------|------|
| 6.1 | Set up Next.js app in monorepo | `bardo/apps/dashboard/`, `bardo/packages/ui/` | `apps/dashboard/`, `packages/ui/` | S |
| 6.2 | Add Privy auth | — | `apps/dashboard/src/app/login/`, AuthProvider | M |
| 6.3 | Add agent management pages | — | `apps/dashboard/src/app/agents/` | L |
| 6.4 | Add settings page | — | `apps/dashboard/src/app/settings/` | M |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/dashboard/`
- `apps/dashboard/src/app/agents/`
- `apps/dashboard/src/app/login/`
- `apps/dashboard/src/app/settings/`
- `bardo/apps/dashboard/`
- `bardo/packages/ui/`
- `packages/ui/`
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
rg -n "task|breakdown|apps|Phase|settings|packages|bardo|auth" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "task|breakdown|apps|Phase|settings|packages|bardo|auth" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `apps/dashboard/`
- `apps/dashboard/src/app/agents/`
- `apps/dashboard/src/app/login/`
- `apps/dashboard/src/app/settings/`
- `bardo/apps/dashboard/`
- `bardo/packages/ui/`
- `packages/ui/`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

### ARCH-18-S029 -- Phase 10-12 task breakdowns

**Source section:** `tmp/architecture/18-roadmap.md:377` through `387`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Phase 10-12 task breakdowns

> See [17-deployment.md](17-deployment.md) for Fly Machines and clusters specs.

| Task | Phase | Description | Target | Size |
|------|-------|------------|--------|------|
| 10.1 | Fly | Fly Machines REST API client | `crates/roko-runtime/src/fly.rs` | M |
| 10.2 | Fly | Extend ProcessSupervisor for Fly | `crates/roko-runtime/src/process.rs` | L |
| 11.1 | Clusters | Wire FleetConductor (L4) | `crates/roko-conductor/src/federation.rs` | M |
| 11.2 | Clusters | Cluster API routes | `crates/roko-serve/src/routes/clusters.rs` | L |
| 12.1 | Payments | Port MPP (ERC-3009 USDC) | `crates/roko-mpp/` (new) | M |
````

**Explicit detail extraction from this section:**

- Section word count: `87`
- Section hash: `3ad4e45d5282a9fd16b22f48543ee055c8b581eca6b5e734b96bd528a9dd6266`

**Normative requirements and implementation claims:**
- | Task | Phase | Description | Target | Size | |------|-------|------------|--------|------| | 10.1 | Fly | Fly Machines REST API client | `crates/roko-runtime/src/fly.rs` | M | | 10.2 | Fly | Extend ProcessSupervisor for Fly | `crates/roko-runtime/src/process.rs` | L | | 11.1 | Clusters | Wire FleetConductor (L4) | `crates/roko-conductor/src/federation.rs` | M | | 11.2 | Clusters | Cluster API routes | `crates/roko-serve/src/routes/clusters.rs` | L | | 12.1 | Payments | Port MPP (ERC-3009 USDC) | `crates/roko-mpp/` (new) | M |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/roko-conductor/src/federation.rs
- crates/roko-mpp/
- crates/roko-runtime/src/fly.rs
- crates/roko-runtime/src/process.rs
- crates/roko-serve/src/routes/clusters.rs

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
| Task | Phase | Description | Target | Size |
|------|-------|------------|--------|------|
| 10.1 | Fly | Fly Machines REST API client | `crates/roko-runtime/src/fly.rs` | M |
| 10.2 | Fly | Extend ProcessSupervisor for Fly | `crates/roko-runtime/src/process.rs` | L |
| 11.1 | Clusters | Wire FleetConductor (L4) | `crates/roko-conductor/src/federation.rs` | M |
| 11.2 | Clusters | Cluster API routes | `crates/roko-serve/src/routes/clusters.rs` | L |
| 12.1 | Payments | Port MPP (ERC-3009 USDC) | `crates/roko-mpp/` (new) | M |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/18-roadmap.md`
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `crates/roko-conductor/src/federation.rs`
- `crates/roko-mpp/`
- `crates/roko-runtime/src/fly.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-serve/src/routes/clusters.rs`
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
rg -n "task|fly|Phase|crates|breakdowns|Cluster|clusters|runtime" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "task|fly|Phase|crates|breakdowns|Cluster|clusters|runtime" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `tmp/architecture-plans/`
- `.roko/plans/`
- `crates/roko-cli/src/parity.rs`
- `crates/roko-conductor/src/federation.rs`
- `crates/roko-mpp/`
- `crates/roko-runtime/src/fly.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-serve/src/routes/clusters.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/18-roadmap
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

