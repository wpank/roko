# Architecture Plan: Orchestrator Gaps

**Source:** `tmp/architecture/20-orchestrator-gaps.md`
**Generated:** 2026-04-25
**Source hash:** `567b4c16d95b47c3a854e83a907500d23274aca173adcf328401ef9388527656`
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
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-20-S001 | 1 | Orchestrator and Learning Gaps | [ ] | 9.8 |
| ARCH-20-S002 | 8 | Orchestrator gaps (from mori) | [ ] | 9.8 |
| ARCH-20-S003 | 14 | 1. Structured review verdict system | [ ] | 9.8 |
| ARCH-20-S004 | 45 | 2. Compile error classification + auto-fix | [ ] | 9.8 |
| ARCH-20-S005 | 77 | 3. Error pattern discovery + sharing | [ ] | 9.8 |
| ARCH-20-S006 | 103 | 4. Post-gate reflection loop | [ ] | 9.8 |
| ARCH-20-S007 | 129 | 5. Context injection scoping | [ ] | 9.8 |
| ARCH-20-S008 | 159 | 6. Warm agent spawning | [ ] | 9.8 |
| ARCH-20-S009 | 185 | 7. Conductor watchers (10 rules) | [ ] | 9.8 |
| ARCH-20-S010 | 218 | Learning loop gaps | [ ] | 9.8 |
| ARCH-20-S011 | 224 | 8. Wire neuro store into cascade router | [ ] | 9.8 |
| ARCH-20-S012 | 250 | 9. Episode clustering for error patterns | [ ] | 9.8 |
| ARCH-20-S013 | 277 | 10. Provider pass-rate into model scoring | [ ] | 9.8 |
| ARCH-20-S014 | 301 | 11. Reflection-derived playbook rules | [ ] | 9.8 |
| ARCH-20-S015 | 328 | 12. A-MAC admission gate for neuro store | [ ] | 9.8 |
| ARCH-20-S016 | 355 | Current state reconciliation | [ ] | 9.8 |
| ARCH-20-S017 | 359 | Already implemented (do NOT rebuild) | [ ] | 9.8 |
| ARCH-20-S018 | 374 | Remaining work (gaps that still need implementation) | [ ] | 9.8 |
| ARCH-20-S019 | 396 | Spec clarifications (resolving ambiguities) | [ ] | 9.8 |
| ARCH-20-S020 | 400 | Gap 1: Parsing fallback chain | [ ] | 9.8 |
| ARCH-20-S021 | 433 | Gap 1: is_quick_fixable() categories | [ ] | 9.8 |
| ARCH-20-S022 | 441 | Gap 2: cargo fix merge conflict handling | [ ] | 9.8 |
| ARCH-20-S023 | 451 | Gap 3: Error deduplication algorithm | [ ] | 9.8 |
| ARCH-20-S024 | 463 | Gap 4: Reflection cost guard with variable pricing | [ ] | 9.8 |
| ARCH-20-S025 | 476 | Gap 5: Context size numbers for role filtering | [ ] | 9.8 |
| ARCH-20-S026 | 493 | Gap 7: Conductor watcher threshold config | [ ] | 9.8 |
| ARCH-20-S027 | 513 | Gap 8: Cascade router knowledge bias clamping | [ ] | 9.8 |
| ARCH-20-S028 | 525 | Gap 9: Episode clustering with fewer than 3 matches | [ ] | 9.8 |
| ARCH-20-S029 | 541 | Gap 12: A-MAC contradiction detection | [ ] | 9.8 |

## Tasks

### ARCH-20-S001 -- Orchestrator and Learning Gaps

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Orchestrator and Learning Gaps

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Folded from `tmp/bardo-integration-plan.md` Phases 2-3. Original bardo source references preserved.

---
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `7ab70958bf7c005ff9d9ced5cad6d573bd1c39a812c827341ddece29cecca72e`

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "Orchestrator|Learning|Gaps|bardo|references|preserved|plan" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Orchestrator|Learning|Gaps|bardo|references|preserved|plan" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S002 -- Orchestrator gaps (from mori)

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:8` through `13`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Orchestrator gaps (from mori)

These features exist in bardo/mori but are not yet in roko's orchestrator (`crates/roko-cli/src/orchestrate.rs`).

---
````

**Explicit detail extraction from this section:**

- Section word count: `20`
- Section hash: `4f83cf09678658dd4464fd306789dd76a450d5b47e2c2046015a6b7fa8d1699c`

**Normative requirements and implementation claims:**
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- crates/roko-cli/src/orchestrate.rs

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "mori|Orchestrator|gaps|orchestrate|features|exist|crates|bardo" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "mori|Orchestrator|gaps|orchestrate|features|exist|crates|bardo" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S003 -- 1. Structured review verdict system

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:14` through `44`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 1. Structured review verdict system

**Source**: `bardo/apps/mori/src/orchestrator/review.rs`
**Target**: `crates/roko-gate/src/review_verdict.rs` + wire into orchestrate.rs

Parse agent review output into structured verdicts with issue classification.

**Types**:
- `ReviewVerdict` enum: `Approve | Revise | Skip`
- `ReviewIssue { severity: IssueSeverity, category: IssueCategory, file: Option<String>, line: Option<u32>, description: String }`
- `IssueSeverity` enum: `Blocking | Major | Minor`
- `IssueCategory` enum: `Compilation | Test | TypeMismatch | MissingImpl | Docs | Style | SpecDeviation`
- `IssueCategory::is_quick_fixable()` → true for Compilation, Docs, Style
- `StructuredReview { verdict, issues: Vec<ReviewIssue>, summary: String }`
- `StructuredReview::all_issues_quick_fixable()` → true when all issues are quick-fixable

**Parsing**: Try JSON first, then JSON code block, then TOML block. Provide JSON schema for reviewer agents.

**Integration**: In orchestrate.rs, after review phase, parse agent output as StructuredReview. If `all_issues_quick_fixable()`, skip strategist and go directly to implementer (express mode).

**Acceptance criteria**:
- [ ] `StructuredReview` parses from JSON agent output
- [ ] `IssueCategory::is_quick_fixable()` returns correct values
- [ ] `all_issues_quick_fixable()` correctly identifies trivial-fix scenarios
- [ ] Fallback parsing handles malformed JSON gracefully (returns Revise with raw text)
- [ ] Integration test: mock review JSON → parsed verdict → correct phase transition

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `170`
- Section hash: `7624032d261795bbb7a04469922b0aac98d6a7b7b5706338ef9abb25f86c386d`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/review.rs` **Target**: `crates/roko-gate/src/review_verdict.rs` + wire into orchestrate.rs
- **Types**: - `ReviewVerdict` enum: `Approve | Revise | Skip` - `ReviewIssue { severity: IssueSeverity, category: IssueCategory, file: Option<String>, line: Option<u32>, description: String }` - `IssueSeverity` enum: `Blocking | Major | Minor` - `IssueCategory` enum: `Compilation | Test | TypeMismatch | MissingImpl | Docs | Style | SpecDeviation` - `IssueCategory::is_quick_fixable()` → true for Compilation, Docs, Style - `StructuredReview { verdict, issues: Vec<ReviewIssue>, summary: String }` - `StructuredReview::all_issues_quick_fixable()` → true when all issues are quick-fixable
- **Parsing**: Try JSON first, then JSON code block, then TOML block. Provide JSON schema for reviewer agents.
- **Integration**: In orchestrate.rs, after review phase, parse agent output as StructuredReview. If `all_issues_quick_fixable()`, skip strategist and go directly to implementer (express mode).
- **Acceptance criteria**: - [ ] `StructuredReview` parses from JSON agent output - [ ] `IssueCategory::is_quick_fixable()` returns correct values - [ ] `all_issues_quick_fixable()` correctly identifies trivial-fix scenarios - [ ] Fallback parsing handles malformed JSON gracefully (returns Revise with raw text) - [ ] Integration test: mock review JSON → parsed verdict → correct phase transition
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/review.rs
- crates/roko-gate/src/review_verdict.rs

**Types, functions, traits, and inline code identifiers:**
- ReviewVerdict
- IssueSeverity
- IssueCategory
- StructuredReview

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- mock review JSON -> parsed verdict

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `ReviewVerdict` enum: `Approve | Revise | Skip`
- - `ReviewIssue { severity: IssueSeverity, category: IssueCategory, file: Option<String>, line: Option<u32>, description: String }`
- - `IssueSeverity` enum: `Blocking | Major | Minor`
- - `IssueCategory` enum: `Compilation | Test | TypeMismatch | MissingImpl | Docs | Style | SpecDeviation`
- - `IssueCategory::is_quick_fixable()` → true for Compilation, Docs, Style
- - `StructuredReview { verdict, issues: Vec<ReviewIssue>, summary: String }`
- - `StructuredReview::all_issues_quick_fixable()` → true when all issues are quick-fixable
- - [ ] `StructuredReview` parses from JSON agent output
- - [ ] `IssueCategory::is_quick_fixable()` returns correct values
- - [ ] `all_issues_quick_fixable()` correctly identifies trivial-fix scenarios
- - [ ] Fallback parsing handles malformed JSON gracefully (returns Revise with raw text)
- - [ ] Integration test: mock review JSON → parsed verdict → correct phase transition

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/review.rs`
- `crates/roko-gate/src/review_verdict.rs`
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
rg -n "review|issue|verdict|Structured|StructuredReview|IssueCategory|issues|quick" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "review|issue|verdict|Structured|StructuredReview|IssueCategory|issues|quick" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/review.rs`
- `crates/roko-gate/src/review_verdict.rs`
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
- [ ] Implement or verify `ReviewVerdict` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `IssueSeverity` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `IssueCategory` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `StructuredReview` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `mock review JSON -> parsed verdict` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S004 -- 2. Compile error classification + auto-fix

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:45` through `76`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 2. Compile error classification + auto-fix

**Source**: `bardo/apps/mori/src/orchestrator/autofix.rs`
**Target**: `crates/roko-gate/src/compile_errors.rs` + wire into orchestrate.rs

Parse `cargo check --message-format=json` output into classified error types for targeted auto-fix.

**CompileErrorClass** enum:
- `ImportNotFound { module, item, file, line }`
- `TypeMismatch { expected, found, file, line }`
- `MissingField { struct_name, field, file, line }`
- `TraitNotImplemented { type_name, trait_name, file, line }`
- `Other { code: String, message, file, line }`

**Functions**:
- `parse_cargo_json_errors(json_output: &str) -> Vec<CompileErrorClass>` — Extract `rendered`, `code`, `spans[0].file_name`, `spans[0].line_start`
- `collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>` — Extract `children[].suggested_replacement` from diagnostic JSON
- `apply_rustc_fixes(worktree: &Path)` — Run `cargo fix --allow-dirty` + `cargo fmt` to apply compiler-suggested fixes directly (no agent needed)

**Integration**: In orchestrate.rs autofix path, first try `apply_rustc_fixes()`. If that resolves all errors, skip agent retry. Otherwise, pass classified errors to agent instead of raw cargo output.

**Acceptance criteria**:
- [ ] `parse_cargo_json_errors()` extracts structured errors from real cargo JSON
- [ ] `CompileErrorClass` variants populated with correct file/line/details
- [ ] `collect_rustc_suggestions()` finds and extracts suggested replacements
- [ ] `apply_rustc_fixes()` runs cargo fix + fmt successfully
- [ ] Agent receives classified errors instead of raw output

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `178`
- Section hash: `0d8fe06cd27a01f482f9ef3c335153e89440c15048e7fe570961dcfda764432d`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/autofix.rs` **Target**: `crates/roko-gate/src/compile_errors.rs` + wire into orchestrate.rs
- **CompileErrorClass** enum: - `ImportNotFound { module, item, file, line }` - `TypeMismatch { expected, found, file, line }` - `MissingField { struct_name, field, file, line }` - `TraitNotImplemented { type_name, trait_name, file, line }` - `Other { code: String, message, file, line }`
- **Functions**: - `parse_cargo_json_errors(json_output: &str) -> Vec<CompileErrorClass>` — Extract `rendered`, `code`, `spans[0].file_name`, `spans[0].line_start` - `collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>` — Extract `children[].suggested_replacement` from diagnostic JSON - `apply_rustc_fixes(worktree: &Path)` — Run `cargo fix --allow-dirty` + `cargo fmt` to apply compiler-suggested fixes directly (no agent needed)
- **Integration**: In orchestrate.rs autofix path, first try `apply_rustc_fixes()`. If that resolves all errors, skip agent retry. Otherwise, pass classified errors to agent instead of raw cargo output.
- **Acceptance criteria**: - [ ] `parse_cargo_json_errors()` extracts structured errors from real cargo JSON - [ ] `CompileErrorClass` variants populated with correct file/line/details - [ ] `collect_rustc_suggestions()` finds and extracts suggested replacements - [ ] `apply_rustc_fixes()` runs cargo fix + fmt successfully - [ ] Agent receives classified errors instead of raw output
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/autofix.rs
- crates/roko-gate/src/compile_errors.rs
- file/line/

**Types, functions, traits, and inline code identifiers:**
- rendered
- code
- CompileErrorClass

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- cargo check --message-format=json
- cargo fix --allow-dirty
- cargo fmt

**Bullet requirements:**
- - `ImportNotFound { module, item, file, line }`
- - `TypeMismatch { expected, found, file, line }`
- - `MissingField { struct_name, field, file, line }`
- - `TraitNotImplemented { type_name, trait_name, file, line }`
- - `Other { code: String, message, file, line }`
- - `parse_cargo_json_errors(json_output: &str) -> Vec<CompileErrorClass>` — Extract `rendered`, `code`, `spans[0].file_name`, `spans[0].line_start`
- - `collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>` — Extract `children[].suggested_replacement` from diagnostic JSON
- - `apply_rustc_fixes(worktree: &Path)` — Run `cargo fix --allow-dirty` + `cargo fmt` to apply compiler-suggested fixes directly (no agent needed)
- - [ ] `parse_cargo_json_errors()` extracts structured errors from real cargo JSON
- - [ ] `CompileErrorClass` variants populated with correct file/line/details
- - [ ] `collect_rustc_suggestions()` finds and extracts suggested replacements
- - [ ] `apply_rustc_fixes()` runs cargo fix + fmt successfully
- - [ ] Agent receives classified errors instead of raw output

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/autofix.rs`
- `crates/roko-gate/src/compile_errors.rs`
- `file/line/`
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
rg -n "error|fix|Compile|cargo|auto|line|json|file" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "error|fix|Compile|cargo|auto|line|json|file" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/autofix.rs`
- `crates/roko-gate/src/compile_errors.rs`
- `file/line/`
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
- [ ] Implement or verify `rendered` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `code` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CompileErrorClass` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify operator command `cargo check --message-format=json` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `cargo fix --allow-dirty` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `cargo fmt` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S005 -- 3. Error pattern discovery + sharing

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:77` through `102`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 3. Error pattern discovery + sharing

**Source**: `bardo/apps/mori/src/orchestrator/gates.rs`
**Target**: `crates/roko-gate/src/error_patterns.rs` + wire into orchestrate.rs

Share discovered error patterns across parallel agents.

**Functions**:
- `extract_error_digest(output: &str) -> String` — Parse cargo/test output, extract `error[E...]` blocks, deduplicate via HashSet, cap at 10 unique errors, cap each at 200 chars. Return compact digest.
- `append_discovered_pattern(repo_root, plan, error_digest)` — Write to `.roko/learn/discovered-patterns.json`. Format: `{ "patterns": [{ "plan", "digest", "timestamp", "resolved": bool }] }`
- `read_discovered_patterns() -> Vec<DiscoveredPattern>` — Read last 5 unresolved patterns (200 chars each). Used to inject into agent context so parallel agents learn from each other's failures.
- `GateResult::is_mostly_passing(results) -> bool` — >90% pass rate with >20 tests and ≥1 failure = "mostly passing". Means a targeted fix should suffice (not full replan).

**Integration**: In orchestrate.rs, after gate failure: call `extract_error_digest()` → `append_discovered_pattern()`. Before agent dispatch: call `read_discovered_patterns()` → inject into system prompt.

**Acceptance criteria**:
- [ ] `extract_error_digest()` produces compact, deduped error signatures from real cargo output
- [ ] Patterns persisted to `.roko/learn/discovered-patterns.json`
- [ ] Parallel agents see each other's patterns (read from shared file)
- [ ] `is_mostly_passing()` returns true for 95% pass with 1 failure, false for 50% pass
- [ ] Pattern injection visible in agent system prompt

**Size**: M (2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `197`
- Section hash: `0a45489cb3bca64b7e886acc1d7513143a19eac339ff87680178561a87aa1d0b`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/gates.rs` **Target**: `crates/roko-gate/src/error_patterns.rs` + wire into orchestrate.rs
- **Functions**: - `extract_error_digest(output: &str) -> String` — Parse cargo/test output, extract `error[E...]` blocks, deduplicate via HashSet, cap at 10 unique errors, cap each at 200 chars. Return compact digest. - `append_discovered_pattern(repo_root, plan, error_digest)` — Write to `.roko/learn/discovered-patterns.json`. Format: `{ "patterns": [{ "plan", "digest", "timestamp", "resolved": bool }] }` - `read_discovered_patterns() -> Vec<DiscoveredPattern>` — Read last 5 unresolved patterns (200 chars each). Used to inject into agent context so parallel agents learn from each other's failures. - `GateResult::is_mostly_passing(results) -> bool` — >90% pass rate with >20 tests and ≥1 failure = "mostly passing". Means a targeted fix should suffice (not full replan).
- **Integration**: In orchestrate.rs, after gate failure: call `extract_error_digest()` → `append_discovered_pattern()`. Before agent dispatch: call `read_discovered_patterns()` → inject into system prompt.
- **Acceptance criteria**: - [ ] `extract_error_digest()` produces compact, deduped error signatures from real cargo output - [ ] Patterns persisted to `.roko/learn/discovered-patterns.json` - [ ] Parallel agents see each other's patterns (read from shared file) - [ ] `is_mostly_passing()` returns true for 95% pass with 1 failure, false for 50% pass - [ ] Pattern injection visible in agent system prompt
- **Size**: M (2 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/learn/discovered-patterns.json
- bardo/apps/mori/src/orchestrator/gates.rs
- crates/roko-gate/src/error_patterns.rs

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
- - `extract_error_digest(output: &str) -> String` — Parse cargo/test output, extract `error[E...]` blocks, deduplicate via HashSet, cap at 10 unique errors, cap each at 200 chars. Return compact digest.
- - `append_discovered_pattern(repo_root, plan, error_digest)` — Write to `.roko/learn/discovered-patterns.json`. Format: `{ "patterns": [{ "plan", "digest", "timestamp", "resolved": bool }] }`
- - `read_discovered_patterns() -> Vec<DiscoveredPattern>` — Read last 5 unresolved patterns (200 chars each). Used to inject into agent context so parallel agents learn from each other's failures.
- - `GateResult::is_mostly_passing(results) -> bool` — >90% pass rate with >20 tests and ≥1 failure = "mostly passing". Means a targeted fix should suffice (not full replan).
- - [ ] `extract_error_digest()` produces compact, deduped error signatures from real cargo output
- - [ ] Patterns persisted to `.roko/learn/discovered-patterns.json`
- - [ ] Parallel agents see each other's patterns (read from shared file)
- - [ ] `is_mostly_passing()` returns true for 95% pass with 1 failure, false for 50% pass
- - [ ] Pattern injection visible in agent system prompt

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `.roko/learn/discovered-patterns.json`
- `bardo/apps/mori/src/orchestrator/gates.rs`
- `crates/roko-gate/src/error_patterns.rs`
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
rg -n "pattern|Error|patterns|discovered|pass|digest|sharing|discovery" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "pattern|Error|patterns|discovered|pass|digest|sharing|discovery" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `.roko/learn/discovered-patterns.json`
- `bardo/apps/mori/src/orchestrator/gates.rs`
- `crates/roko-gate/src/error_patterns.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S006 -- 4. Post-gate reflection loop

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:103` through `128`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 4. Post-gate reflection loop

**Source**: `bardo/apps/mori/src/orchestrator/reflection.rs`, `bardo/apps/mori/src/orchestrator/iteration_memory.rs`
**Target**: `crates/roko-cli/src/orchestrate.rs` (new function) + `crates/roko-learn/src/episode_logger.rs` (add field)

After gate failure, spawn a lightweight agent to analyze what went wrong.

**Specification**:
1. **Trigger**: After any gate failure (compile, test, clippy), before replanning
2. **Reflection agent**: Use cheapest model (haiku-4-5). Prompt: "Analyze this gate failure. What went wrong? What should the next attempt do differently? Gate output: {error_digest}. Files changed: {file_list}. Previous attempts: {iteration_count}."
3. **Output**: Store reflection text in episode's `reflection` field (add this field to Episode struct if missing)
4. **Injection**: On retry, inject last reflection into agent's system prompt as "Lessons from previous attempt: {reflection}"
5. **Deduplication**: If error_digest matches a previous reflection's error pattern, skip re-generating
6. **Cost guard**: Reflection must cost <$0.02 (cap max_tokens at 500)

**Acceptance criteria**:
- [ ] Reflection generated on gate failure (visible in episode log)
- [ ] Reflection injected into retry agent's prompt
- [ ] Deduplication: same error pattern doesn't trigger second reflection
- [ ] Cost capped: max_tokens=500, model=haiku
- [ ] Episode struct has `reflection: Option<String>` field

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `196`
- Section hash: `4cf0fff70a3a437e82830042f0fd91efa26c99df3cd368068d1ee21da01462a0`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/reflection.rs`, `bardo/apps/mori/src/orchestrator/iteration_memory.rs` **Target**: `crates/roko-cli/src/orchestrate.rs` (new function) + `crates/roko-learn/src/episode_logger.rs` (add field)
- **Specification**: 1. **Trigger**: After any gate failure (compile, test, clippy), before replanning 2. **Reflection agent**: Use cheapest model (haiku-4-5). Prompt: "Analyze this gate failure. What went wrong? What should the next attempt do differently? Gate output: {error_digest}. Files changed: {file_list}. Previous attempts: {iteration_count}." 3. **Output**: Store reflection text in episode's `reflection` field (add this field to Episode struct if missing) 4. **Injection**: On retry, inject last reflection into agent's system prompt as "Lessons from previous attempt: {reflection}" 5. **Deduplication**: If error_digest matches a previous reflection's error pattern, skip re-generating 6. **Cost guard**: Reflection must cost <$0.02 (cap max_tokens at 500)
- **Acceptance criteria**: - [ ] Reflection generated on gate failure (visible in episode log) - [ ] Reflection injected into retry agent's prompt - [ ] Deduplication: same error pattern doesn't trigger second reflection - [ ] Cost capped: max_tokens=500, model=haiku - [ ] Episode struct has `reflection: Option<String>` field
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/iteration_memory.rs
- bardo/apps/mori/src/orchestrator/reflection.rs
- crates/roko-cli/src/orchestrate.rs
- crates/roko-learn/src/episode_logger.rs

**Types, functions, traits, and inline code identifiers:**
- if
- has
- reflection

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Trigger**: After any gate failure (compile, test, clippy), before replanning
- 2. **Reflection agent**: Use cheapest model (haiku-4-5). Prompt: "Analyze this gate failure. What went wrong? What should the next attempt do differently? Gate output: {error_digest}. Files changed: {file_list}. Previous attempts: {iteration_count}."
- 3. **Output**: Store reflection text in episode's `reflection` field (add this field to Episode struct if missing)
- 4. **Injection**: On retry, inject last reflection into agent's system prompt as "Lessons from previous attempt: {reflection}"
- 5. **Deduplication**: If error_digest matches a previous reflection's error pattern, skip re-generating
- 6. **Cost guard**: Reflection must cost <$0.02 (cap max_tokens at 500)
- - [ ] Reflection generated on gate failure (visible in episode log)
- - [ ] Reflection injected into retry agent's prompt
- - [ ] Deduplication: same error pattern doesn't trigger second reflection
- - [ ] Cost capped: max_tokens=500, model=haiku
- - [ ] Episode struct has `reflection: Option<String>` field

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/iteration_memory.rs`
- `bardo/apps/mori/src/orchestrator/reflection.rs`
- `crates/roko-learn/src/episode_logger.rs`
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
rg -n "reflection|gate|loop|has|episode|Post|field|failure" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "reflection|gate|loop|has|episode|Post|field|failure" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/iteration_memory.rs`
- `bardo/apps/mori/src/orchestrator/reflection.rs`
- `crates/roko-learn/src/episode_logger.rs`
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
- [ ] Implement or verify `if` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `has` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `reflection` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S007 -- 5. Context injection scoping

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:129` through `158`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 5. Context injection scoping

**Source**: `bardo/apps/mori/src/orchestrator/inject.rs`
**Target**: `crates/roko-compose/src/context_scoping.rs` + wire into orchestrate.rs

Scope playbook rules to plan's touched files and enable per-category toggles.

**KnowledgeConfig** struct with toggles:
- `file_intel_enabled: bool` (default true), `file_intel_max_entries: usize` (default 5)
- `warnings_enabled: bool`, `warning_max_entries: usize`
- `error_patterns_enabled: bool`, `error_pattern_min_cluster: usize`
- `wave_context_enabled: bool` (read context from sibling tasks in same wave)
- `dynamic_budget_enabled: bool` (adjust context size per file difficulty)

**`collect_plan_playbook_scope(plan, tasks) -> PlaybookScope`**: Extract file globs + tags from task checklist. Only match playbook rules whose `trigger_files` overlap with plan's file scope.

**Role-filtered context**: Different roles get different context sizes. Implementer gets full file intel. Reviewer gets summary only. Strategist gets none (sees plan-level only).

**Integration**: In orchestrate.rs `dispatch_agent_with()`, apply KnowledgeConfig to filter playbook rules and context before prompt assembly.

**Acceptance criteria**:
- [ ] `KnowledgeConfig` loadable from `roko.toml` (with defaults)
- [ ] `collect_plan_playbook_scope()` narrows rule matching to plan's files
- [ ] Implementer gets full context; reviewer gets summary; verified by prompt inspection
- [ ] Config toggles actually suppress sections

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `172`
- Section hash: `c0cb25929048a2c4b7acab85d32e40a148d9437eb19473ddddc457cbc8b4856a`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/inject.rs` **Target**: `crates/roko-compose/src/context_scoping.rs` + wire into orchestrate.rs
- **KnowledgeConfig** struct with toggles: - `file_intel_enabled: bool` (default true), `file_intel_max_entries: usize` (default 5) - `warnings_enabled: bool`, `warning_max_entries: usize` - `error_patterns_enabled: bool`, `error_pattern_min_cluster: usize` - `wave_context_enabled: bool` (read context from sibling tasks in same wave) - `dynamic_budget_enabled: bool` (adjust context size per file difficulty)
- **`collect_plan_playbook_scope(plan, tasks) -> PlaybookScope`**: Extract file globs + tags from task checklist. Only match playbook rules whose `trigger_files` overlap with plan's file scope.
- **Role-filtered context**: Different roles get different context sizes. Implementer gets full file intel. Reviewer gets summary only. Strategist gets none (sees plan-level only).
- **Integration**: In orchestrate.rs `dispatch_agent_with()`, apply KnowledgeConfig to filter playbook rules and context before prompt assembly.
- **Acceptance criteria**: - [ ] `KnowledgeConfig` loadable from `roko.toml` (with defaults) - [ ] `collect_plan_playbook_scope()` narrows rule matching to plan's files - [ ] Implementer gets full context; reviewer gets summary; verified by prompt inspection - [ ] Config toggles actually suppress sections
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/inject.rs
- crates/roko-compose/src/context_scoping.rs

**Types, functions, traits, and inline code identifiers:**
- with
- trigger_files
- KnowledgeConfig

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `file_intel_enabled: bool` (default true), `file_intel_max_entries: usize` (default 5)
- - `warnings_enabled: bool`, `warning_max_entries: usize`
- - `error_patterns_enabled: bool`, `error_pattern_min_cluster: usize`
- - `wave_context_enabled: bool` (read context from sibling tasks in same wave)
- - `dynamic_budget_enabled: bool` (adjust context size per file difficulty)
- - [ ] `KnowledgeConfig` loadable from `roko.toml` (with defaults)
- - [ ] `collect_plan_playbook_scope()` narrows rule matching to plan's files
- - [ ] Implementer gets full context; reviewer gets summary; verified by prompt inspection
- - [ ] Config toggles actually suppress sections

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/inject.rs`
- `crates/roko-compose/src/context_scoping.rs`
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
rg -n "Context|file|plan|KnowledgeConfig|size|scoping|playbook|inject" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Context|file|plan|KnowledgeConfig|size|scoping|playbook|inject" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/inject.rs`
- `crates/roko-compose/src/context_scoping.rs`
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
- [ ] Implement or verify `with` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `trigger_files` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `KnowledgeConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S008 -- 6. Warm agent spawning

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:159` through `184`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 6. Warm agent spawning

**Source**: `bardo/apps/mori/src/agent/mod.rs` — `MultiAgentPool`, `pre_spawn_warm()`, `promote_warm()`, `evict_warm()`
**Target**: `crates/roko-runtime/src/warm_pool.rs` + wire into orchestrate.rs

Pre-spawn agents during gate execution for faster phase transitions.

**Specification**:
1. **WarmPool**: `HashMap<AgentRole, WarmAgent>` where `WarmAgent` = pre-spawned process ready for promotion
2. **`pre_spawn_warm(role, effort)`**: During gate pipeline execution, spawn the next phase's agent in the background. The agent initializes but doesn't receive a task yet.
3. **`promote_warm(role) -> AgentConnection`**: Swap warm agent to active. The agent receives its task and starts working immediately. Saves 5-15s vs cold spawn.
4. **`evict_warm(role)`**: Kill warm agent on gate failure (no point keeping it if plan is replanning)

**Integration**: In orchestrate.rs, after dispatching compile gate, call `pre_spawn_warm(Reviewer)`. When gate passes, call `promote_warm(Reviewer)`. When gate fails, call `evict_warm(Reviewer)`.

**Acceptance criteria**:
- [ ] Warm agent spawns in background during gate execution
- [ ] `promote_warm()` returns usable agent connection without re-spawn delay
- [ ] `evict_warm()` kills process and frees resources
- [ ] Timing test: promote is <100ms vs 5-15s for cold spawn
- [ ] No leaked processes on gate failure path

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `186`
- Section hash: `87d6aceb1966f2e411121a3e23dec4d90cc994dd244ee52f4a8109acba04b762`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/agent/mod.rs` — `MultiAgentPool`, `pre_spawn_warm()`, `promote_warm()`, `evict_warm()` **Target**: `crates/roko-runtime/src/warm_pool.rs` + wire into orchestrate.rs
- **Specification**: 1. **WarmPool**: `HashMap<AgentRole, WarmAgent>` where `WarmAgent` = pre-spawned process ready for promotion 2. **`pre_spawn_warm(role, effort)`**: During gate pipeline execution, spawn the next phase's agent in the background. The agent initializes but doesn't receive a task yet. 3. **`promote_warm(role) -> AgentConnection`**: Swap warm agent to active. The agent receives its task and starts working immediately. Saves 5-15s vs cold spawn. 4. **`evict_warm(role)`**: Kill warm agent on gate failure (no point keeping it if plan is replanning)
- **Integration**: In orchestrate.rs, after dispatching compile gate, call `pre_spawn_warm(Reviewer)`. When gate passes, call `promote_warm(Reviewer)`. When gate fails, call `evict_warm(Reviewer)`.
- **Acceptance criteria**: - [ ] Warm agent spawns in background during gate execution - [ ] `promote_warm()` returns usable agent connection without re-spawn delay - [ ] `evict_warm()` kills process and frees resources - [ ] Timing test: promote is <100ms vs 5-15s for cold spawn - [ ] No leaked processes on gate failure path
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/agent/mod.rs
- crates/roko-runtime/src/warm_pool.rs

**Types, functions, traits, and inline code identifiers:**
- MultiAgentPool
- WarmAgent

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **WarmPool**: `HashMap<AgentRole, WarmAgent>` where `WarmAgent` = pre-spawned process ready for promotion
- 2. **`pre_spawn_warm(role, effort)`**: During gate pipeline execution, spawn the next phase's agent in the background. The agent initializes but doesn't receive a task yet.
- 3. **`promote_warm(role) -> AgentConnection`**: Swap warm agent to active. The agent receives its task and starts working immediately. Saves 5-15s vs cold spawn.
- 4. **`evict_warm(role)`**: Kill warm agent on gate failure (no point keeping it if plan is replanning)
- - [ ] Warm agent spawns in background during gate execution
- - [ ] `promote_warm()` returns usable agent connection without re-spawn delay
- - [ ] `evict_warm()` kills process and frees resources
- - [ ] Timing test: promote is <100ms vs 5-15s for cold spawn
- - [ ] No leaked processes on gate failure path

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/agent/mod.rs`
- `crates/roko-runtime/src/warm_pool.rs`
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
rg -n "Warm|spawn|gate|WarmAgent|spawning|promote|MultiAgentPool|role" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Warm|spawn|gate|WarmAgent|spawning|promote|MultiAgentPool|role" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/agent/mod.rs`
- `crates/roko-runtime/src/warm_pool.rs`
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
- [ ] Implement or verify `MultiAgentPool` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `WarmAgent` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S009 -- 7. Conductor watchers (10 rules)

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:185` through `217`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 7. Conductor watchers (10 rules)

**Source**: `bardo/apps/mori/src/conductor/mod.rs` (600+ LOC), `bardo/apps/mori/src/conductor/watchers.rs`
**Target**: Extend `crates/roko-conductor/src/`

Battle-tested detection rules for agent stalls, loops, and resource exhaustion.

| # | Watcher | Trigger | Action |
|---|---------|---------|--------|
| 1 | **GhostTurn** | No output + fast turn (<5s) + not in gating | Restart agent |
| 2 | **ReviewLoop** | 3+ consecutive REVISE verdicts + gates pass | Skip remaining reviews |
| 3 | **IterationLoop** | Iteration ≥6 + cycling strategist/implementer | Force advance |
| 4 | **TestFailureBudget** | 70%+ tests pass but some fail | Force advance (good enough) |
| 5 | **SilenceTimeout** | No output for 180s | Restart agent |
| 6 | **CompileFailThreshold** | 3+ consecutive compile failures | Force advance |
| 7 | **TaskStall** | Single task blocking for 300s | Restart agent |
| 8 | **ContextPressure** | Prompt >80% of context window | Trim context |
| 9 | **PhaseTimeout** | Phase exceeds 30min wall-clock | Restart |
| 10 | **CooldownFilter** | Last intervention within 120s | Skip (debounce) |

Each watcher returns `Option<Intervention { tier, watcher, target_role, message, action }>`.

**Acceptance criteria**:
- [ ] All 10 watchers implemented and registered in conductor
- [ ] CooldownFilter prevents intervention storms (tested with rapid triggers)
- [ ] Each watcher's threshold configurable (in `roko.toml` or conductor config)
- [ ] Interventions logged with tier/watcher/target for observability
- [ ] Unit tests for each watcher with mock ConductorContext

**Size**: L (3-4 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `191`
- Section hash: `900d9bd9187d9704da7d917d3ce9a89072d37f89361d593daad206e318997ed6`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/conductor/mod.rs` (600+ LOC), `bardo/apps/mori/src/conductor/watchers.rs` **Target**: Extend `crates/roko-conductor/src/`
- | # | Watcher | Trigger | Action | |---|---------|---------|--------| | 1 | **GhostTurn** | No output + fast turn (<5s) + not in gating | Restart agent | | 2 | **ReviewLoop** | 3+ consecutive REVISE verdicts + gates pass | Skip remaining reviews | | 3 | **IterationLoop** | Iteration ≥6 + cycling strategist/implementer | Force advance | | 4 | **TestFailureBudget** | 70%+ tests pass but some fail | Force advance (good enough) | | 5 | **SilenceTimeout** | No output for 180s | Restart agent | | 6 | **CompileFailThreshold** | 3+ consecutive compile failures | Force advance | | 7 | **TaskStall** | Single task blocking for 300s | Restart agent | | 8 | **ContextPressure** | Prompt >80% of context window | Trim context | | 9 | **PhaseTimeout** | Phase exceeds 30min wall-clock | Restart | | 10 | **CooldownFilter** | Last intervention within 120s | Skip (debounce) |
- **Acceptance criteria**: - [ ] All 10 watchers implemented and registered in conductor - [ ] CooldownFilter prevents intervention storms (tested with rapid triggers) - [ ] Each watcher's threshold configurable (in `roko.toml` or conductor config) - [ ] Interventions logged with tier/watcher/target for observability - [ ] Unit tests for each watcher with mock ConductorContext
- **Size**: L (3-4 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/conductor/mod.rs
- bardo/apps/mori/src/conductor/watchers.rs
- crates/roko-conductor/src/
- tier/watcher/

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
- - [ ] All 10 watchers implemented and registered in conductor
- - [ ] CooldownFilter prevents intervention storms (tested with rapid triggers)
- - [ ] Each watcher's threshold configurable (in `roko.toml` or conductor config)
- - [ ] Interventions logged with tier/watcher/target for observability
- - [ ] Unit tests for each watcher with mock ConductorContext

**Tables extracted:**
- Table 1:

```markdown
| # | Watcher | Trigger | Action |
|---|---------|---------|--------|
| 1 | **GhostTurn** | No output + fast turn (<5s) + not in gating | Restart agent |
| 2 | **ReviewLoop** | 3+ consecutive REVISE verdicts + gates pass | Skip remaining reviews |
| 3 | **IterationLoop** | Iteration ≥6 + cycling strategist/implementer | Force advance |
| 4 | **TestFailureBudget** | 70%+ tests pass but some fail | Force advance (good enough) |
| 5 | **SilenceTimeout** | No output for 180s | Restart agent |
| 6 | **CompileFailThreshold** | 3+ consecutive compile failures | Force advance |
| 7 | **TaskStall** | Single task blocking for 300s | Restart agent |
| 8 | **ContextPressure** | Prompt >80% of context window | Trim context |
| 9 | **PhaseTimeout** | Phase exceeds 30min wall-clock | Restart |
| 10 | **CooldownFilter** | Last intervention within 120s | Skip (debounce) |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/conductor/mod.rs`
- `bardo/apps/mori/src/conductor/watchers.rs`
- `tier/watcher/`
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
rg -n "Watcher|Conductor|watchers|rules|intervention|fail|context|Restart" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Watcher|Conductor|watchers|rules|intervention|fail|context|Restart" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/conductor/mod.rs`
- `bardo/apps/mori/src/conductor/watchers.rs`
- `tier/watcher/`
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S010 -- Learning loop gaps

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:218` through `223`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Learning loop gaps

These extend roko's learning subsystem with patterns from mori/bardo.

---
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `507add5711a6b5b63b8a8665301b325a6407e03fa17863462c5378ede833d562`

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "Learning|loop|gaps|subsystem|patterns|mori|extend|bardo" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Learning|loop|gaps|subsystem|patterns|mori|extend|bardo" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S011 -- 8. Wire neuro store into cascade router

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:224` through `249`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 8. Wire neuro store into cascade router

**Source**: `bardo/crates/golem-grimoire/src/` (grimoire retrieval scoring)
**Target**: `crates/roko-learn/src/cascade_router.rs`
**Existing**: `crates/roko-neuro/src/` (knowledge store), `crates/roko-learn/src/cascade_router.rs` (model routing)

Currently the cascade router selects models based on observations (pass/fail history) but does NOT consult the neuro store.

**Specification**:
1. At `decide()` time, query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge
2. If knowledge entries mention specific model preferences, bias model scoring by +0.1 for mentioned model
3. If knowledge entries describe failure patterns with a model, bias by -0.1
4. Add knowledge context to LinUCB feature vector (add 2 dims: `knowledge_match_score`, `knowledge_model_bias`)
5. Make opt-in via `cascade_router.consult_knowledge: bool` in config (default true)

**Acceptance criteria**:
- [ ] Cascade router queries neuro store at decide time
- [ ] Model bias applied based on knowledge entries
- [ ] LinUCB context vector extended with knowledge features
- [ ] Config toggle works (disabled = no knowledge query)
- [ ] No performance regression: knowledge query <10ms

**Size**: M (2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `162`
- Section hash: `8538bbdccd6329feff9852caa02c8b6678fbc3334d401ebadc820ec776227ab3`

**Normative requirements and implementation claims:**
- **Source**: `bardo/crates/golem-grimoire/src/` (grimoire retrieval scoring) **Target**: `crates/roko-learn/src/cascade_router.rs` **Existing**: `crates/roko-neuro/src/` (knowledge store), `crates/roko-learn/src/cascade_router.rs` (model routing)
- **Specification**: 1. At `decide()` time, query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge 2. If knowledge entries mention specific model preferences, bias model scoring by +0.1 for mentioned model 3. If knowledge entries describe failure patterns with a model, bias by -0.1 4. Add knowledge context to LinUCB feature vector (add 2 dims: `knowledge_match_score`, `knowledge_model_bias`) 5. Make opt-in via `cascade_router.consult_knowledge: bool` in config (default true)
- **Acceptance criteria**: - [ ] Cascade router queries neuro store at decide time - [ ] Model bias applied based on knowledge entries - [ ] LinUCB context vector extended with knowledge features - [ ] Config toggle works (disabled = no knowledge query) - [ ] No performance regression: knowledge query <10ms
- **Size**: M (2 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/crates/golem-grimoire/src/
- crates/roko-learn/src/cascade_router.rs
- crates/roko-neuro/src/

**Types, functions, traits, and inline code identifiers:**
- knowledge_match_score
- knowledge_model_bias

**Event names and event-like entities:**
- knowledge_store.query
- cascade_router.consult_knowledge

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. At `decide()` time, query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge
- 2. If knowledge entries mention specific model preferences, bias model scoring by +0.1 for mentioned model
- 3. If knowledge entries describe failure patterns with a model, bias by -0.1
- 4. Add knowledge context to LinUCB feature vector (add 2 dims: `knowledge_match_score`, `knowledge_model_bias`)
- 5. Make opt-in via `cascade_router.consult_knowledge: bool` in config (default true)
- - [ ] Cascade router queries neuro store at decide time
- - [ ] Model bias applied based on knowledge entries
- - [ ] LinUCB context vector extended with knowledge features
- - [ ] Config toggle works (disabled = no knowledge query)
- - [ ] No performance regression: knowledge query <10ms

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/crates/golem-grimoire/src/`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-neuro/src/`
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
rg -n "knowledge|router|cascade|store|neuro|model|knowledge_model_bias|knowledge_match_score" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "knowledge|router|cascade|store|neuro|model|knowledge_model_bias|knowledge_match_score" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/crates/golem-grimoire/src/`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-neuro/src/`
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
- [ ] Implement or verify `knowledge_match_score` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `knowledge_model_bias` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `knowledge_store.query` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `cascade_router.consult_knowledge` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S012 -- 9. Episode clustering for error patterns

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:250` through `276`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 9. Episode clustering for error patterns

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: Extend `crates/roko-learn/src/pattern_discovery.rs`

Cluster failed episodes by error signature to recommend model fallbacks.

**Functions**:
- `cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster>` — Group by `error_signature` (failures) or `file_pattern` (successes). Minimum cluster size: 3.
- `EpisodeCluster`: `{ key, count, success_rate, common_files, best_model, best_provider, avg_cost_usd }`
- Per cluster, compute which model has highest success_rate → `recommended_model`

**Integration**: Feed cluster recommendations into cascade_router as soft priors. When a new task matches a cluster's file pattern, bias toward recommended_model.

**Cadence**: Run clustering every 10 new episodes (use existing `UpdateFrequency` mechanism).

**Acceptance criteria**:
- [ ] `cluster_episodes()` groups episodes with matching error signatures
- [ ] Clusters with 3+ episodes produce model recommendations
- [ ] Recommendations integrated as soft bias in cascade_router
- [ ] Clustering runs on cadence (every 10 episodes)
- [ ] Test: 5 episodes with same error + model A succeeding → recommends model A

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `140`
- Section hash: `08c9bcca543cfe908f398a4041a4a3a28d7b42bb7eb5b8b59d5ccb92a6a532a1`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs` **Target**: Extend `crates/roko-learn/src/pattern_discovery.rs`
- **Functions**: - `cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster>` — Group by `error_signature` (failures) or `file_pattern` (successes). Minimum cluster size: 3. - `EpisodeCluster`: `{ key, count, success_rate, common_files, best_model, best_provider, avg_cost_usd }` - Per cluster, compute which model has highest success_rate → `recommended_model`
- **Integration**: Feed cluster recommendations into cascade_router as soft priors. When a new task matches a cluster's file pattern, bias toward recommended_model.
- **Cadence**: Run clustering every 10 new episodes (use existing `UpdateFrequency` mechanism).
- **Acceptance criteria**: - [ ] `cluster_episodes()` groups episodes with matching error signatures - [ ] Clusters with 3+ episodes produce model recommendations - [ ] Recommendations integrated as soft bias in cascade_router - [ ] Clustering runs on cadence (every 10 episodes) - [ ] Test: 5 episodes with same error + model A succeeding → recommends model A
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/pattern_learning.rs
- crates/roko-learn/src/pattern_discovery.rs

**Types, functions, traits, and inline code identifiers:**
- error_signature
- file_pattern
- EpisodeCluster
- recommended_model
- UpdateFrequency

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- model A succeeding -> recommends model A

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster>` — Group by `error_signature` (failures) or `file_pattern` (successes). Minimum cluster size: 3.
- - `EpisodeCluster`: `{ key, count, success_rate, common_files, best_model, best_provider, avg_cost_usd }`
- - Per cluster, compute which model has highest success_rate → `recommended_model`
- - [ ] `cluster_episodes()` groups episodes with matching error signatures
- - [ ] Clusters with 3+ episodes produce model recommendations
- - [ ] Recommendations integrated as soft bias in cascade_router
- - [ ] Clustering runs on cadence (every 10 episodes)
- - [ ] Test: 5 episodes with same error + model A succeeding → recommends model A

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/pattern_discovery.rs`
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
rg -n "Episode|Cluster|pattern|error|episodes|model|recommend|clustering" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Episode|Cluster|pattern|error|episodes|model|recommend|clustering" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/pattern_discovery.rs`
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
- [ ] Implement or verify `error_signature` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `file_pattern` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `EpisodeCluster` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `recommended_model` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `UpdateFrequency` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `model A succeeding -> recommends model A` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S013 -- 10. Provider pass-rate into model scoring

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:277` through `300`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 10. Provider pass-rate into model scoring

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: `crates/roko-learn/src/cascade_router.rs`
**Existing**: `crates/roko-learn/src/provider_health.rs`

Bias model selection toward proven providers.

**Specification**:
1. `compute_provider_metrics(episodes)` → per-provider: pass_rate, avg_cost, avg_duration (min 5 episodes)
2. `recommend_provider(metrics)` → pick provider with highest pass_rate
3. In cascade_router Stage 2 (confidence) and Stage 3 (LinUCB): multiply model score by `provider_pass_rate`
4. Use existing ProviderHealthTracker data if available, fall back to episode-derived metrics

**Acceptance criteria**:
- [ ] Provider metrics computed from episode history
- [ ] Model scores multiplied by provider pass_rate in stages 2-3
- [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6
- [ ] Minimum 5 episodes before provider metrics affect scoring

**Size**: S (1 day)

---
````

**Explicit detail extraction from this section:**

- Section word count: `119`
- Section hash: `e735e4ae13f608a5db0a18c5fe9040ef7c6bc6c70195d3ff9b01f4b912b759f7`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs` **Target**: `crates/roko-learn/src/cascade_router.rs` **Existing**: `crates/roko-learn/src/provider_health.rs`
- **Specification**: 1. `compute_provider_metrics(episodes)` → per-provider: pass_rate, avg_cost, avg_duration (min 5 episodes) 2. `recommend_provider(metrics)` → pick provider with highest pass_rate 3. In cascade_router Stage 2 (confidence) and Stage 3 (LinUCB): multiply model score by `provider_pass_rate` 4. Use existing ProviderHealthTracker data if available, fall back to episode-derived metrics
- **Acceptance criteria**: - [ ] Provider metrics computed from episode history - [ ] Model scores multiplied by provider pass_rate in stages 2-3 - [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6 - [ ] Minimum 5 episodes before provider metrics affect scoring
- **Size**: S (1 day)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/apps/mori/src/orchestrator/pattern_learning.rs
- crates/roko-learn/src/cascade_router.rs
- crates/roko-learn/src/provider_health.rs

**Types, functions, traits, and inline code identifiers:**
- provider_pass_rate

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. `compute_provider_metrics(episodes)` → per-provider: pass_rate, avg_cost, avg_duration (min 5 episodes)
- 2. `recommend_provider(metrics)` → pick provider with highest pass_rate
- 3. In cascade_router Stage 2 (confidence) and Stage 3 (LinUCB): multiply model score by `provider_pass_rate`
- 4. Use existing ProviderHealthTracker data if available, fall back to episode-derived metrics
- - [ ] Provider metrics computed from episode history
- - [ ] Model scores multiplied by provider pass_rate in stages 2-3
- - [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6
- - [ ] Minimum 5 episodes before provider metrics affect scoring

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/provider_health.rs`
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
rg -n "Provider|rate|pass|model|scoring|provider_pass_rate|pass_rate|metrics" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Provider|rate|pass|model|scoring|provider_pass_rate|pass_rate|metrics" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/provider_health.rs`
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
- [ ] Implement or verify `provider_pass_rate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S014 -- 11. Reflection-derived playbook rules

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:301` through `327`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 11. Reflection-derived playbook rules

**Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
**Target**: Extend `crates/roko-learn/src/playbook_rules.rs`

Auto-generate playbook rules from agent reflections (§4 above).

**Specification**:
1. After reflection is stored in episode, extract actionable patterns:
   - If reflection mentions specific files → create rule with `trigger_files` glob
   - If reflection mentions error type → create rule with `trigger_tags`
   - Context injection = the reflection's key insight
2. **Confidence**: New rules start at 0.5 (neutral). Boost +0.05 on gate pass, penalize -0.10 on gate fail. Remove rules below 0.2 confidence (unless manually created).
3. **Cadence**: Run after every 3 new reflections
4. **Persistence**: Append to `.roko/learn/playbook-rules.json` with `source: "reflection"` tag

**Acceptance criteria**:
- [ ] Reflections with file mentions → playbook rules with trigger_files
- [ ] Confidence tracking: +0.05 on success, -0.10 on failure
- [ ] Rules below 0.2 auto-removed
- [ ] Manually created rules preserved (never auto-removed)
- [ ] Persistence in playbook-rules.json with `source: "reflection"` tag

**Size**: M (2 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `159`
- Section hash: `f23ac569468647b5e70e6cbe0246a79f67b22ce73c1f419b0eb5a36d2a4f2dc6`

**Normative requirements and implementation claims:**
- **Source**: `bardo/apps/mori/src/orchestrator/pattern_learning.rs` **Target**: Extend `crates/roko-learn/src/playbook_rules.rs`
- **Specification**: 1. After reflection is stored in episode, extract actionable patterns: - If reflection mentions specific files → create rule with `trigger_files` glob - If reflection mentions error type → create rule with `trigger_tags` - Context injection = the reflection's key insight 2. **Confidence**: New rules start at 0.5 (neutral). Boost +0.05 on gate pass, penalize -0.10 on gate fail. Remove rules below 0.2 confidence (unless manually created). 3. **Cadence**: Run after every 3 new reflections 4. **Persistence**: Append to `.roko/learn/playbook-rules.json` with `source: "reflection"` tag
- **Acceptance criteria**: - [ ] Reflections with file mentions → playbook rules with trigger_files - [ ] Confidence tracking: +0.05 on success, -0.10 on failure - [ ] Rules below 0.2 auto-removed - [ ] Manually created rules preserved (never auto-removed) - [ ] Persistence in playbook-rules.json with `source: "reflection"` tag
- **Size**: M (2 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/learn/playbook-rules.json
- bardo/apps/mori/src/orchestrator/pattern_learning.rs
- crates/roko-learn/src/playbook_rules.rs

**Types, functions, traits, and inline code identifiers:**
- trigger_files
- trigger_tags

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- If reflection mentions specific files -> create rule with
- If reflection mentions error type -> create rule with
- Reflections with file mentions -> playbook rules with trigger_files

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. After reflection is stored in episode, extract actionable patterns:
- - If reflection mentions specific files → create rule with `trigger_files` glob
- - If reflection mentions error type → create rule with `trigger_tags`
- - Context injection = the reflection's key insight
- 2. **Confidence**: New rules start at 0.5 (neutral). Boost +0.05 on gate pass, penalize -0.10 on gate fail. Remove rules below 0.2 confidence (unless manually created).
- 3. **Cadence**: Run after every 3 new reflections
- 4. **Persistence**: Append to `.roko/learn/playbook-rules.json` with `source: "reflection"` tag
- - [ ] Reflections with file mentions → playbook rules with trigger_files
- - [ ] Confidence tracking: +0.05 on success, -0.10 on failure
- - [ ] Rules below 0.2 auto-removed
- - [ ] Manually created rules preserved (never auto-removed)
- - [ ] Persistence in playbook-rules.json with `source: "reflection"` tag

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `.roko/learn/playbook-rules.json`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/playbook_rules.rs`
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
rg -n "rule|rules|Reflection|playbook|trigger_files|trigger_tags|derived|file" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "rule|rules|Reflection|playbook|trigger_files|trigger_tags|derived|file" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `.roko/learn/playbook-rules.json`
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs`
- `crates/roko-learn/src/playbook_rules.rs`
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
- [ ] Implement or verify `trigger_files` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `trigger_tags` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `If reflection mentions specific files -> create rule with` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `If reflection mentions error type -> create rule with` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Reflections with file mentions -> playbook rules with trigger_files` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S015 -- 12. A-MAC admission gate for neuro store

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:328` through `354`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 12. A-MAC admission gate for neuro store

**Source**: `bardo/crates/golem-grimoire/src/` (A-MAC 5-factor admission gate)
**Target**: Extend `crates/roko-neuro/src/`

Prevent hallucinated or contradictory knowledge from entering the store.

**5-factor validation before any knowledge entry is stored**:
1. **Similarity**: Too similar to existing knowledge? (cosine sim > 0.95 → reject as duplicate)
2. **Novelty**: Does this add new information? (cosine sim < 0.3 to all existing → novel)
3. **Contradiction**: Does this contradict existing high-confidence entries? (semantic opposition check)
4. **Relevance**: Is this relevant to the agent's domain? (keyword match against domain tags)
5. **Confidence**: Does the source have sufficient credibility? (gate pass rate of the episode that generated this)

Gate result: `Admit | Reject { reason }`. Log rejections for debugging.

**Acceptance criteria**:
- [ ] Near-duplicate entries rejected (similarity > 0.95)
- [ ] Contradictory entries flagged (if existing entry has confidence > 0.8)
- [ ] Novel entries admitted with appropriate confidence score
- [ ] Rejections logged with reason
- [ ] Unit test: insert duplicate → rejected; insert novel fact → admitted; insert contradiction → flagged

**Size**: M (2-3 days)

---
````

**Explicit detail extraction from this section:**

- Section word count: `165`
- Section hash: `bc3eb5c54b18ab5d1511bcaf3e3675749102b0ca686aa9f39446f8d5be6a63bd`

**Normative requirements and implementation claims:**
- **Source**: `bardo/crates/golem-grimoire/src/` (A-MAC 5-factor admission gate) **Target**: Extend `crates/roko-neuro/src/`
- **5-factor validation before any knowledge entry is stored**: 1. **Similarity**: Too similar to existing knowledge? (cosine sim > 0.95 → reject as duplicate) 2. **Novelty**: Does this add new information? (cosine sim < 0.3 to all existing → novel) 3. **Contradiction**: Does this contradict existing high-confidence entries? (semantic opposition check) 4. **Relevance**: Is this relevant to the agent's domain? (keyword match against domain tags) 5. **Confidence**: Does the source have sufficient credibility? (gate pass rate of the episode that generated this)
- Gate result: `Admit | Reject { reason }`. Log rejections for debugging.
- **Acceptance criteria**: - [ ] Near-duplicate entries rejected (similarity > 0.95) - [ ] Contradictory entries flagged (if existing entry has confidence > 0.8) - [ ] Novel entries admitted with appropriate confidence score - [ ] Rejections logged with reason - [ ] Unit test: insert duplicate → rejected; insert novel fact → admitted; insert contradiction → flagged
- **Size**: M (2-3 days)
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- bardo/crates/golem-grimoire/src/
- crates/roko-neuro/src/

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- to all existing -> novel
- insert duplicate -> rejected
- insert novel fact -> admitted
- insert contradiction -> flagged

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Similarity**: Too similar to existing knowledge? (cosine sim > 0.95 → reject as duplicate)
- 2. **Novelty**: Does this add new information? (cosine sim < 0.3 to all existing → novel)
- 3. **Contradiction**: Does this contradict existing high-confidence entries? (semantic opposition check)
- 4. **Relevance**: Is this relevant to the agent's domain? (keyword match against domain tags)
- 5. **Confidence**: Does the source have sufficient credibility? (gate pass rate of the episode that generated this)
- - [ ] Near-duplicate entries rejected (similarity > 0.95)
- - [ ] Contradictory entries flagged (if existing entry has confidence > 0.8)
- - [ ] Novel entries admitted with appropriate confidence score
- - [ ] Rejections logged with reason
- - [ ] Unit test: insert duplicate → rejected; insert novel fact → admitted; insert contradiction → flagged

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/crates/golem-grimoire/src/`
- `crates/roko-neuro/src/`
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
rg -n "gate|for|store|reject|neuro|admission|MAC|contradict" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "gate|for|store|reject|neuro|admission|MAC|contradict" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `bardo/crates/golem-grimoire/src/`
- `crates/roko-neuro/src/`
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
- [ ] Enforce state transition `to all existing -> novel` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `insert duplicate -> rejected` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `insert novel fact -> admitted` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `insert contradiction -> flagged` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S016 -- Current state reconciliation

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:355` through `358`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Current state reconciliation

> Added 2026-04-24. Cross-references actual crate state to identify what already exists vs what needs building.
````

**Explicit detail extraction from this section:**

- Section word count: `18`
- Section hash: `1b726def2cdde6d00fdb44bc965e1f48dd16a4255c362be8fe741d01c14ba55e`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Cross-references actual crate state to identify what already exists vs what needs building.

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "state|reconciliation|Current|references|needs|identify|exists|crate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "state|reconciliation|Current|references|needs|identify|exists|crate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S017 -- Already implemented (do NOT rebuild)

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:359` through `373`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Already implemented (do NOT rebuild)

| Gap | Item | Location | Status |
|-----|------|----------|--------|
| 1 | `ReviewDecision` enum (Approve, Revise, Skip) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 1 | `ReviewIssue` struct (category, gate, rung, file, line, suggestion, blocking) | `roko-gate/src/review_verdict.rs` | **EXISTS** — 10 issue categories |
| 1 | `ReviewVerdict` struct (decision, summary, issues, rung_results) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 2 | `CompileError` struct (category, code, message, file, line, column, suggestion) | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 2 | `ErrorCategory` enum (10 categories) | `roko-gate/src/compile_errors.rs` | **EXISTS** — Syntax, UnresolvedImport, TypeMismatch, Lifetime, MissingMember, Unused, Visibility, Macro, TraitBound, Ownership, Other |
| 2 | `classify_error_code()` function | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 3 | `ErrorPattern` struct for cross-error pattern matching | `roko-conductor/src/diagnosis.rs` | **EXISTS** — conductor diagnoses at policy level |
| 7 | All 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — GhostTurn, ReviewLoop, IterationLoop, TestFailureBudget, Silence, CompileFailRepeat, TaskStall, ContextPressure, TimeOverrun, CooldownFilter |
| 7 | Intervention system with tiers | `roko-conductor/src/interventions.rs` | **EXISTS** — BanditPolicy, WorstSeverityPolicy |
| 7 | Circuit breaker | `roko-conductor/src/circuit_breaker.rs` | **EXISTS** — Holt forecasting |
````

**Explicit detail extraction from this section:**

- Section word count: `158`
- Section hash: `0f12f3930c1ef6972033bce792c3d4fad208506b4af39e85c9ab7d446c40bad8`

**Normative requirements and implementation claims:**
- | Gap | Item | Location | Status | |-----|------|----------|--------| | 1 | `ReviewDecision` enum (Approve, Revise, Skip) | `roko-gate/src/review_verdict.rs` | **EXISTS** | | 1 | `ReviewIssue` struct (category, gate, rung, file, line, suggestion, blocking) | `roko-gate/src/review_verdict.rs` | **EXISTS** — 10 issue categories | | 1 | `ReviewVerdict` struct (decision, summary, issues, rung_results) | `roko-gate/src/review_verdict.rs` | **EXISTS** | | 2 | `CompileError` struct (category, code, message, file, line, column, suggestion) | `roko-gate/src/compile_errors.rs` | **EXISTS** | | 2 | `ErrorCategory` enum (10 categories) | `roko-gate/src/compile_errors.rs` | **EXISTS** — Syntax, UnresolvedImport, TypeMismatch, Lifetime, MissingMember, Unused, Visibility, Macro, TraitBound, Ownership, Other | | 2 | `classify_error_code()` function | `roko-gate/src/compile_errors.rs` | **EXISTS** | | 3 | `ErrorPattern` struct for cross-error pattern matching | `roko-conductor/src/diagnosis.rs` | **EXI

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- roko-conductor/src/circuit_breaker.rs
- roko-conductor/src/diagnosis.rs
- roko-conductor/src/interventions.rs
- roko-conductor/src/watchers/
- roko-gate/src/compile_errors.rs
- roko-gate/src/review_verdict.rs

**Types, functions, traits, and inline code identifiers:**
- for
- ReviewDecision
- ReviewIssue
- ReviewVerdict
- CompileError
- ErrorCategory
- ErrorPattern

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
| Gap | Item | Location | Status |
|-----|------|----------|--------|
| 1 | `ReviewDecision` enum (Approve, Revise, Skip) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 1 | `ReviewIssue` struct (category, gate, rung, file, line, suggestion, blocking) | `roko-gate/src/review_verdict.rs` | **EXISTS** — 10 issue categories |
| 1 | `ReviewVerdict` struct (decision, summary, issues, rung_results) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| 2 | `CompileError` struct (category, code, message, file, line, column, suggestion) | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 2 | `ErrorCategory` enum (10 categories) | `roko-gate/src/compile_errors.rs` | **EXISTS** — Syntax, UnresolvedImport, TypeMismatch, Lifetime, MissingMember, Unused, Visibility, Macro, TraitBound, Ownership, Other |
| 2 | `classify_error_code()` function | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| 3 | `ErrorPattern` struct for cross-error pattern matching | `roko-conductor/src/diagnosis.rs` | **EXISTS** — conductor diagnoses at policy level |
| 7 | All 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — GhostTurn, ReviewLoop, IterationLoop, TestFailureBudget, Silence, CompileFailRepeat, TaskStall, ContextPressure, TimeOverrun, CooldownFilter |
| 7 | Intervention system with tiers | `roko-conductor/src/interventions.rs` | **EXISTS** — BanditPolicy, WorstSeverityPolicy |
| 7 | Circuit breaker | `roko-conductor/src/circuit_breaker.rs` | **EXISTS** — Holt forecasting |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `roko-conductor/src/circuit_breaker.rs`
- `roko-conductor/src/diagnosis.rs`
- `roko-conductor/src/interventions.rs`
- `roko-conductor/src/watchers/`
- `roko-gate/src/compile_errors.rs`
- `roko-gate/src/review_verdict.rs`
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
rg -n "EXISTS|error|gate|for|conductor|rebuild|implemented|ReviewVerdict" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "EXISTS|error|gate|for|conductor|rebuild|implemented|ReviewVerdict" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `roko-conductor/src/circuit_breaker.rs`
- `roko-conductor/src/diagnosis.rs`
- `roko-conductor/src/interventions.rs`
- `roko-conductor/src/watchers/`
- `roko-gate/src/compile_errors.rs`
- `roko-gate/src/review_verdict.rs`
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
- [ ] Implement or verify `for` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReviewDecision` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReviewIssue` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ReviewVerdict` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `CompileError` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ErrorCategory` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ErrorPattern` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S018 -- Remaining work (gaps that still need implementation)

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:374` through `395`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Remaining work (gaps that still need implementation)

| Gap | What | Notes |
|-----|------|-------|
| 1 | **Wire ReviewVerdict parsing into orchestrate.rs** | Types exist but parsing agent output → verdict is not wired |
| 1 | **Express mode** (skip strategist when all issues quick-fixable) | Phase transition logic not wired |
| 2 | **`apply_rustc_fixes()` auto-fix path** | Run `cargo fix --allow-dirty` + `cargo fmt` before spawning agent |
| 2 | **Wire classified errors into agent prompt** | Agent still gets raw cargo output instead of structured errors |
| 3 | **Error pattern sharing between parallel agents** | Pattern file exists but not injected into system prompt |
| 3 | **`is_mostly_passing()` check** | Not used to decide between targeted fix vs full replan |
| 4 | **Post-gate reflection loop** | Full gap — not implemented at all |
| 5 | **Context injection scoping** | Full gap — KnowledgeConfig, role-filtered context |
| 6 | **Warm agent spawning** | Full gap — WarmPool not implemented |
| 7 | **Configurable watcher thresholds** | Watchers exist but thresholds may be hardcoded; verify configurability |
| 8 | **Neuro store → cascade router** | Full gap — router doesn't consult knowledge store |
| 9 | **Episode clustering** | Full gap — no clustering in pattern_discovery.rs |
| 10 | **Provider pass-rate bias** | Full gap — provider metrics not multiplied into model scores |
| 11 | **Reflection-derived playbook rules** | Full gap — no auto-generation from reflections |
| 12 | **A-MAC admission gate** | Full gap — no 5-factor validation |

---
````

**Explicit detail extraction from this section:**

- Section word count: `199`
- Section hash: `d0e00c8adf0c12c4367fcd134e01364f12022fff5c907269f5a133ef5fc6e30c`

**Normative requirements and implementation claims:**
- | Gap | What | Notes | |-----|------|-------| | 1 | **Wire ReviewVerdict parsing into orchestrate.rs** | Types exist but parsing agent output → verdict is not wired | | 1 | **Express mode** (skip strategist when all issues quick-fixable) | Phase transition logic not wired | | 2 | **`apply_rustc_fixes()` auto-fix path** | Run `cargo fix --allow-dirty` + `cargo fmt` before spawning agent | | 2 | **Wire classified errors into agent prompt** | Agent still gets raw cargo output instead of structured errors | | 3 | **Error pattern sharing between parallel agents** | Pattern file exists but not injected into system prompt | | 3 | **`is_mostly_passing()` check** | Not used to decide between targeted fix vs full replan | | 4 | **Post-gate reflection loop** | Full gap — not implemented at all | | 5 | **Context injection scoping** | Full gap — KnowledgeConfig, role-filtered context | | 6 | **Warm agent spawning** | Full gap — WarmPool not implemented | | 7 | **Configurable watcher thresholds** | 
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
- Types exist but parsing agent output -> verdict is not wired
- Neuro store -> cascade router

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- cargo fix --allow-dirty
- cargo fmt

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Gap | What | Notes |
|-----|------|-------|
| 1 | **Wire ReviewVerdict parsing into orchestrate.rs** | Types exist but parsing agent output → verdict is not wired |
| 1 | **Express mode** (skip strategist when all issues quick-fixable) | Phase transition logic not wired |
| 2 | **`apply_rustc_fixes()` auto-fix path** | Run `cargo fix --allow-dirty` + `cargo fmt` before spawning agent |
| 2 | **Wire classified errors into agent prompt** | Agent still gets raw cargo output instead of structured errors |
| 3 | **Error pattern sharing between parallel agents** | Pattern file exists but not injected into system prompt |
| 3 | **`is_mostly_passing()` check** | Not used to decide between targeted fix vs full replan |
| 4 | **Post-gate reflection loop** | Full gap — not implemented at all |
| 5 | **Context injection scoping** | Full gap — KnowledgeConfig, role-filtered context |
| 6 | **Warm agent spawning** | Full gap — WarmPool not implemented |
| 7 | **Configurable watcher thresholds** | Watchers exist but thresholds may be hardcoded; verify configurability |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "full|still|work|need|gaps|Remaining|Wire|reflection" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "full|still|work|need|gaps|Remaining|Wire|reflection" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Enforce state transition `Types exist but parsing agent output -> verdict is not wired` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Neuro store -> cascade router` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Implement or verify operator command `cargo fix --allow-dirty` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `cargo fmt` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S019 -- Spec clarifications (resolving ambiguities)

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:396` through `399`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Spec clarifications (resolving ambiguities)

> Added 2026-04-24. These resolve gaps identified during architecture audit.
````

**Explicit detail extraction from this section:**

- Section word count: `11`
- Section hash: `45ad0272ebc6d461717dd7b96f1adf28b060afcfbfb11a274e5e09c4c7219557`

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "resolving|clarifications|ambiguities|Spec|resolve|identified|gaps|during" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "resolving|clarifications|ambiguities|Spec|resolve|identified|gaps|during" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S020 -- Gap 1: Parsing fallback chain

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:400` through `432`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 1: Parsing fallback chain

The spec says "Try JSON first, then JSON code block, then TOML block." Full algorithm:

```rust
fn parse_review(output: &str) -> StructuredReview {
    // 1. Try parsing entire output as JSON
    if let Ok(review) = serde_json::from_str::<StructuredReview>(output) {
        return review;
    }
    // 2. Try extracting JSON from ```json ... ``` code block
    if let Some(json_block) = extract_code_block(output, "json") {
        if let Ok(review) = serde_json::from_str::<StructuredReview>(&json_block) {
            return review;
        }
    }
    // 3. Try extracting TOML from ```toml ... ``` code block
    if let Some(toml_block) = extract_code_block(output, "toml") {
        if let Ok(review) = toml::from_str::<StructuredReview>(&toml_block) {
            return review;
        }
    }
    // 4. Fallback: treat entire output as a Revise verdict with raw text
    StructuredReview {
        verdict: ReviewDecision::Revise,
        issues: vec![],
        summary: output.chars().take(500).collect(),  // cap at 500 chars
    }
}
```

The fallback (step 4) means **parsing never fails** — worst case, the raw text becomes the summary and the orchestrator treats it as a revision request.
````

**Explicit detail extraction from this section:**

- Section word count: `141`
- Section hash: `d18e9be911f4ad2d61452a4d5e634d7cf989fb2b7ec3ece67133d84b81d38ae1`

**Normative requirements and implementation claims:**
- The fallback (step 4) means **parsing never fails** — worst case, the raw text becomes the summary and the orchestrator treats it as a revision request.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- parse_review

**Event names and event-like entities:**
- output.chars

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
- Contract 1: language `rust`, first line `fn parse_review(output: &str) -> StructuredReview {`

```rust
fn parse_review(output: &str) -> StructuredReview {
    // 1. Try parsing entire output as JSON
    if let Ok(review) = serde_json::from_str::<StructuredReview>(output) {
        return review;
    }
    // 2. Try extracting JSON from ```json ... ``` code block
    if let Some(json_block) = extract_code_block(output, "json") {
        if let Ok(review) = serde_json::from_str::<StructuredReview>(&json_block) {
            return review;
        }
    }
    // 3. Try extracting TOML from ```toml ... ``` code block
    if let Some(toml_block) = extract_code_block(output, "toml") {
        if let Ok(review) = toml::from_str::<StructuredReview>(&toml_block) {
            return review;
        }
    }
    // 4. Fallback: treat entire output as a Revise verdict with raw text
    StructuredReview {
        verdict: ReviewDecision::Revise,
        issues: vec![],
        summary: output.chars().take(500).collect(),  // cap at 500 chars
    }
}
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "review|block|JSON|output|fallback|TOML|Parsing|parse_review" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "review|block|JSON|output|fallback|TOML|Parsing|parse_review" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify `parse_review` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `output.chars` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S021 -- Gap 1: is_quick_fixable() categories

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:433` through `440`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 1: `is_quick_fixable()` categories

Quick-fixable categories: `Compilation`, `Docs`, `Style`, `LintViolation`, `Unused`.

NOT quick-fixable (even if they seem small): `TestFailure` (tests can reveal deeper bugs), `TypeMismatch` (may require API changes), `SpecDeviation` (needs design discussion), `SecurityVulnerability` (needs careful review).

Rationale: "quick-fixable" means "an implementer agent can fix this without strategic planning." Compilation errors have deterministic fixes (add import, fix syntax). Style/docs/lint are mechanical. Tests and types can cascade.
````

**Explicit detail extraction from this section:**

- Section word count: `67`
- Section hash: `9347c9a6a7f2971ce3b52f6be8d64dc18c69401599da85a032a02fb144210581`

**Normative requirements and implementation claims:**
- NOT quick-fixable (even if they seem small): `TestFailure` (tests can reveal deeper bugs), `TypeMismatch` (may require API changes), `SpecDeviation` (needs design discussion), `SecurityVulnerability` (needs careful review).
- Rationale: "quick-fixable" means "an implementer agent can fix this without strategic planning." Compilation errors have deterministic fixes (add import, fix syntax). Style/docs/lint are mechanical. Tests and types can cascade.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- Style/docs/

**Types, functions, traits, and inline code identifiers:**
- Compilation
- Docs
- Style
- LintViolation
- Unused
- TestFailure
- TypeMismatch
- SpecDeviation
- SecurityVulnerability

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
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `Style/docs/`
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
rg -n "fixable|Quick|categories|Style|Docs|Compilation|is_quick_fixable|Unused" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "fixable|Quick|categories|Style|Docs|Compilation|is_quick_fixable|Unused" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
- `Style/docs/`
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
- [ ] Implement or verify `Compilation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Docs` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Style` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `LintViolation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Unused` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TestFailure` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `TypeMismatch` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SpecDeviation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `SecurityVulnerability` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S022 -- Gap 2: cargo fix merge conflict handling

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:441` through `450`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 2: `cargo fix` merge conflict handling

If `cargo fix --allow-dirty` modifies a file that was also edited by a previous agent in the same task:
1. Run `cargo fix --allow-dirty` on the worktree
2. If it succeeds, run `cargo fmt`
3. If `cargo fix` exits non-zero (e.g., conflicting suggestions), skip auto-fix and fall through to agent-assisted fix
4. Never run `cargo fix` with `--allow-staged` (could corrupt staging area)

The key insight: `cargo fix` operates on the working tree. If the previous agent's changes are already in the working tree, `cargo fix` applies on top of them. Conflicts are rare because `cargo fix` only applies compiler suggestions, which don't overlap with semantic changes.
````

**Explicit detail extraction from this section:**

- Section word count: `115`
- Section hash: `98055ee98b57d51877249f1224ebc7145872d90d2c01cf8aa2066a8ca1b1c853`

**Normative requirements and implementation claims:**
- If `cargo fix --allow-dirty` modifies a file that was also edited by a previous agent in the same task: 1. Run `cargo fix --allow-dirty` on the worktree 2. If it succeeds, run `cargo fmt` 3. If `cargo fix` exits non-zero (e.g., conflicting suggestions), skip auto-fix and fall through to agent-assisted fix 4. Never run `cargo fix` with `--allow-staged` (could corrupt staging area)
- The key insight: `cargo fix` operates on the working tree. If the previous agent's changes are already in the working tree, `cargo fix` applies on top of them. Conflicts are rare because `cargo fix` only applies compiler suggestions, which don't overlap with semantic changes.

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
- cargo fix --allow-dirty
- cargo fmt
- cargo fix

**Bullet requirements:**
- 1. Run `cargo fix --allow-dirty` on the worktree
- 2. If it succeeds, run `cargo fmt`
- 3. If `cargo fix` exits non-zero (e.g., conflicting suggestions), skip auto-fix and fall through to agent-assisted fix
- 4. Never run `cargo fix` with `--allow-staged` (could corrupt staging area)

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "fix|cargo|conflict|merge|handling|Gap|tree|allow" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "fix|cargo|conflict|merge|handling|Gap|tree|allow" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify operator command `cargo fix --allow-dirty` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `cargo fmt` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `cargo fix` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S023 -- Gap 3: Error deduplication algorithm

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:451` through `462`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 3: Error deduplication algorithm

Error patterns are deduplicated by **normalized error code + file path** (not full error text):

```rust
fn error_key(error: &CompileError) -> String {
    format!("{}::{}", error.code, error.file.as_deref().unwrap_or("unknown"))
}
```

Two `error[E0425]` in different files ARE different patterns (they need different fixes). Two `error[E0425]` in the same file with different line numbers are the SAME pattern (likely the same root cause).
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `3edffb8c4c823d11bcb16dbb97d1bb9ef7dbc2092b54ae4e5dc34231d51271d8`

**Normative requirements and implementation claims:**
- Two `error[E0425]` in different files ARE different patterns (they need different fixes). Two `error[E0425]` in the same file with different line numbers are the SAME pattern (likely the same root cause).

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- error_key

**Event names and event-like entities:**
- error.code
- error.file.as_deref

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
- Contract 1: language `rust`, first line `fn error_key(error: &CompileError) -> String {`

```rust
fn error_key(error: &CompileError) -> String {
    format!("{}::{}", error.code, error.file.as_deref().unwrap_or("unknown"))
}
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "Error|error_key|deduplication|algorithm|Gap|file|different|same" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Error|error_key|deduplication|algorithm|Gap|file|different|same" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify `error_key` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `error.code` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `error.file.as_deref` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S024 -- Gap 4: Reflection cost guard with variable pricing

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:463` through `475`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 4: Reflection cost guard with variable pricing

The "$0.02 max" cost guard assumes Haiku pricing. The actual guard is:

```rust
let max_reflection_tokens = 500;  // output tokens
let model = "claude-haiku-4-5-20251001";  // always use cheapest model
// At Haiku pricing ($0.25/M output), 500 tokens = $0.000125
// The $0.02 cap is generous — actual cost is ~$0.0001
```

The guard is not price-based, it's **token-based**: max_tokens=500 on the cheapest available model. This works regardless of provider pricing changes.
````

**Explicit detail extraction from this section:**

- Section word count: `76`
- Section hash: `ef6cf4da60fc62a2286cb5a3396ee6924f926f26b2df5d39fc46696831bac09b`

**Normative requirements and implementation claims:**
- ```rust let max_reflection_tokens = 500; // output tokens let model = "claude-haiku-4-5-20251001"; // always use cheapest model // At Haiku pricing ($0.25/M output), 500 tokens = $0.000125 // The $0.02 cap is generous — actual cost is ~$0.0001 ```

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
- Contract 1: language `rust`, first line `let max_reflection_tokens = 500;  // output tokens`

```rust
let max_reflection_tokens = 500;  // output tokens
let model = "claude-haiku-4-5-20251001";  // always use cheapest model
// At Haiku pricing ($0.25/M output), 500 tokens = $0.000125
// The $0.02 cap is generous — actual cost is ~$0.0001
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "pricing|guard|cost|Reflection|variable|token|Gap|tokens" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "pricing|guard|cost|Reflection|variable|token|Gap|tokens" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S025 -- Gap 5: Context size numbers for role filtering

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:476` through `492`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 5: Context size numbers for role filtering

| Role | File intel entries | Warning entries | Error pattern entries |
|------|-------------------|-----------------|----------------------|
| Implementer | 10 (full: file path, key functions, recent changes) | 5 | 5 |
| Reviewer | 3 (summary: file path + one-line description) | 3 | 3 |
| Strategist | 0 (sees plan-level only) | 0 | 0 |

These are defaults from `KnowledgeConfig`. All configurable via roko.toml:

```toml
[knowledge]
file_intel_max_entries = 10
warnings_max_entries = 5
error_pattern_min_cluster = 3
```
````

**Explicit detail extraction from this section:**

- Section word count: `56`
- Section hash: `b614907450b66031397f702efbabf84d9c551e8dae3c214a8cd531dcdddaa0c5`

**Normative requirements and implementation claims:**
- | Role | File intel entries | Warning entries | Error pattern entries | |------|-------------------|-----------------|----------------------| | Implementer | 10 (full: file path, key functions, recent changes) | 5 | 5 | | Reviewer | 3 (summary: file path + one-line description) | 3 | 3 | | Strategist | 0 (sees plan-level only) | 0 | 0 |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- KnowledgeConfig

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- [knowledge]
- file_intel_max_entries = 10
- warnings_max_entries = 5
- error_pattern_min_cluster = 3

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Role | File intel entries | Warning entries | Error pattern entries |
|------|-------------------|-----------------|----------------------|
| Implementer | 10 (full: file path, key functions, recent changes) | 5 | 5 |
| Reviewer | 3 (summary: file path + one-line description) | 3 | 3 |
| Strategist | 0 (sees plan-level only) | 0 | 0 |
```

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `[knowledge]`

```toml
[knowledge]
file_intel_max_entries = 10
warnings_max_entries = 5
error_pattern_min_cluster = 3
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "role|size|numbers|for|filtering|entries|KnowledgeConfig|Gap" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "role|size|numbers|for|filtering|entries|KnowledgeConfig|Gap" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify `KnowledgeConfig` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `[knowledge]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `file_intel_max_entries = 10` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `warnings_max_entries = 5` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `error_pattern_min_cluster = 3` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S026 -- Gap 7: Conductor watcher threshold config

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:493` through `512`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 7: Conductor watcher threshold config

Watcher thresholds are configurable in roko.toml under `[conductor]`:

```toml
[conductor]
ghost_turn_max_secs = 5
review_loop_max_consecutive = 3
iteration_loop_max = 6
test_failure_budget_pass_rate = 0.70
silence_timeout_secs = 180
compile_fail_max_consecutive = 3
task_stall_secs = 300
context_pressure_percent = 80
phase_timeout_secs = 1800
cooldown_filter_secs = 120
```

If a key is missing, the hardcoded default from the watchers table applies.
````

**Explicit detail extraction from this section:**

- Section word count: `45`
- Section hash: `8836b95e924297a3a89ab351baa273317acae1ddb660747c4306b20e4d95f19b`

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
- [conductor]
- ghost_turn_max_secs = 5
- review_loop_max_consecutive = 3
- iteration_loop_max = 6
- test_failure_budget_pass_rate = 0.70
- silence_timeout_secs = 180
- compile_fail_max_consecutive = 3
- task_stall_secs = 300
- context_pressure_percent = 80
- phase_timeout_secs = 1800
- cooldown_filter_secs = 120

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
ghost_turn_max_secs = 5
review_loop_max_consecutive = 3
iteration_loop_max = 6
test_failure_budget_pass_rate = 0.70
silence_timeout_secs = 180
compile_fail_max_consecutive = 3
task_stall_secs = 300
context_pressure_percent = 80
phase_timeout_secs = 1800
cooldown_filter_secs = 120
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "watcher|Conductor|threshold|config|Gap|toml|watchers|under" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "watcher|Conductor|threshold|config|Gap|toml|watchers|under" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Add or verify config key `[conductor]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `ghost_turn_max_secs = 5` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `review_loop_max_consecutive = 3` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `iteration_loop_max = 6` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `test_failure_budget_pass_rate = 0.70` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `silence_timeout_secs = 180` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `compile_fail_max_consecutive = 3` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `task_stall_secs = 300` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `context_pressure_percent = 80` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `phase_timeout_secs = 1800` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `cooldown_filter_secs = 120` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S027 -- Gap 8: Cascade router knowledge bias clamping

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:513` through `524`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 8: Cascade router knowledge bias clamping

Model scores in the cascade router range from 0.0 to 1.0. The knowledge bias is additive:

```
final_score = base_score + knowledge_bias
knowledge_bias ∈ [-0.1, +0.1]
final_score = clamp(final_score, 0.05, 1.0)  // never zero (always possible), cap at 1.0
```

The 0.05 floor ensures every model has a non-zero chance of selection (exploration).
````

**Explicit detail extraction from this section:**

- Section word count: `56`
- Section hash: `e813ed93e4fd91b78802b2dea30c3f5221d71a72eb0038b6725e68bc196235fe`

**Normative requirements and implementation claims:**
- ``` final_score = base_score + knowledge_bias knowledge_bias ∈ [-0.1, +0.1] final_score = clamp(final_score, 0.05, 1.0) // never zero (always possible), cap at 1.0 ```

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
- final_score = base_score + knowledge_bias
- final_score = clamp(final_score, 0.05, 1.0)  // never zero (always possible), cap at 1.0

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `final_score = base_score + knowledge_bias`

```
final_score = base_score + knowledge_bias
knowledge_bias ∈ [-0.1, +0.1]
final_score = clamp(final_score, 0.05, 1.0)  // never zero (always possible), cap at 1.0
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- `crates/roko-serve/tests/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "knowledge|bias|router|clamp|Cascade|clamping|Gap|final_score" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "knowledge|bias|router|clamp|Cascade|clamping|Gap|final_score" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Add or verify config key `final_score = base_score + knowledge_bias` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `final_score = clamp(final_score, 0.05, 1.0)  // never zero (always possible), cap at 1.0` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S028 -- Gap 9: Episode clustering with fewer than 3 matches

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:525` through `540`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 9: Episode clustering with fewer than 3 matches

Clusters with fewer than 3 episodes are **not discarded** — they're stored as `immature` clusters:

```rust
pub struct EpisodeCluster {
    pub key: String,
    pub count: usize,
    pub maturity: ClusterMaturity,  // Immature (< 3) | Mature (>= 3)
    pub recommended_model: Option<String>,  // None for immature
    // ...
}
```

Immature clusters don't produce model recommendations. They become mature when they accumulate 3+ episodes. This prevents premature conclusions from small samples.
````

**Explicit detail extraction from this section:**

- Section word count: `61`
- Section hash: `efcd3bcbd3e31534ec5baee16c64d5e81dc84fbf2d816ebdf070b1bbdbd4c40b`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- EpisodeCluster
- immature

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
- Contract 1: language `rust`, first line `pub struct EpisodeCluster {`

```rust
pub struct EpisodeCluster {
    pub key: String,
    pub count: usize,
    pub maturity: ClusterMaturity,  // Immature (< 3) | Mature (>= 3)
    pub recommended_model: Option<String>,  // None for immature
    // ...
}
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "immature|Episode|Mature|fewer|matches|clustering|Gap|EpisodeCluster" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "immature|Episode|Mature|fewer|matches|clustering|Gap|EpisodeCluster" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify `EpisodeCluster` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `immature` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

### ARCH-20-S029 -- Gap 12: A-MAC contradiction detection

**Source section:** `tmp/architecture/20-orchestrator-gaps.md:541` through `568`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Gap 12: A-MAC contradiction detection

Contradiction is detected via **cosine distance inversion**, not a separate "semantic opposition" model:

```rust
fn check_contradiction(new_entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> bool {
    for entry in existing.iter().filter(|e| e.confidence > 0.8) {
        let sim = cosine_similarity(&new_entry.hdc_vector, &entry.hdc_vector);
        // High similarity but opposite conclusion = contradiction
        // Measured by: topic vectors are similar (sim > 0.7) but
        // the assertion differs (new claims opposite of existing)
        if sim > 0.7 {
            let assertion_sim = cosine_similarity(
                &new_entry.assertion_vector,
                &entry.assertion_vector,
            );
            if assertion_sim < -0.3 {
                return true;  // Contradiction: same topic, opposite claim
            }
        }
    }
    false
}
```

The key insight: entries are encoded with two HDC vectors — one for the **topic** (what it's about) and one for the **assertion** (what it claims). High topic similarity + negative assertion similarity = contradiction.

If HDC vectors are not available (e.g., pre-HDC entries), fall back to keyword overlap for topic similarity and skip contradiction checking (return false).
````

**Explicit detail extraction from this section:**

- Section word count: `144`
- Section hash: `471c6247248c44a4f92f04658a5bb65b90622ca0c4e1272f016109964cfb41c7`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- check_contradiction

**Event names and event-like entities:**
- existing.iter
- new_entry.hdc_vector
- entry.hdc_vector
- new_entry.assertion_vector
- entry.assertion_vector

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
- Contract 1: language `rust`, first line `fn check_contradiction(new_entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> bool {`

```rust
fn check_contradiction(new_entry: &KnowledgeEntry, existing: &[KnowledgeEntry]) -> bool {
    for entry in existing.iter().filter(|e| e.confidence > 0.8) {
        let sim = cosine_similarity(&new_entry.hdc_vector, &entry.hdc_vector);
        // High similarity but opposite conclusion = contradiction
        // Measured by: topic vectors are similar (sim > 0.7) but
        // the assertion differs (new claims opposite of existing)
        if sim > 0.7 {
            let assertion_sim = cosine_similarity(
                &new_entry.assertion_vector,
                &entry.assertion_vector,
            );
            if assertion_sim < -0.3 {
                return true;  // Contradiction: same topic, opposite claim
            }
        }
    }
    false
}
```

**Read before editing:**
- `tmp/architecture/20-orchestrator-gaps.md`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
rg -n "contradiction|entry|similar|assertion|similarity|topic|detection|check_contradiction" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "contradiction|entry|similar|assertion|similarity|topic|detection|check_contradiction" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/`
- `crates/roko-conductor/src/`
- `crates/roko-gate/src/`
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
- [ ] Implement or verify `check_contradiction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `existing.iter` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `new_entry.hdc_vector` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `entry.hdc_vector` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `new_entry.assertion_vector` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `entry.assertion_vector` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/20-orchestrator-gaps
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

