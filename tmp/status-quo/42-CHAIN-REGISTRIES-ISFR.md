# roko-chain — Witness, Registries, ISFR, Payments

> Status-quo audit · verified 2026-07-07 · re-verified 2026-07-08 (wave-2: 40 src/*.rs re-counted, Engram collision traced, contract count corrected, deploy-path + get_logs + witness/x402 consumer-grep re-run) · **deep second pass 2026-07-08 (HEAD 5852c93c05): per-module external-caller grep, contracts×deploy matrix, client-vs-daeji fence — see three new sections below** · supersedes concise draft · sources: ~30 files read directly, 4 sub-audits (serve surface, contracts/apps, CLI/std/core/agent-server, docs inventory), 5 sibling status-quo docs (18, 20, 21, 44, 58)

**Deep-pass verification (HEAD 5852c93c05, 2026-07-08):** `contracts/src/*.sol` = **13** (confirmed), `contracts/test/*.t.sol` = **10** (not 11 — earlier over-count; the 13 contracts minus tests are `ISFROracle`, `ISFRBountyPool`, `RoleRegistry` which have no `.t.sol`). External-caller grep (`grep roko_chain:: crates/ --exclude roko-chain/src`) resolves to **only** `isfr_keeper`, `isfr_sources`, `isfr_oracle_submit`, `isfr_bootstrap`, `alloy_impl` (client+wallet), `block_watcher::ChainState`, `observer::ObservedEvent`, `types::LogEntry`, `tools::CHAIN_TOOL_NAMES`, `chain_profile::ContractAddresses`. **Every other roko-chain module has 0 external importers** (witness, x402, korai_token, marketplace, agent/reputation/validation registries as Rust, trace_rank, collusion, nelson_siegel, futures_market, gate/*, isfr.rs `IsfrRegistry`, identity_economy_*, phase2, heartbeat_ext). The "143 Marketplace / 12 AgentRegistry / 1 Reputation" grep hits are `roko_core::MarketplaceJob` and serve-side `sol!` bindings — **not** roko-chain Rust structs.

Legend: ✅ wired & runnable · 🔌 built-not-wired (real logic, no runtime caller) · 🟡 partial · ❌ missing · 🕰️ old paradigm/superseded

## Summary

CLAUDE.md's "roko-chain = Phase 2+ primitives" row is badly stale. The crate is **~23K LOC across 40 `src/*.rs` files** (`crates/roko-chain/src/`), backed by **13 authored Solidity contracts with forge tests** (`contracts/src/`), a **standalone chain-watcher binary** (`apps/roko-chain-watcher/`), a **vendored EVM fork simulator** (`apps/mirage-rs/`), a **real WS relay** (`apps/agent-relay/`), and a **demo orchestrator** (`crates/roko-demo/`) that deploys and exercises the contracts.

Three distinct maturity strata coexist inside one crate:

1. **A live ISFR vertical** — the one near-end-to-end chain feature: pluggable rate sources (4 real DeFi protocols + mock) → keeper poll loop → weighted-median composite → serve state + `/api/isfr/*` routes → per-epoch on-chain `submitRate()` to a deployed ISFROracle → `roko isfr` CLI. All config-gated, mock-by-default, degrades gracefully.
2. **Real-logic in-memory primitives with tests but no runtime callers** — ERC-8004-style registries (agent/reputation/validation), KORAI token w/ demurrage, Spore marketplace w/ Vickrey hiring, x402 payment channels, ChainWitnessEngine, TraceRank, collusion detection, MEV/tx-sim/wallet gates, futures market, Nelson-Siegel curves.
3. **Self-declared stubs** — `phase2.rs` (2,312 LOC of placeholder types "derived from docs/08-chain", `phase2.rs:18`) and `identity_economy_{identity,markets}.rs` (3,582 LOC that "intentionally avoid real runtime logic", `identity_economy_identity.rs:8-12`; `identity_economy_markets.rs` opens with `#![allow(dead_code)]` at `:2` and self-labels "Phase 2+ … stubs derived from docs/14-identity-economy" at `:8-9`).

**Crate size (re-counted wave-2):** 40 `src/*.rs` files. Solidity: **13 authored contracts** in `contracts/src/` (`AgentRegistry, WorkerRegistry, RoleRegistry, IdentityRegistry, ReputationRegistry, ValidationRegistry, ISFROracle, ISFRBountyPool, BountyMarket, ConsortiumValidator, InsightBoard, FeeDistributor, MockERC20` — the earlier "14" was an over-count) + **10 forge test files** in `contracts/test/` (deep-pass recount; `RoleRegistry`/`ISFROracle`/`ISFRBountyPool` have no `.t.sol`).

The v2 vision (docs/v2/22-REGISTRIES.md, 24-DEFI.md, 18-PAYMENTS.md) is roughly: on-chain identity/reputation consulted at dispatch time, a chain-backed job market, x402/MPP payments, witness-anchored attestations, verified light clients. **Distance from vision, bluntly:** ISFR delivers the oracle slice of 24-DEFI; everything else is either a primitive waiting for a caller (witness, x402, registries-as-Rust), a demo-only artifact (contracts + roko-demo), or pure paper (light clients/MPP — 22 designed work units, zero code). Nothing in the agent runtime (dispatch, gates, hiring, payments) consults any chain state today. Jobs are two disconnected systems: local `.roko/jobs/*.json` (what actually runs) and the chain BountyMarket (demo only).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| `ChainClient`/`ChainWallet` traits | docs/v1/08-chain wallet traits | `client.rs` (71L), `wallet.rs` (36L) | ✅ stable | `lib.rs:37-43` |
| Alloy backend (HTTP JSON-RPC) | 08-chain | `alloy_impl.rs` (344L): client + wallet, `from_hex_key`, `sign_and_submit`, `wait_for_receipt` | ✅ real, one gap | `get_logs` → `ChainError::Unsupported` (`alloy_impl.rs:147-157`) |
| Mock backend | — | `mock.rs` (738L), `paired_mocks` | ✅ | `lib.rs:109` |
| Feature gating | — | default features **empty**; `alloy-backend` optional | ✅ by design | `roko-chain/Cargo.toml:15-19`; enabled by roko-cli (`Cargo.toml:49`), roko-serve (`:28`), roko-demo (`:18`); roko-std/agent-server take mock-only |
| ISFR keeper | tmp/isfr/ 6-doc plan; docs/v2/24-DEFI | `isfr_keeper.rs` (766L): poll loop, epochs, publish callback, TCP reachability probe + `ETH_RPC_URL` fallback, offline degradation | ✅ wired | spawn: serve `lib.rs:387,2203-2436`; CLI `commands/isfr.rs:14` |
| ISFR sources (5) | tmp/isfr/ | `isfr_sources/`: aave_v3, compound_v3, ethena, lido (`eth_staking`) behind `alloy-backend`; mock always | ✅ wired | `isfr_sources/mod.rs:6-14`; builder `isfr_keeper.rs:293-379` |
| Composite math | 14-identity-economy/13-isfr-clearing | `isfr_sources/mod.rs:178-268`: weighted median per class + confidence | ✅ w/ tests | tests `mod.rs:270-327` |
| On-chain rate submission | 24-DEFI oracle | `isfr_oracle_submit.rs` (123L): real `submitRate()` via alloy `sol!`; non-alloy build = warn-only no-op | ✅ wired (per-epoch) | `isfr_oracle_submit.rs:59-110`; epoch gate serve `lib.rs:2289-2292,2374-2384`; no-op `:25-37` |
| ISFR contract bootstrap | tmp/isfr/ | `isfr_bootstrap.rs` (471L): deploys RoleRegistry→MockERC20→WorkerRegistry→ISFROracle→ISFRBountyPool from forge `out/` artifacts, grants roles, funds pool | ✅ wired | order `isfr_bootstrap.rs:13-26`; called at serve startup when `auto_deploy_contracts` + `contracts/out` exists (`roko-serve/src/lib.rs:348-385`, default anvil key `:356-358`) |
| ISFR registry + clearing | 14-identity-economy/13 | `isfr.rs` (1,277L): `IsfrRegistry`, 6-phase COMMIT→REVEAL→SOLVE→CERTIFICATE→VERIFY→SETTLE state machine, QP/KKT `ClearingCertificate` | 🔌 in-memory, no runtime caller | `isfr.rs:1-17,59-82`; builds on `phase2` types (`isfr.rs:23`) |
| `roko isfr` CLI | tmp/isfr/ | `Start`/`Status`/`Sources` (+`--json`); start = foreground keeper, relay publish is println ("WebSocket relay transport is Phase 2") | ✅ / 🟡 relay | `roko-cli/src/commands/isfr.rs:13-41,112`; registered `main.rs:628` |
| Serve ISFR routes | — | `/api/isfr/{status,current,history,sources}` reading keeper snapshots in `AppState.isfr` | ✅ | `routes/isfr.rs:22-26`; state `roko-serve/src/state.rs:405-416` |
| Serve chain routes | 22-REGISTRIES indexer | `/api/chain/{agents,bounties,status}` = live `eth_call` via `sol!` readers; `{blocks,transactions,events,watcher}` = ring buffers fed by block watcher | ✅ config-gated | `routes/chain.rs:21-44,65-70,175-213` |
| Block watcher (in-serve) | 08-chain chainwitness | `block_watcher.rs` (654L, `alloy-backend`): polls chain, fills `ChainState` ring buffers | ✅ wired | spawn `roko-serve/src/lib.rs:388,2443-2555`; state `state.rs:545,1029` |
| Chain-watcher subprocess | 08-chain peer scoring | serve spawns sibling `roko-chain-watcher` binary when `chain.rpc_url` set; output → `.roko/chain-watcher.log` | ✅ wired (best-effort) | `roko-serve/src/lib.rs:439-484` |
| `apps/roko-chain-watcher` | 08-chain | standalone bin: mirage JSON-RPC probe, block observer, pheromone/insight reaction loop, dry-run flag | ✅ runnable | `apps/roko-chain-watcher/src/main.rs:22-121`; workspace member `Cargo.toml:35-37` |
| Solidity contracts | 08-chain ERC-8004, ISFR | **13 authored**: **IdentityRegistry, ReputationRegistry, ValidationRegistry** (ERC-8004 trio), AgentRegistry, WorkerRegistry, RoleRegistry, ISFROracle, ISFRBountyPool, BountyMarket, ConsortiumValidator, InsightBoard, FeeDistributor, MockERC20; **10 forge test files** | ✅ exist w/ tests; 🟡 deploy parity | `contracts/src/*.sol` (13× `^contract`), `contracts/test/*.t.sol` (10×) — see contracts×deploy matrix below |
| Deploy script | — | `Deploy.s.sol` deploys only 6 contracts (no ERC-8004 trio, no ISFR set); ISFR set deployed by `isfr_bootstrap.rs`; roko-demo has a third, alloy-based deployer | 🟡 three parallel deploy paths | `contracts/script/Deploy.s.sol:14-17,28-44` |
| ERC-8004 registries (Rust) | 22-REGISTRIES; v2-depth/18 | `agent_registry.rs` (785L, soulbound passports, 24h prompt timelock, tier stakes), `reputation_registry.rs` (1,179L, 7-domain EMA), `validation_registry.rs` (456L, gate scores) | 🔌 real logic + tests, in-memory, no runtime consumer | `agent_registry.rs:1-28`; `lib.rs:45,79,87` |
| On-chain agent registration (serve) | ERC-8004 | `register_agent` route can submit via `OnChainAgentRegistry` `sol!` + `AlloyChainWallet` | ✅ wired (config-gated) | `routes/agents.rs:27,509-525` |
| ERC-8004 agent-card publish (sidecar) | ERC-8004 | `registration.rs:26-250` (`updateAgentCardUri` calldata, DataUriPublisher), optional `Arc<dyn ChainWallet>` | 🟡 CLI never builds wallet — `--wallet-key` is a dead knob ("reserved for future signing hooks") | `roko-agent-server/src/registration.rs:76-86`; `roko-cli/src/agent_serve.rs:168,354`; 44-AGENT-SERVER.md:44,70 |
| Marketplace (Spore) | 08-chain spore job market, hiring models, vickrey | `marketplace.rs` (1,096L): full lifecycle FSM, escrow, 3 hiring models (RandomVRF/Sparrow, commit-reveal **Vickrey**, DirectHire), 4-level disputes | 🔌 in-memory; runtime jobs use `.roko/jobs/*.json` instead | `marketplace.rs:1-45`; TUI F8 shows `roko_core::MarketplaceJob` via `/api/jobs`, not this (`tui/views/marketplace_view.rs:25,580`) |
| KORAI token | 08-chain tokenomics | `korai_token.rs` (657L): lazy demurrage | 🔌 in-memory; serve has a `start_demurrage_timer` (`roko-serve/src/lib.rs:347`) but no KORAI.sol on chain | `lib.rs:70-71,104` |
| x402 payments | 08-chain x402; docs/v2/18-PAYMENTS | `x402.rs` (958L): 402 flow, ERC-3009 `transferWithAuthorization`, state channels, balance proofs, settlement | 🔌 **zero consumers outside roko-chain** (grep x402 → 6 files, all in-crate) | `x402.rs:1-10`; `lib.rs:128-133` |
| Chain witness anchoring | 08-chain chainwitness | `witness.rs` (305L): `witness_on_chain` submits marker tx to `0x…c0`, attaches `ChainAttestation`; `verify_on_chain` checks receipt | 🔌 no callers outside roko-chain; explicitly "narrow… not the broader WitnessEngine" | `witness.rs:8-26,53-79`; sub-audit grep: internal-only |
| Triage/observer | 22-REGISTRIES indexer | `observer.rs` (345L), `triage.rs` (510L, MidasRScorer) | 🟡 used only by serve job_runner **against `MockChainClient::local()`** (synthetic chain-monitor jobs) | `roko-serve/src/job_runner.rs:407-449` |
| Gates (MEV, tx-sim, wallet) | 08-chain peer scoring/safety | `gate/` (2,029L): MevDetector, TxSimGate + MockTxSimulator, WalletGate | 🔌 exported, not in the 7-rung gate pipeline | `lib.rs:94-98` |
| TraceRank / collusion / Nelson-Siegel / futures market | 22-REGISTRIES, 24-DEFI | `trace_rank.rs` (508L), `collusion.rs` (379L), `nelson_siegel.rs` (307L), `futures_market.rs` (590L) | 🔌 primitives w/ tests; roko-demo `yield_routing` scenario touches futures market | `lib.rs:55,75,83,93`; `roko-demo/src/scenarios/yield_routing.rs` |
| Chain tools (17) + ISFR tools (4) | v2-depth/13 catalog | defs in `roko-chain/src/tools.rs:21-50` + `roko-std/tool/builtin/isfr.rs:132`; 37 builtins total (16 std + 17 chain + 4 ISFR) | ✅ defined & resolvable — `chain_aware_resolver` wires handlers incl. wallet from `chain.wallet_key` | `roko-std/tool/builtin/mod.rs:50-70`; `roko-cli/src/chain_registry.rs:14-51`; `run.rs:2588-2602`; ISFR tools error w/o live keeper (`isfr.rs:206`), `read_rate_history` always defers to serve (`:236-238`) |
| Config | — | `[chain]` (enabled, profile, rpc_url, chain_id, wallet_key, 5 registry addrs, auto_deploy_contracts, contracts_dir) + `[isfr]` (enabled, epochs, sources[]) | ✅ schema wired, **disabled by default** | `roko-core/src/config/chain.rs:11-28,147`; `schema.rs:145-150` |
| Chain profiles | tmp/isfr/ | `chain_profile.rs` (188L): `mirage` (31337, auto-deploy) / `daeji` (8004, `wss://rpc.daeji.dev/ws`) / custom; contract address book; save/load | ✅ | `chain_profile.rs:50-144` |
| ISFRFeed relay bridge | tmp/isfr/ | `roko-core/src/isfr_feed.rs`: relay TopicMessage → local PulseBus; serve spawns bridge | ✅ wired | `isfr_feed.rs:1-27`; `roko-serve/src/lib.rs:410-412` |
| Feed agents | docs/v2 FEEDS | 29 feed agents incl. `keeper.rs`, `oracle_submitter.rs`, `monitors.rs`, `epoch_tracker.rs`, `derivatives.rs`, `market.rs`, `defi.rs`, `source_scouts.rs` — spawned at startup, loop over in-memory state (synthetic except keeper) | 🟡 real plumbing, mostly synthetic data | `roko-serve/src/lib.rs:414-418`; `feed_agents/` |
| `apps/agent-relay` | tmp/relay-bus | real HTTP/WS relay on 127.0.0.1:9011, topic bus, protocol frames, optional embedded `chain_watcher.rs` task | ✅ runnable | sub-audit; e2e `roko-agent-server/tests/relay_registration.rs:157-222` (per 44-AGENT-SERVER.md:45) |
| `apps/mirage-rs` | 08-chain mirage-rs EVM sim | vendored in-process EVM fork simulator, JSON-RPC server, roko-core trait bridges | ✅ runnable (dev substrate) | sub-audit; referenced as default target throughout (`chain_profile.rs:51-61`) |
| `crates/roko-demo` | — | demo orchestrator: alloy `sol!` bindings (AgentRegistry/MockERC20/WorkerRegistry), own deployer, fixtures, scenarios, tournament, benchmark, TUI, verify | ✅ runnable — primary contracts consumer | `roko-demo/src/main.rs:16-29`; `Deploy.s.sol:14-17` notes demo "uses an alloy-based deployer" |
| Light clients / MPP | tmp/light-clients (31 files, 22 WUs: ConsensusVerifier, VerifiedState\<T\>, Tempo threshold-BLS, eth_getProof, MPP client/budget/ledger/discovery/server) | none | ❌ zero code | grep `ConsensusVerifier\|VerifiedState\|LightClient` in crates/ = 0; 21-TMP-MAY-BATCH.md:15,24 |
| HDC precompile / gossip / knowledge registry | 08-chain HDC precompile; 22-REGISTRIES | none | ❌ doc-only | 18-V2-DEPTH-COVERAGE.md:42 |
| Phase 2 stubs | docs/08-chain, docs/14-identity-economy | `phase2.rs` (2,312L), `identity_economy_identity.rs` (2,154L), `identity_economy_markets.rs` (1,428L) | 🕰️ placeholder types only, but **load-bearing**: registries/marketplace/x402/isfr all import from them | `phase2.rs:18`; `identity_economy_identity.rs:8-12`; `isfr.rs:23`, `marketplace.rs:17-21` |

## Module-by-module reality (roko-chain/src/*.rs — 40 files)

Verdict grep: external caller = any `roko_chain::<mod>` import outside `crates/roko-chain/src/` (HEAD 5852c93c05). "0 external" = shelf-ware: real, tested Rust logic reachable only from within the crate's own tests/re-exports.

| Module (file) | LOC | What it implements | On-chain vs in-memory | External caller (file:line) | Verdict |
|---|---|---|---|---|---|
| `lib.rs` | 132 | crate root, trait + type re-exports (`pub use phase2::*`) | — | many (façade) | ✅ façade |
| `client.rs` / `wallet.rs` | 71 / 36 | `ChainClient` / `ChainWallet` traits | trait defs | via alloy_impl/mock | ✅ stable |
| `alloy_impl.rs` | 344 | HTTP JSON-RPC client+wallet; sign/submit/wait | **on-chain** (real RPC) | serve `state.rs`, isfr_bootstrap `:210` | ✅ real, **get_logs Unsupported** (`:147-157`) |
| `mock.rs` | 738 | in-proc mock chain + paired mocks | in-memory | serve job_runner `:414` | ✅ test/sim |
| `types.rs` | ~200 | `LogEntry`, `BlockNumber`, `ChainError` | data | serve chain routes | ✅ |
| `isfr_keeper.rs` | 766 | poll loop, epochs, publish, TCP probe | **on-chain reads + publish** | serve `lib.rs:387,2203`; CLI `commands/isfr.rs:14` | ✅ **wired (flagship)** |
| `isfr_sources/` | ~500 | aave_v3/compound_v3/ethena/lido + mock; composite median | **on-chain `eth_call`** | isfr_keeper builder | ✅ wired |
| `isfr_oracle_submit.rs` | 123 | `submitRate()` via `sol!` | **on-chain write** | serve `lib.rs:2289`; non-alloy=no-op | ✅ wired per-epoch |
| `isfr_bootstrap.rs` | 471 | deploys 5 ISFR contracts from forge `out/` | **on-chain deploy** | serve `lib.rs:348-385` | ✅ wired (deploy path #2) |
| `block_watcher.rs` | 654 | polls chain → `ChainState` ring buffers | **on-chain reads** | serve `lib.rs:388,2443` | ✅ wired |
| `observer.rs` / `triage.rs` | 345 / 510 | `ObservedEvent`, `MidasRScorer` | in-memory (over MockChainClient) | serve job_runner `:407-449` | 🟡 mock-backed |
| `chain_profile.rs` | 188 | mirage/daeji/custom address book | config | serve/CLI | ✅ |
| `tools.rs` | ~900 | 17 chain tool defs + handlers | mixed | std resolver `chain_registry.rs:14` | ✅ resolvable |
| **`witness.rs`** | 305 | `witness_on_chain` marker tx to `0x…c0`, `ChainAttestation`, verify | on-chain-capable | **0 external** (grep `witness_on_chain` = 0 outside crate) | 🔌 **shelf-ware** |
| **`x402.rs`** | 958 | 402 flow, ERC-3009, state channels, balance proofs, settlement | in-memory (chain-shaped) | **0 external** | 🔌 **shelf-ware** |
| **`korai_token.rs`** | 657 | KORAI token, lazy demurrage | in-memory (no KORAI.sol) | **0 external** | 🔌 **shelf-ware** |
| **`marketplace.rs`** | 1,096 | Spore FSM, escrow, 3 hiring models (Vickrey/Sparrow/Direct), disputes | in-memory | **0 external** (runtime jobs use `.roko/jobs/*.json`) | 🔌 **shelf-ware** |
| **`agent_registry.rs`** | 785 | ERC-8004 soulbound passports, prompt timelock, tier stakes | in-memory (Solidity twin exists) | **0 external** | 🔌 **shelf-ware** |
| **`reputation_registry.rs`** | 1,179 | 7-domain EMA reputation | in-memory | **0 external** (1 hit = sol! binding) | 🔌 **shelf-ware** |
| **`validation_registry.rs`** | 456 | gate-score validation records | in-memory | **0 external** | 🔌 **shelf-ware** |
| **`isfr.rs`** | 1,277 | `IsfrRegistry` 6-phase COMMIT→…→SETTLE, QP/KKT clearing certs | in-memory | **0 external** (keeper does NOT run this) | 🔌 **shelf-ware** |
| **`trace_rank.rs`** | 508 | TraceRank PageRank-style scoring | in-memory | **0 external** | 🔌 shelf-ware |
| **`collusion.rs`** | 379 | collusion-ring detection | in-memory | **0 external** | 🔌 shelf-ware |
| **`nelson_siegel.rs`** | 307 | Nelson-Siegel yield curve | in-memory | **0 external** | 🔌 shelf-ware |
| **`futures_market.rs`** | 590 | perpetual futures book | in-memory | **0 external** (roko-demo `yield_routing` scenario touches, but that's an app not runtime) | 🔌 shelf-ware |
| **`gate/{mev,tx_sim,wallet}_gate.rs`** | 2,029 | MevDetector, TxSimGate+MockTxSimulator, WalletGate | in-memory | **0 external** (not in 7-rung pipeline) | 🔌 shelf-ware |
| **`heartbeat_ext.rs`** | ~450 | policy-cage heartbeat extension | in-memory | **0 external** | 🔌 shelf-ware |
| **`phase2.rs`** | 2,312 | placeholder types (`Address`, `u256`, …) "derived from docs/08-chain" (`:18`) | — | in-crate only (load-bearing import target) | 🕰️ stub, load-bearing |
| **`identity_economy_identity.rs`** | 2,154 | identity stubs "intentionally avoid real runtime logic" (`:8-12`) | — | 0 external | 🕰️ stub |
| **`identity_economy_markets.rs`** | 1,428 | market stubs; `#![allow(dead_code)]` (`:2`); **dead 2nd `struct Engram` (`:653`)** | — | 0 external | 🕰️ stub |

**Tally: 15+ modules with real logic and ZERO runtime callers** (witness, x402, korai_token, marketplace, agent_registry, reputation_registry, validation_registry, isfr.rs/IsfrRegistry, trace_rank, collusion, nelson_siegel, futures_market, gate/mev, gate/tx_sim, gate/wallet, heartbeat_ext = 16 counting the three gates separately). Plus 3 self-declared stub modules. The wired surface is exactly the **ISFR vertical + alloy/mock/block_watcher/observer/tools plumbing** — nothing in the reputation/witness/payments/marketplace half is consulted by the agent runtime.

## Contracts × deploy-path matrix (13 contracts, 3 deployers)

Three disjoint deployers exist; **no single path deploys all 13** contracts. `Deploy.s.sol` is explicitly "not used by roko-demo" (`Deploy.s.sol:14-17` doc comment); roko-demo uses a manifest-driven alloy deployer (`roko-demo/src/deploy.rs:1-5,193`); isfr_bootstrap deploys the ISFR-oracle slice at serve startup (`isfr_bootstrap.rs:236-292`).

| Contract (`contracts/src/*.sol`) | forge test? | `Deploy.s.sol` (6) | `isfr_bootstrap.rs` (5) | `roko-demo` alloy (manifest) |
|---|:--:|:--:|:--:|:--:|
| `MockERC20` ("DAEJI") | ✅ | ✅ | ✅ (token) | ✅ |
| `AgentRegistry` | ✅ | ✅ | — | ✅ |
| `WorkerRegistry` | ✅ | ✅ | ✅ | ✅ |
| `RoleRegistry` | ❌ | — | ✅ | — |
| `BountyMarket` | ✅ | ✅ | — | ✅ |
| `ConsortiumValidator` | ✅ | ✅ | — | ✅ |
| `InsightBoard` | ✅ | ✅ | — | ✅ |
| `ISFROracle` | ❌ | — | ✅ | — |
| `ISFRBountyPool` | ❌ | — | ✅ | — |
| `IdentityRegistry` (ERC-8004) | ✅ | — | — | — |
| `ReputationRegistry` (ERC-8004) | ✅ | — | — | — |
| `ValidationRegistry` (ERC-8004) | ✅ | — | — | — |
| `FeeDistributor` | ✅ | — | — | — |

Coverage gaps: **the ERC-8004 trio (`IdentityRegistry`/`ReputationRegistry`/`ValidationRegistry`) and `FeeDistributor` are authored + forge-tested but deployed by NO path** — they exist only as compiled artifacts and forge fixtures. `RoleRegistry`/`ISFROracle`/`ISFRBountyPool` are deploy-only (no `.t.sol`). Only `MockERC20` + `WorkerRegistry` are covered by all three deployers.

## get_logs Unsupported — blast radius

`AlloyChainClient::get_logs` returns `Err(ChainError::Unsupported("get_logs"))` (`alloy_impl.rs:147-157`) — the only real client cannot do `eth_getLogs`. This is the single load-bearing gap blocking the entire event-indexing half of 22-REGISTRIES:

- **No event-sourced registry.** Any registry that reconstructs state from emitted events (`AgentRegistered`, `ReputationUpdated`, `BountyPosted`) cannot be built against the live client — the Rust registries stay in-memory partly because there's no log stream to hydrate them.
- **Block watcher is the only workaround.** `/api/chain/events` is populated by `block_watcher.rs` decoding full blocks into ring buffers (`routes/chain.rs:175-213`), NOT by a log filter — so it sees only what it polls, can't backfill history, and can't filter by topic/address server-side.
- **x402/witness verification degraded.** Settlement-proof and attestation flows that would confirm via log lookup fall back to receipt polling (`witness.rs:53-79` uses receipts, not logs).
- **Fix is small, unblocks a lot.** The mock already returns logs; wiring `provider.get_logs(Filter)` in `alloy_impl.rs` is ~20 lines and would flip event-indexer, log-based registry hydration, and topic-filtered `/api/chain/events` from ❌ to buildable.

## Client (roko-chain) vs daeji (separate repo) — the fence

This is the single most important navigation fact for this area: **chain maturity is split across two repos, and roko-chain is only the client half.**

| Concern | roko-chain (THIS repo) | daeji (SEPARATE repo — design-only here) |
|---|---|---|
| Location | `crates/roko-chain/` (live code) | `tmp/agentchain-v2/02-daeji/` (design docs only; no `.rs`) |
| Role | **client / caller** — signs txs, reads state, deploys contracts via alloy | **the L1 node** — consensus, execution, state, precompiles |
| Consensus | none (talks to whatever RPC) | Simplex BFT + threshold BLS12-381, ~400ms single-slot finality, 4 validators (`02-daeji/05-roadmap.md:28-50`) |
| Execution | none | REVM (`02-daeji/05-roadmap.md`) |
| State | none | QMDB state storage |
| RPC surface consumed | `eth_*` (+ future `kora_*`) via alloy | serves `eth_*`/`net_*`/`web3_*`/`kora_*` |
| Precompiles | **calls** them once live (roko's job) | **defines** them: HDC similarity `0x09`-overload, QMDB proofs `0x0B`, BTLE `0x0C`, ISFR oracle `0xA01`, agent-comm `0xA10-0xA1F` — all "Designed; pending precompile-registry wiring" (`02-daeji/02-precompiles-and-contracts.md:44-61`); node still loads only `0x01-0x09` |
| Contracts | 13 authored Solidity (client's view) | canonical suite lives daeji-side; roko's may not match daeji's "17→7 redesign" (20-TMP-NEWEST.md:21) |
| Light clients / verified state | ❌ zero code (`tmp/light-clients/`, 22 WUs designed, 0 impl) | daeji's QMDB proofs + BLS consensus ARE the verification primitives → likely **superseded-by-daeji**, not to-build-in-roko |
| Runtime today | mirage-rs (`apps/mirage-rs/`) in-process EVM fork sim + anvil | live devnet per daeji roadmap (chain_id 8004, `wss://rpc.daeji.dev/ws`) — **not runnable from this repo** |

**Reading rule for this doc's ❌/"Not implemented" rows:** they mean "not in *roko*." Several (real BFT, precompiles, verified state, QMDB proofs) are **built or designed in daeji**, not gaps roko must fill — roko's job is to *consume* them via `kora_*` RPC + precompile calls once daeji wires them. `ChainProfile::daeji` (`chain_profile.rs:50-73`, `wss://rpc.daeji.dev/ws`) is precisely the client-side handle to that separate chain. See also doc 106 (`apps/mirage-rs`) — mirage's `dashboard-api` is the schema-of-record for the fork-sim substrate roko-chain targets in dev.

## Deep-pass module-reality checklist

- [ ] **[P0]** Wire ONE consumer for the shelf-ware half or stamp it Phase-2 in CLAUDE.md — 16 modules (witness, x402, korai, marketplace, 3 registries, IsfrRegistry, trace_rank, collusion, nelson_siegel, futures, 3 gates, heartbeat_ext) have real logic + tests + **0 runtime callers** — verify: `grep -rn "witness_on_chain\|SporeMarket\|IsfrRegistry" crates/ | grep -v roko-chain/src` non-empty, or GAPS.md stamps each
- [ ] **[P1]** Implement `AlloyChainClient::get_logs` (~20 LOC, mock already returns logs) — unblocks event indexer + log-based registry hydration + topic-filtered `/api/chain/events` — verify: `cargo test -p roko-chain --features alloy-backend get_logs` green
- [ ] **[P1]** Deploy-path parity: extend one canonical deployer to cover the ERC-8004 trio + `FeeDistributor` (authored + forge-tested but deployed by **no** path) — verify: one `forge script` prints all 13 addresses
- [ ] **[P2]** Delete/annotate the dead 2nd `struct Engram` (`identity_economy_markets.rs:653`, `#![allow(dead_code)]`) shadowing `roko_core::Engram` — verify: `grep -rn "pub struct Engram" crates/roko-chain` empty or carries stub doc-line
- [ ] **[P2]** Fix forge-test/contract accounting in CLAUDE.md + this area's other docs: **13 contracts, 10 `.t.sol`** (RoleRegistry/ISFROracle/ISFRBountyPool untested) — verify: counts match `ls contracts/{src,test}`
- [ ] **[P3]** Declare the canonical Phase-2 target once (mirage vs anvil vs daeji vs Tempo) and fence roko-chain=client / daeji=node in CLAUDE.md — verify: CLAUDE.md roko-chain row links daeji design pack + states client role

## Drift spotlight: the second `Engram` (name collision, not a build error)

roko-chain declares a **second `pub struct Engram`** at `identity_economy_markets.rs:653` ("Content-addressed knowledge unit for forensic replay stubs"), independent of roko-core's canonical `Engram` at `roko-core/src/engram.rs:63`. This is the collision wave-1 flagged; here is the exact shape of it:

- **Not a compile conflict.** They live in different crates/modules and neither is re-exported into the other's namespace. `lib.rs` does `pub use phase2::*` but does **not** glob-re-export `identity_economy_markets`, so the chain `Engram` never reaches roko-chain's crate root, let alone roko-core's.
- **Structurally divergent.** core `Engram` = `{id: ContentHash, fingerprint: Option<HdcFingerprint>, kind, body: Body, created_at_ms: i64, decay: Decay, provenance, score: Score, …}` — the live signal-graph type. chain `Engram` = `{hash: Blake3Hash, kind: Kind, body: Vec<u8>, author: AgentId, tags, lineage: Vec<Blake3Hash>, score: [f64;7], tier: Tier, created_at: u64, provenance: Provenance}` — a hash-DAG placeholder with its own local `Provenance`, `Kind`, `Tier`, `CustodyEntry` (a **second** shadow of core's custody/provenance vocabulary too).
- **Dead code by declaration.** The module opens `#![allow(dead_code)]` (`identity_economy_markets.rs:2`) and self-labels "Phase 2+ … stubs" (`:8-9`). Grep across roko-chain for any use of the chain `Engram` returns **zero** references — every `use … Engram` in the crate imports `roko_core::Engram` (the three gate modules: `gate/{mev,tx_sim,wallet}_gate.rs`). So the chain copy is unreferenced placeholder text, not load-bearing.
- **Why it still matters (navigation drift).** It (a) misleads any grep/index that keys on the type name — the code-intelligence MCP and `roko index` will surface two `Engram` definitions with no hint that one is a stub; (b) signals design confusion — the stub reinvents core's engram/provenance/custody/tier concepts inside chain rather than importing them, so if these markets ever go live they'd fork the memory model. Same pattern repeats for the stub-local `Provenance` (`:678`) and `CustodyEntry` shadowing `roko-core` custody types. **Recommended:** delete the chain `Engram`/`Provenance`/`CustodyEntry` placeholders and import `roko_core::{Engram, Provenance}` if/when markets are built; until then, annotate at the definition that it is a throwaway stub shadowing `roko_core::Engram`.

## ISFR end-to-end trace

The only chain feature that runs source→chain→API→UI today:

1. **Config**: `[isfr] enabled = true` + optional `[[isfr.sources]]` in roko.toml (`roko-core/src/config/chain.rs:147`). No sources → 4-source mock keeper (`roko-serve/src/lib.rs:2229-2230`).
2. **Sources**: `ISFRKeeper::from_config` probes each `rpc_url` via TCP (500ms), falls back to `ETH_RPC_URL` env (live mainnet reads when mirage is down), else marks source `OfflineSource` (`isfr_keeper.rs:190-264,301-364`). Live kinds: `aave_v3`, `compound_v3`, `ethena`, `eth_staking` (Lido) — real `eth_call`s behind `alloy-backend` (`isfr_sources/mod.rs:6-14`).
3. **Aggregate**: per-tick weighted median per rate class + equal-weight composite + live-source confidence (`isfr_sources/mod.rs:178-268`).
4. **Publish**: keeper `publish_fn` → `AppState.isfr` (current rate, 256-entry history, source snapshots, epoch atomic) (`roko-serve/src/lib.rs:2266-2302`).
5. **On-chain**: on epoch rollover, `submit_rate_on_chain` sends `ISFROracle.submitRate(epoch, composite, 4 class rates, confidence)` via alloy `sol!` from the keeper wallet — fire-and-forget (`lib.rs:2289-2292,2374-2384`; `isfr_oracle_submit.rs:59-123`).
6. **Contracts**: ISFROracle + ISFRBountyPool auto-deployed at serve startup from forge artifacts when `chain.auto_deploy_contracts` and `contracts/out/` exist; role grants + pool funding included (`lib.rs:348-385`; `isfr_bootstrap.rs:13-26`). ISFROracle rewards submissions via bounty pool (`contracts/src/ISFROracle.sol`, `ISFRBountyPool.sol`).
7. **Read**: `/api/isfr/{status,current,history,sources}` (`routes/isfr.rs:22-26`); `roko isfr status|sources` hit those endpoints with config-only fallback (`commands/isfr.rs:24-40`); health surface includes ISFR (`routes/status/health.rs`).
8. **Not closed**: relay pub/sub transport (CLI `start` prints "(WebSocket relay transport is Phase 2)", `commands/isfr.rs:112`; serve passes `relay_url: None`, `lib.rs:2224`); the in-crate `IsfrRegistry` 6-phase clearing/settlement (`isfr.rs`) is **not** what the keeper runs — commit-reveal, certificates, and reward settlement per the spec exist only as the unwired in-memory registry.

## Runnable vs primitives-only

**Runnable today** (given mirage/anvil + config): `roko serve` with `[chain]`+`[isfr]` → auto-deploy, keeper, block watcher, chain-watcher subprocess, chain/ISFR routes; `roko isfr start|status|sources`; `roko-chain-watcher` standalone; `roko-demo` deploy/scenarios/tournament; `agent-relay`; chain tools via `roko run` (wallet from `chain.wallet_key`, `run.rs:2588-2602`).

**Default `roko serve` with stock config**: none of it — `[chain]`/`[isfr]` absent ⇒ keeper no-op (`lib.rs:2214-2217`), no chain client (`state.chain_client` None), no watcher, no deploys. Chain surface is fully opt-in.

**Primitives-only (real code, zero runtime callers)**: witness anchoring, x402, Rust registries (agent/reputation/validation), KoraiToken, Spore marketplace, IsfrRegistry clearing, TraceRank, collusion, MEV/tx-sim/wallet gates, futures market, Nelson-Siegel, heartbeat_ext policy cage.

**Mock/sim-backed**: job_runner "chain monitor" jobs (`MockChainClient::local()`, `job_runner.rs:414`), most feed agents (synthetic market/defi/derivatives data), mock ISFR sources, mirage-rs itself (a simulator, not a network).

## V2-aligned

- ISFR keeper+sources+oracle vertical matches 24-DEFI's oracle design; composite math follows 14-identity-economy/13 (weighted median, 3σ, 8h epochs) (`isfr.rs:8-14`).
- ERC-8004 trio exists both as Solidity (contracts/src) and Rust models; serve reads/writes chain via `sol!` (routes/chain.rs, agents.rs:509-525) — the right shape for 22-REGISTRIES, just not consulted by the runtime.
- Trait-first design (`ChainClient`/`ChainWallet` backend-agnostic; mirage/anvil/testnet interchangeable) matches v2's substrate philosophy (`lib.rs:37-43`).
- Marketplace hiring models (Sparrow p2c, Vickrey commit-reveal, DirectHire premium) implement 08-chain's hiring-models spec faithfully in-memory (`marketplace.rs:1-11`).
- x402 struct layer matches 18-PAYMENTS' stateless x402 half (ERC-3009 + channels); MPP streaming half is the unbuilt light-clients WU17-22.

## Old paradigm & tech debt

- **phase2/identity_economy stubs are load-bearing**: 5,894 LOC of "intentionally no runtime logic" types that real modules (isfr, marketplace, x402, registries) import. Any real chain integration must migrate off `phase2::{Address,u256}` toward alloy types. 🕰️
- **Dual/duplicate systems**: Rust in-memory registries vs Solidity contracts (no adapter links them); local JSON jobs vs chain BountyMarket; three deploy paths (Deploy.s.sol ⊂ isfr_bootstrap ⊂ roko-demo deployer) with disjoint contract coverage (`Deploy.s.sol:28-44` deploys 6/13; ERC-8004 trio + FeeDistributor deployed by **none** — see contracts×deploy matrix).
- **Naming collision (ISFR)**: TUI header "ISFR" = "Inter-Signal Frequency Ratio" (gate pass rate!) — `tui/state.rs:1237-1238,1941`, `widgets/header_bar.rs:280-302` — completely unrelated to the fact-registry ISFR. Confusing in any dashboard conversation.
- **Naming collision (Engram)**: a second `Engram` (plus shadow `Provenance`/`CustodyEntry`/`Kind`/`Tier`) in `identity_economy_markets.rs:653` shadows `roko_core::Engram` (`engram.rs:63`). Dead-code stub, zero consumers, but pollutes symbol search/index and forks the memory model on paper (see Drift spotlight). 🕰️
- **Dead knob**: `roko agent serve --wallet-key` parsed, never used (`agent_serve.rs:168,354`); on-chain agent-card publish unreachable from CLI (44-AGENT-SERVER.md:70,93).
- **`get_logs` unsupported** in the only real client (`alloy_impl.rs:147-157`) — blocks any real indexer/event-sourced registry per 22-REGISTRIES.
- **Rustc/alloy constraint**: workspace `rust-version = "1.85"` (`Cargo.toml:93`) but alloy 1.x needs 1.91+ (CLAUDE.md blocker #1); since roko-cli/serve/demo hard-enable `alloy-backend`, the whole default build inherits the constraint despite roko-chain's empty default features.
- **Keeper oracle submission is fire-and-forget** with no retry/nonce management beyond alloy defaults (`isfr_oracle_submit.rs:21-24,120-122`); non-alloy builds silently no-op (`:25-37`, `isfr_bootstrap.rs:28-52`).
- CLAUDE.md crate table ("Chain witness primitives · Phase 2+") materially understates this area — a doc bug that misdirects agents.

## Not implemented

- Light clients / verified state / Tempo BLS / MPP payments — 22 WUs fully designed in `tmp/light-clients/` (31 files), zero code (grep = 0 hits; 21-TMP-MAY-BATCH.md:15).
- HDC precompile, gossip layer, knowledge registry (publication/challenge/resolution), event indexer from 22-REGISTRIES — doc-only (18-V2-DEPTH-COVERAGE.md:42).
- KORAI on chain (no KORAI.sol; deploys use MockERC20 "DAEJI"); real tokenomics/demurrage on-chain.
- Runtime witness anchoring of episodes/attestations (helper exists, nothing calls it).
- Reputation-informed routing/hiring (CLAUDE.md open item 13; ReputationRegistry unconsulted).
- x402 enforcement anywhere (no 402 middleware in serve or agent-server).
- ISFR clearing/settlement on-chain (commit-reveal, certificates, reward distribution) — in-memory only.
- Vickrey/marketplace runtime (jobs never route through `Marketplace`).

## Cross-check vs `tmp/agentchain-v2/` (newer design pack, not in original sources)

The `tmp/agentchain-v2/` pack (dirs `01-roko/`, `02-daeji/`, `03-isfr/`, `04-markets/`) postdates this doc's cited `tmp/isfr` + `tmp/relay-bus` sources and materially reframes several open questions. It splits the vision into **two repos**: `01-roko/` (this crate's runtime side) and `02-daeji/` (a *separate* chain repo). Key deltas for navigation:

- **Daeji is real and running** (answers Open Q1/Q2). `02-daeji/05-roadmap.md:28-50` describes a live devnet: Simplex BFT + threshold BLS12-381 single-slot finality (~400 ms, 4 validators), REVM execution, QMDB state storage, Ed25519 P2P, `eth_*`/`net_*`/`web3_*` + `kora_*` JSON-RPC, DKG ceremonies, EIP-1559. This is **not in the roko workspace** — the canonical contracts/consensus live in the daeji repo, so roko's `contracts/` (13 Solidity files) and `ChainProfile::daeji` (`wss://rpc.daeji.dev/ws`) are the *client-side* view of that chain.
- **Precompiles are the real differentiator** (`02-daeji/02-precompiles-and-contracts.md:44-61`): HDC Similarity Search overloaded on `0x09`, QMDB Historical State Proofs `0x0B`, BTLE threshold enc/dec `0x0C`, ISFR oracle reserved at `0xA01`, agent-comm namespace `0xA10–0xA1F`. All "Designed; pending precompile-registry wiring" — i.e., the node still loads only standard `0x01–0x09`. This is the concrete home of roko's ❌ "HDC precompile / knowledge registry" items — **designed in daeji, not roko**; roko-chain's job is to *call* them once wired.
- **Light-clients / verified-state (roko ❌ zero-code) may be a daeji-side concern**: daeji's QMDB state proofs + threshold-BLS consensus are exactly the primitives `tmp/light-clients/` designed to verify. So the 22 unbuilt WUs may be **superseded-by-daeji** rather than to-be-built-in-roko — this sharpens migration checklist item P0 ("decide fate of tmp/light-clients"): the decision is likely "daeji owns verification; roko consumes via `kora_*`/precompiles."
- **ISFR pack (`03-isfr/`) confirms** the keeper→oracle→bounty vertical as the flagship, with `02-yield-perpetual.md`/`03-market-and-framework.md` matching the in-crate `futures_market.rs` + `IsfrRegistry` clearing design (still in-memory in roko).
- **Net**: this doc's status matrix stays accurate for the **roko repo**, but its "Not implemented / ❌" rows should be read as "not in *roko*" — several (precompiles, verified state, real BFT) are **built or designed in the daeji repo**. The navigation-layer takeaway: chain maturity is split across two repos; auditing roko alone understates the program.

## Migration checklist

- [ ] **[P0]** Decide canonical job/identity mode — local-only, chain-optional, or chain-canonical; today local JSON and chain BountyMarket are unrelated systems — verify: decision doc in docs/v2/22-REGISTRIES.md or `.roko/GAPS.md`; TUI F8 + `roko job list` labeled accordingly
- [ ] **[P0]** Update CLAUDE.md roko-chain row (and status table) to reflect ISFR/watcher/registries reality — verify: `grep -n "Phase 2" CLAUDE.md` no longer describes roko-chain as primitives-only
- [ ] **[P0]** Decide fate of `tmp/light-clients/` (adopt & re-baseline 22 WUs vs mark superseded by x402/alloy) — verify: `grep -rn ConsensusVerifier crates/` non-empty, or WU index stamped SUPERSEDED
- [ ] **[P1]** Build `AlloyChainWallet` from `--wallet-key` in `roko agent serve` and pass to `AgentRegistration.wallet` — verify: `cargo run -p roko-cli -- agent serve --wallet-key 0xac09… --identity-registry …` submits `updateAgentCardUri` against mirage (tx hash in logs)
- [ ] **[P1]** Implement `eth_getLogs` in `AlloyChainClient` — verify: `cargo test -p roko-chain --features alloy-backend get_logs` green; `/api/chain/events` populated from logs not just watcher decode
- [ ] **[P1]** Deploy-path parity: extend `Deploy.s.sol` (or one canonical deployer) to cover ERC-8004 trio + RoleRegistry + ISFROracle + ISFRBountyPool + FeeDistributor — verify: `forge script contracts/script/Deploy.s.sol` prints all 14 addresses; `forge test` green
- [ ] **[P1]** Wire `ChainWitnessEngine` into the attestation/episode flow (config-gated) — verify: `grep -rn witness_on_chain crates/ | grep -v roko-chain/src` shows a caller; an episode gains `chain_attestation` after a gated run against mirage
- [ ] **[P2]** Close the keeper relay loop: honor `relay_url` (serve passes None, CLI prints "Phase 2") so `isfr:rates` reaches agent-relay subscribers — verify: `roko isfr start` + agent-relay subscriber receives topic frames; `ISFRFeed` republishes Pulses
- [ ] **[P2]** Replace job_runner synthetic chain-monitor (`MockChainClient::local()`, job_runner.rs:414) with the configured client/BlockWatcher events — verify: chain-monitor job against mirage reports real block numbers
- [ ] **[P2]** Reputation consumption: feed `ReputationRegistry`/TraceRank into CascadeRouter or hiring — verify: routing decision log cites a reputation score
- [ ] **[P2]** x402: either mount a 402 payment gate on agent-server `/message` per 18-PAYMENTS or stamp it Phase-2-explicitly — verify: `grep -rn x402 crates/ | grep -v roko-chain` non-empty, or GAPS.md entry
- [ ] **[P3]** De-duplicate Rust registries vs Solidity: pick source of truth, add an adapter (Rust model ↔ `sol!` calls) for the other — verify: one documented path; parity test comparing in-memory vs on-chain state
- [ ] **[P3]** Rename or annotate TUI "ISFR" (Inter-Signal Frequency Ratio) to avoid collision with the fact-registry ISFR — verify: header shows distinct label; grep confirms
- [ ] **[P3]** Delete or explicitly stub-annotate the shadow `Engram`/`Provenance`/`CustodyEntry` in `identity_economy_markets.rs` (import `roko_core::{Engram, Provenance}` if markets go live) — verify: `grep -rn "pub struct Engram" crates/roko-chain` empty, or the definition carries a "throwaway stub, shadows roko_core::Engram" doc line
- [ ] **[P3]** Migrate `phase2::{Address,u256}` in live modules (isfr, marketplace, x402) toward `alloy_primitives` — verify: `grep -n "phase2::" crates/roko-chain/src/isfr.rs` shrinks; workspace builds

## Open questions

1. ~~**Is daeji real?**~~ **Resolved (partially) by `tmp/agentchain-v2/02-daeji/`:** daeji is a live devnet (Simplex BFT + BLS12-381, REVM, QMDB) whose canonical contracts/consensus live in the **daeji repo**, not this one. roko's `contracts/` (13 Solidity) + `ChainProfile::daeji` are the client-side view. Still open: does daeji's contract suite match roko's 13, or the 20-TMP-NEWEST.md:21 "17→7 redesign"? (`chain_profile.rs:63-73`).
2. **Target chain identity**: light-clients docs test against Tempo (`rpc.moderato.tempo.xyz`, `tmp/light-clients/00-INDEX.md:37`); agentchain-v2 says the target is **daeji** (chain_id 8004, EVM L1 with agent precompiles). Is Tempo a superseded target, or a second deployment surface? The mirage/anvil/daeji/Tempo quartet needs a single canonical "Phase-2 target" declaration.
3. **Who runs `roko-chain-watcher` in production** — serve's fire-and-forget subprocess (`lib.rs:445-483`), the daemon, or agent-relay's embedded watcher? Three spawn paths exist.
4. **Should synthetic feed-agent data (market/defi/derivatives) be visibly labeled** in dashboard/API responses to avoid being mistaken for live market data?
5. **KORAI plan**: in-memory `KoraiToken` demurrage + serve `start_demurrage_timer` vs MockERC20("DAEJI") in every deploy — is KORAI still the token design, and where does its contract live?
6. **IsfrRegistry (commit-reveal clearing) vs ISFRKeeper (poll-and-publish)** — is the keeper the interim implementation of the same spec, or are both meant to ship (keeper = data plane, registry = settlement plane)?
