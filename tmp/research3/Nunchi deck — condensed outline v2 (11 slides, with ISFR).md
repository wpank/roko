# **Nunchi deck — condensed outline (11 slides, from 20\)**

**Goal:** lead-investor-readable in 5 minutes. Every slide pulls weight. Move synthesis up. Cut redundancy. **Two narratives, one substrate** — Article 50 / agent coordination is the beachhead, ISFR / agent-native finance is the expansion. Both get airtime.  
**Naming convention:** Roko \= off-chain runtime · Nunchi \= on-chain L1 chain (per WP). Drop “Korai.” See \[\[2026-04-27-nunchi-deck-whitepaper-v9-review\]\] §1.

## ---

**Slide map**

| \# | Title | Source | Job to do   |
| :---- | :---- | :---- | :---- |
| 1 | **Cover** — *Models execute. Nunchi coordinates.* | current p.1 | Hook. Set typography tone. |
| 2 | **The whole argument in 6 lines** | current p.19 (moved up; \+1 line for ISFR) | Elevator pitch for readers who close after slide 5\. |
| 3 | **Why now** — 3 signals \+ 14-week clock | merge current p.3 \+ p.4 | Capital is decided. Regulation has a date. |
| 4 | **The failure mode** — 41–86% / 79% coordination | merge current p.5 \+ p.6 | Why models won’t fix this. |
| 5 | **The empty cell** — 5 funded layers, 1 unfunded | current p.13 | The category claim. Vanta/OneTrust precedent footer. |
| 6 | **The wedge** — $42 → $1.42, 22.5× | current p.11 | Concrete, cited, reproducible economics. |
| 7 | **The moat** — knowledge compounds non-linearly | merge current p.9 \+ p.12 | Why the second mover loses. |
| 8 | **Architecture** — Roko \+ Nunchi, one diagram | merge current p.7 \+ p.8 | One page, no repetition. |
| 9 | **The second category** — ISFR \+ Cooperative Clearing | new (lifts WP §IV.08, §IV.10, §VII.02) | Same substrate clears DeFi rate markets. TAM expansion. |
| 10 | **What 12 months buys** — 4 milestones | merge current p.15 \+ p.16 | The use-of-funds proof. |
| 11 | **Ask \+ closing** | merge current p.18 \+ p.20 | $30M, runway, lead investor wanted. Closing typographic card. |

## ---

**Cuts (and why)**

| Cut | Reason   |
| :---- | :---- |
| current p.2 (thesis) | Redundant with p.19 once p.19 moves to slide 2 |
| current p.6 (system) | Re-states architecture; slide 8 covers it |
| current p.7 (where Nunchi sits) | Architecture stack repeats again |
| current p.10 (the loop / observe-decide-enforce-record) | Beautiful but not load-bearing for investor decision |
| current p.11 (who owns what — comp matrix) | Dense; the empty-cell page already makes the point |
| current p.12 (rails crystallizing — MCP/A2A/ERC-8004/x402) | Tells, doesn’t sell. Move to one footnote on slide 5 |
| current p.13 (why it compounds — three flywheels) | Duplicates the moat slide |
| current p.14 (NHI anchor) | Undercuts empty-cell pitch — see review §5. Either cut or rewrite as tailwind footnote |
| current p.17 (Anchor \+ secondary) | Same NHI issue \+ market sizing duplication |

## ---

**Per-slide draft content**

### **Slide 1 — Cover**

* **Nunchi · The Agent Coordination Plane**  
* *A durable runtime for production agents, with verifiable coordination across organizations.*  
* Two components: **Roko** (cognitive runtime) \+ **Nunchi** (verifiable substrate)  
* 177K LOC Rust · 18 crates · Apache 2.0  
* Series A · 2026

### **Slide 2 — The whole argument**

1. Production agents fail at coordination, not capability — 41–86% failure, system-level.  
2. Roko is the runtime wedge — production agents cheaper, safer, replayable. Open source, self-hosting.  
3. Nunchi is the substrate where local traces compound — shared memory, attestable reputation, verifiable settlement across organizations.  
4. Every adjacent layer is funded — Temporal $5B, LangChain $1.25B, Orkes $300M, Keycard $200M. The coordination plane is empty. That’s the category.  
5. **The same substrate clears DeFi rate markets** — ISFR \+ KKT-verified Cooperative Clearing on a $668T-vs-\<$100M opportunity. One chain, two rent surfaces.  
6. Models commoditize. Scaffolds compound. The model is the same. The system is the variable.

### **Slide 3 — Why now**

| Capital | Demand | Clock   |
| :---- | :---- | :---- |
| $750M Google agentic fund (Apr 22\) | 1,445% Gartner multi-agent inquiry surge | EU AI Act Art. 50 enforces Aug 2, 2026 |
| $60M Orkes Series B (Apr 23\) | 26.2% of EU enterprises ready · 73.8% non-compliant on day one | $15M / 3% turnover penalty |
| Hyperscaler-led capital flowing to deployment, not coordination |  | 14 weeks from today |

Footer: *The empty layer between application and execution is the category. It does not yet have a winner.*

### **Slide 4 — The failure mode**

* **41–86% multi-agent failure** — Berkeley MAST, 1,642 production traces, NeurIPS 2025  
* **79% of failures are coordination, not capability** — FC1 (poor spec) \+ FC2 (inter-agent misalignment)  
* **Six post-mortems, same missing layer:** Klarna · CBA · NYC MyCity · Air Canada · Cursor CVE-2025-54135 · Devin (15% completion)  
* *Adding agents doesn’t add capability. It adds failure modes that no model can solve.*

### **Slide 5 — The empty cell**

* Three-layer agent stack: **frameworks** (LangChain, CrewAI) · **coordination plane** (empty) · **execution rails** (Temporal, Keycard, x402, ERC-8004)  
* Five adjacent companies funded · One cell unfunded · Nunchi declares the cell  
* *Vanta declared SOC 2 → $220M ARR. OneTrust declared GDPR → $10B+ exit. Same playbook, EU AI Act window open.*

### **Slide 6 — The wedge**

* $42.11 (HAL baseline) → $8.40 (caching) → $2.80 (routing) → $1.42 (trim \+ batch) — **22.5× on benchmark, 10–20× practical**  
* Each lever cited: Anthropic 0.1× cache · RouteLLM 85% cut at 95% quality · VentureBeat 73% trim · Anthropic 50% async batch  
* Reproducible methodology: task IDs, model versions, hit rates, gate-exit rates published  
* *The receipt the EU AI Act will require — produced as a byproduct of the wedge.*

### **Slide 7 — The moat**

* Four compounding wheels — **knowledge · calibration · reputation · settlement** — each monotonic in successful Cells  
* HDC NeuroStore: 30K tokens recalled per Signal hit, 92% within-distribution precedent at 200 invocations  
* Five network effects, none requiring a social graph  
* *Code can be forked. Lived coordination history cannot. The thousandth agent joins smarter than the first.*

### **Slide 8 — Architecture**

* Single diagram: Roko (off-chain, 8-phase loop, 11 gates) ↔︎ Nunchi chain (ERC-8004 identity, ERC-8183 job market, Cooperative Clearing, native HDC precompile 0xA01)  
* Three primitives: Signal · Pulse · Cell  
* Nine protocols, ten specializations  
* \~50ms blocks via co-located Tokyo validators on Simplex consensus  
* *Models execute. Roko coordinates. Nunchi attests. Nunchi clears.*

### **Slide 9 — The second category (NEW)**

**Title:** *The substrate clears more than agent receipts.* **Subtitle:** Same chain. Two categories. Both rent-bearing.  
**Left column — ISFR (Internet Secured Funding Rate)** \- DeFi’s SOFR analog. Validator-computed at consensus, published every 10 seconds. \- ISFR \= 0.60·LENDING \+ 0.25·STRUCTURED \+ 0.10·FUNDING \+ 0.05·STAKING \- Sources: Aave V3 / Compound V3 · Ethena sUSDe · Hyperliquid ETH perp · ETH consensus \- Manipulation-resistant by construction (TVL-weighted intra-class median \+ inter-class weighted sum)  
**Right column — Cooperative Clearing** \- KKT-verified batch settlement, O(n) verification, \~1.2s settlement (3 blocks) \- 800ms permissionless solver competition · 5% surplus capped at 50 NUNCHI/batch \- CRPS-scored predictions on every order — strictly proper, non-gameable \- Yield-perpetuals: continuous, partially fillable, no expiry — unlike Pendle’s per-maturity pools  
**TAM box:** \- $668T notional rate-derivatives market (BIS H1 2025\) \- \<$100M on-chain rate products today \- **\>1,000,000:1 gap** — addressable as the substrate matures  
Footer: *Article 50 is the beachhead. ISFR is the expansion. Both clear against Nunchi. Both pay the protocol.*

### **Slide 10 — What 12 months buys**

| Month | Milestone   |
| :---- | :---- |
| M1 | 100 SWE-bench-Verified cost benchmark, published, reproducible |
| M3 | SOC 2 Type II (Schellman) \+ Article 50 mapping document — procurement-ready |
| M6 | 3 named design partners with signed reference agreements, real workloads |
| M9 | Cooperative Clearing v1 testnet · ClearingProfile hedging API |
| M12 | Protocol spec donated to Linux Foundation AAIF (alongside MCP, A2A) |

Design-partner targets, priority order: **Hebbia · Harvey · Decagon · Sierra**

### **Slide 11 — Ask**

* **$30M Series A**  
* 18-month runway · 60% engineering+research / 30% GTM+infra / 10% reserve  
* Lead investor wanted  
* Closing card: *Models execute. Nunchi coordinates.*

## ---

**Open questions before drafting**

1. NHI-anchor framing — drop entirely, or one-line footnote on slide 5? (Currently dropped.)  
2. Are the four design-partner targets (Hebbia/Harvey/Decagon/Sierra) shown with logos, or kept generic?  
3. Founder names on slide 11? Or stay anonymized for v6 confidential?  
4. Closing slide — keep Models execute. Nunchi coordinates. or use the longer Models execute. Frameworks compose. Substrates compound. Nunchi is the substrate. from WP p.66?  
5. Slide 9 (the second category) — emphasis split between ISFR-as-oracle vs Cooperative-Clearing-as-mechanism. Both fit the slide; if we have to compress further, which leads?