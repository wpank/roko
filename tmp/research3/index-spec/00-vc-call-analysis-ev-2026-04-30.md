# VC Call Analysis: Nunchi <> Praneeth Srikanti (Emergent Ventures)

**Date:** April 30, 2026
**Attendees:** John Doherty (CEO), Will Pankiewicz (Engineering), Jacob Gadikian (Chain Engineering), Praneeth Srikanti (Emergent Ventures)
**Duration:** ~65 minutes
**Outcome:** Follow-up call requested by Praneeth — cautiously engaged, not sold but not dismissive

---

## 1. Call Summary

Nunchi presented its L1 blockchain (Korai, built on Commonwealth/Quora) and agent coordination platform (Roko) to Praneeth Srikanti of Emergent Ventures. The call covered chain infrastructure (50ms blocks on devnet), the agent runtime and ACP integrations, hyperdimensional computing, and the ISFR benchmark product. Praneeth came in with a clear due-diligence agenda — wanting to distinguish between what's built, simulated, and spec'd — and pushed hardest on oracle/ISFR maturity and graceful degradation under failure.

**The call went well enough to get a follow-up**, but Praneeth left with several unanswered questions and a clear signal that the next conversation needs more structure.

---

## 2. What Resonated with Praneeth

| Signal | Evidence |
|---|---|
| **Team credibility** | Unprompted: "I'm glad you guys are working with John." Explicitly validated Jacob's Cosmos/Osmosis work. |
| **Chain execution speed** | 50ms/block devnet in 2 weeks impressed. No pushback on chain performance claims. |
| **Hyperdimensional computing** | "I have [heard of it], but yeah I think it's actually been vital, so go ahead." Already knew the concept. |
| **Research partnerships** | Brown (Prof. Herlihy, cooperative clearing) and USC SEP lab (security templates) registered as real differentiators. |
| **Agent gossip architecture** | After Jacob clarified the separation from chain gossip: "Hey, that's helpful to understand. Thank you for that." Genuine relief. |
| **Broader vision** | "I am very interested in what this could end up becoming. I don't want to dismiss the broader vision you have in mind." |
| **Self-assigned follow-up** | Praneeth's own action item: "Provide ongoing feedback and collaborate on governance, oracle data partnerships, and system anti-fragility." |

---

## 3. What Praneeth Pushed Back On

### 3a. "What's real vs. spec'd?"
The explicit frame for the call. He wanted to walk through end-to-end flows and distinguish implemented/simulated/spec'd. **The team never gave him a clean, structured answer.** Demos and narrative dominated instead.

### 3b. Gossip layer as MEV surface
Deep probing: "What exactly do you want gossiped? Does gossip influence execution or is this informational? Could it become a new MEV surface?" Jacob's clean answer resolved this, but the question itself shows Praneeth is thinking adversarially.

### 3c. "What's NOT working?"
Asked directly: "What are the top 3-5 things that are not working or you are worried will not work in production?" John deflected to the oracle problem but didn't give a crisp, vulnerable answer.

### 3d. Graceful degradation
"Do you have a fallback path if dependent subsystems fail — solver side, TEE dependencies, agentic identity, adversarial behaviors?" This was the "monolithic vs. modular" concern from his prior written feedback.

### 3e. ISFR is a benchmark BUSINESS, not just infrastructure (**sharpest pushback**)
When John described ISFR as the critical remaining product, Praneeth immediately reframed: **"This is not just an Oracle network. A benchmark business requires trust, methodology, governance, and adoption — not just technical infrastructure."** John agreed but didn't have a plan ready. This was the weakest moment of the call.

### 3f. Narrow before broad
"A broad-based index might be messy. Something narrower gives credibility." Preferred focused index strategy over ambitious generalization.

---

## 4. Specific Asks from Praneeth (for follow-up)

1. **Structured build-vs-spec audit** — What's implemented, simulated, spec'd-but-not-built
2. **Top 3-5 production risks** — Honest fragility assessment
3. **Graceful degradation map** — What works if TEE/solver/identity subsystems fail
4. **ISFR data partner strategy** — Who are the partners, how do you build trust/governance
5. **Narrow index focus** — Which specific narrow index to start with for credibility
6. **Follow-up call** — Explicitly requested to "touch upon a few other points mentioned earlier"

---

## 5. Gaps in the Pitch (Things a VC Expects That Were Missing)

| Gap | Impact | Priority to Fix |
|---|---|---|
| **Token/tokenomics** | Not mentioned once. For an L1 raise, this is foundational. | Critical |
| **Funding ask and use of funds** | Never articulated. How much, at what valuation, what it unlocks. | Critical |
| **Revenue model** | How does Nunchi make money? Protocol fees? Token? Benchmark revenue? | Critical |
| **Competitive landscape** | Hyperliquid mentioned once. No systematic comp analysis (Injective, dYdX, other agentic chains). | High |
| **Go-to-market detail** | "Get 1-2 market makers by end of month, then hackathons." No customer journey, economics, or flywheel detail. | High |
| **Roadmap with milestones** | No devnet→testnet→mainnet timeline. Praneeth asked implicitly several times. | High |
| **Regulatory risk** | Perp derivatives + agentic trading = significant regulatory surface. Never mentioned. | Medium |
| **Security/audit plan** | When/how does the chain get hardened before mainnet? | Medium |
| **Team size and hiring plan** | 3 people presented. How big is the full team? What roles to fill? | Medium |

---

## 6. Opportunities That Were Mentioned But Not Fully Explored

| Opportunity | What was said | What could be developed |
|---|---|---|
| **Agent identity (ERC-8184/8004 + X402)** | 30 seconds of airtime. Praneeth didn't engage. | Could be a whole pitch angle — on-chain agent identity is a genuinely differentiated primitive. |
| **Cooperative clearing cross-venue** | Brown research enables objective function across venues. Private dealer network opportunity. | Pitch as near-term product for institutional MMs, not just academic research. |
| **Barrage (EVM fork simulator)** | Mentioned in passing during Will's demo. | Standalone developer tool with its own adoption path. Could be the "Hardhat for agents." |
| **Non-trading agentic use cases** | Asserted as reason to not be "just a perp dex" but never named specifically. | Need 2-3 concrete non-trading applications to make the "agentic chain" narrative land. |
| **Collective agent learning** | "Something that hasn't really been done at this scale." 90 seconds of airtime. | This is the core thesis of the litepaper. Deserves a dedicated pitch section with concrete examples. |
| **Security templates (USC lab)** | Mentioned once. | Attack surface templates for agentic systems as a product/consulting play. |

---

## 7. Pitch Improvement Recommendations

### Structure
- **Open with the problem and market size** ($665T interest rate derivatives, zero DeFi equivalent), not background intros
- **Give Praneeth the structured audit he wants first** — a clear table of built/simulated/spec'd — then demo
- **End with a clear ask** — amount, valuation, what the capital unlocks, timeline

### Content
- **Prepare a one-pager**: built vs. spec'd vs. planned, with dates for each milestone
- **Prepare a competitive landscape slide**: systematic comparison against Hyperliquid, Injective, dYdX, Pendle, other agentic chains
- **Have a crisp "top 3 risks" answer ready**: be genuinely vulnerable, then explain mitigations
- **Develop the ISFR-as-benchmark-business narrative**: governance model, data partner pipeline, credibility roadmap (Phase 1: narrow index → Phase 2: broader indices → Phase 3: derivatives market). Show you understand this is a trust problem, not a tech problem.
- **Prepare tokenomics overview**: even if not finalized, show the economic model

### Delivery
- **Tighten the demo flow**: Don't jump between screens mid-sentence. Have a scripted 5-minute demo path.
- **Don't take shots at competitors/providers** ("Opus 4.7 kind of sucks") in a VC call
- **Match what Praneeth is asking for**: He came in with a technical DD agenda. The team responded with narrative. Next call should be structured around his questions.

---

## 8. Materials to Prepare Before Follow-Up Call

### Must-Have (send before the call)
1. **System maturity matrix**: Table of every subsystem with status (built/tested/spec'd), confidence level, and dependencies
2. **Architecture diagram**: One-page visual showing chain, agents, ISFR, clearing, and how they connect
3. **Competitive landscape**: 1-page comparison grid vs. Hyperliquid, Pendle, Injective, dYdX, Bittensor
4. **ISFR credibility roadmap**: Phase-gated plan from narrow index → broad → derivatives, with data partner targets

### Should-Have (ready to share during call)
5. **Tokenomics draft**: Supply, distribution, validator incentives, fee structure
6. **Risk register**: Top 5 risks with probability, impact, and mitigations
7. **Roadmap**: Devnet → public testnet → mainnet timeline with milestones
8. **Revenue model sketch**: How Nunchi captures value (protocol fees, benchmark licensing, etc.)

### Nice-to-Have (for deep dives if asked)
9. **Demo video**: 3-minute walkthrough of agent executing a yield perp trade
10. **Academic partnership summary**: Brown and USC work, publication timeline, what it proves
11. **Market maker pipeline**: Who you're talking to, what the deal structure looks like

---

## 9. Recurring VC Questions to Prepare For

These will come up in every pitch, not just with Praneeth:

| Question | Current Answer Quality | Needs |
|---|---|---|
| "What's built vs. what's spec'd?" | Weak — team defaults to demos | Clean matrix document |
| "Why do agents need their own chain?" | Medium — Jacob explains well technically | Sharper 30-second version with economic argument |
| "How is this different from Hyperliquid + AI wrapper?" | Weak — not directly addressed | Crisp differentiation framework |
| "What's your go-to-market?" | Weak — "market makers + hackathons" | Concrete customer pipeline and flywheel |
| "How do you bootstrap liquidity?" | Medium — market maker strategy exists | Numbers: target volume, timeline, incentive structure |
| "What's the token model?" | Missing entirely | Draft tokenomics |
| "What's the regulatory risk?" | Missing entirely | At minimum, acknowledge it and describe the approach |
| "Why now?" | Medium — mentioned AI stagnation at model level | Sharper timing thesis (AI agents + DeFi yield convergence) |
| "What are you worried about?" | Weak — deflected | Honest top-3 with mitigations |
| "What's the ask?" | Missing entirely | Amount, valuation, use of funds |

---

## 10. Overall Assessment

**Praneeth is still in process.** He's genuinely interested in the vision and the team, but he's running a structured DD process and the team gave him narrative when he wanted rigor. The follow-up call is a real opportunity — if the team comes prepared with the structured materials he's asking for, the outcome could be very different.

**Key risk**: The team's default mode is "show cool tech" when Praneeth's mode is "stress-test the system." The next call needs to be run on Praneeth's terms, not the team's.
