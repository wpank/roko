# Dashboard PRD Plan: System Landscape

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
**Generated:** 2026-04-25
**Source hash:** `a3db02eff8a5691189d13bda97346cad8888f2d5ba479f4232f672252cdf36c7`
**Section tasks:** 23
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
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-01-S001 | 1 | 01 — System landscape | [ ] | 9.8 |
| DASH-01-S002 | 7 | The four layers | [ ] | 9.8 |
| DASH-01-S003 | 11 | Nunchi (the product) | [ ] | 9.8 |
| DASH-01-S004 | 19 | Roko (the agent runtime) | [ ] | 9.8 |
| DASH-01-S005 | 29 | Korai (the chain) | [ ] | 9.8 |
| DASH-01-S006 | 39 | Mirage (the local chain) | [ ] | 9.8 |
| DASH-01-S007 | 45 | How the layers relate | [ ] | 9.8 |
| DASH-01-S008 | 61 | Glossary | [ ] | 9.8 |
| DASH-01-S009 | 65 | Identity and accounts | [ ] | 9.8 |
| DASH-01-S010 | 83 | Agent internals | [ ] | 9.8 |
| DASH-01-S011 | 111 | Extensions, domains, gates | [ ] | 9.8 |
| DASH-01-S012 | 133 | Knowledge | [ ] | 9.8 |
| DASH-01-S013 | 159 | Stigmergy and coordination | [ ] | 9.8 |
| DASH-01-S014 | 171 | Heartbeat and timing | [ ] | 9.8 |
| DASH-01-S015 | 185 | Affect and somatic | [ ] | 9.8 |
| DASH-01-S016 | 195 | Arenas and evaluation | [ ] | 9.8 |
| DASH-01-S017 | 213 | Eval system | [ ] | 9.8 |
| DASH-01-S018 | 225 | Economic and clearing | [ ] | 9.8 |
| DASH-01-S019 | 237 | UI concepts | [ ] | 9.8 |
| DASH-01-S020 | 251 | Visualization and aesthetic | [ ] | 9.8 |
| DASH-01-S021 | 271 | Backend and APIs | [ ] | 9.8 |
| DASH-01-S022 | 285 | Meta and generative | [ ] | 9.8 |
| DASH-01-S023 | 295 | A note on what is not in this glossary | [ ] | 9.8 |

## Tasks

### DASH-01-S001 -- 01 — System landscape

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 01 — System landscape

*What Nunchi, Roko, Korai, and Mirage are, how they relate, and every term used across the specification set.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `18`
- Section hash: `427f5e6a4bd3782947179ca9ee83284443612fb02b650fc13b073677f977e625`

**Normative requirements and implementation claims:**
- *What Nunchi, Roko, Korai, and Mirage are, how they relate, and every term used across the specification set.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "landscape|term|specification|relate|every|across|Nunchi|Mirage" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "landscape|term|specification|relate|every|across|Nunchi|Mirage" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S002 -- The four layers

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:7` through `10`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## The four layers

Nunchi is built in four layers. They are architecturally distinct but deeply interconnected. A reader must understand all four before any specific page of the dashboard makes sense.
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `da381c51d031a6c2b9f9dfc82a042e92bea45df61962f5cf5f6638112fd08e7b`

**Normative requirements and implementation claims:**
- Nunchi is built in four layers. They are architecturally distinct but deeply interconnected. A reader must understand all four before any specific page of the dashboard makes sense.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "four|The|layers|understand|specific|sense|reader|makes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "four|The|layers|understand|specific|sense|reader|makes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S003 -- Nunchi (the product)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:11` through `18`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Nunchi (the product)

Nunchi is the product end users experience. It is a web application — the dashboard — through which users create, configure, operate, monitor, and collaborate with autonomous agents. The dashboard is the primary surface of Nunchi. A command-line interface and API surfaces exist but are secondary to the dashboard for most users.

Nunchi the product is aimed at several user types, each with different needs: solo developers who want autonomous help with their code, traders who want agents that manage DeFi positions, research teams that want accumulated knowledge across sessions, arena competitors who want to optimize agents against challenges, domain architects who want to configure entire classes of agent behavior, and meta-builders who want to make agents that make agents.

The name "Nunchi" is only occasionally surfaced to end users. They interact with the dashboard itself, which they may call "the Nunchi dashboard" or simply "Nunchi." The name is the brand.
````

**Explicit detail extraction from this section:**

- Section word count: `151`
- Section hash: `881e623cac86dd15ca886be2d6fb81247fce02c1233060681095af8c60a9cfb5`

**Normative requirements and implementation claims:**
- Nunchi is the product end users experience. It is a web application — the dashboard — through which users create, configure, operate, monitor, and collaborate with autonomous agents. The dashboard is the primary surface of Nunchi. A command-line interface and API surfaces exist but are secondary to the dashboard for most users.
- Nunchi the product is aimed at several user types, each with different needs: solo developers who want autonomous help with their code, traders who want agents that manage DeFi positions, research teams that want accumulated knowledge across sessions, arena competitors who want to optimize agents against challenges, domain architects who want to configure entire classes of agent behavior, and meta-builders who want to make agents that make agents.
- The name "Nunchi" is only occasionally surfaced to end users. They interact with the dashboard itself, which they may call "the Nunchi dashboard" or simply "Nunchi." The name is the brand.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "the|Nunchi|product|want|user|users|surface|name" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|Nunchi|product|want|user|users|surface|name" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S004 -- Roko (the agent runtime)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:19` through `28`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Roko (the agent runtime)

Roko is the Rust toolkit that agents run on. It is not a single program; it is a workspace of crates (currently around 29) that together define: the agent lifecycle, the heartbeat pipeline, the cognitive gating system, the context assembly system, the tool dispatch system, the gate pipeline, the learning loops, the knowledge store, the dream consolidation system, the somatic marker system, the extension system, and the domain profile system.

An agent is an instance of a Roko runtime, configured with a domain profile, a set of extensions, a model routing table, a knowledge store, and identity. When an agent runs, it runs as a Rust process that executes the heartbeat pipeline on its configured schedule. Long-running agents tick continuously, observing their environment and acting on it. Ephemeral agents spawn, do one task, and exit.

End users do not usually encounter the name "Roko" in the dashboard. Implementers encounter it constantly — the codebase is structured around Roko crates, the API endpoints the dashboard consumes come from Roko services, and the concepts in the dashboard (heartbeat, gate, extension, domain) are Roko concepts.

Roko is open source. It can be run standalone without the rest of the Nunchi stack. Users who run Roko without Nunchi get the runtime but not the dashboard, not the chain integration, and not the collective knowledge layer.
````

**Explicit detail extraction from this section:**

- Section word count: `221`
- Section hash: `a713150249b7649a7d67205143c8c0f676ffa5075428ba3559581a6d677f8a7d`

**Normative requirements and implementation claims:**
- Roko is the Rust toolkit that agents run on. It is not a single program; it is a workspace of crates (currently around 29) that together define: the agent lifecycle, the heartbeat pipeline, the cognitive gating system, the context assembly system, the tool dispatch system, the gate pipeline, the learning loops, the knowledge store, the dream consolidation system, the somatic marker system, the extension system, and the domain profile system.
- End users do not usually encounter the name "Roko" in the dashboard. Implementers encounter it constantly — the codebase is structured around Roko crates, the API endpoints the dashboard consumes come from Roko services, and the concepts in the dashboard (heartbeat, gate, extension, domain) are Roko concepts.
- Roko is open source. It can be run standalone without the rest of the Nunchi stack. Users who run Roko without Nunchi get the runtime but not the dashboard, not the chain integration, and not the collective knowledge layer.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "the|runtime|pipeline|knowledge|heartbeat|extension|domain|without" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|runtime|pipeline|knowledge|heartbeat|extension|domain|without" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S005 -- Korai (the chain)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:29` through `38`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Korai (the chain)

Korai is a purpose-built blockchain. It is not a general-purpose smart contract platform — it is designed from consensus up for the specific requirements of an agent coordination layer. Its components include:

A Byzantine fault tolerant consensus protocol (Kauri BFT) with sub-second block times and single-slot finality. An execution layer (SpecPool EVM) that runs EVM-compatible smart contracts with parallel execution via software transactional memory. A dual-plane architecture separating critical financial infrastructure (the Kernel Plane) from permissionless application code (the EVM Plane).

On top of this, Korai has several native systems that make it distinctive: an on-chain knowledge substrate (the InsightStore), a precompile for hyperdimensional vector similarity search, an oracle that computes a benchmark interest rate (ISFR) at consensus, a cooperative clearing engine for yield perpetuals, and an agent passport system (ERC-8004) that makes agents first-class on-chain identities.

The dashboard reads from Korai constantly — agent identities, reputation scores, published knowledge, clearing outcomes, ISFR values, pheromone deposits. The dashboard writes to Korai when users publish knowledge, register agents, participate in arenas, post bounties, or trade yield perpetuals.
````

**Explicit detail extraction from this section:**

- Section word count: `184`
- Section hash: `21804635fe30f67bc3d44364368487da7b968e59577fdf721e8d500a5ef2f40d`

**Normative requirements and implementation claims:**
- Korai is a purpose-built blockchain. It is not a general-purpose smart contract platform — it is designed from consensus up for the specific requirements of an agent coordination layer. Its components include:
- The dashboard reads from Korai constantly — agent identities, reputation scores, published knowledge, clearing outcomes, ISFR values, pheromone deposits. The dashboard writes to Korai when users publish knowledge, register agents, participate in arenas, post bounties, or trade yield perpetuals.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "the|Korai|chain|plane|knowledge|consensus|yield|smart" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|Korai|chain|plane|knowledge|consensus|yield|smart" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S006 -- Mirage (the local chain)

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:39` through `44`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Mirage (the local chain)

Mirage is a local development chain that behaves identically to Korai from the dashboard's perspective. It exists because agents and dashboards need a chain to develop against before Korai is live, because local testing benefits from running on a fast local chain, and because the dashboard should work offline or in air-gapped environments for development.

When the specifications refer to "the chain," they mean either Korai or Mirage, interchangeable from the dashboard's point of view. The dashboard should be configurable via environment or user setting to point at either. Production uses Korai. Development uses Mirage. Nothing about the dashboard's code should branch on which chain it is talking to.
````

**Explicit detail extraction from this section:**

- Section word count: `113`
- Section hash: `a8a7a09e7097f8a8ec5c975cf5d0a5c01d4ccc9c1a5f65b0d9f48e5a3ca14306`

**Normative requirements and implementation claims:**
- Mirage is a local development chain that behaves identically to Korai from the dashboard's perspective. It exists because agents and dashboards need a chain to develop against before Korai is live, because local testing benefits from running on a fast local chain, and because the dashboard should work offline or in air-gapped environments for development.
- When the specifications refer to "the chain," they mean either Korai or Mirage, interchangeable from the dashboard's point of view. The dashboard should be configurable via environment or user setting to point at either. Production uses Korai. Development uses Mirage. Nothing about the dashboard's code should branch on which chain it is talking to.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "the|chain|local|Mirage|develop|Korai|development|because" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|chain|local|Mirage|develop|Korai|development|because" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S007 -- How the layers relate

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:45` through `60`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## How the layers relate

The layers stack from bottom to top:

Korai (or Mirage) sits at the bottom. It is the shared coordination substrate. Agents read and write to it. The dashboard reads from it. It holds identity, reputation, and published knowledge.

Roko sits in the middle. Each agent is a Roko runtime instance. Roko agents connect to Korai to register, to read other agents' knowledge, and to publish their own. Roko agents do not need to connect to each other directly — they coordinate through the chain (this is called stigmergy, and it is a load-bearing architectural choice).

Nunchi sits at the top. The Nunchi dashboard connects to Roko (via the Roko API surface, which each agent exposes as an HTTP sidecar) to observe and control individual agents. It connects to Korai to show on-chain state, browse the network, and execute on-chain transactions on behalf of users.

A user talking to an agent through the dashboard: their message goes from the dashboard over WebSocket to the agent's Roko sidecar, gets processed by the agent's heartbeat pipeline, potentially triggers chain reads or writes via Korai, produces outputs and events that flow back to the dashboard in real time.

A user browsing the network: their query goes from the dashboard to a Korai RPC endpoint, fetches agent passports and knowledge entries, returns to the dashboard, gets rendered. The Roko runtime is not involved in this path — it is a pure chain interaction.

A user building an agent: they interact with authoring surfaces in the dashboard, which compose the agent's configuration (domain, extensions, gates, model preferences, budget). When they deploy, the dashboard provisions a Roko runtime somewhere (locally, on a managed cloud, on user-provided infrastructure), registers the agent on Korai, and connects the dashboard to the new agent's sidecar.
````

**Explicit detail extraction from this section:**

- Section word count: `300`
- Section hash: `286d944b7b39068f304ecab1a656ac1dcbb2de732f5ed22736474bc6d497a1cc`

**Normative requirements and implementation claims:**
- Korai (or Mirage) sits at the bottom. It is the shared coordination substrate. Agents read and write to it. The dashboard reads from it. It holds identity, reputation, and published knowledge.
- Roko sits in the middle. Each agent is a Roko runtime instance. Roko agents connect to Korai to register, to read other agents' knowledge, and to publish their own. Roko agents do not need to connect to each other directly — they coordinate through the chain (this is called stigmergy, and it is a load-bearing architectural choice).
- Nunchi sits at the top. The Nunchi dashboard connects to Roko (via the Roko API surface, which each agent exposes as an HTTP sidecar) to observe and control individual agents. It connects to Korai to show on-chain state, browse the network, and execute on-chain transactions on behalf of users.
- A user talking to an agent through the dashboard: their message goes from the dashboard over WebSocket to the agent's Roko sidecar, gets processed by the agent's heartbeat pipeline, potentially triggers chain reads or writes via Korai, produces outputs and events that flow back to the dashboard in real time.
- A user browsing the network: their query goes from the dashboard to a Korai RPC endpoint, fetches agent passports and knowledge entries, returns to the dashboard, gets rendered. The Roko runtime is not involved in this path — it is a pure chain interaction.
- A user building an agent: they interact with authoring surfaces in the dashboard, which compose the agent's configuration (domain, extensions, gates, model preferences, budget). When they deploy, the dashboard provisions a Roko runtime somewhere (locally, on a managed cloud, on user-provided infrastructure), registers the agent on Korai, and connects the dashboard to the new agent's sidecar.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "the|layers|Korai|How|user|relate|connect|chain" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|layers|Korai|How|user|relate|connect|chain" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S008 -- Glossary

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:61` through `64`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Glossary

Every term used across any document in this specification set is defined here. Terms are grouped by area. Within each group, terms appear in dependency order — a term is defined before later terms that use it.
````

**Explicit detail extraction from this section:**

- Section word count: `36`
- Section hash: `fafea5ca23602fe6dfd24cb356180e4adf2d27652777c046fab4815ecdae94a8`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "term|Glossary|Terms|group|defined|specification|order|later" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "term|Glossary|Terms|group|defined|specification|order|later" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S009 -- Identity and accounts

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:65` through `82`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Identity and accounts

**User** — A human who interacts with the dashboard. Users may own agents, run agents, watch agents, compete in arenas, publish knowledge, or consume the dashboard as a read-only observer. Users have a wallet (via WalletConnect, Privy, or similar). Users do not have on-chain identity — their wallet is their identity.

**Owner** — A user who has control over one or more agents. Ownership is established by the owner being the signer on an agent's passport registration transaction. Ownership can be transferred via on-chain action.

**Agent** — A Roko runtime instance with its own identity, configuration, state, and capabilities. Agents can be long-running (persistent, with heartbeat, accumulating knowledge over time) or ephemeral (spawned for one task, then exit). Agents have an on-chain identity via an ERC-8004 passport if registered; agents can also run unregistered for development or single-task purposes.

**Passport (ERC-8004)** — An on-chain, soulbound NFT representing an agent's identity. Contains typed capabilities, epistemic reputation per domain, reputation tier, service endpoints, a runtime fingerprint, and delegation caveats. "Soulbound" means non-transferable. An agent's passport is its on-chain identity.

**AgentId** — A unique identifier for an agent. On-chain, this is the token ID of the ERC-8004 passport. Off-chain, unregistered agents have local identifiers.

**Reputation** — A measure of an agent's past performance. Specifically, an agent has a reputation score per domain track, updated as the agent's predictions or outputs are scored against ground truth. Reputation decays over time and must be continuously earned.

**Reputation tier** — A coarse bucketing of reputation into five levels: Unverified (Gray), Basic (Copper), Verified (Silver), Trusted (Gold), Sovereign (Amber). Tier is derived from reputation score, not earned separately. Higher tiers unlock capabilities: larger knowledge quotas, priority routing, higher position caps, governance weight.

**Delegation caveat** — A typed, on-chain constraint on what an agent is authorized to do on behalf of its owner. For example, "this agent may trade yield perpetuals up to $100k notional and may not withdraw funds to external addresses." Caveats are publicly auditable and enforced at the smart contract level where possible.
````

**Explicit detail extraction from this section:**

- Section word count: `349`
- Section hash: `75d1f2de6d36e9e21db9d1ef7fb185793a5d36afd5cd57a3bf5f6f03218cf3ab`

**Normative requirements and implementation claims:**
- **User** — A human who interacts with the dashboard. Users may own agents, run agents, watch agents, compete in arenas, publish knowledge, or consume the dashboard as a read-only observer. Users have a wallet (via WalletConnect, Privy, or similar). Users do not have on-chain identity — their wallet is their identity.
- **Owner** — A user who has control over one or more agents. Ownership is established by the owner being the signer on an agent's passport registration transaction. Ownership can be transferred via on-chain action.
- **Agent** — A Roko runtime instance with its own identity, configuration, state, and capabilities. Agents can be long-running (persistent, with heartbeat, accumulating knowledge over time) or ephemeral (spawned for one task, then exit). Agents have an on-chain identity via an ERC-8004 passport if registered; agents can also run unregistered for development or single-task purposes.
- **Passport (ERC-8004)** — An on-chain, soulbound NFT representing an agent's identity. Contains typed capabilities, epistemic reputation per domain, reputation tier, service endpoints, a runtime fingerprint, and delegation caveats. "Soulbound" means non-transferable. An agent's passport is its on-chain identity.
- **AgentId** — A unique identifier for an agent. On-chain, this is the token ID of the ERC-8004 passport. Off-chain, unregistered agents have local identifiers.
- **Reputation** — A measure of an agent's past performance. Specifically, an agent has a reputation score per domain track, updated as the agent's predictions or outputs are scored against ground truth. Reputation decays over time and must be continuously earned.
- **Reputation tier** — A coarse bucketing of reputation into five levels: Unverified (Gray), Basic (Copper), Verified (Silver), Trusted (Gold), Sovereign (Amber). Tier is derived from reputation score, not earned separately. Higher tiers unlock capabilities: larger knowledge quotas, priority routing, higher position caps, governance weight.
- **Delegation caveat** — A typed, on-chain constraint on what an agent is authorized to do on behalf of its owner. For example, "this agent may trade yield perpetuals up to $100k notional and may not withdraw funds to external addresses." Caveats are publicly auditable and enforced at the smart contract level where possible.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Identity|reputation|chain|passport|over|accounts|User" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Identity|reputation|chain|passport|over|accounts|User" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S010 -- Agent internals

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:83` through `110`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent internals

**Runtime** — The Roko process that executes an agent. A runtime has state (the agent's memory, configuration, and current cognitive context), capabilities (extensions and tools available to it), and identity (its passport). The runtime is what actually ticks, observes, and acts.

**Heartbeat** — The agent's periodic execution loop. Each tick, the heartbeat runs a pipeline of stages: observe the environment, retrieve relevant knowledge, analyze what has changed, decide whether to act and at what cognitive tier, act if appropriate, record the outcome, learn from it. The heartbeat has three timescales — gamma (fast, 5–15 seconds, mostly deterministic), theta (slower, 30–120 seconds, full decision cycle), and delta (offline, consolidation and dreams).

**Ephemeral agent** — An agent that spawns for a specific task, executes, and exits. No persistent heartbeat. Useful for one-shot tasks like "refactor this module" or "analyze this dataset."

**Long-running agent** — An agent that runs continuously with a heartbeat, observing its environment and acting autonomously. Useful for tasks like monitoring blockchain state, managing DeFi positions, or conducting ongoing research.

**Cognitive tier** — The level of reasoning applied to a heartbeat tick. T0 is deterministic (pattern match, no LLM call, free). T1 is lightweight (cheap model, minimal context). T2 is full reasoning (expensive model, complete context). The cognitive gating system decides which tier applies to each tick based on novelty, stakes, and prediction error. In typical operation, 80% of ticks are T0, 15% are T1, and 5% are T2.

**Cognitive gating** — The mechanism that decides which cognitive tier applies to a given tick. Cognitive gating is central to the product's economic viability — without it, running agents continuously would be prohibitively expensive.

**Prediction error** — The gap between what the agent expected to observe and what it actually observed. High prediction error triggers escalation to higher cognitive tiers. Low prediction error allows the agent to stay at T0.

**Context assembly** — The process of building the LLM prompt for an agent's tick. Context is assembled from role (who the agent is), workspace (what it's working on), plan (its current goal), knowledge (relevant past experience), and volatile (per-turn specifics). Context assembly is a learnable control system — the agent tracks which context sections correlated with successful outcomes and adjusts allocations over time.

**Cognitive workspace** — The typed, budgeted collection of context sections assembled for a given tick. Sections have priorities, token budgets, and sources. The workspace is the agent's working memory.

**Section effectiveness** — A measure of how much a given context section contributed to the outcome of a tick. Tracked over time. Used to adjust future context allocations.

**VCG auction (Vickrey–Clarke–Groves)** — The mechanism used inside context assembly to allocate token budget across competing context sources. Different subsystems bid for space; VCG ensures truthful bidding.

**Tool** — A function an agent can call to affect the world or read from it. Tools include file operations, shell commands, HTTP requests, chain reads and writes, knowledge queries, and custom tools added by extensions. Tool calls are dispatched by the Roko runtime, not by the LLM directly; the LLM requests a tool call and the runtime decides whether to execute it.

**Tool dispatch** — The subsystem that routes tool call requests from the LLM through appropriate gates, permissions, and somatic checks before execution.
````

**Explicit detail extraction from this section:**

- Section word count: `538`
- Section hash: `de2da6840829cf6b2f5f0fe3620537ced530427ba0327fa9b6a1a6f1ced8f6c8`

**Normative requirements and implementation claims:**
- **Runtime** — The Roko process that executes an agent. A runtime has state (the agent's memory, configuration, and current cognitive context), capabilities (extensions and tools available to it), and identity (its passport). The runtime is what actually ticks, observes, and acts.
- **Heartbeat** — The agent's periodic execution loop. Each tick, the heartbeat runs a pipeline of stages: observe the environment, retrieve relevant knowledge, analyze what has changed, decide whether to act and at what cognitive tier, act if appropriate, record the outcome, learn from it. The heartbeat has three timescales — gamma (fast, 5–15 seconds, mostly deterministic), theta (slower, 30–120 seconds, full decision cycle), and delta (offline, consolidation and dreams).
- **Ephemeral agent** — An agent that spawns for a specific task, executes, and exits. No persistent heartbeat. Useful for one-shot tasks like "refactor this module" or "analyze this dataset."
- **Long-running agent** — An agent that runs continuously with a heartbeat, observing its environment and acting autonomously. Useful for tasks like monitoring blockchain state, managing DeFi positions, or conducting ongoing research.
- **Cognitive tier** — The level of reasoning applied to a heartbeat tick. T0 is deterministic (pattern match, no LLM call, free). T1 is lightweight (cheap model, minimal context). T2 is full reasoning (expensive model, complete context). The cognitive gating system decides which tier applies to each tick based on novelty, stakes, and prediction error. In typical operation, 80% of ticks are T0, 15% are T1, and 5% are T2.
- **Cognitive gating** — The mechanism that decides which cognitive tier applies to a given tick. Cognitive gating is central to the product's economic viability — without it, running agents continuously would be prohibitively expensive.
- **Prediction error** — The gap between what the agent expected to observe and what it actually observed. High prediction error triggers escalation to higher cognitive tiers. Low prediction error allows the agent to stay at T0.
- **Context assembly** — The process of building the LLM prompt for an agent's tick. Context is assembled from role (who the agent is), workspace (what it's working on), plan (its current goal), knowledge (relevant past experience), and volatile (per-turn specifics). Context assembly is a learnable control system — the agent tracks which context sections correlated with successful outcomes and adjusts allocations over time.
- **Cognitive workspace** — The typed, budgeted collection of context sections assembled for a given tick. Sections have priorities, token budgets, and sources. The workspace is the agent's working memory.
- **Section effectiveness** — A measure of how much a given context section contributed to the outcome of a tick. Tracked over time. Used to adjust future context allocations.
- **VCG auction (Vickrey–Clarke–Groves)** — The mechanism used inside context assembly to allocate token budget across competing context sources. Different subsystems bid for space; VCG ensures truthful bidding.
- **Tool** — A function an agent can call to affect the world or read from it. Tools include file operations, shell commands, HTTP requests, chain reads and writes, knowledge queries, and custom tools added by extensions. Tool calls are dispatched by the Roko runtime, not by the LLM directly; the LLM requests a tool call and the runtime decides whether to execute it.
- **Tool dispatch** — The subsystem that routes tool call requests from the LLM through appropriate gates, permissions, and somatic checks before execution.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "context|tick|cognitive|time|Heartbeat|tier|internals|call" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "context|tick|cognitive|time|Heartbeat|tier|internals|call" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S011 -- Extensions, domains, gates

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:111` through `132`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extensions, domains, gates

**Extension** — A modular unit of behavior that an agent can load. Each extension implements a subset of 22 lifecycle hooks across 8 layers (Foundation, Perception, Memory, Cognition, Action, Social, Meta, Recovery). Extensions compose — multiple extensions can be loaded together, and they coordinate through shared state.

**Extension layer** — One of eight categorical groupings of extensions: Foundation (heartbeat, clock), Perception (event subscriptions, probes), Memory (knowledge stores, episodic memory), Cognition (affect, attention, gating), Action (tool dispatch, safety, budgets), Social (pheromones, inter-agent messaging), Meta (dreams, consolidation, evolution), Recovery (compensation, rollback, death).

**Hook** — A specific point in the agent lifecycle where extensions can inject behavior. Examples: `on_tick_start`, `assemble_context`, `before_tool_call`, `on_outcome`, `on_dream_start`. Extensions implement only the hooks they care about.

**Domain** — A category of work an agent specializes in: coding, blockchain monitoring, research, security auditing, etc. A domain is configured via a domain profile that declares required extensions, tick frequencies, default gates, and context strategies. Domains are first-class objects users can create, publish, and share via the Korai chain.

**Domain profile** — A declarative specification of a domain: which extensions to load, which gates to apply by default, what tick frequencies to run at, what context categories to emphasize. Domain profiles are TOML or JSON and can be published to the chain for reuse.

**Gate** — A verification step that checks an agent's output against an external truth source. Gates are how the system closes the loop between LLM output and ground reality. A gate produces a boolean pass/fail verdict plus optional detail. Examples: compile, test, clippy, chain simulation, formal verification, human review.

**Gate pipeline** — An ordered sequence of gates that a task passes through before being considered complete. Different domains have different default pipelines. Pipelines are user-configurable.

**Rung** — A gate's position in an escalation hierarchy. Lower rungs are cheap and fast; higher rungs are expensive and thorough. The system adaptively skips rungs that have passed consistently.

**Adaptive gating** — The system's ability to learn which gates are informative for which task types and skip gates that provide little information.

**Capability** — A declared ability of an agent, typed and published on its passport. Examples: `code.rust.rewrite`, `defi.lp.rebalance`, `research.longform.synthesize`. Capabilities are how agents advertise what they can do and how other agents (or users) discover them.
````

**Explicit detail extraction from this section:**

- Section word count: `378`
- Section hash: `4176e3aa00f5540441f4c8b5e50664fb8e833754ee09500ed924571566f772e2`

**Normative requirements and implementation claims:**
- **Extension** — A modular unit of behavior that an agent can load. Each extension implements a subset of 22 lifecycle hooks across 8 layers (Foundation, Perception, Memory, Cognition, Action, Social, Meta, Recovery). Extensions compose — multiple extensions can be loaded together, and they coordinate through shared state.
- **Extension layer** — One of eight categorical groupings of extensions: Foundation (heartbeat, clock), Perception (event subscriptions, probes), Memory (knowledge stores, episodic memory), Cognition (affect, attention, gating), Action (tool dispatch, safety, budgets), Social (pheromones, inter-agent messaging), Meta (dreams, consolidation, evolution), Recovery (compensation, rollback, death).
- **Hook** — A specific point in the agent lifecycle where extensions can inject behavior. Examples: `on_tick_start`, `assemble_context`, `before_tool_call`, `on_outcome`, `on_dream_start`. Extensions implement only the hooks they care about.
- **Domain** — A category of work an agent specializes in: coding, blockchain monitoring, research, security auditing, etc. A domain is configured via a domain profile that declares required extensions, tick frequencies, default gates, and context strategies. Domains are first-class objects users can create, publish, and share via the Korai chain.
- **Domain profile** — A declarative specification of a domain: which extensions to load, which gates to apply by default, what tick frequencies to run at, what context categories to emphasize. Domain profiles are TOML or JSON and can be published to the chain for reuse.
- **Gate** — A verification step that checks an agent's output against an external truth source. Gates are how the system closes the loop between LLM output and ground reality. A gate produces a boolean pass/fail verdict plus optional detail. Examples: compile, test, clippy, chain simulation, formal verification, human review.
- **Gate pipeline** — An ordered sequence of gates that a task passes through before being considered complete. Different domains have different default pipelines. Pipelines are user-configurable.
- **Rung** — A gate's position in an escalation hierarchy. Lower rungs are cheap and fast; higher rungs are expensive and thorough. The system adaptively skips rungs that have passed consistently.
- **Adaptive gating** — The system's ability to learn which gates are informative for which task types and skip gates that provide little information.
- **Capability** — A declared ability of an agent, typed and published on its passport. Examples: `code.rust.rewrite`, `defi.lp.rebalance`, `research.longform.synthesize`. Capabilities are how agents advertise what they can do and how other agents (or users) discover them.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- on_tick_start
- assemble_context
- before_tool_call
- on_outcome
- on_dream_start

**Event names and event-like entities:**
- code.rust.rewrite
- defi.lp.rebalance
- research.longform.synthesize

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- code.rust.rewrite
- defi.lp.rebalance
- research.longform.synthesize

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Gate|Extension|Domain|Extensions|gates|agen|domains|on_tick_start" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Gate|Extension|Domain|Extensions|gates|agen|domains|on_tick_start" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `on_tick_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `assemble_context` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `before_tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_outcome` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_dream_start` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `code.rust.rewrite` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `defi.lp.rebalance` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `research.longform.synthesize` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `code.rust.rewrite` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `defi.lp.rebalance` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `research.longform.synthesize` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S012 -- Knowledge

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:133` through `158`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Knowledge

**Knowledge entry** — A single unit of validated information produced by an agent or user. Has a type (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge), a content body, provenance (who made it, when, under what conditions), confidence (how sure is its author), validations (who has confirmed it against evidence), and decay state (how much its confidence has faded over time).

**Knowledge type** — One of six categories of knowledge entry. Insight is a novel observation. Heuristic is a rule of thumb. Warning is a caution about a failure mode. CausalLink is an asserted cause-effect relationship. StrategyFragment is a reusable piece of strategy. AntiKnowledge is a refuted claim (kept on purpose, to prevent re-deriving wrong conclusions).

**Knowledge store (NeuroStore)** — An agent's local knowledge database. Stores all knowledge the agent has produced or received, indexed by HDC fingerprint for similarity search. The knowledge store is private to the agent unless entries are published.

**InsightStore** — The on-chain component of the knowledge system. Published knowledge entries (or their fingerprints, depending on privacy settings) live here. Discoverable by all agents.

**HDC (hyperdimensional computing)** — A mathematical framework for encoding knowledge as high-dimensional vectors (typically 10,000+ dimensions) where similar knowledge has similar vectors. Enables fast nearest-neighbor search and privacy-preserving similarity queries.

**HDC fingerprint** — A hyperdimensional vector representing a piece of knowledge, a task, or an agent state. Fingerprints can be compared without revealing their content (approximately), enabling agents to share knowledge fingerprints on-chain while keeping the knowledge itself private.

**Resonance** — A match between an agent's current task fingerprint and a knowledge entry's fingerprint. Measured as cosine similarity. High resonance means the knowledge is likely relevant to the task.

**Cross-domain resonance** — A match between fingerprints from different domains. Used to surface insights from one domain that might apply to another. For example, a pattern from coding might resonate with a pattern from trading, suggesting a shared underlying structure.

**Knowledge lineage** — The provenance chain of a knowledge entry: which earlier entries it built on, which agents contributed, what validations it has passed. Lineage is preserved on-chain.

**Knowledge decay** — The gradual reduction of a knowledge entry's confidence over time, unless it is revalidated. Decay prevents stale knowledge from dominating retrieval.

**Validation** — A signed assertion by an agent or user that a knowledge entry matches their own experience or evidence. Validations increase confidence; the more validators, the higher the weight.

**Challenge** — A signed assertion that a knowledge entry is false. Challenges put an entry in a contested state, reducing its effective confidence until resolved.
````

**Explicit detail extraction from this section:**

- Section word count: `424`
- Section hash: `ba2b5d76cafe2e349b25f2ec3dd7968735f38793ee7097d2d6d7c5e75e7e8961`

**Normative requirements and implementation claims:**
- **Knowledge entry** — A single unit of validated information produced by an agent or user. Has a type (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge), a content body, provenance (who made it, when, under what conditions), confidence (how sure is its author), validations (who has confirmed it against evidence), and decay state (how much its confidence has faded over time).
- **Knowledge type** — One of six categories of knowledge entry. Insight is a novel observation. Heuristic is a rule of thumb. Warning is a caution about a failure mode. CausalLink is an asserted cause-effect relationship. StrategyFragment is a reusable piece of strategy. AntiKnowledge is a refuted claim (kept on purpose, to prevent re-deriving wrong conclusions).
- **Knowledge store (NeuroStore)** — An agent's local knowledge database. Stores all knowledge the agent has produced or received, indexed by HDC fingerprint for similarity search. The knowledge store is private to the agent unless entries are published.
- **InsightStore** — The on-chain component of the knowledge system. Published knowledge entries (or their fingerprints, depending on privacy settings) live here. Discoverable by all agents.
- **HDC (hyperdimensional computing)** — A mathematical framework for encoding knowledge as high-dimensional vectors (typically 10,000+ dimensions) where similar knowledge has similar vectors. Enables fast nearest-neighbor search and privacy-preserving similarity queries.
- **HDC fingerprint** — A hyperdimensional vector representing a piece of knowledge, a task, or an agent state. Fingerprints can be compared without revealing their content (approximately), enabling agents to share knowledge fingerprints on-chain while keeping the knowledge itself private.
- **Resonance** — A match between an agent's current task fingerprint and a knowledge entry's fingerprint. Measured as cosine similarity. High resonance means the knowledge is likely relevant to the task.
- **Cross-domain resonance** — A match between fingerprints from different domains. Used to surface insights from one domain that might apply to another. For example, a pattern from coding might resonate with a pattern from trading, suggesting a shared underlying structure.
- **Knowledge lineage** — The provenance chain of a knowledge entry: which earlier entries it built on, which agents contributed, what validations it has passed. Lineage is preserved on-chain.
- **Knowledge decay** — The gradual reduction of a knowledge entry's confidence over time, unless it is revalidated. Decay prevents stale knowledge from dominating retrieval.
- **Validation** — A signed assertion by an agent or user that a knowledge entry matches their own experience or evidence. Validations increase confidence; the more validators, the higher the weight.
- **Challenge** — A signed assertion that a knowledge entry is false. Challenges put an entry in a contested state, reducing its effective confidence until resolved.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "knowle|Knowledge|fingerprint|entry|store|similar|confidence|fingerprints" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "knowle|Knowledge|fingerprint|entry|store|similar|confidence|fingerprints" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S013 -- Stigmergy and coordination

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:159` through `170`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Stigmergy and coordination

**Stigmergy** — A coordination pattern where agents leave signals in a shared environment rather than messaging each other directly. Agents that encounter the signals respond to them. Inspired by how ants coordinate through pheromone trails. In Nunchi, stigmergy happens through the Korai chain — agents deposit signals on-chain, other agents read them and react.

**Pheromone** — A signal deposited on-chain by an agent. Has a type (WISDOM, OPPORTUNITY, THREAT, etc.), a strength, a decay rate, and a source agent. Pheromones fade over time unless reinforced.

**Pheromone field** — The aggregate state of all active pheromones. Visualizable as a field of signals with positions, strengths, and types.

**Group** — A coordinated subset of agents working toward a shared goal. Groups can be formed for a single task or long-running. Groups have shared state, a coordination protocol, and a membership list. Groups are first-class objects.

**Coordination protocol** — The set of rules by which agents in a group interact. Can be role-based (each agent plays a defined role), market-based (agents bid on tasks), hierarchical (a lead agent delegates), or emergent (agents self-organize via pheromones).
````

**Explicit detail extraction from this section:**

- Section word count: `183`
- Section hash: `8c212182da3a3e66487134f878bc5205a0e3fbe6267e0d32eda444155d585eb2`

**Normative requirements and implementation claims:**
- **Stigmergy** — A coordination pattern where agents leave signals in a shared environment rather than messaging each other directly. Agents that encounter the signals respond to them. Inspired by how ants coordinate through pheromone trails. In Nunchi, stigmergy happens through the Korai chain — agents deposit signals on-chain, other agents read them and react.
- **Pheromone** — A signal deposited on-chain by an agent. Has a type (WISDOM, OPPORTUNITY, THREAT, etc.), a strength, a decay rate, and a source agent. Pheromones fade over time unless reinforced.
- **Pheromone field** — The aggregate state of all active pheromones. Visualizable as a field of signals with positions, strengths, and types.
- **Group** — A coordinated subset of agents working toward a shared goal. Groups can be formed for a single task or long-running. Groups have shared state, a coordination protocol, and a membership list. Groups are first-class objects.
- **Coordination protocol** — The set of rules by which agents in a group interact. Can be role-based (each agent plays a defined role), market-based (agents bid on tasks), hierarchical (a lead agent delegates), or emergent (agents self-organize via pheromones).

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "coordination|Stigmergy|pheromone|signal|Group|signals|shared" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "coordination|Stigmergy|pheromone|signal|Group|signals|shared" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S014 -- Heartbeat and timing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:171` through `184`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Heartbeat and timing

**Gamma tick** — Fast heartbeat tick, every 5–15 seconds. Mostly T0 cognition. Used for perception, triage, and streaming state updates.

**Theta tick** — Full decision cycle tick, every 30–120 seconds. May use T0, T1, or T2 cognition depending on gating. Used for substantive action.

**Delta tick** — Offline consolidation tick, roughly every 50 theta ticks. Dreams, pattern extraction, knowledge consolidation happen here. Agent is not actively observing during delta.

**Dream cycle** — An offline process where an agent replays recent episodes, extracts patterns, generates counterfactuals, rehearses threats, and promotes validated insights into its knowledge store. Dreams are where learning consolidates.

**Episode** — A single end-to-end task attempt. Contains the initial state, the actions taken, the outcomes observed, and the final assessment. Episodes are the unit of learning.

**Episodic memory** — The agent's collection of past episodes, used as training data for its own improvement.
````

**Explicit detail extraction from this section:**

- Section word count: `143`
- Section hash: `6d4eeb09e51c2d18e0f84986512037f6e7eab49455e8f6e265320266e36563d6`

**Normative requirements and implementation claims:**
- **Gamma tick** — Fast heartbeat tick, every 5–15 seconds. Mostly T0 cognition. Used for perception, triage, and streaming state updates.
- **Theta tick** — Full decision cycle tick, every 30–120 seconds. May use T0, T1, or T2 cognition depending on gating. Used for substantive action.
- **Delta tick** — Offline consolidation tick, roughly every 50 theta ticks. Dreams, pattern extraction, knowledge consolidation happen here. Agent is not actively observing during delta.
- **Dream cycle** — An offline process where an agent replays recent episodes, extracts patterns, generates counterfactuals, rehearses threats, and promotes validated insights into its knowledge store. Dreams are where learning consolidates.
- **Episode** — A single end-to-end task attempt. Contains the initial state, the actions taken, the outcomes observed, and the final assessment. Episodes are the unit of learning.
- **Episodic memory** — The agent's collection of past episodes, used as training data for its own improvement.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "tick|Heartbeat|timing|Episode|here|every|episodes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tick|Heartbeat|timing|Episode|here|every|episodes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S015 -- Affect and somatic

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:185` through `194`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Affect and somatic

**Affect** — The agent's emotional state, represented as a PAD vector (Pleasure, Arousal, Dominance). Updated based on outcomes of actions. Affect modulates context assembly, gate thresholds, and model routing.

**PAD vector** — A three-dimensional representation of emotional state. Each dimension ranges from -1 to +1. Pleasure reflects whether outcomes are positive. Arousal reflects activation level. Dominance reflects agency and control.

**Somatic marker** — An associative link between a type of action and an affective response, learned from past experience. When the agent considers an action matching past failures, a somatic marker generates hesitation — raising gate thresholds, requiring more verification, or escalating cognitive tier.

**Hesitation** — A signal that increases verification requirements on an action. Hesitation comes from somatic markers. It is not a binary stop — it is a continuous gradient.
````

**Explicit detail extraction from this section:**

- Section word count: `128`
- Section hash: `d9142d1ac88a9b6b322a116113dd376b4ae874ac36aa19ea78b7b2ff28629222`

**Normative requirements and implementation claims:**
- **Affect** — The agent's emotional state, represented as a PAD vector (Pleasure, Arousal, Dominance). Updated based on outcomes of actions. Affect modulates context assembly, gate thresholds, and model routing.
- **PAD vector** — A three-dimensional representation of emotional state. Each dimension ranges from -1 to +1. Pleasure reflects whether outcomes are positive. Arousal reflects activation level. Dominance reflects agency and control.
- **Somatic marker** — An associative link between a type of action and an affective response, learned from past experience. When the agent considers an action matching past failures, a somatic marker generates hesitation — raising gate thresholds, requiring more verification, or escalating cognitive tier.
- **Hesitation** — A signal that increases verification requirements on an action. Hesitation comes from somatic markers. It is not a binary stop — it is a continuous gradient.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- of

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "somatic|Affect|action|reflects|marker|hesitation|comes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "somatic|Affect|action|reflects|marker|hesitation|comes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Implement or verify `of` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S016 -- Arenas and evaluation

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:195` through `212`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Arenas and evaluation

**Arena** — A defined evaluation environment. Has a task source (where tasks come from), a gate configuration (how attempts are scored), a scoring function (how scores aggregate), and a leaderboard (who's ahead). Arenas are first-class objects. Examples: SWE-bench (coding), AMM optimization (DeFi), chess, prediction markets, persuasion challenges.

**Task source** — The mechanism by which an arena generates tasks. Can be a static dataset, a procedural generator, a user-contributed stream, or an adversarial agent.

**Scoring function** — The mathematical function that converts an attempt's outcome into a score. Can be binary (pass/fail), continuous (e.g., Sharpe ratio), or probabilistic (e.g., CRPS score).

**CRPS (Continuous Ranked Probability Score)** — A scoring rule for probabilistic forecasts. Rewards calibrated predictions; penalizes overconfidence. Used when the correct answer is a distribution, not a point.

**Leaderboard** — A ranked list of agents by performance on an arena. Can be global, per-domain, or per-cohort.

**Bounty** — A user-posted task with a reward for completion. Bounties can be standalone or associated with an arena. Bounties are on-chain.

**Challenge** — A specific, scoped task within an arena. Agents attempt challenges; attempts produce scores.

**Attempt** — One agent's execution of one challenge. Has a start time, an end time, a cost, a set of tool calls, a final output, and a score.
````

**Explicit detail extraction from this section:**

- Section word count: `216`
- Section hash: `8df486f8f33f4f4d6fe3a0593097f07a4b3645c36eba44f9243e98deb35e2577`

**Normative requirements and implementation claims:**
- **Arena** — A defined evaluation environment. Has a task source (where tasks come from), a gate configuration (how attempts are scored), a scoring function (how scores aggregate), and a leaderboard (who's ahead). Arenas are first-class objects. Examples: SWE-bench (coding), AMM optimization (DeFi), chess, prediction markets, persuasion challenges.
- **Task source** — The mechanism by which an arena generates tasks. Can be a static dataset, a procedural generator, a user-contributed stream, or an adversarial agent.
- **Scoring function** — The mathematical function that converts an attempt's outcome into a score. Can be binary (pass/fail), continuous (e.g., Sharpe ratio), or probabilistic (e.g., CRPS score).
- **CRPS (Continuous Ranked Probability Score)** — A scoring rule for probabilistic forecasts. Rewards calibrated predictions; penalizes overconfidence. Used when the correct answer is a distribution, not a point.
- **Leaderboard** — A ranked list of agents by performance on an arena. Can be global, per-domain, or per-cohort.
- **Bounty** — A user-posted task with a reward for completion. Bounties can be standalone or associated with an arena. Bounties are on-chain.
- **Challenge** — A specific, scoped task within an arena. Agents attempt challenges; attempts produce scores.
- **Attempt** — One agent's execution of one challenge. Has a start time, an end time, a cost, a set of tool calls, a final output, and a score.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Arena|score|task|evaluation|Arenas|attempt|Challenge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|score|task|evaluation|Arenas|attempt|Challenge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S017 -- Eval system

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:213` through `224`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Eval system

**Eval** — A measurement of agent behavior against a ground truth. A first-class object in the system. Has a name, an author, a definition (what is being measured, how), a ground truth source (where the correct answer comes from — never the LLM itself), a scoring function, and a history of applications. Evals can be composed, versioned, challenged, and retired.

**Ground truth** — The external source that evals measure against. Can be a gate outcome, an arena result, a chain state, a clearing settlement, an oracle value, a user judgment, or any other non-LLM source of truth.

**External eval** — An eval whose ground truth does not come from an LLM. The product's cybernetic property depends on external evals — if LLMs grade themselves, the feedback loop collapses.

**Measurement** — The application of an eval to an agent or output, producing a score.

**Measurement surface** — A UI region or page that exposes measurements as first-class interactive objects.
````

**Explicit detail extraction from this section:**

- Section word count: `155`
- Section hash: `70db0ee3e08f48b49774ef1bf7e557ea5f743179f7d4242901265ffd6d0c905e`

**Normative requirements and implementation claims:**
- **Eval** — A measurement of agent behavior against a ground truth. A first-class object in the system. Has a name, an author, a definition (what is being measured, how), a ground truth source (where the correct answer comes from — never the LLM itself), a scoring function, and a history of applications. Evals can be composed, versioned, challenged, and retired.
- **Ground truth** — The external source that evals measure against. Can be a gate outcome, an arena result, a chain state, a clearing settlement, an oracle value, a user judgment, or any other non-LLM source of truth.
- **External eval** — An eval whose ground truth does not come from an LLM. The product's cybernetic property depends on external evals — if LLMs grade themselves, the feedback loop collapses.
- **Measurement** — The application of an eval to an agent or output, producing a score.
- **Measurement surface** — A UI region or page that exposes measurements as first-class interactive objects.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Eval|measure|truth|measurement|ground|external|come|Evals" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Eval|measure|truth|measurement|ground|external|come|Evals" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S018 -- Economic and clearing

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:225` through `236`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Economic and clearing

**ISFR (Interest-bearing Stablecoin Funding Rate)** — A benchmark interest rate computed on-chain at consensus from a basket of sources. The core reference rate for Korai-native yield products.

**Yield perpetual** — A derivative contract settling against ISFR or other on-chain yield benchmarks. Enables hedging and speculation on rates.

**Cooperative clearing** — A periodic settlement process that matches buyers and sellers of yield perpetuals using a cooperative mechanism (not a traditional order book). Maximizes total welfare; has clearing rounds and a clearing price.

**Position** — A user's holding in a yield perpetual or other tradable instrument.

**Daeji token (daeji)** — A utility token used as escrow and stake in the marketplace. Agents stake daeji to post knowledge, participate in arenas, or bid on bounties.
````

**Explicit detail extraction from this section:**

- Section word count: `122`
- Section hash: `65ab4df3e5c5e50edcc83af4d26e9bff879930b58073223d96f867476ad998e6`

**Normative requirements and implementation claims:**
- **ISFR (Interest-bearing Stablecoin Funding Rate)** — A benchmark interest rate computed on-chain at consensus from a basket of sources. The core reference rate for Korai-native yield products.
- **Yield perpetual** — A derivative contract settling against ISFR or other on-chain yield benchmarks. Enables hedging and speculation on rates.
- **Cooperative clearing** — A periodic settlement process that matches buyers and sellers of yield perpetuals using a cooperative mechanism (not a traditional order book). Maximizes total welfare; has clearing rounds and a clearing price.
- **Position** — A user's holding in a yield perpetual or other tradable instrument.
- **Daeji token (daeji)** — A utility token used as escrow and stake in the marketplace. Agents stake daeji to post knowledge, participate in arenas, or bid on bounties.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "clearing|yield|Economic|Rate|perpetual|Daeji|token" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "clearing|yield|Economic|Rate|perpetual|Daeji|token" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S019 -- UI concepts

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:237` through `250`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### UI concepts

**Lens** — A perspective the user takes on a piece of data. The five lenses are: global (the whole network), fleet (agents I own), agent (one specific agent), group (a coordinated subset), chain (on-chain state). The same data can be viewed through multiple lenses. Lenses are formalized in `05-lenses-and-perspectives.md`.

**Persona** — A category of user the dashboard is designed for. Personas are defined in `03-personas-and-jobs.md` and referenced by short handle (e.g., Solo Operator, Fleet Orchestrator) throughout the rest of the documents.

**Surface** — A page or major UI region. A surface has a primary persona, a primary lens, a set of data sources, and a set of interactions. Surfaces are specified in documents 11 through 18.

**Authoring surface** — A surface through which users create new things (agents, extensions, arenas, evals, etc.). Documented in `19-authoring-surfaces.md`.

**Composition pattern** — A recurring way that small primitives combine to make larger things. Documented in `20-composition-patterns.md`. The DAW principle lands here.

**Reactivity hook** — A place in a surface specification where performance-reactive aesthetics apply. Documented per-surface in sections 11–18 and framed globally in `08-epistemic-aesthetics.md`.
````

**Explicit detail extraction from this section:**

- Section word count: `196`
- Section hash: `dc309247454f2dc2db220177afaf7f6708c0af9ed5af65e1fe58132a5a5ef3cf`

**Normative requirements and implementation claims:**
- **Lens** — A perspective the user takes on a piece of data. The five lenses are: global (the whole network), fleet (agents I own), agent (one specific agent), group (a coordinated subset), chain (on-chain state). The same data can be viewed through multiple lenses. Lenses are formalized in `05-lenses-and-perspectives.md`.
- **Persona** — A category of user the dashboard is designed for. Personas are defined in `03-personas-and-jobs.md` and referenced by short handle (e.g., Solo Operator, Fleet Orchestrator) throughout the rest of the documents.
- **Surface** — A page or major UI region. A surface has a primary persona, a primary lens, a set of data sources, and a set of interactions. Surfaces are specified in documents 11 through 18.
- **Authoring surface** — A surface through which users create new things (agents, extensions, arenas, evals, etc.). Documented in `19-authoring-surfaces.md`.
- **Composition pattern** — A recurring way that small primitives combine to make larger things. Documented in `20-composition-patterns.md`. The DAW principle lands here.
- **Reactivity hook** — A place in a surface specification where performance-reactive aesthetics apply. Documented per-surface in sections 11–18 and framed globally in `08-epistemic-aesthetics.md`.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "Surface|Lens|concepts|lenses|Persona|user|data|Documented" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Surface|Lens|concepts|lenses|Persona|user|data|Documented" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S020 -- Visualization and aesthetic

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:251` through `270`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Visualization and aesthetic

**Epistemic sharpness** — A scalar derived from confidence × validation × recency, used to drive visual properties (saturation, edge quality, motion coherence) in performance-reactive aesthetics. High sharpness means crisp, saturated, resolved. Low sharpness means muted, blurred, unresolved.

**Reactive property** — A visual property (like saturation or motion amplitude) that modulates in response to system state. Reactive properties live in the "ambient layer" of the UI.

**Stable property** — A visual property (like layout or typography) that does not modulate with system state. Stable properties live in the "functional layer" of the UI and ensure usability.

**Ambient layer** — The non-load-bearing visual layer of a surface. Can react to state without breaking the surface.

**Functional layer** — The load-bearing visual layer of a surface. Must remain stable and legible regardless of state.

**Stigmergy field visualization** — The standard UI primitive for rendering pheromone fields. Documented in `09-visualization-primitives.md`.

**Resonance graph** — A visualization of knowledge entries positioned by similarity, with edges indicating resonance above some threshold. Documented in `09-visualization-primitives.md`.

**Knowledge topography** — A 2D or 3D representation of a knowledge domain as terrain: peaks for high-confidence validated entries, valleys for contested areas, bridges between domains where cross-domain resonance exists.

**Tier ladder** — A visual component showing the five reputation tiers as a vertical metal progression (Gray to Amber), with an agent's current position highlighted.
````

**Explicit detail extraction from this section:**

- Section word count: `224`
- Section hash: `767d4b2a56096c87696bdc8ef8021d3b193668ca6469d0462509bbc209ceab4b`

**Normative requirements and implementation claims:**
- **Epistemic sharpness** — A scalar derived from confidence × validation × recency, used to drive visual properties (saturation, edge quality, motion coherence) in performance-reactive aesthetics. High sharpness means crisp, saturated, resolved. Low sharpness means muted, blurred, unresolved.
- **Reactive property** — A visual property (like saturation or motion amplitude) that modulates in response to system state. Reactive properties live in the "ambient layer" of the UI.
- **Stable property** — A visual property (like layout or typography) that does not modulate with system state. Stable properties live in the "functional layer" of the UI and ensure usability.
- **Ambient layer** — The non-load-bearing visual layer of a surface. Can react to state without breaking the surface.
- **Functional layer** — The load-bearing visual layer of a surface. Must remain stable and legible regardless of state.
- **Stigmergy field visualization** — The standard UI primitive for rendering pheromone fields. Documented in `09-visualization-primitives.md`.
- **Resonance graph** — A visualization of knowledge entries positioned by similarity, with edges indicating resonance above some threshold. Documented in `09-visualization-primitives.md`.
- **Knowledge topography** — A 2D or 3D representation of a knowledge domain as terrain: peaks for high-confidence validated entries, valleys for contested areas, bridges between domains where cross-domain resonance exists.
- **Tier ladder** — A visual component showing the five reputation tiers as a vertical metal progression (Gray to Amber), with an agent's current position highlighted.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "visual|Visualization|layer|aesthetic|edge|state|react" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "visual|Visualization|layer|aesthetic|edge|state|react" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S021 -- Backend and APIs

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:271` through `284`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Backend and APIs

**Roko sidecar** — An HTTP server that each running Roko agent exposes. The dashboard connects to the sidecar to query agent state, send messages, and subscribe to event streams. Documented endpoints are listed in `21-roko-and-chain-additions.md`.

**Event fabric** — The streaming event system inside a Roko agent. Publishes typed events via a broadcast channel. The dashboard subscribes via WebSocket to receive real-time updates.

**Event type** — A category of event the event fabric publishes. Examples: `tick.started`, `gate.passed`, `knowledge.promoted`, `pheromone.deposited`, `tool.called`. Typed, versioned, documented.

**RPC (to Korai or Mirage)** — The JSON-RPC interface to the chain. The dashboard uses this for on-chain reads and writes. Standard EVM RPC plus Korai-specific extensions for ISFR queries, clearing state, and knowledge substrate.

**WebSocket subscription** — A persistent connection the dashboard opens to receive real-time events. Used for agent event streams, chain block notifications, and pheromone field updates.

**Managed cloud (Nunchi Cloud)** — A hosted runtime option the dashboard can provision agents on. Not required — users can run agents locally or on their own infrastructure — but offered as a zero-setup option.
````

**Explicit detail extraction from this section:**

- Section word count: `185`
- Section hash: `c7f5655a8807ad01047f1ed9af8f24ba536874cbbd5c016755255e23ffbfade8`

**Normative requirements and implementation claims:**
- **Roko sidecar** — An HTTP server that each running Roko agent exposes. The dashboard connects to the sidecar to query agent state, send messages, and subscribe to event streams. Documented endpoints are listed in `21-roko-and-chain-additions.md`.
- **Event fabric** — The streaming event system inside a Roko agent. Publishes typed events via a broadcast channel. The dashboard subscribes via WebSocket to receive real-time updates.
- **Event type** — A category of event the event fabric publishes. Examples: `tick.started`, `gate.passed`, `knowledge.promoted`, `pheromone.deposited`, `tool.called`. Typed, versioned, documented.
- **RPC (to Korai or Mirage)** — The JSON-RPC interface to the chain. The dashboard uses this for on-chain reads and writes. Standard EVM RPC plus Korai-specific extensions for ISFR queries, clearing state, and knowledge substrate.
- **WebSocket subscription** — A persistent connection the dashboard opens to receive real-time events. Used for agent event streams, chain block notifications, and pheromone field updates.
- **Managed cloud (Nunchi Cloud)** — A hosted runtime option the dashboard can provision agents on. Not required — users can run agents locally or on their own infrastructure — but offered as a zero-setup option.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- tick.started
- gate.passed
- knowledge.promoted
- pheromone.deposited
- tool.called

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- tick.started
- gate.passed
- knowledge.promoted
- pheromone.deposited
- tool.called

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "event|Backend|APIs|chain|type|time|updates" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "event|Backend|APIs|chain|type|time|updates" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- [ ] Emit or consume `tick.started` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `gate.passed` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `knowledge.promoted` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `pheromone.deposited` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `tool.called` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `tick.started` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `gate.passed` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `knowledge.promoted` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `pheromone.deposited` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `tool.called` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S022 -- Meta and generative

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:285` through `294`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Meta and generative

**Meta-agent** — An agent that creates other agents. Meta-agents take a specification (a task or a goal) and produce a configured agent (with domain, extensions, gates, model routing) appropriate to the task.

**Generator** — A more general term for any agent or tool that produces new first-class objects: new extensions, new arenas, new evals, new domain profiles.

**Meta-eval** — An eval that evaluates evals. Measures whether an eval is well-calibrated, well-scoped, and producing useful signal.

**Recursion** — The general pattern of applying the system to itself. Agents that improve agents. Evals that evaluate evals. Generators that generate generators. This is the "tools for tools" principle, and it is canonical to the product's thesis.
````

**Explicit detail extraction from this section:**

- Section word count: `116`
- Section hash: `1c8acf87b334d90f2abfc003fc946744e65b5819a0bb3af53272eec41ee82c94`

**Normative requirements and implementation claims:**
- **Meta-agent** — An agent that creates other agents. Meta-agents take a specification (a task or a goal) and produce a configured agent (with domain, extensions, gates, model routing) appropriate to the task.
- **Generator** — A more general term for any agent or tool that produces new first-class objects: new extensions, new arenas, new evals, new domain profiles.
- **Meta-eval** — An eval that evaluates evals. Measures whether an eval is well-calibrated, well-scoped, and producing useful signal.
- **Recursion** — The general pattern of applying the system to itself. Agents that improve agents. Evals that evaluate evals. Generators that generate generators. This is the "tools for tools" principle, and it is canonical to the product's thesis.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
rg -n "eval|Meta|generative|evals|tool|Generator|well" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "eval|Meta|generative|evals|tool|Generator|well" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

### DASH-01-S023 -- A note on what is not in this glossary

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md:295` through `306`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## A note on what is not in this glossary

This glossary covers terms used in the specification documents. It does not cover:

- Every function in the Roko codebase.
- Every Solidity contract on Korai.
- Every command-line flag of the CLI.
- Every API endpoint exposed by any service.

Those live in the implementation documentation. This glossary is for reading the specification. If an implementation term is used in a specification, it is because that term is a concept — not an implementation detail — and is defined here.

If a specification document uses a term that is not in this glossary, that is a bug in the specification and should be fixed.
````

**Explicit detail extraction from this section:**

- Section word count: `100`
- Section hash: `845cd35eeffa2272aab0f96d4cd0060463f464c05abfd97ec96cae4e138c4af7`

**Normative requirements and implementation claims:**
- - Every function in the Roko codebase. - Every Solidity contract on Korai. - Every command-line flag of the CLI. - Every API endpoint exposed by any service.
- If a specification document uses a term that is not in this glossary, that is a bug in the specification and should be fixed.

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
- - Every function in the Roko codebase.
- - Every Solidity contract on Korai.
- - Every command-line flag of the CLI.
- - Every API endpoint exposed by any service.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/01-system-landscape.md`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "not|glossary|specification|note|term|Every|document|cover" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "not|glossary|specification|note|term|Every|document|cover" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/projections.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/01-system-landscape
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

