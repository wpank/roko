# Dashboard PRD Plan: Theses And Principles

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
**Generated:** 2026-04-25
**Source hash:** `8ee7dd9f88913b567e800c69deca80f05bcd3ebf6631fda6035597f6d79220ea`
**Section tasks:** 15
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
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-02-S001 | 1 | 02 — Theses and principles | [ ] | 9.8 |
| DASH-02-S002 | 7 | Why this document matters | [ ] | 9.8 |
| DASH-02-S003 | 19 | Thesis 1: Scaffold beats model | [ ] | 9.8 |
| DASH-02-S004 | 41 | Thesis 2: Collective beats individual | [ ] | 9.8 |
| DASH-02-S005 | 63 | Thesis 3: Measurability is the foundation | [ ] | 9.8 |
| DASH-02-S006 | 87 | Thesis 4: Evals come from outside the LLM | [ ] | 9.8 |
| DASH-02-S007 | 113 | Thesis 5: Creating is co-equal with consuming | [ ] | 9.8 |
| DASH-02-S008 | 137 | Thesis 6: The system is composed of primitives | [ ] | 9.8 |
| DASH-02-S009 | 163 | Thesis 7: Real time is the default | [ ] | 9.8 |
| DASH-02-S010 | 187 | Thesis 8: Tools for tools | [ ] | 9.8 |
| DASH-02-S011 | 213 | Thesis 9: The aesthetic carries information | [ ] | 9.8 |
| DASH-02-S012 | 239 | Thesis 10: Everything has a lens | [ ] | 9.8 |
| DASH-02-S013 | 259 | Thesis 11: Ownership is typed and constrained | [ ] | 9.8 |
| DASH-02-S014 | 281 | Thesis 12: Users grow into the product | [ ] | 9.8 |
| DASH-02-S015 | 305 | Summary of theses | [ ] | 9.8 |

## Tasks

### DASH-02-S001 -- 02 — Theses and principles

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 02 — Theses and principles

*The load-bearing beliefs that every page of the Nunchi dashboard must honor.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `13`
- Section hash: `633e8c264c0cd0f37bf5bf69bbd10745c5cb48733bd430f17472790207ff8d1e`

**Normative requirements and implementation claims:**
- *The load-bearing beliefs that every page of the Nunchi dashboard must honor.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "principles|Theses|load|honor|every|beliefs|bearing" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "principles|Theses|load|honor|every|beliefs|bearing" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S002 -- Why this document matters

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:7` through `18`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Why this document matters

The specification set contains many individual decisions about pages, components, data flows, and visual patterns. Most of those decisions are judgment calls that could have gone several ways. A reader who understands the underlying theses can make consistent judgment calls when the specification does not directly cover a situation.

Without this document, a reader would have to reverse-engineer the logic of individual page decisions. With it, page decisions become applications of shared principles.

Every thesis below has three parts. First, the thesis itself — a claim about how the world works or how this product should work. Second, what follows from it for the dashboard — concrete implications. Third, what it rules out — things the dashboard should not do.

Read this document before any surface specification. When a surface specification conflicts with a thesis, the thesis wins and the surface specification should be revised.

---
````

**Explicit detail extraction from this section:**

- Section word count: `143`
- Section hash: `26fd5436d7c1bc6ff4929d1dc4c69c80eae59c35644d54102032f09d44a2ebdc`

**Normative requirements and implementation claims:**
- Every thesis below has three parts. First, the thesis itself — a claim about how the world works or how this product should work. Second, what follows from it for the dashboard — concrete implications. Third, what it rules out — things the dashboard should not do.
- Read this document before any surface specification. When a surface specification conflicts with a thesis, the thesis wins and the surface specification should be revised.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "document|specification|matters|Why|thesis|decisions|surface" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|specification|matters|Why|thesis|decisions|surface" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S003 -- Thesis 1: Scaffold beats model

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:19` through `40`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 1: Scaffold beats model

**The thesis.** Most of the measurable difference in agent capability comes from the scaffolding around the LLM, not from the LLM itself. A weaker model with better gates, better context, better memory, and better routing outperforms a stronger model with naive scaffolding. This has been demonstrated empirically — a well-designed harness can swing benchmark performance by 20+ percentage points on the same base model.

**What follows for the dashboard.**

The dashboard should treat scaffolding as the primary thing users design, configure, and iterate on. The choice of LLM is one parameter among many, not the central decision. The surfaces where users compose extensions, gates, context strategies, and knowledge sources should be as rich and expressive as the surfaces where they would pick a model.

Measurable comparisons between scaffolding choices should be easy to run and easy to view. A user should be able to ask "does adding this extension improve this metric?" and get an answer with statistical confidence, not a guess.

Users should see the cost-performance frontier for different scaffolding configurations, not just for different models. "Claude 4.6 with four gates and HDC retrieval" is a configuration; "Claude 4.6" alone is not.

**What this rules out.**

Model-centric framing. The dashboard should not default to asking "which model do you want?" when spinning up an agent. It should default to asking "what domain is this agent for?" and inferring a model routing policy from that.

Hero metrics that attribute performance to the model. If the dashboard shows a benchmark result, it should attribute the result to the full configuration, not just the model.

"Upgrade to a better model" as a primary call to action. The dashboard should not nudge users toward more expensive models as the path to better results. The path to better results is better scaffolding.

---
````

**Explicit detail extraction from this section:**

- Section word count: `302`
- Section hash: `9e57cf4df645029972befdb5a3c7667ede9e7887ad2b572046bc07af120ce78f`

**Normative requirements and implementation claims:**
- **The thesis.** Most of the measurable difference in agent capability comes from the scaffolding around the LLM, not from the LLM itself. A weaker model with better gates, better context, better memory, and better routing outperforms a stronger model with naive scaffolding. This has been demonstrated empirically — a well-designed harness can swing benchmark performance by 20+ percentage points on the same base model.
- **What follows for the dashboard.**
- The dashboard should treat scaffolding as the primary thing users design, configure, and iterate on. The choice of LLM is one parameter among many, not the central decision. The surfaces where users compose extensions, gates, context strategies, and knowledge sources should be as rich and expressive as the surfaces where they would pick a model.
- Measurable comparisons between scaffolding choices should be easy to run and easy to view. A user should be able to ask "does adding this extension improve this metric?" and get an answer with statistical confidence, not a guess.
- Users should see the cost-performance frontier for different scaffolding configurations, not just for different models. "Claude 4.6 with four gates and HDC retrieval" is a configuration; "Claude 4.6" alone is not.
- **What this rules out.**
- Model-centric framing. The dashboard should not default to asking "which model do you want?" when spinning up an agent. It should default to asking "what domain is this agent for?" and inferring a model routing policy from that.
- Hero metrics that attribute performance to the model. If the dashboard shows a benchmark result, it should attribute the result to the full configuration, not just the model.
- "Upgrade to a better model" as a primary call to action. The dashboard should not nudge users toward more expensive models as the path to better results. The path to better results is better scaffolding.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "model|Scaffold|better|scaffolding|Thesis|user|beats" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "model|Scaffold|better|scaffolding|Thesis|user|beats" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S004 -- Thesis 2: Collective beats individual

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:41` through `62`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 2: Collective beats individual

**The thesis.** A fleet of agents that share knowledge outperforms an isolated agent of the same capability level. The thousandth agent inherits from the first 999. The more users and agents participating in the collective knowledge layer, the faster all of them improve. This is a network effect, and it is compounding.

**What follows for the dashboard.**

The dashboard should make collective participation visible and valuable. Users should see, clearly, what they gain by participating — what knowledge they can draw on, how often they draw on it, how often their knowledge is drawn on by others.

Knowledge surfaces should be among the most prominent in the dashboard, not hidden under a secondary section. An agent's productivity is a function of its access to collective knowledge.

Cross-user coordination should be a first-class feature. A user should be able to form a group with other users' agents, join a collective arena, or subscribe to another user's knowledge stream. Coordination is not an advanced feature; it is the default mode of operation.

**What this rules out.**

Silo-first design. The dashboard should not default to treating each user's agents as isolated. The default should be participating in the collective; isolation is an explicit choice.

Paywalled knowledge as a primary monetization. Knowledge has value because it is shared; restricting it defeats the purpose. Some knowledge may be private, and some may be marketed, but the default posture is open circulation with provenance and reputation-weighted quality.

Measures of individual agent performance that ignore the collective context. An agent's measured performance in isolation is not the agent's real performance. Real performance includes the collective it has access to.

---
````

**Explicit detail extraction from this section:**

- Section word count: `280`
- Section hash: `f0f8cef25408da41d8fe75bd0813492de3b92eca20f92fc718869a173a47f417`

**Normative requirements and implementation claims:**
- **The thesis.** A fleet of agents that share knowledge outperforms an isolated agent of the same capability level. The thousandth agent inherits from the first 999. The more users and agents participating in the collective knowledge layer, the faster all of them improve. This is a network effect, and it is compounding.
- **What follows for the dashboard.**
- The dashboard should make collective participation visible and valuable. Users should see, clearly, what they gain by participating — what knowledge they can draw on, how often they draw on it, how often their knowledge is drawn on by others.
- Knowledge surfaces should be among the most prominent in the dashboard, not hidden under a secondary section. An agent's productivity is a function of its access to collective knowledge.
- Cross-user coordination should be a first-class feature. A user should be able to form a group with other users' agents, join a collective arena, or subscribe to another user's knowledge stream. Coordination is not an advanced feature; it is the default mode of operation.
- **What this rules out.**
- Silo-first design. The dashboard should not default to treating each user's agents as isolated. The default should be participating in the collective; isolation is an explicit choice.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "Collective|knowledge|user|individual|form|Thesis|beats" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Collective|knowledge|user|individual|form|Thesis|beats" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S005 -- Thesis 3: Measurability is the foundation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:63` through `86`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 3: Measurability is the foundation

**The thesis.** Everything that matters can be measured, and everything that is measured can be optimized. If a property of the system cannot be measured, it cannot be improved through the cybernetic feedback loops that make the system better over time. Conversely, anything measurable is a potential optimization target, a potential arena, a potential bounty, a potential training signal.

**What follows for the dashboard.**

Every meaningful state in the dashboard should expose a measurable quantity. Not as a footnote — as a first-class element of the UI. "How good is this agent at this task?" should always have a numerical answer, with uncertainty, attribution, and comparability.

Every surface should be interrogable as an eval target. Given any view, the user should be able to ask "what's the measurement behind this?" and get a clear answer. Given any number, the user should be able to ask "how is this computed?" and see the full calculation.

Measurements should be first-class objects with their own UI. There should be a page where users can author, browse, compose, and challenge measurements. This is what `18-system-surfaces.md` calls the Measurements page and the Evals page.

The dashboard itself should be a measurable artifact. Page load times, interaction latency, data freshness, user engagement — all of these should be instrumented and visible to the product team, and the principles should apply recursively.

**What this rules out.**

Vague claims about agent quality. The dashboard should not say "this agent is good at research" without the backing measurement. Either show the measurement or do not make the claim.

Hidden evaluation. The dashboard should not grade agents or knowledge or actions through criteria that are not inspectable. If a leaderboard ranks agents by "quality," the reader must be able to drill into "quality" and see its definition and its components.

LLM-as-judge for load-bearing scoring. See the next thesis.

---
````

**Explicit detail extraction from this section:**

- Section word count: `315`
- Section hash: `a55c1586a6bcb7e2b559a038bc6215884287901b5c23b875aa4bf1b9ff891ee3`

**Normative requirements and implementation claims:**
- **The thesis.** Everything that matters can be measured, and everything that is measured can be optimized. If a property of the system cannot be measured, it cannot be improved through the cybernetic feedback loops that make the system better over time. Conversely, anything measurable is a potential optimization target, a potential arena, a potential bounty, a potential training signal.
- **What follows for the dashboard.**
- Every meaningful state in the dashboard should expose a measurable quantity. Not as a footnote — as a first-class element of the UI. "How good is this agent at this task?" should always have a numerical answer, with uncertainty, attribution, and comparability.
- Every surface should be interrogable as an eval target. Given any view, the user should be able to ask "what's the measurement behind this?" and get a clear answer. Given any number, the user should be able to ask "how is this computed?" and see the full calculation.
- Measurements should be first-class objects with their own UI. There should be a page where users can author, browse, compose, and challenge measurements. This is what `18-system-surfaces.md` calls the Measurements page and the Evals page.
- The dashboard itself should be a measurable artifact. Page load times, interaction latency, data freshness, user engagement — all of these should be instrumented and visible to the product team, and the principles should apply recursively.
- **What this rules out.**
- Vague claims about agent quality. The dashboard should not say "this agent is good at research" without the backing measurement. Either show the measurement or do not make the claim.
- Hidden evaluation. The dashboard should not grade agents or knowledge or actions through criteria that are not inspectable. If a leaderboard ranks agents by "quality," the reader must be able to drill into "quality" and see its definition and its components.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "the|able|Thesis|measurement|foundation|Measurability|user" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|able|Thesis|measurement|foundation|Measurability|user" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S006 -- Thesis 4: Evals come from outside the LLM

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:87` through `112`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 4: Evals come from outside the LLM

**The thesis.** If the LLM grades its own work, the feedback loop collapses into confident self-endorsement. Meaningful measurement requires a ground truth external to the system being measured. In this product, external ground truth comes from: gate outcomes (compile, test, lint), arena results (did the agent win the chess game?), chain state (did the transaction settle at the expected price?), clearing results (did the clearing round resolve at the predicted ISFR?), oracle values (is the benchmark rate what the oracle says it is?), and user judgments (did the user accept the agent's output?).

This thesis is what makes meta-design possible. If every feedback signal is hallucinated, stacking layers of self-improvement just stacks layers of hallucination. If the base signals are grounded in external reality, then each layer of self-improvement inherits grounding from the layer below. Agents can improve agents because the improvement is measured against gates, not against other LLMs' opinions.

**What follows for the dashboard.**

Every eval surfaced in the dashboard must declare its ground truth source. No eval should be accepted without this declaration. If a user creates an eval, the authoring flow must require them to specify where the ground truth comes from.

LLM-based components inside evals are allowed but not primary. An eval might use an LLM to help interpret an outcome (e.g., "did this code output match the expected behavior?"), but the final scoring decision must come from outside the LLM — the code either compiled and tested or it did not.

When the dashboard shows a measurement, it should make the ground truth source visible in the UI, not just buried in documentation. A measurement card should include: what was measured, what it was measured against, and where the ground truth came from.

Reputation and reputation tiers should be computed from external-eval outcomes, not from LLM opinions about agent quality.

**What this rules out.**

"AI-graded" anything as a load-bearing measurement. The dashboard may display AI-grader opinions, but those are signals, not scores.

Evals that cannot articulate their ground truth. If someone publishes an eval without a clear ground truth source, the dashboard should flag it.

Reputation-from-peer-review as the only reputation source. Peer validation is useful but not sufficient. Reputation must be backed by external-eval outcomes.

---
````

**Explicit detail extraction from this section:**

- Section word count: `385`
- Section hash: `3a5e1d13179643d26901236bf0ddeff167cb849d1730b385737f9b16f4218503`

**Normative requirements and implementation claims:**
- **The thesis.** If the LLM grades its own work, the feedback loop collapses into confident self-endorsement. Meaningful measurement requires a ground truth external to the system being measured. In this product, external ground truth comes from: gate outcomes (compile, test, lint), arena results (did the agent win the chess game?), chain state (did the transaction settle at the expected price?), clearing results (did the clearing round resolve at the predicted ISFR?), oracle values (is the benchmark rate what the oracle says it is?), and user judgments (did the user accept the agent's output?).
- **What follows for the dashboard.**
- Every eval surfaced in the dashboard must declare its ground truth source. No eval should be accepted without this declaration. If a user creates an eval, the authoring flow must require them to specify where the ground truth comes from.
- LLM-based components inside evals are allowed but not primary. An eval might use an LLM to help interpret an outcome (e.g., "did this code output match the expected behavior?"), but the final scoring decision must come from outside the LLM — the code either compiled and tested or it did not.
- When the dashboard shows a measurement, it should make the ground truth source visible in the UI, not just buried in documentation. A measurement card should include: what was measured, what it was measured against, and where the ground truth came from.
- Reputation and reputation tiers should be computed from external-eval outcomes, not from LLM opinions about agent quality.
- **What this rules out.**
- "AI-graded" anything as a load-bearing measurement. The dashboard may display AI-grader opinions, but those are signals, not scores.
- Evals that cannot articulate their ground truth. If someone publishes an eval without a clear ground truth source, the dashboard should flag it.
- Reputation-from-peer-review as the only reputation source. Peer validation is useful but not sufficient. Reputation must be backed by external-eval outcomes.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "the|eval|come|round|LLM|ground|truth|Thesis" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|eval|come|round|LLM|ground|truth|Thesis" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S007 -- Thesis 5: Creating is co-equal with consuming

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:113` through `136`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 5: Creating is co-equal with consuming

**The thesis.** The dashboard is not a monitoring console. It is a workshop. Users come not only to watch their agents but to make new things: new agents, new extensions, new gates, new domains, new arenas, new evals, new meta-agents. The current dashboard is heavily biased toward observation; this specification pulls it toward creation.

**What follows for the dashboard.**

Authoring surfaces must be as polished as observation surfaces. The Agent Composer should be as well-designed as the Command Center. The Arena Constructor should be as considered as the Leaderboard.

Every first-class object in the system should have both a read surface (where it is observed) and a write surface (where it is created and edited). If users can read a knowledge entry, they can create one. If users can browse arenas, they can create one. If users can look at domain profiles, they can make one.

Authoring should feel like creative work, not form filling. Wizards with ten pages of required fields are a failure mode. Authoring surfaces should take inspiration from creative tools — digital audio workstations, 3D modeling software, code editors — where the interface itself invites exploration.

Templates, examples, and forks should be everywhere. Users should rarely start from a blank slate. They should start from a working thing and modify it.

**What this rules out.**

Observation-first design. The dashboard should not treat observation as primary and authoring as secondary. They are peers.

Single-user creation. Authoring surfaces should support collaboration — multiple users working on the same agent configuration, the same arena, the same domain profile. Version control concepts (branches, forks, merges) apply to these objects.

Hidden authoring. Authoring surfaces should not be hidden under settings menus or admin pages. They are first-class sidebar destinations.

---
````

**Explicit detail extraction from this section:**

- Section word count: `291`
- Section hash: `42ad94b07f47d01e06f378e69e53a31547cf3cb878f9a63f5ac3b329d1f49e5b`

**Normative requirements and implementation claims:**
- **The thesis.** The dashboard is not a monitoring console. It is a workshop. Users come not only to watch their agents but to make new things: new agents, new extensions, new gates, new domains, new arenas, new evals, new meta-agents. The current dashboard is heavily biased toward observation; this specification pulls it toward creation.
- **What follows for the dashboard.**
- Authoring surfaces must be as polished as observation surfaces. The Agent Composer should be as well-designed as the Command Center. The Arena Constructor should be as considered as the Leaderboard.
- Every first-class object in the system should have both a read surface (where it is observed) and a write surface (where it is created and edited). If users can read a knowledge entry, they can create one. If users can browse arenas, they can create one. If users can look at domain profiles, they can make one.
- Authoring should feel like creative work, not form filling. Wizards with ten pages of required fields are a failure mode. Authoring surfaces should take inspiration from creative tools — digital audio workstations, 3D modeling software, code editors — where the interface itself invites exploration.
- Templates, examples, and forks should be everywhere. Users should rarely start from a blank slate. They should start from a working thing and modify it.
- **What this rules out.**
- Observation-first design. The dashboard should not treat observation as primary and authoring as secondary. They are peers.
- Single-user creation. Authoring surfaces should support collaboration — multiple users working on the same agent configuration, the same arena, the same domain profile. Version control concepts (branches, forks, merges) apply to these objects.
- Hidden authoring. Authoring surfaces should not be hidden under settings menus or admin pages. They are first-class sidebar destinations.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "user|surface|Authoring|Users|Thesis|work|surfaces" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "user|surface|Authoring|Users|Thesis|work|surfaces" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S008 -- Thesis 6: The system is composed of primitives

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:137` through `162`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 6: The system is composed of primitives

**The thesis.** A small set of primitive objects combine in unlimited ways to produce the full functionality of the product. The primitives are: agents, extensions, gates, domains, knowledge entries, arenas, evals, pheromones, groups, and bounties. Every complex capability is expressible as a composition of primitives.

This is the DAW principle. A digital audio workstation gives musicians a small set of primitives — tracks, clips, instruments, effects, buses, automation — and lets them combine those primitives into arbitrarily complex music. The primitives are themselves well-designed, and their composition is what produces the creative space. Nunchi should work the same way.

**What follows for the dashboard.**

Every primitive has a consistent representation. Agents look like agents everywhere. Arenas look like arenas everywhere. This lets users learn the system once and apply it in many contexts.

Composition is explicit and visible. When a user looks at an agent, they should see its constituent parts (domain, extensions, gates, knowledge sources) and be able to inspect each. When a user looks at an arena, they should see its constituent parts (task source, gates, scoring, leaderboard) and be able to modify each.

Composition is saveable and shareable. A particular combination of primitives — "the Sonnet Coder with these gates and this knowledge subscription" — is itself a first-class object that can be named, saved, and published as a template.

Composition is recursive. Meta-agents compose regular agents with agent-creation tools. Meta-evals compose regular evals with eval-quality metrics. Groups compose agents with coordination protocols. The same composition machinery works at every level.

**What this rules out.**

Monolithic "modes" where the dashboard behaves completely differently for different use cases. The dashboard has lenses (see `05-lenses-and-perspectives.md`), but the primitives stay the same.

Special-case pages that introduce concepts not reducible to the primitives. If a page needs a new concept, the concept should either be added to the primitive set (a deliberate decision) or expressed as a composition.

Hidden power. There should not be capabilities accessible only through CLI or API that are not exposed in the dashboard. If the primitive supports an operation, the dashboard should expose that operation.

---
````

**Explicit detail extraction from this section:**

- Section word count: `356`
- Section hash: `4b42e778323b7a084f77d94ad7d382485d67d7d7d8d1513b9c51f1d32234d8e7`

**Normative requirements and implementation claims:**
- **The thesis.** A small set of primitive objects combine in unlimited ways to produce the full functionality of the product. The primitives are: agents, extensions, gates, domains, knowledge entries, arenas, evals, pheromones, groups, and bounties. Every complex capability is expressible as a composition of primitives.
- This is the DAW principle. A digital audio workstation gives musicians a small set of primitives — tracks, clips, instruments, effects, buses, automation — and lets them combine those primitives into arbitrarily complex music. The primitives are themselves well-designed, and their composition is what produces the creative space. Nunchi should work the same way.
- **What follows for the dashboard.**
- Composition is explicit and visible. When a user looks at an agent, they should see its constituent parts (domain, extensions, gates, knowledge sources) and be able to inspect each. When a user looks at an arena, they should see its constituent parts (task source, gates, scoring, leaderboard) and be able to modify each.
- **What this rules out.**
- Monolithic "modes" where the dashboard behaves completely differently for different use cases. The dashboard has lenses (see `05-lenses-and-perspectives.md`), but the primitives stay the same.
- Special-case pages that introduce concepts not reducible to the primitives. If a page needs a new concept, the concept should either be added to the primitive set (a deliberate decision) or expressed as a composition.
- Hidden power. There should not be capabilities accessible only through CLI or API that are not exposed in the dashboard. If the primitive supports an operation, the dashboard should expose that operation.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "The|primitive|primitives|compose|composition|Thesis|composed" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|primitive|primitives|compose|composition|Thesis|composed" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S009 -- Thesis 7: Real time is the default

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:163` through `186`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 7: Real time is the default

**The thesis.** The system generates events continuously. Heartbeats tick. Gates fire. Knowledge gets promoted. Pheromones decay. Chain blocks confirm. Arena attempts complete. The dashboard must reflect this continuous generation, not snapshot it.

**What follows for the dashboard.**

Every data source that streams should stream into the UI. The dashboard subscribes to event fabrics, listens to chain blocks, watches for pheromone deposits, and updates the UI as events arrive. No "refresh to see latest."

Real time is legible, not chaotic. A page with a thousand events per second should not be a wall of flashing text. It should convey rhythm, density, and recent change through aggregated visualization and animation. `10-realtime-and-motion.md` specifies this.

Real time degrades gracefully. When the connection is lost, the UI should show the last known state and clearly indicate it is stale. When the connection recovers, it should catch up from the last known sequence number, not start fresh.

Real time produces aesthetic opportunity. Streaming data enables the performance-reactive aesthetics described in `08-epistemic-aesthetics.md`. The UI can be alive because the underlying system is.

**What this rules out.**

Polling-based UI where streaming is available. Polling produces jitter, delay, and wasted bandwidth.

"Refresh" buttons as the primary update mechanism. Refresh buttons are a fallback, not a feature.

Static visualizations where the underlying data is dynamic. A knowledge graph should animate as entries are added, validated, and challenged.

---
````

**Explicit detail extraction from this section:**

- Section word count: `236`
- Section hash: `ec809fe1ae01e81b8b8d0c46453de0354e3f120582747ee3461216206412a0c6`

**Normative requirements and implementation claims:**
- **The thesis.** The system generates events continuously. Heartbeats tick. Gates fire. Knowledge gets promoted. Pheromones decay. Chain blocks confirm. Arena attempts complete. The dashboard must reflect this continuous generation, not snapshot it.
- **What follows for the dashboard.**
- Every data source that streams should stream into the UI. The dashboard subscribes to event fabrics, listens to chain blocks, watches for pheromone deposits, and updates the UI as events arrive. No "refresh to see latest."
- Real time is legible, not chaotic. A page with a thousand events per second should not be a wall of flashing text. It should convey rhythm, density, and recent change through aggregated visualization and animation. `10-realtime-and-motion.md` specifies this.
- Real time degrades gracefully. When the connection is lost, the UI should show the last known state and clearly indicate it is stale. When the connection recovers, it should catch up from the last known sequence number, not start fresh.
- **What this rules out.**
- "Refresh" buttons as the primary update mechanism. Refresh buttons are a fallback, not a feature.
- Static visualizations where the underlying data is dynamic. A knowledge graph should animate as entries are added, validated, and challenged.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "the|time|Real|Thesis|default|stream|fresh" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|time|Real|Thesis|default|stream|fresh" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S010 -- Thesis 8: Tools for tools

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:187` through `212`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 8: Tools for tools

**The thesis.** The most powerful surfaces are those that let users create surfaces. The most powerful agents are those that create agents. The most powerful evals are those that evaluate evals. Recursion is not a quirk of the system; it is the engine of its improvement.

A platform that provides only fixed surfaces can improve only as fast as its designers work. A platform that lets users create new surfaces can improve as fast as its users work. When both happen together, the platform compounds.

**What follows for the dashboard.**

The dashboard must include authoring surfaces for the things that authoring surfaces are made from. Users should be able to create a new page, a new visualization, a new workflow, and have it coexist with the built-in surfaces.

Meta-levels should not be esoteric. The meta pages (see `16-meta-surfaces.md`) are a first-class sidebar section, not hidden. The thesis is that meta work is normal work, and the UI should treat it as such.

Meta-objects inherit from their base objects. A meta-agent is still an agent, with all the same lifecycle hooks, measurable properties, and creative options. Users who understand base agents can understand meta-agents without learning a second system.

Recursion should be visible and traceable. When a meta-agent creates a regular agent, the dashboard should show the lineage. When a meta-eval retires a regular eval, the dashboard should explain why.

**What this rules out.**

A fixed set of pages with no authoring mechanism. If the dashboard cannot grow through use, it is brittle.

Hidden meta layers. Making meta-agents, meta-evals, and generators esoteric defeats their purpose.

Meta features that only power users can reach. The meta layer should be accessible to the same users who operate regular agents, with scaffolding that makes the leap navigable.

---
````

**Explicit detail extraction from this section:**

- Section word count: `306`
- Section hash: `98ed818cd07180a5254741c747f26df0289e362d8dd75ff315528bc03ae03f0c`

**Normative requirements and implementation claims:**
- **The thesis.** The most powerful surfaces are those that let users create surfaces. The most powerful agents are those that create agents. The most powerful evals are those that evaluate evals. Recursion is not a quirk of the system; it is the engine of its improvement.
- A platform that provides only fixed surfaces can improve only as fast as its designers work. A platform that lets users create new surfaces can improve as fast as its users work. When both happen together, the platform compounds.
- **What follows for the dashboard.**
- The dashboard must include authoring surfaces for the things that authoring surfaces are made from. Users should be able to create a new page, a new visualization, a new workflow, and have it coexist with the built-in surfaces.
- Meta-levels should not be esoteric. The meta pages (see `16-meta-surfaces.md`) are a first-class sidebar section, not hidden. The thesis is that meta work is normal work, and the UI should treat it as such.
- Recursion should be visible and traceable. When a meta-agent creates a regular agent, the dashboard should show the lineage. When a meta-eval retires a regular eval, the dashboard should explain why.
- **What this rules out.**
- A fixed set of pages with no authoring mechanism. If the dashboard cannot grow through use, it is brittle.
- Meta features that only power users can reach. The meta layer should be accessible to the same users who operate regular agents, with scaffolding that makes the leap navigable.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "Meta|for|surfaces|users|Thesis|eval|work" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Meta|for|surfaces|users|Thesis|eval|work" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S011 -- Thesis 9: The aesthetic carries information

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:213` through `238`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 9: The aesthetic carries information

**The thesis.** The dashboard's visual system is not decoration. It carries information. A user who glances at a knowledge entry should see, visually, how much to trust it. A user who glances at an agent should see, visually, what tier it is at. A user who glances at a measurement should see, visually, how fresh and validated it is.

This is the correspondence principle: every visual decision maps to something true about the system. Reputation becomes metal because reputation is forged. Confidence becomes saturation because uncertainty is literally muted. Decay becomes fade because decayed things fade. When visual form matches underlying property, the UI is legible before it is read.

**What follows for the dashboard.**

The visual system must be designed as a semantic system, not just an aesthetic one. Every reactive property maps to a specific measurable quantity. The mapping is documented in `08-epistemic-aesthetics.md` and consistent across the product.

Visual changes must be earned by state changes. A card does not become brighter because the designer thought it should be brighter; it becomes brighter because the underlying confidence rose.

Visual changes must remain legible under adverse conditions. Accessibility requirements do not compromise; the reactive system augments a baseline that already meets contrast and motion standards.

The aesthetic is a product feature. It is not a skin that can be swapped. Replacing the visual system would change the product's capabilities, because the visual system is doing informational work.

**What this rules out.**

Theming that breaks the correspondence. The user cannot select "dark red" as a theme and have the reputation tiers change material — the metal ladder is a semantic structure, not a color preference.

Performance-reactive effects tied to the wrong quantity. Aesthetic reward tied to raw outcomes (an agent's PnL, an attempt's win) creates slot machines. Aesthetic reward tied to epistemic states (how well-calibrated, how well-validated) avoids the trap.

Purely decorative motion or color. If a visual element does not correspond to something true, it should be removed, not kept for ornament.

---
````

**Explicit detail extraction from this section:**

- Section word count: `342`
- Section hash: `3ebcff34a008af33abfc17cea36e67f869246e7bc129788c606b9a3d12f83c66`

**Normative requirements and implementation claims:**
- **The thesis.** The dashboard's visual system is not decoration. It carries information. A user who glances at a knowledge entry should see, visually, how much to trust it. A user who glances at an agent should see, visually, what tier it is at. A user who glances at a measurement should see, visually, how fresh and validated it is.
- **What follows for the dashboard.**
- The visual system must be designed as a semantic system, not just an aesthetic one. Every reactive property maps to a specific measurable quantity. The mapping is documented in `08-epistemic-aesthetics.md` and consistent across the product.
- Visual changes must be earned by state changes. A card does not become brighter because the designer thought it should be brighter; it becomes brighter because the underlying confidence rose.
- Visual changes must remain legible under adverse conditions. Accessibility requirements do not compromise; the reactive system augments a baseline that already meets contrast and motion standards.
- **What this rules out.**
- Purely decorative motion or color. If a visual element does not correspond to something true, it should be removed, not kept for ornament.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "The|visual|aesthetic|form|information|carries|because|Thesis" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "The|visual|aesthetic|form|information|carries|because|Thesis" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S012 -- Thesis 10: Everything has a lens

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:239` through `258`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 10: Everything has a lens

**The thesis.** The same underlying data looks different from different vantage points. A leaderboard looks different when you're viewing your own agents, all agents in your team, all agents in an arena, or all agents on the network. The dashboard should support switching between these vantage points explicitly and cheaply.

**What follows for the dashboard.**

Lenses are a first-class UI concept. Documented in `05-lenses-and-perspectives.md`. The five standard lenses are global, fleet, agent, group, and chain. Surfaces declare which lenses they support, and users can switch between them without leaving the page.

Data should be queryable through the same API surface regardless of lens. The lens changes the filter, not the underlying query. This keeps the implementation simple.

Lenses compose with filters. A user viewing "arena leaderboard, fleet lens" is viewing the leaderboard filtered to their agents. The filter is composable with other filters (by domain, by time, by model).

**What this rules out.**

Lens-specific pages that duplicate work. "My Agents Leaderboard" and "Network Leaderboard" are not two pages; they are one page with a lens toggle.

Hidden lens context. The user must always know which lens they are viewing. Lens indicators are persistent.

---
````

**Explicit detail extraction from this section:**

- Section word count: `200`
- Section hash: `7e8954acc26ae9f1cab1b21e031dadbe9800080a54543cd3e4691ac882eaab64`

**Normative requirements and implementation claims:**
- **The thesis.** The same underlying data looks different from different vantage points. A leaderboard looks different when you're viewing your own agents, all agents in your team, all agents in an arena, or all agents on the network. The dashboard should support switching between these vantage points explicitly and cheaply.
- **What follows for the dashboard.**
- Lenses are a first-class UI concept. Documented in `05-lenses-and-perspectives.md`. The five standard lenses are global, fleet, agent, group, and chain. Surfaces declare which lenses they support, and users can switch between them without leaving the page.
- Data should be queryable through the same API surface regardless of lens. The lens changes the filter, not the underlying query. This keeps the implementation simple.
- **What this rules out.**
- Hidden lens context. The user must always know which lens they are viewing. Lens indicators are persistent.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "lens|Thesis|leaderboard|has|filter|Lenses|Everything" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "lens|Thesis|leaderboard|has|filter|Lenses|Everything" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S013 -- Thesis 11: Ownership is typed and constrained

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:259` through `280`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 11: Ownership is typed and constrained

**The thesis.** Ownership is not just "whose agent is this?" It is a typed relationship with specific permissions. An owner can configure, pause, and terminate their agents. They cannot exceed the caveats their delegation allows. Other users can observe public state of any agent but cannot modify it. Some actions (e.g., forking a public template) produce new ownership.

**What follows for the dashboard.**

The dashboard clearly displays ownership. Agents, knowledge entries, templates, arenas, and evals all indicate who owns them. Ownership transfers (rare but possible) are logged and visible.

Modification is gated by ownership. If a user does not own an object, the modification UI does not appear. If they partially own (e.g., member of a group with shared control), the UI reflects the partial permission.

Delegation caveats are inspectable and editable. Users can see what their agents are authorized to do. They can tighten caveats at will. They can broaden caveats subject to the system's safety policies.

Public forking is encouraged. If an object is public, other users can fork it into their own ownership. Forking preserves lineage.

**What this rules out.**

Ambiguity about who can change what. Every modification affordance must have a clear ownership precondition.

Unclear caveats. Users should not have to read contracts to know what their agents are authorized to do.

---
````

**Explicit detail extraction from this section:**

- Section word count: `219`
- Section hash: `e9d038633d2dabe77e34b40af17db81bb0b4c83e9d734001669eac8fe9e8704c`

**Normative requirements and implementation claims:**
- **The thesis.** Ownership is not just "whose agent is this?" It is a typed relationship with specific permissions. An owner can configure, pause, and terminate their agents. They cannot exceed the caveats their delegation allows. Other users can observe public state of any agent but cannot modify it. Some actions (e.g., forking a public template) produce new ownership.
- **What follows for the dashboard.**
- The dashboard clearly displays ownership. Agents, knowledge entries, templates, arenas, and evals all indicate who owns them. Ownership transfers (rare but possible) are logged and visible.
- **What this rules out.**
- Ambiguity about who can change what. Every modification affordance must have a clear ownership precondition.
- Unclear caveats. Users should not have to read contracts to know what their agents are authorized to do.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "owner|Ownership|typed|Thesis|user|constrained|caveats" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "owner|Ownership|typed|Thesis|user|constrained|caveats" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S014 -- Thesis 12: Users grow into the product

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:281` through `304`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Thesis 12: Users grow into the product

**The thesis.** A new user should be productive on day one. A six-month user should have access to power that the new user does not yet need. The dashboard should not dump all of its surface area onto new users, but it should reveal power gradually as users engage with more of the system.

**What follows for the dashboard.**

Default configurations are good enough to start. A user who creates an agent by clicking "new coding agent" should get a working agent, not a blank slate.

Progressive disclosure reveals power. Advanced configuration lives behind a toggle, not behind a different page. Users who want to customize their model routing, their context budget, or their gate thresholds can, but they are not required to.

Meta surfaces are reachable but not primary. A new user does not need to see the Meta-Agent Studio on their first day. A sixth-month user should know where it is and how to get there.

The dashboard has a notion of a user's level of engagement, but does not make it a profile attribute. Progressive disclosure is based on what the user does, not on an assigned level.

**What this rules out.**

Separate "basic" and "advanced" dashboards. One dashboard, with layers.

Hidden-by-default power features that require documentation to find. Discoverability is critical. Hidden is not the same as gated.

Mandatory "advanced mode" for normal operations. If running a second agent requires switching to advanced mode, the default mode is too thin.

---
````

**Explicit detail extraction from this section:**

- Section word count: `250`
- Section hash: `d08cffff86d4aa65875c14422ad0919badba787adce5cf9d12933bffa050d5e3`

**Normative requirements and implementation claims:**
- **The thesis.** A new user should be productive on day one. A six-month user should have access to power that the new user does not yet need. The dashboard should not dump all of its surface area onto new users, but it should reveal power gradually as users engage with more of the system.
- **What follows for the dashboard.**
- Default configurations are good enough to start. A user who creates an agent by clicking "new coding agent" should get a working agent, not a blank slate.
- Progressive disclosure reveals power. Advanced configuration lives behind a toggle, not behind a different page. Users who want to customize their model routing, their context budget, or their gate thresholds can, but they are not required to.
- Meta surfaces are reachable but not primary. A new user does not need to see the Meta-Agent Studio on their first day. A sixth-month user should know where it is and how to get there.
- The dashboard has a notion of a user's level of engagement, but does not make it a profile attribute. Progressive disclosure is based on what the user does, not on an assigned level.
- **What this rules out.**
- Separate "basic" and "advanced" dashboards. One dashboard, with layers.
- Mandatory "advanced mode" for normal operations. If running a second agent requires switching to advanced mode, the default mode is too thin.
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "the|user|Users|product|Thesis|grow|power|mode" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|user|Users|product|Thesis|grow|power|mode" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

### DASH-02-S015 -- Summary of theses

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md:305` through `322`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Summary of theses

| # | Thesis | Memorable handle |
|---|---|---|
| 1 | Scaffold beats model | Scaffolding is the product. |
| 2 | Collective beats individual | The network is the engine. |
| 3 | Measurability is the foundation | If it matters, it's measured. |
| 4 | Evals come from outside the LLM | The loop must close on reality. |
| 5 | Creating is co-equal with consuming | The dashboard is a workshop. |
| 6 | The system is composed of primitives | Few primitives, infinite compositions. |
| 7 | Real time is the default | The system is alive; the UI shows it. |
| 8 | Tools for tools | Recursion is the engine of improvement. |
| 9 | The aesthetic carries information | Form corresponds to truth. |
| 10 | Everything has a lens | Vantage point is a first-class concept. |
| 11 | Ownership is typed and constrained | Permissions are explicit. |
| 12 | Users grow into the product | Power reveals through use. |

When a surface specification contradicts one of these, the thesis wins.
````

**Explicit detail extraction from this section:**

- Section word count: `142`
- Section hash: `266a1448ab5225b855e0c1c037c5c5c3707ac822e80d3d196ad38100ec91c481`

**Normative requirements and implementation claims:**
- | # | Thesis | Memorable handle | |---|---|---| | 1 | Scaffold beats model | Scaffolding is the product. | | 2 | Collective beats individual | The network is the engine. | | 3 | Measurability is the foundation | If it matters, it's measured. | | 4 | Evals come from outside the LLM | The loop must close on reality. | | 5 | Creating is co-equal with consuming | The dashboard is a workshop. | | 6 | The system is composed of primitives | Few primitives, infinite compositions. | | 7 | Real time is the default | The system is alive; the UI shows it. | | 8 | Tools for tools | Recursion is the engine of improvement. | | 9 | The aesthetic carries information | Form corresponds to truth. | | 10 | Everything has a lens | Vantage point is a first-class concept. | | 11 | Ownership is typed and constrained | Permissions are explicit. | | 12 | Users grow into the product | Power reveals through use. |

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
| # | Thesis | Memorable handle |
|---|---|---|
| 1 | Scaffold beats model | Scaffolding is the product. |
| 2 | Collective beats individual | The network is the engine. |
| 3 | Measurability is the foundation | If it matters, it's measured. |
| 4 | Evals come from outside the LLM | The loop must close on reality. |
| 5 | Creating is co-equal with consuming | The dashboard is a workshop. |
| 6 | The system is composed of primitives | Few primitives, infinite compositions. |
| 7 | Real time is the default | The system is alive; the UI shows it. |
| 8 | Tools for tools | Recursion is the engine of improvement. |
| 9 | The aesthetic carries information | Form corresponds to truth. |
| 10 | Everything has a lens | Vantage point is a first-class concept. |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/02-theses-and-principles.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
rg -n "theses|Summary|product|primitives|engine|beats|Tools" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "theses|Summary|product|primitives|engine|beats|Tools" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-core/src/score.rs`
- `crates/roko-learn/src/`
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
./target/debug/roko parity check --strict --area dashboard-prd/02-theses-and-principles
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

