# Plan 08: End-to-end acceptance and parity gates

**Layer:** 0-6+
**Effort:** L (1-2 weeks)
**Depends on:** Plans 01-07

## Goal

Define the final gate for "architecture parity." A feature is not complete
because its types compile or its route exists. It is complete only when the
full user path works across CLI, HTTP, WebSocket/SSE, dashboard/TUI, storage,
agents, gates, learning, docs, and deployment where applicable.

## Non-negotiable invariants

- All source docs are mapped in `.roko/parity/docs-ledger.json`.
- All mounted HTTP routes have documented request/response contracts and route tests.
- All long-running systems have health, metrics, shutdown, and recovery tests.
- All agent actions go through safety, audit, capability, and provenance hooks.
- All runtime events flow through Bus/StateHub and are visible to WS/SSE consumers.
- All dashboard pages degrade gracefully when optional services are absent.
- All "built but not wired" notes are either resolved or explicitly deferred.

## Tasks

### 8.1 Workspace build and static gates

**Commands:**

```bash
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cd contracts && forge test
```

**Implementation:**
- [ ] Add a `roko parity gates static` command that runs the Rust/Solidity static gate set
- [ ] Make output machine-readable as `.roko/parity/gates/static.json`
- [ ] Capture duration, failures, skipped tests, and environment metadata
- [ ] Add per-crate allowlist for intentionally skipped network tests

**Acceptance criteria:**
- [ ] Static gate exits 0 on a clean workspace
- [ ] Any failing crate or contract is reported with package, test, and command
- [ ] No destructive git commands are used by the gate

### 8.2 HTTP API and route-contract gates

**Read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/`
- `/Users/will/dev/nunchi/roko/roko/docs/API-REFERENCE.md`

**Implementation:**
- [ ] Generate route inventory from `routes/mod.rs` and mounted subrouters
- [ ] Compare inventory with OpenAPI/API docs
- [ ] Run smoke tests for health, auth, agents, plans, jobs, feeds, connectors, gateway, neuro, learning, deployments, projections, team, secrets, chain, SSE, and WS
- [ ] Test all auth scopes: unauthenticated, read, agent:write, plan:write, admin, agent token, Privy JWT
- [ ] Verify error responses use the common `ApiError` shape and redact secrets

**Acceptance criteria:**
- [ ] Every route has at least one smoke test
- [ ] Every mutating route has a scope test
- [ ] OpenAPI/API reference includes every mounted route
- [ ] Secret values never appear in test logs or JSON responses

### 8.3 Relay, Bus, StateHub, and realtime gates

**Implementation:**
- [ ] Start `roko-serve`, connect WS and SSE clients, and subscribe to system, agent, plan, feed, knowledge, arena, and group rooms
- [ ] Emit synthetic events through the runtime Bus and verify WS/SSE delivery
- [ ] Disconnect and reconnect with last sequence; verify replay or replay-gap behavior
- [ ] Overload event channel; verify coalescing/backpressure metrics instead of producer blocking
- [ ] Query StateHub projections and verify freshness metadata

**Acceptance criteria:**
- [ ] Event-to-render/projection latency p95 is under the target in `tmp/architecture/15-dashboard.md`
- [ ] Reconnect replay produces no duplicate state transitions
- [ ] Optional unavailable services produce degraded state, not panics

### 8.4 Agent lifecycle and safety gates

**Implementation:**
- [ ] Create an agent from CLI and HTTP
- [ ] Start, stop, restart, delete, backup, restore, and register the agent
- [ ] Issue, use, refresh, and revoke an agent token
- [ ] Run a tool-using task and assert safety hooks execute: capability token, allowlist, taint, path, network, bash, git, spending, witness, audit
- [ ] Verify process supervision: graceful stop, SIGKILL fallback, no orphan processes
- [ ] Verify ephemeral, persistent, and reactive modes

**Acceptance criteria:**
- [ ] Agent lifecycle state is consistent across CLI, HTTP, discovery registry, StateHub, and dashboard/TUI
- [ ] Unsafe tool action is blocked before side effects
- [ ] Audit chain contains provenance for every allowed tool action

### 8.5 Self-hosting workflow gate

**Scenario:**

```bash
roko prd idea "Add a small test-only feature"
roko prd draft new self-hosting-smoke
roko research enhance-prd self-hosting-smoke
roko prd plan self-hosting-smoke
roko plan run .roko/plans/self-hosting-smoke
```

**Implementation:**
- [ ] Run the scenario through CLI
- [ ] Run the same scenario through HTTP/dashboard plan endpoints
- [ ] Verify live events in WS/SSE
- [ ] Verify task review diff, gates, approve/reject/skip, and merge queue
- [ ] Kill the process mid-run and resume from snapshot/event log

**Acceptance criteria:**
- [ ] PRD -> plan -> execution -> gates -> review -> merge works end to end
- [ ] Resume produces exactly one final state, no duplicated completed tasks
- [ ] Learning episode, gate verdict, cost, and conductor signals are persisted

### 8.6 Cognitive loop gate

**Implementation:**
- [ ] Run a persistent agent with gamma/theta/delta enabled
- [ ] Feed synthetic observations that trigger T0, T1, and T2
- [ ] Verify T0 reflex execution, T1 normal inference, T2 deep inference, and sleepwalk on budget exhaustion
- [ ] Trigger theta reflection and verify PAD/behavioral-state/conductor updates
- [ ] Trigger delta consolidation and verify NREM/REM/integration outputs reach Neuro

**Acceptance criteria:**
- [ ] Tier decisions are explainable and logged with prediction error inputs
- [ ] Delta cycle creates or updates KnowledgeEntry records
- [ ] Dream outputs are visible in CLI, API, and dashboard/TUI

### 8.7 Chain, DeFi, registry, and payment gates

**Implementation:**
- [ ] Start `mirage-rs`, deploy contracts, and seed fixtures
- [ ] Register an agent passport and index the event
- [ ] Publish/validate/challenge knowledge and update reputation
- [ ] Run DeFi batch smoke: chain logs -> triage -> VenueAdapter -> risk gate -> simulated action -> P&L attribution
- [ ] Run paid feed smoke: create paid feed, deny unpaid subscriber, settle payment/session, stream feed
- [ ] Run Oracle smoke: chain/coding/research prediction -> score -> calibration update

**Acceptance criteria:**
- [ ] Indexer catches up to current block with zero event loss
- [ ] Simulated DeFi action never bypasses TxSimGate/WalletGate/MEV/risk checks
- [ ] Payment-gated feed enforces access and records cost/revenue

### 8.8 Dashboard, TUI, docs, and deployment gates

**Implementation:**
- [ ] Dashboard relay-only mode: mirage/relay running, roko-serve down
- [ ] Dashboard full workspace mode: roko-serve + agents + WS/SSE
- [ ] TUI surface inventory: all required screens render non-stub data or a documented degraded placeholder
- [ ] Docs parity: run `roko parity check --strict`
- [ ] Deployment smoke: Docker compose, Railway config validation, Fly config validation, daemon install dry-run for launchd/systemd

**Acceptance criteria:**
- [ ] Dashboard has no console errors in relay-only mode
- [ ] Dashboard can create/start/message/stream/review an agent in full mode
- [ ] TUI exposes plan, agent, knowledge, collective, and system status
- [ ] Generated docs and route/CLI references have no drift
- [ ] Deployment dry-runs produce concrete commands and validated env requirements

## Gate implementation packets

These packets define how to implement each acceptance gate. A fresh Codex
agent should implement one packet at a time, commit only its owned code paths,
and leave the final `roko parity gates all --strict` aggregator green or with a
machine-readable failing report.

### Shared gate result contract

Every gate command must write JSON under `.roko/parity/gates/` using this
shape. Human-readable output is allowed, but the JSON is the source of truth.

```json
{
  "gate": "static",
  "strict": true,
  "status": "pass",
  "started_at": "2026-04-25T12:00:00Z",
  "finished_at": "2026-04-25T12:03:11Z",
  "duration_ms": 191000,
  "environment": {
    "os": "darwin",
    "arch": "aarch64",
    "rustc": "rustc ...",
    "forge": "forge ...",
    "workspace_root": "."
  },
  "checks": [
    {
      "id": "cargo-check-workspace",
      "command": "cargo check --workspace --all-targets",
      "status": "pass",
      "duration_ms": 42000,
      "stdout_tail": "",
      "stderr_tail": ""
    }
  ],
  "failures": [],
  "warnings": [],
  "artifacts": []
}
```

**Status values:** `pass`, `fail`, `skip`, `degraded`.

**Required CLI behavior:**

```bash
roko parity gates static --strict
roko parity gates routes --strict
roko parity gates realtime --strict
roko parity gates lifecycle --strict
roko parity gates self-hosting --strict
roko parity gates cognitive --strict
roko parity gates chain --strict
roko parity gates surfaces --strict
roko parity gates all --strict
```

`--strict` exits non-zero on `fail` or unexpected `skip`. Non-strict mode may
return zero with `degraded` if optional external services are absent and the
degradation is documented in the report.

### Packet 8.1 -- workspace build and static gates

**Start state to verify:**

```bash
sed -n '1,180p' Cargo.toml
find contracts -maxdepth 2 -type f | sort
rg -n "ParityCommand|parity gates|surface_inventory|cargo check|forge test" crates/roko-cli/src
```

**Concrete implementation steps:**

1. Add `roko parity gates static` under the Plan 07 parity command module.
2. Implement a small command runner that captures command, exit status,
   duration, stdout/stderr tails, and environment metadata without invoking a
   shell unless necessary.
3. Run these commands by default:
   `cargo check --workspace --all-targets`,
   `cargo test --workspace --all-targets`,
   `cargo clippy --workspace --all-targets --no-deps -- -D warnings`, and
   `forge test` from `contracts/` if Foundry is installed.
4. Add `.roko/parity/gates/allowlist.toml` support for intentionally skipped
   network, credentialed, flaky, or platform-specific checks. Allowlist entries
   need owner, reason, expiry date, and replacement gate.
5. Write `.roko/parity/gates/static.json` with the shared result contract.
6. Add unit tests for command result serialization and allowlist expiry; add an
   integration test using harmless commands so CI does not need full workspace
   runtime.

**Verification commands:**

```bash
cargo test -p roko-cli parity_gates_static
./target/debug/roko parity gates static --strict
```

**Completion rule:** Static gate is not complete if it only shells out and
prints text. It must emit machine-readable JSON with per-command failures.

### Packet 8.2 -- HTTP API and route-contract gates

**Start state to verify:**

```bash
sed -n '1,220p' crates/roko-serve/src/routes/mod.rs
find crates/roko-serve/src/routes -maxdepth 1 -type f | sort
rg -n "route\\(|Router|ApiError|auth|scope|middleware|openapi|API-REFERENCE" crates/roko-serve docs/API-REFERENCE.md
```

**Concrete implementation steps:**

1. Generate a route inventory by parsing or instrumenting the mounted Axum
   routers. Include method, path, handler name if available, auth requirement,
   request type, response type, and source file.
2. Compare inventory to `docs/API-REFERENCE.md` and `/api/openapi.json` if it
   exists. Missing docs are failures in strict mode.
3. Build a route smoke harness that can start `roko-serve` on an ephemeral port
   with temp workspace state and no real secrets.
4. For every mounted route, add at least one smoke test. Mutating routes need
   scope tests for unauthenticated, read-only, required scope, admin, agent
   token, and Privy JWT where applicable.
5. Assert all error responses use the common `ApiError` shape and never include
   configured secret values, token hashes, provider keys, private keys, or full
   Authorization headers.
6. Write `.roko/parity/gates/routes.json` with route counts, covered routes,
   missing tests, missing docs, auth failures, redaction failures, and drift.

**Verification commands:**

```bash
cargo test -p roko-serve route_inventory
cargo test -p roko-serve route_smoke
cargo test -p roko-serve auth_scope_matrix
./target/debug/roko parity gates routes --strict
```

**Fixtures to include:**

- Temp `.roko/` workspace with one plan, one agent, one PRD, one config file,
  one connector, one feed, one fake secret, and one job.
- Test JWT/JWKS fixture for Privy-compatible validation.
- Agent token fixture with active, expired, revoked, and wrong-scope tokens.

### Packet 8.3 -- relay, Bus, StateHub, and realtime gates

**Start state to verify:**

```bash
rg -n "SSE|WebSocket|ws|broadcast|StateHub|Bus|projection|sequence|replay|backpressure|coalesce|event" crates/roko-serve crates/roko-runtime apps/mirage-rs crates/roko-core
sed -n '1,180p' crates/roko-serve/src/events.rs
```

**Concrete implementation steps:**

1. Add a realtime gate harness that starts `roko-serve` with temp state,
   connects one SSE client and one WebSocket client, and records all received
   events with sequence ids.
2. Subscribe to required rooms/topics: system, agent, plan, feed, knowledge,
   arena, group, chain, gateway, heartbeat, and parity. If a room is not yet
   implemented, strict mode must fail unless Plan 07 ledger marks it deferred.
3. Emit synthetic Bus events for every topic through production event paths,
   not by writing directly to the WebSocket sink.
4. Disconnect clients, reconnect with last sequence id, and verify replay or
   explicit replay-gap behavior. Duplicate terminal state transitions fail.
5. Overload the event channel with a bounded burst. Verify producers do not
   block indefinitely and metrics report drops/coalescing/backpressure.
6. Query StateHub projections and verify freshness metadata:
   `source`, `sequence`, `generated_at`, `stale_after`, `last_event_id`, and
   `degraded_reason`.
7. Write `.roko/parity/gates/realtime.json`.

**Verification commands:**

```bash
cargo test -p roko-serve realtime_gate
cargo test -p roko-runtime bus_statehub
./target/debug/roko parity gates realtime --strict
```

**Latency target:** Read the actual target from
`tmp/architecture/15-dashboard.md`; do not hard-code a different value. The
gate report must include p50, p95, p99, max, and sample count.

### Packet 8.4 -- agent lifecycle and safety gates

**Start state to verify:**

```bash
rg -n "create agent|managed-agents|token|backup|restore|delete|start|stop|restart|register|ephemeral|persistent|reactive|SafetyLayer|Capability|audit|taint|process" crates/roko-cli crates/roko-serve crates/roko-agent crates/roko-runtime
```

**Concrete implementation steps:**

1. Build a lifecycle scenario runner that can drive both CLI and HTTP against a
   temp workspace.
2. Create an agent through CLI, create an equivalent agent through HTTP, and
   assert consistent fields in registry, StateHub, `/api/agents`, dashboard
   projection data if available, and TUI inventory.
3. Start, stop, restart, backup, restore, delete, and register the agent. Each
   operation must produce an audit/provenance event and a lifecycle transition.
4. Issue, use, refresh, revoke, expire, and wrong-scope an agent token. Verify
   access succeeds/fails at protected routes and event streams as expected.
5. Run a tool-using task that attempts allowed and denied actions for path,
   network, bash, git, spending, custody, and secret access. Assert denial
   happens before side effects.
6. Verify process supervision: graceful stop, timeout, SIGKILL fallback,
   orphan cleanup, pid reuse safety, log capture, and restart policy.
7. Run the same lifecycle in ephemeral, persistent, and reactive modes.
8. Write `.roko/parity/gates/lifecycle.json`.

**Verification commands:**

```bash
cargo test -p roko-agent lifecycle_safety
cargo test -p roko-cli agent_lifecycle
cargo test -p roko-serve agent_lifecycle_routes
./target/debug/roko parity gates lifecycle --strict
```

**Completion rule:** This gate fails if any agent action can bypass safety,
audit, capability, or provenance hooks on the production path.

### Packet 8.5 -- self-hosting workflow gate

**Start state to verify:**

```bash
rg -n "prd|research enhance-prd|plan run|plans/generate|execute|pause|resume|review|diff|gate|merge queue|snapshot|episode|cost|conductor" crates/roko-cli crates/roko-serve crates/roko-orchestrator crates/roko-gate crates/roko-learn
```

**Concrete implementation steps:**

1. Add a deterministic self-hosting smoke fixture. It should use a tiny
   test-only feature, such as adding a generated fixture file under a temp
   workspace, so the gate does not mutate the real repo.
2. Drive the fixture through CLI:
   `prd idea`, `prd draft new`, `research enhance-prd`, `prd plan`, and
   `plan run`.
3. Drive the same scenario through HTTP/dashboard plan endpoints using the
   route smoke client from 8.2. The dashboard itself can be covered in 8.8, but
   the HTTP plan path must be equivalent here.
4. Subscribe to WS/SSE during execution and assert plan/task/gate/review/merge
   events arrive in order.
5. Pause or kill the process after a known event, resume from snapshot/event
   log, and assert exactly one terminal state and no duplicate completed task.
6. Verify review diff, approve/reject/skip, gate result persistence, merge
   queue behavior, episode logging, cost recording, and conductor signals.
7. Write `.roko/parity/gates/self-hosting.json`.

**Verification commands:**

```bash
cargo test -p roko-cli self_hosting_smoke
cargo test -p roko-serve plan_execution_routes
cargo test -p roko-orchestrator resume_exactly_once
./target/debug/roko parity gates self-hosting --strict
```

**Non-negotiable:** Never run this gate against the real workspace without an
explicit `--allow-real-workspace` flag. Default behavior must use temp dirs.

### Packet 8.6 -- cognitive loop gate

**Start state to verify:**

```bash
rg -n "TickPipeline|CorticalState|gamma|theta|delta|T0|T1|T2|sleepwalk|prediction error|Daimon|PAD|DeltaConsumer|KnowledgeEntry|dream report" crates/roko-runtime crates/roko-daimon crates/roko-dreams crates/roko-neuro crates/roko-cli crates/roko-serve
```

**Concrete implementation steps:**

1. Add a synthetic persistent-agent fixture with controllable observations,
   budgets, prediction-error inputs, PAD state, and dream/knowledge stores.
2. Trigger T0, T1, and T2 tier decisions deterministically and assert the
   selected path: reflex execution, normal inference, deep inference, and
   sleepwalk on budget exhaustion.
3. Trigger theta manually and assert PAD/behavioral-state updates, conductor
   input, route/API status update, and event emission.
4. Trigger delta manually and assert NREM replay, REM counterfactual generation,
   integration, knowledge promotion/pruning, confidence update, lineage, and
   dream report persistence.
5. Verify CLI, API, WS/SSE, and TUI/dashboard projection visibility for tier
   decisions, theta update, delta report, and knowledge output.
6. Write `.roko/parity/gates/cognitive.json`.

**Verification commands:**

```bash
cargo test -p roko-runtime cognitive_loop_gate
cargo test -p roko-dreams dream_integration
cargo test -p roko-neuro knowledge_from_dreams
./target/debug/roko parity gates cognitive --strict
```

**Completion rule:** A cognitive primitive existing as a library is not enough.
The gate must prove a running agent uses it and emits observable outcomes.

### Packet 8.7 -- chain, DeFi, registry, and payment gates

**Start state to verify:**

```bash
rg -n "mirage|deploy|contract|passport|registry|indexer|knowledge|challenge|reputation|VenueAdapter|TxSimGate|WalletGate|MEV|paid feed|x402|MPP|oracle|calibration" apps/mirage-rs crates contracts tmp/defi/gap
```

**Concrete implementation steps:**

1. Add a chain gate harness that starts `mirage-rs` on an ephemeral port or
   connects to a caller-supplied devnet. It must never default to mainnet.
2. Deploy contracts from `contracts/`, seed fixtures, and record deployed
   addresses in the gate report.
3. Register an agent passport, emit the event, run the indexer, and assert the
   indexed state catches up to the current block without event loss.
4. Publish, validate, challenge, and resolve a knowledge item; assert
   reputation and registry state update.
5. Run a DeFi smoke path: chain logs -> triage -> VenueAdapter -> risk gate ->
   TxSimGate -> WalletGate -> MEV/risk checks -> simulated action -> P&L
   attribution. No real funds path is allowed.
6. Run paid feed smoke: create paid feed, deny unpaid subscriber, create or
   settle payment/session, stream feed to authorized subscriber, record cost
   and revenue.
7. Run oracle smoke: make chain/coding/research predictions, score outcomes,
   update calibration, store history, and expose through API/dashboard
   projection.
8. Write `.roko/parity/gates/chain.json`.

**Verification commands:**

```bash
cargo test -p mirage-rs chain_gate
cargo test -p roko-chain
cargo test -p roko-serve chain_payment_routes
cargo test -p roko-learn oracle_calibration
cd contracts && forge test
./target/debug/roko parity gates chain --strict
```

**Failure modes to cover:**

- Indexer restart mid-stream.
- Reorg or duplicate event.
- Unpaid feed subscriber reconnects with stale token.
- VenueAdapter returns malformed quote.
- Oracle outcome arrives after calibration window.

### Packet 8.8 -- dashboard, TUI, docs, and deployment gates

**Start state to verify:**

```bash
rg -n "VITE_ROKO|relay-only|backendOnline|connectivity|Playwright|vitest|TUI|surface inventory|Docker|Railway|Fly|launchd|systemd|daemon|CLI-REFERENCE|API-REFERENCE" /Users/will/dev/nunchi/nunchi-dashboard/src crates/roko-cli crates/roko-serve deploy docker docs
```

**Concrete implementation steps:**

1. Add dashboard relay-only smoke: start mirage/relay, keep roko-serve down,
   launch the dashboard, navigate required pages, assert no console errors,
   relay-backed data renders, and roko-serve-only pages show degraded
   placeholders.
2. Add dashboard full-workspace smoke: start roko-serve, agents, WS/SSE, and
   dashboard; create/start/message/stream/review an agent; create/run a plan;
   assert live UI updates.
3. Add or extend TUI surface inventory gate. Required screens must render real
   data or a documented degraded placeholder tied to a ledger deferral.
4. Run docs parity: `roko parity check --strict --include-links`, CLI reference
   drift check, API reference drift check, and generated status check.
5. Add deployment dry-runs: Docker compose config, Railway config validation,
   Fly config validation, launchd dry-run, systemd dry-run, daemon IPC smoke,
   required env var validation, secrets redaction, and telemetry endpoint
   sanity.
6. Write `.roko/parity/gates/surfaces.json`.

**Verification commands:**

```bash
cargo test -p roko-cli tui_surface_inventory
cargo test -p roko-cli docs_reference_drift
cargo test -p roko-serve api_reference_drift
./target/debug/roko parity check --strict --include-links
./target/debug/roko parity gates surfaces --strict
```

Dashboard tests live in the dashboard repo at
`/Users/will/dev/nunchi/nunchi-dashboard/`; use that repo's package manager and
existing test framework. If no Playwright/Vitest setup exists, add the smallest
smoke harness consistent with that codebase rather than inventing a parallel
test stack.

### Packet 8.all -- final aggregator

**Concrete implementation steps:**

1. Implement `roko parity gates all --strict` as an aggregator over all gate
   subcommands. It must preserve each child report and write
   `.roko/parity/final-report.json`.
2. Support `--only`, `--skip`, `--fail-fast`, `--keep-going`, `--json`, and
   `--root` options.
3. In strict mode, exit non-zero if any child gate fails, unexpectedly skips,
   has expired allowlist entries, or is missing its JSON artifact.
4. Link the final report from generated `docs/STATUS.md`.
5. Add a short "how to reproduce" section to every failure in the final report.

**Verification commands:**

```bash
cargo test -p roko-cli parity_gates_all
./target/debug/roko parity gates all --strict
```

## Final release gate

The architecture plans are complete only when this command succeeds:

```bash
roko parity gates all --strict
```

The command must run:
- static gates
- route-contract gates
- realtime gates
- lifecycle/safety gates
- self-hosting workflow gate
- cognitive loop gate
- chain/DeFi/payment gates
- dashboard/TUI/docs/deployment gates

The output must be persisted to `.roko/parity/final-report.json` and linked
from `docs/STATUS.md`.
