# Architecture Plan: Tui And Operations

**Source:** `tmp/architecture/21-tui-and-operations.md`
**Generated:** 2026-04-25
**Source hash:** `0a825b2de286721078ba29d32b42802d3cc37760be96b0164faef7cba1e1d6a5`
**Section tasks:** 13
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
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-21-S001 | 1 | TUI Enhancements and Operations | [ ] | 9.8 |
| ARCH-21-S002 | 8 | TUI enhancements | [ ] | 9.8 |
| ARCH-21-S003 | 14 | 1. DaimonState visualization | [ ] | 9.8 |
| ARCH-21-S004 | 42 | 2. Heartbeat status view in Learning tab | [ ] | 9.8 |
| ARCH-21-S005 | 67 | 3. Knowledge browser in Inspect tab | [ ] | 9.8 |
| ARCH-21-S006 | 91 | Operational infrastructure | [ ] | 9.8 |
| ARCH-21-S007 | 97 | 4. Justfile (developer convenience) | [ ] | 9.8 |
| ARCH-21-S008 | 132 | 5. E2E test harness | [ ] | 9.8 |
| ARCH-21-S009 | 156 | 6. Self-healing supervisor script | [ ] | 9.8 |
| ARCH-21-S010 | 181 | Conductor watcher configuration (added 2026-04-25) | [ ] | 9.8 |
| ARCH-21-S011 | 206 | Implementation state (updated 2026-04-25) | [ ] | 9.8 |
| ARCH-21-S012 | 208 | What already exists | [ ] | 9.8 |
| ARCH-21-S013 | 219 | What needs building | [ ] | 9.8 |

## Tasks

### ARCH-21-S001 -- TUI Enhancements and Operations

**Source section:** `tmp/architecture/21-tui-and-operations.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# TUI Enhancements and Operations

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Folded from `tmp/bardo-integration-plan.md` Phases 8-9. Original bardo source references preserved.

---
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `c3c07dda7ad434a16bd316defe9a5cac998f1cf9dc1b119f25422d2c78ee36e4`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
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
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "TUI|Operations|Enhancements|bardo|references|preserved|plan" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "TUI|Operations|Enhancements|bardo|references|preserved|plan" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S002 -- TUI enhancements

**Source section:** `tmp/architecture/21-tui-and-operations.md:8` through `13`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## TUI enhancements

These add visualization capabilities to roko's ratatui TUI (`crates/roko-cli/src/tui/`), porting screens from bardo's terminal (`bardo/apps/bardo-terminal/src/screens/`).

---
````

**Explicit detail extraction from this section:**

- Section word count: `26`
- Section hash: `004e34b63edbc77333f5fd43c92292db0f5043356542e3b77f30a5bd14ebf4b6`

**Normative requirements and implementation claims:**
- These add visualization capabilities to roko's ratatui TUI (`crates/roko-cli/src/tui/`), porting screens from bardo's terminal (`bardo/apps/bardo-terminal/src/screens/`).
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/bardo-terminal/src/screens/
- crates/roko-cli/src/tui/

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
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/apps/bardo-terminal/src/screens/`
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
rg -n "TUI|enhancements|bardo|terminal|screens|visualization|ratatui|porting" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "TUI|enhancements|bardo|terminal|screens|visualization|ratatui|porting" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/apps/bardo-terminal/src/screens/`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S003 -- 1. DaimonState visualization

**Source section:** `tmp/architecture/21-tui-and-operations.md:14` through `41`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 1. DaimonState visualization

**Source**: `bardo/apps/bardo-terminal/src/screens/` — Emotions, Vitality screens
**Target**: `crates/roko-cli/src/tui/views/` (new view)
**Existing**: `roko/crates/roko-daimon/src/` — DaimonState already loaded in orchestrate.rs

New sub-view in Dashboard (F1) or new tab (F11 Affect):

**Layout**:
- PAD vector display: Pleasure [-1,1], Arousal [-1,1], Dominance [-1,1] as horizontal gauges
- Current PadRegion label (Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/Bored)
- Somatic marker histogram: last 10 markers with valence coloring
- Behavioral bias indicators: which biases are active (AvoidTrade, SeekSafety, etc.)

**Data source**: DaimonState is already loaded in orchestrate.rs — pipe it through DashboardEvent to TUI state.

**Visual style**: Use existing braille sparklines and gauge widgets.

**Acceptance criteria**:
- [ ] PAD gauges render with correct values from DaimonState
- [ ] PadRegion label updates in real-time
- [ ] Somatic markers visible with positive (green) / negative (red) coloring
- [ ] View accessible via F-key or tab navigation

**Size**: M (2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `150`
- Section hash: `8a4846a388ce0a39ec00f05925261039c56bf9965d7b79dbeeb59bfc9c061453`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/bardo-terminal/src/screens/` — Emotions, Vitality screens **Target**: `crates/roko-cli/src/tui/views/` (new view) **Existing**: `roko/crates/roko-daimon/src/` — DaimonState already loaded in orchestrate.rs
- New sub-view in Dashboard (F1) or new tab (F11 Affect):
- **Layout**: - PAD vector display: Pleasure [-1,1], Arousal [-1,1], Dominance [-1,1] as horizontal gauges - Current PadRegion label (Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/Bored) - Somatic marker histogram: last 10 markers with valence coloring - Behavioral bias indicators: which biases are active (AvoidTrade, SeekSafety, etc.)
- **Data source**: DaimonState is already loaded in orchestrate.rs — pipe it through DashboardEvent to TUI state.
- **Visual style**: Use existing braille sparklines and gauge widgets.
- **Acceptance criteria**: - [ ] PAD gauges render with correct values from DaimonState - [ ] PadRegion label updates in real-time - [ ] Somatic markers visible with positive (green) / negative (red) coloring - [ ] View accessible via F-key or tab navigation
- **Size**: M (2 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/
- bardo/apps/bardo-terminal/src/screens/
- crates/roko-cli/src/tui/views/
- roko/crates/roko-daimon/src/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- DashboardEvent

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - PAD vector display: Pleasure [-1,1], Arousal [-1,1], Dominance [-1,1] as horizontal gauges
- - Current PadRegion label (Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/Bored)
- - Somatic marker histogram: last 10 markers with valence coloring
- - Behavioral bias indicators: which biases are active (AvoidTrade, SeekSafety, etc.)
- - [ ] PAD gauges render with correct values from DaimonState
- - [ ] PadRegion label updates in real-time
- - [ ] Somatic markers visible with positive (green) / negative (red) coloring
- - [ ] View accessible via F-key or tab navigation

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/`
- `bardo/apps/bardo-terminal/src/screens/`
- `crates/roko-cli/src/tui/views/`
- `roko/crates/roko-daimon/src/`
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
rg -n "state|daimon|DaimonState|Visual|visualization|view|marker|gauge" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "state|daimon|DaimonState|Visual|visualization|view|marker|gauge" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/`
- `bardo/apps/bardo-terminal/src/screens/`
- `crates/roko-cli/src/tui/views/`
- `roko/crates/roko-daimon/src/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S004 -- 2. Heartbeat status view in Learning tab

**Source section:** `tmp/architecture/21-tui-and-operations.md:42` through `66`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 2. Heartbeat status view in Learning tab

**Source**: `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs`
**Target**: `crates/roko-cli/src/tui/views/learning_view.rs` (extend)

Add a heartbeat status sub-view to the Learning tab (F10):

**Layout**:
1. **Accuracy sparkline**: Rolling sparkline of prediction accuracy (correct/total per window)
2. **Recent predictions**: Table of last 10 prediction/outcome pairs with model, tier, cost
3. **Tier distribution**: Bar chart showing T0/T1/T2 tick percentages
4. **Cost trend**: Sparkline of per-tick cost over last hour

**Data source**: Efficiency events JSONL + episodes JSONL (already tailed by TUI).

**Acceptance criteria**:
- [ ] Accuracy sparkline updates as new episodes arrive
- [ ] Tier distribution shows percentage of T0/T1/T2 ticks
- [ ] Cost trend sparkline renders
- [ ] Accessible as sub-view within F10 (Learning tab)

**Size**: S (1 day)

---
````

**Explicit detail extraction from this section:**

- Section word count: `124`
- Section hash: `2c20c9c1f99da30004233b81d72f9d633580db219262afc745293cc70320c4b4`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs` **Target**: `crates/roko-cli/src/tui/views/learning_view.rs` (extend)
- Add a heartbeat status sub-view to the Learning tab (F10):
- **Layout**: 1. **Accuracy sparkline**: Rolling sparkline of prediction accuracy (correct/total per window) 2. **Recent predictions**: Table of last 10 prediction/outcome pairs with model, tier, cost 3. **Tier distribution**: Bar chart showing T0/T1/T2 tick percentages 4. **Cost trend**: Sparkline of per-tick cost over last hour
- **Data source**: Efficiency events JSONL + episodes JSONL (already tailed by TUI).
- **Acceptance criteria**: - [ ] Accuracy sparkline updates as new episodes arrive - [ ] Tier distribution shows percentage of T0/T1/T2 ticks - [ ] Cost trend sparkline renders - [ ] Accessible as sub-view within F10 (Learning tab)
- **Size**: S (1 day)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- T0/T1/
- bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs
- crates/roko-cli/src/tui/views/learning_view.rs

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
- 1. **Accuracy sparkline**: Rolling sparkline of prediction accuracy (correct/total per window)
- 2. **Recent predictions**: Table of last 10 prediction/outcome pairs with model, tier, cost
- 3. **Tier distribution**: Bar chart showing T0/T1/T2 tick percentages
- 4. **Cost trend**: Sparkline of per-tick cost over last hour
- - [ ] Accuracy sparkline updates as new episodes arrive
- - [ ] Tier distribution shows percentage of T0/T1/T2 ticks
- - [ ] Cost trend sparkline renders
- - [ ] Accessible as sub-view within F10 (Learning tab)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `T0/T1/`
- `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs`
- `crates/roko-cli/src/tui/views/learning_view.rs`
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
rg -n "view|tab|Learning|status|Heartbeat|sparkline|cost|tier" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "view|tab|Learning|status|Heartbeat|sparkline|cost|tier" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `T0/T1/`
- `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs`
- `crates/roko-cli/src/tui/views/learning_view.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S005 -- 3. Knowledge browser in Inspect tab

**Source section:** `tmp/architecture/21-tui-and-operations.md:67` through `90`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 3. Knowledge browser in Inspect tab

**Source**: `bardo/apps/bardo-terminal/src/screens/knowledge.rs` — Grimoire stats, top-confidence entries
**Target**: `crates/roko-cli/src/tui/views/` (extend inspect view)

Enhance the Knowledge sub-view in Inspect tab (F7):

**Layout**:
1. **Store stats**: Total entries, per-tier counts (Tier1/2/3/Archive), health percentage
2. **Top entries**: Scrollable list of highest-confidence entries with: title, confidence, tier, last_accessed, type label
3. **Decay visualization**: Time-since-access color gradient (green=fresh, yellow=aging, red=decaying)
4. **Distillation events**: Recent distillation timeline (when knowledge was summarized/compressed)

**Data source**: roko-neuro knowledge store API.

**Acceptance criteria**:
- [ ] Store stats render (entry counts, tier distribution)
- [ ] Top entries scrollable with confidence and tier info
- [ ] Decay visualization uses color gradients

**Size**: S (1 day)

---
````

**Explicit detail extraction from this section:**

- Section word count: `122`
- Section hash: `0cf78aa2593ecd69a7cf8cf8b69cc6531927a632938a945fa778663cd9a4d9d1`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/bardo-terminal/src/screens/knowledge.rs` — Grimoire stats, top-confidence entries **Target**: `crates/roko-cli/src/tui/views/` (extend inspect view)
- **Layout**: 1. **Store stats**: Total entries, per-tier counts (Tier1/2/3/Archive), health percentage 2. **Top entries**: Scrollable list of highest-confidence entries with: title, confidence, tier, last_accessed, type label 3. **Decay visualization**: Time-since-access color gradient (green=fresh, yellow=aging, red=decaying) 4. **Distillation events**: Recent distillation timeline (when knowledge was summarized/compressed)
- **Data source**: roko-neuro knowledge store API.
- **Acceptance criteria**: - [ ] Store stats render (entry counts, tier distribution) - [ ] Top entries scrollable with confidence and tier info - [ ] Decay visualization uses color gradients
- **Size**: S (1 day)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Tier1/2/3/
- bardo/apps/bardo-terminal/src/screens/knowledge.rs
- crates/roko-cli/src/tui/views/

**Types, functions, traits, and inline code identifiers:**
- label

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Store stats**: Total entries, per-tier counts (Tier1/2/3/Archive), health percentage
- 2. **Top entries**: Scrollable list of highest-confidence entries with: title, confidence, tier, last_accessed, type label
- 3. **Decay visualization**: Time-since-access color gradient (green=fresh, yellow=aging, red=decaying)
- 4. **Distillation events**: Recent distillation timeline (when knowledge was summarized/compressed)
- - [ ] Store stats render (entry counts, tier distribution)
- - [ ] Top entries scrollable with confidence and tier info
- - [ ] Decay visualization uses color gradients

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `Tier1/2/3/`
- `bardo/apps/bardo-terminal/src/screens/knowledge.rs`
- `crates/roko-cli/src/tui/views/`
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
rg -n "Knowledge|Inspect|tab|tier|label|entries|browser|confidence" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|Inspect|tab|tier|label|entries|browser|confidence" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `Tier1/2/3/`
- `bardo/apps/bardo-terminal/src/screens/knowledge.rs`
- `crates/roko-cli/src/tui/views/`
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
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `label` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S006 -- Operational infrastructure

**Source section:** `tmp/architecture/21-tui-and-operations.md:91` through `96`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Operational infrastructure

Development tooling and production reliability features.

---
````

**Explicit detail extraction from this section:**

- Section word count: `6`
- Section hash: `654beb6cdc5b61b361cbf541fad643810eb9484ff917bed2044a36d1383097be`

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
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "infrastructure|Operational|tooling|reliability|production|features|Development" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "infrastructure|Operational|tooling|reliability|production|features|Development" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S007 -- 4. Justfile (developer convenience)

**Source section:** `tmp/architecture/21-tui-and-operations.md:97` through `131`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 4. Justfile (developer convenience)

**Source**: `bardo/justfile` (136 lines)
**Target**: `justfile` at repo root

Common development commands:

```just
build         := cargo build --workspace
test          := cargo test --workspace
lint          := cargo clippy --workspace --no-deps -- -D warnings
fmt           := cargo +nightly fmt --all
fmt-check     := cargo +nightly fmt --all -- --check
check         := cargo check --workspace
ci            := fmt-check && lint && test
coverage      := cargo llvm-cov --workspace --html
watch         := cargo watch -x 'check --workspace'
deny          := cargo deny check
doc           := cargo doc --workspace --no-deps
clean         := cargo clean
serve         := cargo run -p roko-cli -- serve
dashboard     := cargo run -p roko-cli -- dashboard
run           := cargo run -p roko-cli --
```

**Acceptance criteria**:
- [ ] `just ci` runs fmt-check + lint + test
- [ ] `just serve` starts the server
- [ ] `just dashboard` starts the TUI
- [ ] All shortcuts work from repo root

**Size**: S (half day)

---
````

**Explicit detail extraction from this section:**

- Section word count: `125`
- Section hash: `bef3990561dbe9f2134c1264c4f9511052bb2ea1c7f4a3a1ea6f1f16d2c2cb2c`

**Normative requirements and implementation claims:**
- **Source**: `bardo/justfile` (136 lines) **Target**: `justfile` at repo root
- ```just build := cargo build --workspace test := cargo test --workspace lint := cargo clippy --workspace --no-deps -- -D warnings fmt := cargo +nightly fmt --all fmt-check := cargo +nightly fmt --all -- --check check := cargo check --workspace ci := fmt-check && lint && test coverage := cargo llvm-cov --workspace --html watch := cargo watch -x 'check --workspace' deny := cargo deny check doc := cargo doc --workspace --no-deps clean := cargo clean serve := cargo run -p roko-cli -- serve dashboard := cargo run -p roko-cli -- dashboard run := cargo run -p roko-cli -- ```
- **Acceptance criteria**: - [ ] `just ci` runs fmt-check + lint + test - [ ] `just serve` starts the server - [ ] `just dashboard` starts the TUI - [ ] All shortcuts work from repo root
- **Size**: S (half day)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- justfile

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- just
build         := cargo build --workspace
test          := cargo test --workspace
lint          := cargo clippy --workspace --no-deps -- -D warnings
fmt           := cargo +nightly fmt --all
fmt-check     := cargo +nightly fmt --all -- --check
check         := cargo check --workspace
ci            := fmt-check && lint && test
coverage      := cargo llvm-cov --workspace --html
watch         := cargo watch -x 'check --workspace'
deny          := cargo deny check
doc           := cargo doc --workspace --no-deps
clean         := cargo clean
serve         := cargo run -p roko-cli -- serve
dashboard     := cargo run -p roko-cli -- dashboard
run           := cargo run -p roko-cli --

- just ci
- just serve
- just dashboard

**Bullet requirements:**
- - [ ] `just ci` runs fmt-check + lint + test
- - [ ] `just serve` starts the server
- - [ ] `just dashboard` starts the TUI
- - [ ] All shortcuts work from repo root

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "cargo|just|work|check|workspace|Justfile|developer|convenience" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cargo|just|work|check|workspace|Justfile|developer|convenience" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- [ ] Implement or verify `justfile` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify operator command `just
build         := cargo build --workspace
test          := cargo test --workspace
lint          := cargo clippy --workspace --no-deps -- -D warnings
fmt           := cargo +nightly fmt --all
fmt-check     := cargo +nightly fmt --all -- --check
check         := cargo check --workspace
ci            := fmt-check && lint && test
coverage      := cargo llvm-cov --workspace --html
watch         := cargo watch -x 'check --workspace'
deny          := cargo deny check
doc           := cargo doc --workspace --no-deps
clean         := cargo clean
serve         := cargo run -p roko-cli -- serve
dashboard     := cargo run -p roko-cli -- dashboard
run           := cargo run -p roko-cli --
` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `just ci` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `just serve` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `just dashboard` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S008 -- 5. E2E test harness

**Source section:** `tmp/architecture/21-tui-and-operations.md:132` through `155`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 5. E2E test harness

**Source**: `bardo/tests/harness/src/lib.rs` — BardoTestHarness, HealthReport, TerminalProbe
**Target**: `tests/harness/`

Multi-component integration test framework:

1. **RokoTestHarness** struct: Manages spawning roko-serve + mirage-rs as child processes
2. **spawn_serve(config) -> ServerHandle**: Start roko-serve on random port, wait for health check
3. **spawn_mirage(config) -> MirageHandle**: Start mirage-rs on random port
4. **health_check(url) -> HealthReport**: Poll `/api/health` until ready or timeout (30s)
5. **cleanup()**: Kill all child processes on Drop (no leaked processes)
6. Add to workspace as `[dev-dependencies]` for integration tests

**Acceptance criteria**:
- [ ] `RokoTestHarness::new()` spawns serve + mirage
- [ ] Health check waits up to 30s for services to be ready
- [ ] Drop impl kills all child processes
- [ ] Integration test using harness passes: spawn → health check → stop

**Size**: M (2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `124`
- Section hash: `5359eb1b94d1303cc89515d14dcf6d2fcae1c6ea2e6da7551020b2d92468fca9`

**Normative requirements and implementation claims:**
- **Source**: `bardo/tests/harness/src/lib.rs` — BardoTestHarness, HealthReport, TerminalProbe **Target**: `tests/harness/`
- Multi-component integration test framework:
- 1. **RokoTestHarness** struct: Manages spawning roko-serve + mirage-rs as child processes 2. **spawn_serve(config) -> ServerHandle**: Start roko-serve on random port, wait for health check 3. **spawn_mirage(config) -> MirageHandle**: Start mirage-rs on random port 4. **health_check(url) -> HealthReport**: Poll `/api/health` until ready or timeout (30s) 5. **cleanup()**: Kill all child processes on Drop (no leaked processes) 6. Add to workspace as `[dev-dependencies]` for integration tests
- **Acceptance criteria**: - [ ] `RokoTestHarness::new()` spawns serve + mirage - [ ] Health check waits up to 30s for services to be ready - [ ] Drop impl kills all child processes - [ ] Integration test using harness passes: spawn → health check → stop
- **Size**: M (2 days)
- ---

**Routes and endpoint references:**
- /api/health

**Files and path references:**
- bardo/tests/harness/src/lib.rs
- tests/harness/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- spawn -> health check

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **RokoTestHarness** struct: Manages spawning roko-serve + mirage-rs as child processes
- 2. **spawn_serve(config) -> ServerHandle**: Start roko-serve on random port, wait for health check
- 3. **spawn_mirage(config) -> MirageHandle**: Start mirage-rs on random port
- 4. **health_check(url) -> HealthReport**: Poll `/api/health` until ready or timeout (30s)
- 5. **cleanup()**: Kill all child processes on Drop (no leaked processes)
- 6. Add to workspace as `[dev-dependencies]` for integration tests
- - [ ] `RokoTestHarness::new()` spawns serve + mirage
- - [ ] Health check waits up to 30s for services to be ready
- - [ ] Drop impl kills all child processes
- - [ ] Integration test using harness passes: spawn → health check → stop

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/tests/harness/src/lib.rs`
- `tests/harness/`
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
rg -n "test|harness|health|spawn|serve|mirage|E2E|processes" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "test|harness|health|spawn|serve|mirage|E2E|processes" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/tests/harness/src/lib.rs`
- `tests/harness/`
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
- [ ] Implement or verify route `/api/health` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Enforce state transition `spawn -> health check` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S009 -- 6. Self-healing supervisor script

**Source section:** `tmp/architecture/21-tui-and-operations.md:156` through `180`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 6. Self-healing supervisor script

**Source**: `bardo/bardo-supervisor.sh` (381 LOC)
**Target**: `scripts/roko-supervisor.sh`

Production crash recovery:

1. **Crash detection**: Monitor roko process exit code. On non-zero exit, extract panic signature from stderr.
2. **Error deduplication**: Track error signatures in `/tmp/roko-supervisor-errors.json`. Skip auto-fix for already-seen errors.
3. **Auto-fix** (optional, requires Claude CLI): Feed crash report + recent logs to Claude for diagnosis. Apply suggested fix. Restart.
4. **Circuit breaker**: After 3 consecutive restarts within 5 minutes, stop trying. Alert via stderr.
5. **Signal handling**: Forward SIGTERM/SIGINT to child process. Clean shutdown.
6. **Configurable**: `ROKO_SUPERVISOR_MAX_RESTARTS=3`, `ROKO_SUPERVISOR_WINDOW_SECS=300`, `ROKO_SUPERVISOR_AUTOFIX=false`

**Acceptance criteria**:
- [ ] Script restarts roko on crash
- [ ] Error signatures deduplicated
- [ ] Circuit breaker stops after N restarts in window
- [ ] SIGTERM forwarded to child process
- [ ] Works without Claude CLI (autofix disabled by default)

**Size**: M (1-2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `141`
- Section hash: `9c7cdb8d203bb586a4b87be60a0a2fc9f43261e7c7d7d161dce9af17e5856c45`

**Normative requirements and implementation claims:**
- **Source**: `bardo/bardo-supervisor.sh` (381 LOC) **Target**: `scripts/roko-supervisor.sh`
- 1. **Crash detection**: Monitor roko process exit code. On non-zero exit, extract panic signature from stderr. 2. **Error deduplication**: Track error signatures in `/tmp/roko-supervisor-errors.json`. Skip auto-fix for already-seen errors. 3. **Auto-fix** (optional, requires Claude CLI): Feed crash report + recent logs to Claude for diagnosis. Apply suggested fix. Restart. 4. **Circuit breaker**: After 3 consecutive restarts within 5 minutes, stop trying. Alert via stderr. 5. **Signal handling**: Forward SIGTERM/SIGINT to child process. Clean shutdown. 6. **Configurable**: `ROKO_SUPERVISOR_MAX_RESTARTS=3`, `ROKO_SUPERVISOR_WINDOW_SECS=300`, `ROKO_SUPERVISOR_AUTOFIX=false`
- **Acceptance criteria**: - [ ] Script restarts roko on crash - [ ] Error signatures deduplicated - [ ] Circuit breaker stops after N restarts in window - [ ] SIGTERM forwarded to child process - [ ] Works without Claude CLI (autofix disabled by default)
- **Size**: M (1-2 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/bardo-supervisor.sh
- scripts/roko-supervisor.sh
- tmp/roko-supervisor-errors.json

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
- 1. **Crash detection**: Monitor roko process exit code. On non-zero exit, extract panic signature from stderr.
- 2. **Error deduplication**: Track error signatures in `/tmp/roko-supervisor-errors.json`. Skip auto-fix for already-seen errors.
- 3. **Auto-fix** (optional, requires Claude CLI): Feed crash report + recent logs to Claude for diagnosis. Apply suggested fix. Restart.
- 4. **Circuit breaker**: After 3 consecutive restarts within 5 minutes, stop trying. Alert via stderr.
- 5. **Signal handling**: Forward SIGTERM/SIGINT to child process. Clean shutdown.
- 6. **Configurable**: `ROKO_SUPERVISOR_MAX_RESTARTS=3`, `ROKO_SUPERVISOR_WINDOW_SECS=300`, `ROKO_SUPERVISOR_AUTOFIX=false`
- - [ ] Script restarts roko on crash
- - [ ] Error signatures deduplicated
- - [ ] Circuit breaker stops after N restarts in window
- - [ ] SIGTERM forwarded to child process
- - [ ] Works without Claude CLI (autofix disabled by default)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/bardo-supervisor.sh`
- `scripts/roko-supervisor.sh`
- `tmp/roko-supervisor-errors.json`
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
rg -n "supervisor|script|healing|Self|Restart|Error|restarts|crash" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "supervisor|script|healing|Self|Restart|Error|restarts|crash" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `bardo/bardo-supervisor.sh`
- `scripts/roko-supervisor.sh`
- `tmp/roko-supervisor-errors.json`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S010 -- Conductor watcher configuration (added 2026-04-25)

**Source section:** `tmp/architecture/21-tui-and-operations.md:181` through `205`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Conductor watcher configuration (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase OG and `20-orchestrator-gaps.md` spec clarifications.

All 10 conductor watchers are implemented in `crates/roko-conductor/src/watchers/`. Their thresholds are configurable via `[conductor]` in `roko.toml`:

```toml
[conductor]
# Watcher thresholds (all have sensible defaults)
ghost_turn_max_secs = 5           # GhostTurn: no output + fast turn
review_loop_max_consecutive = 3   # ReviewLoop: consecutive REVISE verdicts
iteration_loop_max = 6            # IterationLoop: cycling strategist/implementer
test_failure_budget_pass_rate = 0.70  # TestFailureBudget: force advance threshold
silence_timeout_secs = 180        # SilenceTimeout: no output
compile_fail_max_consecutive = 3  # CompileFailThreshold: consecutive failures
task_stall_secs = 300             # TaskStall: single task blocking
context_pressure_percent = 80     # ContextPressure: prompt >80% of window
phase_timeout_secs = 1800         # PhaseTimeout: 30min wall-clock
cooldown_filter_secs = 120        # CooldownFilter: debounce interval
```

Missing keys use the hardcoded defaults from the watchers table in `20-orchestrator-gaps.md`.

---
````

**Explicit detail extraction from this section:**

- Section word count: `122`
- Section hash: `4041b0104152aa0d296314b04d8289d64aae0636edf5eddcc824d77cdcf31fc2`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/roko-conductor/src/watchers/
- tmp/architecture-plans/06-architecture-implementation.md

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml
- [conductor]
- ghost_turn_max_secs = 5           # GhostTurn: no output + fast turn
- review_loop_max_consecutive = 3   # ReviewLoop: consecutive REVISE verdicts
- iteration_loop_max = 6            # IterationLoop: cycling strategist/implementer
- test_failure_budget_pass_rate = 0.70  # TestFailureBudget: force advance threshold
- silence_timeout_secs = 180        # SilenceTimeout: no output
- compile_fail_max_consecutive = 3  # CompileFailThreshold: consecutive failures
- task_stall_secs = 300             # TaskStall: single task blocking
- context_pressure_percent = 80     # ContextPressure: prompt >80% of window
- phase_timeout_secs = 1800         # PhaseTimeout: 30min wall-clock
- cooldown_filter_secs = 120        # CooldownFilter: debounce interval

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `[conductor]`

```toml
[conductor]
# Watcher thresholds (all have sensible defaults)
ghost_turn_max_secs = 5           # GhostTurn: no output + fast turn
review_loop_max_consecutive = 3   # ReviewLoop: consecutive REVISE verdicts
iteration_loop_max = 6            # IterationLoop: cycling strategist/implementer
test_failure_budget_pass_rate = 0.70  # TestFailureBudget: force advance threshold
silence_timeout_secs = 180        # SilenceTimeout: no output
compile_fail_max_consecutive = 3  # CompileFailThreshold: consecutive failures
task_stall_secs = 300             # TaskStall: single task blocking
context_pressure_percent = 80     # ContextPressure: prompt >80% of window
phase_timeout_secs = 1800         # PhaseTimeout: 30min wall-clock
cooldown_filter_secs = 120        # CooldownFilter: debounce interval
```

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-conductor/src/watchers/`
- `tmp/architecture-plans/06-architecture-implementation.md`
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
rg -n "watcher|Conductor|configuration|added|threshold|consecutive|watchers|turn" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "watcher|Conductor|configuration|added|threshold|consecutive|watchers|turn" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-conductor/src/watchers/`
- `tmp/architecture-plans/06-architecture-implementation.md`
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
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[conductor]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ghost_turn_max_secs = 5           # GhostTurn: no output + fast turn` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `review_loop_max_consecutive = 3   # ReviewLoop: consecutive REVISE verdicts` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `iteration_loop_max = 6            # IterationLoop: cycling strategist/implementer` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `test_failure_budget_pass_rate = 0.70  # TestFailureBudget: force advance threshold` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `silence_timeout_secs = 180        # SilenceTimeout: no output` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `compile_fail_max_consecutive = 3  # CompileFailThreshold: consecutive failures` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `task_stall_secs = 300             # TaskStall: single task blocking` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `context_pressure_percent = 80     # ContextPressure: prompt >80% of window` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `phase_timeout_secs = 1800         # PhaseTimeout: 30min wall-clock` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `cooldown_filter_secs = 120        # CooldownFilter: debounce interval` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S011 -- Implementation state (updated 2026-04-25)

**Source section:** `tmp/architecture/21-tui-and-operations.md:206` through `207`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Implementation state (updated 2026-04-25)
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `73ec4eb9188b46b085de13fa491ab5f6e06559e197f7b00174e453963dec6480`

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
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "updated|state|tui|operations" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "updated|state|tui|operations" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S012 -- What already exists

**Source section:** `tmp/architecture/21-tui-and-operations.md:208` through `218`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What already exists

| Item | Location | Status |
|------|----------|--------|
| TUI with F1-F7 tabs | `roko-cli/src/tui/` | **EXISTS** — ratatui, file watcher, live data |
| DaimonState loaded per-task | `roko-cli/src/orchestrate.rs` | **EXISTS** — PAD vector computed, used in dispatch |
| Efficiency events JSONL | `.roko/learn/efficiency.jsonl` | **EXISTS** — per-turn cost/latency/outcome |
| Episode log JSONL | `.roko/episodes.jsonl` | **EXISTS** — full agent turn records |
| Knowledge store | `roko-neuro/src/knowledge_store.rs` | **EXISTS** — query API for knowledge browser |
| 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — all rules implemented |
````

**Explicit detail extraction from this section:**

- Section word count: `82`
- Section hash: `a54dc84fc132f265241b2671dcfa651cbbd843df32cf7e787d175f4f05024e09`

**Normative requirements and implementation claims:**
- | Item | Location | Status | |------|----------|--------| | TUI with F1-F7 tabs | `roko-cli/src/tui/` | **EXISTS** — ratatui, file watcher, live data | | DaimonState loaded per-task | `roko-cli/src/orchestrate.rs` | **EXISTS** — PAD vector computed, used in dispatch | | Efficiency events JSONL | `.roko/learn/efficiency.jsonl` | **EXISTS** — per-turn cost/latency/outcome | | Episode log JSONL | `.roko/episodes.jsonl` | **EXISTS** — full agent turn records | | Knowledge store | `roko-neuro/src/knowledge_store.rs` | **EXISTS** — query API for knowledge browser | | 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — all rules implemented |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/episodes.json
- .roko/learn/efficiency.json
- cost/latency/
- roko-cli/src/orchestrate.rs
- roko-cli/src/tui/
- roko-conductor/src/watchers/
- roko-neuro/src/knowledge_store.rs

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
| Item | Location | Status |
|------|----------|--------|
| TUI with F1-F7 tabs | `roko-cli/src/tui/` | **EXISTS** — ratatui, file watcher, live data |
| DaimonState loaded per-task | `roko-cli/src/orchestrate.rs` | **EXISTS** — PAD vector computed, used in dispatch |
| Efficiency events JSONL | `.roko/learn/efficiency.jsonl` | **EXISTS** — per-turn cost/latency/outcome |
| Episode log JSONL | `.roko/episodes.jsonl` | **EXISTS** — full agent turn records |
| Knowledge store | `roko-neuro/src/knowledge_store.rs` | **EXISTS** — query API for knowledge browser |
| 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — all rules implemented |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `.roko/episodes.json`
- `.roko/learn/efficiency.json`
- `cost/latency/`
- `roko-cli/src/orchestrate.rs`
- `roko-cli/src/tui/`
- `roko-conductor/src/watchers/`
- `roko-neuro/src/knowledge_store.rs`
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
rg -n "exists|already|jsonl|watcher|tui|Knowledge|watchers|turn" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "exists|already|jsonl|watcher|tui|Knowledge|watchers|turn" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
- `.roko/episodes.json`
- `.roko/learn/efficiency.json`
- `cost/latency/`
- `roko-cli/src/orchestrate.rs`
- `roko-cli/src/tui/`
- `roko-conductor/src/watchers/`
- `roko-neuro/src/knowledge_store.rs`
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
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

### ARCH-21-S013 -- What needs building

**Source section:** `tmp/architecture/21-tui-and-operations.md:219` through `228`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### What needs building

| Task | Status | Notes |
|------|--------|-------|
| DaimonState TUI view | **Missing** | PAD gauges, somatic markers, behavioral bias |
| Heartbeat status in Learning tab | **Missing** | Accuracy sparkline, tier distribution, cost trend |
| Knowledge browser in Inspect tab | **Missing** | Store stats, top entries, decay visualization |
| Justfile | **Missing** | Developer convenience commands |
| E2E test harness | **Missing** | Multi-component integration test framework |
| Self-healing supervisor | **Missing** | Crash recovery, circuit breaker, signal forwarding |
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `8183194e54563c49f8619fa13eec6b0fb48c8b9682ae8d87067a292e869101fd`

**Normative requirements and implementation claims:**
- | Task | Status | Notes | |------|--------|-------| | DaimonState TUI view | **Missing** | PAD gauges, somatic markers, behavioral bias | | Heartbeat status in Learning tab | **Missing** | Accuracy sparkline, tier distribution, cost trend | | Knowledge browser in Inspect tab | **Missing** | Store stats, top entries, decay visualization | | Justfile | **Missing** | Developer convenience commands | | E2E test harness | **Missing** | Multi-component integration test framework | | Self-healing supervisor | **Missing** | Crash recovery, circuit breaker, signal forwarding |

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
| Task | Status | Notes |
|------|--------|-------|
| DaimonState TUI view | **Missing** | PAD gauges, somatic markers, behavioral bias |
| Heartbeat status in Learning tab | **Missing** | Accuracy sparkline, tier distribution, cost trend |
| Knowledge browser in Inspect tab | **Missing** | Store stats, top entries, decay visualization |
| Justfile | **Missing** | Developer convenience commands |
| E2E test harness | **Missing** | Multi-component integration test framework |
| Self-healing supervisor | **Missing** | Crash recovery, circuit breaker, signal forwarding |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/21-tui-and-operations.md`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "Missing|needs|building|test|Status|visualization|view|tui" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Missing|needs|building|test|Status|visualization|view|tui" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/surface_inventory.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/21-tui-and-operations
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

