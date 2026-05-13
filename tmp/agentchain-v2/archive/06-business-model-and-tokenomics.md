# Business Model and Tokenomics

How Nunchi makes money. Dual-asset corporate structure (Delaware C-corp + foundation, modeled on Story Protocol and Helium). Apache 2.0 + BSL licensing stack. Enterprise support beachhead, managed cloud expansion, protocol fees long-term. NUNCHI burn-and-mint with USD-pegged Trust Credits. Token graveyard analysis. Comparable economics. Series A target.

---

## 1. The Dual-Asset Corporate Structure

Nunchi operates two complementary entities. The structure separates equity from protocol governance and is now standard practice for infrastructure projects that intend to decentralize.

### Entity 1: NunchiLabs Inc. (Delaware C-corp)

The operating company. Raises equity, employs engineers, signs enterprise contracts, owns intellectual property in the managed cloud services, operates the sequencer during the bootstrapping phase. **Series A equity flows here.** Token warrants — giving equity investors pro-rata rights into the token allocation — are attached to the Series A as a side letter.

- **HQ:** Berlin (engineering) / Delaware (incorporation).
- **Operates:** Roko cloud, enterprise support, forward-deployed engineering.
- **Licenses:** Roko = Apache 2.0; on-chain contracts = MIT; managed cloud = BSL with 4-year Apache conversion.
- **Revenue:** managed cloud, enterprise licensing, support contracts.
- **Grants:** Berlin/EU non-dilutive grants for R&D (NLnet, Sovereign Tech Fund, Rust Foundation Community Grants — see section 9).

### Entity 2: Nunchi Foundation (Cayman Islands or Swiss Association, TBD)

The Foundation governs the protocol, controls the on-chain treasury, and issues NUNCHI tokens. It is **formed 18–24 months after the Series A closes** — after the product has demonstrated product-market fit and the token has a defensible utility story. The Foundation cannot be formed on Day 1 without triggering securities risk; the delay is deliberate and non-negotiable.

- **Operates:** the Nunchi blockchain, NUNCHI token, Trust Credits.
- **Governs:** ERC-8004 reputation parameters, ERC-8183 protocol fees, genesis parameters.
- **Revenue:** 5% protocol fee on settled jobs (2% to Foundation treasury, 3% to validators).
- **Audit:** multi-firm — Trail of Bits + Halmos + EthSecurity.

### Why dual-entity (and the R13 dual-entity flag risk)

For non-crypto infrastructure companies, a dual-entity structure is unusual and will itself trigger diligence questions. Casado's infrastructure deals are single-entity C-corps. The dual-entity pattern is associated with crypto-native projects (Story Protocol, Olas, Nava) where a Foundation governs the protocol and a C-corp builds the software. If Nunchi is pitched as infrastructure to Casado's team (not Dixon's crypto team), the Foundation entity may pattern-match to unnecessary complexity.

**Mitigation:** evaluate collapsing to a single C-corp before the meeting. If dual-entity is retained, have a crisp one-sentence justification: *"The Foundation governs the protocol specification and ensures neutrality; the C-corp builds the commercial software and captures revenue."* If Casado raises it, do not be defensive — acknowledge the complexity and explain why it is structurally necessary for the coordination plane's neutrality claim. If the honest answer is that it is not structurally necessary for the pre-chain phases, collapse it.

### Precedents

- **Story Protocol / PIP Labs** — closed Series B of $80M at $2.25B valuation (August 2024), led by a16z crypto and Polychain. Equity in PIP Labs (operating company) plus token warrants giving investors pro-rata participation in the IP token. Cleanest recent template for dual-asset structure where the token is explicitly tied to on-chain utility.
- **Helium / Nova Labs** — a16z and Khosla invested in Nova Labs (equity); HNT is governed by the Helium Foundation. Operational precedent. Helium's burn-and-mint equilibrium (HIP-141) is the direct inspiration for NUNCHI tokenomics.
- **Worldcoin / Tools for Humanity** — raised $240M+ from a16z, Khosla, Bain Capital Crypto, Tiger Global. In July 2024, Worldcoin extended insider vesting from 3 years to 5 years — the market now expects longer alignment periods. Baked into Nunchi design from the start.
- **Temporal** — $300M at $5B (February 2026, Sarah Wang and Raghu Raghuram, NOT Casado) with no token whatsoever — pure enterprise SaaS. Demonstrates that infrastructure orchestration businesses command billion-dollar valuations without tokenization. The token is not a requirement for the business; it is a requirement for the decentralized network layer.

---

## 2. Licensing Stack

Licensing is the fulcrum of the distribution-versus-revenue tradeoff. The wrong choice at any tier collapses either adoption or commercial viability.

| Component | License | Rationale |
|---|---|---|
| Roko runtime (18 crates) | **Apache 2.0** | Maximum adoption; embeds in enterprise CI/CD and cloud platforms without legal friction |
| Token contracts | **MIT** | Standard for on-chain code; auditors, forks, and integrators expect it |
| Managed cloud services | **BSL with 4-year Apache conversion** | Revenue protection during the critical growth window with a sunset that prevents community hostility |

### Why Apache 2.0 for the runtime

Enterprise legal teams pattern-match Apache 2.0 as safe. Embeds without copyleft contamination. Kubernetes, gRPC, Rust itself — every infrastructure project that achieved platform-level adoption chose Apache 2.0 or similar permissive license. Roko being Apache 2.0 means it can be adopted inside Cursor, Replit, Vercel, and any other dev-tool platform without a license review cycle. It also gives Nunchi a real legal-procurement moat for enterprise customers whose legal teams require explicit patent grants — among credible Rust agent frameworks (rig-core MIT, swiftide MIT, llm-chain MIT and dormant), the Apache-2.0 niche is uncontested for runtimes (Codex CLI occupies it for IDE-attached coding agents, structurally different).

### Why BSL for managed cloud

Business Source License restricts production use of the managed services for 4 years, after which the code converts to Apache 2.0 automatically. Not OSI-approved open source, but source-available — readable, auditable, forkable for non-commercial use.

- **HashiCorp BUSL** (August 10, 2023): preserved $6.4B in commercial value (IBM acquired HashiCorp February 2025). Provoked OpenTofu fork within weeks (now Linux Foundation-backed); HashiCorp lost <10% of community contributions in first year. **The lesson:** BSL works if the product is genuinely better than any plausible fork; fails if the community can replicate the value quickly.
- **Elastic SSPL** (January 2021): AWS forked as OpenSearch within weeks. By late 2024, OpenSearch had outgrown Elastic on DB-Engines metrics. **The lesson:** if your managed service is just packaging of a commodity store, SSPL will not hold. Roko's orchestration logic, gate pipeline, cascade router, and learning layer are not commodity — they are the compounding IP.
- **Supabase** chose full Apache 2.0 / PostgreSQL licensing → $5B valuation (Series E October 2025) with ~$70M ARR by Oct 2025. **The lesson:** sometimes full open source wins, especially when growth is driven by AI-native builders auto-provisioning the backend (Bolt, Lovable, Cursor, Replit, v0). Roko splits the difference.

### Relicensing cautionary tales

**Never relicense the OSS core.** Three cases:

| Company | Action | Date | Community response |
|---|---|---|---|
| HashiCorp | MPL → BUSL | Aug 10, 2023 | OpenTofu fork within weeks |
| PlanetScale | Killed free tier | Apr 8, 2024 | $0 → $39/mo with tone-deaf layoff messaging; community panned |
| Railway | Killed free tier | Jun 2, 2023 | Trial credits + $5/mo entry; community responded warmly |

**The rule:** never relicense the OSS core; gate operational convenience (hosted, support, SLA), never functionality. Declare licensing structure once and stick with it. Community accepts BSL when announced upfront (Sentry, MariaDB, CockroachDB) but punishes it when retrofitted.

### Lock-in mechanism

Lock-in comes from three sources, none of which are switching costs in the traditional sense:

1. **Love of the runtime** — Roko's tooling compounds with playbooks, episode history, and gate calibration. Switching means starting the learning loop from zero.
2. **Trust of the chain** — audit trails, ISO 42001 attestations, EU AI Act compliance records anchored on-chain. The chain is the compliance system of record.
3. **Necessity of the compliance posture** — after August 2, 2026, EU enterprises operating high-risk AI systems face €35M / 7% of global turnover penalties for prohibited practices and €15M / 3% for transparency failures. Nunchi's compliance tooling becomes legal necessity, not nice-to-have.

---

## 3. Revenue Model: Three Streams

Revenue is designed to be durable across the token cycle. The business does not depend on token appreciation.

### Stream 1: Managed Cloud (near-term, primary)

The primary revenue engine for the first 24 months. BSL-licensed (4-year Apache conversion). Hosted Roko runtime with the inference gateway, the harness, the gate pipeline, and the knowledge substrate.

| Tier | Price | What |
|---|---|---|
| Standard | $5–25K / month | Team-scale, single tenant, shared region |
| Professional | $25–100K / month | Org-scale, dedicated region, custom gates |
| Enterprise | >$100K / month | FDE included, on-prem option, ZK-HDC export |

**12-month target post-Series A:** 30 paying customers, $9M ARR run rate.

- **Per-action billing in USD:** enterprises pay per agent action — compose, gate, persist, route. Settlement in stablecoin or fiat at enterprise preference.
- **Enterprise tiers with SLAs:** uptime guarantees, dedicated support, private gate configurations.
- **Governance and compliance features:** EU AI Act Article 50 audit logs, ISO 42001 attestation artifacts, SOC 2 Type II controls built into the managed tier.

Comparable ARR trajectories: Vercel at $340M ARR; Temporal at $5B with >380% YoY growth in 2025; Confluent at $925M ARR (Q4 2025).

### Stream 2: Enterprise Licensing (forward-deployed engineering)

One Nunchi engineer embedded with the customer for the integration window; per-customer ARR is the metric. The Palantir model applied to agent infrastructure. High-margin because the product does the work; FDEs handle customization.

| Tier | Price | What |
|---|---|---|
| FDE | $1M / yr / customer | Engineer + tooling + support SLA |
| Industry | $2–5M / yr | Regulated verticals (banking, healthcare, government) |
| Protected | $5–10M / yr | EU AI Act Article 50 retrofit + audit trail |

**Beachhead:** 5 enterprise contracts in months 6–12. Reference design for sales scaling.

Additional enterprise licensing line items:
- **ISO 42001 certification support** as a program engagement ($85–150K upfront, $40–60K/yr maintenance) plus managed cloud subscription. **Note:** ISO 42001 carries no Annex ZA, is not a harmonised European standard, creates no presumption of conformity under EU AI Act Article 40. Position as "AI governance scaffolding that demonstrates management system maturity" — not as an EU AI Act compliance certificate. (R9 correction.)
- **EU AI Act compliance tooling** — Article 50 transparency obligations, risk classification, human oversight documentation.
- **SOC 2 Type II + ISO 27001** — target certification within 6 months post-Series A close.

### Stream 3: Protocol Fees (Foundation-stage, long-term)

Activates 18–24 months after Series A. ERC-8183 job marketplace charges 5% on settled jobs. 2% to Foundation treasury, 3% to validators.

| Component | Rate | Recipient |
|---|---|---|
| Fee per settled job | 5% (2% + 3%) | Treasury + validators |
| Treasury | 2% | Audits, grants, security disclosures |
| Validators | 3% | Infra cost + bonded stake yield |

**Year-3 base case:** 100K daily settled jobs × $4 average = ~$7.3M ARR to treasury.

Additional chain-economic streams:
- **Block production revenue** — transaction fees collected by NunchiLabs-operated validators on the sovereign L1 during bootstrapping. Sovereign L1 keeps all fee revenue (no L2/L3 rent).
- **Validator staking economics** — inflation rewards distributed to stakers; NunchiLabs earns from its own validator stake during the early period.
- **Knowledge posting and query fees** — agents pay to write to and read from the on-chain knowledge registry. Denominated in Trust Credits (USD-pegged).

### What the beachhead is NOT

**The beachhead is enterprise support contracts on Roko OSS, not a managed platform product.** Compliance alone will not satisfy a Casado-grade Series A: combined ARR across all four pure-play AI governance vendors is less than $50M (Credo AI ~$4M, Holistic AI ~$8M, Monitaur small, others negligible). Compliance is distribution, not revenue. Knowledge-as-a-Service has no scaled precedent — frame the shared knowledge store as a lock-in mechanism driving managed runtime revenue, not as an independent revenue line.

---

## 4. Token Design: Helium-Hybrid Burn-and-Mint

### What the token is NOT

**NUNCHI does not have demurrage.** This requires explicit clarification because demurrage does exist elsewhere in the Nunchi architecture — specifically on knowledge stored in the on-chain substrate, as an Ebbinghaus-style decay mechanism that prunes stale data accumulating indefinitely. That is a chain storage retention policy enforced by demurrage on substrate records, not a token mechanic. **The NUNCHI token carries no demurrage, no decay, and no time-based penalty.** Token holders are not penalized for holding.

If anyone conflates token demurrage with substrate demurrage, correct them immediately. This misunderstanding would be fatal to investor interest. Use "knowledge decay" or "confidence decay" in investor conversations; reserve "demurrage" for technical documentation.

### Mechanism: burn-and-mint with USD-pegged Trust Credits

Modeled on Helium HIP-141 (May 2023, 18 months in production).

**Trust Credits** — USD-pegged unit ($1 TC = $1 USD), oracle-anchored, redeemable for NUNCHI at burn rate. Pre-purchased by enterprise; drawn down per action. Minted on USD or USDC deposit; burned on action. Treasury-managed float in 90-day Treasury bills. Audit: Trail of Bits + monthly attestation reports.

**NUNCHI** — native token. Stake for ERC-8004 identity tiers. Validator emissions + protocol fee share. Every NUNCHI committed to a job is burned at settlement. Reputation domain failures, gate refusals, and censorship can trigger slashing.

**Why this works for procurement:** enterprise procurement signs a USD invoice, not a token. Validators and identity stakers earn NUNCHI. Users never need to learn the token. The two systems intersect only at the burn-and-mint boundary, which is the only place price discovery happens.

When network demand increases, more NUNCHI is burned to mint Trust Credits. This creates deflationary pressure on NUNCHI supply proportional to actual network usage. Not speculative deflation; deflation tied to the quantity of agent work completed. The Helium HIP-141 net-emissions cap kicks in if NUNCHI supply falls below a floor — small inflationary emission rewards active validators and prevents the network from seizing. Safety valve, not growth mechanism.

### NUNCHI utility (mandatory, not speculative)

1. **Agent identity staking** — agents must stake NUNCHI to register an ERC-8004 agent identity on the network (tiered; see section 5). No stake, no access. Same economic commitment mechanism that makes The Graph's GRT and Chainlink's LINK durable.
2. **Validator stake** — running a validator requires bonded NUNCHI. Validators earn share of Trust Credit fees.
3. **Governance via veNUNCHI lock** — token holders lock NUNCHI for up to 4 years in exchange for vote-escrowed NUNCHI (veNUNCHI), which grants governance rights. Longer locks = more voting power. Discourages short-term extraction.

### Allocation

| Category | Allocation | Notes |
|---|---|---|
| Foundation / ecosystem | 40% | Community incentives, grants, developer programs |
| Team | 18% | Founders and employees |
| Investors | 18% | Series A pro-rata via token warrants |
| Bootstrap rewards (Proof-of-Agent-Activity) | 12% | Early network participants |
| Foundation operations | 8% | |
| Gated airdrop | 4% | Activity-gated; prevents Sybil extraction |

**Total:** 100%. No advisor allocation separate from team. No exchange reserve allocation (exchanges paid via listing fees from Foundation operations budget).

### Vesting

12-month cliff for all insiders (team and investors). 48-month linear vesting after cliff. **5-year total alignment period** (following the Worldcoin July 2024 extension from 3 to 5 years). Foundation ecosystem tokens release on a community-governed schedule, not linear — released as grants are approved, preventing large unlocks that pressure price.

---

## 5. Staking Tiers

ERC-8004 agent identity staking is the primary NUNCHI utility. Stake amounts denominated in NUNCHI but described in approximate USD equivalents at target mainnet prices; actual amounts set by governance prior to mainnet launch.

| Tier | Name | Approximate stake (USD equiv.) | Privileges |
|---|---|---|---|
| 0 | Sandbox | ERC-8004 identity only, no stake | Testing, rate-limited API access; no marketplace participation |
| 1 | Verified Solo | ~$100–500 | Basic marketplace; job bidding up to $1K value |
| 2 | Worker (Standard Operator) | ~$5K–10K | Full marketplace; auction bidding; knowledge posting; schema consumption |
| 3 | Sovereign | ~$50K–250K with delegation support | Priority access; consortium lead; schema authoring; custom gate configurations |
| 4 | Protocol | Core infrastructure agents | Governance |

Delegation supported at Tier 3+: institutional operators can accept delegated stake from smaller NUNCHI holders, sharing rewards proportionally.

### Slashing parameters (initial, ratcheted upward via governance)

| Violation | Slash percentage |
|---|---|
| Missed deadline (first offense) | 1% of staked position |
| Abandoned job | 3% of staked position |
| Quality rejection (repeated) | 2% of staked position |
| Plagiarism | 10% of staked position |
| Result manipulation | 10% of staked position |
| TEE violation (attestation failure) | 10% of TOTAL stake across all domains |

TEE violations carry the highest penalty because they indicate an attempt to subvert hardware-attested computation guarantees that underpin enterprise trust.

**Slash distribution:** 50% to protocol treasury (security bounties + insurance fund), 30% to reporting party (incentivizes honest reporting), 20% burned (deflationary pressure). This distribution prevents slash-farming while maintaining reporter incentive.

### Why this design will not collapse like the 2024–2026 agent token cohort

| Token | Failure mode | Lesson |
|---|---|---|
| **VIRTUAL** (Virtuals Protocol) | Speculative platform without mandatory utility. $5.07 (Jan 2 2025) → ~$0.70 (Apr 2026), –86%. Daily protocol revenue collapsed from $1.02M to $34,792 in 7 weeks. | Do not launch a token before product has revenue. |
| **AI16Z → ELIZAOS** | Force-migrated 1:6 ratio (Nov 2025), supply expanded 6.6B → 11B; exchange delistings; –99.98% drawdown. | Token mergers and supply expansions destroy community trust regardless of technical rationale. |
| **FET → ASI Alliance** | Multi-project merger (Fetch + Ocean + SingularityNET, mid-2024) created governance ambiguity. Ocean withdrew October 2025 alleging undisclosed minting. –94%. | Multi-entity merger needs explicit ownership and clear protocol-vs-product boundaries from day zero. |
| **Bittensor (TAO)** | Reputational corrosion. Covenant AI exit April 12 2026; co-founder accused of dumping 37,000 TAO (~$50M) without disclosure. | Insider vesting must be legally enforced and technically verifiable. |

**What works (surviving token mechanics):** Chainlink LINK (1B fixed supply, mandatory staking for oracle nodes); The Graph GRT (100,000 GRT minimum stake, 28-day thawing, 10% slashing); Filecoin, Arweave (physical utility floors). Common pattern: token utility works when staking has a clear, mandatory purpose tied to service delivery. NUNCHI is designed to be mandatory for agent network participation — no stake, no ERC-8004 identity, no access.

---

## 6. Comparable Economics

### The Terraform reference model (verified)

HashiCorp is the strongest structural precedent for Nunchi's open-core + chain model.

- **Acquisition data:** IBM acquired HashiCorp for $6.4B in cash, closing February 2025 (verified via SEC filings). Revenue at acquisition: $671M FY24 (May 2023–April 2024), growing 23% YoY.
- **BSL transition (August 2023):** trigger for the acquisition, not cause of decline. BSL explicitly enables a managed cloud service to be the moat — exactly Nunchi's structure.
- **Ecosystem lock-in:** 3,500+ providers, 14k+ modules at acquisition. Lock-in started at ~500 providers (2019–2020); became prohibitive at ~2,000 (2022).
- **Terraform's verified module pattern:** 376 modules total in 2018, 42 HashiCorp-verified accounted for >95% of all downloads. AWS modules alone >94%. The lesson: a verification badge on a small curated set does nearly all the discovery work. Plan v1 around 10–20 "Roko Verified" reference adapters.

### Temporal: the closest comparable

- **Funding:** $146M Series C (Nov 2024) at $1.7B → $300M Series D (Aug 2025) at $5B (Reuters Aug 12 2025; Sarah Wang and Raghu Raghuram, NOT Casado).
- **ARR:** not publicly disclosed but estimated $80–120M+ at Series D. 40–60x ARR multiple at the $5B valuation.
- **License:** Apache 2.0 forever; commercial Cloud is the moat.
- **Lifetime executions:** 9.1 trillion. 380% YoY revenue growth in 2025.
- **Customers:** OpenAI runs Codex on Temporal, Replit uses it for agent orchestration, Lovable for AI web dev agent, Snap for backend services, Datadog for pipeline orchestration.

This is the single strongest comp for Nunchi. Same structure (Apache 2.0 runtime + managed cloud monetization), Casado-led at Series C and D, different layer of the stack (workflow orchestration vs agent coordination).

### LangChain: the cautionary tale

- **Funding:** $125M Series B (Oct 2025) at $1.25B (Crunchbase; Forbes).
- **The lesson:** LangSmith retained value by becoming a generic LLM observability product, not by being tied to LangChain. Do not let the commercial product depend on the OSS framework's continued popularity. LangChain bet the OSS on framework lock-in (chains, agents, prompts as proprietary abstractions); Nunchi bets on standards (MCP, A2A, ERC-8004) and verifiability (gate pipeline, signed receipts). The OSS does not lock in users; the chain and the audit trail lock in enterprises.

### dbt Labs: the license transition lesson

- $222M Series D (Feb 2022) at $4.2B; has not raised since.
- 2024 ARR estimated $200M+ (Forbes, Crunchbase).
- License transitions: Apache 2.0 → BSL (Aug 2023) → ELv2 (March 2024) — losing community trust each transition.
- **Lesson:** license transitions are reputationally expensive. Plan structure once, stick with it.

### Gateway position economics: the honest assessment

| Company | Raised | Est. ARR | Model |
|---|---|---|---|
| OpenRouter | Not disclosed | $30–50M run rate | 5.5% credit-purchase fee, no per-token markup |
| Helicone | $4M | <$5M | $79/mo Pro |
| Portkey | $14M Series A | $5–10M | $49/mo Pro |
| Braintrust | Series B at $300M (Casado-led, TechCrunch May 2025; **R4 corrected from earlier $800M figure**) | <$10M | Eval + gateway bundle |
| LiteLLM (BerriAI) | YC W23 | Small | Primarily open-source |

**Honest assessment:** the gateway position alone is a $10–50M ARR business, not a $1B+ business. None of the standalone gateway companies have broken out. The exception: gateways that bundle eval/observability/learning (where Braintrust is positioning, where CascadeRouter's learning loop is an advantage). Cloudflare AI Gateway is bundled with Cloudflare Workers at near-zero pricing — platform threat.

**Strategic implication:** position the Roko gateway as "the gateway with the learning loop," not "another gateway." The gateway is the data-acquisition layer for the agent orchestration platform, and that is the $1B+ business — Temporal at the agent layer.

### Summary of business-model comps

| Company | Valuation | Structure | Lesson for Nunchi |
|---|---|---|---|
| HashiCorp | $6.4B (IBM acquisition) | OSS + BSL cloud + registry | Registry-as-lock-in works; declare BSL upfront |
| Temporal | $5B (Series D, Sarah Wang led) | Apache 2.0 + managed cloud | Closest comp; same structure, different layer |
| LangChain | $1.25B (Series B) | OSS framework + LangSmith SaaS | Cautionary: don't depend on OSS mindshare |
| dbt Labs | $4.2B (Series D, 2022) | OSS + license transition | License transitions are reputationally expensive |
| Story Protocol | $2.25B (Series B, a16z crypto + Polychain) | Dual-asset (software + protocol token) | Token warrant template |
| OpenRouter | Not disclosed | Gateway + credit fee | Standalone gateways cap at ~$50M ARR |
| Braintrust | $300M (Series B, Casado-led; corrected R4) | Eval + gateway bundle | Gateway-with-learning is the right position |

### Infrastructure orchestration multiples

Companies that provide infrastructure orchestration (Temporal, HashiCorp, Pulumi) trade at 30–60x ARR multiples. For comparison: SaaS companies 10–20x; developer tools 15–30x; **infrastructure orchestration 30–60x**. The premium reflects the stickiness of infrastructure. Once an organization builds on your orchestration layer, switching costs are enormous. Platform businesses that define a new coordination layer command 8.2x revenue (Equal Ventures), vs 3.9x for SaaS tools (BVP Emerging Cloud Index).

---

## 7. Series A Target

**Raise:** $20–30M.
**Valuation:** $200–400M post-money.
**Modal range:** $15–35M at $150–250M post-money based on infrastructure-Series-A comps.

### Use of funds (illustrative on $25M raise)

| Category | Amount | % | Notes |
|---|---|---|---|
| Engineering & product | $11.25M | 45% | 20 to 35 engineers — runtime, gateway, chain core |
| Cloud infrastructure | $4.50M | 18% | Multi-region, GPU inference, validator nodes, CDN |
| Security & audits | $2.50M | 10% | Trail of Bits, Halmos, EthSecurity, bug bounty |
| Research | $2.50M | 10% | HDC, ZK, Resonator, ARLC; 4 published papers |
| Enterprise GTM | $2.50M | 10% | 5 FDEs, 3 SEs, 2 partnership leads, zero SDRs |
| Reserve & working capital | $1.75M | 7% | Foundation formation, regulatory, contingency |

**Headcount end of runway:** 35 (20 engineering). **Runway:** 20 months to Phase 3 mainnet. **Dilution:** 10–15% pre-stack adjustment.

### Series A milestones for the raise

- Working self-hosting demo (Roko develops Roko).
- Reproducible cost benchmarks (measured, not projected) — 5-task HAL subset.
- 2–3 design partners using Roko in production under signed Common Paper Design Partner Agreements.
- Chain testnet with simulated settlement.
- EU AI Act compliance feature set documented and partially built.
- $63K bookings in 90 days post-close ($48K Tier 1 ARR + $15K Tier 2 adapter), structurally identical to Temporal's early-2021 position.

### Series A comparables

| Company | Round | Amount | Valuation | Status at raise | Lead |
|---|---|---|---|---|---|
| LangChain | A | $25M | $200M | <$5M ARR | Sequoia |
| CrewAI | Seed+A | $18M | ~$100M | 150 beta enterprise customers | Insight |
| E2B | A | $21M | — | 88% Fortune 100 adoption | Insight |
| Inngest | A | $21M | — | ~$2.5M ARR | Altimeter + a16z |
| Mastra | A | $22M | — | Brex, Indeed, PayPal in production | Spark |
| Dust | A | $16M | — | $1M ARR, 70% WAU/MAU | Sequoia |
| /dev/agents | Seed | $56M | $500M | Pre-product | Index / CapitalG |
| Story Protocol | A→B | $25M→$80M | $2.25B | Dual-asset (software + protocol) | a16z crypto |

### Three valuation lanes

1. **Community proof** ($200–250M post): 50K+ GitHub stars or 10M+ monthly downloads. The LangChain lane.
2. **Revenue proof** ($200–250M post): $1–5M ARR with 70%+ WAU/MAU. The Dust lane.
3. **Team + thesis + marquee partners** ($300–500M+ post): pre-product. The /dev/agents lane ($56M at $500M pre-product).
4. **Crypto-protocol-first** ($2B+): Story Protocol framing. Higher ceiling, different investor base, increased regulatory exposure.

### Step-up trajectories for framing

For follow-on conversations: Decagon $650M → $1.5B → $4.5B in 15 months. Sierra $1B → $4.5B → $10B in 19 months. Harvey $715M → $11B in 28 months. These are not normal — they reflect the current market's willingness to pay for enterprise AI infrastructure with demonstrated revenue. Nunchi's infrastructure position is orthogonal to any single AI product company, capturing value across the entire portfolio of enterprise AI deployments rather than from one specific use case.

---

## 8. Common Paper Design Partner Agreement v1.3

Use Common Paper DPA v1.3 (CC BY 4.0, written by 30+ attorneys, used by Temporal/Snyk). Do not write your own contract.

**Benchmarks from executed agreements:** 49% have 3–6 month terms; 72% require regular feedback; 64% allow private reference and 61% public reference; 43% include a future-discount commitment.

**Cover Page values for Roko:**
- 6-month term
- $12,000 fee paid quarterly ($3K Q1, $9K Q2–Q4 — discounted from $24K list)
- 25% future discount for 12 months post-program (specify trigger: "if Partner signs within 30 days of Term expiry")
- Bi-monthly 1-hour feedback session
- Named architecture lead
- Commitment to ship 2 partner-requested features in first 90 days
- **No exclusivity** — Common Paper template explicitly excludes it
- **IP on feedback owned by Provider** — critical for Series A diligence; LOIs that leave IP ambiguous kill diligence

**Why the fee should not be zero:** a free pilot is psychologically a beta; a $12K contract is a commitment. Common Paper explicitly recommends non-refundable fees. Skin-in-the-game is the Series A signal.

### R8 Common Paper gotchas

1. **Only modify the Cover Page**, not Standard Terms. Standard Terms are incorporated by URL; editing inline breaks the model.
2. **Term + Fees default to "none" if blank.** Both must be explicitly populated.
3. **Section 1.3 + 6 Provider-owns-Feedback IP is load-bearing.** Do not concede if a Partner pushes back.
4. **No exclusivity, no right-of-first-refusal** in the standard. Partners may ask; the answer is no.
5. **30-day termination doesn't address fee refunds.** Add Common Paper's "make fees non-refundable" language.
6. **US-drafted governing law.** For German/EU partners, specify Berlin courts. For US partners, Delaware.

---

## 9. Berlin Grants: EUR 80–150K Non-Dilutive Capital

Berlin location is a structural asset for non-dilutive capital that no other agent-runtime founder can claim.

| Grant | Amount | Deadline | Fit |
|---|---|---|---|
| **NLnet NGI Zero Commons Fund** | EUR 5–50K, scalable up | Next: June 1, 2026 (rolling every two months) | Strong — Rust projects routinely funded (rPGP, Oxigraph, Rauthy, librice). No incorporation required. |
| **Sovereign Tech Fund** (German federal, BMWK) | EUR 23M+ across 60 OSS projects 2022–2024 | Rolling | Plausible but stretched — "currently not looking for user-facing applications." Wait until Roko has 2–3 dependent crates proving "prevalence." Past Rust grants: uutils (EUR 99,060), CycloneDX. |
| **Sovereign Tech Fellowship** | EUR 64–82K/yr employed (TVoD-Bund), 3–12 months | Next opens ~Q1 2027 | For maintainers personally, not orgs. |
| **Rust Foundation Community Grants** | $100K allocation 2026 | 2026 | $1,500/mo + $4,000 travel/equipment. Modest but signals Rust Foundation engagement. |

**Total realistic non-dilutive within 18 months: EUR 80–150K.** Runs in parallel with design-partner revenue, extending runway by ~6–12 months without dilution. Single most underused Berlin advantage.

### Ferrous Systems precedent (Rust-native commercial template)

EUR 25/seat/month or EUR 240/seat/year for binary distributions + LTS + qualified compiler. Sells certainty, not features; OSS remains Apache-2.0 + MIT. "Rust Experts" as fixed monthly retainer. **Bootstrapped since 2018.** IEC 61508 SIL 2 certification of Ferrocene core library shipped December 3, 2025 (release 25.11.0). Confirmed customers: Sonair, Kiteshield. Berlin office: Wallstr. 59, 10179 Berlin Mitte (shared with Slint and KDAB). This is the dominant Rust-native OSS monetization shape — Rust enterprise contracts trend lower-volume but higher-touch than TypeScript/Python equivalents. Anchor here, not on Supabase's PLG model.

---

## 10. Free Tier Design (When Cloud Ships)

Match Supabase's auto-pause-on-inactivity pattern:
- 1 project, 1 deployment
- Auto-pause after 7 days idle
- No credit card required
- Allow OSS self-host of everything with no rug-pull risk
- **Gate operational convenience (auto-scale, monitoring, support) — never gate functionality**

Supabase's auto-pause is the unsung hero of their economics; users who would otherwise abandon do not burn ongoing infrastructure cost.

---

## 11. EU AI Act as Revenue Catalyst

The EU AI Act enters full enforcement on August 2, 2026 — approximately 14 weeks from late April 2026. Acute, time-bounded buying opportunity.

Deloitte's Q1 2026 survey found that only **35.7% of EU managers feel prepared** for AI Act compliance. Only 26.2% have started concrete compliance activities. For a regulation with penalties of €35M / 7% of global turnover for prohibited practices and €15M / 3% for transparency violations including Article 50, this represents enterprise spending trigger with a known date.

**Compliance-as-distribution precedents:**
- Vanta reached ~$220M ARR primarily from SOC 2 automation (the compliance workflow enterprises had to complete anyway).
- OneTrust exceeded $5B valuation from GDPR compliance tooling (2018 enforcement created comparable buying impulse).

**The pattern:** regulation creates a compliance spend category; whoever provides the workflow tooling first captures recurring revenue, because compliance programs do not switch vendors mid-audit cycle.

**Insurance overlay:** Munich Re's aiSure product (via Mosaic) provides coverage of up to EUR 15M for AI Act regulatory fines. Nunchi's compliance attestation artifacts are the evidence base aiSure underwriters require for coverage. Natural integration: Nunchi provides the compliance record; Munich Re provides the insurance backstop. Insurers offer 15–25% premium discounts for ISO 42001 certification.

**Critical R9 correction (re-iterated):** Position ISO 42001 to customers as "AI governance scaffolding that demonstrates management system maturity" — not as an EU AI Act compliance certificate. ~40–50% overlap with AI Act governance requirements; useful scaffolding but not a regulatory shield. The first OJEU-listed harmonised standard will likely be prEN 18286 (currently at CEN Enquiry stage).

---

## 12. The ISFR Expansion (Future Domain)

**This is the long-term expansion lane, not the Series A wedge.** Frame only if the investor explicitly asks "what's the really big vision?"

### Interest rate derivatives market

Interest rate derivatives represent **$668 trillion in notional outstanding** (BIS data, H1 2025). The largest financial market in the world. On-chain interest rate products today are <$100M — a >1,000,000:1 gap.

### Cooperative Clearing as inference

Every interest rate swap, forward rate agreement, or swaption generates a structured computation that looks exactly like agent coordination: multiple parties with different models and information; need for settlement; need for verification; enormous volumes of structured knowledge generated as a byproduct. Nunchi's coordination plane primitives — identity verification, execution settlement, knowledge management, verifiable computation — map directly onto cooperative clearing infrastructure.

### ISFR (Internet Secured Funding Rate)

Validator-computed reference rate, published every 10 seconds:

```
ISFR = 0.60·LENDING + 0.25·STRUCTURED + 0.10·FUNDING + 0.05·STAKING
```

- **Lending (60%):** Aave V3 + Compound V3 supply rates
- **Structured (25%):** Ethena sUSDe yield basket
- **Funding (10%):** Hyperliquid funding, weighted by OI
- **Staking (5%):** ETH staking yield (Lido / native)

Worked example: 0.60·6.20 + 0.25·5.80 + 0.10·7.10 + 0.05·12.40 = 6.775%.

Yield perpetual on ISFR: no expiration, 10x leverage (10% initial / 5% maintenance margin), premium + carry funding components, 800ms solver competition with KKT verification, 3-block (~1.2s) settlement. Smart-contract vault custody with validator slashing.

### Why NOT to lead with this

- Sounds like fantasy from a solo founder with a Rust toolkit.
- Requires the chain to be live, battle-tested, and regulated.
- Requires multi-year financial-institution relationships.
- Investors will dismiss as "boiling the ocean" if presented before developer tools business is proven.

**The correct framing:** *"We're building agent coordination infrastructure. Our beachhead is developer tools. The architecture we're building — verifiable computation, identity, settlement — has applications in financial infrastructure. That's the 10-year expansion path, not the 2-year plan."* See `09-benchmark-business-thesis.md` for the strategic-level treatment.

---

## 13. Series B Path

The Series B thesis requires demonstrating that agent orchestration infrastructure can generate enterprise ARR at Temporal-comparable multiples.

**Target:** $3–5M ARR with 4+ marquee enterprise logos by month 12 post-Series A close. At Temporal's 40–60x ARR multiple, that is a $90–150M Series B at a $1B+ valuation.

### Milestone schedule

- **Month 3:** SOC 2 Type II certification complete. Engages Schellman (Anthropic's auditor) on audit.
- **Month 6:** ISO 27001 certification complete. Two design partner deployments live with signed reference agreements. EU AI Act compliance pilot completed with at least one EU-headquartered enterprise.
- **Month 9:** Protocol specification donated to Linux Foundation AI and Data (LFAI). The "neutral home" move that legitimizes the protocol as industry infrastructure rather than proprietary tooling. Precedents: PyTorch (Meta → Linux Foundation), Kubernetes (Google → CNCF), Hyperledger (IBM → Linux Foundation).
- **Month 12:** 4+ enterprise logos, $3–5M ARR, Series B roadshow.

### Platform multiple criteria (R12)

For platform-tier valuation (8.2x revenue vs 3.9x SaaS tools):
- NRR ≥130%
- Multi-product attach ≥40% within 6 months
- Ecosystem leverage with measurable partner revenue

### Deck commitments for Series B narrative

- 130%+ NRR by Series B (expansion from enterprise support → managed cloud → compliance)
- Second product GA by month 12 with 40% attach rate
- 5+ runtimes supported (not just Roko — LangChain, CrewAI, Mastra, AutoGen integration)
- No customer >15% of ARR (concentration risk mitigation)

**Twilio cautionary tale:** buying breadth without integration drives multiple compression. Twilio's acquisitions added product lines but did not compound. The second product must integrate deeply with the first, not just co-exist in the pricing page.

---

## 14. Money Transmission Risk

Agent wallets that hold and spend funds trigger FinCEN's money-services-business (MSB) classification in the US and equivalent frameworks in EU (CASP under MiCA) and UK. Regulatory pattern increasingly aggressive: OKX settled $500M+ with US DoJ in 2025; Paxful settled $3.5M with FinCEN; first EU CASP enforcement action produced €21.5M fine against Coinbase Europe (November 2025). 49-state money-transmitter licensing is multi-year and costs $2–5M in aggregate compliance overhead even for platforms that prevail.

**Mitigation:** the NUNCHI token is for staking and governance only, not payments. Agent payments flow via x402 and stablecoins through established rails (Tempo, Stripe) — the company does not hold or transmit funds. Trust Credits are an accounting unit, not a payment vehicle. This architecture must be maintained precisely: the moment Trust Credits can be used to pay third parties directly, the MSB classification argument degrades. **Obtain a formal money transmission classification legal opinion before Series A close.**

---

## 15. MiCA and Agent Payments

MiCA (Markets in Crypto-Assets Regulation), fully applicable since December 30, 2024, imposes requirements on stablecoin usage that affect agent payment infrastructure:

- Only MiCA-authorized stablecoins (currently USDC and EURC from Circle, which obtained an Electronic Money Institution license in France) for EU agent payments.
- **Article 23 volume caps:** no more than 1 million transactions per day or EUR 200 million in transaction volume per day for any single stablecoin issuer if not denominated in EU currency. EURC (euro-denominated) exempt; USDC (dollar-denominated) is not.
- Agent payment infrastructure must integrate KYC/AML checks consistent with the Travel Rule (Regulation 2023/1113) for crypto-asset transfers.

---

## 16. Business Model Summary

| Element | Content |
|---|---|
| **Corporate structure** | Dual-entity. NunchiLabs Inc. (Delaware C-corp) for equity, operations, enterprise. Nunchi Foundation (Cayman or Swiss) for protocol governance and token, deferred 18–24 months post-close. Modeled on Story Protocol / PIP Labs. |
| **Licensing** | Apache 2.0 (Roko runtime), MIT (token contracts), BSL with 4-year Apache conversion (managed cloud). |
| **Revenue streams** | Managed cloud ($5–25K Standard / $25–100K Pro / >$100K Enterprise per month); enterprise licensing ($1M FDE / $2–5M Industry / $5–10M Protected); protocol fees (5% on settled jobs, Foundation-stage 18–24 months out). |
| **Beachhead** | Enterprise support contracts on Roko OSS. $24K/yr Tier 1 with zero engineering effort. |
| **Series A target** | $20–30M at $200–400M post-money. |
| **Series B target** | $3–5M ARR + 4 logos by month 12 → $90–150M Series B at $1B+ valuation. |
| **Token** | NUNCHI. Helium-hybrid burn-and-mint. USD-pegged Trust Credits. Mandatory utility (agent identity staking, validator stake, governance). No demurrage on token. |
| **Vesting** | 12-month cliff, 48-month linear, 5-year total alignment (Worldcoin precedent). |
| **Token launch timing** | Defer 18–24 months past Series A. No SAFT, no token sale, no public tokenomics until Foundation forms. |
| **Non-dilutive capital** | EUR 80–150K Berlin grants (NLnet, STF, Sovereign Tech Fellowship, Rust Foundation Community). |
| **Compliance gates** | SOC 2 Type II by month 3; ISO 27001 by month 6; ISO 42001 program (positioned as governance scaffolding, not Article 50 certificate). |
| **MSB risk** | Token for staking/governance only, not payments. Trust Credits as accounting unit. Formal legal opinion before Series A close. |
| **Anti-patterns** | Never relicense OSS core. No percentage-of-savings pricing. No token sale before product. No multi-project token mergers. |
