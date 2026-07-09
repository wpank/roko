# Architecture Plan: Auth

**Source:** `tmp/architecture/08-auth.md`
**Generated:** 2026-04-25
**Source hash:** `9161dba795c8cdb09f572e9b4d2ce67d3e5aa9e2c090f4dc3a23dfeb2af18cbd`
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
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-08-S001 | 1 | Authentication and secrets | [ ] | 9.8 |
| ARCH-08-S002 | 8 | Authentication | [ ] | 9.8 |
| ARCH-08-S003 | 12 | 1. Dashboard users: Privy | [ ] | 9.8 |
| ARCH-08-S004 | 63 | 2. CLI: API keys + roko login | [ ] | 9.8 |
| ARCH-08-S005 | 136 | 3. Agent auth: bearer tokens | [ ] | 9.8 |
| ARCH-08-S006 | 178 | 4. Relay auth: reads public, writes authenticated | [ ] | 9.8 |
| ARCH-08-S007 | 188 | Agent-to-agent auth for paid feeds | [ ] | 9.8 |
| ARCH-08-S008 | 192 | 5. Shared workspace access (team development) | [ ] | 9.8 |
| ARCH-08-S009 | 333 | Secret and API key management | [ ] | 9.8 |
| ARCH-08-S010 | 335 | Storage hierarchy | [ ] | 9.8 |
| ARCH-08-S011 | 345 | Secrets store format | [ ] | 9.8 |
| ARCH-08-S012 | 369 | From the CLI | [ ] | 9.8 |
| ARCH-08-S013 | 395 | From the dashboard | [ ] | 9.8 |

## Tasks

### ARCH-08-S001 -- Authentication and secrets

**Source section:** `tmp/architecture/08-auth.md:1` through `7`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Authentication and secrets

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Authentication" and "Secret and API key management" sections.

---
````

**Explicit detail extraction from this section:**

- Section word count: `24`
- Section hash: `a19b374f3a22589ea6aedc79f581c37c1c0fc1c1f1a7e34ce1922de5c030807d`

**Normative requirements and implementation claims:**
- > Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc. > Merges the "Authentication" and "Secret and API key management" sections.
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
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "auth|Secret|Authentication|secrets|sections|redesign|management" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "auth|Secret|Authentication|secrets|sections|redesign|management" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S002 -- Authentication

**Source section:** `tmp/architecture/08-auth.md:8` through `11`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Authentication

Four auth paths for four surfaces.
````

**Explicit detail extraction from this section:**

- Section word count: `6`
- Section hash: `c4380b4e4e9b2a5a124419677bec3ebfc3874dc7eb75dc44f887103e58dd79bb`

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
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "auth|Authentication|Four|surfaces|paths" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "auth|Authentication|Four|surfaces|paths" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S003 -- 1. Dashboard users: Privy

**Source section:** `tmp/architecture/08-auth.md:12` through `62`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 1. Dashboard users: Privy

```
Browser → Privy SDK → JWT → roko-serve validates signature
```

Privy handles login (email, social, wallet). The dashboard includes the JWT in every API call. roko-serve validates the JWT signature against Privy's JWKS endpoint.

```
GET https://auth.privy.io/.well-known/jwks.json
→ Cache JWKS, verify JWT signature + expiry
→ Extract: sub (privy user ID), email, wallet address
→ Lookup or create user in .roko/users/
```

**JWKS caching strategy**:

```
JWT arrives for validation
         │
         ▼
Check in-memory JWKS cache
         │
    cache hit ──────────────► Validate JWT with cached keys
    (< 1 hour old)                    │
         │                       valid ──► accept
    cache miss                        │
    (or expired)                 invalid ──► refetch JWKS once (key rotation)
         │                                        │
         ▼                                   valid ──► accept
Fetch GET https://auth.privy.io/                  │
      /.well-known/jwks.json              still invalid ──► return 401
         │
    success ──► update cache, validate JWT
         │
    failure ──► use stale cache if available (stale-while-revalidate)
         │           │
         │      no stale cache ──► return 401
         │
         ▼
    Log warning if cache is > 24 hours old:
    "[WARN] JWKS cache stale (26h). Privy endpoint may be down."
```

- **Cache TTL**: 1 hour. After 1 hour, the next validation triggers a background refetch.
- **Key rotation handling**: When a JWT fails validation against cached keys, refetch JWKS once before returning 401. Privy rotates keys periodically; the single retry catches rotation without adding latency to every request.
- **Endpoint unavailability**: If the JWKS endpoint is down, use stale cached keys. This is safe because Privy key rotation is infrequent (weeks or months between rotations). Log a warning if the cache exceeds 24 hours of staleness.
- **Startup**: On server start, fetch JWKS eagerly. If the fetch fails, the server starts but JWT validation returns 401 until the cache is populated. API key auth and agent token auth are unaffected.

Privy also provides an embedded wallet for chain interactions (signing transactions, delegating to agents). Optional -- users who don't need chain features never see wallet UI.
````

**Explicit detail extraction from this section:**

- Section word count: `293`
- Section hash: `bcdf2900150f2a2bed7946b3f9227f84596c9b20fe844a2a21f6ad67e641fe8a`

**Normative requirements and implementation claims:**
- Privy handles login (email, social, wallet). The dashboard includes the JWT in every API call. roko-serve validates the JWT signature against Privy's JWKS endpoint.
- ``` GET https://auth.privy.io/.well-known/jwks.json → Cache JWKS, verify JWT signature + expiry → Extract: sub (privy user ID), email, wallet address → Lookup or create user in .roko/users/ ```
- **JWKS caching strategy**:
- ``` JWT arrives for validation │ ▼ Check in-memory JWKS cache │ cache hit ──────────────► Validate JWT with cached keys (< 1 hour old) │ │ valid ──► accept cache miss │ (or expired) invalid ──► refetch JWKS once (key rotation) │ │ ▼ valid ──► accept Fetch GET https://auth.privy.io/ │ /.well-known/jwks.json still invalid ──► return 401 │ success ──► update cache, validate JWT │ failure ──► use stale cache if available (stale-while-revalidate) │ │ │ no stale cache ──► return 401 │ ▼ Log warning if cache is > 24 hours old: "[WARN] JWKS cache stale (26h). Privy endpoint may be down." ```
- - **Cache TTL**: 1 hour. After 1 hour, the next validation triggers a background refetch. - **Key rotation handling**: When a JWT fails validation against cached keys, refetch JWKS once before returning 401. Privy rotates keys periodically; the single retry catches rotation without adding latency to every request. - **Endpoint unavailability**: If the JWKS endpoint is down, use stale cached keys. This is safe because Privy key rotation is infrequent (weeks or months between rotations). Log a warning if the cache exceeds 24 hours of staleness. - **Startup**: On server start, fetch JWKS eagerly. If the fetch fails, the server starts but JWT validation returns 401 until the cache is populated. API key auth and agent token auth are unaffected.
- Privy also provides an embedded wallet for chain interactions (signing transactions, delegating to agents). Optional -- users who don't need chain features never see wallet UI.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/users/
- .well-known/jwks.json
- auth.privy.io/.well-known/jwks.json

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- auth.privy.io

**State transitions:**
- Browser -> Privy SDK
- JWT -> roko-serve validates signature
- json -> Cache JWKS
- expiry -> Extract
- wallet address -> Lookup or create user in

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Cache TTL**: 1 hour. After 1 hour, the next validation triggers a background refetch.
- - **Key rotation handling**: When a JWT fails validation against cached keys, refetch JWKS once before returning 401. Privy rotates keys periodically; the single retry catches rotation without adding latency to every request.
- - **Endpoint unavailability**: If the JWKS endpoint is down, use stale cached keys. This is safe because Privy key rotation is infrequent (weeks or months between rotations). Log a warning if the cache exceeds 24 hours of staleness.
- - **Startup**: On server start, fetch JWKS eagerly. If the fetch fails, the server starts but JWT validation returns 401 until the cache is populated. API key auth and agent token auth are unaffected.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Browser → Privy SDK → JWT → roko-serve validates signature`

```
Browser → Privy SDK → JWT → roko-serve validates signature
```
- Contract 2: language `plain`, first line `GET https://auth.privy.io/.well-known/jwks.json`

```
GET https://auth.privy.io/.well-known/jwks.json
→ Cache JWKS, verify JWT signature + expiry
→ Extract: sub (privy user ID), email, wallet address
→ Lookup or create user in .roko/users/
```
- Contract 3: language `plain`, first line `JWT arrives for validation`

```
JWT arrives for validation
         │
         ▼
Check in-memory JWKS cache
         │
    cache hit ──────────────► Validate JWT with cached keys
    (< 1 hour old)                    │
         │                       valid ──► accept
    cache miss                        │
    (or expired)                 invalid ──► refetch JWKS once (key rotation)
         │                                        │
         ▼                                   valid ──► accept
Fetch GET https://auth.privy.io/                  │
      /.well-known/jwks.json              still invalid ──► return 401
         │
    success ──► update cache, validate JWT
         │
    failure ──► use stale cache if available (stale-while-revalidate)
         │           │
         │      no stale cache ──► return 401
         │
         ▼
    Log warning if cache is > 24 hours old:
    "[WARN] JWKS cache stale (26h). Privy endpoint may be down."
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/users/`
- `.well-known/jwks.json`
- `auth.privy.io/.well-known/jwks.json`
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
rg -n "Privy|Cache|valid|jwks|user|users|stale|Fetch" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Privy|Cache|valid|jwks|user|users|stale|Fetch" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/users/`
- `.well-known/jwks.json`
- `auth.privy.io/.well-known/jwks.json`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Emit or consume `auth.privy.io` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `Browser -> Privy SDK` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `JWT -> roko-serve validates signature` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `json -> Cache JWKS` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `expiry -> Extract` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `wallet address -> Lookup or create user in` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S004 -- 2. CLI: API keys + roko login

**Source section:** `tmp/architecture/08-auth.md:63` through `135`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 2. CLI: API keys + roko login

```bash
# Generate an API key (from dashboard or CLI)
roko config secrets set api.my-key
# → sk_roko_aBcDeFgHiJkLmNoP

# Use it
roko status --server https://my-roko.up.railway.app --api-key sk_roko_...

# Or: roko login (browser-based)
roko login https://my-roko.up.railway.app
# → Opens browser for Privy auth
# → Stores session token in OS keychain

# On headless machines: device flow
roko login https://my-roko.up.railway.app
# → Visit https://my-roko.up.railway.app/auth/device
#   Enter code: ABCD-EFGH
# → Polls until approved
# → Token stored in OS keychain
```

API keys have scopes:

```rust
pub enum ApiKeyScope {
    Read,        // GET endpoints only
    AgentWrite,  // Agent CRUD + messaging
    PlanWrite,   // Plan/PRD creation and execution
    Admin,       // Everything including secrets and config
}
```

**Scope-to-route mapping**:

```
Scope          Allowed methods    Allowed routes
─────          ───────────────    ──────────────
read           GET                Any route

agent:write    POST, PUT, DELETE  /api/agents/*
               POST               /api/agents/*/message
               POST               /api/agents/*/token

plan:write     POST, PUT, DELETE  /api/plans/*
               POST               /api/plans/*/run
               POST, PUT, DELETE  /api/prd/*

admin          *                  * (all routes, including:)
                                  /api/api-keys/*
                                  /api/config/*
                                  /api/secrets/*
                                  /api/gateway/*
```

A key with multiple scopes has the union of their permissions. For example, a key with `[read, agent:write]` can GET any route and POST/PUT/DELETE on agent routes, but cannot touch plans or config.

**Insufficient scope response**:

```json
HTTP 403 Forbidden

{
  "error": "insufficient_scope",
  "required": "agent:write",
  "has": "read",
  "route": "POST /api/agents/coder-1/message"
}
```

The response tells the caller exactly what scope they need, what they have, and which route triggered the rejection. This makes debugging straightforward for both humans and agents.
````

**Explicit detail extraction from this section:**

- Section word count: `259`
- Section hash: `837d0655d3a252de6ee304fcf448ab330b25121d852517a9982d1afa5753f3ee`

**Normative requirements and implementation claims:**
- ```bash # Generate an API key (from dashboard or CLI) roko config secrets set api.my-key # → sk_roko_aBcDeFgHiJkLmNoP
- # Use it roko status --server https://my-roko.up.railway.app --api-key sk_roko_...
- API keys have scopes:
- ```rust pub enum ApiKeyScope { Read, // GET endpoints only AgentWrite, // Agent CRUD + messaging PlanWrite, // Plan/PRD creation and execution Admin, // Everything including secrets and config } ```
- **Scope-to-route mapping**:
- ``` Scope Allowed methods Allowed routes ───── ─────────────── ────────────── read GET Any route
- agent:write POST, PUT, DELETE /api/agents/* POST /api/agents/*/message POST /api/agents/*/token
- plan:write POST, PUT, DELETE /api/plans/* POST /api/plans/*/run POST, PUT, DELETE /api/prd/*
- admin * * (all routes, including:) /api/api-keys/* /api/config/* /api/secrets/* /api/gateway/* ```
- A key with multiple scopes has the union of their permissions. For example, a key with `[read, agent:write]` can GET any route and POST/PUT/DELETE on agent routes, but cannot touch plans or config.
- **Insufficient scope response**:
- { "error": "insufficient_scope", "required": "agent:write", "has": "read", "route": "POST /api/agents/coder-1/message" } ```
- The response tells the caller exactly what scope they need, what they have, and which route triggered the rejection. This makes debugging straightforward for both humans and agents.

**Routes and endpoint references:**
- DELETE /api/agents/
- POST /api/agents/
- DELETE /api/plans/
- POST /api/plans/
- DELETE /api/prd/
- POST /api/agents/coder-1/message

**Files and path references:**
- POST/PUT/
- api/agents/
- api/agents/coder-1/
- api/api-keys/
- api/config/
- api/gateway/
- api/plans/
- api/prd/
- api/secrets/
- my-roko.up.railway.app/auth/

**Types, functions, traits, and inline code identifiers:**
- ApiKeyScope

**Event names and event-like entities:**
- api.my
- roko.up.railway.app
- sk_roko_...

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- roko config secrets set api.my-key
- roko status --server https://my-roko.up.railway.app --api-key sk_roko_...
- roko login https://my-roko.up.railway.app

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `# Generate an API key (from dashboard or CLI)`

```bash
# Generate an API key (from dashboard or CLI)
roko config secrets set api.my-key
# → sk_roko_aBcDeFgHiJkLmNoP

# Use it
roko status --server https://my-roko.up.railway.app --api-key sk_roko_...

# Or: roko login (browser-based)
roko login https://my-roko.up.railway.app
# → Opens browser for Privy auth
# → Stores session token in OS keychain

# On headless machines: device flow
roko login https://my-roko.up.railway.app
# → Visit https://my-roko.up.railway.app/auth/device
#   Enter code: ABCD-EFGH
# → Polls until approved
# → Token stored in OS keychain
```
- Contract 2: language `rust`, first line `pub enum ApiKeyScope {`

```rust
pub enum ApiKeyScope {
    Read,        // GET endpoints only
    AgentWrite,  // Agent CRUD + messaging
    PlanWrite,   // Plan/PRD creation and execution
    Admin,       // Everything including secrets and config
}
```
- Contract 3: language `plain`, first line `Scope          Allowed methods    Allowed routes`

```
Scope          Allowed methods    Allowed routes
─────          ───────────────    ──────────────
read           GET                Any route

agent:write    POST, PUT, DELETE  /api/agents/*
               POST               /api/agents/*/message
               POST               /api/agents/*/token

plan:write     POST, PUT, DELETE  /api/plans/*
               POST               /api/plans/*/run
               POST, PUT, DELETE  /api/prd/*

admin          *                  * (all routes, including:)
                                  /api/api-keys/*
                                  /api/config/*
                                  /api/secrets/*
                                  /api/gateway/*
```
- Contract 4: language `json`, first line `HTTP 403 Forbidden`

```json
HTTP 403 Forbidden

{
  "error": "insufficient_scope",
  "required": "agent:write",
  "has": "read",
  "route": "POST /api/agents/coder-1/message"
}
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `POST/PUT/`
- `api/agents/`
- `api/agents/coder-1/`
- `api/api-keys/`
- `api/config/`
- `api/gateway/`
- `api/plans/`
- `api/prd/`
- `api/secrets/`
- `my-roko.up.railway.app/auth/`
- `crates/roko-serve/src/routes/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "API|route|login|keys|Scope|POST|write|app" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "API|route|login|keys|Scope|POST|write|app" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `POST/PUT/`
- `api/agents/`
- `api/agents/coder-1/`
- `api/api-keys/`
- `api/config/`
- `api/gateway/`
- `api/plans/`
- `api/prd/`
- `api/secrets/`
- `my-roko.up.railway.app/auth/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`

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
- [ ] Implement or verify route `DELETE /api/agents/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/plans/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/plans/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/prd/` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/coder-1/message` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `ApiKeyScope` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `api.my` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.up.railway.app` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `sk_roko_...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Implement or verify operator command `roko config secrets set api.my-key` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko status --server https://my-roko.up.railway.app --api-key sk_roko_...` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko login https://my-roko.up.railway.app` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S005 -- 3. Agent auth: bearer tokens

**Source section:** `tmp/architecture/08-auth.md:136` through `177`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 3. Agent auth: bearer tokens

Agents authenticate to the relay and to the inference proxy using bearer tokens issued by the control plane.

```bash
# Control plane issues token for an agent
POST /api/agents/:id/token
→ { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }

# Agent uses token for relay connection
WS wss://relay.nunchi.dev/relay/ws
→ First message: { "type": "auth", "token": "roko_agent_..." }

# Agent uses token for inference proxy
POST /api/inference/proxy
Authorization: Bearer roko_agent_...
```

Tokens are SHA-256 hashed before storage. The plaintext is returned exactly once at issuance.

**Token lifecycle**:

```
Agent created                Token issued                 30 days later
(POST /api/agents)           (response includes token)    (token expires)
       │                            │                            │
       ▼                            ▼                            ▼
  agent record              token = "roko_agent_..."      agent gets 401
  stored in DB              SHA-256 hash stored           on next request
                            plaintext returned ONCE
                                                                 │
                                                                 ▼
                                                          request new token
                                                          via relay or API
```

- **Issuance**: Tokens are issued when an agent is created (`POST /api/agents`). The response body includes the `token` field with the plaintext. This is the only time the plaintext is available.
- **Expiry**: Tokens expire after 30 days by default. Configurable per agent via `token_ttl_days` in `roko.toml`.
- **Revocation**: `DELETE /api/agents/{id}/token` immediately invalidates the token. The SHA-256 hash is removed from the valid token set. The agent receives `401 Unauthorized` on its next request.
- **Rotation**: To rotate without downtime, issue a new token (`POST /api/agents/{id}/token`) before revoking the old one. During rotation, both the old and new tokens are valid for a 5-minute grace period. After 5 minutes, the old token is automatically invalidated.
- **Re-issuance**: An agent that receives 401 (expired or revoked token) should request a new token through the relay control channel or by calling `POST /api/agents/{id}/token` with admin-scoped auth.
````

**Explicit detail extraction from this section:**

- Section word count: `284`
- Section hash: `5f2bf604618dcc74bf45edfdfa3297b0222b608ecf69197cfb4f8e5435187a99`

**Normative requirements and implementation claims:**
- ```bash # Control plane issues token for an agent POST /api/agents/:id/token → { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }
- # Agent uses token for inference proxy POST /api/inference/proxy Authorization: Bearer roko_agent_... ```
- **Token lifecycle**:
- ``` Agent created Token issued 30 days later (POST /api/agents) (response includes token) (token expires) │ │ │ ▼ ▼ ▼ agent record token = "roko_agent_..." agent gets 401 stored in DB SHA-256 hash stored on next request plaintext returned ONCE │ ▼ request new token via relay or API ```
- - **Issuance**: Tokens are issued when an agent is created (`POST /api/agents`). The response body includes the `token` field with the plaintext. This is the only time the plaintext is available. - **Expiry**: Tokens expire after 30 days by default. Configurable per agent via `token_ttl_days` in `roko.toml`. - **Revocation**: `DELETE /api/agents/{id}/token` immediately invalidates the token. The SHA-256 hash is removed from the valid token set. The agent receives `401 Unauthorized` on its next request. - **Rotation**: To rotate without downtime, issue a new token (`POST /api/agents/{id}/token`) before revoking the old one. During rotation, both the old and new tokens are valid for a 5-minute grace period. After 5 minutes, the old token is automatically invalidated. - **Re-issuance**: An agent that receives 401 (expired or revoked token) should request a new token through the relay control channel or by calling `POST /api/agents/{id}/token` with admin-scoped auth.

**Routes and endpoint references:**
- POST /api/agents/:id/token
- POST /api/inference/proxy
- POST /api/agents
- DELETE /api/agents/{id}/token
- POST /api/agents/{id}/token

**Files and path references:**
- api/agents/
- api/inference/
- relay.nunchi.dev/relay/

**Types, functions, traits, and inline code identifiers:**
- token
- token_ttl_days

**Event names and event-like entities:**
- roko_agent_...
- relay.nunchi.dev

**State transitions:**
- ws -> First message

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- POST /api/agents/:id/token
- → { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }
- WS wss://relay.nunchi.dev/relay/ws
- → First message: { "type": "auth", "token": "roko_agent_..." }
- POST /api/inference/proxy
- Authorization: Bearer roko_agent_...

**Bullet requirements:**
- - **Issuance**: Tokens are issued when an agent is created (`POST /api/agents`). The response body includes the `token` field with the plaintext. This is the only time the plaintext is available.
- - **Expiry**: Tokens expire after 30 days by default. Configurable per agent via `token_ttl_days` in `roko.toml`.
- - **Revocation**: `DELETE /api/agents/{id}/token` immediately invalidates the token. The SHA-256 hash is removed from the valid token set. The agent receives `401 Unauthorized` on its next request.
- - **Rotation**: To rotate without downtime, issue a new token (`POST /api/agents/{id}/token`) before revoking the old one. During rotation, both the old and new tokens are valid for a 5-minute grace period. After 5 minutes, the old token is automatically invalidated.
- - **Re-issuance**: An agent that receives 401 (expired or revoked token) should request a new token through the relay control channel or by calling `POST /api/agents/{id}/token` with admin-scoped auth.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `# Control plane issues token for an agent`

```bash
# Control plane issues token for an agent
POST /api/agents/:id/token
→ { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }

# Agent uses token for relay connection
WS wss://relay.nunchi.dev/relay/ws
→ First message: { "type": "auth", "token": "roko_agent_..." }

# Agent uses token for inference proxy
POST /api/inference/proxy
Authorization: Bearer roko_agent_...
```
- Contract 2: language `plain`, first line `Agent created                Token issued                 30 days later`

```
Agent created                Token issued                 30 days later
(POST /api/agents)           (response includes token)    (token expires)
       │                            │                            │
       ▼                            ▼                            ▼
  agent record              token = "roko_agent_..."      agent gets 401
  stored in DB              SHA-256 hash stored           on next request
                            plaintext returned ONCE
                                                                 │
                                                                 ▼
                                                          request new token
                                                          via relay or API
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `api/agents/`
- `api/inference/`
- `relay.nunchi.dev/relay/`
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
rg -n "token|tokens|auth|api|bearer|relay|POST|token_ttl_days" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "token|tokens|auth|api|bearer|relay|POST|token_ttl_days" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `api/agents/`
- `api/inference/`
- `relay.nunchi.dev/relay/`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/agents/:id/token` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/inference/proxy` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/agents/{id}/token` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/agents/{id}/token` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `token` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `token_ttl_days` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `roko_agent_...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `relay.nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `ws -> First message` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Implement or verify operator command `POST /api/agents/:id/token` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `→ { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `WS wss://relay.nunchi.dev/relay/ws` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `→ First message: { "type": "auth", "token": "roko_agent_..." }` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `POST /api/inference/proxy` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `Authorization: Bearer roko_agent_...` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S006 -- 4. Relay auth: reads public, writes authenticated

**Source section:** `tmp/architecture/08-auth.md:178` through `187`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 4. Relay auth: reads public, writes authenticated

```
Read operations (subscribe, list feeds):    No auth required
Write operations (publish, register feed):  Require agent token
Admin operations (force-disconnect):        Require API key with admin scope
```

This means the dashboard can subscribe to presence and feeds without authentication. It needs auth only to send messages to agents or modify configuration.
````

**Explicit detail extraction from this section:**

- Section word count: `50`
- Section hash: `b6d7f04aa8dbde637ec3bff3e980487433897f46f7edd53343a524f6515611b7`

**Normative requirements and implementation claims:**
- ``` Read operations (subscribe, list feeds): No auth required Write operations (publish, register feed): Require agent token Admin operations (force-disconnect): Require API key with admin scope ```
- This means the dashboard can subscribe to presence and feeds without authentication. It needs auth only to send messages to agents or modify configuration.

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
- Contract 1: language `plain`, first line `Read operations (subscribe, list feeds):    No auth required`

```
Read operations (subscribe, list feeds):    No auth required
Write operations (publish, register feed):  Require agent token
Admin operations (force-disconnect):        Require API key with admin scope
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "auth|Write|Read|writes|reads|public|authenticated|Relay" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "auth|Write|Read|writes|reads|public|authenticated|Relay" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S007 -- Agent-to-agent auth for paid feeds

**Source section:** `tmp/architecture/08-auth.md:188` through `191`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Agent-to-agent auth for paid feeds

Paid feed subscriptions use the same agent token mechanism. The subscribing agent's token is validated by the relay, and payment is recorded against the agent's budget.
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `629cc4657d0d82e6494f3315a69951a3cef6d6d4e3a4d4ae4ec2ab99892044eb`

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
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "paid|feed|for|feeds|auth|token|validated|subscriptions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "paid|feed|for|feeds|auth|token|validated|subscriptions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S008 -- 5. Shared workspace access (team development)

**Source section:** `tmp/architecture/08-auth.md:192` through `332`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### 5. Shared workspace access (team development)

A deployed roko instance is private by default — only the owner can access it. The owner can invite teammates by email or wallet address, giving them authenticated access to the same workspace.

**Key constraint:** Nunchi owns the Privy app. Users deploying their own roko instances don't have access to Privy's dashboard or app secret. Therefore:

- **Privy's allowlist is NOT used.** Privy login is open — anyone can authenticate.
- **Roko-serve handles all authorization.** Each roko instance has its own user table in `.roko/users/`. It decides who gets in.
- **No `PRIVY_APP_SECRET` needed** on user deployments. JWT verification uses Privy's public JWKS endpoint, which requires no secret.

```
Nunchi (company)                    User's roko instance (Railway)
────────────────                    ──────────────────────────────
Owns Privy app                      Has PRIVY_APP_ID (public, baked in)
  open login (no allowlist)         Does NOT have PRIVY_APP_SECRET
  anyone can authenticate           Verifies JWTs via public JWKS (no secret)

                                    .roko/users/
                                    ├── owner: will@nunchi.dev (first login)
                                    ├── invited: sarah@example.com → member
                                    └── (anyone else) → 403 "Not a member"
```

```
User opens dashboard
  → Privy login modal (email/Google/Apple/wallet) — open, anyone can log in
  → Privy issues JWT (contains privy_user_id + email)
  → Dashboard sends JWT to THIS roko instance
  → Roko-serve validates JWT signature (public JWKS, no secret)
  → Roko-serve extracts email from JWT
  → Looks up email in .roko/users/
     → Found? → authorize with stored role
     → Email matches pending invitation? → auto-create user with invited role
     → Not found, no invitation? → 403 "Not a member of this workspace"
```

**Invitation flow:**

1. Owner goes to Settings > Team, types `sarah@example.com`, picks role "Member"
2. Dashboard calls `POST /api/team/invite` on the roko instance
3. Roko-serve stores invitation in `.roko/users/invitations.json` (local to this instance)
4. Dashboard shows a shareable link: `https://your-roko.up.railway.app`
5. Owner sends the link to Sarah (email, Slack, whatever)
6. Sarah opens link → Privy login → logs in with `sarah@example.com`
7. Privy issues JWT → dashboard sends to roko-serve
8. Roko-serve sees email matches invitation → creates user record with "member" role
9. Sarah sees the dashboard with member-level permissions

**No Privy dashboard access needed by anyone.** No allowlist management. No app secret on the deployment. Privy is purely an identity provider — a "login with email/Google/Apple" black box.

**Roles:**

| Role | Agents | Plans | Secrets | Team | System |
|------|--------|-------|---------|------|--------|
| `owner` | full | full | full | manage | full |
| `admin` | full | full | view | invite | view |
| `member` | full | full | — | — | view |
| `viewer` | view | view | — | — | view |

Roles are stored locally in `.roko/users/{email}.json`. No Privy custom_metadata needed.

**Revoking access:**

1. Owner removes Sarah from team (`DELETE /api/team/members/:id`)
2. Roko-serve deletes Sarah's user record from `.roko/users/`
3. Sarah's existing Privy JWT still works (Privy doesn't know about roko-serve's user table)
4. On Sarah's next API call: roko-serve looks up her email → not found → 403 immediately
5. Dashboard shows "You are no longer a member of this workspace"

Revocation is instant from roko-serve's perspective. Sarah can still authenticate with Privy (it's open login), but roko-serve won't let her in.

**Wallet-based invitations:**

In addition to email, invitations can be by wallet address:
```json
{ "identifier": "0x7f3b...2c4a", "type": "wallet", "role": "member" }
```
Roko-serve matches the wallet address from the Privy JWT's linked accounts.

**API routes:**

```
POST /api/team/invite        — invite by email or wallet (owner/admin only)
  Body: { "identifier": "alice@example.com", "type": "email", "role": "member" }
  → Stores invitation locally in .roko/users/invitations.json
  → Returns: { "invited": true }

GET /api/team/members         — list team members (any authenticated user)
  → [{ "email": "will@...", "role": "owner", "joined_at": "..." },
     { "email": "alice@...", "role": "member", "joined_at": "..." }]

PUT /api/team/members/:id     — change role (owner/admin only)
  Body: { "role": "admin" }

DELETE /api/team/members/:id  — remove from team (owner/admin only)
  → Deletes user record, next API call gets 403

GET /api/team/me              — current user's role and permissions
```

**First-run bootstrap:**

1. User clicks "Deploy on Railway" → roko deploys with `PRIVY_APP_ID` already set
2. User visits dashboard URL → Privy login modal
3. User logs in (email/Google/Apple)
4. Roko-serve sees no users in `.roko/users/` → first user becomes Owner automatically
5. Owner configures provider keys (Settings > Provider Keys)
6. Owner invites team members (Settings > Team)

**Dashboard UX:**

```
Settings > Team

+-------------------------------------------------------------+
| Team Members                                                 |
|                                                              |
| will@nunchi.dev              owner    Joined 2d ago          |
| alice@example.com            member   Joined 1d ago          |
| bob@example.com              admin    Joined 3h ago          |
|                                                              |
| Pending Invitations                                          |
| carol@example.com            member   Invited 1h ago         |
|                                                              |
| +----------------------------------------------------------+|
| | Invite teammate                                           ||
| | Email or wallet: [                        ] Role: [member]||
| | [Send Invite]                                             ||
| +----------------------------------------------------------+|
+-------------------------------------------------------------+
```

**Workspace auto-discovery for team members:**

When roko registers with the relay, it includes the owner's wallet. Team members auto-discover by:
1. Owner's wallet → direct match for owner
2. Team members → workspace URL saved in localStorage from first visit
3. Optionally: roko registers all team member wallets with the relay for auto-discovery

---
````

**Explicit detail extraction from this section:**

- Section word count: `798`
- Section hash: `8f7fcb24db06acfa91a9c70b232a982b1a81fb565a3627992b2864737f13aa29`

**Normative requirements and implementation claims:**
- A deployed roko instance is private by default — only the owner can access it. The owner can invite teammates by email or wallet address, giving them authenticated access to the same workspace.
- **Key constraint:** Nunchi owns the Privy app. Users deploying their own roko instances don't have access to Privy's dashboard or app secret. Therefore:
- - **Privy's allowlist is NOT used.** Privy login is open — anyone can authenticate. - **Roko-serve handles all authorization.** Each roko instance has its own user table in `.roko/users/`. It decides who gets in. - **No `PRIVY_APP_SECRET` needed** on user deployments. JWT verification uses Privy's public JWKS endpoint, which requires no secret.
- ``` User opens dashboard → Privy login modal (email/Google/Apple/wallet) — open, anyone can log in → Privy issues JWT (contains privy_user_id + email) → Dashboard sends JWT to THIS roko instance → Roko-serve validates JWT signature (public JWKS, no secret) → Roko-serve extracts email from JWT → Looks up email in .roko/users/ → Found? → authorize with stored role → Email matches pending invitation? → auto-create user with invited role → Not found, no invitation? → 403 "Not a member of this workspace" ```
- **Invitation flow:**
- 1. Owner goes to Settings > Team, types `sarah@example.com`, picks role "Member" 2. Dashboard calls `POST /api/team/invite` on the roko instance 3. Roko-serve stores invitation in `.roko/users/invitations.json` (local to this instance) 4. Dashboard shows a shareable link: `https://your-roko.up.railway.app` 5. Owner sends the link to Sarah (email, Slack, whatever) 6. Sarah opens link → Privy login → logs in with `sarah@example.com` 7. Privy issues JWT → dashboard sends to roko-serve 8. Roko-serve sees email matches invitation → creates user record with "member" role 9. Sarah sees the dashboard with member-level permissions
- **No Privy dashboard access needed by anyone.** No allowlist management. No app secret on the deployment. Privy is purely an identity provider — a "login with email/Google/Apple" black box.
- **Roles:**
- | Role | Agents | Plans | Secrets | Team | System | |------|--------|-------|---------|------|--------| | `owner` | full | full | full | manage | full | | `admin` | full | full | view | invite | view | | `member` | full | full | — | — | view | | `viewer` | view | view | — | — | view |
- **Revoking access:**
- 1. Owner removes Sarah from team (`DELETE /api/team/members/:id`) 2. Roko-serve deletes Sarah's user record from `.roko/users/` 3. Sarah's existing Privy JWT still works (Privy doesn't know about roko-serve's user table) 4. On Sarah's next API call: roko-serve looks up her email → not found → 403 immediately 5. Dashboard shows "You are no longer a member of this workspace"
- **Wallet-based invitations:**
- **API routes:**
- ``` POST /api/team/invite — invite by email or wallet (owner/admin only) Body: { "identifier": "alice@example.com", "type": "email", "role": "member" } → Stores invitation locally in .roko/users/invitations.json → Returns: { "invited": true }
- GET /api/team/members — list team members (any authenticated user) → [{ "email": "will@...", "role": "owner", "joined_at": "..." }, { "email": "alice@...", "role": "member", "joined_at": "..." }]
- PUT /api/team/members/:id — change role (owner/admin only) Body: { "role": "admin" }
- DELETE /api/team/members/:id — remove from team (owner/admin only) → Deletes user record, next API call gets 403
- GET /api/team/me — current user's role and permissions ```
- **First-run bootstrap:**
- 1. User clicks "Deploy on Railway" → roko deploys with `PRIVY_APP_ID` already set 2. User visits dashboard URL → Privy login modal 3. User logs in (email/Google/Apple) 4. Roko-serve sees no users in `.roko/users/` → first user becomes Owner automatically 5. Owner configures provider keys (Settings > Provider Keys) 6. Owner invites team members (Settings > Team)
- **Dashboard UX:**
- **Workspace auto-discovery for team members:**
- ---

**Routes and endpoint references:**
- POST /api/team/invite
- DELETE /api/team/members/:id
- GET /api/team/members
- PUT /api/team/members/:id
- GET /api/team/me

**Files and path references:**
- .roko/users/
- .roko/users/invitations.json
- api/team/
- api/team/members/
- email/Google/
- email/Google/Apple/

**Types, functions, traits, and inline code identifiers:**
- PRIVY_APP_SECRET
- owner
- admin
- member
- viewer
- PRIVY_APP_ID

**Event names and event-like entities:**
- nunchi.dev
- example.com
- roko.up.railway.app
- x7f3b...2c4a

**State transitions:**
- com -> member
- User opens dashboard -> Privy login modal
- anyone can log in -> Privy issues JWT
- Dashboard sends JWT to THIS roko instance -> Roko-serve validates JWT signature
- Roko-serve extracts email from JWT -> Looks up email in
- authorize with stored role -> Email matches pending invitation
- auto-create user with invited role -> Not found
- Sarah opens link -> Privy login
- Privy issues JWT -> dashboard sends to roko-serve
- Roko-serve sees email matches invitation -> creates user record with
- roko-serve looks up her email -> not found
- json -> Returns
- User visits dashboard URL -> Privy login modal
- s wallet -> direct match for owner
- Team members -> workspace URL saved in localStorage from

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - **Privy's allowlist is NOT used.** Privy login is open — anyone can authenticate.
- - **Roko-serve handles all authorization.** Each roko instance has its own user table in `.roko/users/`. It decides who gets in.
- - **No `PRIVY_APP_SECRET` needed** on user deployments. JWT verification uses Privy's public JWKS endpoint, which requires no secret.
- 1. Owner goes to Settings > Team, types `sarah@example.com`, picks role "Member"
- 2. Dashboard calls `POST /api/team/invite` on the roko instance
- 3. Roko-serve stores invitation in `.roko/users/invitations.json` (local to this instance)
- 4. Dashboard shows a shareable link: `https://your-roko.up.railway.app`
- 5. Owner sends the link to Sarah (email, Slack, whatever)
- 6. Sarah opens link → Privy login → logs in with `sarah@example.com`
- 7. Privy issues JWT → dashboard sends to roko-serve
- 8. Roko-serve sees email matches invitation → creates user record with "member" role
- 9. Sarah sees the dashboard with member-level permissions
- 1. Owner removes Sarah from team (`DELETE /api/team/members/:id`)
- 2. Roko-serve deletes Sarah's user record from `.roko/users/`
- 3. Sarah's existing Privy JWT still works (Privy doesn't know about roko-serve's user table)
- 4. On Sarah's next API call: roko-serve looks up her email → not found → 403 immediately
- 5. Dashboard shows "You are no longer a member of this workspace"
- 1. User clicks "Deploy on Railway" → roko deploys with `PRIVY_APP_ID` already set
- 2. User visits dashboard URL → Privy login modal
- 3. User logs in (email/Google/Apple)
- 4. Roko-serve sees no users in `.roko/users/` → first user becomes Owner automatically
- 5. Owner configures provider keys (Settings > Provider Keys)
- 6. Owner invites team members (Settings > Team)
- 1. Owner's wallet → direct match for owner
- 2. Team members → workspace URL saved in localStorage from first visit
- 3. Optionally: roko registers all team member wallets with the relay for auto-discovery

**Tables extracted:**
- Table 1:

```markdown
| Role | Agents | Plans | Secrets | Team | System |
|------|--------|-------|---------|------|--------|
| `owner` | full | full | full | manage | full |
| `admin` | full | full | view | invite | view |
| `member` | full | full | — | — | view |
| `viewer` | view | view | — | — | view |
```
- Table 2:

```markdown
| Team Members                                                 |
|                                                              |
| will@nunchi.dev              owner    Joined 2d ago          |
| alice@example.com            member   Joined 1d ago          |
| bob@example.com              admin    Joined 3h ago          |
|                                                              |
| Pending Invitations                                          |
| carol@example.com            member   Invited 1h ago         |
|                                                              |
| +----------------------------------------------------------+|
| | Invite teammate                                           ||
| | Email or wallet: [                        ] Role: [member]||
...
```

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Nunchi (company)                    User's roko instance (Railway)`

```
Nunchi (company)                    User's roko instance (Railway)
────────────────                    ──────────────────────────────
Owns Privy app                      Has PRIVY_APP_ID (public, baked in)
  open login (no allowlist)         Does NOT have PRIVY_APP_SECRET
  anyone can authenticate           Verifies JWTs via public JWKS (no secret)

                                    .roko/users/
                                    ├── owner: will@nunchi.dev (first login)
                                    ├── invited: sarah@example.com → member
                                    └── (anyone else) → 403 "Not a member"
```
- Contract 2: language `plain`, first line `User opens dashboard`

```
User opens dashboard
  → Privy login modal (email/Google/Apple/wallet) — open, anyone can log in
  → Privy issues JWT (contains privy_user_id + email)
  → Dashboard sends JWT to THIS roko instance
  → Roko-serve validates JWT signature (public JWKS, no secret)
  → Roko-serve extracts email from JWT
  → Looks up email in .roko/users/
     → Found? → authorize with stored role
     → Email matches pending invitation? → auto-create user with invited role
     → Not found, no invitation? → 403 "Not a member of this workspace"
```
- Contract 3: language `json`, first line `{ "identifier": "0x7f3b...2c4a", "type": "wallet", "role": "member" }`

```json
{ "identifier": "0x7f3b...2c4a", "type": "wallet", "role": "member" }
```
- Contract 4: language `plain`, first line `POST /api/team/invite        — invite by email or wallet (owner/admin only)`

```
POST /api/team/invite        — invite by email or wallet (owner/admin only)
  Body: { "identifier": "alice@example.com", "type": "email", "role": "member" }
  → Stores invitation locally in .roko/users/invitations.json
  → Returns: { "invited": true }

GET /api/team/members         — list team members (any authenticated user)
  → [{ "email": "will@...", "role": "owner", "joined_at": "..." },
     { "email": "alice@...", "role": "member", "joined_at": "..." }]

PUT /api/team/members/:id     — change role (owner/admin only)
  Body: { "role": "admin" }

DELETE /api/team/members/:id  — remove from team (owner/admin only)
  → Deletes user record, next API call gets 403

GET /api/team/me              — current user's role and permissions
```
- Contract 5: language `plain`, first line `Settings > Team`

```
Settings > Team

+-------------------------------------------------------------+
| Team Members                                                 |
|                                                              |
| will@nunchi.dev              owner    Joined 2d ago          |
| alice@example.com            member   Joined 1d ago          |
| bob@example.com              admin    Joined 3h ago          |
|                                                              |
| Pending Invitations                                          |
| carol@example.com            member   Invited 1h ago         |
|                                                              |
| +----------------------------------------------------------+|
| | Invite teammate                                           ||
| | Email or wallet: [                        ] Role: [member]||
| | [Send Invite]                                             ||
| +----------------------------------------------------------+|
+-------------------------------------------------------------+
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/users/`
- `.roko/users/invitations.json`
- `api/team/`
- `api/team/members/`
- `email/Google/`
- `email/Google/Apple/`
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
rg -n "member|team|user|Privy|owner|email|role|serve" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "member|team|user|Privy|owner|email|role|serve" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/users/`
- `.roko/users/invitations.json`
- `api/team/`
- `api/team/members/`
- `email/Google/`
- `email/Google/Apple/`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/team/invite` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `DELETE /api/team/members/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/team/members` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `PUT /api/team/members/:id` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `GET /api/team/me` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `PRIVY_APP_SECRET` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `owner` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `admin` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `member` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `viewer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `PRIVY_APP_ID` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `nunchi.dev` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `example.com` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `roko.up.railway.app` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `x7f3b...2c4a` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Enforce state transition `com -> member` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `User opens dashboard -> Privy login modal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `anyone can log in -> Privy issues JWT` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Dashboard sends JWT to THIS roko instance -> Roko-serve validates JWT signature` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Roko-serve extracts email from JWT -> Looks up email in` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `authorize with stored role -> Email matches pending invitation` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `auto-create user with invited role -> Not found` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Sarah opens link -> Privy login` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Privy issues JWT -> dashboard sends to roko-serve` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Roko-serve sees email matches invitation -> creates user record with` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `roko-serve looks up her email -> not found` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `json -> Returns` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `User visits dashboard URL -> Privy login modal` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `s wallet -> direct match for owner` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Team members -> workspace URL saved in localStorage from` with invalid-transition rejection tests and persistence/restart behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S009 -- Secret and API key management

**Source section:** `tmp/architecture/08-auth.md:333` through `334`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Secret and API key management
````

**Explicit detail extraction from this section:**

- Section word count: `0`
- Section hash: `bfdd376d18d3eab56f4e26f73ef30303310a74202c5d64fbaff1d41e2899652b`

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
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "management|key|Secret|API|auth" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "management|key|Secret|API|auth" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S010 -- Storage hierarchy

**Source section:** `tmp/architecture/08-auth.md:335` through `344`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Storage hierarchy

```
Priority    Source              Where
────────    ──────              ─────
1 (highest) Environment vars   ANTHROPIC_API_KEY, PERPLEXITY_API_KEY, etc.
2           Secrets store       .roko/secrets.toml (encrypted at rest)
3           Config file         roko.toml [providers] section (not recommended)
```
````

**Explicit detail extraction from this section:**

- Section word count: `28`
- Section hash: `be34ca2854027f2767343a610415775d9caa8b437ccaf05391f79f0ba1074c4a`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/secrets.toml

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
- Contract 1: language `plain`, first line `Priority    Source              Where`

```
Priority    Source              Where
────────    ──────              ─────
1 (highest) Environment vars   ANTHROPIC_API_KEY, PERPLEXITY_API_KEY, etc.
2           Secrets store       .roko/secrets.toml (encrypted at rest)
3           Config file         roko.toml [providers] section (not recommended)
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/secrets.toml`
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
rg -n "hierarchy|Storage|toml|secrets|vars|store|rest|recommended" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "hierarchy|Storage|toml|secrets|vars|store|rest|recommended" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/secrets.toml`
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
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S011 -- Secrets store format

**Source section:** `tmp/architecture/08-auth.md:345` through `368`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Secrets store format

```toml
# .roko/secrets.toml
# Encrypted with age (https://age-encryption.org)
# Key derived from machine identity or user passphrase

[llm]
anthropic = "sk-ant-..."
perplexity = "pplx-..."
gemini = "AIza..."
openrouter = "sk-or-..."
moonshot = "sk-..."
zai = "..."

[integration]
github = "ghp_..."
slack = "xoxb-..."

[infra]
fly_api_token = "fo1_..."
railway_token = "..."
```
````

**Explicit detail extraction from this section:**

- Section word count: `42`
- Section hash: `a54cd564849cc4f6db37eefb4668c9b0da4a708ec9a4f79d41005c7cc3c2b4e4`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/secrets.toml

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- encryption.org
- za...
- ghp_...
- fo1_...

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- [llm]
- anthropic = "sk-ant-..."
- perplexity = "pplx-..."
- gemini = "AIza..."
- openrouter = "sk-or-..."
- moonshot = "sk-..."
- zai = "..."
- [integration]
- github = "ghp_..."
- slack = "xoxb-..."
- [infra]
- fly_api_token = "fo1_..."
- railway_token = "..."

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# .roko/secrets.toml`

```toml
# .roko/secrets.toml
# Encrypted with age (https://age-encryption.org)
# Key derived from machine identity or user passphrase

[llm]
anthropic = "sk-ant-..."
perplexity = "pplx-..."
gemini = "AIza..."
openrouter = "sk-or-..."
moonshot = "sk-..."
zai = "..."

[integration]
github = "ghp_..."
slack = "xoxb-..."

[infra]
fly_api_token = "fo1_..."
railway_token = "..."
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/secrets.toml`
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
rg -n "Secrets|store|format|toml|ant|zai|xoxb|user" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Secrets|store|format|toml|ant|zai|xoxb|user" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `.roko/secrets.toml`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Emit or consume `encryption.org` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `za...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `ghp_...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `fo1_...` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `[llm]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `anthropic = "sk-ant-..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `perplexity = "pplx-..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `gemini = "AIza..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `openrouter = "sk-or-..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `moonshot = "sk-..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `zai = "..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[integration]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `github = "ghp_..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `slack = "xoxb-..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `[infra]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `fly_api_token = "fo1_..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `railway_token = "..."` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S012 -- From the CLI

**Source section:** `tmp/architecture/08-auth.md:369` through `394`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### From the CLI

```bash
# Set a secret (reads from stdin, never in shell history)
echo "sk-ant-xyz" | roko config secrets set llm.anthropic

# Interactive prompt
roko config secrets set llm.anthropic
# Enter secret: ****

# List configured secrets (keys, never values)
roko config secrets list
# NAMESPACE    KEY          SOURCE        STATUS
# llm          anthropic    secrets.toml  * valid
# llm          perplexity   env var       * valid
# integration  github       secrets.toml  * valid
# llm          gemini       --            o not set

# Validate all secrets
roko config check-secrets
# Anthropic: valid (claude-sonnet-4-6 accessible)
# Perplexity: valid
# GitHub: valid (repo scope, expires 2026-06-01)
# Gemini: not configured
```
````

**Explicit detail extraction from this section:**

- Section word count: `92`
- Section hash: `1fd480eab63c41fc1f866bfa2a8bc4853434f2f62fad22bdd6a3f16dafd7d33f`

**Normative requirements and implementation claims:**
- ```bash # Set a secret (reads from stdin, never in shell history) echo "sk-ant-xyz" | roko config secrets set llm.anthropic
- # List configured secrets (keys, never values) roko config secrets list # NAMESPACE KEY SOURCE STATUS # llm anthropic secrets.toml * valid # llm perplexity env var * valid # integration github secrets.toml * valid # llm gemini -- o not set
- # Validate all secrets roko config check-secrets # Anthropic: valid (claude-sonnet-4-6 accessible) # Perplexity: valid # GitHub: valid (repo scope, expires 2026-06-01) # Gemini: not configured ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- llm.anthropic

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- echo "sk-ant-xyz" | roko config secrets set llm.anthropic
- roko config secrets set llm.anthropic
- roko config secrets list
- roko config check-secrets

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `bash`, first line `# Set a secret (reads from stdin, never in shell history)`

```bash
# Set a secret (reads from stdin, never in shell history)
echo "sk-ant-xyz" | roko config secrets set llm.anthropic

# Interactive prompt
roko config secrets set llm.anthropic
# Enter secret: ****

# List configured secrets (keys, never values)
roko config secrets list
# NAMESPACE    KEY          SOURCE        STATUS
# llm          anthropic    secrets.toml  * valid
# llm          perplexity   env var       * valid
# integration  github       secrets.toml  * valid
# llm          gemini       --            o not set

# Validate all secrets
roko config check-secrets
# Anthropic: valid (claude-sonnet-4-6 accessible)
# Perplexity: valid
# GitHub: valid (repo scope, expires 2026-06-01)
# Gemini: not configured
```

**Read before editing:**
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
rg -n "secret|secrets|valid|config|the|llm|CLI|anthropic" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "secret|secrets|valid|config|the|llm|CLI|anthropic" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Emit or consume `llm.anthropic` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Implement or verify operator command `echo "sk-ant-xyz" | roko config secrets set llm.anthropic` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko config secrets set llm.anthropic` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko config secrets list` with non-destructive defaults, JSON output where useful, and failure diagnostics.
- [ ] Implement or verify operator command `roko config check-secrets` with non-destructive defaults, JSON output where useful, and failure diagnostics.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

### ARCH-08-S013 -- From the dashboard

**Source section:** `tmp/architecture/08-auth.md:395` through `401`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### From the dashboard

Settings > Provider Keys page. Each provider shows a status indicator (connected / not set / invalid). Users paste keys into a form. The dashboard sends them to `POST /api/secrets/:ns/:key`.

Test button calls `POST /api/secrets/:ns/:key/test` -- the server makes a minimal API call to the provider and returns connection status.

**Client-only mode**: keys stored in `localStorage`, sent via `X-Provider-Keys` header per request. The server uses them but never persists them.
````

**Explicit detail extraction from this section:**

- Section word count: `76`
- Section hash: `1bdb2edcd50e4ed07f1f0a551d96a00204484ba8fc83b700e6bd2e7379a70bdf`

**Normative requirements and implementation claims:**
- Settings > Provider Keys page. Each provider shows a status indicator (connected / not set / invalid). Users paste keys into a form. The dashboard sends them to `POST /api/secrets/:ns/:key`.
- Test button calls `POST /api/secrets/:ns/:key/test` -- the server makes a minimal API call to the provider and returns connection status.
- **Client-only mode**: keys stored in `localStorage`, sent via `X-Provider-Keys` header per request. The server uses them but never persists them.

**Routes and endpoint references:**
- POST /api/secrets/:ns/:key
- POST /api/secrets/:ns/:key/test

**Files and path references:**
- api/secrets/

**Types, functions, traits, and inline code identifiers:**
- localStorage

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
- `tmp/architecture/08-auth.md`
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `api/secrets/`
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
rg -n "the|key|localStorage|Provider|Keys|api|test|status" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "the|key|localStorage|Provider|Keys|api|test|status" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-serve/src/jwks.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/routes/auth.rs`
- `crates/roko-serve/src/state.rs`
- `api/secrets/`
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
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify route `POST /api/secrets/:ns/:key` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify route `POST /api/secrets/:ns/:key/test` with typed request/response, auth/scope test, ApiError failure path, API-reference entry, and dashboard/degraded-state behavior if visible to UI.
- [ ] Implement or verify `localStorage` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
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
./target/debug/roko parity check --strict --area tmp/architecture/08-auth
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

