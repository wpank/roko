# D — Job Market and Reputation (Docs 10, 11, 12, 13, 14)

Parity of the five market-layer chapters: Spore job marketplace, Sparrow
power-of-two-choices dispatch, three hiring models, Vickrey reputation
auction, 7-domain reputation system.

Section D is where the **demo Solidity contracts** at `contracts/src/`
matter most. `BountyMarket.sol`, `WorkerRegistry.sol`, and
`ConsortiumValidator.sol` implement **partial, single-domain** versions of
Docs 10 + 14 — real code, real tests (43 test functions across
`contracts/test/`), but scoped more narrowly than the full Korai design.
The mesh of auction mechanisms (sealed-bid Vickrey, VRF selection,
reputation-weighted second-price) is absent.

Generated 2026-04-16.

---

## D.01 — Spore marketplace job lifecycle (POSTED → … → SETTLED) is partially implemented (Doc 10 §"Job Lifecycle", Doc 24 §"3. Marketplace")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 10 §"Job Lifecycle" describes a 9-state flow: `POSTED → BIDDING → ASSIGNED → IN_PROGRESS → SUBMITTED → GATING → VERIFIED → PAID → SETTLED`. Supports Solo / Pair / Consortium / Collective job types. 2% escrow fee + 3% marketplace fee.
**Reality**: `contracts/src/BountyMarket.sol:11-136` is a **6-state** machine with a different vocabulary: `enum State { None, Open, Funded, Assigned, Submitted, Terminal }`. The file comment at `:7-10` honestly frames itself: "ERC-8183 style 4-state programmable escrow. Job lifecycle: Open → Funded → Assigned → Submitted → Terminal." Flow details:
- `postJob(specHash, bounty, deadline, minTier)` at `:59-80` transitions Open → Funded atomically via `bountyToken.transferFrom`
- `assign(id, worker)` at `:84-93` pseudo-VRF — comment at `:10`: "`assign` is pseudo-VRF via blockhash, `resolve` is owner/consortium."
- `submit(id, resultHash)` at `:96-103` worker posts result hash
- `resolve(id, accepted)` at `:108-125` resolver (owner or `ConsortiumValidator`) decides outcome; accept transfers bounty + `workerRegistry.updateReputation(true)`, reject refunds + `updateReputation(false)` + `slash(SLASH_QUALITY_REJECT, 500 bps)`.

So the lifecycle IS real but compressed: no `BIDDING` (no bid phase — direct assignment), no `IN_PROGRESS` / `GATING` / `VERIFIED` / `PAID` — these are collapsed into `Submitted` and `Terminal`. No Solo/Pair/Consortium/Collective job-type distinction at the contract level. No fee-split math. 7 `function test...` entries in `contracts/test/BountyMarket.t.sol`.
**Fix sketch**: Doc 10 should add a §"Current `contracts/src/BountyMarket.sol` implementation" subsection explicitly mapping its 6-state machine to the 9-state target. Flag the collapsed states and the pseudo-VRF assignment as a bounded partial; the full Spore marketplace is post-self-hosting work.

---

## D.02 — Sparrow power-of-two-choices VRF dispatch is absent (Doc 11 §"Power-of-Two-Choices")

**Status**: NOT DONE (with design partial in `BountyMarket.assign`)
**Severity**: LOW
**Doc claim**: Sparrow dispatch uses power-of-two-choices selection (Mitzenmacher 2001) with VRF randomness for Sybil resistance. Target O(log log N) maximum load. Load probes + fallback to auction.
**Reality**: `Grep 'Sparrow|power_of_two|PowerOfTwo' crates/ apps/ contracts/src/` returns **one hit**: `contracts/src/BountyMarket.sol:10` comment calling `assign` "pseudo-VRF via blockhash". The actual `assign` function at `:84-93` takes a worker address directly from the caller — there is no randomness sampling of two candidates, no load probe, no VRF, and no fallback to auction. The "pseudo-VRF" comment aspires to the Sparrow model but the code does not implement it.
**Fix sketch**: Doc 11 stays `Design — Phase 2+`. The "pseudo-VRF" comment in `BountyMarket.sol:10` should be amended to call out the gap explicitly.

---

## D.03 — Three hiring models (Random VRF / Blind Auction / Direct Hire 1.5×) are absent (Doc 12 §"Three Models")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Three hiring primitives: (1) Random VRF (fast, O(1) selection time, any-tier); (2) Blind Auction (3 variants: sealed-bid, second-price, commit-reveal; competitive); (3) Direct Hire with 1.5× premium (Tier 0-1 only). Speed-quality-cost tradeoff table at Doc 12 `:50-70`.
**Reality**: None of the three mechanisms exist. `contracts/src/BountyMarket.sol` is a direct-assignment bounty ("assign worker address") without any selection semantics. No sealed-bid, commit-reveal, or 1.5× premium anywhere. `Grep 'sealed_bid|commit_reveal|direct_hire|auction' crates/ apps/ contracts/src/` returns zero matches on the code side.
**Fix sketch**: Phase 2+ banner. The nearest shipping surface is the generic bounty direct-assign at `BountyMarket.assign`.

---

## D.04 — Vickrey reputation-weighted auction is absent (Doc 13 §"Adjusted Scoring", §"Commit-Reveal")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Adjusted score `s_i = p_i × (1 + (1 - R_i))` where `p_i` is bid and `R_i` is reputation. Winner pays `s_second / (1 + (1 - R_winner))`. Truthful bidding preserved (Myerson 1981). Commit-reveal scheme to prevent bid sniping.
**Reality**: `Grep 'VickreyAuction|adjusted_score|second_price|commit_reveal' crates/ apps/ contracts/src/` returns zero matches. No auction code. The reputation-weighted second-price design has no implementation.
**Fix sketch**: Phase 2+ banner. Cross-link to D.05 for reputation storage (which does ship in a single-domain form).

---

## D.05 — 7-domain reputation is single-domain in shipping code (Doc 14 §"Seven Base Domains", Doc 24 §"2. Reputation Registry")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: 7 base domains (coding, security, research, chain, knowledge, operations, strategy). EMA smoothing with adaptive alpha. 30-day half-life decay. 4 discipline states. Slash rates per violation type. C-factor aggregation (Woolley et al. 2010).
**Reality**: `contracts/src/WorkerRegistry.sol:1-233` implements a **single-domain** version of the EMA reputation model with real formulas and tests:
- `ALPHA_NUM = 200_000` at `:15` (α = 0.2, matches "EMA smoothing with adaptive alpha" but α is fixed, not adaptive)
- `updateReputation(worker, outcome)` at `:115-130` applies `R_new = α·O + (1-α)·R_old` exactly (`:122`)
- `_applyDecay(w)` called at `:118` implements "halves toward 0.5 every 30 days of inactivity" per the file comment at `:7-10`; `DECAY_PERIOD = 30 days` at `:19` matches Doc 14 §"30-day half-life decay"
- Tier thresholds at file comment `:9-10`: `Probation < 350_000 ≤ Standard < 550_000 ≤ Trusted < 800_000 ≤ Elite`
- Slash codes at `:24-26`: `SLASH_MISSED_DEADLINE = 1` (1%), `SLASH_QUALITY_REJECT = 2` (5%), `SLASH_ABANDONMENT = 3` (10%) — three of the violation types Doc 14 enumerates
- Tests: 10 `function test...` entries in `contracts/test/WorkerRegistry.t.sol`

What's missing for full Doc 14 parity: (a) the 7 named domains — the shipping contract has one aggregate `reputation` field, not a `mapping(bytes32 domain => uint256)`; (b) adaptive alpha (shipping α is a constant); (c) 4 discipline states (shipping has 4 tiers but no suspension/probation state machine); (d) C-factor aggregation (single-number scoring, no cross-agent C-factor).
**Fix sketch**: Doc 14 should add a §"Current `contracts/src/WorkerRegistry.sol` implementation" subsection mapping its single-domain EMA + tier + slash to the 7-domain target. Mark the 7-domain split, adaptive alpha, C-factor aggregation, and suspension/probation discipline states as Phase 2+ extensions. Cross-link from Doc 24 §"2. Reputation Registry" to `contracts/src/WorkerRegistry.sol` as the shipping precursor (not a replacement).

---

## D.06 — Discipline states (Active / Probation / Suspended / Exiled) are partial (Doc 14 §"Four Discipline States")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Four discipline states — Active, Probation, Suspended, Exiled — with automatic transitions based on reputation thresholds and slash history.
**Reality**: `contracts/src/WorkerRegistry.sol:21` declares `enum Tier { Unregistered, Probation, Standard, Trusted, Elite }`. This is a **5-state tier ladder** used for work-gating, not a **4-state discipline ladder** as Doc 14 describes. `Probation` overlaps conceptually but the shipping contract has no `Suspended` or `Exiled` states — a slashed worker's reputation simply drops into the Probation tier. No timelock, no governance override.
**Fix sketch**: Doc 14 §"Four Discipline States" should either (a) align with the 5-state Tier ladder shipping today, or (b) note that the discipline states layer on top of Tier and govern actions (message privileges, dispute authority) rather than replace Tier. Decide which, write it, cross-link.

---

## D.07 — Gaming resistance (EigenTrust, whitewashing, collusion rings) is pure design (Doc 14 §"Gaming Resistance")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Whitewashing prevention via soulbound identity + slash history; collusion ring detection via EigenTrust transitive trust (Kamvar et al. 2003); strategic cherry-picking resistance via randomized audits; governance amnesty for proven wrong slashes.
**Reality**: `Grep 'EigenTrust|whitewashing|collusion|governance_amnesty' crates/ apps/ contracts/src/` returns zero matches. All gaming-resistance concepts are in the 2026-04-13 enhancement pass and remain design-only.

---

## D.08 — C-factor aggregation is absent (Doc 14 §"C-Factor")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: C-factor (collective intelligence factor, Woolley et al. 2010 *Science*) aggregates per-agent reputation into team-level predictive scores for consortium jobs.
**Reality**: `Grep 'c_factor|CFactor|collective_intelligence' crates/ apps/ contracts/src/` returns zero matches. `contracts/src/ConsortiumValidator.sol:1-114` is a consortium voting contract but it does not compute a C-factor. Design-only.

---

## D.09 — Escrow 2% + marketplace 3% fee split is absent (Doc 10 §"Fee Structure")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Fee structure: 2% escrow fee (non-refundable), 3% marketplace fee (deducted from agent payout on completion).
**Reality**: `contracts/src/BountyMarket.sol:108-125` `resolve(id, accepted)` either transfers the full bounty to the worker or refunds it to the poster — **no fee split**. No escrow fee, no marketplace fee collected by the contract. `contracts/src/FeeDistributor.sol:1-103` (103 LOC) is a separate fee-split contract but is not consumed by `BountyMarket` (cross-ref: `contracts/src/BountyMarket.sol` does not import `FeeDistributor`).
**Fix sketch**: Either wire `FeeDistributor` into `BountyMarket.resolve` or mark Doc 10 §"Fee Structure" Phase 2+.

---

## D.10 — Capability matching on bid is tier-gated only (Doc 10 §"Capability Matching", Doc 12 §"Capability Check")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Jobs declare required capability mask (10 capability bits — see B.06). Assigned worker must have all required bits set in their passport capability bitmask.
**Reality**: `contracts/src/BountyMarket.sol:59-80` `postJob(specHash, bounty, deadline, minTier)` accepts a **single `uint8 minTier`** — not a capability bitmask. `assign(id, worker)` at `:84-93` calls `workerRegistry.canAccept(worker, Tier(j.minTier))` — **tier gate only**. No capability match because no capability bitmask exists in `WorkerRegistry` (the field `string capabilities` in `AgentRegistry.sol:11` is free-form text, not consulted here). So capability matching is reduced to "worker tier ≥ minTier".
**Fix sketch**: When Korai v1 capability bitmask lands (see B.06), extend `BountyMarket.postJob` to carry a `uint64 requiredCapabilities` and update `assign` to check the bitmask. Document this dependency in Doc 10 §"Capability Matching".

---

## D.11 — ConsortiumValidator (shipping; not in Doc 10-14 spec)

**Status**: DONE (undocumented on chain topic)
**Severity**: LOW
**Doc claim**: Docs 10-14 do not describe a `ConsortiumValidator` construct by that name. Doc 06 §"Validation Registry" and Doc 10 §"Consortium jobs" reference consortium flows but at a higher level.
**Reality**: `contracts/src/ConsortiumValidator.sol:1-114` is a real shipping contract acting as an optional resolver for `BountyMarket` (per `BountyMarket.sol:10`: "`resolve` is owner/consortium"). Supports committee-style validation. 5 test functions in `contracts/test/ConsortiumValidator.t.sol`. This is adjacent to Doc 06 "Validation Registry" but does not implement the full ERC-8004 Validation semantics.
**Fix sketch**: Add a cross-link from Doc 06 §"Validation Registry" and Doc 10 §"Consortium jobs" to `contracts/src/ConsortiumValidator.sol` as the nearest shipping ancestor, and clarify that it is NOT the planned `0xA300` Validation Registry.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 (D.11 ConsortiumValidator shipping; not in the doc spec by this name) |
| PARTIAL | 4 (D.01 BountyMarket 6-state vs 9-state spec, D.05 single-domain EMA + tiers + slash, D.06 5-state tier vs 4-state discipline, D.10 tier-only capability match) |
| NOT DONE | 6 (D.02 Sparrow VRF, D.03 three hiring models, D.04 Vickrey auction, D.07 gaming resistance, D.08 C-factor, D.09 fee split) |

Section D has **more partial shipping code than any other non-F
section** in the chain topic — because the `contracts/src/*.sol` demo
set really does implement (a) a 6-state bounty marketplace, (b) a
single-domain EMA reputation with 30-day decay and tier thresholds, and
(c) a consortium validator. None of them are the full Korai v1
design, but all three are meaningful and tested (43 test functions
across `contracts/test/`).

The honest framing is: **the Solidity demo contracts are Korai v0.1
prototypes**, not Korai v1, and not nothing. The D-level gap is between
that prototype and the full Sparrow / Vickrey / 7-domain / discipline-state
design, not between "zero code" and "full design."

## Agent Execution Notes

### D.01 / D.05 / D.06 / D.10 — Acknowledge the demo contracts

Best use of this section in batch `08`:

1. Doc 10 should explicitly map `BountyMarket.sol:11-136` 6-state machine onto its 9-state target as a partial,
2. Doc 14 should explicitly map `WorkerRegistry.sol:1-233` single-domain EMA onto its 7-domain target as a partial,
3. Doc 06 should cross-link to `ConsortiumValidator.sol` as a shipping precursor,
4. all other items stay Phase 2+.

### D.02-D.04 / D.07-D.09 — Frontier Banner Pass

Apply `Design — Phase 2+` to Docs 11, 12, 13, and the gaming-resistance + C-factor + fee-split subsections of Doc 10 / 14.

Acceptance criteria for this section:

- the shipping demo contracts are visible from the docs,
- later agents can tell what's partial (D.01, D.05, D.06) and what's pure design,
- the Sparrow / Vickrey / 7-domain gap is explicitly scoped.
