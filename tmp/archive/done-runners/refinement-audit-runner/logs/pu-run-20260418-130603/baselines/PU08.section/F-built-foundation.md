# F — Built Foundation (Docs 17, 18)

Parity of `docs/08-chain/17-chain-client-wallet-traits.md` and
`docs/08-chain/18-mirage-rs-evm-simulator.md` against the shipping
`crates/roko-chain/` and `apps/mirage-rs/`. These are the two **"Implementation:
Built"** chapters in topic 08 — F is therefore the section where most entries
should be DONE, and the ones that drift do so in the **over-claim** direction
(specs narrate a richer surface than the code currently ships).

Generated 2026-04-16.

---

## F.01 — Crate layout and public surface (Doc 17 §"ChainClient Trait", Doc 24 §"What Is Built")

**Status**: DONE
**Severity**: —
**Doc claim**: `roko-chain` ships `ChainClient` (8 read methods), `ChainWallet` (5 write methods), all supporting types, `TxSimGate` + `WalletGate`, and `MockChainClient` + `MockChainWallet`.
**Reality**: The shipping `lib.rs:1-31` declares modules `alloy_impl` (feature-gated), `client`, `gate`, `mock`, `types`, `wallet`, `witness`, and re-exports `ChainClient`, `ChainWallet`, `{MockTxSimulator, SimulationOutcome, TxSimGate, TxSimGateConfig, TxSimulator, WalletCheck, WalletGate, WalletGateConfig}`, `{MockChainClient, MockChainWallet, paired_mocks}`, `{BlockNumber, CallResult, ChainError, ChainHeader, ChainResult, LogEntry, Receipt, TxHash, TxRequest}`, and `{ChainWitnessEngine, verify_on_chain, witness_on_chain}`. Eight top-level source files plus the `gate/` subdirectory:

| File | LOC | What |
|------|-----|------|
| `lib.rs` | 30 | Re-exports |
| `client.rs` | 64 | `ChainClient` trait |
| `wallet.rs` | 36 | `ChainWallet` trait |
| `types.rs` | 232 | Types + tests |
| `mock.rs` | 719 | Mock client/wallet + 30+ tests |
| `witness.rs` | 199 | `ChainWitnessEngine` + 2 tests |
| `alloy_impl.rs` | 327 | Live alloy backend (feature-gated) |
| `gate/mod.rs` | 32 | Gate re-exports |
| `gate/tx_sim_gate.rs` | 448 | `TxSimGate` + `TxSimulator` |
| `gate/wallet_gate.rs` | 523 | `WalletGate` + `WalletCheck` |

Doc 24 §"What Is Built" lists every one of the above. The crate surface matches the doc.

---

## F.02 — `ChainClient` trait has 8 methods (Doc 17 §"ChainClient Trait")

**Status**: DONE
**Severity**: —
**Doc claim**: The trait has exactly eight methods — `block_number`, `get_block_header`, `get_receipt`, `get_logs`, `get_storage_at`, `eth_call`, `chain_id`, `name` — and is `Send + Sync`.
**Reality**: `client.rs:22-64` declares `#[async_trait] pub trait ChainClient: Send + Sync` with exactly those eight methods: `block_number` (`:24`), `get_block_header` (`:27`), `get_receipt` (`:30`), `get_logs` (`:36-42`), `get_storage_at` (`:45-50`), `eth_call` (`:53-57`), `chain_id` (`:60`), `name` (`:63`). The doc's snippet uses `&[Address]` + `&[Vec<B256>]` for logs, but the shipping trait takes `&[String]` + `&[String]` (`client.rs:40-41`) — the trait is deliberately **typeless at the address/topic boundary** so the mock and alloy impls can share it. `types.rs:14-21` explains: "Stored as a `String` rather than `[u8; 32]` so the mock backends and downstream serialized formats can round-trip unmodified." The method count and shape match; the parameter typing in the doc is illustrative, not literal.

---

## F.03 — `ChainWallet` trait has 6 methods, not 5 (Doc 17 §"ChainWallet Trait")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: "ChainWallet ... 5 methods for chain writes" (Doc 17 §"Current Status" + Doc 24 §"What Is Built"). The code-shape snippet at Doc 17 `:86-111` shows six entries (`address`, `balance`, `nonce`, `sign_and_submit`, `wait_for_receipt`, `name`).
**Reality**: The trait at `wallet.rs:17-36` has exactly six methods — `address` (`:19`), `balance` (`:22`), `nonce` (`:25`), `sign_and_submit` (`:29`), `wait_for_receipt` (`:32`), `name` (`:35`). The "5 methods" sentence in Doc 24 undercounts by omitting `name()` (a non-async metadata accessor). The trait itself is fine; the status-doc count is stale.
**Fix sketch**: Update Doc 24 §"What Is Built" row for `ChainWallet` to say "6 methods" to match the code and the Doc 17 code snippet.

---

## F.04 — Supporting types use plain strings, not alloy types (Doc 17 §"Supporting Types")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: The types table uses `Address`, `U256`, `B256`, `Bytes` (alloy-primitives) — e.g. `pub struct TxHash = B256;`, `pub value: U256`, `pub data: Bytes`.
**Reality**: `types.rs` intentionally does not import alloy. `TxHash` is `struct TxHash(pub String)` (`:20-21`), `value: u128` (`:74`), `data: Vec<u8>` (`:76`), hash fields are `String` (`:47-49`), storage-slot reads return `Vec<u8>` (`client.rs:50`). The rationale is written into the file: `types.rs:1-7` says "deliberately narrow ... backend-agnostic, no Alloy / revm / k256 coupling." The alloy-typed versions live only inside `alloy_impl.rs:10-14` where the `AlloyChainClient` converts at the boundary (`alloy_impl.rs:36-58` `tx_request_to_alloy`).
**Fix sketch**: Annotate Doc 17 §"Supporting Types" that the shipping crate uses string-typed representations and converts at backend boundaries; keep the alloy snippet as illustrative of what an alloy consumer would see.

---

## F.05 — `ChainError` has 7 variants, not the documented 7 (Doc 17 §"Error Types")

**Status**: DONE
**Severity**: —
**Doc claim**: `ChainError` enumerates `Rpc, Timeout, Offline, InsufficientFunds, NonceGap, InvalidAddress, Unsupported` — seven variants.
**Reality**: `types.rs:121-154` declares exactly those seven: `Rpc(String)` (`:125`), `Timeout(String)` (`:128`), `Offline` (`:131`), `InsufficientFunds { have, need }` (`:134-139`), `NonceGap { expected, got }` (`:141-147`), `InvalidAddress(String)` (`:150`), `Unsupported(String)` (`:153`). `type ChainResult<T> = Result<T, ChainError>` at `:157`. Error messages at `:123-154` contain the expected substrings; test `chain_error_display_messages` at `:181-200` pins them.

---

## F.06 — MockChainClient + MockChainWallet + paired_mocks ship and work (Doc 17 §"Mock Client")

**Status**: DONE
**Severity**: —
**Doc claim**: Mock implementations allow tests to set up specific chain states and verify agent code handles them. `paired_mocks(balance) -> (client, wallet)` wires them together.
**Reality**: `mock.rs` is 719 LOC of real mock. `MockChainClient::local()` at `:62-78` builds a seeded genesis block, `with_chain_id` at `:81-85`, `with_call_result` at `:88-96`, `push_block` at `:100-102`, `mine_empty_block` at `:107-121`, `insert_receipt` at `:124-129`, `insert_log` at `:132-134`, `insert_storage` at `:137-148`. `MockChainWallet::funded(balance)` at `:259-271` + `pair_with(client)` at `:294-296`. `paired_mocks(balance)` at `:403-408` returns the pair already wired. `impl ChainClient for MockChainClient` (`:151-233`) and `impl ChainWallet for MockChainWallet` (`:299-396`) cover every trait method. The mock wallet simulates balance deduction (`:335`), nonce increment (`:339`), nonce-gap checks (`:326-333`), insufficient-funds errors (`:318-322`), and even emits a witness log when the tx data carries the `WITNESS_MARKER` (`:340-348`) so the `ChainWitnessEngine` roundtrip test can pass. Tests at `:410-719` cover 25+ scenarios including `paired_mocks_wire_wallet_to_client` (`:661`), `wait_for_receipt_polls_until_match` (`:679`), `wait_for_receipt_times_out_when_not_mined` (`:700`).

---

## F.07 — AlloyChainClient (live RPC backend) ships feature-gated (Doc 17 §"Live RPC Client", Doc 24 §"Not yet built §R1")

**Status**: DONE (Doc drift — Doc 24 says NOT BUILT)
**Severity**: MEDIUM
**Doc claim**: Doc 17 `:233-242` says the live RPC client uses alloy but does not promise a shipped impl. Doc 24 §"What Is Not Yet Built" lists `Live RPC client implementation with alloy (§R1)` as Tier 6 deferred.
**Reality**: `crates/roko-chain/src/alloy_impl.rs` is **327 LOC** of a shipping `AlloyChainClient` + local-key `AlloyWallet` implementation. `lib.rs:10-11` feature-gates it: `#[cfg(feature = "alloy-backend")] pub mod alloy_impl;`. `AlloyChainClient::http(rpc_url)` at `alloy_impl.rs:70-78` builds a `DynProvider` via `ProviderBuilder::new().connect_http(url).erased()`; the full `impl ChainClient for AlloyChainClient` at `:86-...` covers all eight trait methods (block_number, get_block_header, get_receipt, get_logs, get_storage_at, eth_call, chain_id, name). `crates/roko-demo/` already consumes this via its `alloy-backend` feature (per CLAUDE.md "What exists" table). Doc 24 is **stale** — the §R1 "Not Yet Built" line contradicts the shipping crate.
**Fix sketch**: Move "Live RPC client" in Doc 24 §"What Is Not Yet Built" up to §"What Is Built" row with a `crates/roko-chain/src/alloy_impl.rs` anchor. Note it requires the `alloy-backend` feature flag.

---

## F.08 — `ChainWitnessEngine` + `witness_on_chain` / `verify_on_chain` ship (not in Doc 17 table)

**Status**: DONE (undocumented surface)
**Severity**: LOW
**Doc claim**: Neither Doc 17 nor Doc 24 mentions `ChainWitnessEngine`. Doc 18 §"Chain Witness" is documented in topic 15 only.
**Reality**: `witness.rs:17-91` defines `ChainWitnessEngine` with `.witness_on_chain(attestation, wallet, client)` (`:40-66`) and `.verify_on_chain(attestation, client)` (`:70-90`). Free-function shims `witness_on_chain` / `verify_on_chain` at `:93-112`. Constants `WITNESS_MARKER = b"roko.attestation.witness:"` (`:11`), `WITNESS_TOPIC = "roko.attestation.witness"` (`:12`), `WITNESS_TO = "0x00000000000000000000000000000000000000c0"` (`:13`). Roundtrip test `witness_roundtrip_records_chain_attestation` at `:168-190` exercises the end-to-end path: sign an Engram → `witness_on_chain` → receipt → `chain_attestation` field populated → `verify_on_chain` returns `true`. Re-exported from `lib.rs:30`. Also consumed by `MockChainWallet::sign_and_submit` at `mock.rs:340-348` which strips `WITNESS_MARKER` from tx data and emits a `WITNESS_TOPIC` log so `verify_on_chain` can find it.
**Fix sketch**: Add a `ChainWitnessEngine` row to Doc 24 §"What Is Built" and cross-link from Doc 15 §"ChainWitness" (which documents a different, richer witness design; the shipping one is narrower).

---

## F.09 — `WalletGate` is a real Gate, not a stub (Doc 24 §"What Is Built")

**Status**: DONE (Doc drift — Doc 24 says STUB)
**Severity**: MEDIUM
**Doc claim**: Doc 24 row: `WalletGate stub | roko-chain/src/lib.rs | **Stub** | Interface defined, verification logic not implemented`.
**Reality**: `gate/wallet_gate.rs` is **523 LOC** of real implementation. `WalletGate` struct at `:63-70` holds `Arc<dyn ChainWallet>` + `Arc<dyn ChainClient>` + `WalletGateConfig`. `WalletCheck` enum at `:77-105` has `BalanceOk`, `InsufficientBalance`, `NonceGap`, `Unsupported` variants. `WalletGate::verify` pulls `TxRequest` from signal body, reads `wallet.balance()` + `wallet.nonce()`, computes `value + gas_limit * max_fee_per_gas`, and returns a `Verdict`. `parse_tx_from_signal` helper re-used by `TxSimGate`. The only surface that is still a stub is the `require_allowance_for` Permit2 check, explicitly flagged as reserved at `:10-14` and `:37-41`. The wallet-gate verification logic for balance and nonce IS implemented and tested.
**Fix sketch**: Update Doc 24 "WalletGate stub" row to read `WalletGate | roko-chain/src/gate/wallet_gate.rs | Built (Permit2 pending) | Balance + nonce checks implemented with tests; allowance check reserved.`

---

## F.10 — `TxSimGate` is a real Gate with pluggable `TxSimulator` (Doc 24 §"What Is Built")

**Status**: DONE (Doc drift — Doc 24 says STUB)
**Severity**: MEDIUM
**Doc claim**: Doc 24 row: `TxSimGate stub | roko-chain/src/lib.rs | **Stub** | Interface defined, verification logic not implemented`.
**Reality**: `gate/tx_sim_gate.rs` is **448 LOC** of real implementation. `TxSimulator` trait at `:72-77` is the pluggable backend seam (`async fn simulate(&self, tx: &TxRequest) -> Result<SimulationOutcome, ChainError>`). `SimulationOutcome { success, gas_used, revert_reason }` at `:35-42`, constructors `ok(gas_used)` / `reverted(gas_used, reason)` at `:46-63`. `TxSimGateConfig { gas_buffer_pct, require_success }` at `:81-96` defaults to `(10, true)` — the "10% gas buffer, fail on revert" default the doc snippet at Doc 18 `:77` describes. `TxSimGate` implements `Gate` from `roko_core` (`:21`). `MockTxSimulator` ships (re-exported via `gate/mod.rs:29-31`). The verdict rules at `:9-18` exactly match the gate-mod.rs comment: "simulator errors → fail; require_success + revert → fail; `gas_used > gas_limit * (1 - gas_buffer_pct/100)` → fail; otherwise pass with gas usage in detail."
**Fix sketch**: Update Doc 24 "TxSimGate stub" row to read `TxSimGate | roko-chain/src/gate/tx_sim_gate.rs | Built | Pluggable TxSimulator trait; 10% gas buffer + require_success default.`

---

## F.11 — mirage-rs ships as a full EVM simulator on revm (Doc 18 §"Abstract", §"Module Architecture")

**Status**: DONE
**Severity**: —
**Doc claim**: mirage-rs is built on revm, passes all official Ethereum test suites, supports local/fork/scenario modes, and provides identical execution semantics to production EVM nodes. Doc 18 §"Module Architecture" lists `lib.rs`, `cow/`, `events/`, `fork/`, `integration/`, `provider/`, `rate_limit/`, `replay/`, `resources/`, `rpc/`, `scenario/` — eleven core modules.
**Reality**: `apps/mirage-rs/src/` contains exactly the 11 non-chain modules the doc lists — `lib.rs`, `cow.rs`, `events.rs`, `fork.rs`, `integration.rs`, `provider.rs`, `rate_limit.rs`, `replay.rs`, `resources.rs`, `rpc.rs`, `scenario.rs` (note: shipped as flat `.rs` files in the current tree rather than subdirectories; the doc's "module/" notation is conceptual). Cargo deps at `apps/mirage-rs/Cargo.toml:25-51` pull in `revm`, `alloy-primitives`, `axum`, `jsonrpsee`, `tokio`, `futures`, `serde`, `reqwest` — the full EVM + RPC stack. `lib.rs` exposes `TransactionRequest`, `MirageError` (matching Doc 18 `:50-72`). The "141 tests" line in Doc 18 `:219` is close but slightly stale — repo-wide mirage-rs tests ran at **243** in the parity report (CLAUDE.md "Built" row). Both numbers count the same test suite; the 243 is newer.

---

## F.12 — mirage-rs chain extensions ship as a scaffold under `chain` feature (Doc 18 §"Korai Chain Extensions", Doc 24 §"mirage-rs chain extensions")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 18 §"Korai Chain Extensions" promises emulated `Identity Registry (0xA100)`, `Reputation Registry (0xA200)`, `Validation Registry (0xA300)`, and HDC precompile at `0xA01`. Doc 24 calls this "Scaffold" — module structure exists, implementation incomplete.
**Reality**: `apps/mirage-rs/src/chain/mod.rs:24-57` declares nine submodules — `agent, hdc_index, hnsw, insight, knowledge, pheromone, prediction, projection, task` — collectively **~4,530 LOC**. These match Doc 18's scaffold claim but the _content_ is not the doc's registry emulation: the modules implement a **generic agent-coordination knowledge substrate**, not a 1-to-1 emulation of the three Solidity registries the doc describes. Concrete mismatches:
- `chain/agent.rs:1-450` defines `AgentEntry`, `AgentRegistry`, `AgentStats`, `AgentTrace`, `CognitivePhase`, `SkillConfig` — none of which map to the ERC-721 soulbound Passport spec (see B.02).
- `chain/knowledge.rs:1-583` + `chain/insight.rs:1-483` implement a `KnowledgeStore` / `InsightEntry` flow (post / confirm / challenge / decay / search), not a work-proof Validation Registry.
- `chain/pheromone.rs:1-409` is a stigmergy / decay signal layer not mentioned by Doc 18's registry table.
- No precompile address `0xA01` is registered anywhere — the HDC operations live in `chain/hdc_index.rs` (brute-force) and `chain/hnsw.rs` (HNSW) as **library code**, not as an EVM precompile.
- `chain/mod.rs:17-22` is candid: "The design docs describe a token named `GNOS` with bespoke tokenomics. This POC intentionally collapses that to plain ETH (wei). Callers that need richer demurrage / slash schedules can build those on top of the plain `stake_wei` / `base_reward_wei` scalars." The module acknowledges it is NOT the documented chain.

So the scaffold is **real and substantial** (~4.5K LOC), but it implements a *different* design from the one Doc 18 §"Korai Chain Extensions" describes. See G.NN for scaffold drift.
**Fix sketch**: Doc 18 §"Korai Chain Extensions" should be split into two subsections: (a) "What `apps/mirage-rs/src/chain/` currently implements" (agent-coordination substrate, HDC index, pheromone decay, knowledge store) and (b) "Planned Korai-specific extensions (§Q1-Q5)" (HDC precompile, registry emulation at 0xA100/0xA200/0xA300, `roko_bridge` ChainClient/Wallet, TxSimGate integration). Do not let readers infer that the emulated registries already exist.

---

## F.13 — `roko_bridge` already implements `Gate` and `Substrate` traits (Doc 24 §"Not yet built §Q4")

**Status**: DONE (Doc drift — Doc 24 says NOT BUILT)
**Severity**: MEDIUM
**Doc claim**: Doc 24 §"Not yet built §Q4": `roko_bridge implementations of ChainClient and ChainWallet` are Tier 6 deferred.
**Reality**: `apps/mirage-rs/src/roko_bridge/` ships today behind the `roko` feature (`Cargo.toml:109-111`). Four modules:
- `mod.rs:1-82` — `map_kind` helper + re-exports
- `simulation_gate.rs:1-330` — `SimulationGate` + `SimulationGateConfig`, implementing `roko_core::traits::Gate`
- `hdc_substrate.rs:1-289` — `HdcSubstrate`, implementing `roko_core::traits::Substrate`
- `chain_substrate.rs:1-394` — `ChainSubstrate` + `ChainSubstrateConfig`, a richer substrate wired to the chain knowledge layer
- `subscription/` — `InsightBus`, `PheromoneBus`, `BackpressurePolicy`, `MpscSink`, `BroadcastSink`, `VecSink`, `SubscriptionStats`

Doc 24 is wrong: §Q4 is what `roko_bridge` already does. What is **not** implemented is `ChainClient` / `ChainWallet` trait impls backed by mirage-rs — those would be a separate `MirageChainClient` / `MirageChainWallet` (see F.14). The roko_bridge ships the `Gate` and `Substrate` impls, which the doc under-claims.
**Fix sketch**: Doc 24 §Q4 should split into "§Q4a roko_bridge Gate/Substrate impls (DONE — `apps/mirage-rs/src/roko_bridge/`)" and "§Q4b MirageChainClient/MirageChainWallet bridging (NOT DONE — see F.14)".

---

## F.14 — No `MirageChainClient` exists — mirage-rs is not consumable via `roko-chain` traits (Doc 17 §"mirage-rs Client", Doc 18 §"Integration with roko-chain")

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 17 `:244-255` shows `pub struct MirageChainClient { mirage: Arc<MirageInstance>, chain_id: u64 }` with an `impl ChainClient`. Doc 18 §"Integration with roko-chain" `:160-190` expands the same snippet and claims "Agent code uses `ChainClient` and `ChainWallet` trait objects, so switching from mirage-rs to a live chain requires only changing the implementation — no agent code changes."
**Reality**: `Grep 'MirageChainClient' crates/roko-chain apps/mirage-rs` returns zero matches. `apps/mirage-rs/src/roko_bridge/mod.rs:1-82` does NOT export a `MirageChainClient` or `MirageChainWallet`; it re-exports `ChainSubstrate`, `HdcSubstrate`, `SimulationGate`, and subscription primitives. The mirage→`ChainClient` bridge does not exist. Concretely, a roko agent cannot today call `mirage_rs::MirageInstance` and receive an `Arc<dyn ChainClient>`. Doc 17 and Doc 18 both show code that does not compile.
**Fix sketch**: Either (a) implement `apps/mirage-rs/src/roko_bridge/chain_client.rs` exposing `pub struct MirageChainClient { mirage: Arc<MirageInstance>, chain_id: u64 }` with an `impl ChainClient` (matching Doc 17 `:244-255`), or (b) update Doc 17 §"mirage-rs Client" to mark the snippet `Design — not yet implemented` and point callers at the currently-shipping `ChainSubstrate` / `HdcSubstrate` bridge surfaces (which are NOT `ChainClient` but serve a different purpose).

---

## F.15 — `chain_rpc.rs` exposes custom RPC methods but no `korai_*` namespace (Doc 01 §"Custom korai_ RPC methods", Doc 18 §"Korai RPC Methods")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 18 `:146-156` lists `korai_registerPassport(...)`, `korai_getPassport(id)`, `korai_submitKnowledge(...)`, `korai_queryKnowledge(...)`, `korai_submitFeedback(...)`, `korai_getReputation(id, dom)` as mirage-rs RPC methods.
**Reality**: `apps/mirage-rs/src/chain_rpc.rs` is **2,085 LOC** — real, substantial. But `Grep 'korai_' apps/mirage-rs/src/chain_rpc.rs` returns zero matches. The file instead exposes a `chain_*` / `eth_*` namespace (per the file comment at the top; the RPC methods handle fork-EVM concerns plus the knowledge/pheromone layer via `chain_postInsight`, `chain_querySimilar`, etc., not the Passport/Reputation/Feedback methods the doc lists). The "~74K LOC" and `korai_*` method list in the plan/docs are both wrong — the file is 2K LOC and uses a different prefix.
**Fix sketch**: Update Doc 01 §"Custom korai_ RPC methods" and Doc 18 §"Korai RPC Methods" to list the actual `chain_*` methods shipping today, mark the `korai_*` prefix as a planned post-chain-launch rename, and explicitly note the 6-method list is Tier 6 deferred because `Passport` + `Reputation` have no implementation yet.

---

## F.16 — mirage-rs feature flags (Doc 18 §"Korai Chain Extensions")

**Status**: DONE
**Severity**: —
**Doc claim**: "When the `chain-extensions` feature flag is enabled, mirage-rs emulates Korai-specific functionality" (Doc 18 `:122-123`). Doc 18 tables separate registry / precompile / RPC emulation.
**Reality**: The feature flag is not named `chain-extensions` — `Cargo.toml:88-111` declares `default = ["binary", "chain", "legacy-api"]`, with `chain = ["dep:roko-primitives"]` (`:104`), `legacy-api = ["chain"]` (`:107`), `roko = ["chain", "dep:roko-core", "dep:async-trait"]` (`:111`). Three extension knobs: `chain` (HDC + knowledge + stigmergy), `legacy-api` (REST surface incl. ISFR proxy), `roko` (`Gate`/`Substrate` bridge impls). The doc's `chain-extensions` is a hyphenated misnomer for `chain`. Otherwise the gating story matches.
**Fix sketch**: Rename `chain-extensions` → `chain` throughout Doc 18 and explicitly enumerate the three-feature matrix so readers know which extension group requires which flag.

---

## F.17 — mirage-rs tests and `legacy-api` REST surface (Doc 18 §"Test Coverage", §"Module Architecture")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 18 `:219` says "141 tests". Doc 18 §"Module Architecture" lists `http_api/` under "Chain Extensions" but does not enumerate routes.
**Reality**: Current repo-wide mirage-rs test count is **243** (per CLAUDE.md "What exists" row). `apps/mirage-rs/src/http_api/` ships as a real REST surface gated behind `legacy-api`: 10 files, ~3.9K LOC — `mod.rs` (448), `agent.rs` (333), `isfr.rs` (58), `knowledge.rs` (727), `pheromone.rs` (508), `prediction.rs` (311), `skills.rs` (149), `task.rs` (555), `topology.rs` (145), `ws.rs` (308). The ISFR endpoints at `http_api/isfr.rs:1-58` are **proxy-only** — `isfr_current` and `isfr_history` forward to `http://localhost:8546/v1/isfr/...` — there is no local ISFR logic (see G.NN).
**Fix sketch**: Update Doc 18 §"Test Coverage" to 243 tests. Add a §"Legacy REST API" subsection enumerating the ten `http_api/*` routes and flagging `http_api/isfr.rs` as a **proxy to the upstream ISFR service**, not a local implementation of QP clearing.

---

## F.18 — Simulation confidence / differential / formal-verification features (Doc 18 §"Simulation Fidelity Guarantees")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 18 §"Simulation Fidelity Guarantees" defines `SimulationConfidence` (`:264-304`), `DifferentialTest` + `DiffComparison` (`:318-346`), `KoraiInvariantTest` + `KoraiInvariant` enum (`:359-381`), and integration with Certora / Halmos / Kontrol formal-verification tools.
**Reality**: `Grep 'SimulationConfidence|ConfidenceFactors|DifferentialTest|KoraiInvariantTest|KoraiInvariant' apps/mirage-rs` returns zero matches. None of the confidence-scoring, differential-testing, or formal-verification structs exist. The §"Simulation Fidelity Guarantees" block is pure design. The shipping simulation gate (`roko_bridge/simulation_gate.rs:1-330`) returns a simple `Verdict::pass / fail` based on tx revert + gas-limit thresholds — no confidence score.
**Fix sketch**: Add a `Design — Phase 2+` banner to Doc 18 §"Simulation Fidelity Guarantees" + §"Simulation-to-Mainnet Migration Testing". Keep the tables of what revm CAN / CANNOT faithfully simulate (those are factual). Move the `SimulationConfidence` / `DifferentialTest` / `KoraiInvariant` snippets behind that banner.

---

## F.19 — `roko-chain-watcher` app ships as a long-running observer (not in Doc 17/18)

**Status**: DONE (undocumented surface)
**Severity**: LOW
**Doc claim**: Neither Doc 17 nor Doc 18 mentions a chain-watcher binary. Doc 15 §"ChainWitness" describes an observer pipeline but no shipping binary.
**Reality**: `apps/roko-chain-watcher/` ships as a **separate binary app**, ~2,931 LOC across 7 files — `block_observer.rs`, `config.rs`, `known_addresses.rs`, `main.rs`, `reactions.rs`, `rpc_client.rs`, `watcher.rs`. It subscribes to a chain (most likely a mirage instance), reacts to observed events through `reactions.rs`, and integrates with the `roko-serve` HTTP control plane per CLAUDE.md. No unit tests in the crate. Production-relevant: if the docs ever claim "chain observation is Tier 6 deferred," they contradict this binary.
**Fix sketch**: Add a §"Chain Watcher" subsection to Doc 15 (or a new Doc 25) pointing at `apps/roko-chain-watcher/`, describing its role (long-running `eth_newHeads` + log subscription, event-driven `reactions.rs`), and cross-linking with `ChainWitnessEngine`.

---

## F.20 — `roko-demo` consumes the chain stack end-to-end (Doc 17 §"Implementations" — live backend)

**Status**: DONE (undocumented example)
**Severity**: LOW
**Doc claim**: Doc 17 §"Live RPC Client" shows a `RpcChainClient` but does not cite a demo or a consuming crate.
**Reality**: CLAUDE.md "What exists" row: `roko-demo chain integration | crates/roko-demo/ | Built | Full chain wallet + client demo via alloy-backend feature`. Plus the Solidity contracts at `contracts/src/*.sol` (see A.NN) are deployed / tested against the chain stack; `contracts/script/Deploy.s.sol` exists. So the end-to-end path from `ChainClient + ChainWallet` traits → `alloy_impl` → on-chain Solidity bytecode is **exercised by the demo today**.
**Fix sketch**: Add a §"Demo integration" cross-link in Doc 17 pointing readers at `crates/roko-demo/` + `contracts/` for a concrete end-to-end example.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 13 (F.01 crate layout, F.02 ChainClient, F.05 ChainError, F.06 mocks, F.07 AlloyChainClient, F.08 WitnessEngine, F.09 WalletGate, F.10 TxSimGate, F.11 mirage EVM, F.13 roko_bridge impls, F.16 feature flags, F.19 chain-watcher, F.20 demo) |
| PARTIAL | 5 (F.03 ChainWallet method count, F.04 supporting types typing, F.12 mirage chain extensions scope, F.15 chain_rpc vs `korai_*`, F.17 tests + legacy-api) |
| NOT DONE | 2 (F.14 MirageChainClient missing, F.18 confidence/differential) |

Doc 17 and Doc 18 are the two "Implementation: Built" chapters in topic 08,
and the live code substantially honors that claim. The biggest drift is in
the **opposite direction** from the rest of the chain topic: Doc 24
understates what ships (F.07 AlloyChainClient, F.09 WalletGate, F.10
TxSimGate, F.13 roko_bridge impls all live at real LOC counts, but Doc 24
marks them Stub / Not Yet Built). The second-biggest drift is in the
"mirage-rs Chain Extensions" claim (F.12) — the scaffold is large and real,
but it implements an agent-coordination knowledge substrate, not the
Passport-Reputation-Validation registry-emulation set Doc 18 promises.
Only one item is cleanly NOT DONE: F.14 `MirageChainClient` — the bridge
that Doc 18 "Integration with roko-chain" relies on never shipped.

## Agent Execution Notes

### F.07 / F.09 / F.10 / F.13 — Doc 24 Under-Claim Fix (1 pass)

Best use of this section in batch `08`:

1. update Doc 24 rows for `AlloyChainClient`, `WalletGate`, `TxSimGate`, `roko_bridge` from "stub / not built" to "Built (path: ...)"
2. do not re-audit the shipping code; the parity entries here are the audit
3. leave §R1, §R5, §Q4a as DONE and §Q4b (Mirage\*Client bridge) as the surviving gap

### F.12 / F.14 / F.18 — Scoping Calls, Not Code

F.12 decides whether the chain scaffold **is** the promised "Korai Chain
Extensions" under a different name or is a separate thing that needs its
own doc chapter. F.14 is the only real code gap in F. F.18 is a pure
frontier-banner cleanup.

Acceptance criteria for this section:

- Doc 24 no longer under-claims the four shipping "stub" items,
- Doc 18 §"Korai Chain Extensions" either re-scopes to match the shipping scaffold or explicitly labels the scaffold as a separate design,
- a later agent either builds `MirageChainClient` or demotes Doc 17 §"mirage-rs Client" to `Design — not yet implemented`.
