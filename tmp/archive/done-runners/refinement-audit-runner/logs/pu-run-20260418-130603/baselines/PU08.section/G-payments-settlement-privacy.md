# G — Payments, Settlement, Privacy (Docs 20, 21, 22, 23)

Parity of the four settlement / privacy / market chapters: x402 HTTP
micropayments, ISFR clearing with KKT certificates, Valhalla 4-tier
privacy, knowledge futures market.

All four chapters describe Phase 2+ / P3 frontier work. The only
shipping surface is the **ISFR HTTP proxy** at `apps/mirage-rs/src/
http_api/isfr.rs:1-58`, which is a pass-through to an upstream
service — not a local QP-solver-based clearing implementation. None
of the x402, state-channel, Valhalla-privacy, PSI, or futures
mechanisms exist as Rust code.

Generated 2026-04-16.

---

## G.01 — x402 HTTP 402 + ERC-3009 micropayments absent (Doc 20 §"x402 Protocol")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 20 specifies x402 — "HTTP 402 Payment Required" middleware returning payment instructions, client library that signs ERC-3009 `transferWithAuthorization` messages, server-side settler, no API keys / no accounts. Self-funding agent loop.
**Reality**: `Grep 'x402|HTTP 402|ERC-?3009|transferWithAuthorization' crates/ apps/ --include=*.rs` returns zero matches. No x402 middleware, no ERC-3009 signing, no HTTP 402 response handler. `Grep '402' apps/roko-agent-server/src/*.rs` returns zero matches.
**Fix sketch**: Doc 20 stays `Design — Phase 2+`. Cross-link to future agent-server sidecar work if payment-per-call semantics are added.

---

## G.02 — Agent payment channels / Superfluid streaming absent (Doc 20 §"Payment Channels")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: The 2026-04-13 enhancement pass added state-channel payment channels (Poon & Dryja 2016 / Lightning Network analogue for Korai), Superfluid streaming payments for long-running agent rentals, knowledge attestation structs tying payments to delivered insights.
**Reality**: `Grep 'state_channel|StateChannel|Superfluid|payment_channel|streaming_payment' crates/ apps/ --include=*.rs` returns zero matches. Design-only.

---

## G.03 — 4-level dispute resolution is pure design (Doc 20 §"Dispute Resolution")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Four escalation levels: (1) optimistic settlement, (2) bond escalation, (3) peer jury (Kleros-style, Lesaege et al. 2019), (4) governance override.
**Reality**: `Grep 'peer_jury|Kleros|dispute_resolution|bond_escalation' crates/ apps/ contracts/src/ --include=*.rs --include=*.sol` returns zero matches. `contracts/src/BountyMarket.sol:108-125` `resolve(id, accepted)` is a single-step resolver-call — no escalation.
**Fix sketch**: Phase 2+ banner. Cross-link `BountyMarket.resolve` + `ConsortiumValidator` as the shipping level-1 / level-2 analogues.

---

## G.04 — ISFR HTTP routes are **proxy-only** (Doc 21 §"Clearing Mechanism", Doc 21 §"Abstract")

**Status**: PARTIAL (route surface) / NOT DONE (economic logic)
**Severity**: MEDIUM
**Doc claim**: Doc 21 `:1-3` summary: "ISFR provides collective fact validation and price discovery. The clearing mechanism uses a QP solver with bisection (O(80n)) to find market-clearing prices. Clearing certificates carry KKT optimality proofs verifiable on-chain." Doc 21 banner: `Implementation: Built`.
**Reality**: `apps/mirage-rs/src/http_api/isfr.rs` is **58 LOC** of HTTP **proxy endpoints** that forward `/api/isfr/current` and `/api/isfr/history` to `http://localhost:8546/v1/isfr/...` (`DEFAULT_ISFR_SERVICE_URL` at `:13`). `proxy_isfr(path, query)` at `:31-57` is a plain reqwest forwarder — no QP, no bisection, no KKT certificate generation, no reputation-weighted aggregation, no on-chain submission. The upstream `ISFR_SERVICE_URL` is an external service (likely legacy from bardo-heritage), not a local ISFR implementation. `Grep '\bISFR\b|Valhalla|knowledge_futures|zkproof|zk_proof' crates/ apps/ --include=*.rs` returns **one match** — this file.

The Doc 21 banner "Implementation: Built" is **actively misleading**: the route surface exists but the economic substance does not live in this repo.
**Fix sketch**: Change Doc 21 banner to `Implementation: Proxy-only` and rewrite the abstract to clarify that the QP solver + KKT certificates + reputation-weighted aggregation are Phase 2+ work. Link to `apps/mirage-rs/src/http_api/isfr.rs:1-58` as the shipping **proxy** surface.

---

## G.05 — QP solver with bisection is absent (Doc 21 §"Clearing Algorithm", §"O(80n) bisection")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Bisection-based QP solver running in O(80n) with KKT optimality certificates. Boyd & Vandenberghe 2004 convex optimization foundations.
**Reality**: `Grep 'quadratic_programming|qp_solver|kkt_certificate|bisection_solver' crates/ apps/ --include=*.rs` returns zero matches. No QP solver code; no convex optimization crate dependency (no `osqp`, `clarabel`, `clp`, etc. in any `Cargo.toml`).

---

## G.06 — KKT on-chain certificate verification absent (Doc 21 §"On-Chain Verification")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: KKT certificate verification runs in O(n) on-chain — a Solidity function that confirms a provided clearing solution satisfies the KKT conditions for the submitted orders.
**Reality**: No Solidity file in `contracts/src/` verifies KKT conditions. Phase 2+.

---

## G.07 — Reputation-weighted aggregation at the ISFR level absent (Doc 21 §"Fact Submission", §"Why Intersubjective")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Agent-submitted fact claims are weighted by the agent's reputation in the relevant domain and aggregated into the "intersubjective fact".
**Reality**: No weighting / aggregation code exists. The shipping `WorkerRegistry.sol` tracks single-domain reputation (see D.05) but does not expose a `getWeightedAverage(claims[], domain) -> value` function or similar.

---

## G.08 — Four privacy tiers (Public / Access-Gated / TEE / ZK) are pure design (Doc 22 §"Privacy Tiers")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Four privacy tiers with distinct trust models:
1. **Public** — data in plain text on-chain
2. **Access-Gated** — data encrypted with a capability token owned by approved passports
3. **Confidential (TEE)** — data processed inside SGX/TDX enclaves, only aggregate results published (Costan & Devadas 2016)
4. **Full Sealed (ZK)** — data processed via ZK proofs, nothing leaves the producer

PSI (Private Set Intersection) used for capability matching without revealing the full capability set.
**Reality**: `Grep 'PrivacyTier|Valhalla|private_set|PSI|zk_proof|ZkProof' crates/ apps/ contracts/src/ --include=*.rs --include=*.sol` returns zero matches. No TEE integration (see B.09 — TEE string tokens exist in prompt templates only). No ZK integration. No PSI library.
**Fix sketch**: Doc 22 is entirely `Design — Phase 2+`.

---

## G.09 — TEE aggregation path absent (Doc 22 §"Confidential Tier")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Confidential tier uses FABRIC TEE aggregation (see Doc 07 §"Four-Tier Architecture") to process raw data inside enclaves and publish only aggregates.
**Reality**: See C.01 — FABRIC absent. See B.09 — no TEE surface anywhere.

---

## G.10 — ZK circuits / optimistic fraud proofs / Binius STARKs absent (Doc 22 §"Full Sealed Tier", Doc 03 §"Verifiable HDC")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Sealed tier uses ZK proofs (RISC Zero / SP1) or optimistic fraud proofs for verifiable off-chain computation. Doc 03 adds Binius binary-field STARKs as a promising future direction for verifiable HDC search.
**Reality**: `Grep 'RISC\s?Zero|SP1|fraud_proof|Binius|STARK' crates/ apps/ --include=*.rs` returns zero matches. No ZK crate deps in any `Cargo.toml`. Design-only.

---

## G.11 — Knowledge futures market is P3 deferred (Doc 23 §"Abstract", §"P3 Deferral")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 23 explicitly marks itself as **P3 deferred** — committed knowledge production via staked futures, demand signaling, early withdrawal penalties, market-making function.
**Reality**: `Grep 'knowledge_futures|staked_future|market_maker' crates/ apps/ contracts/src/ --include=*.rs --include=*.sol` returns zero matches. Doc 23 is internally consistent in that its own banner concedes P3 deferral. This is the one G chapter whose doc claim matches the reality.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 0 |
| PARTIAL | 1 (G.04 ISFR HTTP proxy surface exists, economic logic upstream) |
| NOT DONE | 10 (G.01 x402, G.02 state channels, G.03 4-level dispute, G.05 QP solver, G.06 KKT verifier, G.07 reputation-weighted aggregation, G.08 4 privacy tiers, G.09 TEE aggregation, G.10 ZK/fraud/Binius, G.11 futures market) |

Section G is almost entirely **frontier**. The single PARTIAL entry
(G.04) is the most interesting: Doc 21 banner claims "Implementation:
Built" but only **HTTP proxy routes** ship — the QP solver, KKT
certificate generation, and reputation-weighted aggregation live in
an upstream service at `ISFR_SERVICE_URL` (defaulted to
`http://localhost:8546`) that is **not in this repo**. This is the
most acute doc-banner drift in topic 08: a "Built" banner covering a
proxy to a service that is itself design elsewhere.

Doc 23 (knowledge futures) is the honest outlier — its doc marks it
P3 deferred, and the code reflects that honesty (nothing ships).

## Agent Execution Notes

### G.04 — Doc 21 Banner Honesty (1 pass)

Best use of this section in batch `08`:

1. Change Doc 21 banner from `Implementation: Built` to `Implementation: Proxy-only` (or `Design — Phase 2+`, depending on whether the upstream ISFR service is considered part of the Korai plane),
2. Rewrite Doc 21 §"Abstract" to state that the QP / KKT / aggregation lives upstream,
3. Link `apps/mirage-rs/src/http_api/isfr.rs:1-58` as the shipping proxy-only surface.

### G.01-G.03 / G.05-G.11 — Frontier Banner Pass

Apply `Design — Phase 2+` banners uniformly. Doc 23 already concedes
P3 — no change needed there except surfacing the concession more
prominently at the top of the file.

Acceptance criteria for this section:

- Doc 21's "Built" banner no longer misleads readers; the QP solver / KKT / aggregation is explicitly upstream,
- x402 / Valhalla / futures chapters are uniformly marked frontier,
- a later agent grepping `ISFR` / `Valhalla` / `x402` in the codebase finds only the single HTTP proxy file and can correlate that to Doc 21.
