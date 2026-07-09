---
attention: active
confidence: probable
created: '2026-04-30'
domain: chain
last_modified: '2026-04-30'
owner: jl
priority: high
status: draft
title: 'Daeji Differentiation PRD — Coordination Primitives Ethereum and Rollups Cannot Run'
refinements:
  - date: '2026-04-30'
    by: jl
    focus: 'Initial synthesis from 2026-04-30 Praneeth/Ethereal debrief call (Jacob, Will, JD, Jae) — anchored seven core primitives P1–P7 directly to in-call discussion (browser/CLI light client, big-chat agent control, DKG-private collaboration, proof-of-work-done, proof-of-learning, on-chain reputation).'
  - date: '2026-04-30'
    by: jl
    focus: 'Second pass — added eight extension primitives V1–V8 (sub-block-time IRO, composable feeds, cross-agent proof composition, robotics privacy, EU-compliance loop, multi-agent ensembles, light-client streaming, symphony-of-life), Q2–Q4 sequencing matrix, seven investor kill lines, public-materials publication path, and 8 open questions including terminology lock items (kora vs Roko, DKG vs threshold).'
---

# Daeji Differentiation PRD — Coordination Primitives Ethereum and Rollups Cannot Run

**Date:** 2026-04-30
**Status:** Draft v1 — synthesizes the 2026-04-30 Praneeth/Ethereal debrief call (Jacob, Will, JD, Jae)
**Authors:** Jae Lee
**Related:**
- [agent-chain-features.md](agent-chain-features.md) — internal architecture synthesis
- [erc-8004-evaluation.md](erc-8004-evaluation.md) — agent identity / reputation compatibility
- [korai-reputation-framework.md](../korai/korai-reputation-framework.md) — three-layer reputation model
- [hdc-implementation-plan.md](hdc-implementation-plan.md) — HDC precompile plan
- `~/obsidian-vault/research/2026-04-30-jacob-gadikian-call.md` — source call
- `~/obsidian-vault/research/2026-04-29-daeji-commonware-chat-spec.md` — chat-vs-libp2p split

---

## 1. Why this PRD now

The 2026-04-30 Ethereal/Praneeth call confirmed a pattern: VCs (and most observers) default to "do you know what you're doing?" before any market-opportunity conversation. Will P's observation on the call: Praneeth was running a competency check, not a market thesis discussion. JD's read: he was re-running the A16z/January feedback ("you should have people on the team who can build the chain").

The cure is **concrete differentiated capabilities** — primitives that are not "another Ethereum but faster" and that Praneeth, Eshan, or any future investor can see before they ever ask the competency question. Will P committed on the call to publish updated chain architecture publicly; this PRD is the framing layer that organizes those publications into a single differentiation surface.

**Working principle (from the call, JD-coined):** "Generic blockchain demos are death. We don't show blockchain things — we show things you can't do on Ethereum or a rollup."

---

## 2. Thesis

Daeji is not a faster L1 or a cheaper L2. It is a **coordination substrate for verifiable agent work**, shipped with four substrate features that each independently exceed what Ethereum/rollups can deliver:

1. **Native light clients** in the browser and CLI (Merkle-verified, real-time state streams).
2. **Chat as a chain-native control surface** for thousands of agents (identity, role, ACL all on chain).
3. **DKG / threshold privacy primitives** so agents can collaborate without exposing inputs.
4. **Proof-anchored reputation** built on proof-of-work-done and proof-of-learning, where reputation is a *consequence* of verified work rather than an off-chain rating.

Every primitive in this PRD requires at least one of those features. None is replicable by adding contracts to Ethereum or by deploying a rollup, because each depends on chain-level capabilities (consensus design, native chat transport, DKG-aware crypto, light-client conformance) that aren't first-class on those substrates.

---

## 3. Core primitives (anchored to call discussion)

### P1 — Browser-native light client

| | |
|---|---|
| **What** | Streaming chain state into the browser as a verifying client. Merkle-proof verified per event. Reference: Alto's Commonware consensus map. |
| **Why Ethereum/rollup can't** | Ethereum light-client work (Helios, etc.) is heroic effort against a chain not designed for it. Rollups inherit Ethereum finality + DA constraints — sequencer trust + L1 finality kills "live, browser-verified, app-grade UX." Cosmos technically *could*, but the libraries were never built; Commonware is the first project to ship them as a primitive. |
| **Demo path** | Public web page that renders agent chat + job activity verified end-to-end by the visitor's browser. No trusted RPC. |
| **Owner** | @wp (chain arch + spec), @jacob (light-client lib), @jl (demo wrapper) |
| **Investor framing** | "Open this URL. Your browser is the validator." |

### P2 — CLI-native light client

| | |
|---|---|
| **What** | Same Merkle-proof + state streaming, but the CLI **is** the light client. Agent code running locally verifies chain state directly without trusting any remote RPC. |
| **Why Ethereum/rollup can't** | Ethereum CLI tooling assumes a trusted RPC (Infura, Alchemy). The verification path is structurally absent from the developer surface. |
| **Demo path** | An agent CLI that displays a verification badge in real time and attaches a verifiable proof bundle to every job receipt. |
| **Owner** | @jacob (chain RPC + light-client conformance), @wp (CLI primitives) |
| **Investor framing** | "Your shell command is the validator." |

### P3 — Big-chat agent control (10,000-agent coordination surface)

| | |
|---|---|
| **What** | Chain-native chat (commonware-chat) carrying agent identity, role, scope, and message ACLs as first-class state. Operator addresses subsets of N agents (by role, by stake tier, by reputation, by user-trust); only matched agents reply or execute. Jacob 04-30: *"imagine a chat with 10,000 agents — it becomes a really powerful way to control them."* |
| **Why Ethereum/rollup can't** | Ethereum has no native chat transport. Off-chain layers (XMTP, Push, Waku) lack the verifiability + on-chain ACL gating + light-client compatibility. Adding them on top doesn't recover the property. |
| **Demo path** | 1,000+ bot agents in a single chat. User sends `/run isfr-recompute`; only ISFR-eligible agents (proven via reputation + stake) execute and reply. Operator can scope by user identity ("agents listen to me but ignore that user") on chain. |
| **Owner** | @wp (commonware-chat integration), @jl (demo + ACL semantics) |
| **Investor framing** | "Slack with 10,000 verified workers and a kill switch built into the protocol." |

### P4 — DKG-private agent collaboration

| | |
|---|---|
| **What** | Agents pair, derive shared secrets via DKG, collaborate (compute, exchange data, jointly sign) without ever exposing inputs. Threshold signatures finalize the collaborative output. Sub-block-time IRO collaboration available when agents are <5ms apart (Jacob 04-30). |
| **Why Ethereum/rollup can't** | No native DKG primitives. Bolted-on networks (Lit, etc.) require trusting a separate quorum — defeats the "no extra trust assumption" pitch. |
| **Demo path** | Two robotics agents pool training data privately; one publishes a derived model proof to chain. Raw data never moves. Maps directly onto the EU/UK regulatory wedge (see V4 below). |
| **Owner** | @jacob (DKG primitives), @wp (agent-side wrapper) |
| **Investor framing** | "Two competitors collaborate without ever showing each other their data — and the chain is the only witness." |

### P5 — Proof-of-work-done (verified agent execution)

| | |
|---|---|
| **What** | Every job claim → commit → execute → settle cycle ends with a cryptographic proof that the agent did what it claimed. Reputation updates automatically from proofs — no oracle, no manual rating. Jae 04-30: *"all work is verified on chain and all agents can prove that they've done the work that they say they were going to do."* |
| **Why Ethereum/rollup can't** | Ethereum has no shared notion of agent job lifecycle. Every dApp invents its own bespoke off-chain attestation system. Reputation systems exist (Lens, Karma) but they sit on top of social graphs, not on top of work. |
| **Demo path** | ISFR run end-to-end. Council of agents pulls component data, each commits + proves their slice, the initiator (us) sets weights, payout flows through Roko/kora AgentRegistry. Same job, replayed by a different council, produces an identical (or weight-defended) result. |
| **Owner** | @jl (job spec), @jacob (proof primitives), @wp (Roko AgentRegistry + bounty flow) |
| **Investor framing** | "If they didn't do the work, they can't get paid. The chain is the timekeeper." |

### P6 — Proof-of-learning

| | |
|---|---|
| **What** | An agent ingests dataset X (or trains on it) and emits a proof that exists on chain. Other agents compose against that proof: "I trust your output because you provably learned X." Jae 04-30: *"the definition of as an agent actually learn something can be based on proofs as well... we can literally issue like a proof of 'I have learned this' and have that exist on the chain."* |
| **Why Ethereum/rollup can't** | ZK-ML and TEE attestation projects exist (RiscZero, EZKL, Marlin), but none are wired into a coordination layer with reputation and bounty payout primitives. Ethereum can verify a proof; it cannot make the proof a reputation-bearing event in a multi-agent marketplace. |
| **Demo path** | Agent A trains on open robotics dataset X, posts proof. Agent B advertises ability on the same dataset; user can pick A (proven) over B (claimed). A's proof becomes a reusable badge across future jobs. |
| **Owner** | @jl (use-case shaping), @wp (proof composition spec) |
| **Investor framing** | "Reputation is what you proved you learned, not what you claimed on your résumé." |

### P7 — On-chain reputation (composes P5 + P6)

| | |
|---|---|
| **What** | Reputation is not a number set by a DAO or rated by users. It is a continuously updated composition of: P5 proof count + accuracy, P6 attestations (which datasets, which models), stake at risk, time + diversity of jobs, slashes from disputed proofs. Reputation is *gossip-gated* on commonware-chat per the [2026-04-29 split](../../../research/2026-04-29-daeji-commonware-chat-spec.md). |
| **Why Ethereum/rollup can't** | Ethereum has no shared schema for any of the inputs. Existing reputation tokens (Lens, EAS, Galxe) are social graph or attestation registries — not work-anchored, not light-client streamable, not chat-gateable. |
| **Demo path** | Leaderboard view inside the browser light client (composes P1). Click an agent; see verified job history + proof bundles + reputation curve. Reputation slot affects message visibility in the chat (composes P3). |
| **Owner** | @jl (PRD ownership, demo design), @wp (chain schema), @jd (BD framing for HIP-3-style indices) |
| **Investor framing** | "Yelp stars are claims. Daeji reputation is receipts." |

---

## 4. Extension primitives (compose with the core seven)

These compound the surface but are out of scope for the **Q2 demo gating**. They appear in public materials so the differentiation perimeter is visible to investors and partners.

| ID | Primitive | Anchored In | One-liner |
|----|-----------|-------------|-----------|
| V1 | **Sub-block-time IRO collaboration** | Jacob 04-30 (~5ms agent proximity) | Latency-sensitive negotiation between co-located agents (MM, sub-block consensus). Parked behind libp2p workstream per [chat-vs-libp2p split](../../../research/2026-04-29-daeji-commonware-chat-spec.md). |
| V2 | **Composable continuous feeds** | Will + JD 04-30 ("Internet Archive on chain if all else fails") | Chain carries any continuous stream — research papers, market prices, social feeds, archive snapshots. Agents both ingest and emit feeds. **Legal envelope, not chain capacity, is the constraint.** |
| V3 | **Cross-agent proof composition** | Jae 04-30 ("a lot of different things around proofs") | Agent A's proof becomes input to agent B's job. Building block for derived-intelligence marketplaces. |
| V4 | **Privacy-preserving robotics data marketplaces** | Jae 04-30 (Unitree → Boston Dynamics framing) | Rich datasets monetized without exposure. DKG agent learns from data, emits derived agent/model. Robotics-native because regulatory pressure is highest there. |
| V5 | **EU-compliance loop on agent-derived indices** | JD 04-30 (PIFF / HIP-3 PIP3 agreement context) | UK/EU restrict raw-feed-derived indices, but agent-derived computation is a regulatory wedge. Same primitive applies to GDPR-compliant data products. |
| V6 | **Multi-agent ensembles with provable diversity** | Jae 04-30 ("$20/mo Claude vs ours") | Claude + Codex + Sonnet voting on a job, with the diversity itself attested. Pitch line: provably smarter ensemble + cost arbitrage vs. single-model agents. |
| V7 | **Light-client-verified streaming for non-financial domains** | JD 04-30 (Wayback / MMORPG / social) | Same primitive as P1+P2 but for archive provenance, social moderation, prediction-market video-game state. Anywhere a "did this really happen" question exists. |
| V8 | **Symphony-of-life for high-net-worth users** | Jae 04-30 ("$1k/month for that") | Continuous agent ensembles spanning multiple domains of a single user's life. Builds on P3 (chat as control surface) + V6 (provable diversity). |

---

## 5. Sequencing

| Quarter | Milestone | Anchored Primitives |
|---------|-----------|---------------------|
| Q2 2026 | Ethereal demo + Miami | P1, P3, P5, P6 (teaser), P7 (leaderboard) |
| Q2 2026 (post-Miami) | Public coordination layer + first paying job | P2, P4, V1, V6 |
| Q3 2026 | Vertical wedges | V4, V5, V7 |
| Q4 2026 | Symphony / HNW | V8 + Nunchi House composition |

The Q2 cut is deliberately small. Per JD's call read on Praneeth: shipping a demo with P1 + P3 + P5 working end-to-end is enough to **graduate past the competency gate**, after which market-opportunity conversations become possible. Subsequent quarters ladder the rest in.

---

## 6. Investor-facing kill lines

For Series A pitch language, blog posts, and FAQ. Each line picks a single primitive and contrasts it directly against Ethereum/rollups.

- **"We don't run faster Ethereum. We run things Ethereum doesn't run."** — overall thesis.
- **"Open this URL. Your browser is the validator."** — P1.
- **"Slack with 10,000 verified workers and a kill switch built into the protocol."** — P3.
- **"Two competitors collaborate without showing each other their data, and the chain is the only witness."** — P4.
- **"If they didn't do the work, they can't get paid. The chain is the timekeeper."** — P5.
- **"Reputation is what you proved you learned, not what you claimed on your résumé."** — P6.
- **"Yelp stars are claims. Daeji reputation is receipts."** — P7.

---

## 7. Public materials path (what Will publishes, what Jae chops)

Per the 2026-04-30 call commitment:

1. **Will P** publishes updated chain architecture docs publicly (chain topology, light-client + Merkle proof design, consensus + gossip story, commonware-chat spec).
2. **Jae** chops these into:
   - One blog post per core primitive (P1–P7) — same template, different anchor.
   - One FAQ document for Eshan-led DD that pre-answers the Praneeth-style competency questions.
   - One investor-facing one-pager that lists the seven primitives with the kill lines from §6.
3. **JD** converts the Praneeth call notes into a Nunchi-initiated fact doc and DMs Eshan to schedule the deep-dive **after Miami**.

Ship target: blog series begins within one week of Will's first public arch post.

---

## 8. Open questions (must resolve before publishing externally)

1. **What's the cleanest one-line for proof-of-learning (P6)?** ZK-ML adjacent but distinct. Need a phrase that doesn't trigger "isn't this Risc Zero?" pushback.
2. **DKG-private vs threshold-encrypted — terminology lock?** Today's [DKG/TEE assessment](../../../research/2026-04-30-daeji-dkg-confidential-compute-assessment.md) proposes "Path 3" hybrid. Need consistent vocabulary across this PRD and that assessment.
3. **Reputation schema canonical version** — is the [korai-reputation-framework](../korai/korai-reputation-framework.md) (2026-04-02) still authoritative, or has it moved? PRD must reference current.
4. **Roko vs kora terminology** — terminology lock from 2026-04-30 standup says **kora** (not Roko). This PRD uses both because the source call uses Roko; future revs should adopt kora consistently.
5. **Demo path for P6 proof-of-learning** — what dataset is small enough to run end-to-end in a browser-rendered demo by Miami? Need a candidate within 1 week.
6. **Public scope of V4 (privacy robotics)** — keep at "wedge mention" or commit to a concrete BD partner (Unitree, etc.)?
7. **Does V2 conflict with the V2-for-V8 nesting?** ID collision risk with internal "V2 CLOB" terminology — rename if confusing.
8. **What is the Quora/Simplex BFT story in the public arch doc?** Must be consistent with the Kauri-deprecated 2026-04-08 update.

---

## 9. Reviewers

- **@wp** — chain primitives, commonware-chat scope, light-client lib reality check
- **@jacob** — light client, RPC compliance (PR16 lesson), DKG primitives
- **@sdb** (Sam) — product framing, Series A pitch language alignment
- **@jd** (John Doherty) — investor-facing language, partner-trust mapping, Praneeth/Eshan delivery path

(Reviewer list reflects 2026-04-30 org changes.)
