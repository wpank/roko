# Plan 07: Docs parity closure

**Layer:** 6+
**Effort:** XL (4-6 weeks with 4 agents)
**Depends on:** Plan 06

## Goal

Every requirement in the older `docs/` corpus is either implemented, wired into
runtime behavior, covered by a newer architecture phase, or explicitly deferred
with a testable future gate. No markdown file may remain unmapped.

## Audit scope

This plan closes the gap left by Plan 06, which focuses on
`tmp/architecture`. The older docs are wider and include surfaces that are not
fully represented by the new architecture docs.

**Source docs:**
- `/Users/will/dev/nunchi/roko/roko/docs/` -- 422 markdown files, ~208K total doc lines
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/` -- 22 newer architecture docs
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/` -- 41 DeFi implementation batches plus benchmarks
- `/Users/will/dev/nunchi/roko/roko/docs/API-REFERENCE.md`
- `/Users/will/dev/nunchi/roko/roko/docs/CLI-REFERENCE.md`
- `/Users/will/dev/nunchi/roko/roko/docs/STATUS.md`
- `/Users/will/dev/nunchi/roko/roko/docs/EXECUTIVE-SUMMARY.md`

**Codebase audited:**
- 28 Rust crates/apps in the workspace
- 936 Rust source files under `crates/` and `apps/`
- 20 Solidity files under `contracts/src` and `contracts/test`
- `roko-serve` currently mounts 30+ route modules

## Current-state summary

The dominant gap pattern is not "missing code." It is **built but not wired**:
heartbeat primitives exist but are not the universal runtime loop; safety layers
exist but do not protect every execution path; code intelligence exists but is
not universally injected into prompts; dreams and delta consolidation have
phase stubs; MCP/plugin/tool systems exist but are not first-class across all
agent lifecycles; dashboard/TUI docs describe screens that are not all wired.

## Coverage matrix

| Docs area | Source | Primary existing code | Covered by | Remaining parity work |
|-----------|--------|-----------------------|------------|-----------------------|
| Core architecture | `docs/00-architecture` | `roko-core`, `roko-runtime`, `roko-std`, `roko-fs`, `roko-primitives` | Plan 06 A, B, C, E, M + 7.2 | Kernel trait parity, score/engram/pulse/data-medium alignment, config schema validation, cross-cut arbitration, performance gates |
| Orchestration | `docs/01-orchestration` | `roko-orchestrator`, `roko-cli/src/orchestrate.rs` | Plan 06 OG + 7.3 | Remove duplicate orchestrator logic, wire event log/worktree/merge queue/recovery universally |
| Agents | `docs/02-agents` | `roko-agent`, provider adapters, MCP client, process modules | Plan 06 A, C + 7.4 | Universal ToolDispatcher/SafetyLayer, temperament propagation, provider parity tests |
| Composition | `docs/03-composition` | `roko-compose`, `roko-runtime/src/heartbeat_attention.rs` | Plan 06 A, C + 7.4 | SystemPromptBuilder runtime wiring, active inference and VCG budget use, affect-modulated retrieval |
| Verification | `docs/04-verification` | `roko-gate`, gate routes/status endpoints | Plan 06 OG, K + 7.4 | Gate selector coverage, PRM feedback, autonomous eval lifecycle, verdict-as-signal wiring |
| Learning | `docs/05-learning` | `roko-learn`, episode logs, routers, playbooks | Plan 06 OG, E + 7.4 | Neuro-aware routing, clustering cadence, playbook rule promotion, research-to-runtime ledger |
| Neuro | `docs/06-neuro` | `roko-neuro`, `roko-primitives` | Plan 06 E + 7.5 | Backup/restore CLI, HDC operations exposed on Engrams, resonance/resonator features, knowledge lifecycle API |
| Conductor | `docs/07-conductor` | `roko-conductor`, watchers, circuit breaker | Plan 06 OG, A + 7.3 | ConductorBandit wiring, watcher actions into orchestrator, dashboard/TUI status surfaces |
| Chain | `docs/08-chain` | `roko-chain`, `mirage-rs`, contracts | Plan 06 H, J + 7.8 | TxSimGate/WalletGate full behavior, Korai RPC parity, registries/indexer, chain heartbeat |
| Daimon | `docs/09-daimon` | `roko-daimon`, `roko-runtime::CorticalState` | Plan 06 A, H, I + 7.5 | Behavioral-state merge, tier bias to CascadeRouter, somatic marker retrieval, contagion |
| Dreams | `docs/10-dreams` | `roko-dreams`, `roko-runtime::DeltaConsumer` | Plan 06 E + 7.5 | Replace delta stubs, NREM/REM/integration, hypnagogia, oneirography/reporting |
| Safety | `docs/11-safety` | `roko-agent/src/safety`, `roko-orchestrator/src/safety`, gates | Plan 06 A, D, K + 7.6 | SafetyLayer on every path, witness DAG, taint/audit/capability enforcement tests |
| Interfaces | `docs/12-interfaces` | `roko-cli`, `roko-serve`, TUI, StateHub | Plans 01-04, 06 L + 7.7 | CLI command parity, `roko new`, TUI screen wiring, StateHub freshness, SDK UX |
| Coordination | `docs/13-coordination` | `roko-orchestrator::coordination`, feeds, groups planned | Plan 06 F, E + 7.8 | Groups, pheromone scope, mesh sync, subnets, c-factor operational metrics |
| Identity economy | `docs/14-identity-economy` | `roko-chain`, contracts, jobs | Plan 06 G, J + 7.8 | Passport expansion, marketplaces, x402/MPP, tokenomics, regulatory/event index |
| Code intelligence | `docs/15-code-intelligence` | `roko-index`, lang providers, `roko-mcp-code` | 7.6 | Tree-sitter completeness, usage refs, prompt injection, MCP server end-to-end |
| Heartbeat | `docs/16-heartbeat` | `roko-runtime/heartbeat*`, theta/delta consumers | Plan 06 A + 7.5 | Concurrent gamma/theta/delta runtime, TickPipeline, probe-to-tier-to-action loop |
| Lifecycle | `docs/17-lifecycle` | `roko-runtime::lifecycle`, agents routes, deployments | Plan 06 D, K + 7.7 | Type-state provisioning wired to `roko init`, backup/restore/delete flows |
| Tools/plugins | `docs/18-tools` | `roko-core::tool`, `roko-agent::tool_loop`, MCP crates, plugin SDK | Plan 06 M + 7.6 | Plugin loading, MCP servers verified, event sources, tool safety hooks universal |
| Deployment | `docs/19-deployment` | Docker/Fly/Railway configs, daemon modules, deploy routes | Plan 06 K + 7.7 | Release pipeline checks, daemon install wiring, observability, production hardening |
| Technical analysis | `docs/20-technical-analysis` | `roko-learn::oracles`, `roko-primitives`, DeFi batches | Plan 06 H + 7.8 | Oracle trait runtime API, coding/research/chain oracle use, TA benchmarks |
| References | `docs/21-references` | Docs only, research-to-runtime expected | 7.9 | Citation-to-runtime ledger, falsifier extraction, no code required unless referenced by another doc |

## Tasks

### 7.1 Build the parity ledger and inventory

**What:** Add a deterministic source inventory and docs coverage ledger so the
system can prove that every source doc has an implementation owner.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/INDEX.md`
- `/Users/will/dev/nunchi/roko/roko/docs/STATUS.md`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/parity.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/surface_inventory.rs`

**Target files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/parity.rs` (new or extend `surface_inventory.rs`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/parity.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status.rs`
- `/Users/will/dev/nunchi/roko/roko/.roko/parity/docs-ledger.json`
- `/Users/will/dev/nunchi/roko/roko/.roko/parity/source-inventory.json`

**Ledger row schema:**

```json
{
  "source_path": "docs/03-composition/10-vcg-attention-auction.md",
  "source_hash": "blake3...",
  "doc_area": "03-composition",
  "status": "implemented_by",
  "owning_plan": "07-docs-parity-closure",
  "owning_task": "7.4",
  "code_targets": ["crates/roko-runtime/src/heartbeat_attention.rs"],
  "route_targets": [],
  "tests": ["cargo test -p roko-runtime heartbeat_attention"],
  "acceptance": "VCG auction is called by context assembly under load.",
  "deferred_reason": null
}
```

**Implementation:**
- [ ] Walk `docs/**/*.md`, `tmp/architecture/*.md`, and `tmp/defi/gap/*.md`
- [ ] Hash every file with BLAKE3
- [ ] Extract headings, code references, route references, and "not wired/not implemented/stub" lines
- [ ] Generate `source-inventory.json` for crates, apps, contracts, route modules, tests, docs, and stubs
- [ ] Generate `docs-ledger.json` using the coverage matrix in this plan
- [ ] Add `roko parity check --strict` to fail when any non-reference doc is unmapped
- [ ] Extend `/api/parity` to return ledger summary, uncovered count, stale rows, and code/doc hash drift

**Acceptance criteria:**
- [ ] `roko parity inventory` produces both JSON files deterministically
- [ ] `roko parity check --strict` fails if one docs file is removed from the ledger
- [ ] `/api/parity` reports `uncovered_docs: 0`
- [ ] A changed source doc hash marks its ledger row stale until refreshed

### 7.2 Core kernel and configuration parity

**What:** Reconcile `docs/00-architecture` against the current core crates.
This closes primitive shape gaps that are too foundational to leave implied.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/16-config.md`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/`

**Target areas:**
- `roko-core`: Engram, Pulse, Datum, Score, Decay, Provenance, Bus, StateHub, config schema
- `roko-runtime`: energy, demurrage, heartbeat, resource limits, lifecycle
- `roko-fs`: substrate layout, GC, observability

**Implementation:**
- [ ] Confirm the canonical `Score` has the documented axes or add explicit deprecated/derived-axis mappings
- [ ] Ensure `Engram`, `Pulse`, and `Datum` support provenance, lineage, content addressing, decay, and signal graduation
- [ ] Add schema validation for every top-level `roko.toml` section described in `tmp/architecture/16-config.md`
- [ ] Wire config hot reload into serve, TUI, gateway, heartbeat, conductor, and secrets paths
- [ ] Add numerical stability tests for HDC, decay, demurrage, score normalization, and energy functions
- [ ] Add a `cargo test -p roko-core architecture_parity` suite that checks config defaults and serde roundtrips

**Acceptance criteria:**
- [ ] Every `docs/00-architecture/*.md` file has a ledger row
- [ ] `RokoConfig::default()` serializes to a valid full example accepted by the schema
- [ ] Core primitives have property tests for serde, hash stability, and no NaN/Inf outputs
- [ ] `/api/config` returns a schema version and validation errors by field

### 7.3 Orchestration, conductor, and process-management parity

**What:** Make `roko-orchestrator` the runtime source of truth and remove
parallel ad hoc behavior in `roko-cli/src/orchestrate.rs`.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/`
- `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/`
- `/Users/will/dev/nunchi/roko/roko/plans/P06-process-management/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

**Implementation:**
- [ ] Move DAG construction, wave scheduling, worktree isolation, event logging, snapshot recovery, merge queue, and repair through `roko-orchestrator` public APIs
- [ ] Keep `roko-cli/src/orchestrate.rs` as the CLI harness, not the canonical state machine
- [ ] Wire conductor watchers and `ConductorBandit` into every task iteration, not just diagnostics
- [ ] Emit StateHub/Bus events for every phase transition, gate verdict, retry, review, merge, and conductor intervention
- [ ] Add crash recovery tests that kill a plan mid-task and resume from event log + snapshot
- [ ] Add process-management tests for SIGTERM, SIGKILL fallback, orphan cleanup, and worktree lock cleanup

**Acceptance criteria:**
- [ ] A plan run uses `roko-orchestrator::executor` state transitions end-to-end
- [ ] Conductor interventions can pause, resume, reprioritize, or abort a task
- [ ] Event log replay reconstructs the same plan state as the latest snapshot
- [ ] Merge queue serializes conflicting branch merges and retries safely

### 7.4 Agents, composition, verification, and learning parity

**What:** Wire the agent harness, prompt assembly, gate system, and learning
loops across all provider/runtime paths.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/02-agents/`
- `/Users/will/dev/nunchi/roko/roko/docs/03-composition/`
- `/Users/will/dev/nunchi/roko/roko/docs/04-verification/`
- `/Users/will/dev/nunchi/roko/roko/docs/05-learning/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/`

**Implementation:**
- [ ] Route every model backend through the same `ToolLoop`, `ToolDispatcher`, `SafetyLayer`, retry policy, usage accounting, and transcript format
- [ ] Propagate temperament, domain profile, budget, gate rung, model-router decision, and tool profile into every provider request
- [ ] Make `SystemPromptBuilder` the only production system-prompt construction path for orchestration agents
- [ ] Wire VCG/attention/active-inference context selection into prompt assembly instead of leaving it as standalone primitives
- [ ] Publish every gate verdict as a signal for learning, routing, conductor, dashboard, and knowledge admission
- [ ] Turn review verdicts, compile-error classes, PRM scores, and eval-generation outputs into episode records
- [ ] Implement regression tests for provider parity: mock, Claude CLI, Codex, Cursor, OpenAI-compatible, Ollama, Gemini, Perplexity

**Acceptance criteria:**
- [ ] No provider path can bypass safety hooks without an explicit `unsafe_allow_bypass` test fixture
- [ ] Prompt traces show every SystemPromptBuilder layer with token budgets and truncation decisions
- [ ] Gate verdicts appear in episode logs, learning metrics, WebSocket/SSE, and dashboard projections
- [ ] CascadeRouter decisions include provider health, learning history, knowledge bias, and cost normalization

### 7.5 Neuro, daimon, dreams, and heartbeat parity

**What:** Convert cognitive cross-cuts from libraries/scaffolds into the live
agent runtime loop.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/06-neuro/`
- `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/`
- `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/`
- `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/heartbeat*.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/theta_consumer.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/delta_consumer.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/`

**Implementation:**
- [ ] Run gamma/theta/delta as concurrent async tasks with shared `CorticalState` and Bus topics
- [ ] Implement the 9-step TickPipeline facade using existing heartbeat primitives
- [ ] Compute prediction error from the full probe set and feed tier selection plus CascadeRouter
- [ ] Wire Daimon PAD/behavioral state into retrieval bias, tier thresholds, risk posture, and dashboard/TUI display
- [ ] Replace `DeltaConsumer` NREM/REM/integration stubs with `roko-dreams`, `roko-learn`, and `roko-neuro` calls
- [ ] Implement `roko neuro backup`, `roko neuro restore`, and `roko dream run/report` parity with docs
- [ ] Persist dream outputs as KnowledgeEntry records with lineage and confidence updates

**Acceptance criteria:**
- [ ] A running persistent agent emits gamma, theta, and delta events on schedule
- [ ] A theta tick updates PAD and can trigger conductor intervention
- [ ] A delta cycle replays episodes, generates counterfactuals, and promotes/prunes knowledge
- [ ] Knowledge extracted from dreams is queryable through CLI, API, and dashboard

### 7.6 Safety, tools, plugins, and code-intelligence parity

**What:** Make safety, plugins, MCP, and code intelligence production-path
features instead of optional islands.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/11-safety/`
- `/Users/will/dev/nunchi/roko/roko/docs/15-code-intelligence/`
- `/Users/will/dev/nunchi/roko/roko/docs/18-tools/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tool/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-*/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-plugin/src/`

**Implementation:**
- [ ] Enforce capability tokens, taint tracking, audit chain, path/network/bash/git/custody safety, and witness DAG on every tool action
- [ ] Implement `roko new` generators for gate, scorer, connector, feed, recipe, extension, MCP server, agent template, and domain profile; generated code must compile and test
- [ ] Complete MCP GitHub, Slack, Scripts, Stdio, and Code servers with integration tests and docs examples
- [ ] Wire `roko-index` into `roko-compose` context assembly and `roko-mcp-code`
- [ ] Populate symbol references, dependency graph, PageRank, HDC fingerprints, SQLite index, snapshots, and incremental updates
- [ ] Add plugin discovery/loading for manifest tiers that are safe to run locally; leave WASM/native unsafe tiers gated behind explicit permissions

**Acceptance criteria:**
- [ ] A tool call produces a capability-checked audit record with taint/provenance metadata
- [ ] `roko mcp code` answers symbol/search/dependency queries from a freshly indexed workspace
- [ ] `roko new gate my-gate` creates compiling code and tests without manual edits
- [ ] Plugin load failure is isolated and visible in health/status endpoints

### 7.7 Interfaces, lifecycle, and deployment parity

**What:** Align CLI, TUI, HTTP, dashboard, lifecycle, and deployment docs with
the actual user-facing product.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/`
- `/Users/will/dev/nunchi/roko/roko/docs/17-lifecycle/`
- `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/`
- `/Users/will/dev/nunchi/roko/roko/docs/API-REFERENCE.md`
- `/Users/will/dev/nunchi/roko/roko/docs/CLI-REFERENCE.md`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- `/Users/will/dev/nunchi/roko/roko/deploy/`
- `/Users/will/dev/nunchi/roko/roko/docker/`

**Implementation:**
- [ ] Generate CLI reference from `clap` and fail CI if `docs/CLI-REFERENCE.md` drifts
- [ ] Generate OpenAPI/reference from `roko-serve` routes and fail CI if `docs/API-REFERENCE.md` drifts
- [ ] Wire TUI screens for agent detail, plan detail, knowledge, collective, and system docs
- [ ] Implement StateHub freshness metadata on HTTP responses and projections
- [ ] Wire type-state provisioning to `roko init` and agent creation
- [ ] Implement backup, restore, deletion, selective restore, and knowledge transfer flows
- [ ] Verify Docker, Railway, Fly, launchd, systemd, daemon IPC, secrets, observability, and production-hardening docs against executable scripts/tests

**Acceptance criteria:**
- [ ] `roko --help` and `docs/CLI-REFERENCE.md` are generated from the same command tree
- [ ] `/api/openapi.json` includes every mounted route and documented request/response schema
- [ ] TUI surface inventory reports no required screen as `stub`
- [ ] Agent creation, backup, delete, restore, deploy, and status work from CLI and HTTP

### 7.8 Chain, coordination, identity economy, DeFi, and technical-analysis parity

**What:** Merge chain/economy/coordination/TA docs with Plan 06 phases and
DeFi gap batches.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/`
- `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/`
- `/Users/will/dev/nunchi/roko/roko/docs/14-identity-economy/`
- `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/12-defi.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/14-registries.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/`
- `/Users/will/dev/nunchi/roko/roko/contracts/src/`
- `/Users/will/dev/nunchi/roko/roko/apps/mirage-rs/src/`

**Implementation:**
- [ ] Complete `TxSimGate`, `WalletGate`, MEV gate, chain witness, wallet registry, and multi-chain subscription behavior
- [ ] Implement groups, pheromone scopes, mesh sync, permissioned subnets, c-factor metrics, and collective dashboards
- [ ] Expand passport, reputation, validation, knowledge, bounty, fee, worker, and identity contracts/routes to match docs
- [ ] Implement x402/MPP session/payment flows for paid feeds and agent services
- [ ] Execute all 41 DeFi gap batches in topological order and run benchmarks in `tmp/defi/gap/13-BENCHMARKS.md`
- [ ] Expose Oracle trait runtime surfaces for chain, coding, research, witness, HDC TA, spectral manifolds, causal discovery, robust/adversarial signals, and predictive geometry

**Acceptance criteria:**
- [ ] Mirage devnet deploys contracts, emits registry events, and indexer catches up with zero lag
- [ ] A DeFi agent can subscribe to chain logs, route through VenueAdapter, simulate risk, and produce a gated action
- [ ] Paid feed access denies without payment and streams after settlement
- [ ] Oracle predictions are scored, stored, calibrated, and visible through API/dashboard

### 7.9 Documentation reconciliation and research-to-runtime ledger

**What:** Once implementation catches up, update docs so they describe reality
and preserve future-facing research as falsifiable runtime work.

**Read:**
- `/Users/will/dev/nunchi/roko/roko/docs/21-references/`
- `/Users/will/dev/nunchi/roko/roko/docs/EXECUTIVE-SUMMARY.md`
- `/Users/will/dev/nunchi/roko/roko/docs/STATUS.md`
- `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/31-implementation-readiness-audit.md`
- `/Users/will/dev/nunchi/roko/roko/docs/05-learning/20-research-to-runtime.md`

**Implementation:**
- [ ] Replace stale "not implemented" notes where code now exists, with exact file references
- [ ] Keep future/research docs but attach falsifiers, runtime metrics, and experiment gates
- [ ] Add `research-to-runtime.jsonl` entries for every reference-backed architectural claim used by runtime code
- [ ] Update `docs/STATUS.md` from the parity ledger instead of manual status tables
- [ ] Add docs CI that rejects broken local links, stale code references, and unmapped docs

**Acceptance criteria:**
- [ ] `docs/STATUS.md` is generated from `docs-ledger.json`
- [ ] Every `docs/21-references/*.md` item used in implementation has a falsifier or measurement path
- [ ] Broken local links count is zero
- [ ] No doc claims "not implemented" for a feature whose ledger row is `implemented`

## Execution packets

The packets below are the detailed runbooks for the checklist items above.
They are intentionally explicit so a fresh Codex agent can execute one packet
without prior conversation context. If a packet conflicts with current code,
trust current code and update the packet outcome in the ledger.

### Packet 7.1 -- parity ledger and inventory

**Start state to verify:**

```bash
rg --files docs tmp/architecture tmp/defi/gap -g '*.md' | sort | wc -l
rg --files crates apps contracts -g '*.rs' -g '*.sol' | sort | wc -l
sed -n '1,140p' crates/roko-cli/src/main.rs
sed -n '1,220p' crates/roko-cli/src/surface_inventory.rs
sed -n '1,140p' crates/roko-serve/src/parity.rs
sed -n '1840,1895p' crates/roko-serve/src/routes/status.rs
```

**Concrete implementation steps:**

1. Add or extend a CLI module for parity commands. Prefer
   `crates/roko-cli/src/parity.rs`; if `surface_inventory.rs` already owns the
   relevant inventory functions, keep generic inventory there and expose the
   command-facing API from `parity.rs`.
2. Add a `ParityCommand` enum under the existing `clap` command tree in
   `crates/roko-cli/src/main.rs` with these subcommands:
   `inventory`, `check`, `status`, and `gates`. `gates` can initially dispatch
   only static/doc checks; Plan 08 expands it.
3. Define stable serde types:
   `DocCoverageStatus`, `DocLedgerRow`, `DocsLedger`, `SourceInventory`,
   `InventoryCounts`, `StubFinding`, `RouteInventory`, `TestInventory`, and
   `ParityCheckReport`.
4. Walk exactly these doc roots unless the caller passes overrides:
   `docs/**/*.md`, `tmp/architecture/*.md`, and `tmp/defi/gap/*.md`.
5. Hash each source file with BLAKE3. If the crate does not already depend on
   `blake3`, add it to the smallest crate that needs it and reuse the same hash
   code from CLI and server via a shared crate if practical.
6. Extract headings, fenced code block languages, local file references, route
   references such as `/api/...`, and gap markers matching:
   `not implemented`, `not wired`, `stub`, `placeholder`, `TODO`, `FIXME`,
   `future`, `deferred`, `missing`, `unimplemented`.
7. Generate `.roko/parity/source-inventory.json` deterministically with sorted
   arrays and counts for crates, apps, route modules, binaries, tests, docs,
   contracts, Solidity tests, TODO/FIXME markers, and Rust stub macros.
8. Generate `.roko/parity/docs-ledger.json` deterministically. The first pass
   may map rows by top-level docs directory using the coverage matrix in this
   plan, but it must never leave `unknown` or blank status values.
9. Implement `roko parity check --strict`. In strict mode fail if any
   non-reference row is missing, stale, blank, duplicated, has no owner, has no
   acceptance gate, or points to a missing target file.
10. Extend `crates/roko-serve/src/parity.rs` and `/api/parity` to load the
   ledger/inventory from disk and return summary fields:
   `total_docs`, `implemented`, `covered`, `deferred`, `reference_only`,
   `uncovered_docs`, `stale_rows`, `missing_targets`, `last_generated_at`,
   and `strict_pass`.

**Status decision rules:**

| Status | Use only when | Required fields |
|--------|---------------|-----------------|
| `implemented_by` | Runtime behavior exists or is implemented by this task | `owning_plan`, `owning_task`, `code_targets`, `tests`, `acceptance` |
| `covered_by` | Another doc has the identical runtime requirement | `covered_by_source`, `acceptance`, `owning_plan`, `owning_task` |
| `deferred` | The requirement is intentionally not implemented now | `owner`, `dependency`, `deferred_reason`, `future_acceptance` |
| `reference_only` | Background/citation material under `docs/21-references` | `reference_category`, optional `used_by` |

**Test requirements:**

```bash
cargo test -p roko-cli parity_inventory
cargo test -p roko-cli parity_check
cargo test -p roko-serve parity
./target/debug/roko parity inventory --root .
./target/debug/roko parity check --strict --root .
```

**Edge cases to implement:**

- Ledger order must be stable across OS/filesystem order.
- A moved doc should be reported as removed plus added unless its hash is
  matched and the row is updated.
- Broken symlinks should be reported as inventory warnings, not panics.
- `docs/21-references` may be `reference_only`; every other source root needs
  executable coverage or an explicit deferral.
- Generated files must not include machine-specific absolute paths except in
  optional debug fields.

### Packet 7.2 -- core kernel and configuration parity

**Start state to verify:**

```bash
rg -n "struct (Score|Engram|Pulse|Datum|RokoConfig)|enum (Score|Engram|Pulse|Datum)|trait .*Kernel|StateHub|Bus|Provenance|Decay" crates/roko-core crates/roko-runtime crates/roko-fs
rg -n "roko.toml|Config|schema|hot reload|watch" crates/roko-core crates/roko-cli crates/roko-serve crates/roko-runtime
rg -n "Score|Engram|Pulse|Datum|roko.toml|StateHub|Bus|substrate|GC|demurrage" docs/00-architecture tmp/architecture/16-config.md
```

**Concrete implementation steps:**

1. Create a reconciliation table in the ledger for every
   `docs/00-architecture/*.md` file before code changes. Each row must list the
   primitive or service it affects.
2. Identify the canonical Rust type for `Score`, `Engram`, `Pulse`, `Datum`,
   provenance, decay, Bus, StateHub, and config. If multiple versions exist,
   document the canonical owner and leave compatibility shims where needed.
3. Add missing serde fields with `#[serde(default)]` and migration-safe
   constructors. Do not break existing serialized state without a migration.
4. Add or extend config schema validation for every top-level section in
   `tmp/architecture/16-config.md`: providers, routing, agents, runtime,
   heartbeat, safety, tools, storage, chain, feeds, connectors, secrets,
   dashboard, TUI, deployment, and observability.
5. Wire config reload notifications into serve, gateway routing, heartbeat
   policy, conductor policy, secrets, and TUI state. If a subsystem cannot hot
   reload safely, return a typed "restart required" validation result.
6. Add property tests for serde roundtrip, hash stability, content addressing,
   no NaN/Inf numeric output, decay monotonicity, score normalization, and
   demurrage bounds.
7. Expose `/api/config/schema` or extend `/api/config` so API consumers can get
   schema version, default config, validation errors by field, and restart
   requirements.

**Test requirements:**

```bash
cargo test -p roko-core architecture_parity
cargo test -p roko-runtime heartbeat config
cargo test -p roko-fs substrate gc
cargo test -p roko-serve config
./target/debug/roko parity check --strict --area docs/00-architecture
```

**Do not finish until:**

- Every `docs/00-architecture/*.md` row is no longer `deferred` unless the row
  names a dependency and future command.
- A default `roko.toml` can be generated, parsed, validated, serialized, and
  accepted by serve.
- API validation errors include field paths, not only free-form text.

### Packet 7.3 -- orchestration, conductor, and process management parity

**Start state to verify:**

```bash
rg -n "DAG|ParallelExecutor|snapshot|event log|worktree|merge|repair|retry|resume|pause" crates/roko-cli/src/orchestrate.rs crates/roko-orchestrator crates/roko-conductor
rg -n "Watcher|ConductorBandit|CircuitBreaker|intervention|pause|reprioritize|abort" crates/roko-conductor crates/roko-orchestrator crates/roko-cli/src
rg -n "orchestrat|conductor|process|SIGTERM|SIGKILL|worktree|merge queue|event log" docs/01-orchestration docs/07-conductor plans/P06-process-management
```

**Concrete implementation steps:**

1. Inventory duplicate orchestration behavior in `roko-cli/src/orchestrate.rs`
   versus `roko-orchestrator`. Mark each function as `keep in CLI harness`,
   `move to orchestrator`, `wrap orchestrator API`, or `delete after migration`.
2. Add public orchestrator APIs for plan loading, DAG construction, wave
   scheduling, task leasing, agent dispatch request construction, gate
   execution request construction, event append, snapshot write/read, recovery,
   merge queue operations, and repair loops.
3. Update CLI execution so `orchestrate.rs` becomes a thin harness:
   argument parsing, workspace setup, user output, and calls into
   `roko-orchestrator`. It must not own the canonical state machine.
4. Wire `roko-conductor` watchers and `ConductorBandit` into every task
   iteration. Conductor actions must be typed events: `Pause`, `Resume`,
   `Reprioritize`, `EscalateGate`, `Abort`, `SpawnRepair`, `Throttle`, and
   `AnnotateOnly`.
5. Emit Bus/StateHub events for plan start, task lease, phase change, agent
   spawn, gate verdict, retry, conductor action, merge queued, merge completed,
   snapshot written, recovery started, recovery completed, and plan terminal
   state.
6. Implement crash recovery tests using a temp workspace: start a plan, force
   exit after a known event, resume, replay event log, compare replayed state
   to latest snapshot, then complete exactly once.
7. Implement process-management tests for SIGTERM, SIGKILL fallback, orphan
   cleanup, worktree lock cleanup, stale branch cleanup, and merge conflict
   serialization.

**Test requirements:**

```bash
cargo test -p roko-orchestrator
cargo test -p roko-conductor
cargo test -p roko-cli orchestrate
cargo test -p roko-cli process_management
./target/debug/roko parity check --strict --area docs/01-orchestration --area docs/07-conductor
```

**Failure modes to cover:**

- Agent process exits without writing a result.
- Gate fails because a command is missing.
- Snapshot exists but event log has newer events.
- Merge queue sees two tasks editing the same file.
- Conductor abort arrives while a task is in review.

### Packet 7.4 -- agents, composition, verification, and learning parity

**Start state to verify:**

```bash
rg -n "ToolLoop|ToolDispatcher|SafetyLayer|Provider|Claude|Codex|Cursor|OpenAI|Ollama|Gemini|Perplexity|transcript|usage|retry" crates/roko-agent crates/roko-cli crates/roko-serve
rg -n "SystemPromptBuilder|prompt|VCG|attention|active inference|context|retrieval|token budget" crates/roko-compose crates/roko-runtime crates/roko-agent
rg -n "Gate|ReviewVerdict|PRM|eval|Episode|CascadeRouter|playbook|learning" crates/roko-gate crates/roko-learn crates/roko-agent crates/roko-orchestrator
```

**Concrete implementation steps:**

1. Create a provider matrix listing every model backend and whether it uses the
   common tool loop, dispatcher, safety layer, retry policy, transcript format,
   usage accounting, and cancellation path.
2. Refactor only the missing paths so all provider invocations pass through one
   execution envelope. The envelope must include temperament, domain profile,
   budget, gate rung, model-router decision, tool profile, capabilities,
   workspace policy, and trace id.
3. Make `SystemPromptBuilder` the production path for orchestration prompts.
   Legacy prompt builders can remain only as wrappers or test fixtures.
4. Wire VCG/attention/active-inference context selection into prompt assembly:
   choose candidate memories/symbols/docs, score them, respect token budget,
   record truncation decisions, and expose a prompt trace.
5. Publish every gate verdict as an event and as an episode signal. Consumers:
   `roko-learn`, `CascadeRouter`, conductor, knowledge admission, dashboard
   projections, WS/SSE, and plan status.
6. Convert review verdicts, compile-error classes, PRM scores, eval generation,
   route smoke failures, and provider failures into typed episode records.
7. Add mock-provider tests first, then parity tests for real adapters that can
   run in offline mode or with env-gated credentials.

**Test requirements:**

```bash
cargo test -p roko-agent provider_parity
cargo test -p roko-compose system_prompt_builder
cargo test -p roko-gate verdict_signal
cargo test -p roko-learn episode
cargo test -p roko-orchestrator agent_execution_envelope
./target/debug/roko parity check --strict --area docs/02-agents --area docs/03-composition --area docs/04-verification --area docs/05-learning
```

**Provider parity minimum:**

Every backend must prove: request construction, streaming or non-streaming
response handling, cancellation, tool-call handling, safety denial, usage/cost
capture, retry classification, transcript persistence, and episode emission.

### Packet 7.5 -- neuro, daimon, dreams, and heartbeat parity

**Start state to verify:**

```bash
rg -n "CorticalState|HeartbeatPolicy|FrequencyScheduler|gate_tier|Tick|gamma|theta|delta|DeltaConsumer|ThetaConsumer|probe|prediction error" crates/roko-runtime
rg -n "KnowledgeEntry|HDC|resonance|backup|restore|dream|NREM|REM|oneiro|PAD|behavioral|somatic|Daimon" crates/roko-neuro crates/roko-dreams crates/roko-daimon crates/roko-cli crates/roko-serve
rg -n "heartbeat|theta|delta|dream|daimon|PAD|neuro|knowledge lifecycle|backup|restore" docs/06-neuro docs/09-daimon docs/10-dreams docs/16-heartbeat
```

**Concrete implementation steps:**

1. Do not duplicate heartbeat primitives. Build a `TickPipeline` facade around
   existing `heartbeat.rs`, `heartbeat_attention.rs`, `heartbeat_probes.rs`,
   `theta_consumer.rs`, and `delta_consumer.rs`.
2. Implement the canonical 9-step tick sequence: collect probes, compute
   prediction error, update cortical state, select tier, allocate attention,
   choose action path, execute/reflex/infer, emit events, persist snapshot.
3. Run gamma, theta, and delta as cancellable async tasks sharing typed state
   through channels or locks with bounded backpressure. Each task needs health,
   last tick, lag, error, and shutdown metrics.
4. Feed tier decisions into `CascadeRouter`, risk posture, budget policy,
   conductor actions, and dashboard/TUI status.
5. Merge Daimon PAD/behavioral state into retrieval bias, tier thresholds,
   risk posture, and somatic marker lookup. Add tests for extreme PAD values
   and contagion isolation.
6. Replace `DeltaConsumer` NREM/REM/integration stubs with real calls into
   `roko-dreams`, `roko-learn`, and `roko-neuro`: replay episodes, generate
   counterfactuals, consolidate candidates, update confidence, promote/prune
   knowledge, and write dream reports.
7. Add CLI/API parity: `roko neuro backup`, `roko neuro restore`,
   `roko dream run`, `roko dream report`, dream status route, and ledger rows
   pointing to each command/route.

**Test requirements:**

```bash
cargo test -p roko-runtime tick_pipeline
cargo test -p roko-runtime theta_consumer delta_consumer
cargo test -p roko-neuro backup restore knowledge_lifecycle
cargo test -p roko-daimon behavioral_state
cargo test -p roko-dreams consolidation
./target/debug/roko parity check --strict --area docs/06-neuro --area docs/09-daimon --area docs/10-dreams --area docs/16-heartbeat
```

**End-to-end smoke:**

Run a persistent test agent with synthetic observations that force T0, T1, and
T2 tiers; trigger theta manually; trigger delta manually; confirm emitted
events, persisted snapshots, dream reports, and queryable knowledge entries.

### Packet 7.6 -- safety, tools, plugins, and code intelligence parity

**Start state to verify:**

```bash
rg -n "Capability|taint|audit|witness|allowlist|deny|path|network|bash|git|custody|spend|SafetyLayer" crates/roko-agent crates/roko-orchestrator crates/roko-gate crates/roko-core
rg -n "Tool|ToolDispatcher|MCP|plugin|manifest|recipe|connector|feed|extension|roko new" crates/roko-core crates/roko-agent crates/roko-plugin crates/roko-mcp-* crates/roko-cli
rg -n "tree-sitter|symbol|PageRank|HDC|SQLite|snapshot|incremental|dependency graph|usage refs" crates/roko-index crates/roko-mcp-code docs/15-code-intelligence
```

**Concrete implementation steps:**

1. Define one safety envelope for every tool call. Required checks:
   capability token, allowlist, denylist, path policy, network policy, bash
   policy, git policy, spend/custody policy, taint propagation, witness DAG,
   audit record, and redaction.
2. Find every direct tool/process/network/git execution path and route it
   through the safety envelope. Any bypass must be named
   `unsafe_allow_bypass_for_test_only` or equivalent and limited to tests.
3. Implement `roko new` generators for gate, scorer, connector, feed, recipe,
   extension, MCP server, agent template, and domain profile. Each generator
   must create compiling code, a manifest if applicable, a unit test, and a
   README or doc comment with usage.
4. Complete MCP GitHub, Slack, Scripts, Stdio, and Code servers to the level
   described by docs. Each server needs offline contract tests; real external
   calls must be env-gated.
5. Wire `roko-index` into context assembly and `roko-mcp-code`: symbols,
   references, dependency graph, PageRank, HDC fingerprints, SQLite persistence,
   snapshots, incremental updates, and stale-index detection.
6. Add plugin discovery/loading for safe local manifest tiers. WASM/native or
   network-capable tiers require explicit permissions and health reporting.
7. Surface tool/plugin/index health through CLI, `/api/parity`, `/api/status`,
   and dashboard/TUI where those surfaces exist.

**Test requirements:**

```bash
cargo test -p roko-agent safety
cargo test -p roko-core tool
cargo test -p roko-plugin
cargo test -p roko-index
cargo test -p roko-mcp-code
cargo test -p roko-cli generators
./target/debug/roko parity check --strict --area docs/11-safety --area docs/15-code-intelligence --area docs/18-tools
```

**Hard blockers:**

- A shell command cannot execute before safety approval.
- A git write cannot execute without audit/provenance.
- A plugin load failure cannot crash the host process.
- Code-index answers must cite file path, symbol, and index snapshot id.

### Packet 7.7 -- interfaces, lifecycle, and deployment parity

**Start state to verify:**

```bash
rg -n "derive\\(.*Parser|Subcommand|CommandFactory|openapi|utoipa|ApiDoc|route\\(" crates/roko-cli/src crates/roko-serve/src
rg -n "TUI|ratatui|tab|screen|StateHub|projection|freshness|backup|restore|delete|provision|deploy|daemon|launchd|systemd" crates/roko-cli crates/roko-serve deploy docker docs/12-interfaces docs/17-lifecycle docs/19-deployment
find deploy docker -maxdepth 3 -type f | sort
```

**Concrete implementation steps:**

1. Generate CLI reference from the actual `clap` command tree. The generator
   must include command, aliases, args, defaults, env vars, examples, and exit
   code notes. CI must fail if `docs/CLI-REFERENCE.md` drifts.
2. Generate OpenAPI or equivalent route reference from `roko-serve`. Every
   mounted route needs method, path, auth scope, request schema, response
   schema, error shape, and example. CI must fail if `docs/API-REFERENCE.md`
   drifts.
3. Add TUI surface inventory checks. Required screens: plans, plan detail,
   agent list/detail, knowledge, collective/groups, system status, jobs,
   streams/events, config, deployments, and parity.
4. Add StateHub freshness metadata to HTTP/projection responses that serve live
   state: source, sequence, generated_at, stale_after, last_event_id, and
   degradation reason.
5. Wire type-state provisioning into `roko init` and agent creation. Lifecycle
   transitions must be explicit and auditable: created, provisioned, running,
   paused, stopped, backed_up, restored, deleting, deleted, failed.
6. Implement backup, restore, selective restore, deletion, and knowledge
   transfer through CLI and HTTP, using dry-run previews for destructive flows.
7. Turn Docker, Railway, Fly, launchd, systemd, daemon IPC, secrets, telemetry,
   and production-hardening docs into executable validation scripts or tests.

**Test requirements:**

```bash
cargo test -p roko-cli cli_reference
cargo test -p roko-cli tui_surface_inventory
cargo test -p roko-serve api_reference
cargo test -p roko-serve statehub_freshness
cargo test -p roko-runtime lifecycle
./target/debug/roko parity check --strict --area docs/12-interfaces --area docs/17-lifecycle --area docs/19-deployment
```

**Do not finish until:**

- Generated docs are deterministic.
- Every mutating lifecycle command has `--dry-run` or an explicit confirmation
  bypass intended for non-interactive tests.
- Deployment validation prints concrete missing env vars and commands, not a
  generic failure.

### Packet 7.8 -- chain, coordination, identity economy, DeFi, and technical analysis parity

**Start state to verify:**

```bash
rg -n "TxSimGate|WalletGate|MEV|VenueAdapter|risk|passport|reputation|registry|bounty|x402|MPP|payment|feed|oracle|calibration|P&L|chain witness" crates apps contracts docs/08-chain docs/13-coordination docs/14-identity-economy docs/20-technical-analysis tmp/defi/gap
find contracts/src contracts/test apps/mirage-rs crates/roko-chain -maxdepth 4 -type f | sort
sed -n '1,220p' tmp/defi/gap/11-CHECKLIST-IMPLEMENTATION.md
```

**Concrete implementation steps:**

1. Create a topological DeFi execution ledger from
   `tmp/defi/gap/11-CHECKLIST-IMPLEMENTATION.md`. Each batch needs owner,
   dependencies, target files, fixture data, expected events, tests, and
   benchmark commands.
2. Complete `TxSimGate`, `WalletGate`, MEV gate, chain witness, wallet
   registry, and multi-chain subscription behavior before any agent can
   produce a chain-affecting action.
3. Implement coordination runtime surfaces: groups, pheromone scopes, mesh
   sync, permissioned subnets, collective state, c-factor metrics, dashboard
   projections, and WS/SSE rooms.
4. Expand identity/economy surfaces: passport, reputation, validation,
   knowledge registry, bounty lifecycle, fee routing, worker registry, identity
   events, and indexer catch-up.
5. Implement x402/MPP or the documented payment-session abstraction for paid
   feeds and agent services. Include denial before payment, settlement, access
   grant, stream delivery, and revenue/cost accounting.
6. Expose Oracle trait runtime surfaces for chain, coding, research, witness,
   HDC technical analysis, spectral manifolds, causal discovery,
   robust/adversarial signals, and predictive geometry. Every oracle needs
   prediction, scoring, calibration, storage, and API/dashboard visibility.
7. Run all DeFi benchmark scenarios in `tmp/defi/gap/13-BENCHMARKS.md` and
   store machine-readable results under `.roko/parity/gates/defi.json`.

**Test requirements:**

```bash
cargo test -p roko-chain
cargo test -p mirage-rs
cargo test -p roko-serve chain feeds connectors
cargo test -p roko-learn oracle
cd contracts && forge test
./target/debug/roko parity check --strict --area docs/08-chain --area docs/13-coordination --area docs/14-identity-economy --area docs/20-technical-analysis
```

**Safety requirements:**

- DeFi action tests must default to simulation/devnet only.
- A wallet-affecting path must prove `TxSimGate`, `WalletGate`, MEV/risk gate,
  witness, and audit all ran.
- Paid-feed authorization must be checked both at subscription creation and at
  stream delivery.

### Packet 7.9 -- documentation reconciliation and research-to-runtime ledger

**Start state to verify:**

```bash
rg -n "not implemented|not wired|stub|placeholder|future work|TODO|FIXME|implemented|Status|readiness|research-to-runtime|falsifier|metric" docs tmp/architecture
rg --files docs/21-references docs -g '*.md' | sort
```

**Concrete implementation steps:**

1. Add a docs reconciliation command or script behind `roko parity docs` or
   `roko parity inventory` that reads `docs-ledger.json` and updates generated
   status tables. Generated regions must be clearly marked so manual prose is
   not overwritten accidentally.
2. Replace stale "not implemented" or "not wired" notes only when the ledger
   row is `implemented_by` and the referenced code/test exists. Keep the exact
   file references and verification command in the updated prose.
3. Add `.roko/parity/research-to-runtime.jsonl` with one JSON object per
   research-backed architectural claim that affects runtime behavior. Required
   fields: `claim`, `source_doc`, `reference`, `runtime_use`, `metric`,
   `falsifier`, `owner`, `status`, and `last_checked`.
4. For `docs/21-references`, classify each doc as `reference_only`,
   `used_by_runtime`, or `superseded`. If used by runtime, add a falsifier or
   measurement path.
5. Add link/code-reference checking to the parity gate: local markdown links,
   referenced source files, route references, command examples, and generated
   docs anchors.
6. Update `docs/STATUS.md`, `docs/API-REFERENCE.md`, and
   `docs/CLI-REFERENCE.md` from generated data where possible. If a section is
   not generated, add a ledger note explaining why.

**Test requirements:**

```bash
./target/debug/roko parity docs --check
./target/debug/roko parity check --strict --include-links
cargo test -p roko-cli docs_reconciliation
```

**Completion rule:**

No source doc may contain a stale implementation-status claim. If the code is
done, the doc must cite the code and gate. If the code is not done, the ledger
must identify the owning plan, owner, dependency, and future gate.

## Dependencies

Plan 07 can run in parallel with Plan 06 after the audit addendum is understood.
Do not wait for all Plan 06 phases to complete before creating the ledger. The
ledger is a coordination artifact and should exist first.

## Definition of done

Plan 07 is done when:
- Every source doc has a ledger row
- Every ledger row has an owner and acceptance gate
- Every "built but not wired" item has either code wiring or a deferred gate
- `roko parity check --strict` passes
- Plan 08's end-to-end acceptance harness can run against the resulting build
