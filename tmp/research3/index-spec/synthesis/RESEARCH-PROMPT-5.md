# Research Prompt 5: Execution-Critical Gaps, Technical Deep-Dives, and the First 30 Days

## Context

This is the fifth research iteration. Each round has progressively narrowed from theory to execution:

**Round 1** established ISFR-YBS as the lead product, UK BMR Cat-6 as the regulatory path, five named competitors, 10 partnership targets, and agent-attested data sources as the compound thesis.

**Round 2** mapped 12 research directions across agent capabilities and identified five highest-leverage findings: Binius64 ZK proofs native to HDC, categorical foundations as load-bearing public claim, Nayebi 5-head provable corrigibility, V-JEPA 2 + HyperDUM unified perception, and Memp + CycleQD targeting METR's 8-hour horizon. Confirmed the 64-agent plateau and ρ≈0.23 communication density threshold.

**Round 3** shifted to formal limits and failure modes: 5 compounding loops, 8 arena-of-one capabilities, 15 impossible results constraining the design space, production agent economics validated, hard "no" list established, self-bootstrapping state of the art mapped, emergent collusion proven, time-series foundation models production-ready.

**Round 4** grounded everything in May 2026 operational reality:
- **90-day window** before Microsoft + Google + AWS + OpenAI Symphony fully consume horizontal agent orchestration
- **Canonical interop stack locked**: MCP + A2A + x402 + ERC-8004 — compose, don't compete
- **Security-first wedge validated**: Replit incident (1,206 records deleted), GTG-1002 (state-actor jailbreak via task decomposition), Vercel/Context.ai breach (OAuth token theft) — every enterprise sales call now asks "how does your protocol prevent the Replit incident?"
- **Cost wedge calibrated**: 7-10× stacked floor (cache + tier-routing + loop discipline + batching), 20× with DeepSeek V4 substitution, 30× requires aggressive quality tradeoff. Lead with 10×.
- **EU AI Act Article 50 deadline August 2, 2026** — binding transparency + provenance obligations. Colorado AI Act June 30. ISO 42001 becoming enterprise table-stakes.
- **NeurIPS 2026 deadline May 6** — missed (as of this prompt). Next windows: AAMAS, OSDI, ICLR 2027.
- **Token waste empirically measured**: 70% waste in production agent loops; naive 20-step loop rebills context quadratically; bounded retries + scope-limited specialists cut 50-70%
- **10K-agent studies**: AgentSociety (10K agents, 5M interactions) showed echo-chamber polarization. Project Sid PIANO hit infrastructure ceiling at 1,000. Polymarket: 30%+ active wallets are AI agents.
- **Dark horses identified**: Sycamore ($65M seed, ex-Atlassian CTO, direct competitor), Sakana ShinkaEvolve ($200M Series B, embedded in Claude Code), VERSES AXIOM (beat DreamerV3 by 60%), NeoCognition ($40M seed, Ion Stoica)
- **Agent liability insurance exists**: Munich Re (March 2026), Armilla AI (Lloyd's-underwritten), Google + Beazley/Chubb — ISO 42001 firms get 15-25% discounts
- **Memory contamination is now OWASP ASI06**: benign cross-user contamination proven (arXiv:2604.01350), memory poisoning 40→80%+ success (arXiv:2603.20357)
- **Spec format validated**: short (<50 pages), distinctively named, TLA+/P safety model in appendix, reference implementation pre-launch, "7 lines of code" hero demo

The synthesis documents (01-21) contain the full consolidated context. Read ALL of them before starting.

---

## What I need you to research now

Round 5's focus is **execution-critical gaps** — the specific unknowns that block the first 30 days of actual building. Round 4 produced a 90-day roadmap. This round stress-tests it by asking: "For each action item in Days 0-30, what exactly do you need to know to start typing code Monday morning?"

### A. The Spec — What Exactly Goes In It

Round 4 said "ship core spec (<50 pages) by Day 30." But what IS the spec?

1. **Protocol message format — concrete design** — Round 4 said "principal-binding and delegate-binding distinct header fields." Research the actual message envelope designs of:
    - MCP's JSON-RPC messages (exact schema, required fields, extension points)
    - A2A's AgentCard + TaskObject (exact schema)
    - OpenAI Symphony's orchestration primitives (what shipped April 27, 2026?)
    - Microsoft Agent Framework 1.0's message format (what shipped April 3, 2026?)
    - What fields MUST the roko protocol add that none of these have? (HDC fingerprint, attestation hash, cost-tracking, stigmergic trace ID, reputation score)
    - What's the minimal message envelope that is: (a) backwards-compatible with MCP, (b) carries HDC/attestation metadata, (c) supports principal/delegate separation?

2. **Transport layer — what to build on** — Research:
    - Does MCP use HTTP, WebSocket, stdio, or all three? What's the production transport?
    - Does A2A mandate HTTP or allow alternatives?
    - What transport does Temporal use for activity dispatch? Can roko messages ride Temporal's transport?
    - What's the right transport for stigmergic traces (pub/sub? shared log? DHT?)
    - Should the spec mandate a transport or be transport-agnostic?

3. **Identity and auth — the non-negotiable** — Round 4 flagged this repeatedly. Research:
    - NIST NCCoE's "agent identity and authorization" concept paper (April 2026) — what exactly does it recommend?
    - SPIFFE for enterprise agent identity — what's the integration cost?
    - How does ERC-8004 agent registration map to SPIFFE/DID? Has anyone bridged them?
    - What's the minimal auth flow for "agent A calls agent B's tool" that satisfies: scoped, time-bound, revocable, non-shared credentials?
    - OAuth 2.1+PKCE (MCP's choice) vs mTLS vs DID-based auth — which one for Day 1?

4. **Safety primitives — what goes in the spec vs. what's implementation** — Research:
    - What safety properties should be in the TLA+/P model? (deadlock-freedom? bounded-message-delivery? no-privilege-escalation? no-unattended-destructive-action?)
    - What does AWS's experience with TLA+ say about which properties catch real bugs vs. which are academic exercises? (CACM 2015 paper)
    - What's the minimal set of safety primitives that prevents the Replit incident, GTG-1002 jailbreak, and Vercel/Context.ai token theft?
    - How do you spec "default-deny destructive grants" in a way that's protocol-level, not implementation-level?

5. **Spec naming — the identity question** — Round 4 said "3-5 letter pronounceable acronym with meaningful expansion" and "test cold-recall after 24 hours on 20 developers." Research:
    - What names are already taken in the agent protocol space? (MCP, A2A, ACP, AGNTCY, Symphony, AgentOS, etc.)
    - What naming patterns worked (MCP = "Model Context Protocol", REST, GraphQL, gRPC) vs. failed (AGNTCY, WS-*, WSDL)?
    - Is the name "roko" viable as the protocol name, or does it carry too much Roko's Basilisk association?
    - What's the "USB-C for AI" one-sentence metaphor for this protocol?

### B. The Reference Implementation — Architecture Decisions

6. **Temporal integration vs. custom executor** — Round 4 said Temporal has "no historical track record of losing" the durable-execution category ($5B, 9.1T lifetime actions, OpenAI Codex uses it). Research:
    - What's Temporal's SDK for Rust? (temporal-sdk-core — is it production-ready?)
    - How do you model the roko universal loop (query→score→route→compose→act→verify→write→react) as a Temporal workflow?
    - What's the latency overhead of Temporal per activity dispatch vs. direct function call?
    - Does Temporal's deterministic replay conflict with non-deterministic LLM calls? How do others handle this?
    - What's the alternative if Temporal Rust SDK isn't ready? (custom, restate.dev, inngest?)

7. **HDC library selection** — The stack depends on 10,240-bit binary HDC vectors. Research:
    - What production-quality HDC/VSA libraries exist in Rust? (torchhd is Python)
    - What operations are needed? (bind, bundle, permute, similarity, encode, random-hypervector-generation)
    - What's the state of HDC hardware acceleration? (Can you use SIMD popcnt for Hamming distance?)
    - Has anyone published HDC fingerprinting of code/text at the scale roko needs (millions of fingerprints)?
    - What's the memory footprint of 1M 10,240-bit vectors?

8. **LLM provider abstraction — the tier-routing layer** — Round 4 showed 100× price spread between DeepSeek V4-Flash and GPT-5.5. Research:
    - What existing Rust crates abstract multiple LLM providers? (Is there an equivalent of LiteLLM for Rust?)
    - How do you implement tier routing (frontier for hard tasks, cheap for easy tasks) without a classifier that itself costs tokens?
    - What's the latency of provider-switching (cold connection to a new API endpoint)?
    - How do you handle the Anthropic rate-limit problem (190 RPM for Pro, sliding-window enforcement)?
    - What's the circuit-breaker pattern for correlated 429s across providers?

9. **Stigmergic trace storage** — The coordination mechanism depends on shared traces with TTL, provenance, and scope. Research:
    - What's the data model for a stigmergic trace? (HDC address, value, timestamp, TTL, provenance tag, scope, author agent ID?)
    - What storage backend? (Redis with TTL? Custom append-log? Temporal durable state?)
    - How do you prevent trace pollution without central authority? (Cryptographic signing? Reputation-weighted writes?)
    - What's the read/write throughput needed for 256 agents at ρ≈0.23 communication density?
    - How does this scale to 10K agents with the 64-agent cluster sharding strategy?

### C. The "7 Lines of Code" Demo — Making It Real

10. **Hero demo implementation** — Round 4 identified three candidate demos. For the recommended one ("composable-agent-network snippet"), research:
    - What's the actual API surface needed? (How many functions/types must exist for a 7-line demo to work?)
    - How do existing frameworks handle this? (LangGraph's "hello world" is how many lines? CrewAI? AutoGen? DSPy?)
    - What's the equivalent demo for MCP (first MCP server is ~30 lines of Python)?
    - What runtime must exist behind the scenes for the 7-line demo to execute? (Process management? LLM connection? Tool registration?)
    - Can the demo run without any API keys (using a mock LLM or hosted sandbox)?

11. **Hosted sandbox — zero-install trial** — Round 4 said "sandbox/mock server reachable without signup." Research:
    - How do Replit, CodeSandbox, StackBlitz, and Gitpod handle free-tier sandboxes?
    - What's the cost per sandbox session? (Compute, memory, egress)
    - Can you run the demo in a WebContainer (StackBlitz-style) to avoid backend costs?
    - What's Val Town's model? (They do agent sandboxing — what's their infrastructure?)
    - What's the security model for running untrusted agent code in a sandbox?

### D. Launch Partners — Who and How

12. **The 20+ named partner list** — Round 4 said "20+ named launch partners publicly listed" by Day 30. Research:
    - How did MCP get Block + Apollo as launch partners? Was it commercial agreement, open-source contribution, or just early adoption?
    - How did A2A get 50 partners on day one? Were they real integrations or letters of intent?
    - What categories of partners are needed? (Cloud providers, agent frameworks, enterprise tools, academic labs, DeFi protocols?)
    - Which of the Round 1 partnership targets (Aave, Lido, Pendle, CF Benchmarks, etc.) could be Day 1 launch partners vs. which require the product to exist first?
    - What's the ask for each partner category? (Logo usage? Integration? Co-announcement? Contribution?)

13. **The MCP/A2A adapter — compose don't compete** — Round 4's key insight is that new protocols must compose with the locked-in stack. Research:
    - What does an MCP adapter look like? (Translate roko messages to MCP JSON-RPC calls? Or expose roko capabilities as an MCP server?)
    - What does an A2A AgentCard extension look like? (Add roko-specific capabilities to an A2A AgentCard?)
    - Has anyone built a multi-protocol adapter/bridge? (MCP↔A2A? MCP↔AGNTCY?)
    - What's the impedance mismatch? (MCP is tool-centric, A2A is task-centric, roko is signal-centric — how do you map between them?)
    - What breaks when you bridge? (Semantics, auth scopes, error handling, streaming?)

### E. Security-First Wedge — The Differentiation

14. **Scoped, time-bound, revocable agent credentials — implementation spec** — This is the security wedge. Research:
    - How does Anthropic's Claude Code handle tool permissions? (What's the actual permission model?)
    - How does GitHub's fine-grained PATs work? (Scoped to repo, read/write, expiring — is this the model for agent credentials?)
    - What's AWS IAM's session-token approach and can it be adapted for agent-to-agent calls?
    - How do you implement "per-tool credentials" when the tool is on a remote agent? (Token embedding? Capability URI? Macaroon?)
    - What's the minimal credential format that prevents: (a) credential theft propagation, (b) privilege escalation, (c) indefinite persistence?

15. **Default-deny destructive grants — the Replit prevention** — Research:
    - What operations should be classified as "destructive"? (Database writes? File deletion? External API calls? Message sending? Payment initiation?)
    - How do you classify operations as destructive at the protocol level (not just implementation)?
    - What's the approval gate UX? (Synchronous human-in-the-loop? Asynchronous approval queue? Auto-approve below threshold?)
    - How does Linear's "delegation-not-assignment" pattern implement this?
    - What's the latency cost of approval gates in a multi-agent workflow?

16. **OWASP ASI06 memory poisoning defense** — Research:
    - What's the exact specification of OWASP's ASI06 "Memory & Context Poisoning"?
    - What defenses are recommended? (Provenance tagging? Scope isolation? TTL? Cryptographic integrity?)
    - How does this interact with the stigmergic trace design? (Traces ARE shared memory — how do you prevent poisoning without destroying the coordination mechanism?)
    - What's the monitoring/detection approach for memory contamination?
    - Has anyone implemented ASI06 defenses in production?

### F. The Academic Contribution — What's Publishable Now

17. **The citable spec as intellectual contribution** — NeurIPS was missed. What's next?
    - AAMAS 2027 deadline — when is it? What track?
    - OSDI 2026 — when is the deadline? Is a protocol-systems paper appropriate?
    - What workshop papers could be submitted faster (NeurIPS 2026 workshops, ICML 2026 workshops)?
    - What's the minimum viable paper? (Spec + benchmark on 3 tasks showing 10× cost reduction with model held constant?)
    - Can the categorical-foundation claim be a standalone workshop paper at NeurIPS 2026 workshops?

18. **Benchmark suite design** — Round 4 said "publish reference benchmark suite with leaderboard." Research:
    - What makes a good benchmark suite? (GLUE had 9 tasks, SWE-bench has one repo, GAIA has 3 levels)
    - What existing agent benchmarks should be included? (GAIA, SWE-bench Verified, tau-bench, AppWorld, WebArena?)
    - What novel benchmarks should be added to demonstrate composability and collective intelligence?
    - What's the infrastructure for running a public leaderboard? (Papers With Code? Custom? GitHub-based?)
    - How do you prevent benchmark gaming/overfitting?

### G. The ISFR-YBS Track — Parallel Execution

19. **Yield-bearing stablecoin data availability** — Round 1 identified the product but Round 4 didn't spec the data pipeline. Research:
    - What yield-bearing stablecoins exist as of May 2026? (sDAI, sfrxETH, stETH, Ethena sUSDe, Sky USDS, Usual USD0, others?)
    - For each: what contract address, what chain, what API returns the current yield rate?
    - What's the update frequency for each? (Per-block? Daily? Weekly?)
    - Are there existing aggregators? (DeFi Llama, DeBank, Zapper — do they expose yield rate APIs?)
    - What's the minimum viable data pipeline? (Single script that polls N contracts, computes weighted median, writes to JSONL?)

20. **Methodology paper — concrete outline** — Round 4 asked for this but it wasn't delivered in detail. Research:
    - What does SOFR's methodology paper look like? (Section structure, mathematical notation, governance clauses)
    - What does SONIA's methodology paper look like?
    - What does CESR's methodology paper look like? (It's the closest crypto benchmark)
    - What mathematical formalism is standard for rate computation? (Volume-weighted median? Supply-weighted mean? Trimmed mean?)
    - What outlier/staleness handling is standard? (N-sigma filtering? Minimum contributor threshold? Fallback waterfall?)
    - Draft a concrete table of contents with estimated page count per section.

### H. Anti-Patterns and Failure Modes — What Kills This

21. **The death zone (100-1,000 integrations)** — Round 4 identified that protocols that can't cross this in 6 months die. Research:
    - What specific protocols died in the death zone? (AtomPub? RDF? Something more recent?)
    - What tactics got MCP through the death zone? (Was it the OpenAI endorsement alone, or were there other factors?)
    - What's the minimum integration velocity needed? (10/week? 50/week? What's MCP's actual growth curve?)
    - How do you measure integration quality vs. quantity? (GitHub stars vs. actual usage vs. downloads?)

22. **Emergent collusion in agent-attested benchmarks** — Round 3 proved LLMs spontaneously collude. For ISFR-YBS specifically:
    - If agents are attesting yield data, what's the collusion vector? (Agents coordinate to report inflated/deflated yields?)
    - How does SOFR prevent bank collusion? What's the analogue?
    - Is VCG auction applicable to data attestation? (Or does the NL side-channel collusion problem from Round 3 make it useless?)
    - What's the minimum number of independent data sources to make collusion unprofitable?
    - Can you use the PID-synergy metric to detect collusion? (Anomalous redundancy in agent reports?)

23. **The Chinese model substitution risk** — Round 4 showed 80% of US startups use Chinese base models. Research:
    - What's the regulatory risk of depending on DeepSeek V4 for the 20× cost wedge?
    - Has any US government action targeted Chinese LLM usage? (Export controls? Entity list? Data sovereignty?)
    - What's the European position on Chinese model usage?
    - If DeepSeek is blocked, what's the next-cheapest frontier-quality model? (Mistral? Llama? Qwen is also Chinese.)
    - Should the cost wedge be demonstrated with Western models only to de-risk the claim?

---

## Source Documents

Read ALL of these before starting research:

### Synthesis documents (in `synthesis/` folder):

**Round 1 synthesis (ISFR benchmark business):**
- `01-isfr-benchmark-business-strategy.md` — ISFR-YBS wedge, governance, regulatory path, revenue model, partnerships
- `02-agent-benchmark-synergy.md` — 6 compounding mechanisms, research papers, implementation priorities
- `03-architecture-paradigms.md` — Signals/Cells/Graphs, 4 universal patterns, cognitive architecture
- `04-research-paradigms-competitive.md` — Category naming, 5 paradigms, moat ranking
- `05-marketplace-payments-defi.md` — Agent marketplace, x402/MPP, registries, DeFi
- `06-security-observability-deployment.md` — Security model, TEE, telemetry, audit trail

**Round 2 synthesis (agent capabilities frontier):**
- `07-zk-hdc-hardware-codesign.md` — Binius64, Lasso/Jolt, Worldcoin MPC, FPGA prototype path
- `08-compositional-generalization-categorical-foundations.md` — Para(Lens), Poly, DPO, kernel-additivity ceiling, TRM/HRM, GEPA/AFlow
- `09-collective-intelligence-emergent-communication.md` — 64-agent plateau, ρ≈0.23, Agora protocols, sheaf consensus, PID
- `10-adversarial-robustness-safety.md` — Anthropic natural misalignment, Nayebi, memory poisoning, three-pillar anti-collapse
- `11-long-horizon-planning-self-improvement.md` — METR horizons, active inference/EFE, causal discovery, self-improvement
- `12-synergy-map-unique-capabilities.md` — 7 compound capabilities, 10 unique capabilities, competitive window

**Round 3 synthesis (formal limits + production components):**
- `13-self-bootstrapping-metacognition.md` — HGM, Live-SWE-agent, AlphaEvolve RSI closure, SPICE grounding requirement, Introspection Adapters, MASA, defection probes
- `14-emergent-economics-cross-system-composition.md` — Agent economies, emergent collusion, cross-system composition, SRE agents, METR −19%, MAST failures, hard "no" list
- `15-information-theoretic-limits-time-primitives.md` — Aaronson coordination floor, Crutchfield Cμ, Vicsek phase transition, time-series foundation models, metacontroller, allostasis
- `16-mathematical-discovery-measurable-understanding.md` — AlphaProof/Seed-Prover, Stitch/LILO, ARC-AGI-2, PID-synergy, cominterpretant, GWT
- `17-flywheel-map-impossible-results.md` — 5 compounding loops, 8 arena-of-one capabilities, 15 impossible results, competitive position

**Round 4 synthesis (measurement + go-to-market execution):**
- `18-protocol-adoption-developer-experience.md` — MCP/A2A/ERC-8004 adoption curves, failed protocol patterns, DX benchmarks, launch checklist, community strategy
- `19-agent-economics-production-deployment.md` — Production costs ($2-$25/agent-hour), token waste (70%), cost optimization stack (7-20×), self-hosted break-even, multi-agent economics
- `20-regulatory-landscape-10k-agent-scaling.md` — EU AI Act Aug 2, Colorado Jun 30, ISO 42001, NIST agent identity, MiCA, liability, 10K-agent studies, herding, memory contamination
- `21-measurement-frameworks-90day-roadmap.md` — 4 claims to prove, 5 KPIs, "7 lines of code" demos, 90-day phased roadmap, dark horses, risk register

### Research documents (in `research/` folder):
- `reserach1.md` — Round 1 raw research
- `reserach2.md` — Round 2 raw research
- `research3.md` — Round 3 raw research
- `reaserch4.md` — Round 4 raw research

---

## Output format

Structure your research as:

1. **Executive Summary** (1 page) — Top 5 findings that change what gets built THIS WEEK
2. **Section A: Protocol Spec Deep-Dive** — Message format, transport, identity, safety primitives — with actual JSON schemas and field-level comparisons to MCP/A2A/Symphony
3. **Section B: Reference Implementation Architecture** — Temporal integration decision, HDC library evaluation, LLM abstraction layer, stigmergic storage — with Rust crate recommendations and code-level architecture
4. **Section C: Demo & Sandbox** — The actual API surface for the 7-line demo, hosted sandbox infrastructure, zero-install trial architecture
5. **Section D: Launch Partner Playbook** — Who, how, what's the ask, what's the timeline — with named contacts/organizations where findable
6. **Section E: Security Primitives** — Credential format, destructive-grant classification, memory poisoning defense — spec-level detail, not hand-waving
7. **Section F: Academic Publication Path** — Deadlines, minimum viable papers, benchmark suite design
8. **Section G: ISFR-YBS Parallel Track** — Data pipeline spec, methodology paper outline, collusion defense
9. **Section H: Failure Modes & De-Risking** — Death zone tactics, Chinese model dependency, competitive response playbook

For each finding, flag:
- **Build now** — can start coding Monday morning with no further research
- **Research first** — needs 1-2 days of investigation before building
- **Blocked on external** — depends on something outside our control (name the dependency)
- **Abandon** — not worth pursuing (explain why)

**Critical constraint: every recommendation must include a CONCRETE ARTIFACT** — a file to create, a function to write, a contract to deploy, a paper to draft, a person to email. "Consider doing X" is not acceptable. "Create `src/protocol/message.rs` with fields [A, B, C] and implement `From<McpMessage>` trait" is acceptable.

**The output should be a build spec, not a strategy document.** If an engineer reads Section B and can't start writing Rust code, it's too vague.
