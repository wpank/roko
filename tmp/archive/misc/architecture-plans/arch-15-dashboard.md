# Architecture Plan: Dashboard

**Source:** `tmp/architecture/15-dashboard.md`
**Generated:** 2026-04-25
**Source hash:** `440e485dc3ca2f36f277c31109bf35ab219dcb8d10e74bd66a9284f7ab3433da`
**Section tasks:** 24
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
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-15-S001 | 1 | Dashboard architecture | [ ] | 9.8 |
| ARCH-15-S002 | 15 | Three-tier deployment model | [ ] | 9.8 |
| ARCH-15-S003 | 29 | Per-agent sidecar access | [ ] | 9.8 |
| ARCH-15-S004 | 38 | Data layer | [ ] | 9.8 |
| ARCH-15-S005 | 68 | Page-to-data-source mapping | [ ] | 9.8 |
| ARCH-15-S006 | 90 | Chain feed integration | [ ] | 9.8 |
| ARCH-15-S007 | 102 | Adaptive information density | [ ] | 9.8 |
| ARCH-15-S008 | 124 | Progressive disclosure | [ ] | 9.8 |
| ARCH-15-S009 | 149 | Epistemic aesthetics | [ ] | 9.8 |
| ARCH-15-S010 | 164 | Layout | [ ] | 9.8 |
| ARCH-15-S011 | 211 | Feeds page | [ ] | 9.8 |
| ARCH-15-S012 | 247 | Settings page | [ ] | 9.8 |
| ARCH-15-S013 | 287 | Tech stack | [ ] | 9.8 |
| ARCH-15-S014 | 300 | Performance targets | [ ] | 9.8 |
| ARCH-15-S015 | 313 | Appendix: API surface | [ ] | 9.8 |
| ARCH-15-S016 | 315 | Agent lifecycle | [ ] | 9.8 |
| ARCH-15-S017 | 332 | Clusters | [ ] | 9.8 |
| ARCH-15-S018 | 342 | Inference gateway | [ ] | 9.8 |
| ARCH-15-S019 | 351 | Feeds | [ ] | 9.8 |
| ARCH-15-S020 | 360 | Secrets | [ ] | 9.8 |
| ARCH-15-S021 | 369 | Auth | [ ] | 9.8 |
| ARCH-15-S022 | 384 | WebSocket | [ ] | 9.8 |
| ARCH-15-S023 | 393 | Infrastructure | [ ] | 9.8 |
| ARCH-15-S024 | 403 | Existing routes | [ ] | 9.8 |

## Tasks

### ARCH-15-S001 -- Dashboard architecture

**Source section:** `tmp/architecture/15-dashboard.md:1` through `14`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Dashboard architecture

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Includes the API surface section as an appendix.

---

The dashboard works in two modes:

- **Backbone only**: connected to relay. Shows agent presence, chain data, feeds. No plans, PRDs, or learning -- those require roko-serve.
- **Full mode**: connected to relay AND roko-serve. Shows everything.

The UI gracefully degrades. When roko-serve is unreachable, workspace tabs (Plans, PRDs, Learning) show "Connect to a roko workspace to use this feature" instead of an error.
````

**Explicit detail extraction from this section:**

- Section word count: `86`
- Section hash: `0e269b69defa87573b9e87d7aeda466e8cba3b87c9d0c89790a74ca9ab83b9d9`

**Normative requirements and implementation claims:**
- > Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc. > Includes the API surface section as an appendix.
- ---
- The dashboard works in two modes:
- - **Backbone only**: connected to relay. Shows agent presence, chain data, feeds. No plans, PRDs, or learning -- those require roko-serve. - **Full mode**: connected to relay AND roko-serve. Shows everything.

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
- - **Backbone only**: connected to relay. Shows agent presence, chain data, feeds. No plans, PRDs, or learning -- those require roko-serve.
- - **Full mode**: connected to relay AND roko-serve. Shows everything.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "works|show|serve|Connect|workspace|relay|plans|mode" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "works|show|serve|Connect|workspace|relay|plans|mode" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S002 -- Three-tier deployment model

**Source section:** `tmp/architecture/15-dashboard.md:15` through `28`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Three-tier deployment model

> See `architecture-cross-reference.md` (section 1) for the full deployment model and conflict analysis.

The dashboard talks to three independent infrastructure tiers, each of which can operate independently:

| Tier | Components | Always required |
|------|-----------|-----------------|
| **Tier 1 -- Backbone** | Mirage chain (dev) / Korai (prod), relay, indexer | Yes (provides coordination substrate) |
| **Tier 2 -- Workspace** | roko-serve (HTTP API + WS), roko FS | No (enables agent lifecycle, plans, learning) |
| **Tier 3 -- Remote agents** | Per-agent sidecars on Fly/Railway | No (enables isolated cloud agents) |

**Conflict identified (cross-reference doc):** The `BackendOfflineBanner` component currently shows a single "backend offline" message. It does not distinguish between mirage down, roko down, or both down. The banner needs three-state detection: chain status, roko status, and remote agent status.
````

**Explicit detail extraction from this section:**

- Section word count: `119`
- Section hash: `8791f5a130e8c17849a544dae99e688283535a2bfb293fb55069a7ef42c00393`

**Normative requirements and implementation claims:**
- The dashboard talks to three independent infrastructure tiers, each of which can operate independently:
- | Tier | Components | Always required | |------|-----------|-----------------| | **Tier 1 -- Backbone** | Mirage chain (dev) / Korai (prod), relay, indexer | Yes (provides coordination substrate) | | **Tier 2 -- Workspace** | roko-serve (HTTP API + WS), roko FS | No (enables agent lifecycle, plans, learning) | | **Tier 3 -- Remote agents** | Per-agent sidecars on Fly/Railway | No (enables isolated cloud agents) |
- **Conflict identified (cross-reference doc):** The `BackendOfflineBanner` component currently shows a single "backend offline" message. It does not distinguish between mirage down, roko down, or both down. The banner needs three-state detection: chain status, roko status, and remote agent status.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- BackendOfflineBanner

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- architecture-cross-reference.md

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Tier | Components | Always required |
|------|-----------|-----------------|
| **Tier 1 -- Backbone** | Mirage chain (dev) / Korai (prod), relay, indexer | Yes (provides coordination substrate) |
| **Tier 2 -- Workspace** | roko-serve (HTTP API + WS), roko FS | No (enables agent lifecycle, plans, learning) |
| **Tier 3 -- Remote agents** | Per-agent sidecars on Fly/Railway | No (enables isolated cloud agents) |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "tier|Three|model|deployment|BackendOfflineBanner|status|down|reference" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "tier|Three|model|deployment|BackendOfflineBanner|status|down|reference" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- [ ] Implement or verify `BackendOfflineBanner` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `architecture-cross-reference.md` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S003 -- Per-agent sidecar access

**Source section:** `tmp/architecture/15-dashboard.md:29` through `37`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Per-agent sidecar access

Per-agent sidecar endpoints (`/agent/status`, `/agent/config`, `/agent/events`, etc.) are accessed through the roko-serve proxy, not via direct dashboard-to-sidecar connections. This is the resolved architecture from the cross-reference doc:

- The dashboard connects to roko-serve (Tier 2) only
- roko-serve proxies requests to per-agent sidecars (Tier 3) on the dashboard's behalf
- This avoids CORS issues, simplifies auth (single Tier 2 token), and prevents the dashboard from tracking per-agent URLs
- The proxy routes are: `GET /api/agents/:id/sidecar/*` -> forwards to `{sidecar_url}/*`
````

**Explicit detail extraction from this section:**

- Section word count: `90`
- Section hash: `6cd4d41b9b2747af9cb8bcc1db8f48581a314ce5532ad49366231e62d339cba5`

**Normative requirements and implementation claims:**
- Per-agent sidecar endpoints (`/agent/status`, `/agent/config`, `/agent/events`, etc.) are accessed through the roko-serve proxy, not via direct dashboard-to-sidecar connections. This is the resolved architecture from the cross-reference doc:
- - The dashboard connects to roko-serve (Tier 2) only - roko-serve proxies requests to per-agent sidecars (Tier 3) on the dashboard's behalf - This avoids CORS issues, simplifies auth (single Tier 2 token), and prevents the dashboard from tracking per-agent URLs - The proxy routes are: `GET /api/agents/:id/sidecar/*` -> forwards to `{sidecar_url}/*`

**Routes and endpoint references:**
- GET /api/agents/:id/sidecar/

**Files and path references:**
- api/agents/
- id/sidecar/

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
- - The dashboard connects to roko-serve (Tier 2) only
- - roko-serve proxies requests to per-agent sidecars (Tier 3) on the dashboard's behalf
- - This avoids CORS issues, simplifies auth (single Tier 2 token), and prevents the dashboard from tracking per-agent URLs
- - The proxy routes are: `GET /api/agents/:id/sidecar/*` -> forwards to `{sidecar_url}/*`

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/agents/`
- `id/sidecar/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "sidecar|Per|access|serve|Tier|proxy|events|tracking" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "sidecar|Per|access|serve|Tier|proxy|events|tracking" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/agents/`
- `id/sidecar/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `GET /api/agents/:id/sidecar/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S004 -- Data layer

**Source section:** `tmp/architecture/15-dashboard.md:38` through `67`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Data layer

Three components manage the flow from WebSocket events to rendered pixels.

**SubscriptionManager**: Multiplexes connections to agent, chain, relay, and workspace event streams. Each dashboard page declares its subscriptions on mount and releases them on unmount. The manager maintains a single WebSocket to the relay and a single WebSocket to roko-serve, using room-based subscription messages to filter server-side.

```typescript
// Per-page lifecycle
function useDashboardSubscriptions(rooms: string[]) {
  const manager = useSubscriptionManager();

  useEffect(() => {
    manager.subscribe(rooms);
    return () => manager.unsubscribe(rooms);
  }, [rooms]);
}
```

**EventAggregator**: Batches burst events with a 100ms flush window. High-frequency sources (heartbeats at 100ms, chain blocks at 2s) produce more events than the DOM can absorb per frame. The aggregator collects events during the flush window and delivers them as a single batch. A ring buffer (200 events) supports replay for components that mount after events have already fired.

**RenderScheduler**: Coordinates DOM and canvas updates. DOM updates are coalesced and applied in rAF callbacks. Canvas/WebGL renders (Three.js visualizations, real-time charts) run at 60fps on a separate requestAnimationFrame loop. The scheduler prevents DOM thrashing by batching state changes from the EventAggregator into single React renders.

**Three-tier motion system**:

| Tier | Source | Visual expression |
|------|--------|-------------------|
| Heartbeat rhythm | Per-agent heartbeat ticks (100ms-1s) | Agent card pulse, glow intensity |
| Event-paced tickers | Chain blocks (~2s), gate results, task completions | Counter increments, progress bar steps |
| Ambient decay | Knowledge staleness (Ebbinghaus curve), feed inactivity | Fade, desaturation, visual aging |
````

**Explicit detail extraction from this section:**

- Section word count: `236`
- Section hash: `80d3446c6c7bce042d536f9536bf8f170a8d228fbe512a78035e29f4a469a75c`

**Normative requirements and implementation claims:**
- **SubscriptionManager**: Multiplexes connections to agent, chain, relay, and workspace event streams. Each dashboard page declares its subscriptions on mount and releases them on unmount. The manager maintains a single WebSocket to the relay and a single WebSocket to roko-serve, using room-based subscription messages to filter server-side.
- **EventAggregator**: Batches burst events with a 100ms flush window. High-frequency sources (heartbeats at 100ms, chain blocks at 2s) produce more events than the DOM can absorb per frame. The aggregator collects events during the flush window and delivers them as a single batch. A ring buffer (200 events) supports replay for components that mount after events have already fired.
- **RenderScheduler**: Coordinates DOM and canvas updates. DOM updates are coalesced and applied in rAF callbacks. Canvas/WebGL renders (Three.js visualizations, real-time charts) run at 60fps on a separate requestAnimationFrame loop. The scheduler prevents DOM thrashing by batching state changes from the EventAggregator into single React renders.
- **Three-tier motion system**:
- | Tier | Source | Visual expression | |------|--------|-------------------| | Heartbeat rhythm | Per-agent heartbeat ticks (100ms-1s) | Agent card pulse, glow intensity | | Event-paced tickers | Chain blocks (~2s), gate results, task completions | Counter increments, progress bar steps | | Ambient decay | Knowledge staleness (Ebbinghaus curve), feed inactivity | Fade, desaturation, visual aging |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- manager.subscribe
- manager.unsubscribe

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
| Tier | Source | Visual expression |
|------|--------|-------------------|
| Heartbeat rhythm | Per-agent heartbeat ticks (100ms-1s) | Agent card pulse, glow intensity |
| Event-paced tickers | Chain blocks (~2s), gate results, task completions | Counter increments, progress bar steps |
| Ambient decay | Knowledge staleness (Ebbinghaus curve), feed inactivity | Fade, desaturation, visual aging |
```

**Data/code contracts extracted:**
- Contract 1: language `typescript`, first line `// Per-page lifecycle`

```typescript
// Per-page lifecycle
function useDashboardSubscriptions(rooms: string[]) {
  const manager = useSubscriptionManager();

  useEffect(() => {
    manager.subscribe(rooms);
    return () => manager.unsubscribe(rooms);
  }, [rooms]);
}
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "event|manage|events|manager|subscription|room|layer|Data" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "event|manage|events|manager|subscription|room|layer|Data" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- [ ] Emit or consume `manager.subscribe` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `manager.unsubscribe` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S005 -- Page-to-data-source mapping

**Source section:** `tmp/architecture/15-dashboard.md:68` through `89`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Page-to-data-source mapping

Every dashboard section subscribes to specific WebSocket rooms and event types. REST fallbacks provide initial state on page load when WebSocket history is insufficient.

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Pulse / Command center | `system`, `agent:*:heartbeat` | `heartbeat_aggregate`, `agent_status`, `presence_join`, `presence_leave` | `GET /relay/api/agents` |
| Pulse / Live console | `agent:*`, `agent:*:heartbeat` | `heartbeat`, `output_chunk`, `gate_result` | -- |
| Pulse / Event stream | `system`, `agent:*` | All event types (filtered client-side by user selection) | -- |
| Fleet / Agent fleet | `system` | `presence_join`, `presence_leave`, `heartbeat_aggregate` | `GET /relay/api/agents` |
| Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:output`, `agent:{id}:trace` | `heartbeat`, `output_chunk`, `gate_result`, `trace`, `cost_update` | -- |
| Forge / Plans | `plan:*` | `task_started`, `task_completed`, `phase_transition` | `GET /api/plans` |
| Forge / Execution | `plan:*`, `agent:*` | `task_started`, `task_completed`, `gate_result`, `output_chunk` | `GET /api/plans/:id` |
| Knowledge / Store | `chain:knowledge` | `knowledge_published`, `knowledge_validated`, `knowledge_challenged` | `GET /mirage/api/knowledge` |
| Knowledge / Stigmergy | `chain:pheromones` | `pheromone_deposited`, `pheromone_decayed` | `GET /mirage/api/pheromones` |
| Treasury / ISFR | `chain:isfr` | `isfr_updated`, `rate_changed` | `GET /mirage/api/isfr` |
| Treasury / Positions | `agent:{id}` | `position_opened`, `position_closed`, `pnl_update` | -- |
| Treasury / Cost | `system` | `cost_update`, per-request from gateway WS | `GET /api/gateway/stats` |
| Arena / Leaderboard | `arena:{id}` | `attempt_completed`, `score_updated` | `GET /api/arenas/:id` |
| System / Providers | `system` | `provider_status`, per-request from gateway WS | `GET /api/gateway/stats` |
| System / Jobs | `system` | `job_created`, `job_assigned`, `job_completed` | `GET /api/jobs` |
````

**Explicit detail extraction from this section:**

- Section word count: `202`
- Section hash: `e050f2d059234b23dac2d3da314cc4dab4ce1c6e25f185c32d9f37cc48a39be1`

**Normative requirements and implementation claims:**
- Every dashboard section subscribes to specific WebSocket rooms and event types. REST fallbacks provide initial state on page load when WebSocket history is insufficient.
- | Section | WS rooms | Event types | REST fallback | |---------|----------|-------------|---------------| | Pulse / Command center | `system`, `agent:*:heartbeat` | `heartbeat_aggregate`, `agent_status`, `presence_join`, `presence_leave` | `GET /relay/api/agents` | | Pulse / Live console | `agent:*`, `agent:*:heartbeat` | `heartbeat`, `output_chunk`, `gate_result` | -- | | Pulse / Event stream | `system`, `agent:*` | All event types (filtered client-side by user selection) | -- | | Fleet / Agent fleet | `system` | `presence_join`, `presence_leave`, `heartbeat_aggregate` | `GET /relay/api/agents` | | Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:output`, `agent:{id}:trace` | `heartbeat`, `output_chunk`, `gate_result`, `trace`, `cost_update` | -- | | Forge / Plans | `plan:*` | `task_started`, `task_completed`, `phase_transition` | `GET /api/plans` | | Forge / Execution | `plan:*`, `agent:*` | `task_started`, `task_completed`, `gate_result`, `output_chunk` | `GE

**Routes and endpoint references:**
- GET /api/plans
- GET /api/plans/:id
- GET /api/gateway/stats
- GET /api/arenas/:id
- GET /api/jobs

**Files and path references:**
- api/arenas/
- api/gateway/
- api/plans/
- mirage/api/
- relay/api/

**Types, functions, traits, and inline code identifiers:**
- system
- heartbeat_aggregate
- agent_status
- presence_join
- presence_leave
- heartbeat
- output_chunk
- gate_result
- trace
- cost_update
- task_started
- task_completed
- phase_transition
- knowledge_published
- knowledge_validated
- knowledge_challenged
- pheromone_deposited
- pheromone_decayed
- isfr_updated
- rate_changed
- position_opened
- position_closed
- pnl_update
- attempt_completed
- score_updated
- provider_status
- job_created
- job_assigned
- job_completed

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
| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Pulse / Command center | `system`, `agent:*:heartbeat` | `heartbeat_aggregate`, `agent_status`, `presence_join`, `presence_leave` | `GET /relay/api/agents` |
| Pulse / Live console | `agent:*`, `agent:*:heartbeat` | `heartbeat`, `output_chunk`, `gate_result` | -- |
| Pulse / Event stream | `system`, `agent:*` | All event types (filtered client-side by user selection) | -- |
| Fleet / Agent fleet | `system` | `presence_join`, `presence_leave`, `heartbeat_aggregate` | `GET /relay/api/agents` |
| Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:output`, `agent:{id}:trace` | `heartbeat`, `output_chunk`, `gate_result`, `trace`, `cost_update` | -- |
| Forge / Plans | `plan:*` | `task_started`, `task_completed`, `phase_transition` | `GET /api/plans` |
| Forge / Execution | `plan:*`, `agent:*` | `task_started`, `task_completed`, `gate_result`, `output_chunk` | `GET /api/plans/:id` |
| Knowledge / Store | `chain:knowledge` | `knowledge_published`, `knowledge_validated`, `knowledge_challenged` | `GET /mirage/api/knowledge` |
| Knowledge / Stigmergy | `chain:pheromones` | `pheromone_deposited`, `pheromone_decayed` | `GET /mirage/api/pheromones` |
| Treasury / ISFR | `chain:isfr` | `isfr_updated`, `rate_changed` | `GET /mirage/api/isfr` |
...
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/arenas/`
- `api/gateway/`
- `api/plans/`
- `mirage/api/`
- `relay/api/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "heartbeat|api|GET|output_chunk|gate_result|Knowledge|trace|task_started" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "heartbeat|api|GET|output_chunk|gate_result|Knowledge|trace|task_started" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/arenas/`
- `api/gateway/`
- `api/plans/`
- `mirage/api/`
- `relay/api/`
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
- [ ] Implement or verify route `GET /api/plans` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/plans/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/gateway/stats` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/arenas/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/jobs` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `system` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `heartbeat_aggregate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `agent_status` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `presence_join` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `presence_leave` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `heartbeat` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `output_chunk` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `gate_result` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `trace` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `cost_update` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `task_started` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `task_completed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `phase_transition` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `knowledge_published` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `knowledge_validated` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `knowledge_challenged` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pheromone_deposited` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pheromone_decayed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `isfr_updated` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `rate_changed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `position_opened` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `position_closed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pnl_update` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `attempt_completed` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `score_updated` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S006 -- Chain feed integration

**Source section:** `tmp/architecture/15-dashboard.md:90` through `101`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Chain feed integration

The dashboard consumes agent-exposed chain feeds defined in the [Feeds and Data Streams](05-feeds.md) section.

**Agent list**: Each agent's card shows its registered feeds (from the relay feed registry). A chain agent with three feeds shows three small feed indicators with live rates.

**Agent detail page**: When viewing a single agent, its active chain feeds are displayed as live data panels -- raw RPC data, derived indicators, signals, and analysis. This is the same data the agent is processing, made visible to the operator.

**Treasury pages**: Raw and derived price/rate feeds from chain agents flow directly into Treasury views. ISFR rates, position P&L, and cost data all arrive via WebSocket subscription. No polling.

**Feed subscription rule**: All chain data flows through the relay's WebSocket room system (`agent:{id}:feed:{feed_id}`). The dashboard subscribes on page mount and unsubscribes on unmount. Feeds are never polled.
````

**Explicit detail extraction from this section:**

- Section word count: `150`
- Section hash: `90acedae110462f50bc13f6a9cbc7fe339539cacab03de60e8278d20f92cc35c`

**Normative requirements and implementation claims:**
- The dashboard consumes agent-exposed chain feeds defined in the [Feeds and Data Streams](05-feeds.md) section.
- **Agent list**: Each agent's card shows its registered feeds (from the relay feed registry). A chain agent with three feeds shows three small feed indicators with live rates.
- **Agent detail page**: When viewing a single agent, its active chain feeds are displayed as live data panels -- raw RPC data, derived indicators, signals, and analysis. This is the same data the agent is processing, made visible to the operator.
- **Treasury pages**: Raw and derived price/rate feeds from chain agents flow directly into Treasury views. ISFR rates, position P&L, and cost data all arrive via WebSocket subscription. No polling.
- **Feed subscription rule**: All chain data flows through the relay's WebSocket room system (`agent:{id}:feed:{feed_id}`). The dashboard subscribes on page mount and unsubscribes on unmount. Feeds are never polled.

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
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "feed|Chain|feeds|Data|integration|rate|three|subscription" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "feed|Chain|feeds|Data|integration|rate|three|subscription" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S007 -- Adaptive information density

**Source section:** `tmp/architecture/15-dashboard.md:102` through `123`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Adaptive information density

The dashboard adjusts its information density based on the system's `CorticalState`. Three display regimes:

```
Regime      Trigger                     What changes
──────      ───────                     ────────────
Cruise      All agents calm,            Minimal display. Green status dots.
(calm)      no active plans,            Aggregated metrics only. Agent cards
            PE < 0.15 avg               collapsed to single-line summaries.

Volatile    1+ agents in T2,            Affected agents expand automatically.
            active gate failures,       Healthy agents stay collapsed.
            PE 0.15-0.40 avg            Event stream highlights anomalies.

Crisis      Multiple gate failures,     Full traces visible. Remediation
            agent errors,               suggestions shown inline. Per-tick
            PE > 0.40 avg               timeline appears. All agents expand.
```

The regime transitions smoothly (CSS transitions, not hard cuts). The dashboard computes the regime from the aggregate cortical state of all connected agents, updated on each heartbeat aggregate event.
````

**Explicit detail extraction from this section:**

- Section word count: `125`
- Section hash: `5b8597f63d08eec407c40e5bcf609a6eca0daf14a81f4a91ec80f66dd49225d1`

**Normative requirements and implementation claims:**
- The dashboard adjusts its information density based on the system's `CorticalState`. Three display regimes:
- ``` Regime Trigger What changes ────── ─────── ──────────── Cruise All agents calm, Minimal display. Green status dots. (calm) no active plans, Aggregated metrics only. Agent cards PE < 0.15 avg collapsed to single-line summaries.
- Volatile 1+ agents in T2, Affected agents expand automatically. active gate failures, Healthy agents stay collapsed. PE 0.15-0.40 avg Event stream highlights anomalies.
- The regime transitions smoothly (CSS transitions, not hard cuts). The dashboard computes the regime from the aggregate cortical state of all connected agents, updated on each heartbeat aggregate event.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- CorticalState

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
- Contract 1: language `plain`, first line `Regime      Trigger                     What changes`

```
Regime      Trigger                     What changes
──────      ───────                     ────────────
Cruise      All agents calm,            Minimal display. Green status dots.
(calm)      no active plans,            Aggregated metrics only. Agent cards
            PE < 0.15 avg               collapsed to single-line summaries.

Volatile    1+ agents in T2,            Affected agents expand automatically.
            active gate failures,       Healthy agents stay collapsed.
            PE 0.15-0.40 avg            Event stream highlights anomalies.

Crisis      Multiple gate failures,     Full traces visible. Remediation
            agent errors,               suggestions shown inline. Per-tick
            PE > 0.40 avg               timeline appears. All agents expand.
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "information|density|gate|CorticalState|Adaptive|Regime|line|aggregate" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "information|density|gate|CorticalState|Adaptive|Regime|line|aggregate" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- [ ] Implement or verify `CorticalState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S008 -- Progressive disclosure

**Source section:** `tmp/architecture/15-dashboard.md:124` through `148`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Progressive disclosure

Three interaction layers control how much detail is visible.

**Layer 0 -- Summary** (visible without interaction):
```
12 agents online    3 plans active    $4.23/hr burn    99.2% gate pass rate
```
One line. Scannable in under a second.

**Layer 1 -- Detail** (one click/hover):
```
Per-agent costs:    coder-1 $0.02/hr  |  research $0.08/hr  |  coder-2 $0.15/hr
Per-model split:    Sonnet 72%  |  Haiku 24%  |  Opus 4%
Cache savings:      $12.40 saved today (L1: 68%, L2: 12%)
```
Breakdown appears in a popover or expanded card.

**Layer 2 -- Trace** (second interaction from Layer 1):
```
Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail,
tool call history, convergence/loop detection events, thinking token breakdown
```
Opens in a slide-out panel or dedicated sub-page.
````

**Explicit detail extraction from this section:**

- Section word count: `128`
- Section hash: `9387ca3f1468fcbc4e10be251b46a50fcf827159b5ef4a193009b89549ddfc6e`

**Normative requirements and implementation claims:**
- **Layer 0 -- Summary** (visible without interaction): ``` 12 agents online 3 plans active $4.23/hr burn 99.2% gate pass rate ``` One line. Scannable in under a second.
- **Layer 1 -- Detail** (one click/hover): ``` Per-agent costs: coder-1 $0.02/hr | research $0.08/hr | coder-2 $0.15/hr Per-model split: Sonnet 72% | Haiku 24% | Opus 4% Cache savings: $12.40 saved today (L1: 68%, L2: 12%) ``` Breakdown appears in a popover or expanded card.
- **Layer 2 -- Trace** (second interaction from Layer 1): ``` Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail, tool call history, convergence/loop detection events, thinking token breakdown ``` Opens in a slide-out panel or dedicated sub-page.

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
- Contract 1: language `plain`, first line `12 agents online    3 plans active    $4.23/hr burn    99.2% gate pass rate`

```
12 agents online    3 plans active    $4.23/hr burn    99.2% gate pass rate
```
- Contract 2: language `plain`, first line `Per-agent costs:    coder-1 $0.02/hr  |  research $0.08/hr  |  coder-2 $0.15/hr`

```
Per-agent costs:    coder-1 $0.02/hr  |  research $0.08/hr  |  coder-2 $0.15/hr
Per-model split:    Sonnet 72%  |  Haiku 24%  |  Opus 4%
Cache savings:      $12.40 saved today (L1: 68%, L2: 12%)
```
- Contract 3: language `plain`, first line `Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail,`

```
Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail,
tool call history, convergence/loop detection events, thinking token breakdown
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "disclosure|Progressive|Layer|interaction|detail|visible|token|second" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "disclosure|Progressive|Layer|interaction|detail|visible|token|second" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S009 -- Epistemic aesthetics

**Source section:** `tmp/architecture/15-dashboard.md:149` through `163`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Epistemic aesthetics

Visual properties react to system state. These are not decorative -- each visual channel encodes a data dimension.

| Visual property | Data source | Encoding |
|----------------|-------------|----------|
| Glow intensity | Epistemic confidence (gate pass rate, neuro store match quality) | Brighter = higher confidence in agent's knowledge |
| Fade / decay | Knowledge staleness (Ebbinghaus forgetting curve) | Faded entries need re-validation or consolidation |
| Turbulence | Contested knowledge entries (challenged in neuro store) | Shimmering/jittering indicates active dispute |
| Velocity streaks | Active agent output (tokens/sec) | Faster streaks = higher throughput |
| Heartbeat pulse | Per-agent tick cadence (gamma/theta/delta) | Visible rhythm matches agent's adaptive clock |
| Saturation | Validation strength (gate rung depth) | Deeper validation = richer color |

Implemented via Three.js shaders for canvas elements and CSS custom properties for DOM elements. The `CorticalState` broadcast drives all six channels.
````

**Explicit detail extraction from this section:**

- Section word count: `126`
- Section hash: `b2ca2ad6c313fe65d121a0debbceca654a7cfc1741c317ed6dd4c35634b1eb61`

**Normative requirements and implementation claims:**
- Visual properties react to system state. These are not decorative -- each visual channel encodes a data dimension.
- | Visual property | Data source | Encoding | |----------------|-------------|----------| | Glow intensity | Epistemic confidence (gate pass rate, neuro store match quality) | Brighter = higher confidence in agent's knowledge | | Fade / decay | Knowledge staleness (Ebbinghaus forgetting curve) | Faded entries need re-validation or consolidation | | Turbulence | Contested knowledge entries (challenged in neuro store) | Shimmering/jittering indicates active dispute | | Velocity streaks | Active agent output (tokens/sec) | Faster streaks = higher throughput | | Heartbeat pulse | Per-agent tick cadence (gamma/theta/delta) | Visible rhythm matches agent's adaptive clock | | Saturation | Validation strength (gate rung depth) | Deeper validation = richer color |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- gamma/theta/

**Types, functions, traits, and inline code identifiers:**
- CorticalState

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
| Visual property | Data source | Encoding |
|----------------|-------------|----------|
| Glow intensity | Epistemic confidence (gate pass rate, neuro store match quality) | Brighter = higher confidence in agent's knowledge |
| Fade / decay | Knowledge staleness (Ebbinghaus forgetting curve) | Faded entries need re-validation or consolidation |
| Turbulence | Contested knowledge entries (challenged in neuro store) | Shimmering/jittering indicates active dispute |
| Velocity streaks | Active agent output (tokens/sec) | Faster streaks = higher throughput |
| Heartbeat pulse | Per-agent tick cadence (gamma/theta/delta) | Visible rhythm matches agent's adaptive clock |
| Saturation | Validation strength (gate rung depth) | Deeper validation = richer color |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `gamma/theta/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Epistemic|aesthetics|CorticalState|validation|knowledge|Visual|streaks|store" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Epistemic|aesthetics|CorticalState|validation|knowledge|Visual|streaks|store" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `gamma/theta/`
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
- [ ] Implement or verify `CorticalState` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S010 -- Layout

**Source section:** `tmp/architecture/15-dashboard.md:164` through `210`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Nunchi              [+ Agent]  [+ Cluster]  [user@email v]      │
├──────┬──────────────────────────────────────────────────────────┤
│      │                                                          │
│ Nav  │  Overview                                                │
│      │  ┌────────────────────────────────────────────────────┐  │
│ Home │  │ * 5 agents   2 clusters   $4.23 today   ^ 3d 2h   │  │
│      │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│Agents│  Agents                                                  │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│      │  │ coder-1  │ │ research │ │ chain-1  │ │ coder-2  │   │
│Feeds │  │ * T0     │ │ * T1     │ │ * T0     │ │ o T2     │   │
│      │  │ coding   │ │ research │ │ chain    │ │ coding   │   │
│      │  │ idle     │ │ querying │ │ monitor  │ │ building │   │
│Plans │  │ $0.02/hr │ │ $0.08/hr │ │ $0/hr    │ │ $0.15/hr │   │
│      │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│      │                                                          │
│Learn │  Cluster: feature-xyz                                    │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ researcher --> impl-1 --> reviewer                 │  │
│Costs │  │ + done        o working   . waiting                │  │
│      │  │               impl-2 --/                           │  │
│      │  │               o working                            │  │
│ Logs │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│  ⚙   │  Agent: coder-2 (expanded)                              │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ Status: T2 reasoning  |  Uptime: 12m  |  Cost: $1.8│  │
│      │  │ Task: Implement pagination in users API            │  │
│      │  │                                                    │  │
│      │  │ Heartbeat ─────────────────────────────            │  │
│      │  │ T0 T0 T0 T0 T1 T0 T0 T2 T0 T0 T0 T1 [T2]       │  │
│      │  │                                                    │  │
│      │  │ Logs (live -- WebSocket, not polling)              │  │
│      │  │ 14:32:21 [T2] PE=0.73 -> full reasoning           │  │
│      │  │ 14:32:25 [T2] action: edit src/users.rs:142       │  │
│      │  │ 14:32:30 [T0] verify: cargo test -> 47 passed     │  │
│      │  │                                                    │  │
│      │  │ [Stop] [Restart] [View Full Trace] [Open in CLI]  │  │
│      │  └────────────────────────────────────────────────────┘  │
└──────┴──────────────────────────────────────────────────────────┘
```
````

**Explicit detail extraction from this section:**

- Section word count: `144`
- Section hash: `75514e0382419a41736555c4ba6c5dcc6b823cf50f206425bddde0ef75424c4e`

**Normative requirements and implementation claims:**
- ``` ┌─────────────────────────────────────────────────────────────────┐ │ Nunchi [+ Agent] [+ Cluster] [user@email v] │ ├──────┬──────────────────────────────────────────────────────────┤ │ │ │ │ Nav │ Overview │ │ │ ┌────────────────────────────────────────────────────┐ │ │ Home │ │ * 5 agents 2 clusters $4.23 today ^ 3d 2h │ │ │ │ └────────────────────────────────────────────────────┘ │ │ │ │ │Agents│ Agents │ │ │ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │ │ │ │ coder-1 │ │ research │ │ chain-1 │ │ coder-2 │ │ │Feeds │ │ * T0 │ │ * T1 │ │ * T0 │ │ o T2 │ │ │ │ │ coding │ │ research │ │ chain │ │ coding │ │ │ │ │ idle │ │ querying │ │ monitor │ │ building │ │ │Plans │ │ $0.02/hr │ │ $0.08/hr │ │ $0/hr │ │ $0.15/hr │ │ │ │ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │ │ │ │ │Learn │ Cluster: feature-xyz │ │ │ ┌────────────────────────────────────────────────────┐ │ │ │ │ researcher --> impl-1 --> reviewer │ │ │Costs │ │ + done o working . waiting │ │ │ │ │ impl-2 --/

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- src/users.rs

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- researcher - -> impl-1 --

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `┌─────────────────────────────────────────────────────────────────┐`

```
┌─────────────────────────────────────────────────────────────────┐
│ Nunchi              [+ Agent]  [+ Cluster]  [user@email v]      │
├──────┬──────────────────────────────────────────────────────────┤
│      │                                                          │
│ Nav  │  Overview                                                │
│      │  ┌────────────────────────────────────────────────────┐  │
│ Home │  │ * 5 agents   2 clusters   $4.23 today   ^ 3d 2h   │  │
│      │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│Agents│  Agents                                                  │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│      │  │ coder-1  │ │ research │ │ chain-1  │ │ coder-2  │   │
│Feeds │  │ * T0     │ │ * T1     │ │ * T0     │ │ o T2     │   │
│      │  │ coding   │ │ research │ │ chain    │ │ coding   │   │
│      │  │ idle     │ │ querying │ │ monitor  │ │ building │   │
│Plans │  │ $0.02/hr │ │ $0.08/hr │ │ $0/hr    │ │ $0.15/hr │   │
│      │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│      │                                                          │
│Learn │  Cluster: feature-xyz                                    │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ researcher --> impl-1 --> reviewer                 │  │
│Costs │  │ + done        o working   . waiting                │  │
│      │  │               impl-2 --/                           │  │
│      │  │               o working
...
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `src/users.rs`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Layout|user|research|impl|coder|Cluster|working|users" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Layout|user|research|impl|coder|Cluster|working|users" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `src/users.rs`
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
- [ ] Enforce state transition `researcher - -> impl-1 --` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S011 -- Feeds page

**Source section:** `tmp/architecture/15-dashboard.md:211` through `246`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feeds page

A dedicated page for browsing agent chain feeds:

```
┌──────────────────────────────────────────────────────────────────┐
│ Feeds                                                            │
│                                                                  │
│ Active feeds (6)                              [Subscribe to new] │
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ eth-mainnet-blocks    chain-watcher-1    raw     0.08 Hz    │ │
│ │ public | 3 subscribers                                       │ │
│ │ Latest: block 21,432,891 | 142 txs | 15.2 gwei             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ eth-gas-trend         chain-watcher-1    derived  0.5 Hz    │ │
│ │ paid ($1/hr) | 1 subscriber                                 │ │
│ │ Latest: trend=rising, 5m_avg=18.4, 1h_avg=15.7             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ base-dex-swaps        defi-scanner      derived  2.0 Hz    │ │
│ │ public | 5 subscribers                                       │ │
│ │ Latest: WETH/USDC swap 12.5 ETH @ $3,241.50                │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Feed detail: eth-mainnet-blocks                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ [live chart: block times + gas prices, last 100 blocks]     │ │
│ │                                                              │ │
│ │ Recent events:                                               │ │
│ │ 14:32:45  block 21,432,891  142 txs  15.2 gwei  0.8s       │ │
│ │ 14:32:33  block 21,432,890  98 txs   14.8 gwei  12.1s      │ │
│ │ 14:32:21  block 21,432,889  203 txs  16.1 gwei  12.0s      │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```
````

**Explicit detail extraction from this section:**

- Section word count: `142`
- Section hash: `9f87ae4d028c47234c3b43d92b6ce85fcf123e1393e098875baf7162a32cb623`

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
- Contract 1: language `plain`, first line `┌──────────────────────────────────────────────────────────────────┐`

```
┌──────────────────────────────────────────────────────────────────┐
│ Feeds                                                            │
│                                                                  │
│ Active feeds (6)                              [Subscribe to new] │
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ eth-mainnet-blocks    chain-watcher-1    raw     0.08 Hz    │ │
│ │ public | 3 subscribers                                       │ │
│ │ Latest: block 21,432,891 | 142 txs | 15.2 gwei             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ eth-gas-trend         chain-watcher-1    derived  0.5 Hz    │ │
│ │ paid ($1/hr) | 1 subscriber                                 │ │
│ │ Latest: trend=rising, 5m_avg=18.4, 1h_avg=15.7             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ base-dex-swaps        defi-scanner      derived  2.0 Hz    │ │
│ │ public | 5 subscribers                                       │ │
│ │ Latest: WETH/USDC swap 12.5 ETH @ $3,241.50                │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Feed detail: eth-mainnet-blocks                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ [live chart: block times + gas prices, last 100 blocks]     │ │
│ │                                                              │ │
│ │ Recent events:
...
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "Feed|block|Feeds|gwei|Subscribe|subscriber|chain|blocks" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|block|Feeds|gwei|Subscribe|subscriber|chain|blocks" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S012 -- Settings page

**Source section:** `tmp/architecture/15-dashboard.md:247` through `286`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Settings page

```
Settings
|-- Account
|   |-- Profile (name, email, avatar from Privy)
|   +-- Wallet (Privy embedded wallet address, delegation status)
|
|-- Provider keys
|   |-- Anthropic (Claude) ---- * connected
|   |-- Perplexity (Sonar) ---- o not set
|   |-- Google (Gemini) ------- o not set
|   |-- Moonshot (Kimi) ------- o not set
|   |-- ZAI (GLM) ------------ o not set
|   |-- OpenRouter ----------- o not set
|   |-- Ollama --------------- * found (localhost:11434)
|   +-- Storage: [server-side v] / [client-only]
|
|-- Integrations
|   |-- GitHub -- o not set (enables PR creation)
|   |-- Slack --- o not set (enables notifications)
|   +-- Railway - * connected (OAuth)
|
|-- Infrastructure
|   |-- Fly.io -- o not set (enables isolated agents)
|   |-- Control plane: https://my-roko.up.railway.app -- * healthy
|   |-- Relay: wss://relay.nunchi.dev -- * healthy
|   +-- Mirage: https://mirage-devnet.fly.dev -- * healthy
|
|-- API keys
|   |-- github-actions (agent:write) -- created 2d ago
|   |-- [+ Create key]
|   +-- [Manage keys]
|
+-- Team (phase 2)
    |-- Members
    |-- Invitations
    +-- Roles
```
````

**Explicit detail extraction from this section:**

- Section word count: `121`
- Section hash: `6997117632967171f014cd6ea145e3bcfc9cc2b2af6063b358fe74ccd5374b47`

**Normative requirements and implementation claims:**
- ``` Settings |-- Account | |-- Profile (name, email, avatar from Privy) | +-- Wallet (Privy embedded wallet address, delegation status) | |-- Provider keys | |-- Anthropic (Claude) ---- * connected | |-- Perplexity (Sonar) ---- o not set | |-- Google (Gemini) ------- o not set | |-- Moonshot (Kimi) ------- o not set | |-- ZAI (GLM) ------------ o not set | |-- OpenRouter ----------- o not set | |-- Ollama --------------- * found (localhost:11434) | +-- Storage: [server-side v] / [client-only] | |-- Integrations | |-- GitHub -- o not set (enables PR creation) | |-- Slack --- o not set (enables notifications) | +-- Railway - * connected (OAuth) | |-- Infrastructure | |-- Fly.io -- o not set (enables isolated agents) | |-- Control plane: https://my-roko.up.railway.app -- * healthy | |-- Relay: wss://relay.nunchi.dev -- * healthy | +-- Mirage: https://mirage-devnet.fly.dev -- * healthy | |-- API keys | |-- github-actions (agent:write) -- created 2d ago | |-- [+ Create key] | +-- [Manage ke

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- ly.io
- roko.up.railway.app
- relay.nunchi.dev
- devnet.fly.dev

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
- Contract 1: language `plain`, first line `Settings`

```
Settings
|-- Account
|   |-- Profile (name, email, avatar from Privy)
|   +-- Wallet (Privy embedded wallet address, delegation status)
|
|-- Provider keys
|   |-- Anthropic (Claude) ---- * connected
|   |-- Perplexity (Sonar) ---- o not set
|   |-- Google (Gemini) ------- o not set
|   |-- Moonshot (Kimi) ------- o not set
|   |-- ZAI (GLM) ------------ o not set
|   |-- OpenRouter ----------- o not set
|   |-- Ollama --------------- * found (localhost:11434)
|   +-- Storage: [server-side v] / [client-only]
|
|-- Integrations
|   |-- GitHub -- o not set (enables PR creation)
|   |-- Slack --- o not set (enables notifications)
|   +-- Railway - * connected (OAuth)
|
|-- Infrastructure
|   |-- Fly.io -- o not set (enables isolated agents)
|   |-- Control plane: https://my-roko.up.railway.app -- * healthy
|   |-- Relay: wss://relay.nunchi.dev -- * healthy
|   +-- Mirage: https://mirage-devnet.fly.dev -- * healthy
|
|-- API keys
|   |-- github-actions (agent:write) -- created 2d ago
|   |-- [+ Create key]
|   +-- [Manage keys]
|
+-- Team (phase 2)
    |-- Members
    |-- Invitations
    +-- Roles
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Settings|keys|healthy|enables|dev|relay|railway|https" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Settings|keys|healthy|enables|dev|relay|railway|https" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- [ ] Emit or consume `ly.io` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.up.railway.app` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `relay.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `devnet.fly.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S013 -- Tech stack

**Source section:** `tmp/architecture/15-dashboard.md:287` through `299`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Tech stack

| Layer | Library | Version | Purpose |
|-------|---------|---------|---------|
| Framework | React | 19 | Component model, concurrent features |
| Build | Vite | 8 | Dev server, production bundling |
| Data fetching | TanStack Query | 5 | REST cache, stale-while-revalidate |
| State | Zustand | 5 | Client-side stores (agent state, UI preferences) |
| Blockchain | ethers.js | 6 | Chain reads, contract interaction |
| 3D / Canvas | Three.js | latest | Epistemic visualizations, particle systems |
| Charts | Recharts | latest | Time series, cost breakdowns, gate pass rates |
| Auth | Privy | 3 | Wallet + social login, embedded wallets |
````

**Explicit detail extraction from this section:**

- Section word count: `73`
- Section hash: `5c5bc8ffa4e944ae129f54670d7a62758fb4ab816daeb2a93257f03716ac7a53`

**Normative requirements and implementation claims:**
- | Layer | Library | Version | Purpose | |-------|---------|---------|---------| | Framework | React | 19 | Component model, concurrent features | | Build | Vite | 8 | Dev server, production bundling | | Data fetching | TanStack Query | 5 | REST cache, stale-while-revalidate | | State | Zustand | 5 | Client-side stores (agent state, UI preferences) | | Blockchain | ethers.js | 6 | Chain reads, contract interaction | | 3D / Canvas | Three.js | latest | Epistemic visualizations, particle systems | | Charts | Recharts | latest | Time series, cost breakdowns, gate pass rates | | Auth | Privy | 3 | Wallet + social login, embedded wallets |

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
| Layer | Library | Version | Purpose |
|-------|---------|---------|---------|
| Framework | React | 19 | Component model, concurrent features |
| Build | Vite | 8 | Dev server, production bundling |
| Data fetching | TanStack Query | 5 | REST cache, stale-while-revalidate |
| State | Zustand | 5 | Client-side stores (agent state, UI preferences) |
| Blockchain | ethers.js | 6 | Chain reads, contract interaction |
| 3D / Canvas | Three.js | latest | Epistemic visualizations, particle systems |
| Charts | Recharts | latest | Time series, cost breakdowns, gate pass rates |
| Auth | Privy | 3 | Wallet + social login, embedded wallets |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "stack|Tech|latest|Wallet|State|Charts|Chain|while" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "stack|Tech|latest|Wallet|State|Charts|Chain|while" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S014 -- Performance targets

**Source section:** `tmp/architecture/15-dashboard.md:300` through `312`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Performance targets

| Metric | Target | How |
|--------|--------|-----|
| FCP | < 1.2s | Code splitting per route, preloaded critical CSS |
| LCP | < 2.0s | SSR for initial state, streaming HTML |
| CLS | < 0.05 | Reserved layout slots for async data |
| WS event-to-render p95 | < 100ms | EventAggregator batching + rAF scheduling |
| Canvas/WebGL | >= 60fps sustained | Separate render loop, instanced geometry |
| Initial JS bundle | < 250KB gzipped | Tree shaking, dynamic imports for heavy deps (Three.js, ethers) |

---
````

**Explicit detail extraction from this section:**

- Section word count: `65`
- Section hash: `6d6786f52b0dacd5dc7a33c8ef99b822c5dce711fccbfba780a81d412c4c02c6`

**Normative requirements and implementation claims:**
- | Metric | Target | How | |--------|--------|-----| | FCP | < 1.2s | Code splitting per route, preloaded critical CSS | | LCP | < 2.0s | SSR for initial state, streaming HTML | | CLS | < 0.05 | Reserved layout slots for async data | | WS event-to-render p95 | < 100ms | EventAggregator batching + rAF scheduling | | Canvas/WebGL | >= 60fps sustained | Separate render loop, instanced geometry | | Initial JS bundle | < 250KB gzipped | Tree shaking, dynamic imports for heavy deps (Three.js, ethers) |
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
- Table 1:

```markdown
| Metric | Target | How |
|--------|--------|-----|
| FCP | < 1.2s | Code splitting per route, preloaded critical CSS |
| LCP | < 2.0s | SSR for initial state, streaming HTML |
| CLS | < 0.05 | Reserved layout slots for async data |
| WS event-to-render p95 | < 100ms | EventAggregator batching + rAF scheduling |
| Canvas/WebGL | >= 60fps sustained | Separate render loop, instanced geometry |
| Initial JS bundle | < 250KB gzipped | Tree shaking, dynamic imports for heavy deps (Three.js, ethers) |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "Target|targets|Performance|render|initial|event|sustained|streaming" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Target|targets|Performance|render|initial|event|sustained|streaming" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S015 -- Appendix: API surface

**Source section:** `tmp/architecture/15-dashboard.md:313` through `314`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Appendix: API surface
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `62b608a9dc206f627e1ca374ef90e2c2c68fde724d22a977a9aa64021db892c0`

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
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "surface|Appendix|API" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "surface|Appendix|API" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S016 -- Agent lifecycle

**Source section:** `tmp/architecture/15-dashboard.md:315` through `331`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent lifecycle

```
POST   /api/agents                    Create agent
GET    /api/agents                    List agents (status, health, cost)
GET    /api/agents/:id                Agent detail (full status, heartbeat history)
POST   /api/agents/:id/start         Start a stopped agent
POST   /api/agents/:id/stop          Graceful stop
DELETE /api/agents/:id                Destroy agent + clean up resources
GET    /api/agents/:id/logs           Agent logs (paginated, filterable)
GET    /api/agents/:id/trace/:tick    Full decision trace for a specific tick
POST   /api/agents/:id/message       Send a message/task to an agent
POST   /api/agents/:id/token         Issue/rotate agent bearer token
GET    /api/agents/:id/token/status   Token status (exists, expiry)
GET    /api/agents/:id/feeds          List feeds exposed by this agent
```
````

**Explicit detail extraction from this section:**

- Section word count: `113`
- Section hash: `68c87fccc781d58938933fd0f4cb95b0247a3e7a0014ef83ed1c95dfe53aadc0`

**Normative requirements and implementation claims:**
- ``` POST /api/agents Create agent GET /api/agents List agents (status, health, cost) GET /api/agents/:id Agent detail (full status, heartbeat history) POST /api/agents/:id/start Start a stopped agent POST /api/agents/:id/stop Graceful stop DELETE /api/agents/:id Destroy agent + clean up resources GET /api/agents/:id/logs Agent logs (paginated, filterable) GET /api/agents/:id/trace/:tick Full decision trace for a specific tick POST /api/agents/:id/message Send a message/task to an agent POST /api/agents/:id/token Issue/rotate agent bearer token GET /api/agents/:id/token/status Token status (exists, expiry) GET /api/agents/:id/feeds List feeds exposed by this agent ```

**Routes and endpoint references:**
- POST /api/agents
- GET /api/agents
- GET /api/agents/:id
- POST /api/agents/:id/start
- POST /api/agents/:id/stop
- DELETE /api/agents/:id
- GET /api/agents/:id/logs
- GET /api/agents/:id/trace/:tick
- POST /api/agents/:id/message
- POST /api/agents/:id/token
- GET /api/agents/:id/token/status
- GET /api/agents/:id/feeds

**Files and path references:**
- api/agents/
- id/token/
- id/trace/

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
- Contract 1: language `plain`, first line `POST   /api/agents                    Create agent`

```
POST   /api/agents                    Create agent
GET    /api/agents                    List agents (status, health, cost)
GET    /api/agents/:id                Agent detail (full status, heartbeat history)
POST   /api/agents/:id/start         Start a stopped agent
POST   /api/agents/:id/stop          Graceful stop
DELETE /api/agents/:id                Destroy agent + clean up resources
GET    /api/agents/:id/logs           Agent logs (paginated, filterable)
GET    /api/agents/:id/trace/:tick    Full decision trace for a specific tick
POST   /api/agents/:id/message       Send a message/task to an agent
POST   /api/agents/:id/token         Issue/rotate agent bearer token
GET    /api/agents/:id/token/status   Token status (exists, expiry)
GET    /api/agents/:id/feeds          List feeds exposed by this agent
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/agents/`
- `id/token/`
- `id/trace/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "api|GET|lifecycle|POST|token|status|stop|trace" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "api|GET|lifecycle|POST|token|status|stop|trace" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/agents/`
- `id/token/`
- `id/trace/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/agents` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/:id/start` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/:id/stop` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/agents/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents/:id/logs` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents/:id/trace/:tick` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/:id/message` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/:id/token` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents/:id/token/status` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/agents/:id/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S017 -- Clusters

**Source section:** `tmp/architecture/15-dashboard.md:332` through `341`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Clusters

```
POST   /api/clusters                  Create cluster with pipeline
GET    /api/clusters                  List clusters
GET    /api/clusters/:id              Cluster status (pipeline progress)
POST   /api/clusters/:id/stop         Stop all agents in cluster
DELETE /api/clusters/:id              Destroy cluster + all agents
```
````

**Explicit detail extraction from this section:**

- Section word count: `38`
- Section hash: `2f44873ff965338c240edaa56b7cce9611145c2819b8cbd7d8dbcbeddd5fe50e`

**Normative requirements and implementation claims:**
- ``` POST /api/clusters Create cluster with pipeline GET /api/clusters List clusters GET /api/clusters/:id Cluster status (pipeline progress) POST /api/clusters/:id/stop Stop all agents in cluster DELETE /api/clusters/:id Destroy cluster + all agents ```

**Routes and endpoint references:**
- POST /api/clusters
- GET /api/clusters
- GET /api/clusters/:id
- POST /api/clusters/:id/stop
- DELETE /api/clusters/:id

**Files and path references:**
- api/clusters/

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
- Contract 1: language `plain`, first line `POST   /api/clusters                  Create cluster with pipeline`

```
POST   /api/clusters                  Create cluster with pipeline
GET    /api/clusters                  List clusters
GET    /api/clusters/:id              Cluster status (pipeline progress)
POST   /api/clusters/:id/stop         Stop all agents in cluster
DELETE /api/clusters/:id              Destroy cluster + all agents
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/clusters/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "cluster|Clusters|api|stop|pipeline|POST|GET|status" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "cluster|Clusters|api|stop|pipeline|POST|GET|status" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/clusters/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
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
- [ ] Implement or verify route `GET /api/clusters` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/clusters/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/clusters/:id/stop` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/clusters/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S018 -- Inference gateway

**Source section:** `tmp/architecture/15-dashboard.md:342` through `350`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Inference gateway

```
POST   /api/inference/proxy           Proxied inference for remote agents
GET    /api/inference/stats           Gateway stats (cache hit rate, costs, latency)
GET    /api/inference/models          Available models + routing weights
POST   /api/inference/models/:id/pin  Pin a model for an agent (override router)
```
````

**Explicit detail extraction from this section:**

- Section word count: `42`
- Section hash: `d71ce6cff1f5000d06c7167f6ff22c4cf2062d5987d1134eb97a8b7c6076fdb2`

**Normative requirements and implementation claims:**
- ``` POST /api/inference/proxy Proxied inference for remote agents GET /api/inference/stats Gateway stats (cache hit rate, costs, latency) GET /api/inference/models Available models + routing weights POST /api/inference/models/:id/pin Pin a model for an agent (override router) ```

**Routes and endpoint references:**
- POST /api/inference/proxy
- GET /api/inference/stats
- GET /api/inference/models
- POST /api/inference/models/:id/pin

**Files and path references:**
- api/inference/
- api/inference/models/

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
- Contract 1: language `plain`, first line `POST   /api/inference/proxy           Proxied inference for remote agents`

```
POST   /api/inference/proxy           Proxied inference for remote agents
GET    /api/inference/stats           Gateway stats (cache hit rate, costs, latency)
GET    /api/inference/models          Available models + routing weights
POST   /api/inference/models/:id/pin  Pin a model for an agent (override router)
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/inference/`
- `api/inference/models/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Inference|gateway|model|api|models|stats|pin|POST" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Inference|gateway|model|api|models|stats|pin|POST" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/inference/`
- `api/inference/models/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `POST /api/inference/proxy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/inference/stats` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/inference/models` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/inference/models/:id/pin` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S019 -- Feeds

**Source section:** `tmp/architecture/15-dashboard.md:351` through `359`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Feeds

```
GET    /api/feeds                     List all registered feeds
GET    /api/feeds/:feed_id            Feed detail + recent data
POST   /api/feeds/:feed_id/subscribe  Subscribe to a feed
DELETE /api/feeds/:feed_id/subscribe  Unsubscribe from a feed
```
````

**Explicit detail extraction from this section:**

- Section word count: `33`
- Section hash: `4ae56e237e40a8dd24dcddbaef1acbe8e75d0ae5e91643503bb00ad18c0cf91d`

**Normative requirements and implementation claims:**
- ``` GET /api/feeds List all registered feeds GET /api/feeds/:feed_id Feed detail + recent data POST /api/feeds/:feed_id/subscribe Subscribe to a feed DELETE /api/feeds/:feed_id/subscribe Unsubscribe from a feed ```

**Routes and endpoint references:**
- GET /api/feeds
- GET /api/feeds/:feed_id
- POST /api/feeds/:feed_id/subscribe
- DELETE /api/feeds/:feed_id/subscribe

**Files and path references:**
- api/feeds/

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
- Contract 1: language `plain`, first line `GET    /api/feeds                     List all registered feeds`

```
GET    /api/feeds                     List all registered feeds
GET    /api/feeds/:feed_id            Feed detail + recent data
POST   /api/feeds/:feed_id/subscribe  Subscribe to a feed
DELETE /api/feeds/:feed_id/subscribe  Unsubscribe from a feed
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Feed|Feeds|subscribe|api|feed_id|GET|registered|recent" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|Feeds|subscribe|api|feed_id|GET|registered|recent" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/feeds/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `GET /api/feeds` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/feeds/:feed_id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/feeds/:feed_id/subscribe` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/feeds/:feed_id/subscribe` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S020 -- Secrets

**Source section:** `tmp/architecture/15-dashboard.md:360` through `368`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Secrets

```
GET    /api/secrets                    List secret namespaces + keys (not values)
POST   /api/secrets/:ns/:key          Set a secret
DELETE /api/secrets/:ns/:key          Remove a secret
POST   /api/secrets/:ns/:key/test     Test if a secret is valid
```
````

**Explicit detail extraction from this section:**

- Section word count: `37`
- Section hash: `3671b36df6551828c13b9920dba63f6cbc6db10e02e385c2ec65e1ce24f3be6a`

**Normative requirements and implementation claims:**
- ``` GET /api/secrets List secret namespaces + keys (not values) POST /api/secrets/:ns/:key Set a secret DELETE /api/secrets/:ns/:key Remove a secret POST /api/secrets/:ns/:key/test Test if a secret is valid ```

**Routes and endpoint references:**
- GET /api/secrets
- POST /api/secrets/:ns/:key
- DELETE /api/secrets/:ns/:key
- POST /api/secrets/:ns/:key/test

**Files and path references:**
- api/secrets/

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
- Contract 1: language `plain`, first line `GET    /api/secrets                    List secret namespaces + keys (not values)`

```
GET    /api/secrets                    List secret namespaces + keys (not values)
POST   /api/secrets/:ns/:key          Set a secret
DELETE /api/secrets/:ns/:key          Remove a secret
POST   /api/secrets/:ns/:key/test     Test if a secret is valid
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/secrets/`
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
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "secret|Secrets|key|api|test|POST|values|valid" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "secret|Secrets|key|api|test|POST|values|valid" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/secrets/`
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
- `crates/roko-serve/src/routes/mod.rs`
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
- [ ] Implement or verify route `GET /api/secrets` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/secrets/:ns/:key` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/secrets/:ns/:key` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/secrets/:ns/:key/test` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S021 -- Auth

**Source section:** `tmp/architecture/15-dashboard.md:369` through `383`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Auth

```
POST   /auth/login                     Email/password login
POST   /auth/privy/verify              Verify Privy JWT, create roko session
POST   /auth/device/authorize          Start device flow
POST   /auth/device/token              Poll for device flow token
GET    /auth/callback                  PKCE OAuth callback (for CLI)
POST   /auth/refresh                   Refresh access token
GET    /auth/session                   Current user info
POST   /api/api-keys                   Create API key
DELETE /api/api-keys/:id               Revoke API key
GET    /api/api-keys                   List API keys
```
````

**Explicit detail extraction from this section:**

- Section word count: `74`
- Section hash: `1775c11d49e28783d0f11ac1e3781eece122190471531b58df754678f9bf2203`

**Normative requirements and implementation claims:**
- ``` POST /auth/login Email/password login POST /auth/privy/verify Verify Privy JWT, create roko session POST /auth/device/authorize Start device flow POST /auth/device/token Poll for device flow token GET /auth/callback PKCE OAuth callback (for CLI) POST /auth/refresh Refresh access token GET /auth/session Current user info POST /api/api-keys Create API key DELETE /api/api-keys/:id Revoke API key GET /api/api-keys List API keys ```

**Routes and endpoint references:**
- POST /api/api-keys
- DELETE /api/api-keys/:id
- GET /api/api-keys

**Files and path references:**
- api/api-keys/
- auth/device/
- auth/privy/

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
- Contract 1: language `plain`, first line `POST   /auth/login                     Email/password login`

```
POST   /auth/login                     Email/password login
POST   /auth/privy/verify              Verify Privy JWT, create roko session
POST   /auth/device/authorize          Start device flow
POST   /auth/device/token              Poll for device flow token
GET    /auth/callback                  PKCE OAuth callback (for CLI)
POST   /auth/refresh                   Refresh access token
GET    /auth/session                   Current user info
POST   /api/api-keys                   Create API key
DELETE /api/api-keys/:id               Revoke API key
GET    /api/api-keys                   List API keys
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/api-keys/`
- `auth/device/`
- `auth/privy/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
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
rg -n "Auth|api|POST|keys|device|token|GET|verify" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Auth|api|POST|keys|device|token|GET|verify" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/api-keys/`
- `auth/device/`
- `auth/privy/`
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
- `crates/roko-serve/src/routes/mod.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/api-keys` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/api-keys/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/api-keys` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S022 -- WebSocket

**Source section:** `tmp/architecture/15-dashboard.md:384` through `392`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `verification`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### WebSocket

```
WS     /ws                            roko-serve event stream (plans, gates, episodes)
WS     /relay/ws                      Relay event stream (presence, feeds, messages)
```

Both WebSocket endpoints support the room-based subscription protocol described in [Connectivity and Relay](04-connectivity.md).
````

**Explicit detail extraction from this section:**

- Section word count: `35`
- Section hash: `c90234f7014812c0b351da380670e6044b516a2eaab4e7ce689ffd5f36e2d936`

**Normative requirements and implementation claims:**
- ``` WS /ws roko-serve event stream (plans, gates, episodes) WS /relay/ws Relay event stream (presence, feeds, messages) ```
- Both WebSocket endpoints support the room-based subscription protocol described in [Connectivity and Relay](04-connectivity.md).

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
- Contract 1: language `plain`, first line `WS     /ws                            roko-serve event stream (plans, gates, episodes)`

```
WS     /ws                            roko-serve event stream (plans, gates, episodes)
WS     /relay/ws                      Relay event stream (presence, feeds, messages)
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "WebSocket|relay|stream|event|connectivity|support|subscription|serve" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "WebSocket|relay|stream|event|connectivity|support|subscription|serve" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S023 -- Infrastructure

**Source section:** `tmp/architecture/15-dashboard.md:393` through `402`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Infrastructure

```
GET    /api/health                     Service health
GET    /api/providers                  Configured LLM providers + health
GET    /api/costs                      Cost summary (per agent, per day, per model)
GET    /api/costs/:agent_id            Cost breakdown for one agent
GET    /api/relay/health               Relay connection health
```
````

**Explicit detail extraction from this section:**

- Section word count: `39`
- Section hash: `c4faf8f3f4e5ac12634c8f338ad8a0369cfab67ae25079a3a23b52cdb390945c`

**Normative requirements and implementation claims:**
- ``` GET /api/health Service health GET /api/providers Configured LLM providers + health GET /api/costs Cost summary (per agent, per day, per model) GET /api/costs/:agent_id Cost breakdown for one agent GET /api/relay/health Relay connection health ```

**Routes and endpoint references:**
- GET /api/health
- GET /api/providers
- GET /api/costs
- GET /api/costs/:agent_id
- GET /api/relay/health

**Files and path references:**
- api/costs/
- api/relay/

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
- Contract 1: language `plain`, first line `GET    /api/health                     Service health`

```
GET    /api/health                     Service health
GET    /api/providers                  Configured LLM providers + health
GET    /api/costs                      Cost summary (per agent, per day, per model)
GET    /api/costs/:agent_id            Cost breakdown for one agent
GET    /api/relay/health               Relay connection health
```

**Read before editing:**
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/costs/`
- `api/relay/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/mod.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "health|api|Infrastructure|GET|Cost|relay|providers|costs" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "health|api|Infrastructure|GET|Cost|relay|providers|costs" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
- `api/costs/`
- `api/relay/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
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
- [ ] Implement or verify route `GET /api/health` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/providers` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/costs` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/costs/:agent_id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/relay/health` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

### ARCH-15-S024 -- Existing routes

**Source section:** `tmp/architecture/15-dashboard.md:403` through `405`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Existing routes

All ~85 existing routes in roko-serve (plans, PRDs, gates, episodes, signals, knowledge, learning, research, diagnosis, deployments, etc.) remain unchanged. They get auth middleware added.
````

**Explicit detail extraction from this section:**

- Section word count: `25`
- Section hash: `9767421c065cafd21e304f2924c3eae4b55e2731c88127e4c8b50cda4353f6e7`

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
- `tmp/architecture/15-dashboard.md`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
rg -n "routes|Existing|unchanged|signals|serve|research|remain|plans" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "routes|Existing|unchanged|signals|serve|research|remain|plans" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/status.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/15-dashboard
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

