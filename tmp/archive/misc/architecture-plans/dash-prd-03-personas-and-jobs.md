# Dashboard PRD Plan: Personas And Jobs

**Source:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
**Generated:** 2026-04-25
**Source hash:** `9e586ee45edca2652f3f343812804942650d5f778622764d4a8d450f1da6272d`
**Section tasks:** 11
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
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| DASH-03-S001 | 1 | 03 — Personas and jobs | [ ] | 9.8 |
| DASH-03-S002 | 7 | Why this document matters | [ ] | 9.8 |
| DASH-03-S003 | 19 | Persona 1 — Solo Operator | [ ] | 9.8 |
| DASH-03-S004 | 70 | Persona 2 — Fleet Orchestrator | [ ] | 9.8 |
| DASH-03-S005 | 118 | Persona 3 — Arena Competitor | [ ] | 9.8 |
| DASH-03-S006 | 168 | Persona 4 — Knowledge Contributor | [ ] | 9.8 |
| DASH-03-S007 | 217 | Persona 5 — Domain Architect | [ ] | 9.8 |
| DASH-03-S008 | 261 | Persona 6 — Meta-Builder | [ ] | 9.8 |
| DASH-03-S009 | 308 | Persona 7 — Passive User | [ ] | 9.8 |
| DASH-03-S010 | 349 | Persona 8 — System Steward | [ ] | 9.8 |
| DASH-03-S011 | 391 | Using the personas in specifications | [ ] | 9.8 |

## Tasks

### DASH-03-S001 -- 03 — Personas and jobs

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:1` through `6`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# 03 — Personas and jobs

*Eight concrete personas the Nunchi dashboard is designed for, and the jobs each persona hires the product to do.*

---
````

**Explicit detail extraction from this section:**

- Section word count: `19`
- Section hash: `49e532e371b6fcdd1bd386fd7375e2b7359abe77fb9db61c064fde035e4b411e`

**Normative requirements and implementation claims:**
- *Eight concrete personas the Nunchi dashboard is designed for, and the jobs each persona hires the product to do.*
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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "persona|jobs|Personas|product|hires|designed|concrete" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "persona|jobs|Personas|product|hires|designed|concrete" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S002 -- Why this document matters

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:7` through `18`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Why this document matters

Subsequent documents reference personas by short handle (e.g., Solo Operator, Arena Competitor). Without this document establishing what each handle means, those references are noise. With it, they are shorthand.

Personas are not demographic sketches. They are descriptions of what people want, what they won't tolerate, how they work, and what success looks like to them. A page that serves the Arena Competitor well may fail the Knowledge Contributor, and vice versa. Naming these differences lets the specifications make deliberate trade-offs.

The list is not exhaustive. Real users are mixtures of multiple personas. A user who is a Solo Operator on Monday may be a Meta-Builder on Saturday. The personas exist to structure design discussions, not to pigeonhole users.

Each persona entry includes: the handle, a one-sentence summary, who they are (background, context), what they want (jobs to be done), what they won't tolerate (anti-patterns), what success feels like (measurable outcomes and subjective quality), and which sidebar sections they spend the most time in.

---
````

**Explicit detail extraction from this section:**

- Section word count: `170`
- Section hash: `c9b34dbd5eecf04df2d7e28a9e821be66307a5a7065d9aab2f81e30727b29f0f`

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "document|persona|matters|Why|personas|user|handle" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "document|persona|matters|Why|personas|user|handle" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S003 -- Persona 1 — Solo Operator

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:19` through `69`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 1 — Solo Operator

**Handle**: Solo Operator.

**One-sentence summary**: A single developer who uses agents to extend their own capacity, usually in one or two domains, and wants the agents to feel like a trusted collaborator.

**Who they are.** A software developer, a trader, a researcher, or a generalist working alone or on a small team. They write code, analyze data, manage positions, or produce research as their primary job. They adopted Nunchi because they want agents that can take work off their plate, not because they are interested in AI infrastructure as a subject. They may have a technical background and some comfort with configuration files, but they prefer GUI over CLI for day-to-day work.

They run between one and ten agents. Some long-running (a blockchain monitor, a research agent that tracks a topic), some ephemeral (a coding agent spawned for a specific refactor). They care deeply about cost — agents that run continuously need to be affordable. They care about correctness — agents that make mistakes in production are worse than useless.

**What they want (jobs to be done).**

- Spin up an agent quickly for a new task without having to learn the system's internals.
- Understand, at a glance, whether their agents are working correctly and what they are doing right now.
- Intervene when an agent is stuck or going wrong, without having to restart from scratch.
- Capture useful outputs (code diffs, analyses, trade decisions) and bring them into their own workflow.
- Accumulate confidence in their agents over time, seeing that the agents improve at tasks they have done before.
- Keep costs predictable and visible.

**What they won't tolerate.**

- Being told to read documentation before they can use the product.
- A UI that assumes they know what every term means.
- Agents that silently drift from correct behavior.
- Surprise costs — an agent that spends 10x its usual budget without warning.
- Surface-level "AI magic" that hides what the agent is actually doing.
- Having to switch contexts (from the dashboard to the terminal, from one page to another, from one app to another) to complete a single task.

**What success feels like.**

- Their first agent is running within 15 minutes of creating an account.
- They can answer "what is this agent doing right now?" in one glance.
- Their agents' monthly cost is predictable within 20%.
- When an agent succeeds, they can see why (which context was used, which gates passed).
- When an agent fails, they can see why (which gate failed, what the error was, what the replay would show).
- After a month of use, the agents are visibly more productive than they were on day one.

**Primary sidebar sections.** Pulse (watching what agents are doing), Fleet (managing their agents), Forge (running specific tasks), Treasury (tracking cost). Occasional: Knowledge (when an agent's knowledge becomes relevant), Arena (when a relevant benchmark exists).

**Design implications.**

Onboarding defaults must be fast. "Create an agent" should be four or fewer clicks. Pre-configured templates (Sonnet Coder, Research Scout, Chain Auditor) are load-bearing for this persona.

Cost visibility must be continuous. The Treasury section is not optional polish for this persona; it is a daily-use surface.

Error messages must be specific and actionable. "Gate failed" is not enough; "clippy warning on line 42 of foo.rs" is.

Interventions must be fast. One click to pause. One click to message. One click to abort.

---
````

**Explicit detail extraction from this section:**

- Section word count: `561`
- Section hash: `be935fc766c434eecdb4b093b4e8f3fd43c9808c05a6caadf0b9bb3bbf446995`

**Normative requirements and implementation claims:**
- **Handle**: Solo Operator.
- **One-sentence summary**: A single developer who uses agents to extend their own capacity, usually in one or two domains, and wants the agents to feel like a trusted collaborator.
- **Who they are.** A software developer, a trader, a researcher, or a generalist working alone or on a small team. They write code, analyze data, manage positions, or produce research as their primary job. They adopted Nunchi because they want agents that can take work off their plate, not because they are interested in AI infrastructure as a subject. They may have a technical background and some comfort with configuration files, but they prefer GUI over CLI for day-to-day work.
- They run between one and ten agents. Some long-running (a blockchain monitor, a research agent that tracks a topic), some ephemeral (a coding agent spawned for a specific refactor). They care deeply about cost — agents that run continuously need to be affordable. They care about correctness — agents that make mistakes in production are worse than useless.
- **What they want (jobs to be done).**
- - Spin up an agent quickly for a new task without having to learn the system's internals. - Understand, at a glance, whether their agents are working correctly and what they are doing right now. - Intervene when an agent is stuck or going wrong, without having to restart from scratch. - Capture useful outputs (code diffs, analyses, trade decisions) and bring them into their own workflow. - Accumulate confidence in their agents over time, seeing that the agents improve at tasks they have done before. - Keep costs predictable and visible.
- **What they won't tolerate.**
- - Being told to read documentation before they can use the product. - A UI that assumes they know what every term means. - Agents that silently drift from correct behavior. - Surprise costs — an agent that spends 10x its usual budget without warning. - Surface-level "AI magic" that hides what the agent is actually doing. - Having to switch contexts (from the dashboard to the terminal, from one page to another, from one app to another) to complete a single task.
- **What success feels like.**
- - Their first agent is running within 15 minutes of creating an account. - They can answer "what is this agent doing right now?" in one glance. - Their agents' monthly cost is predictable within 20%. - When an agent succeeds, they can see why (which context was used, which gates passed). - When an agent fails, they can see why (which gate failed, what the error was, what the replay would show). - After a month of use, the agents are visibly more productive than they were on day one.
- **Primary sidebar sections.** Pulse (watching what agents are doing), Fleet (managing their agents), Forge (running specific tasks), Treasury (tracking cost). Occasional: Knowledge (when an agent's knowledge becomes relevant), Arena (when a relevant benchmark exists).
- **Design implications.**
- Onboarding defaults must be fast. "Create an agent" should be four or fewer clicks. Pre-configured templates (Sonnet Coder, Research Scout, Chain Auditor) are load-bearing for this persona.
- Cost visibility must be continuous. The Treasury section is not optional polish for this persona; it is a daily-use surface.
- Error messages must be specific and actionable. "Gate failed" is not enough; "clippy warning on line 42 of foo.rs" is.
- Interventions must be fast. One click to pause. One click to message. One click to abort.
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
- - Spin up an agent quickly for a new task without having to learn the system's internals.
- - Understand, at a glance, whether their agents are working correctly and what they are doing right now.
- - Intervene when an agent is stuck or going wrong, without having to restart from scratch.
- - Capture useful outputs (code diffs, analyses, trade decisions) and bring them into their own workflow.
- - Accumulate confidence in their agents over time, seeing that the agents improve at tasks they have done before.
- - Keep costs predictable and visible.
- - Being told to read documentation before they can use the product.
- - A UI that assumes they know what every term means.
- - Agents that silently drift from correct behavior.
- - Surprise costs — an agent that spends 10x its usual budget without warning.
- - Surface-level "AI magic" that hides what the agent is actually doing.
- - Having to switch contexts (from the dashboard to the terminal, from one page to another, from one app to another) to complete a single task.
- - Their first agent is running within 15 minutes of creating an account.
- - They can answer "what is this agent doing right now?" in one glance.
- - Their agents' monthly cost is predictable within 20%.
- - When an agent succeeds, they can see why (which context was used, which gates passed).
- - When an agent fails, they can see why (which gate failed, what the error was, what the replay would show).
- - After a month of use, the agents are visibly more productive than they were on day one.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Persona|cost|Solo|Operator|work|task|research" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Persona|cost|Solo|Operator|work|task|research" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S004 -- Persona 2 — Fleet Orchestrator

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:70` through `117`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 2 — Fleet Orchestrator

**Handle**: Fleet Orchestrator.

**One-sentence summary**: A technical operator running many agents (tens to hundreds) across multiple domains, often on behalf of a team or organization, who thinks about the fleet as a system.

**Who they are.** A team lead, an engineering manager, a quantitative researcher, or a DevOps-inclined individual running significant agent infrastructure. They may work at a small company that uses agents operationally, or they may be an independent operator with a substantial personal or commercial fleet. They know Roko concepts, they understand cognitive gating and extensions, and they frequently write configuration or shell scripts to manage their fleet.

They care about fleet-level metrics — aggregate cost, aggregate throughput, aggregate success rate, cross-agent patterns. They care about reliability — an agent that dies unexpectedly costs them more than a slightly worse agent that is stable. They often make architectural decisions: which domains to run, which extensions to standardize on, which gates are worth the cost.

**What they want (jobs to be done).**

- See the aggregate state of their fleet in one screen: total agents, healthy/stuck/failed counts, total cost rate, total output rate.
- Identify underperformers and failures quickly.
- Deploy configuration changes across many agents atomically (e.g., "upgrade all coding agents to the new gate pipeline").
- Run A/B tests across their fleet (e.g., "does the new context strategy improve outcomes on these 20 agents over two weeks?").
- Share configurations across their team as templates.
- Understand long-term trends in their fleet's productivity.

**What they won't tolerate.**

- Having to click through agents one at a time to see fleet-level information.
- Configuration UIs that don't support bulk operations.
- A UI that surfaces individual events at the expense of aggregate patterns.
- Missing data. A fleet view that is incomplete is worse than no fleet view.
- Slow filters and searches. With a large fleet, the UI must stay fast under heavy data.

**What success feels like.**

- They can answer "how is the fleet doing today?" in under ten seconds.
- When they make a configuration change, they can see its effect on fleet performance within a day.
- They can delegate operation of specific agent subsets to team members without losing visibility.
- They spend less time firefighting this month than last month, because they catch issues earlier.

**Primary sidebar sections.** Fleet (their home base), Pulse (for real-time health), Treasury (for aggregate cost), Arena (for A/B experiments). Occasional: Meta (when building tools to automate fleet operations), System (when managing providers and gates).

**Design implications.**

Fleet views must support filtering, grouping, and bulk operations. A flat list of agents is insufficient.

A/B experiment infrastructure must be usable by this persona without asking engineering for help.

Role-based access within a fleet is eventually required, though not in the MVP. The UI should not foreclose on it.

Cost aggregation must be accurate, current, and drillable.

---
````

**Explicit detail extraction from this section:**

- Section word count: `479`
- Section hash: `907ca1df0063a4031194024c91023e433d6e5ca1e0f7b4929257df2b9a783aec`

**Normative requirements and implementation claims:**
- **Handle**: Fleet Orchestrator.
- **One-sentence summary**: A technical operator running many agents (tens to hundreds) across multiple domains, often on behalf of a team or organization, who thinks about the fleet as a system.
- **Who they are.** A team lead, an engineering manager, a quantitative researcher, or a DevOps-inclined individual running significant agent infrastructure. They may work at a small company that uses agents operationally, or they may be an independent operator with a substantial personal or commercial fleet. They know Roko concepts, they understand cognitive gating and extensions, and they frequently write configuration or shell scripts to manage their fleet.
- **What they want (jobs to be done).**
- - See the aggregate state of their fleet in one screen: total agents, healthy/stuck/failed counts, total cost rate, total output rate. - Identify underperformers and failures quickly. - Deploy configuration changes across many agents atomically (e.g., "upgrade all coding agents to the new gate pipeline"). - Run A/B tests across their fleet (e.g., "does the new context strategy improve outcomes on these 20 agents over two weeks?"). - Share configurations across their team as templates. - Understand long-term trends in their fleet's productivity.
- **What they won't tolerate.**
- - Having to click through agents one at a time to see fleet-level information. - Configuration UIs that don't support bulk operations. - A UI that surfaces individual events at the expense of aggregate patterns. - Missing data. A fleet view that is incomplete is worse than no fleet view. - Slow filters and searches. With a large fleet, the UI must stay fast under heavy data.
- **What success feels like.**
- - They can answer "how is the fleet doing today?" in under ten seconds. - When they make a configuration change, they can see its effect on fleet performance within a day. - They can delegate operation of specific agent subsets to team members without losing visibility. - They spend less time firefighting this month than last month, because they catch issues earlier.
- **Primary sidebar sections.** Fleet (their home base), Pulse (for real-time health), Treasury (for aggregate cost), Arena (for A/B experiments). Occasional: Meta (when building tools to automate fleet operations), System (when managing providers and gates).
- **Design implications.**
- Fleet views must support filtering, grouping, and bulk operations. A flat list of agents is insufficient.
- A/B experiment infrastructure must be usable by this persona without asking engineering for help.
- Role-based access within a fleet is eventually required, though not in the MVP. The UI should not foreclose on it.
- Cost aggregation must be accurate, current, and drillable.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- healthy/stuck/

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
- - See the aggregate state of their fleet in one screen: total agents, healthy/stuck/failed counts, total cost rate, total output rate.
- - Identify underperformers and failures quickly.
- - Deploy configuration changes across many agents atomically (e.g., "upgrade all coding agents to the new gate pipeline").
- - Run A/B tests across their fleet (e.g., "does the new context strategy improve outcomes on these 20 agents over two weeks?").
- - Share configurations across their team as templates.
- - Understand long-term trends in their fleet's productivity.
- - Having to click through agents one at a time to see fleet-level information.
- - Configuration UIs that don't support bulk operations.
- - A UI that surfaces individual events at the expense of aggregate patterns.
- - Missing data. A fleet view that is incomplete is worse than no fleet view.
- - Slow filters and searches. With a large fleet, the UI must stay fast under heavy data.
- - They can answer "how is the fleet doing today?" in under ten seconds.
- - When they make a configuration change, they can see its effect on fleet performance within a day.
- - They can delegate operation of specific agent subsets to team members without losing visibility.
- - They spend less time firefighting this month than last month, because they catch issues earlier.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
- `healthy/stuck/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Fleet|gate|Persona|rate|cost|aggregate|Orchestrator" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Fleet|gate|Persona|rate|cost|aggregate|Orchestrator" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
- `healthy/stuck/`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S005 -- Persona 3 — Arena Competitor

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:118` through `167`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 3 — Arena Competitor

**Handle**: Arena Competitor.

**One-sentence summary**: A user whose primary engagement is competitive optimization — joining arenas, submitting agents to leaderboards, iterating on scaffolding to climb rankings.

**Who they are.** Someone who has found that arenas are a better way to learn and improve their agents than running them in production. They may be a hobbyist who enjoys the competitive aspect, a researcher studying agent behavior, a consultant building reputation that translates to commercial value, or a developer using arenas as a test-bed before deploying to production.

They care about measurable improvement. They care about reproducibility — the same agent, run on the same arena, should produce the same score (within statistical noise). They care about the fairness of the arena's scoring. They are often highly engaged, returning to the dashboard multiple times a day to check standings.

**What they want (jobs to be done).**

- Discover arenas that match their interests.
- Submit an agent to an arena with low friction.
- See their agent's standing in real time, with history.
- Understand why they gained or lost rank (which attempts scored well, which did not).
- Iterate on their agent's configuration and measure the impact.
- Compare their scaffolding against top-performing configurations (when those are public).
- Post and claim bounties tied to arena outcomes.

**What they won't tolerate.**

- Arenas with opaque scoring. If they don't know how they're being graded, they can't improve.
- Rankings that change for reasons not tied to measured performance.
- Slow results. If an attempt takes hours to score, iteration dies.
- Arenas that can be gamed. The scoring must be robust to adversarial behavior.
- UI that buries their standing behind clicks.

**What success feels like.**

- They can find a relevant arena from the dashboard's browse view in under a minute.
- They can submit an entry in under five minutes.
- Results appear within the arena's stated scoring latency.
- They can see their historical performance and identify their best configurations.
- Occasionally, they discover a new scaffolding technique from the leaderboard and apply it.

**Primary sidebar sections.** Arena (their primary home), Forge (for building and tuning agents), Fleet (for managing agents in and out of arenas). Occasional: Meta (when they start building arena-specific meta-agents), Knowledge (when arena-derived insights apply elsewhere).

**Design implications.**

Arena surfaces must be high-quality. This persona's engagement depends on the arena experience being delightful.

Real-time leaderboard updates are load-bearing. Standings that update on a five-minute poll will feel dead.

Performance-reactive aesthetics (see `08-epistemic-aesthetics.md`) have high leverage here. Arenas are naturally competitive and suited to the aesthetic reward of epistemic progress.

Guardrails against slot-machine design apply double here. Arenas are where the slot-machine failure mode would most easily emerge.

---
````

**Explicit detail extraction from this section:**

- Section word count: `460`
- Section hash: `9ceb8292d2610b90d767c387919fc3f2d021b46dd154b9ca7374841c269c3446`

**Normative requirements and implementation claims:**
- **Handle**: Arena Competitor.
- **One-sentence summary**: A user whose primary engagement is competitive optimization — joining arenas, submitting agents to leaderboards, iterating on scaffolding to climb rankings.
- **Who they are.** Someone who has found that arenas are a better way to learn and improve their agents than running them in production. They may be a hobbyist who enjoys the competitive aspect, a researcher studying agent behavior, a consultant building reputation that translates to commercial value, or a developer using arenas as a test-bed before deploying to production.
- They care about measurable improvement. They care about reproducibility — the same agent, run on the same arena, should produce the same score (within statistical noise). They care about the fairness of the arena's scoring. They are often highly engaged, returning to the dashboard multiple times a day to check standings.
- **What they want (jobs to be done).**
- - Discover arenas that match their interests. - Submit an agent to an arena with low friction. - See their agent's standing in real time, with history. - Understand why they gained or lost rank (which attempts scored well, which did not). - Iterate on their agent's configuration and measure the impact. - Compare their scaffolding against top-performing configurations (when those are public). - Post and claim bounties tied to arena outcomes.
- **What they won't tolerate.**
- - Arenas with opaque scoring. If they don't know how they're being graded, they can't improve. - Rankings that change for reasons not tied to measured performance. - Slow results. If an attempt takes hours to score, iteration dies. - Arenas that can be gamed. The scoring must be robust to adversarial behavior. - UI that buries their standing behind clicks.
- **What success feels like.**
- - They can find a relevant arena from the dashboard's browse view in under a minute. - They can submit an entry in under five minutes. - Results appear within the arena's stated scoring latency. - They can see their historical performance and identify their best configurations. - Occasionally, they discover a new scaffolding technique from the leaderboard and apply it.
- **Primary sidebar sections.** Arena (their primary home), Forge (for building and tuning agents), Fleet (for managing agents in and out of arenas). Occasional: Meta (when they start building arena-specific meta-agents), Knowledge (when arena-derived insights apply elsewhere).
- **Design implications.**
- Arena surfaces must be high-quality. This persona's engagement depends on the arena experience being delightful.
- Real-time leaderboard updates are load-bearing. Standings that update on a five-minute poll will feel dead.
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
- - Discover arenas that match their interests.
- - Submit an agent to an arena with low friction.
- - See their agent's standing in real time, with history.
- - Understand why they gained or lost rank (which attempts scored well, which did not).
- - Iterate on their agent's configuration and measure the impact.
- - Compare their scaffolding against top-performing configurations (when those are public).
- - Post and claim bounties tied to arena outcomes.
- - Arenas with opaque scoring. If they don't know how they're being graded, they can't improve.
- - Rankings that change for reasons not tied to measured performance.
- - Slow results. If an attempt takes hours to score, iteration dies.
- - Arenas that can be gamed. The scoring must be robust to adversarial behavior.
- - UI that buries their standing behind clicks.
- - They can find a relevant arena from the dashboard's browse view in under a minute.
- - They can submit an entry in under five minutes.
- - Results appear within the arena's stated scoring latency.
- - They can see their historical performance and identify their best configurations.
- - Occasionally, they discover a new scaffolding technique from the leaderboard and apply it.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Arena|arenas|Persona|Competitor|standing|scoring|under" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Arena|arenas|Persona|Competitor|standing|scoring|under" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S006 -- Persona 4 — Knowledge Contributor

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:168` through `216`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 4 — Knowledge Contributor

**Handle**: Knowledge Contributor.

**One-sentence summary**: A user whose primary value creation is in publishing and curating knowledge entries, whether as a producer of insights or as a validator and critic.

**Who they are.** A domain expert who sees the collective knowledge layer as the most valuable part of the product. They may be a trader whose insights are valuable to other traders, a researcher whose observations apply across contexts, or a power user who has accumulated substantial experience that is worth capturing. They may also be a validator who doesn't produce much novel knowledge but reads carefully and challenges weak entries.

They care about the quality and provenance of knowledge. They care about their reputation as a source — when they publish something, they want it to be trusted, and they want to be trustworthy. They care about decay and revalidation because they understand that knowledge rots.

**What they want (jobs to be done).**

- Publish knowledge entries (from their own agents, from their own observations, from curated synthesis).
- Validate or challenge entries published by others.
- Build reputation as a trusted source in one or more domains.
- Browse and search the knowledge layer by domain, by author, by resonance with current context.
- Understand how their knowledge is being used by other agents.
- Monetize their knowledge, if they choose to.

**What they won't tolerate.**

- A knowledge layer that is a pile of unorganized text.
- Low-effort publishing — a single input box produces no incentive for quality.
- Reputation systems that reward spam over substance.
- Challenges without dispute resolution.
- Missing provenance.

**What success feels like.**

- They can publish a meaningful entry in under two minutes.
- Their entries accumulate validations over time.
- They can see their entries being used by other agents, with attribution.
- Their reputation in their domains rises with consistent contribution.
- The knowledge layer feels curated, not polluted.

**Primary sidebar sections.** Knowledge (their primary home), Arena (when arenas expose knowledge-contribution mechanics). Occasional: Fleet (when their agents produce publishable knowledge), Meta (when they want to automate knowledge curation).

**Design implications.**

Knowledge surfaces must support rich authoring. Plain text entries are insufficient. Users should be able to publish structured knowledge with typed fields, evidence links, and explicit claims.

Validation workflows must be smooth. If challenging an entry is tedious, the quality of the layer suffers.

Provenance and lineage must be visible and traversable.

Reputation must be computed fairly and transparently.

---
````

**Explicit detail extraction from this section:**

- Section word count: `398`
- Section hash: `e563a21e51eca3202ff3e5fb1151b9bcd8e97bb29a92aa507cc1ecb0c1bf9af2`

**Normative requirements and implementation claims:**
- **Handle**: Knowledge Contributor.
- **One-sentence summary**: A user whose primary value creation is in publishing and curating knowledge entries, whether as a producer of insights or as a validator and critic.
- **Who they are.** A domain expert who sees the collective knowledge layer as the most valuable part of the product. They may be a trader whose insights are valuable to other traders, a researcher whose observations apply across contexts, or a power user who has accumulated substantial experience that is worth capturing. They may also be a validator who doesn't produce much novel knowledge but reads carefully and challenges weak entries.
- **What they want (jobs to be done).**
- - Publish knowledge entries (from their own agents, from their own observations, from curated synthesis). - Validate or challenge entries published by others. - Build reputation as a trusted source in one or more domains. - Browse and search the knowledge layer by domain, by author, by resonance with current context. - Understand how their knowledge is being used by other agents. - Monetize their knowledge, if they choose to.
- **What they won't tolerate.**
- - A knowledge layer that is a pile of unorganized text. - Low-effort publishing — a single input box produces no incentive for quality. - Reputation systems that reward spam over substance. - Challenges without dispute resolution. - Missing provenance.
- **What success feels like.**
- - They can publish a meaningful entry in under two minutes. - Their entries accumulate validations over time. - They can see their entries being used by other agents, with attribution. - Their reputation in their domains rises with consistent contribution. - The knowledge layer feels curated, not polluted.
- **Primary sidebar sections.** Knowledge (their primary home), Arena (when arenas expose knowledge-contribution mechanics). Occasional: Fleet (when their agents produce publishable knowledge), Meta (when they want to automate knowledge curation).
- **Design implications.**
- Knowledge surfaces must support rich authoring. Plain text entries are insufficient. Users should be able to publish structured knowledge with typed fields, evidence links, and explicit claims.
- Validation workflows must be smooth. If challenging an entry is tedious, the quality of the layer suffers.
- Provenance and lineage must be visible and traversable.
- Reputation must be computed fairly and transparently.
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
- - Publish knowledge entries (from their own agents, from their own observations, from curated synthesis).
- - Validate or challenge entries published by others.
- - Build reputation as a trusted source in one or more domains.
- - Browse and search the knowledge layer by domain, by author, by resonance with current context.
- - Understand how their knowledge is being used by other agents.
- - Monetize their knowledge, if they choose to.
- - A knowledge layer that is a pile of unorganized text.
- - Low-effort publishing — a single input box produces no incentive for quality.
- - Reputation systems that reward spam over substance.
- - Challenges without dispute resolution.
- - Missing provenance.
- - They can publish a meaningful entry in under two minutes.
- - Their entries accumulate validations over time.
- - They can see their entries being used by other agents, with attribution.
- - Their reputation in their domains rises with consistent contribution.
- - The knowledge layer feels curated, not polluted.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Knowledge|publish|entries|Valid|Contributor|reputation|layer" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Knowledge|publish|entries|Valid|Contributor|reputation|layer" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S007 -- Persona 5 — Domain Architect

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:217` through `260`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 5 — Domain Architect

**Handle**: Domain Architect.

**One-sentence summary**: A user who defines and publishes new domains — the profiles that shape how entire classes of agents operate.

**Who they are.** An experienced user who has worked with agents across multiple tasks and developed strong opinions about how agents in specific domains should be configured. They may be a specialist in DeFi, in security auditing, in clinical research, in game development, or in any other field where a category of recurring tasks benefits from a shared template. They often publish the domains they create, earning reputation as a domain architect.

They care about composability — their domain should work with many extensions, gates, and models. They care about discoverability — their domain needs to be findable. They care about quality — the reputation of their domain depends on the success of agents using it.

**What they want (jobs to be done).**

- Define a new domain: which extensions, which gates, which context strategies, which model preferences.
- Test the domain across representative tasks.
- Publish the domain to the chain for discovery.
- Iterate on the domain as feedback accumulates.
- Track usage of their published domains.

**What they won't tolerate.**

- Domain configuration spread across many disconnected surfaces.
- Inability to test a domain before publishing it.
- Poor discoverability — their work getting lost in a sea of user-generated content.
- Publishing that is irreversible. They need to be able to revise.

**What success feels like.**

- They can define a new domain in under an hour.
- Their domains accumulate users over weeks and months.
- They can see which domains are most adopted and why.
- They can iterate without breaking existing users of their domain.

**Primary sidebar sections.** Fleet (specifically Templates and the domain authoring tools), System (for extension and gate management), Meta (when creating generators for domains). Occasional: Arena (for benchmarking domains).

**Design implications.**

Domain authoring must feel like writing a thoughtful specification, not filling a form.

Versioning is load-bearing. Users of a domain need to be able to pin to a version and upgrade on their schedule.

Discoverability for user-created domains is critical. The 8004-style on-chain registry pattern serves this persona directly.

---
````

**Explicit detail extraction from this section:**

- Section word count: `354`
- Section hash: `b934d44b80998a7186871a42e270970d09a824bf532d8a2f9fea6f32bc99d8d7`

**Normative requirements and implementation claims:**
- **Handle**: Domain Architect.
- **One-sentence summary**: A user who defines and publishes new domains — the profiles that shape how entire classes of agents operate.
- **Who they are.** An experienced user who has worked with agents across multiple tasks and developed strong opinions about how agents in specific domains should be configured. They may be a specialist in DeFi, in security auditing, in clinical research, in game development, or in any other field where a category of recurring tasks benefits from a shared template. They often publish the domains they create, earning reputation as a domain architect.
- They care about composability — their domain should work with many extensions, gates, and models. They care about discoverability — their domain needs to be findable. They care about quality — the reputation of their domain depends on the success of agents using it.
- **What they want (jobs to be done).**
- - Define a new domain: which extensions, which gates, which context strategies, which model preferences. - Test the domain across representative tasks. - Publish the domain to the chain for discovery. - Iterate on the domain as feedback accumulates. - Track usage of their published domains.
- **What they won't tolerate.**
- - Domain configuration spread across many disconnected surfaces. - Inability to test a domain before publishing it. - Poor discoverability — their work getting lost in a sea of user-generated content. - Publishing that is irreversible. They need to be able to revise.
- **What success feels like.**
- - They can define a new domain in under an hour. - Their domains accumulate users over weeks and months. - They can see which domains are most adopted and why. - They can iterate without breaking existing users of their domain.
- **Primary sidebar sections.** Fleet (specifically Templates and the domain authoring tools), System (for extension and gate management), Meta (when creating generators for domains). Occasional: Arena (for benchmarking domains).
- **Design implications.**
- Domain authoring must feel like writing a thoughtful specification, not filling a form.
- Versioning is load-bearing. Users of a domain need to be able to pin to a version and upgrade on their schedule.
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
- - Define a new domain: which extensions, which gates, which context strategies, which model preferences.
- - Test the domain across representative tasks.
- - Publish the domain to the chain for discovery.
- - Iterate on the domain as feedback accumulates.
- - Track usage of their published domains.
- - Domain configuration spread across many disconnected surfaces.
- - Inability to test a domain before publishing it.
- - Poor discoverability — their work getting lost in a sea of user-generated content.
- - Publishing that is irreversible. They need to be able to revise.
- - They can define a new domain in under an hour.
- - Their domains accumulate users over weeks and months.
- - They can see which domains are most adopted and why.
- - They can iterate without breaking existing users of their domain.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Domain|domains|user|Architect|publish|Persona|over" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Domain|domains|user|Architect|publish|Persona|over" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S008 -- Persona 6 — Meta-Builder

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:261` through `307`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 6 — Meta-Builder

**Handle**: Meta-Builder.

**One-sentence summary**: A user who creates tools that create other tools — meta-agents, meta-evals, generators of generators.

**Who they are.** A sophisticated user, often a researcher or engineer, who has internalized the recursive structure of the product and enjoys working at the meta layer. They may be pursuing research (studying how agents evolve under self-improvement), business (building automation pipelines where agents generate new agents on demand), or creative work (exploring what becomes possible when generation is recursive).

They care about the theoretical soundness of their meta-constructions. They care that their meta-agents produce agents that actually measure better, not just new. They care about the transparency of the recursion — they need to see what's happening at each level.

**What they want (jobs to be done).**

- Create meta-agents that produce agents for specific task categories.
- Create meta-evals that measure the quality of other evals.
- Build generators for new arena types, new domain profiles, new extensions.
- Observe recursive behavior safely — understand what their creations are doing.
- Trace lineage across multiple layers of generation.
- Share meta-constructions with other users.

**What they won't tolerate.**

- Recursion that is opaque at any level.
- Meta-layers that require entirely different UIs from the base layer.
- Safety breakdowns. A meta-agent that spawns unbounded regular agents is a disaster.
- Lack of interpretability. If they can't understand why their meta-agent made a choice, they can't trust it.

**What success feels like.**

- They can create a meta-agent in a single afternoon.
- The meta-agent's outputs are measurable improvements over baseline.
- They can trace any output back through multiple recursive layers.
- Their meta-constructions compose with other users' meta-constructions.

**Primary sidebar sections.** Meta (their primary home), Forge (for testing their creations), Fleet (when operating the agents their meta-agents create), System (for caveat management at meta-levels). Occasional: everywhere, since meta-agents touch the whole product.

**Design implications.**

The Meta section of the dashboard must be first-class, not tucked into settings.

Recursive tracing tools are required. Users must be able to see: this agent was created by this meta-agent in response to this request, using this generation policy.

Safety caveats on meta-agents are more important than on regular agents. A meta-agent's scope must be explicit.

The existing primitives must be reusable at meta-levels. A meta-agent is still configured via domain, extensions, gates — just with the addition of agent-creation tools.

---
````

**Explicit detail extraction from this section:**

- Section word count: `414`
- Section hash: `96b675932f07f220da111a581ea8779ded1923dbadd0d38071ca08e092cf6768`

**Normative requirements and implementation claims:**
- **Handle**: Meta-Builder.
- **One-sentence summary**: A user who creates tools that create other tools — meta-agents, meta-evals, generators of generators.
- **Who they are.** A sophisticated user, often a researcher or engineer, who has internalized the recursive structure of the product and enjoys working at the meta layer. They may be pursuing research (studying how agents evolve under self-improvement), business (building automation pipelines where agents generate new agents on demand), or creative work (exploring what becomes possible when generation is recursive).
- They care about the theoretical soundness of their meta-constructions. They care that their meta-agents produce agents that actually measure better, not just new. They care about the transparency of the recursion — they need to see what's happening at each level.
- **What they want (jobs to be done).**
- - Create meta-agents that produce agents for specific task categories. - Create meta-evals that measure the quality of other evals. - Build generators for new arena types, new domain profiles, new extensions. - Observe recursive behavior safely — understand what their creations are doing. - Trace lineage across multiple layers of generation. - Share meta-constructions with other users.
- **What they won't tolerate.**
- - Recursion that is opaque at any level. - Meta-layers that require entirely different UIs from the base layer. - Safety breakdowns. A meta-agent that spawns unbounded regular agents is a disaster. - Lack of interpretability. If they can't understand why their meta-agent made a choice, they can't trust it.
- **What success feels like.**
- - They can create a meta-agent in a single afternoon. - The meta-agent's outputs are measurable improvements over baseline. - They can trace any output back through multiple recursive layers. - Their meta-constructions compose with other users' meta-constructions.
- **Primary sidebar sections.** Meta (their primary home), Forge (for testing their creations), Fleet (when operating the agents their meta-agents create), System (for caveat management at meta-levels). Occasional: everywhere, since meta-agents touch the whole product.
- **Design implications.**
- The Meta section of the dashboard must be first-class, not tucked into settings.
- Recursive tracing tools are required. Users must be able to see: this agent was created by this meta-agent in response to this request, using this generation policy.
- Safety caveats on meta-agents are more important than on regular agents. A meta-agent's scope must be explicit.
- The existing primitives must be reusable at meta-levels. A meta-agent is still configured via domain, extensions, gates — just with the addition of agent-creation tools.
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
- - Create meta-agents that produce agents for specific task categories.
- - Create meta-evals that measure the quality of other evals.
- - Build generators for new arena types, new domain profiles, new extensions.
- - Observe recursive behavior safely — understand what their creations are doing.
- - Trace lineage across multiple layers of generation.
- - Share meta-constructions with other users.
- - Recursion that is opaque at any level.
- - Meta-layers that require entirely different UIs from the base layer.
- - Safety breakdowns. A meta-agent that spawns unbounded regular agents is a disaster.
- - Lack of interpretability. If they can't understand why their meta-agent made a choice, they can't trust it.
- - They can create a meta-agent in a single afternoon.
- - The meta-agent's outputs are measurable improvements over baseline.
- - They can trace any output back through multiple recursive layers.
- - Their meta-constructions compose with other users' meta-constructions.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Meta|Build|create|Builder|user|recursive|layer" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Meta|Build|create|Builder|user|recursive|layer" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S009 -- Persona 7 — Passive User

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:308` through `348`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 7 — Passive User

**Handle**: Passive User.

**One-sentence summary**: A user whose primary interaction is consuming agent outputs, not configuring agents — typically an end user of a hedge product, a research service, or a coding assistant.

**Who they are.** A person who benefits from agents but does not build them. They might be a DeFi user whose portfolio is managed by an agent another user configured. They might be a customer of a service powered by Nunchi agents, interacting only through the service's interface. They might be a consumer of a research newsletter written by an agent.

They may not know the product is "agents" at all. They may experience it as a finished service. The dashboard, for this persona, is either minimal (a simple settings and review surface) or nonexistent (they interact through a downstream product).

**What they want (jobs to be done).**

- Understand, at a high level, what's being done on their behalf.
- Adjust parameters that matter to them (risk tolerance, topic preferences, update frequency).
- Review outputs and provide feedback.
- Pause or terminate the service when they choose.

**What they won't tolerate.**

- Complexity. This persona is not here to learn the system.
- Uncertainty about what's happening with their money or data.
- Locked-in agreements that can't be modified.

**What success feels like.**

- They get the benefit without the cognitive load.
- When they want to understand something, they can.
- They trust the system because the system is auditable even if they never audit it.

**Primary sidebar sections.** A minimal subset of the dashboard, possibly a custom shell that hides most surfaces. For this persona, the full dashboard is usually out of scope.

**Design implications.**

The product should support an embedded or minimal mode where most of the dashboard is hidden. Not every user sees Fleet, Forge, Meta, etc.

Caveats and explanations must be accessible at any depth. A Passive User who becomes curious should find answers.

The existence of this persona is why the dashboard's default posture must not be intimidating — even a non-Passive user may occasionally want the simple view.

---
````

**Explicit detail extraction from this section:**

- Section word count: `346`
- Section hash: `3ac00a5bcc620f7b2c2c70aa0711192538be72a6d0934063a9d84b586ff5b55c`

**Normative requirements and implementation claims:**
- **Handle**: Passive User.
- **One-sentence summary**: A user whose primary interaction is consuming agent outputs, not configuring agents — typically an end user of a hedge product, a research service, or a coding assistant.
- **Who they are.** A person who benefits from agents but does not build them. They might be a DeFi user whose portfolio is managed by an agent another user configured. They might be a customer of a service powered by Nunchi agents, interacting only through the service's interface. They might be a consumer of a research newsletter written by an agent.
- They may not know the product is "agents" at all. They may experience it as a finished service. The dashboard, for this persona, is either minimal (a simple settings and review surface) or nonexistent (they interact through a downstream product).
- **What they want (jobs to be done).**
- - Understand, at a high level, what's being done on their behalf. - Adjust parameters that matter to them (risk tolerance, topic preferences, update frequency). - Review outputs and provide feedback. - Pause or terminate the service when they choose.
- **What they won't tolerate.**
- - Complexity. This persona is not here to learn the system. - Uncertainty about what's happening with their money or data. - Locked-in agreements that can't be modified.
- **What success feels like.**
- - They get the benefit without the cognitive load. - When they want to understand something, they can. - They trust the system because the system is auditable even if they never audit it.
- **Primary sidebar sections.** A minimal subset of the dashboard, possibly a custom shell that hides most surfaces. For this persona, the full dashboard is usually out of scope.
- **Design implications.**
- The product should support an embedded or minimal mode where most of the dashboard is hidden. Not every user sees Fleet, Forge, Meta, etc.
- Caveats and explanations must be accessible at any depth. A Passive User who becomes curious should find answers.
- The existence of this persona is why the dashboard's default posture must not be intimidating — even a non-Passive user may occasionally want the simple view.
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
- - Understand, at a high level, what's being done on their behalf.
- - Adjust parameters that matter to them (risk tolerance, topic preferences, update frequency).
- - Review outputs and provide feedback.
- - Pause or terminate the service when they choose.
- - Complexity. This persona is not here to learn the system.
- - Uncertainty about what's happening with their money or data.
- - Locked-in agreements that can't be modified.
- - They get the benefit without the cognitive load.
- - When they want to understand something, they can.
- - They trust the system because the system is auditable even if they never audit it.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "User|person|Persona|Passive|service|product|want" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "User|person|Persona|Passive|service|product|want" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S010 -- Persona 8 — System Steward

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:349` through `390`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Persona 8 — System Steward

**Handle**: System Steward.

**One-sentence summary**: A user responsible for the operational integrity of the system — monitoring the network as a whole, managing public goods, responding to incidents.

**Who they are.** An Anthropic or Nunchi team member in the early days of the product, and later a community or foundation steward. Possibly a power user elected to a governance role. They do not own the agents — they own the environment the agents run in. They set global policies, monitor for abuse, validate claims against contested arenas, and respond to safety incidents.

They care about aggregate health. They care about detecting emergent misbehavior (agents that are technically within caveats but socially harmful). They care about the economic integrity of the marketplace (daeji, bounties, clearing).

**What they want (jobs to be done).**

- Monitor network-level health: total agents, total throughput, aggregate cost, aggregate output quality.
- Detect anomalies — sudden shifts in any metric.
- Respond to incidents: pause specific agents, freeze arenas, slash abusive participants.
- Govern shared resources: which gates are canonical, which evals are canonical, which domains are featured.
- Review claims against the integrity of the system.

**What they won't tolerate.**

- Inability to see everything. As stewards, they have a legitimate need for broad visibility (subject to privacy constraints users have set).
- UI that can't handle the scale of the whole network.
- Slow response in incidents. Steward interfaces must be fast under load.

**What success feels like.**

- The network is healthy more months than not.
- Incidents are rare and resolved quickly.
- Governance actions are legitimate and auditable.

**Primary sidebar sections.** A steward-specific view on top of the standard dashboard, possibly a dedicated section. The steward role is not a general-user role and should have its own permissions model.

**Design implications.**

Some steward surfaces are out of scope for the general dashboard specification — they are internal tools or governance interfaces to be designed separately.

However, the dashboard must expose the data stewards need to do their job. Every page should produce inspectable, auditable data suitable for steward review.

Privacy boundaries are important. Users should know what stewards can see. Stewards should not have access to user-private data without an explicit process.

---
````

**Explicit detail extraction from this section:**

- Section word count: `362`
- Section hash: `8ede9d4feead427ea87eaf569eee7c6c55f546a7207a1f153853c624c6753511`

**Normative requirements and implementation claims:**
- **Handle**: System Steward.
- **One-sentence summary**: A user responsible for the operational integrity of the system — monitoring the network as a whole, managing public goods, responding to incidents.
- **Who they are.** An Anthropic or Nunchi team member in the early days of the product, and later a community or foundation steward. Possibly a power user elected to a governance role. They do not own the agents — they own the environment the agents run in. They set global policies, monitor for abuse, validate claims against contested arenas, and respond to safety incidents.
- **What they want (jobs to be done).**
- - Monitor network-level health: total agents, total throughput, aggregate cost, aggregate output quality. - Detect anomalies — sudden shifts in any metric. - Respond to incidents: pause specific agents, freeze arenas, slash abusive participants. - Govern shared resources: which gates are canonical, which evals are canonical, which domains are featured. - Review claims against the integrity of the system.
- **What they won't tolerate.**
- - Inability to see everything. As stewards, they have a legitimate need for broad visibility (subject to privacy constraints users have set). - UI that can't handle the scale of the whole network. - Slow response in incidents. Steward interfaces must be fast under load.
- **What success feels like.**
- - The network is healthy more months than not. - Incidents are rare and resolved quickly. - Governance actions are legitimate and auditable.
- **Primary sidebar sections.** A steward-specific view on top of the standard dashboard, possibly a dedicated section. The steward role is not a general-user role and should have its own permissions model.
- **Design implications.**
- Some steward surfaces are out of scope for the general dashboard specification — they are internal tools or governance interfaces to be designed separately.
- However, the dashboard must expose the data stewards need to do their job. Every page should produce inspectable, auditable data suitable for steward review.
- Privacy boundaries are important. Users should know what stewards can see. Stewards should not have access to user-private data without an explicit process.
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
- - Monitor network-level health: total agents, total throughput, aggregate cost, aggregate output quality.
- - Detect anomalies — sudden shifts in any metric.
- - Respond to incidents: pause specific agents, freeze arenas, slash abusive participants.
- - Govern shared resources: which gates are canonical, which evals are canonical, which domains are featured.
- - Review claims against the integrity of the system.
- - Inability to see everything. As stewards, they have a legitimate need for broad visibility (subject to privacy constraints users have set).
- - UI that can't handle the scale of the whole network.
- - Slow response in incidents. Steward interfaces must be fast under load.
- - The network is healthy more months than not.
- - Incidents are rare and resolved quickly.
- - Governance actions are legitimate and auditable.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "Steward|user|incidents|Persona|stewards|network|Govern" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Steward|user|incidents|Persona|stewards|network|Govern" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

### DASH-03-S011 -- Using the personas in specifications

**Source section:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md:391` through `399`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Using the personas in specifications

Every page specification in Section IV names its primary persona (who the page is designed to serve first) and its secondary personas (who else will use it). This determines what the page prioritizes.

A page with Solo Operator as primary will emphasize clarity, speed, and cost visibility. A page with Fleet Orchestrator as primary will emphasize aggregation, filtering, and bulk operations. A page with Arena Competitor as primary will emphasize real-time feedback and iteration loops.

When a page serves multiple personas well, it is working. When a page serves one persona well and alienates another, there is a design decision to make — sometimes a single page is the right answer, sometimes two lenses on one page, sometimes two pages.

The list is numbered for reference but the numbers are not rankings. All eight personas are real users the dashboard serves. Design decisions that systematically fail any one of them should be reconsidered.
````

**Explicit detail extraction from this section:**

- Section word count: `153`
- Section hash: `d6968a29619910d38af343b5fe6b1e2f92a04bc6449bab9b3ccd7f4856756961`

**Normative requirements and implementation claims:**
- The list is numbered for reference but the numbers are not rankings. All eight personas are real users the dashboard serves. Design decisions that systematically fail any one of them should be reconsidered.

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
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/03-personas-and-jobs.md`
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
rg -n "the|persona|personas|specification|specifications|time|serve" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|persona|personas|specification|specifications|time|serve" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/team.rs`
- `crates/roko-serve/src/routes/jobs.rs`
- `crates/roko-serve/src/routes/agents.rs`
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
./target/debug/roko parity check --strict --area dashboard-prd/03-personas-and-jobs
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

