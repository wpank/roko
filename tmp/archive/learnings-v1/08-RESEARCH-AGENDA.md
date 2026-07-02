# Research Agenda — Topics for Deep Investigation

> Exhaustive list of high-impact topics to research, organized by category. Each entry describes what to investigate and why it matters. Use these as Claude Desktop deep research prompts or as focus areas for team investigation.

---

## A. Fundraise Mechanics and Investor Psychology

### A1. How Series A rounds actually work (process, not strategy)
- Term sheet anatomy for dual-structure (equity + token warrant) raises
- What's in a SAFE vs priced round at $200-400M post? Pro-rata rights, liquidation preferences, board seats
- The actual timeline from first meeting to wire (industry average: 6-12 weeks — what accelerates/stalls it?)
- How many meetings does it take? (a16z reportedly meets 3,000 companies/year, funds ~30)
- Data room preparation — what documents does a16z expect? Financial model? Technical architecture doc? Cap table?
- The role of the associate vs the partner — who screens, who decides, who does diligence?
- Reference checks — who do they call? What questions do they ask? How to prepare references?
- What kills deals in diligence? (Common: founder conflicts, IP issues, cap table problems, customer churn)

### A2. Investor psychology — what converts skeptics
- What makes an investor say "I need to fund this" in the first 5 minutes?
- The role of FOMO vs conviction — which drives a16z specifically?
- How does social proof work at this stage? (Other investors interested? Customer logos? GitHub stars?)
- What makes a founder "backable"? (Domain expertise? Previous exits? Technical depth? Vision clarity?)
- How do crypto-native investors (Dixon/Yahya) and infra investors (Casado) evaluate differently?
- The "non-consensus product, consensus pitch" pattern — concrete examples of how founders executed this
- How do investors evaluate open-source companies differently? (Monetization skepticism, community metrics)

### A3. Pitch deck design and storytelling
- What pitch decks actually closed $20-50M rounds in agent infrastructure? (Any publicly available?)
- Sequoia's recommended deck format vs a16z's — do they differ?
- The role of design quality in pitch decks — does it matter? (Some say boring slides, founder is the show)
- How to present technical depth without losing the room — the "one slide of math" pattern
- Storytelling structure: problem → insight → solution → traction → team → ask — or a different order?
- How to present a dual equity + token structure without triggering "this is a crypto pitch" pattern-match
- How to present "we have no revenue" without losing momentum — the team-thesis lane framing

### A4. Post-raise execution — what happens after the wire
- How do a16z portfolio companies use the platform? (Recruiting, BD intros, marketing, talent network)
- Board meeting cadence and content — what does Casado want to see quarterly?
- How to set milestones that align with Series B expectations ($3-5M ARR, 4+ logos)
- The "announce the round" playbook — blog post, press, hiring signal
- How fast should you hire post-raise? (Common mistake: hiring too fast, burning runway before PMF)

### A5. Comparable deep dives
- Temporal's full fundraise history: how Maxim Fateev pitched "durable execution" as a category
- Confluent's path from Kafka open-source to IPO — the business model evolution
- HashiCorp's journey: MIT → BSL controversy → $6.4B IBM acquisition — what worked and what broke
- Story Protocol's dual-structure: how did a16z crypto structure the equity + token? What precedent does it set?
- Stripe's first year: Patrick Collison's actual activities, first 7 users, how docs evolved

---

## B. Business Model Deep Dives

### B1. Open-source monetization models that actually work
- The "open core" vs "cloud-only premium" vs "marketplace take rate" decision tree
- What percentage of Temporal/Confluent/Supabase users convert to paid? At what price point?
- Hosted vs self-hosted usage split for infrastructure protocols
- How does the "free forever" tier drive enterprise adoption? (Supabase: MIT, no license change)
- Revenue per employee benchmarks for infrastructure companies ($200K-$500K — where to target)

### B2. Token economics for infrastructure protocols (without demurrage)
- Detailed comparison: LINK, GRT, AR, FIL token models — what survived bear markets and why
- The "gas in stablecoins" model (Tempo has no native gas token) — pros/cons/investor perception
- Burn-and-mint equilibrium: how to model token supply under different usage scenarios
- Token vesting schedules that a16z crypto considers standard
- Foundation vs company token allocation — what ratio is standard? (Typically 20-30% foundation, 15-25% team, 15-25% investors, 30-40% community/ecosystem)
- How to avoid the "utility token → security" classification (Howey test, recent SEC guidance)
- What happens when token price drops 90%? (Most AI tokens have) — impact on protocol, validators, agents

### B3. Pricing strategy for agent infrastructure
- Usage-based vs seat-based vs outcome-based — what works for which customer segment?
- The "Sierra model" ($1.50/resolution) vs "Temporal model" (execution-hours) vs "Stripe model" (% of volume)
- Free tier design — what to include that drives adoption without giving away the moat
- Enterprise pricing: ACV benchmarks for agent infrastructure ($50K? $100K? $500K?)
- How to price the managed cloud vs self-hosted — multiplier benchmarks (typically 3-10×)

### B4. Revenue model for the Nunchi chain specifically
- Transaction fee revenue at different adoption levels (1K, 10K, 100K active agents)
- Agent identity registration fees as revenue
- Marketplace take rate (ERC-8183 job market) — what rate maximizes volume, not revenue?
- Knowledge publishing/querying fees — how to set without discouraging participation
- Validator economics — what makes running a Nunchi validator attractive?

---

## C. Product, UX, and Developer Experience

### C1. What makes developer tools feel "premium"
- Deep dive on Linear's design decisions (spring constants, animation timing, keyboard shortcuts, information density)
- Figma's approach to real-time collaboration UX — what makes multiplayer feel magical?
- Raycast's keyboard-first design — how they measure and optimize for speed
- Tailwind's documentation — why it's considered the gold standard for developer docs
- The "dark mode as default" pattern — why developer tools go dark and the specific aesthetic choices

### C2. Agent-specific UX patterns nobody has solved
- Debugging multi-agent systems: what does a brilliant trace visualization look like?
- Cost attribution visualization: real-time meters vs post-run reports vs budget alerts — what's the right pattern?
- Agent learning visualization: how do you show "this agent is getting better"?
- The "agent just failed" experience: from opaque error to actionable insight — what's the 10× improvement?
- Trust calibration displays: showing when an agent is uncertain (metacognitive sensitivity)
- The autonomy slider: progressive trust from "watch everything" to "fully autonomous" — interaction patterns

### C3. Landing page as pitch deck — precedents and optimization
- Do any successful companies use their website as the investor pitch? (Linear's website IS the sell)
- Scroll-driven narrative storytelling: best practices, conversion data, engagement metrics
- Interactive data visualizations in pitch context — what makes them credible vs gimmicky?
- The "live data on the landing page" pattern — showing real chain blocks, real agent activity
- Dark aesthetic for developer products — when does it help vs hurt? (Accessibility concerns?)

### C4. Onboarding and time-to-first-value
- Sub-60-second TTV: how did Lovable, Bolt, Cursor achieve this? Specific interaction flows
- Template-first vs blank-canvas onboarding — A/B data on which converts better
- The "hello world" for an agent protocol — what should `nunchi init` produce?
- Progressive disclosure: how many features to show at each stage (first minute, first hour, first day, first week)
- Onboarding for the dashboard specifically — what should the first-time app experience look like?

### C5. The "impossible to go back" moment
- For each successful dev tool, what's THE moment? (Stripe: never touch payment APIs again. Vercel: preview deployments. Linear: speed.)
- Can you engineer this moment deliberately? How?
- For Nunchi: is it the cost meter? The gate pass? The knowledge transfer? The crash recovery? The chain deposit?
- User research methodology for identifying the "aha moment" in early products

### C6. Dashboard architecture for the refocused product
- What should the 3-4 essential views be for the pitch demo?
- How to build a "demo mode" that shows the thesis without requiring real workloads
- Terminal-in-browser patterns (xterm.js, Warp-style rendering) for the embedded cost comparison
- The "StarCraft minimap" for agent coordination — has anyone actually built something like this?

---

## D. Growth and Distribution

### D1. Developer distribution channels that work in 2026
- Getting scaffolded by default in AI coding tools (Cursor, Claude Code, Lovable, Bolt) — the actual mechanism
- The "Supabase in 40% of YC" phenomenon — how did this happen? (Templates? Recommendations? Personal relationships?)
- MCP server distribution as a growth channel — does publishing useful MCP servers drive protocol adoption?
- ACP registry as distribution — how many developers actually browse the ACP registry vs get recommendations?
- npm/PyPI as discovery — what drives package discovery? (Stars? Downloads? README quality? Blog posts?)

### D2. Content strategy for developer protocols
- What developer blog posts go viral? (Technical depth vs accessibility — the optimal point)
- The "workshop that gets 300K views" — how to prepare, deliver, and amplify (MCP's Mahesh Murag example)
- Podcast strategy: should the founder do Latent Space, Lenny's, Lex Fridman? Which converts to adoption?
- Twitter/X strategy for a developer protocol — founder voice vs company account vs both?
- Conference talk strategy: which conferences matter? (AI Engineer Summit, ETHGlobal, AGI House, NeurIPS)

### D3. Community building from zero
- The first 100 contributors — where they come from and what motivates them
- Open-source contributor incentive structures (bounties, grants, recognition, swag — what works?)
- Hackathon design that drives lasting adoption (not just weekend projects that die)
- The "founding member" program — giving early adopters elevated status and access
- Measuring community health: which metrics predict long-term success? (Second-PR rate, issue resolution time, outside-org PRs)

### D4. Enterprise sales motion for agent infrastructure
- The forward-deployed engineering model: Palantir → Sierra → Harvey → Nunchi — how to adapt the playbook
- How long does an enterprise pilot take? (30 days? 90 days? What's realistic for agent infra?)
- Who is the buyer? (VP Engineering? CTO? Head of AI? Platform team lead?)
- The "champion" problem: who internally advocates for a new agent protocol?
- Enterprise procurement: how to navigate security review, legal review, vendor assessment
- SOC 2 / ISO 42001 — when do you need them? (Before first enterprise customer or after?)

### D5. Ecosystem flywheel design
- The "two-sided marketplace" dynamics for a Cell/Graph marketplace — what triggers the flywheel?
- How many published Cells/Graphs does the marketplace need before network effects kick in?
- Creator economics: what makes marketplace creators stay? (Revenue? Recognition? Tooling? Both?)
- Cross-side network effects: how does more developer usage create more enterprise demand?

---

## E. Technical Deep Dives Not Yet Covered

### E1. Simplex consensus deep dive
- Chan & Pass original paper (IACR 2023/463) — what are the actual guarantees?
- Shoup's "Sing a Song of Simplex" (DISC 2024) — what does it add?
- Tempo's production implementation (Commonware library) — performance data
- Solana's Alpenglow upgrade (Votor = Simplex-based) — timeline and implications
- How to achieve 50ms blocks with geographically clustered validators — specific topology design
- Comparison: Simplex vs Tendermint/HotStuff/Narwhal-Tusk/Bullshark for agent workloads

### E2. reth/revm fork engineering
- How to fork reth for a custom EVM chain — the actual engineering steps
- Custom precompile development in revm — what's the API? How do you test?
- Gas metering for custom precompiles — how to calibrate fairly
- Production reth forks: Base, Optimism, Taiko, MegaETH — what patterns do they share?
- The ExEx (Execution Extensions) framework — can it replace full forking?
- Timeline estimate: competent team, custom EVM chain from reth fork — 6 months? 9? 12?

### E3. ZK-HDC engineering specifics
- Circom + Groth16 + Poseidon-2 for Hamming distance proofs — step-by-step implementation
- Constraint budget: ~12K R1CS (10K Poseidon commitment, 2K Hamming math) — can this be reduced?
- Client-side proving performance: laptop <1s, smartphone <2 min — verified on what hardware?
- On-chain verifier deployment: ~250-350K gas — what does this cost at different gas prices?
- The SP1 `hdc_xor_popcnt` precompile idea — is 1-2 engineer-months realistic?
- Fuzzy PSI over HDC vectors for private agent matching — current academic state vs engineering readiness

### E4. HDC in production — the honest assessment
- IBM Zurich's PCM work — where is it now? Any commercial deployment?
- Intel Loihi 2 and HDC — what workloads actually run on it?
- BrainChip Akida — commercial status, real deployments?
- The "HDC for agent workloads" thesis: has anyone validated this beyond academic papers?
- Performance benchmarks: HDC similarity search vs FAISS/ScaNN/Milvus float-vector alternatives on real agent data
- When would you NOT use HDC? What problems is it genuinely bad at?

### E5. Active inference in practice
- VERSES AI: 18 months, one investment firm customer, $819K revenue — what went wrong?
- pymdp and RxInfer.jl — are they production-ready or research tools?
- The gap between "active inference explains cognition" and "active inference improves agents" — how big?
- ODAR (EFE routing) — has anyone replicated the 84.4% accuracy at 82% lower compute?
- When should you use active inference vs simpler approaches (bandits, RL)? Decision framework.

### E6. Stigmergy at scale — real evidence
- The ρ≈0.23 density threshold — has this been replicated beyond the one paper?
- Polymarket/DeFi MEV bots as real-world stigmergic agents — what actually emerged?
- How does stigmergic coordination compare to direct messaging at 10, 100, 1000 agents?
- The "diversity collapse" problem — does stigmergy make it worse or better than direct coordination?

---

## F. Regulatory and Legal

### F1. EU AI Act implementation specifics
- Article 50 transparency requirements: what exactly must an AI agent disclose? Technical implementation
- The Code of Practice on Transparency (expected June 2026): what will it require?
- High-risk AI classification: do autonomous agents qualify? What's the threshold?
- FRIA (Fundamental Rights Impact Assessment) requirements for high-risk systems
- Conformity assessment procedures: self-assessment vs third-party? Cost? Timeline?

### F2. Agent legal identity and liability
- Can an AI agent be a legal entity? (Wyoming DAO LLC? Swiss Foundation? Singapore structure?)
- Who is liable when an autonomous agent causes financial harm? (Operator? Developer? Protocol?)
- The "agent as employee" framing — employment law implications
- Agent identity and eIDAS 2.0 digital identity wallets — convergence opportunity?
- Insurance: Munich Re aiSure, HSB AI Liability — what do policies cover? What's excluded?

### F3. Money transmission and agent payments
- FinCEN MSB classification for agent wallets — specific triggers and thresholds
- State money-transmitter licensing (49 states) — cost, timeline, and the "licensed partner" workaround
- GENIUS Act (July 2025) implications for stablecoin-based agent payments
- Travel Rule compliance for agent-to-agent transfers ≥$3,000
- The x402 Foundation approach to regulatory compliance — what structure did they use?

### F4. IP and open-source legal
- MIT vs Apache 2.0 vs BSL for different components — which license for what?
- Patent strategy for protocol companies — file or don't file? (Defensive publications?)
- Contributor License Agreements (CLAs) for open-source protocols — necessary evil or community killer?
- Trademark protection for "Nunchi" — jurisdiction, cost, timeline
- The OpenTofu/Terraform split precedent — what license decisions prevent this?

---

## G. Market Intelligence and Competitive Dynamics

### G1. Detailed competitor profiles
- **Tempo** deep dive: team, architecture, partnerships (Visa, DoorDash, Stripe), roadmap, weaknesses
- **0G Labs** deep dive: $359M capital, what they've actually shipped, where they're headed
- **Nava** deep dive: $8.3M seed, architecture, differentiation, team
- **Olas** deep dive: 361 daily agents, Mech marketplace economics, Vitalik connection, limitations
- **ai16z/ElizaOS** deep dive: chain plans, timeline, community, technical approach
- **RNWY and Chitin.id**: agent identity credential competitors on Base — what exactly have they built?

### G2. Adjacent market dynamics
- Agent identity as a market: Keycard ($38M), Catena Labs ($18M), Astrix ($85M), Oasis ($120M) — what are they building?
- Agent payments: x402 Foundation, Stripe ACP, Google AP2, Visa Trusted Agent Protocol — market structure
- Agent observability: who's building the "Datadog for agents"? (Langfuse, Helicone, Portkey, Braintrust)
- Agent testing: AgentDojo, AgentHarm, MAST — is there a company here?
- The "agent infrastructure" venture map: who's funded, at what valuation, in what lane?

### G3. Timing and market dynamics
- When does the agent coordination market tip from "nice to have" to "must have"?
- Leading indicators: what metrics signal the inflection? (Enterprise agent failures reaching press? Regulatory enforcement?)
- The "crossing the chasm" framework applied to agent infrastructure — are we in early adopters or early majority?
- What catalysts could accelerate the timeline? (Major agent incident? Regulatory mandate? Flagship customer public case study?)

---

## H. Narrative, Brand, and Communication

### H1. Category creation and naming
- How did "cloud computing," "DevOps," "data streaming," "durable execution" become categories?
- The Play Bigger framework applied to "agent coordination" — who owns the category? Can it be owned?
- Naming: "the trust layer" vs "the coordination layer" vs "the identity layer" — which sticks?
- The role of a manifesto/thesis document in category creation (Temporal's blog, Confluent's Kafka whitepaper)

### H2. Narrative for different audiences
- For a16z Casado: the OpenFlow/Nicira pattern (control plane abstraction)
- For a16z Dixon: the ERC-20 composability pattern (protocol-level value capture)
- For enterprise buyers: the cost/reliability/compliance story
- For developers: the DX/tooling/community story
- For crypto-native: the on-chain identity/reputation/marketplace story
- How to tell ONE coherent story that resonates with all five audiences

### H3. Public communication strategy
- When to go public (announce, blog, tweet) vs when to stay quiet
- The "stealth" vs "building in public" tradeoff — what's right for a protocol company?
- How to handle the "crypto company" label if you don't want it
- Press strategy: which journalists cover agent infrastructure? (Kate Clark, Connie Loizos, specific AI/crypto beats)
- The role of a "thesis document" (like the Ethereum yellowpaper or the Bitcoin whitepaper) — should Nunchi have one?

### H4. Brand and visual identity
- The ROSEDUST design language — is it distinctive enough? Does it scale to documentation, CLI, packaging?
- "Observe. Predict. Compound." vs "The model is the same. The system is the variable." — tagline testing
- Logo and wordmark considerations for a protocol that appears on chain explorers, SDK packages, and slide decks
- The relationship between "Nunchi" (project/chain) and "Roko" (runtime) in branding — one brand or two?

---

## I. Team and Organization

### I1. Hiring the first 10 people
- What roles first? (Research6 says FDEs, then technical writer. What else?)
- The "founding engineer" archetype for a protocol company — what to look for
- How to recruit from a16z's talent network post-raise
- Compensation benchmarks for agent-infrastructure startups in 2026
- The "we can't afford senior engineers yet" problem — how to hire great juniors

### I2. Forward-deployed engineering model
- How Palantir's FDE model evolved (Echo teams + Delta teams)
- How to prevent FDEs from becoming professional services / consulting
- The "gravel road to paved highway" loop — when does FDE custom work become product?
- FDE compensation and career path — how to make it not feel like second-class engineering
- Budget: ~$1M/year for 4 FDEs — is this the right allocation of Series A capital?

### I3. Open-source governance
- When to set up a foundation (MCP: 13 months after launch)
- The Linux Foundation AAIF (Agentic AI Foundation) — should Nunchi join? Cost? Process?
- Maintainer burnout and the "bus factor" — how to make the project sustainable
- The "benevolent dictator for life" vs "committee" governance model — what works for protocols?
- Contributor pipeline: how to convert users → occasional contributors → core maintainers

---

## J. Technical Infrastructure for Launch

### J1. SDK architecture and release engineering
- Monorepo vs polyrepo for TypeScript + Python SDKs
- Code generation from OpenAPI/protobuf vs hand-written SDKs — tradeoffs
- Release automation: how to do lockstep TS + Python releases reliably
- The conformance test suite: what it should cover, how third-party SDKs self-certify
- Documentation generation: from code comments? Separate docs site? Both?

### J2. Testing and benchmarking infrastructure
- How to build a credible cost-comparison benchmark (SWE-bench Pro, HAL format)
- Reproducibility: how to make benchmark results independently verifiable
- CI pipeline for the protocol: what tests run on every PR? (Conformance, integration, gas benchmarks?)
- Load testing for the chain: how to simulate 10K agents on testnet
- The "benchmark fraud" problem (DGM faked scores, Cognition stopped reporting) — how to build trust

### J3. Infrastructure for the demo
- The side-by-side terminal demo: technical architecture (Docker? Live servers? Pre-recorded fallback?)
- Making the demo work reliably in a meeting room (hotspot backup, pre-warmed caches, frozen Docker image)
- The "investor picks the task" interaction — how to make it feel genuine while being reliable
- Cost meter visualization: real-time ticking, running total, comparison overlay — technical implementation
- Recording and sharing the demo: can it be a video on the website too?

---

## K. Emerging Opportunities and Wildcards

### K1. Agent identity for compliance (the biggest near-term opportunity?)
- EU AI Act Article 50 + eIDAS 2.0 = forced demand for agent identity. How big is this?
- If every AI agent interacting with EU citizens needs transparent identification by August 2, what does that market look like?
- First-mover advantage in compliance: did any company successfully build a business on being "the compliance layer"?
- The "compliance-as-distribution" playbook — make compliance easy, capture the market via requirement

### K2. The "agent operating system" thesis
- What happens when agents become the primary interface to computers? (Not CLI, not GUI — agent)
- The transition from "human uses tool" to "agent uses tool on human's behalf" — where is it happening fastest?
- Implications for operating system design, application architecture, and interface design
- The iPhone moment for agents: what does it look like and when does it happen?

### K3. Cross-chain agent coordination
- Agents that operate on multiple chains simultaneously (Ethereum + Base + Nunchi + Arbitrum)
- Cross-chain reputation: how to make reputation portable while keeping it meaningful
- Interoperability: IBC, Hyperlane, LayerZero — which pattern works for agent coordination messages?
- The "chain-agnostic agent" thesis — is it possible or do agents specialize per chain?

### K4. Agent-to-agent economics
- What does a healthy agent-to-agent marketplace look like? (Not speculation — real services)
- Pricing agent labor: fixed price, auction, negotiated — what mechanism works?
- Agent reputation as a pricing signal — how much does reputation affect willingness to pay?
- The "agent gig economy" vs "agent employment" models — which pattern emerges?

### K5. The intersection of agents and physical world
- Robotics + agent coordination: what happens when software agents control physical robots?
- IoT + agent coordination: smart buildings, industrial automation, supply chain
- Autonomous vehicles as agents: what coordination primitives do they need?
- HDC for robotics: binary vectors for sensor fusion, motor control, spatial reasoning

### K6. Privacy-preserving agent coordination (post-Valhalla)
- Even without Valhalla, there's a market for private agent coordination
- Confidential computing (TEEs) for agent workloads — practical or performant?
- Multi-party computation for agent decision-making — any real applications?
- The GDPR right-to-erasure applied to agent memory — how do other systems handle this?

### K7. What happens when agents write most code?
- GitHub Copilot / Cursor / Claude Code trajectory — what percentage of code will be AI-written in 2027? 2028?
- Implications for software engineering as a profession
- Implications for code quality, testing, verification
- Implications for open-source contribution patterns
- The "agent-written code needs different tooling" thesis — is it true?

### K8. The academic frontier
- Papers to write: the Raft-style "understandable agent coordination" paper
- Labs to collaborate with: which academic groups would strengthen the thesis?
- The NeurIPS 2026 / USENIX ATC 2027 pipeline — specific deadlines and approaches
- The role of formal methods (TLA+, Coq) in credibility — does having proofs matter for adoption?
- How to make the HDC + active inference + stigmergy combination a recognized research direction

---

## L. Things That Could Change Everything

### L1. What if frontier models get much cheaper much faster?
- Epoch AI projects 5-10× annual cost decline. What if it's 100×?
- Does the cost-reduction wedge still work if inference is nearly free? (Pivot to coordination/trust)
- At what price point does multi-agent coordination become free enough to be universal?

### L2. What if a major agent failure makes headlines?
- Replit's database deletion was bad. What if the next one costs $100M?
- How would a major agent-caused financial loss change the regulatory timeline?
- Is there an opportunity in being "the safety layer" if this happens?

### L3. What if OpenAI/Anthropic/Google ship native agent coordination?
- Model providers adding orchestration features to their SDKs — how to stay relevant
- The "your features get absorbed into the platform" risk — how Stripe survived credit card companies, how AWS survived Google Cloud
- Protocol-level positioning as defense: "they adopted our standard" vs "they replaced our product"

### L4. What if the crypto market crashes again?
- Impact on the token, on fundraising, on the chain thesis
- How to maintain credibility in a bear market
- The "building through the bear" narrative — who did this successfully? (Ethereum 2018-2020)

### L5. What if HDC doesn't work for agent workloads?
- Research7 flagged this: "no commercial traction yet"
- Fallback: use standard embeddings but keep the ZK-proof capability
- How to design the architecture so HDC is swappable, not load-bearing
- The honest assessment: is HDC a genuine advantage or a 6-month research rabbit hole?
