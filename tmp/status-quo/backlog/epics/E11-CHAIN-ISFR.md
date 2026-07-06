# E11 — Chain / ISFR

> **Epic header**
> - Epic ID: `E11`
> - Repo HEAD: `5852c93c05a4f1bda8ff880fc752d9fba2ba453e` (branch `main`)
> - Source docs: `42-CHAIN-REGISTRIES-ISFR`, `106-APPS-MIRAGE-RELAY-WATCHER`
> - Prior plan coverage: **PARTIAL & BROKEN** — `plans/architecture-defi-critical-path/` (D01–D03) exists and targets chain registries/indexer, but its parity ledger points at `plans/architecture-core-queue/tasks.toml#Q14-…` which is **NOT committed to `plans/`** (lives only in `.claude/worktrees/*` and `.roko/worktrees/*`). The DeFi critical path is therefore unsatisfiable as committed. No plan touches get_logs, deploy-path parity, the dead `Engram`, or the 16 zero-caller modules.
> - Blast radius: **LOW-to-MEDIUM at runtime** — the entire chain surface is opt-in (`[chain]`/`[isfr]` absent in stock config ⇒ no chain client, no keeper, no deploys). Most of this epic is Phase-2+/optional. The **critical-path subset** is small: recover the core queue, implement `get_logs`, and reach deploy parity for the registries the DeFi path indexes.
> - Fence (read first): chain maturity is split across **two repos**. `roko-chain` = the **client half** (signs txs, reads state, deploys via alloy). Canonical consensus + precompiles live in a **separate daeji devnet** (`tmp/agentchain-v2/02-daeji/`, design-only in this repo; not runnable here). This epic scopes **only** roko's client side. "Not implemented" for BFT / precompiles / verified-state = "not in roko, owned by daeji" — do NOT build them here.

## Problem statement

`roko-chain` is ~23K LOC across 40 `src/*.rs` files backed by 13 authored Solidity contracts,
but only the **ISFR vertical** (sources → keeper → composite → serve routes → per-epoch
`submitRate()`) is wired into the runtime. Three distinct maturity strata coexist:

1. **Live ISFR vertical** — the one near-end-to-end chain feature (already ✅, not this epic's target beyond the deploy path it shares).
2. **16 modules with real, tested Rust logic and ZERO runtime callers** — witness anchoring, x402 channels, KORAI token, Spore marketplace, the ERC-8004 registry trio (agent/reputation/validation), `IsfrRegistry` clearing, TraceRank, collusion, Nelson-Siegel, futures market, three chain gates, heartbeat_ext.
3. **Self-declared stubs** — `phase2.rs`, `identity_economy_{identity,markets}.rs` (5,894 LOC "intentionally no runtime logic", but **load-bearing** — real modules import their placeholder types).

Four concrete defects gate the DeFi critical path and pollute the tree:

- `AlloyChainClient::get_logs` returns `Err(ChainError::Unsupported("get_logs"))` (`alloy_impl.rs:147-157`) — the only real client cannot `eth_getLogs`, blocking any event indexer / log-hydrated registry (exactly what `architecture-defi-critical-path` D01 promises to build).
- **Three disjoint deploy paths** (`Deploy.s.sol` = 6 contracts, `isfr_bootstrap.rs` = 5, `roko-demo` alloy = manifest); **no path deploys all 13**. The ERC-8004 trio + `FeeDistributor` are authored + forge-tested but deployed by **none**.
- A dead second `struct Engram` (+ shadow `Provenance`/`CustodyEntry`) at `identity_economy_markets.rs:653/678/691` shadows `roko_core::Engram`; pollutes symbol search (overlaps **E03-T07**).
- `architecture-core-queue` (Q01–Q24, the chain **foundation** queue, `Q14` = chain-registries-defi-foundation) exists only in worktrees — the committed DeFi plan references it and cannot resolve.

## Module status table (`roko-chain/src/*.rs`)

| Module | Real logic? | Runtime caller (file) | Verdict |
|---|---|---|---|
| `alloy_impl.rs` | ✅ real RPC client+wallet | serve `state.rs`, `isfr_bootstrap.rs:210` | ✅ wired — **`get_logs` Unsupported (`:147-157`)** |
| `isfr_keeper.rs` / `isfr_sources/` / `isfr_oracle_submit.rs` / `isfr_bootstrap.rs` | ✅ | serve `lib.rs`, CLI `commands/isfr.rs` | ✅ **wired (flagship)** |
| `block_watcher.rs` / `observer.rs` / `triage.rs` | ✅ | serve `lib.rs`, `job_runner.rs` (mock-backed) | ✅ / 🟡 |
| `mock.rs` / `types.rs` / `chain_profile.rs` / `tools.rs` / `client.rs` / `wallet.rs` | ✅ | serve / std resolver | ✅ plumbing |
| `witness.rs` | ✅ marker-tx anchoring + verify | **none** | 🔌 shelf-ware |
| `x402.rs` | ✅ 402 flow, ERC-3009, channels | **none** | 🔌 shelf-ware |
| `korai_token.rs` | ✅ demurrage token | **none** (no KORAI.sol) | 🔌 shelf-ware |
| `marketplace.rs` | ✅ Spore FSM, Vickrey/Sparrow/Direct | **none** (jobs use `.roko/jobs/*.json`) | 🔌 shelf-ware |
| `agent_registry.rs` / `reputation_registry.rs` / `validation_registry.rs` | ✅ ERC-8004 Rust twins | **none** (serve uses `sol!` bindings, not these) | 🔌 shelf-ware |
| `isfr.rs` (`IsfrRegistry`) | ✅ 6-phase commit-reveal clearing | **none** (keeper does NOT run it) | 🔌 shelf-ware |
| `trace_rank.rs` / `collusion.rs` / `nelson_siegel.rs` / `futures_market.rs` | ✅ tested primitives | **none** (roko-demo scenario ≠ runtime) | 🔌 shelf-ware |
| `gate/{mev,tx_sim,wallet}_gate.rs` | ✅ 2,029 LOC | **none** (not in 7-rung pipeline) | 🔌 shelf-ware |
| `heartbeat_ext.rs` | ✅ policy-cage | **none** | 🔌 shelf-ware |
| `phase2.rs` | 🕰️ placeholder types | in-crate only (load-bearing) | 🕰️ stub |
| `identity_economy_identity.rs` | 🕰️ stub | none | 🕰️ stub |
| `identity_economy_markets.rs` | 🕰️ stub + **dead `Engram`/`Provenance`/`CustodyEntry`** | none | 🕰️ stub + dead-code |

**Tally: 16 real-logic modules with zero runtime callers** + 3 load-bearing stub modules.
The wired surface is exactly the ISFR vertical + alloy/mock/block_watcher/observer/tools plumbing.

## Findings table

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| F1 | `get_logs` → `Unsupported` in the only real client | `alloy_impl.rs:147-157` (confirmed) | E11-T02 |
| F2 | 13 contracts, 3 disjoint deployers, none deploy all 13; ERC-8004 trio + `FeeDistributor` deployed by none | `contracts/src/*.sol`=13; `Deploy.s.sol` imports 6 (MockERC20, AgentRegistry, WorkerRegistry, BountyMarket, ConsortiumValidator, InsightBoard) | E11-T03 |
| F3 | Dead 2nd `Engram` + shadow `Provenance`/`CustodyEntry` | `identity_economy_markets.rs:653/678/691` (confirmed) | E11-T04 (⇄ E03-T07) |
| F4 | `architecture-core-queue` (Q14 = DeFi chain foundation) missing from committed `plans/`; DeFi plan's parity ledger references it | `plans/architecture-core-queue/` absent; 5 identical worktree copies (`md5 d05464…`, 24 tasks); `architecture-defi-critical-path/tasks.toml:69,129,187` `source_ref = plans/architecture-core-queue/tasks.toml#Q14-…` | E11-T01 |
| F5 | 16 zero-caller modules: real logic, tests, no runtime consumer | grep `roko_chain::<mod>` outside crate = ∅ for witness/x402/korai/marketplace/registries/isfr/trace_rank/collusion/nelson_siegel/futures/gates/heartbeat_ext | E11-T05 (decide: wire vs shelve) |
| F6 | daeji = separate repo (design-only here); precompiles/BFT/verified-state not roko's to build | `tmp/agentchain-v2/02-daeji/` docs; `chain_profile.rs:50-73` (`wss://rpc.daeji.dev/ws`) | E11-T05 decision doc |

## Recover architecture-core-queue (F4) — the unblocking action

`architecture-defi-critical-path` (the committed DeFi plan) declares parity rows whose
`source_ref` is `plans/architecture-core-queue/tasks.toml#Q14-chain-registries-defi-foundation`.
That plan file **does not exist under `plans/`** — it survives only as **5 byte-identical copies**
in `.claude/worktrees/agent-*/` (`md5 d05464260a54ef6d1e5618b55c5c3eb4`, 24 tasks Q01–Q24,
`max_parallel = 2`) plus one under `.roko/worktrees/`. Until it is committed to `plans/`, the
DeFi critical path cannot be validated or resolved against its parent.

**Recovery** (E11-T01): copy one canonical worktree copy into
`plans/architecture-core-queue/tasks.toml`, `roko plan validate` it, and confirm the
`source_ref` anchors in `architecture-defi-critical-path` now resolve to a real Q14 task.
This is the single prerequisite that turns the DeFi plan from unsatisfiable → runnable.
(All 5 copies are identical, so there is no reconciliation ambiguity — pick any.)

## Critical-path vs deferrable

**Critical path (needed for the DeFi vertical to be runnable):**
- **E11-T01** recover `architecture-core-queue` — without it the committed DeFi plan is orphaned.
- **E11-T02** implement `get_logs` — the load-bearing gap under the D01 "event indexer foundation."
- **E11-T03** deploy-path parity for the ERC-8004 trio + `FeeDistributor` — D02 exposes registry/passport routes that need those contracts deployed by a single canonical path.

**Deferrable (Phase-2+/optional, chain surface is opt-in):**
- **E11-T04** delete dead `Engram` — pure cleanup; coordinate with **E03-T07** (do NOT double-delete).
- **E11-T05** wire-or-shelve the 16 zero-caller modules — a **decision doc**, not code; default recommendation is SHELVE (stamp Phase-2 in CLAUDE.md + GAPS.md) since none are on the DeFi path and several (witness/x402/registries) await daeji-side primitives.
- Full 13-contract deploy (beyond the trio + FeeDistributor), reputation-informed routing, x402 middleware, marketplace runtime, IsfrRegistry on-chain clearing — all explicitly deferred.

## Ordering

1. `E11-T01` Recover `architecture-core-queue` into `plans/` — **mechanical** (critical)
2. `E11-T02` Implement `AlloyChainClient::get_logs` — **standard** (critical)
3. `E11-T03` Deploy-path parity: one canonical deployer covers the ERC-8004 trio + FeeDistributor — **standard** (critical)
4. `E11-T04` Delete dead `Engram`/`Provenance`/`CustodyEntry` (⇄ E03-T07) — **mechanical** (deferrable)
5. `E11-T05` Decision doc: wire-or-shelve the 16 zero-caller modules + fence daeji — **standard** (deferrable)

## Tasks

### E11-T01 — Recover architecture-core-queue into committed plans/
- **tier**: mechanical
- **files**: `plans/architecture-core-queue/tasks.toml` (create)
- **depends_on**: none
- **acceptance**: `plans/architecture-core-queue/tasks.toml` exists, byte-identical to the canonical worktree copy (`.claude/worktrees/agent-aefd7c48/plans/architecture-core-queue/tasks.toml`, `md5 d05464260a54ef6d1e5618b55c5c3eb4`, 24 tasks Q01–Q24). `roko plan validate plans/architecture-core-queue/` passes. The `source_ref` anchors in `architecture-defi-critical-path` (`#Q14-chain-registries-defi-foundation`) now resolve to a task that exists.
- **verify**: `test -f plans/architecture-core-queue/tasks.toml` · `grep -q 'Q14-chain-registries-defi-foundation' plans/architecture-core-queue/tasks.toml` · `cargo run -p roko-cli -- plan validate plans/architecture-core-queue`

### E11-T02 — Implement AlloyChainClient::get_logs (unblocks event indexers)
- **tier**: standard
- **files**: `crates/roko-chain/src/alloy_impl.rs`
- **depends_on**: none
- **acceptance**: `get_logs` builds an alloy `Filter` from `(from, to, addresses, topics)` and calls `provider.get_logs(&filter)`, mapping results into `Vec<LogEntry>` (mirror `mock.rs`'s LogEntry shape). Returns real logs — never `ChainError::Unsupported`. Non-alloy build path unaffected. A unit/integration test asserts a filtered query returns logs (against mirage/anvil or a mocked provider).
- **verify**: `! grep -A6 'async fn get_logs' crates/roko-chain/src/alloy_impl.rs | grep -q 'Unsupported'` (get_logs no longer returns Unsupported) · `cargo test -p roko-chain --features alloy-backend get_logs 2>&1 | tail -5`

### E11-T03 — Deploy-path parity: deploy the ERC-8004 trio + FeeDistributor
- **tier**: standard
- **files**: `contracts/script/Deploy.s.sol`
- **depends_on**: none
- **acceptance**: `Deploy.s.sol` (the pure-forge canonical path) is extended to also deploy `IdentityRegistry`, `ReputationRegistry`, `ValidationRegistry`, and `FeeDistributor` (currently deployed by NO path), plus `RoleRegistry`/`ISFROracle`/`ISFRBountyPool` so one script emits all **13** contract addresses with correct constructor wiring. `forge test` stays green. (isfr_bootstrap and roko-demo deployers documented as the runtime/demo paths that consume a subset; Deploy.s.sol becomes the reference full-suite path.)
- **verify**: `[ "$(grep -c 'new [A-Z]' contracts/script/Deploy.s.sol)" -ge 13 ]` (script instantiates ≥13 contracts) · `grep -q 'IdentityRegistry\|ReputationRegistry\|ValidationRegistry\|FeeDistributor' contracts/script/Deploy.s.sol` · `cd contracts && forge build && forge test 2>&1 | tail -5`

### E11-T04 — Delete dead Engram/Provenance/CustodyEntry stubs (coordinate with E03-T07)
- **tier**: mechanical
- **files**: `crates/roko-chain/src/identity_economy_markets.rs`
- **depends_on**: none (but **mutually exclusive with E03-T07** — whichever runs first satisfies both; the other becomes a no-op)
- **acceptance**: the never-wired forensic-replay `struct Engram` (`:653`) and its local shadow `struct Provenance` (`:678`) / `struct CustodyEntry` (`:691`) are removed (or, if any sibling stub in the file still references them, annotated `// throwaway stub — shadows roko_core::{Engram,Provenance}` and left `#[allow(dead_code)]`). `roko_core::engram::Engram` is the only bare `struct Engram` in the tree. `roko-chain` compiles.
- **verify**: `[ "$(rg -c 'pub struct Engram \{' crates/ | wc -l)" -eq 1 ]` · `cargo check -p roko-chain 2>&1 | tail -5`
- **coordination note**: E03-T07 (`E03-TYPE-CONSOLIDATION.md`) deletes the same stub. Do NOT schedule both in the same wave. If E03-T07 lands first, close E11-T04 as done-by-E03.

### E11-T05 — Wire-or-shelve decision doc for the 16 zero-caller modules
- **tier**: standard
- **files**: `.roko/GAPS.md`, `CLAUDE.md` (roko-chain row + status table), `docs/v2/22-REGISTRIES.md` (or a new `docs/decisions/chain-phase2.md`)
- **depends_on**: none
- **acceptance**: a single decision doc records, per module (witness, x402, korai_token, marketplace, agent_registry, reputation_registry, validation_registry, isfr.rs, trace_rank, collusion, nelson_siegel, futures_market, gate/{mev,tx_sim,wallet}, heartbeat_ext), an explicit verdict: **WIRE** (name the runtime call site + a follow-up task id) or **SHELVE** (stamp Phase-2, note the blocking dependency — e.g. daeji precompiles for HDC/verified-state, KORAI.sol for korai_token, a 402 middleware for x402). CLAUDE.md's stale "roko-chain = Phase 2+ primitives" row is corrected to reflect the ISFR-wired reality + the shelf-ware tally. The daeji client/node fence is stated once in CLAUDE.md (roko = client, daeji = node/precompiles, separate repo).
- **verify**: `grep -qi 'daeji' CLAUDE.md` (fence documented) · `grep -qiE 'shelve|phase.?2|wire' .roko/GAPS.md` · `! grep -q 'Chain witness primitives · Phase 2+' CLAUDE.md` (stale row corrected)
- **recommended default**: SHELVE all 16. None sit on the DeFi critical path; witness/x402/verified-state await daeji-side primitives; wiring any one is its own follow-up epic, not blocking self-hosting.

## First 3 tasks — executable TOML

```toml
[meta]
plan = "E11-chain-isfr"
total = 5
done = 0
status = "ready"
max_parallel = 1

# ── E11-T01: Recover architecture-core-queue into committed plans/ ──
#
# The committed DeFi plan (plans/architecture-defi-critical-path/tasks.toml)
# declares parity rows whose source_ref is
#   plans/architecture-core-queue/tasks.toml#Q14-chain-registries-defi-foundation
# but plans/architecture-core-queue/ DOES NOT EXIST. It survives only as five
# byte-identical copies under .claude/worktrees/agent-*/ (md5
# d05464260a54ef6d1e5618b55c5c3eb4, 24 tasks Q01-Q24). Copy one canonical copy
# into plans/ so the DeFi critical path can resolve its parent + validate.
# Pure recovery — no content changes; all 5 worktree copies are identical.

[[task]]
id = "E11-T01"
title = "Recover architecture-core-queue tasks.toml into committed plans/"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = ["plans/architecture-core-queue/tasks.toml"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = ".claude/worktrees/agent-aefd7c48/plans/architecture-core-queue/tasks.toml", lines = "1-30", why = "Canonical source copy (md5 d05464…, 24 tasks) — copy verbatim into plans/" },
    { path = "plans/architecture-defi-critical-path/tasks.toml", lines = "64-71", why = "The dangling source_ref #Q14-chain-registries-defi-foundation this recovery satisfies" },
]
symbols = [
    "Q14-chain-registries-defi-foundation — the DeFi-foundation task the committed plan references",
]
anti_patterns = [
    "Do NOT edit the task content — copy verbatim; all 5 worktree copies are byte-identical (md5 d05464260a54ef6d1e5618b55c5c3eb4).",
    "Do NOT pull from .roko/worktrees/ if it differs — prefer a .claude/worktrees/agent-* copy and verify md5 before committing.",
    "Do NOT invent a new Q14 — recover the existing 24-task queue so downstream parity anchors resolve.",
]

[[task.verify]]
phase = "structural"
command = "test -f plans/architecture-core-queue/tasks.toml"
fail_msg = "architecture-core-queue must be recovered into committed plans/"

[[task.verify]]
phase = "structural"
command = "grep -q 'Q14-chain-registries-defi-foundation' plans/architecture-core-queue/tasks.toml"
fail_msg = "recovered plan must contain the Q14 chain-registries-defi-foundation task the DeFi plan references"

[[task.verify]]
phase = "custom"
command = "cargo run -p roko-cli -- plan validate plans/architecture-core-queue 2>&1 | tail -5"
fail_msg = "recovered architecture-core-queue must pass plan validate"


# ── E11-T02: Implement AlloyChainClient::get_logs ──
#
# alloy_impl.rs:147-157 returns Err(ChainError::Unsupported("get_logs")) — the
# only real chain client cannot eth_getLogs. This is the single load-bearing
# gap blocking any event indexer / log-hydrated registry, i.e. exactly what
# architecture-defi-critical-path D01 ("event indexer foundation") promises.
# mock.rs already returns LogEntry values, so the target shape is defined.
# Build an alloy Filter from (from,to,addresses,topics), call
# provider.get_logs(&filter), map into Vec<LogEntry>. Never return Unsupported.

[[task]]
id = "E11-T02"
title = "Implement AlloyChainClient::get_logs via eth_getLogs (unblocks event indexers)"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 80
files = ["crates/roko-chain/src/alloy_impl.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-chain/src/alloy_impl.rs", lines = "140-175", why = "The stub get_logs returning Unsupported + surrounding provider-call conventions (get_receipt maps logs already)" },
    { path = "crates/roko-chain/src/mock.rs", why = "Reference LogEntry shape the mock returns — match it" },
    { path = "crates/roko-chain/src/types.rs", why = "LogEntry / BlockNumber / ChainError definitions" },
    { path = "crates/roko-chain/src/client.rs", why = "ChainClient::get_logs trait signature to satisfy" },
]
symbols = [
    "AlloyChainClient::get_logs — alloy_impl.rs:147 (currently Unsupported)",
    "LogEntry — roko-chain/src/types.rs",
    "ChainError::Unsupported — remove this return path for get_logs",
]
anti_patterns = [
    "Do NOT leave the Unsupported fallback — a real provider.get_logs call must replace it.",
    "Do NOT change the ChainClient::get_logs trait signature — implement the existing one.",
    "Do NOT panic/unwrap on empty topic or address slices — empty means unfiltered (build a Filter without those constraints).",
    "Do NOT make the non-alloy build fail — this method lives behind the alloy-backend impl only.",
    "Parse hex address/topic strings the same way the rest of alloy_impl does (from_hex-style helpers), not ad-hoc.",
]

[[task.verify]]
phase = "structural"
command = "! (grep -A8 'async fn get_logs' crates/roko-chain/src/alloy_impl.rs | grep -q 'Unsupported(\"get_logs\")')"
fail_msg = "get_logs must no longer return ChainError::Unsupported"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-chain --features alloy-backend 2>&1 | tail -10"
fail_msg = "roko-chain must compile with alloy-backend after get_logs implementation"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-chain --features alloy-backend get_logs 2>&1 | tail -10"
fail_msg = "a get_logs test must build and pass (real logs, not Unsupported)"


# ── E11-T03: Deploy-path parity — deploy all 13 contracts ──
#
# 13 authored contracts, 3 disjoint deployers, none deploys all 13.
# Deploy.s.sol currently instantiates 6 (MockERC20, AgentRegistry,
# WorkerRegistry, BountyMarket, ConsortiumValidator, InsightBoard). The
# ERC-8004 trio (IdentityRegistry, ReputationRegistry, ValidationRegistry) and
# FeeDistributor are authored + forge-tested but deployed by NO path — they
# exist only as compiled artifacts. Make Deploy.s.sol the canonical full-suite
# reference path: deploy all 13 with correct constructor wiring. isfr_bootstrap
# (serve runtime) + roko-demo (alloy) stay as subset paths, documented as such.

[[task]]
id = "E11-T03"
title = "Extend Deploy.s.sol to deploy all 13 contracts (ERC-8004 trio + FeeDistributor + ISFR set)"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 120
files = ["contracts/script/Deploy.s.sol"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "contracts/script/Deploy.s.sol", lines = "1-70", why = "Current 6-contract deployer + Deployed struct + post-deploy wiring to extend" },
    { path = "crates/roko-chain/src/isfr_bootstrap.rs", lines = "13-52", why = "Reference deploy order + role grants for RoleRegistry/ISFROracle/ISFRBountyPool (the ISFR slice)" },
    { path = "contracts/src/IdentityRegistry.sol", why = "ERC-8004 IdentityRegistry constructor args (deployed by no path today)" },
    { path = "contracts/src/ReputationRegistry.sol", why = "ReputationRegistry constructor + dependency on IdentityRegistry" },
    { path = "contracts/src/ValidationRegistry.sol", why = "ValidationRegistry constructor" },
    { path = "contracts/src/FeeDistributor.sol", why = "FeeDistributor constructor args" },
]
symbols = [
    "Deploy.run() — contracts/script/Deploy.s.sol (6 contracts today; extend to 13)",
    "IdentityRegistry / ReputationRegistry / ValidationRegistry — ERC-8004 trio, deployed by NO path",
    "FeeDistributor — authored + tested, deployed by NO path",
]
anti_patterns = [
    "Do NOT break the existing 6-contract wiring (setAuthorized / setResolver) — extend, don't rewrite.",
    "Do NOT deploy KORAI.sol — it does not exist; deploys use MockERC20(\"DAEJI\").",
    "Do NOT wire constructor deps in the wrong order — ReputationRegistry/ValidationRegistry may need IdentityRegistry's address first; read each constructor.",
    "Do NOT change the runtime deploy path (isfr_bootstrap.rs) — Deploy.s.sol is the pure-forge reference; keep the two consistent, not merged.",
    "Emit every deployed address via console2 so `forge script` prints all 13.",
]

[[task.verify]]
phase = "structural"
command = "[ \"$(grep -c 'new [A-Z][A-Za-z0-9]*' contracts/script/Deploy.s.sol)\" -ge 13 ]"
fail_msg = "Deploy.s.sol must instantiate at least 13 contracts"

[[task.verify]]
phase = "structural"
command = "grep -q 'IdentityRegistry' contracts/script/Deploy.s.sol && grep -q 'ReputationRegistry' contracts/script/Deploy.s.sol && grep -q 'ValidationRegistry' contracts/script/Deploy.s.sol && grep -q 'FeeDistributor' contracts/script/Deploy.s.sol"
fail_msg = "the ERC-8004 trio + FeeDistributor (deployed by no path today) must appear in Deploy.s.sol"

[[task.verify]]
phase = "custom"
command = "cd contracts && forge build 2>&1 | tail -5 && forge test 2>&1 | tail -5"
fail_msg = "contracts must build and forge tests must stay green after extending the deployer"
```

## Downstream unblocks

- **`plans/architecture-defi-critical-path`** blocks on **E11-T01** — its D01/D02/D03 parity rows reference `Q14` in a plan that isn't committed; recovery makes the DeFi path resolvable.
- **DeFi D01 "event indexer foundation"** blocks on **E11-T02** — a real indexer needs `eth_getLogs`; without it the indexer can only decode full blocks (no backfill, no topic filter).
- **DeFi D02 "registry/passport routes"** benefits from **E11-T03** — the ERC-8004 registries those routes query must be deployable by one canonical path.

## Follow-ups (out of E11 scope, logged)

- Wire `ChainWitnessEngine` into the attestation/episode flow (config-gated) — a real E11-T05 "WIRE" outcome if chosen.
- Reputation-informed routing: feed `ReputationRegistry`/TraceRank into CascadeRouter (CLAUDE.md open item 13).
- x402 402-payment middleware on agent-server `/message` (18-PAYMENTS) — or explicit Phase-2 stamp.
- Close the keeper relay loop (`relay_url` honored so `isfr:rates` reaches agent-relay subscribers).
- Migrate `phase2::{Address,u256}` in live modules (isfr/marketplace/x402) toward `alloy_primitives`.
- Decide fate of `tmp/light-clients/` (22 WUs, 0 code) — likely SUPERSEDED-BY-DAEJI (verified state = daeji QMDB proofs + BLS).
- Fix the TUI "ISFR" (Inter-Signal Frequency Ratio) label collision with the fact-registry ISFR.
