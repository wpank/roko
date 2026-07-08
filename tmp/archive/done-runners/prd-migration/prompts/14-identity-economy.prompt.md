# Prompt: 14-identity-economy

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/14-identity-economy/`. Covers ERC-8004 agent identity, reputation system (7-domain EMA), knowledge marketplace, commerce bazaar, Machine Payment Protocol (MPP), x402 micropayments, agent economy, KORAI/DAEJI tokenomics with demurrage, Vickrey reputation-adjusted auction, ISFR, clearing & settlement, Knowledge Futures Market (P3), regulatory moat, a16z Series A framing.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/04-knowledge-and-mesh.md` §2 Korai, §3 ERC-8004
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §VIII x402 Micropayments, §XVI Knowledge Futures Market, §IX Forensic AI (regulatory moat), §XVIII Blue Ocean Summary
3. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 6E/6G, §What Makes This a Series A Story

## Step 3 — SOURCE-INDEX entry `## 14-identity-economy.md`

Key legacy:
- `bardo-backup/prd/09-economy/00-identity.md`, `01-reputation.md`, `02-clade.md` (rename), `03-marketplace.md`, `05-agent-economy.md`, `06-commerce-bazaar.md`
- `bardo-backup/prd/shared/x402-protocol.md`, `eip-analysis.md`
- `bardo-backup/tmp/agent-chain/06-tokenomics.md` (rename GNOS→KORAI/DAEJI)
- `bardo-backup/tmp/agent-chain/12-golem-orchestrators.md` (OaaS)
- `bardo-backup/tmp/agent-chain/13-orchestration-as-a-service.md`
- `bardo-backup/tmp/agent-chain-new/05-token-economics.md`, `12-agent-economy.md`, `11-adversarial-defense.md`
- `bardo-backup/tmp/death/14-proposals-and-billing.md`, `15-cost-tracking.md` (extract mechanism, drop mortality)
- All of `bardo-backup/tmp/death/payments/` (10 files)

## Step 4 — implementation-plans

- `12b-chain-layer.md` §A (Identity, Korai Passport, tiers, ventriloquist), §C (Job Market), §K (Reputation 7-domain), §L (Payments DAEJI, x402, escrow), §N (ISFR), §O (Clearing)

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/14-identity-economy
```

Write **16 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-a16z-framing.md` | Vision: agent-native economy. a16z-compatible framing. KYA (Know Your Agent). Agent-native infrastructure. 8 Series A pitch points from 07-implementation-priorities.md. |
| 01 | `01-erc-8004-three-registries.md` | Identity (ERC-721 Agent Card with capabilities, endpoints, payment address). Reputation (feedback authorization, off-chain scoring). Validation (agents request verification; contracts provide attestation; reputation/stake/zkML/TEE options). |
| 02 | `02-korai-passport.md` | ERC-721 soulbound NFT. Full struct: passportId, owner, capabilityList bitmask, domainStakes, reputationTracks, teeAttestation, systemPromptHash (ventriloquist defense), tier, slashHistory. |
| 03 | `03-passport-tiers.md` | 4 tiers: Protocol (governance-approved), Sovereign (25K KORAI, direct hire, consortium lead, schema authoring), Worker (5K KORAI, standard marketplace, bidding), Edge (none, random assignment only, ≤50 DAEJI jobs). |
| 04 | `04-reputation-7-domain-ema.md` | 7-domain EMA decay, halving, tiers, disputes. Glicko-2 rating integration. Staking payoffs. Per-domain reputation score. On-chain: who can rate whom. Off-chain: actual scores. |
| 05 | `05-knowledge-marketplace.md` | Marketplace for knowledge contributions. Curation. Pricing. Discovery. Links to NeuroStore for local + Korai for global. |
| 06 | `06-commerce-bazaar.md` | Bazaar primitives. Kept concept from legacy. Commerce transactions. |
| 07 | `07-mpp-machine-payment-protocol.md` | HTTP 402 Payment Required. ERC-3009 signatures (off-chain authorization). Charge/session intents. MPP as the payment foundation. |
| 08 | `08-x402-micropayments.md` | Coinbase x402 protocol (Linux Foundation, AWS/Visa/Mastercard/Stripe). Per-API-call billing < $0.001. Sub-second USDC settlement on Base. Self-funding agent loop: KORAI earnings → USDC → x402 compute → output → user pays → reinvest. Cycle accelerates. Agent-as-a-business. |
| 09 | `09-agent-economy.md` | Revenue streams. Billing. Proposals. Cost tracking. How agents earn and spend autonomously. Example: a research agent earning from knowledge contributions, spending on inference, producing better services. |
| 10 | `10-korai-tokenomics.md` | KORAI (mainnet) 1% annual demurrage. DAEJI (testnet). Earning (registration mint, validated knowledge posting, confirmation, heartbeat, challenge defense). Spending (posting/anti-spam, querying, challenging). Quality incentives (duplicate penalty, novelty bonus, curation bonds, cross-agent confirmation multiplier). Why demurrage: knowledge must be actively maintained; prevents unbounded garbage accumulation; mirrors Engram half-life at token level. |
| 11 | `11-vickrey-reputation-auction.md` | Vickrey reputation-adjusted formula: `s_i = p_i × (1 + (1 - R_i))`. Winner = argmin. Payment = `s_second / (1 + (1 - R_winner))`. Truthfulness guarantee (can't inflate bids). |
| 12 | `12-three-hiring-models.md` | (a) Random VRF assignment (< 50 DAEJI jobs), (b) Blind auction FPSB/Vickrey/Dutch (encrypted via ECIES, decrypted in TEE), (c) Direct hire (1.5× fee premium, anti-centralization: >20% volume in 30 days → 2× fee). |
| 13 | `13-isfr-clearing-settlement.md` | ISFR (Intersubjective Fact Registry): collective price discovery, rate aggregation, disputed claim resolution, 3-arbitrator voting. Clearing: QP solver, bisection, certificates, fallback. Batch clearing, DVP, settlement finality. |
| 14 | `14-knowledge-futures-market.md` | P3, deferred. On-chain escrow for committed knowledge production. Research agent publishes Knowledge Future → operations agents purchase via x402 → escrow funds research → delivery triggers release → non-delivery slashes staked KORAI. Predictive market for knowledge production — network prices knowledge before it exists. |
| 15 | `15-regulatory-moat-and-current-status.md` | Forensic AI Causal Replay enables compliance (cross-reference 11-safety.md §15). Enterprise value $100-500K/month per regulated enterprise. Compliance failures cost $10M-$1B. Moat: once regulator blesses a configuration, switching cost is astronomical. Tier 6 deferred status. Solo agents and event-driven agents do NOT need the chain layer. Cross-references. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥4000 total. Citations: EIP-8004, EIP-721, ERC-3009, ERC-4337, x402 protocol (Coinbase/Linux Foundation), Glicko-2, VCG (Vickrey 1961, Clarke 1971, Groves 1973), Ousterhout 2013, Metcalfe's Law, Reed's Law, EU AI Act, MiFID II.

Cross-reference 00-architecture, 04-verification (forensic AI), 08-chain, 13-coordination.

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE CITATIONS.
- Rename: GNOS → KORAI (mainnet) / DAEJI (testnet); golem → agent; clade → collective/mesh; bardo → roko; mori → Roko Orchestrator.
- The regulatory moat + a16z framing is the investor narrative — make it prominent.
- No death framing.
- Use Write tool. Don't ask questions.
