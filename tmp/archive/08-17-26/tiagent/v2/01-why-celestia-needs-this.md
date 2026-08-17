# Why Celestia Needs tiagent

## What this document is

This is a strategic case for the Celestia Foundation to fund tiagent, a
general-purpose coding agent that uses Celestia's data availability layer as
its shared learning substrate. tiagent competes with Claude Code, Codex, and
Cursor --- but every user becomes a Celestia DA consumer.

---

## 1. The AI Narrative Gap

AI + crypto is a $22.6--27 billion market as of mid-2026 and growing
exponentially. Every major blockchain ecosystem has staked a position. Celestia
has not.

The numbers are stark:

| Ecosystem | AI-specific investment | AI-specific grants | AI branding |
|---|---|---|---|
| **0G Labs** | $88.88M AI ecosystem fund + $20M Apollo AI Accelerator | Dedicated AI grants up to $1M+ | Explicit: "The AI Blockchain" |
| **Filecoin** | $3.68M ProPGF Batch 1 (includes AI/ML) | AI-aware grant categories | Moderate |
| **Solana** | ARC, Solana Agent Kit ecosystem | Dedicated hackathon tracks | Growing |
| **Base** | Coinbase AgentKit investment | AI agent builder programs | Active |
| **Polkadot** | polkagent (90-crate framework) | Ecosystem grants | Active |
| **Celestia** | **$0** | **None** | **None** |

0G Labs has been especially aggressive. With $290M in total funding, they have
deployed $108M specifically toward AI positioning:

- **$88.88M AI Ecosystem Fund** --- funding any project that builds on 0G's
  DA + execution stack
- **$20M Apollo AI Accelerator** --- with CoinFund and Hack VC, targeting AI +
  blockchain startups specifically
- **Dedicated AI Labs** --- building reference implementations
- **Explicit market positioning**: "0G is purpose-built for AI"

The result: when a developer or investor thinks "AI + data availability," they
think 0G. Not Celestia.

This is not a technology problem. Celestia has objectively superior
infrastructure:

| Capability | Celestia | 0G Labs |
|---|---|---|
| Block size | 128 MB (post-Matcha), roadmap to 1 Tb/s | 50 MB max (early testnet) |
| Network maturity | Production mainnet, 56+ rollups | Early-stage testnet |
| Economic activity | Real transaction volume, established ecosystem | Pre-launch, pre-revenue |
| Light node verification | DAS with production lumina-node | Not yet available |
| Data partitioning | 29-byte namespace system, production-ready | Custom sharding (unstable) |
| Developer community | Established, active, building in production | Nascent |

Celestia has better block sizes, a production mainnet, real rollup adoption,
working DAS infrastructure, and a proven namespace system. 0G has none of
these. But 0G owns the narrative because they spent $108M claiming it.

EigenDA and Avail are not pursuing AI positioning either, which means the
competitive field is narrow. Right now it is 0G vs. nobody. That is a problem
Celestia can solve with a single well-placed investment.

The risk is concrete: **Celestia cedes the "AI infrastructure" narrative to 0G
by default, not by technical merit.** The longer this goes unaddressed, the
harder it becomes to reclaim.

---

## 2. How tiagent Closes the Gap

tiagent is a general-purpose coding agent written in Rust. It competes
directly with Claude Code, Codex, Cursor, and Windsurf on capability. It
writes code, runs tests, debugs, refactors, and deploys using any LLM backend
(Claude, GPT, Gemini, Llama, Mistral, Ollama, or any OpenAI-compatible API).

What makes it novel: tiagent uses Celestia's DA layer as a shared learning
substrate.

This is not "build a tool for Celestia developers." It is **"build the best
coding agent for all developers, powered by Celestia."**

Here is what that means concretely:

**The standalone product**: tiagent works as a fully local coding agent with no
blockchain dependencies. A developer installs it, points it at their codebase,
and uses it exactly like Claude Code or Cursor. It handles single prompts,
multi-step plan execution, PRD-to-code workflows, parallel agent dispatch, and
automated quality gates (compile, test, lint, diff review). It learns locally
--- which models work best for which tasks, which strategies to reuse, which
thresholds to tune.

**The Celestia layer**: With DA integration enabled, tiagent publishes learning
artifacts to Celestia as namespace-organized blobs:

- **Traces namespace**: Structured records of agent actions, tool calls,
  outcomes, and error patterns
- **Routing namespace**: Cascade router weights --- which model performed best
  for which task type
- **Vectors namespace**: Embeddings of successful trajectories, enabling
  trajectory-RAG (retrieval-augmented generation from other agents' experience)
- **Fingerprint namespace**: HDC (Hyperdimensional Computing) behavioral
  signatures for similarity matching and Sybil resistance

Other tiagent instances retrieve relevant artifacts from these namespaces. A
new agent bootstraps from the network's collective routing experience instead
of starting cold. An agent stuck on a bug can find "how did another agent solve
a similar problem?" and use that trajectory as in-context learning.

The DA layer's properties --- append-only, namespace-partitioned, verifiable,
permissionless --- map directly to the requirements of shared agent learning.
The learning corpus grows monotonically, is tamper-evident, and scales
naturally through namespace partitioning.

**Knowledge demurrage and Celestia's pruning window.** Celestia's 7-day blob
pruning is often cited as a limitation for AI use cases. tiagent turns it into
an advantage. tiagent's knowledge model is built on *demurrage*: information
that is not actively validated by successful agent outcomes decays and dies
within days. Unreinforced routing weights, stale trajectories, and unconfirmed
embeddings are designed to expire --- they represent noise, not signal. This
creates natural alignment with Celestia's architecture. tiagent does not fight
the pruning window; it embraces it. Fresh, validated knowledge is continuously
republished by the agents that use it. Stale knowledge is pruned by both
tiagent and Celestia simultaneously. The result is a self-cleaning shared
learning layer where only battle-tested intelligence survives --- exactly the
property you want from a collective knowledge substrate.

**Every developer who uses tiagent becomes a Celestia DA consumer.** A Python
developer debugging a Django app. A TypeScript developer building a React
frontend. A Go developer writing infrastructure. None of them need to know or
care about blockchain. They use tiagent because it is a better coding agent.
Celestia powers the learning layer invisibly.

The AI narrative writes itself: **"Celestia powers the world's first
collectively intelligent coding agent."**

---

## 3. Marketing and Narrative Value

A working tiagent deployment generates ongoing narrative material for Celestia
without additional marketing spend.

**Conference talks and developer content:**

- "How Celestia's DA Layer Made Our Coding Agent 40% Better" --- a talk at any
  AI or developer conference, reaching audiences who have never heard of
  Celestia
- "Why We Chose Celestia Over 0G for Shared Agent Learning" --- direct
  counter-narrative with technical substance
- "Building Collective Intelligence on a Data Availability Layer" --- academic
  and research crossover appeal

**Blog posts and case studies:**

- Performance comparisons: solo agent vs. collectively-learning agent, with
  measurable improvement curves
- Cost analysis: DA fees vs. centralized database hosting for shared agent
  state
- Architecture deep-dives that position Celestia as infrastructure for novel
  use cases beyond rollups

**Developer advocacy:**

- tiagent tutorials bring non-crypto developers into the Celestia ecosystem
  organically --- they come for the coding agent, they stay for the DA layer
- Every tiagent contributor becomes a potential Celestia developer
- Open-source community building around tiagent extends Celestia's developer
  relations reach

**The media angle**: "The blockchain you use without knowing it." tiagent users
interact with Celestia invisibly. This is a compelling story for mainstream
tech press --- blockchain infrastructure that developers use without friction,
without wallets, without gas management, without any of the barriers that
typically prevent adoption.

**Counter-narrative to 0G**: Celestia does not need to build "an AI chain" or
launch an $88M fund. It already IS AI infrastructure. It just needs one project
that demonstrates it. tiagent is that project. The response to 0G's marketing
budget is a working product, not a competing fund.

---

## 4. Attracting Non-Blockchain Developers

The traditional blockchain developer funnel has a well-known problem:

```
Traditional funnel:

    Learn blockchain concepts (weeks)
         |
         v
    Choose a chain (research)
         |
         v
    Learn chain-specific tooling (weeks)
         |
         v
    Build a dApp (months)

    Conversion rate: extremely low
    Most developers drop off at step 1
```

tiagent inverts this funnel:

```
tiagent funnel:

    Install a coding agent (minutes)
         |
         v
    Use it on your existing codebase (immediate value)
         |
         v
    Notice your agent getting smarter from collective learning
         |
         v
    Discover the DA layer powering it
         |
         v
    Explore Celestia (organic curiosity)
         |
         v
    Build on Celestia (optional, self-selected)

    Conversion rate: dramatically higher
    Value delivered at step 1, not step 4
```

The numbers behind this matter:

- There are approximately **30 million** software developers worldwide
- There are approximately **300,000** blockchain developers
- Every existing blockchain developer tool targets the 300K
- tiagent targets the 30M

The market for coding agents is enormous and growing. Millions of developers
already use Claude Code, Codex, and Cursor daily. These are not niche tools
--- they are mainstream development infrastructure. tiagent competes in this
market, with Celestia as the invisible competitive advantage.

Every developer who converts from Claude Code to tiagent is a net-new Celestia
ecosystem participant. They were not going to use Celestia otherwise. They are
not blockchain developers. They are React developers, Python data scientists,
Go infrastructure engineers --- the long tail of software development that
blockchain ecosystems have struggled to reach.

This is a growth vector that no other Celestia project provides. Rollup teams
bring rollup developers. DeFi protocols bring DeFi users. tiagent brings
**everyone else.**

---

## 5. Network Effects and Exponential Returns

Most developer tools have linear value: one user generates one unit of value.
Two users generate two units. The relationship is additive.

tiagent has a genuine network effect. Each new user makes the product better
for all existing users. This is because the shared learning layer on Celestia
is a common pool that everyone contributes to and draws from:

- **Agent A** solves a tricky Django migration. The successful trajectory is
  published to Celestia.
- **Agent B**, encountering a similar Django migration next week, retrieves
  Agent A's trajectory via embedding similarity and uses it as in-context
  learning. Agent B solves the problem faster.
- **Agent B's** improved trajectory is also published. Now the collective pool
  has two Django migration strategies.
- **Agent C** benefits from both.

This is superlinear:

| Scale | What happens | DA impact |
|---|---|---|
| **100 users** | Modest shared learning. Routing data starts to converge on optimal model selection for common tasks. | Small but measurable blob volume |
| **1,000 users** | Meaningful collective intelligence. Trajectory-RAG returns high-quality matches for most common development tasks. | Consistent daily DA consumption |
| **10,000 users** | Comprehensive knowledge base. The collective learning pool covers most programming languages, frameworks, and common tasks. | DA consumption comparable to a mid-size rollup |
| **100,000 users** | Transformative. The learning corpus is so rich that tiagent consistently outperforms single-instance commercial agents. | DA consumption comparable to major rollups |
| **1,000,000 users** | The product becomes nearly impossible to compete with. No single vendor can match a million-developer learning corpus. | DA consumption rivaling the largest rollups |

The network effect creates a flywheel:

```
More users --> more learning data on Celestia
                  |
                  v
           Better agent performance
                  |
                  v
           More users attracted by performance
                  |
                  v
           More learning data on Celestia ...
```

Unlike most crypto network effects (which rely on financial incentives),
tiagent's network effect is product-driven. Users stay because the agent is
genuinely better, not because of token rewards. This is more durable.

All of this DA usage generates blob fees for Celestia validators, directly
supporting TIA economics. The more successful tiagent becomes as a product,
the more DA demand it generates.

---

## 6. Revenue and DA Consumption Model

Every active tiagent instance generates DA consumption. Here are conservative
estimates based on typical agent workloads:

**Per-agent data production:**

| Data type | Size per task | Tasks/day (active agent) | Daily DA per agent |
|---|---|---|---|
| Episode trace | 5--50 KB | 10--100 | 50 KB -- 5 MB |
| Embedding vectors | 2--10 KB | 10--100 | 20 KB -- 1 MB |
| HDC fingerprint | 1--2 KB | 1 (per session) | 1--2 KB |
| Routing delta | 0.5--2 KB | 1 (per session) | 0.5--2 KB |
| **Total** | | | **~71 KB -- 6 MB/day** |

At current Celestia blob costs (~$0.07--$0.81/MB), per-agent daily DA cost
ranges from approximately **$0.005 to $4.86/day**. Using a mid-range estimate
of $0.37--$4.30/day per active agent:

| Active agents | Daily DA consumption | Daily DA fees (est.) | Annual DA fees (est.) |
|---|---|---|---|
| **100** | 7 MB -- 600 MB | $37 -- $430 | $13.5K -- $157K |
| **1,000** | 70 MB -- 6 GB | $370 -- $4,300 | $135K -- $1.57M |
| **10,000** | 700 MB -- 60 GB | $3,700 -- $43,000 | $1.35M -- $15.7M |
| **100,000** | 7 GB -- 600 GB | $37,000 -- $430,000 | $13.5M -- $157M |

For context: Eclipse is currently one of the largest DA consumers on Celestia.
At 10,000 active tiagent agents, the DA consumption would be comparable to or
exceed Eclipse's throughput. At 100,000 agents, tiagent would be one of the
largest sources of DA demand on the network.

These are not hypothetical numbers. The coding agent market already has millions
of daily active users across Claude Code, Codex, and Cursor. Capturing even a
small fraction of that market generates meaningful DA volume.

The economics are also favorable for users. $0.37--$4.30/day is trivial
compared to LLM API costs (a heavy Claude Code user spends $20--100+/day on
API calls). The DA cost is a rounding error in the total cost of running a
coding agent, but it aggregates into significant network-level demand.

---

## 7. What $200K Buys

The requested grant is USD $200,000 over 12 months, structured as six
milestones with concrete, verifiable deliverables.

**Milestone 1 (Months 1--2): Core harness and local agent** --- $30K

- Working standalone coding agent with universal execution loop
- Model-agnostic LLM dispatch (Claude, GPT, Gemini, Llama, Ollama)
- Local learning: cascade routing, efficiency tracking, adaptive gates
- CLI interface comparable to Claude Code for single-task execution
- Deliverable: a developer can install tiagent and use it as a local coding
  agent

**Milestone 2 (Months 3--4): Plan execution and quality gates** --- $35K

- Multi-step plan DAG executor with parallel dispatch
- 7-rung gate pipeline: compile, test, lint, diff review, and beyond
- PRD-to-plan generation and autonomous execution
- State persistence and resume-after-interruption
- Deliverable: tiagent can execute complex, multi-task development workflows

**Milestone 3 (Months 5--6): Celestia DA integration** --- $40K

- Blob submission and retrieval through organized namespaces
- Episode traces published to DA
- Routing weights shared across agents via DA
- Light node integration for DA verification
- Deliverable: learning artifacts are published to Celestia Mocha testnet

**Milestone 4 (Months 7--8): Shared learning and trajectory-RAG** --- $40K

- Cross-agent trajectory retrieval via embedding similarity
- Collective routing bootstrap (new agents start from network experience)
- HDC behavioral fingerprinting for similarity and Sybil resistance
- Deliverable: measurable performance improvement from shared vs. solo learning

**Milestone 5 (Months 9--10): Protocol integration and developer tools** --- $30K

- MCP (Model Context Protocol) client and server implementation
- Celestia-specific developer tools: namespace explorer, blob inspector,
  rollup scaffolding
- A2A (Agent-to-Agent) protocol support for multi-agent coordination
- Deliverable: tiagent interoperates with the broader agent ecosystem

**Milestone 6 (Months 11--12): Production hardening and launch** --- $25K

- Mainnet deployment (Celestia mainnet DA integration)
- Performance optimization and stress testing
- Documentation, tutorials, and developer onboarding
- Open-source community launch
- Deliverable: production-ready tiagent with mainnet Celestia DA

**What this buys Celestia beyond the deliverables:**

- An open-source codebase (MIT/Apache-2.0) that compounds value indefinitely
- The Celestia Foundation's **first AI-specific investment** --- a signal to
  the market that Celestia is serious about AI infrastructure
- A working counter-narrative to 0G's $108M marketing spend
- A new DA consumer category that scales with the coding agent market
- A developer funnel that reaches 30M software developers, not 300K
  blockchain developers

---

## 8. The Cost of Inaction

The window for Celestia to claim AI infrastructure positioning is open now.
It will not stay open indefinitely.

**If Celestia does nothing:**

- 0G continues to capture AI mindshare unopposed. Their $108M in AI funding
  attracts builders, and the narrative compounds. "AI + DA" becomes synonymous
  with 0G in developer and investor minds.

- Celestia remains perceived as "just for rollups." The modular DA thesis is
  powerful, but it limits the addressable market to rollup teams --- a
  relatively small developer population.

- Non-blockchain developers have no entry point to Celestia. The 30M software
  developers who could become DA consumers remain untouched. Every day without
  a product like tiagent is a day those developers choose Claude Code (value
  captured by Anthropic), Codex (value captured by OpenAI), or Cursor (value
  captured by Anysphere).

- When AI agents do arrive on Celestia --- and they will, because the
  infrastructure is suited for it --- the narrative credit goes to 0G.
  "Celestia finally caught up to 0G's vision" is not the headline Celestia
  wants.

**If Celestia funds tiagent:**

- For $200K --- less than 0.2% of what 0G spent on AI positioning --- Celestia
  gets a working product, a counter-narrative, a new DA consumer category, and
  a developer funnel that reaches non-blockchain developers.

- The first AI-specific grant signals to the ecosystem that Celestia is open
  for AI infrastructure. Other builders follow. The narrative shifts from "0G
  is the AI DA layer" to "Celestia already has production AI infrastructure."

- tiagent's network effect means early investment compounds. The first mover
  in shared-learning-via-DA captures the largest learning corpus and becomes
  progressively harder to displace.

The comparison is simple:

| | 0G's approach | Celestia + tiagent |
|---|---|---|
| Cost | $108M+ | $200K |
| Infrastructure | Early testnet | Production mainnet |
| Product | Marketing fund | Working coding agent |
| Developer reach | Blockchain developers | All software developers |
| Network maturity | Pre-launch | 56+ rollups, real volume |

Celestia does not need to outspend 0G. It needs to out-build them. $200K and
one focused project is enough to change the narrative --- because the
narrative should have been Celestia's all along. The infrastructure is already
there. It just needs a product that demonstrates it.

---

## Summary

tiagent is not a blockchain tool that happens to involve AI. It is an AI tool
that happens to use Celestia. That distinction is the entire strategy.

Millions of developers use coding agents daily. tiagent gives them a better
one --- open source, model-agnostic, self-improving, collectively intelligent.
Celestia's DA layer is what makes the collective intelligence possible: shared,
verifiable, tamper-evident, and not captured by any single vendor.

Every tiagent user becomes a Celestia DA consumer. Every trace published,
every routing weight shared, every trajectory stored is a Celestia blob
generating fees and growing the network. The product scales with the coding
agent market, not the blockchain developer market.

For $200K, Celestia gets its first AI-specific investment, a working
counter-narrative to 0G's $108M spend, a new DA consumer category, and a
developer funnel that reaches 30 million software developers instead of 300
thousand blockchain developers.

The infrastructure is already the best. The product to prove it is what is
missing.
