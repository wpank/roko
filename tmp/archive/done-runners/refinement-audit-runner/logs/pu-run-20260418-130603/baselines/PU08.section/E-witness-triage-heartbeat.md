# E — Witness, Triage, and Heartbeat (Docs 15, 16, 19)

Parity of the three "chain-agent runtime" chapters: ChainWitness event
watching pipeline, Triage with MIDAS-R + curiosity scoring, and the
9-step chain-agent heartbeat mapping onto the universal Synapse loop.

Two shipping surfaces are adjacent:

1. `crates/roko-chain/src/witness.rs` — **attestation witness engine**, different from Doc 15's block-observation WitnessEngine (see F.08).
2. `apps/roko-chain-watcher/` (~2,931 LOC) — a real long-running observer that polls a chain via HTTP, applies hand-written reaction rules, and deposits insights / pheromones — but without the Binary Fuse filter, Roaring Bitmap, or MIDAS-R pieces Docs 15-16 specify.

Generated 2026-04-16.

---

## E.01 — `WitnessEngine` with Binary Fuse filter + `ArcSwap` swap is absent (Doc 15 §"WitnessEngine", §"Binary Fuse Pre-Screening")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `WitnessEngine { watch_filter: Arc<ArcSwap<BinaryFuse8>>, subscription: Arc<WsProvider>, query_pool: deadpool::Pool<WsProvider>, seen_blocks: Arc<Mutex<RoaringBitmap>>, latest_block: AtomicU64 }`. Binary Fuse filter achieves 8.7 bits/entry with <1% FPR (Lemire et al. 2022). Zero false negatives.
**Reality**: `Grep 'BinaryFuse|xorf|ArcSwap.*Filter|watch_filter' crates/ apps/ --include=*.rs` returns zero matches. No `xorf` crate dependency; `grep -n 'xorf\|roaring' apps/*/Cargo.toml crates/*/Cargo.toml` is empty. The shipping `apps/roko-chain-watcher/src/block_observer.rs:1-60` polls `eth_getBlockByNumber` over HTTP JSON-RPC (not `eth_subscribe("newHeads")`), stores `HashSet` / `VecDeque` for dedup (`:11`), and computes gas / base-fee / saturation per block **without** a Binary Fuse filter. The `seen_blocks: Arc<Mutex<RoaringBitmap>>` gap-detection surface is absent too.
**Fix sketch**: Doc 15 should carry a `Design — Phase 2+` banner. Cross-link `apps/roko-chain-watcher/src/block_observer.rs` as the shipping precursor (HTTP polling + hand-written heuristics) whose WebSocket + Binary Fuse upgrade is the Doc 15 work item.

---

## E.02 — Dedicated subscription + separate fetch pool (`deadpool`) absent (Doc 15 §"Block Ingestion Pipeline")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Subscription connection is dedicated to `eth_subscribe("newHeads")`; block fetches fan out across a separate `deadpool::Pool<WsProvider>` so burst activity cannot starve the subscription.
**Reality**: `apps/roko-chain-watcher/src/rpc_client.rs` is a single-connection HTTP client (one reqwest client, no pool). `Grep 'deadpool|WsProvider|ws_subscribe|eth_subscribe' crates/ apps/ --include=*.rs` returns zero matches. No WebSocket path at all.

---

## E.03 — Gap detection via Roaring Bitmap is absent (Doc 15 §"Gap Detection")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `seen_blocks: Arc<Mutex<RoaringBitmap>>` tracks processed blocks; gap-detection scan fires if the bitmap has holes, triggering a backfill.
**Reality**: `Grep 'RoaringBitmap|roaring::' crates/ apps/ --include=*.rs` returns zero matches. No `roaring` crate dependency. The shipping watcher uses `VecDeque` + `AtomicU64` (`block_observer.rs:11-13`) for recent-block tracking but does not run a gap-detection scan.

---

## E.04 — Long-running chain observer ships (shipping; adjacent to Doc 15)

**Status**: DONE (adjacent surface; not Doc 15's WitnessEngine)
**Severity**: —
**Doc claim**: Doc 15 §"Abstract" says the witness is "the agent's eyes on the chain". Doc 15's framing does not name a binary or an app name.
**Reality**: `apps/roko-chain-watcher/` ships as the real long-running observer, ~2,931 LOC per CLAUDE.md "What exists" table. 7 source files:

| File | What |
|---|---|
| `main.rs` | CLI entry point + tokio runtime |
| `watcher.rs` | Main polling loop (`poll_interval_ms`, rate-limited reactions, exit on `max_events`) |
| `block_observer.rs` | `eth_getBlockByNumber` poll, `ObservedBlock`, `LargeTransfer` detection |
| `rpc_client.rs` | HTTP JSON-RPC client (`MirageRpcClient`) |
| `reactions.rs` | Five hand-written pattern rules (see E.06) |
| `known_addresses.rs` | Contract address / method-selector catalog |
| `config.rs` | `WatcherCli` config |

Integrates with the `roko-serve` HTTP control plane (CLAUDE.md table). No unit tests in the crate per CLAUDE.md.
**Fix sketch**: Doc 15 should explicitly acknowledge the shipping `apps/roko-chain-watcher/` observer as a simpler precursor to `WitnessEngine`. Note the gap (HTTP polling vs WS; hand-rules vs Binary Fuse; no Roaring Bitmap).

---

## E.05 — Connection pool with HTTP fallback absent as described (Doc 15 §"Connection Pool with HTTP Fallback")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: The witness uses a WebSocket connection pool with HTTP fallback for degraded mode.
**Reality**: The shipping observer is HTTP-only (no WS); there is no fallback because there is no primary.

---

## E.06 — Reaction rules are 5 hand-written heuristics, not MIDAS-R (Doc 16 §"Four-Stage Pipeline")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 16 §"Four-Stage Pipeline" specifies (1) rule-based fast filter → (2) MIDAS-R anomaly detection (Bhatia et al. 2020) → (3) contextual enrichment → (4) HDC / Bayesian curiosity scoring. No LLM in this path.
**Reality**: `apps/roko-chain-watcher/src/reactions.rs:1-40+` implements **stage 1 only**: five hand-written rules:
1. Threat pheromone intensity > 0.7 + no similar warning → post warning insight
2. Opportunity pheromone intensity > 0.6 → post `strategy_fragment` insight
3. Wisdom pheromone → confirm matching insight (top hit by similarity)
4. Insight content containing anti-pattern keywords (`WRONG`, `BUG`, `INCORRECT`) with ≥1 confirmation → challenge
5. Every poll with ≥1 observation → deposit wisdom pheromone summarizing state

No MIDAS-R anomaly detection (`Grep 'MIDAS|midas_r|anomaly_score_chain' crates/ apps/ --include=*.rs` returns zero matches). No Bayesian curiosity. The stage-1 rule-based filter is real, hand-written, and executes today.
**Fix sketch**: Update Doc 16 §"Four-Stage Pipeline" to split §1 (shipping as `reactions.rs`) from §§2-4 (Phase 2+). Keep the MIDAS-R / HDC / Bayesian math as reference design.

---

## E.07 — Curiosity scoring (HDC + Bayesian) absent (Doc 16 §"Curiosity Scoring")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Curiosity score combines HDC-based novelty against stored knowledge + Bayesian surprise against prior expectation. Used to rank events for agent attention.
**Reality**: `Grep 'curiosity_score|bayesian_surprise|novelty_score' crates/ apps/ --include=*.rs` returns zero matches. The shipping `reactions.rs` uses threshold checks on pheromone intensity, not curiosity scoring.

---

## E.08 — Contextual enrichment stage (price feeds, oracles) absent (Doc 16 §"Contextual Enrichment")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Stage 3 enriches triage events with oracle / price / liquidity context from registered providers. Runs only for events that passed anomaly detection.
**Reality**: `apps/roko-chain-watcher/src/known_addresses.rs` provides a static catalog of contract addresses and method selectors for decoding — that is a form of enrichment, but not the Doc 16 "oracle / price / liquidity context" model. No Chainlink / TWAP / DEX-liquidity reader in the watcher.

---

## E.09 — 9-step chain heartbeat maps onto the shipping 7-phase Synapse loop (Doc 19 §"The 9-Step Mapping")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Chain-agent heartbeat extends the universal 6-step Synapse loop (`PERCEIVE → EVALUATE → ATTEND → ACT → VERIFY → ADAPT`) with SIMULATE + VALIDATE, producing a 9-step mapping: `OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT`.
**Reality**: CLAUDE.md states "Synapse loop has 7 phases" (versus the doc's claim of 6 verbs). The actual shipping universal loop in `crates/roko-core/src/traits.rs` and consumed by `orchestrate.rs` is described as `query → score → route → compose → act → verify → write → react` — **8 transitions** / 7+1 phases in the universal loop notation. The chain-specific 9-step heartbeat is a **conceptual expansion** of this loop to include `SIMULATE` (mirage-rs pre-flight) and `VALIDATE` (PolicyCage / limits). Neither SIMULATE nor VALIDATE have dedicated chain-agent entries in `orchestrate.rs`:
- SIMULATE has a shipping analogue in `TxSimGate` (`crates/roko-chain/src/gate/tx_sim_gate.rs:1-448`) which runs inside the gate pipeline.
- VALIDATE has a shipping analogue in `WalletGate` (`gate/wallet_gate.rs:1-523`) and in the broader 7-rung gate pipeline (CLAUDE.md "Wired").

So the heartbeat mapping is partially wired via gates: simulation and validation are real runtime activities, but they are gate-pipeline steps, not distinct agent-heartbeat steps.
**Fix sketch**: Doc 19 §"The 9-Step Mapping" should cross-link SIMULATE → `TxSimGate` and VALIDATE → `WalletGate` + PolicyCage. Clarify that the 9 steps are a didactic expansion of the universal loop; they are not 9 separate runtime phases in `orchestrate.rs`.

---

## E.10 — PolicyCage (VALIDATE step) has no chain-specific implementation (Doc 19 §"Step 6: VALIDATE")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: VALIDATE step runs PolicyCage — domain-specific constraint enforcement (max position size, allowed counterparties, liquidity thresholds) before executing on-chain.
**Reality**: `Grep 'PolicyCage|policy_cage' crates/ apps/ --include=*.rs` returns zero matches. `crates/roko-agent/src/safety/` exists (per CLAUDE.md) for role-auth + pre/post checks in the tool dispatcher but is not a chain-specific policy surface. No position-size / counterparty / liquidity gates.
**Fix sketch**: Doc 19 §"Step 6: VALIDATE" should point at `crates/roko-agent/src/safety/` as the generic safety layer and mark chain-specific PolicyCage as Phase 2+.

---

## E.11 — Three cognitive speeds (Delta / Theta / Gamma) are partial (Doc 19 §"Three Cognitive Speeds")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Chain agent runs three oscillators: Delta (~1 Hz, fast witness), Theta (~0.1 Hz, deliberative), Gamma (~0.01 Hz, reflective). Each step of the heartbeat runs at a specific frequency.
**Reality**: `crates/roko-conductor/src/stuck_detection.rs:582-584` ships `MetaCognitionHook::frequency() -> OperatingFrequency::Theta` per batch 07 section A.07. So `OperatingFrequency` is a real enum in `roko-conductor`, but it is consumed by the conductor's meta-cognition hook, not by the chain-agent heartbeat. The chain agent does not today run on three distinct oscillators; it uses the generic `conductor.evaluate()` tick path.
**Fix sketch**: Doc 19 should cross-link `OperatingFrequency` in `roko-conductor` as the shared frequency primitive and mark chain-agent-specific Delta / Theta / Gamma binding as Phase 2+.

---

## E.12 — REFLECT step → episode logging is wired (Doc 19 §"Step 9: REFLECT")

**Status**: DONE
**Severity**: —
**Doc claim**: REFLECT step writes an episode with predictions-vs-actuals, inputs, outputs, and agent reflection notes. Feeds the learning system.
**Reality**: CLAUDE.md table: "EpisodeLogger (agent turn recording) | Wired | `.roko/episodes.jsonl` via orchestrate.rs". So the generic reflection / episode recording path is real and runs for every agent, not just chain agents. The chain-specific extension (predicted tx effects vs observed receipt) is not a distinct path yet, but the base REFLECT path does ship.

---

## E.13 — Attestation-anchoring `ChainWitnessEngine` (shipping; different semantics from Doc 15)

**Status**: DONE (different surface; cross-ref F.08)
**Severity**: LOW
**Doc claim**: Doc 15 describes a block-observation `WitnessEngine`. No section of Doc 15 mentions attestation anchoring.
**Reality**: `crates/roko-chain/src/witness.rs:17-91` ships a **different** `ChainWitnessEngine` — see F.08 for full coverage. This engine takes an `Attestation` + `ChainWallet` + `ChainClient`, submits a witness transaction with the 32-byte `witness_hash()` prefixed by `b"roko.attestation.witness:"`, waits for receipt, and writes `ChainAttestation { chain_id, tx_hash, block_number }` into the attestation. The name overlap with Doc 15's `WitnessEngine` is confusing but they are **two different subsystems**: Doc 15 is observational (watching), `roko-chain/src/witness.rs` is assertive (anchoring).
**Fix sketch**: Add an explicit callout to Doc 15 §"Abstract": "Note: `roko-chain/src/witness.rs::ChainWitnessEngine` is a separate subsystem that anchors attestation hashes on-chain; the ChainWitness described here is a block-observation pipeline." Consider renaming one of them in a future pass to reduce confusion.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 3 (E.04 roko-chain-watcher shipping observer, E.12 REFLECT via episode log, E.13 attestation-anchoring witness is a separate shipping surface) |
| PARTIAL | 3 (E.06 reactions.rs stage-1 rules, E.09 9-step heartbeat / gate-pipeline wiring, E.11 OperatingFrequency enum exists but not chain-specific) |
| NOT DONE | 7 (E.01 WitnessEngine + Binary Fuse, E.02 subscription + fetch pool, E.03 Roaring Bitmap gap detection, E.05 HTTP fallback, E.07 curiosity scoring, E.08 contextual enrichment, E.10 PolicyCage) |

Section E has the **most interesting drift** in the chain topic:
the Docs describe a specific architecture (Binary Fuse filter + WS +
Roaring Bitmap + MIDAS-R + 9-step heartbeat) while the codebase
ships a *functionally similar but differently-implemented* observer
(HTTP polling + 5 hand-written rules + generic gates). The shipping
observer is not a stub — it runs, reacts, deposits pheromones, and
posts insights. It just doesn't match the doc's architecture.

The naming overlap between Doc 15's `WitnessEngine` (block observer)
and `roko-chain/src/witness.rs::ChainWitnessEngine` (attestation
anchor) is an active source of reader confusion (E.13); cross-linking
both from Doc 15 §"Abstract" is the cheapest fix.

## Agent Execution Notes

### E.04 / E.06 / E.13 — Acknowledge shipping precursors

Best use of this section in batch `08`:

1. Doc 15 should cross-link `apps/roko-chain-watcher/` as the shipping observer and call out the WS + Binary Fuse upgrade as Phase 2+ work,
2. Doc 15 §"Abstract" should add a callout distinguishing the two `WitnessEngine` surfaces (block-observation vs attestation-anchoring),
3. Doc 16 §"Four-Stage Pipeline" should cite `reactions.rs` for stage 1 and mark §§2-4 Phase 2+,
4. Doc 19 should cross-link SIMULATE → `TxSimGate`, VALIDATE → `WalletGate` + `roko-agent/src/safety/`.

### E.01-E.03 / E.05 / E.07 / E.08 / E.10 — Frontier Banner Pass

Apply `Design — Phase 2+` to the respective subsections.

### E.09 / E.11 / E.12 — Cross-link only

Nothing to write new; just add the cross-links identified above.

Acceptance criteria for this section:

- a reader opening Doc 15 knows the shipping observer is at `apps/roko-chain-watcher/` and is not the Binary Fuse WitnessEngine,
- the two `WitnessEngine` surfaces are no longer confusingly co-named without comment,
- Doc 16 §1 cites the real rule file; Doc 16 §§2-4 are explicitly frontier.
