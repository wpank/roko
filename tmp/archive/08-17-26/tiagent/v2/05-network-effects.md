# Network Effects, Growth, and Scale

**Document 5 of the tiagent Celestia Grant Proposal**

**Date**: August 2026

---

## Executive Summary

Most coding agents deliver linear value: one user gets one unit of value, a
thousand users get a thousand units. tiagent is structurally different. Each
new user makes every existing user's agent smarter, because learning
artifacts---routing weights, playbooks, efficiency patterns---flow through
Celestia DA to every agent in the network. This is the same dynamic that
made Facebook, Uber, and Waze into category-defining platforms. The
difference is that tiagent's network runs on public, verifiable
infrastructure that Celestia provides.

This document makes the economic and strategic case for why tiagent creates
exponential returns, durable competitive advantage, and meaningful DA
demand for the Celestia ecosystem---and why $200K in grant funding is the
highest-leverage ecosystem investment available today.

---

## 1. The Network Effect Thesis

### Why most coding agents have a scaling problem

Every major coding agent on the market today---Claude Code, Codex, Cursor,
Aider, Windsurf---operates in isolation. Each user's agent starts from
zero, learns nothing from its own usage, and learns nothing from anyone
else's usage. Your thousandth coding session is exactly as capable as your
first. The tool has no memory.

This means the value curve is linear:

```
Value
  ^
  |                              Linear (Claude Code, Cursor, etc.)
  |                            /
  |                          /
  |                        /
  |                      /
  |                    /
  |                  /
  |                /
  |              /
  |            /
  |          /
  |        /
  |      /
  |    /
  |  /
  |/
  +---------------------------------------------> Users
```

One user = one unit of value. A thousand users = a thousand units. There
is no interaction between users, no shared intelligence, no compounding.
Each installation is an island.

### Why tiagent has superlinear value

tiagent's self-improvement loops (described in Document 02) produce
structured learning artifacts: which models work best for which task
types, which prompt strategies succeed, which code patterns cause gate
failures, and what playbooks solve recurring problems. These artifacts
are small (typically 1-50 KB each), structured (JSON/CBOR), and highly
compressible.

When published to Celestia DA, these artifacts become available to every
tiagent instance in the network. The result is a fundamentally different
value curve:

```
Value
  ^
  |                                    Superlinear (tiagent)
  |                                  /
  |                                /
  |                              /
  |                           /
  |                        /       Linear (baseline)
  |                      /       /
  |                   /        /
  |                /         /
  |             /          /
  |          /           /
  |        /           /
  |      /           /
  |    /           /
  |  /           /
  |/           /
  +---------------------------------------------> Users
```

Each new user contributes learning data. That data improves the
collective intelligence. The improved intelligence makes tiagent more
effective for all users. More effective agents attract more users.

This is not theoretical. The mechanics are concrete:

- **Agent A** discovers that Claude Haiku handles unit test generation
  95% as well as Sonnet at 1/10th the cost. It publishes this routing
  weight to Celestia DA.
- **Agent B**, encountering a unit test task for the first time, retrieves
  Agent A's routing data and routes to Haiku immediately---skipping the
  exploration phase entirely.
- **Agent C** discovers a playbook for handling Rust lifetime errors in
  async code. It publishes the playbook. Now every tiagent instance can
  solve that class of problem on first encounter.
- **Agent D** finds that a particular system prompt structure reduces
  token usage by 35% for refactoring tasks. That efficiency gain
  propagates to the entire network.

The more agents participate, the more complete the collective knowledge
becomes, and the faster new agents reach peak performance. This is the
same dynamic that made Waze valuable: each driver contributes traffic
data, all drivers benefit from collective routing intelligence. But
Waze's data sits on proprietary servers controlled by Google. tiagent's
learning data sits on Celestia---public, verifiable, censorship-
resistant infrastructure.

**Celestia is the backbone of this network effect.** It is the shared
memory layer that makes collective intelligence possible. Without a
public DA layer, tiagent would still be a good standalone agent (the
inner and middle loops work locally). With Celestia, it becomes a
platform with compounding returns.

---

## 2. The Growth Flywheel

The flywheel has six stages. Each stage feeds the next, creating a self-
reinforcing cycle that accelerates with scale.

```
    1. Developer adopts tiagent
       (better than Claude Code: open-source, self-improving, model-agnostic)
                                    |
                                    v
    2. Agent publishes learning artifacts to Celestia DA
       (routing weights, playbooks, efficiency data: ~$0.05-0.50/day per agent)
                                    |
                                    v
    3. More data --> collective intelligence improves
       (playbook coverage expands, routing accuracy increases)
                                    |
                                    v
    4. Better agent --> word of mouth --> more developers adopt
       ("my coding agent gets smarter when other people use it too")
                                    |
                                    v
    5. More developers --> more learning data --> exponentially better agent
       (coverage approaches completeness across languages and patterns)
                                    |
                                    v
    6. Celestia DA usage grows linearly with user count
       (blob fees accrue to validators, ecosystem value increases)
                                    |
                                    v
                          Back to step 1.
```

### The math behind the flywheel

The key metric is **playbook coverage**: what percentage of coding tasks
a new user encounters are already covered by playbooks in the network.

Each agent encounters roughly 5-15 distinct task patterns per day. Some
of these are novel (no existing playbook), and some match existing
patterns. As the network grows, the ratio shifts dramatically:

| Network Size | Estimated Distinct Patterns | Coverage for New User | Time to Peak Performance |
|---|---|---|---|
| 1 agent (standalone) | 50-100 | 0% | 3+ months of solo use |
| 100 agents | 2,000-5,000 | ~40% | 4-6 weeks |
| 1,000 agents | 10,000-30,000 | ~75% | 1-2 weeks |
| 10,000 agents | 50,000-100,000 | ~92% | 1-3 days |
| 100,000 agents | 200,000-500,000 | ~98% | Hours |

At 100,000 agents, a brand-new tiagent installation is immediately
effective at 98% of common coding tasks because the network has already
seen and solved nearly every pattern. Compare this to any existing coding
agent, where every user starts at 0% and stays there.

The implication for Celestia: **each user who joins the network increases
the value proposition for every future user**. This is the definition of
a demand-side network effect, and it is the most durable form of
competitive advantage in technology.

### Viral coefficient

Network effects create organic growth. The viral coefficient (K-factor)
for tiagent is driven by a simple, shareable value proposition:

> "Your coding agent gets smarter when other people use it too."

This is the kind of sentence that spreads on Hacker News, Reddit, and
Twitter. It is counterintuitive (most tools are isolated), concrete
(measurable improvement), and verifiable (the learning data is on-chain).
The closest analogy in developer tooling is GitHub: a platform that gets
more valuable as more people join, with strong word-of-mouth growth.

Conservative estimate: K-factor of 1.2-1.5 at the growth stage,
meaning each cohort of users brings in 20-50% more users through
organic referral.

---

## 3. Why This Is Novel

### No existing coding agent has this property

This is worth stating plainly: **no coding agent available today shares
learning across users.** The competitive landscape:

| Agent | Local Learning | Cross-User Learning | Network Effects |
|---|---|---|---|
| Claude Code | None | None | None |
| OpenAI Codex | None | None | None |
| Cursor | Minimal (project context) | None | None |
| Aider | None | None | None |
| Windsurf | Minimal (memory) | None | None |
| GitHub Copilot | None | Implicit (training data) | None |
| **tiagent** | **Full (3 feedback loops)** | **Full (Celestia DA)** | **Yes** |

GitHub Copilot comes closest: Microsoft trains the model on aggregate
usage data, so in a loose sense, all Copilot users contribute to future
model quality. But this is implicit (users have no visibility or control),
slow (model retraining cycles are months), opaque (you cannot inspect what
was learned), and centralized (Microsoft controls it entirely).

tiagent's cross-agent learning is explicit (structured artifacts with
clear provenance), fast (propagation within minutes via DA), transparent
(anyone can inspect the learning data on-chain), and decentralized (no
single entity controls the knowledge commons).

### The Waze analogy

The most precise analogy is Waze, the traffic navigation app:

- Each Waze user passively contributes real-time traffic data.
- The collective data improves routing for all users.
- More users = more data = better routes = more users.
- Waze became so valuable through this dynamic that Google acquired it
  for $1.1 billion in 2013, when it had approximately 50 million users.

tiagent follows the same pattern, but with three critical differences:

1. **Open infrastructure.** Waze's data sits on Google's proprietary
   servers. tiagent's learning data sits on Celestia---public,
   verifiable, and owned by no one.

2. **Larger addressable market.** Waze serves drivers (approximately
   150 million MAU today). tiagent serves software developers (approximately
   30 million worldwide, growing rapidly with AI-assisted coding).

3. **Higher value per user.** A Waze user saves minutes per day. A
   tiagent user saves hours per day and reduces LLM costs by 30-50%.
   The economic value per user is significantly higher.

### Net-new for both ecosystems

This network-effect property is new for the AI agent space (no existing
agent has it) AND new for the Celestia ecosystem (no existing Celestia
application demonstrates demand-side network effects at the application
layer). tiagent creates a new category in both.

---

## 4. Scaling Economics

The economic model scales favorably because learning artifacts are small
and compressible, while the value they create is large and compounding.

### Per-agent DA costs

A single tiagent instance produces roughly 50-200 KB of publishable
learning data per day:

| Artifact Type | Size (compressed) | Frequency | Daily Volume |
|---|---|---|---|
| Episode traces | 2-10 KB each | 5-15 per day | 10-150 KB |
| Routing weight updates | 1-5 KB | 1-3 per day | 1-15 KB |
| Playbook publications | 5-20 KB each | 0-2 per day | 0-40 KB |
| Efficiency deltas | 0.5-2 KB | 5-15 per day | 2.5-30 KB |
| HDC fingerprints | 1-4 KB each | 2-5 per day | 2-20 KB |
| **Total** | | | **~15-255 KB/day** |

At current Celestia DA pricing (approximately $2.10 per MB after Matcha),
this translates to roughly $0.03-0.54 per agent per day, with a median
around $0.21.

### Aggregate DA demand at scale

| Stage | Agents | Daily DA Volume | Monthly DA Cost | Annual DA Cost | Collective Intelligence |
|---|---|---|---|---|---|
| Seed | 100 | ~10 MB | ~$630 | ~$7,600 | Low---limited cross-pollination |
| Growth | 1,000 | ~100 MB | ~$6,300 | ~$76,000 | Moderate---routing improvements visible |
| Traction | 10,000 | ~1 GB | ~$63,000 | ~$760,000 | High---playbooks cover most task types |
| Scale | 100,000 | ~10 GB | ~$630,000 | ~$7.6M | Very High---approaching full coverage |
| Mass | 1,000,000 | ~100 GB | ~$6.3M | ~$76M | Transformative---collective superintelligence for coding |

For context on what these numbers mean for Celestia:

- At the **Traction** stage (10,000 agents), tiagent generates ~1 GB/day
  of DA demand---comparable to a mid-sized rollup.
- At the **Scale** stage (100,000 agents), tiagent generates ~10 GB/day---
  comparable to multiple top-tier rollups combined.
- At the **Mass** stage (1,000,000 agents), tiagent would be one of the
  largest DA consumers in the Celestia ecosystem, generating $76M in
  annual blob fees.

These numbers are conservative. They assume only learning artifact
publication, not the broader range of DA use cases that tiagent enables
(agent-to-agent coordination, verifiable tool call logs, audit trails).

### Knowledge demurrage: why DA demand is recurring, not one-time

A static knowledge base would plateau: agents write once, read forever,
and DA demand flattens. tiagent's knowledge model prevents this. All
learning artifacts carry a decay schedule (demurrage). Routing weights
lose confidence over time. Playbooks that are never retrieved and
validated lose tier status. This means agents must continuously publish
fresh learning artifacts to maintain the network's collective
intelligence---creating recurring, organic DA consumption that grows
with the network rather than tapering off.

Three mechanisms amplify this effect:

1. **Reinforcement-driven consumption.** When Agent A publishes a
   playbook that Agent B retrieves and successfully uses (passing a
   gate), Agent B's reinforcement signal validates the playbook. That
   validation is itself a publishable artifact. The more agents
   participate, the more reinforcement signals flow, the more DA blobs
   are written. Each successful use generates a new write.

2. **Dream consolidation cycles.** Each agent runs periodic dream
   cycles that read recent DA blobs, cluster related artifacts, and
   publish distilled knowledge back to the network. This creates a
   continuous feedback loop of DA reads and writes---even when no
   human-initiated coding sessions are running.

3. **Genomic bottleneck bootstrapping.** New agents do not need to scan
   the full DA history. They bootstrap from the latest "genomic
   bottleneck" snapshot---a compressed top-N knowledge blob that
   captures the network's best current intelligence. This reduces
   onboarding cost while the periodic republication of these snapshots
   adds another layer of steady DA demand.

The net result is natural selection pressure on the network's knowledge.
Artifacts that are widely retrieved, used, and reinforced get
republished and survive. Artifacts that nobody uses decay locally
(demurrage) and expire on Celestia (blob pruning). The network
self-selects for high-quality learning data without any central curation,
and the selection process itself drives DA consumption.

### Cost per unit of value

The critical economic question is: does the DA cost justify the value
created?

| Stage | DA Cost/Agent/Month | Value Created/Agent/Month | ROI |
|---|---|---|---|
| Seed | ~$6.30 | ~$50-100 (time savings) | 8-16x |
| Growth | ~$6.30 | ~$100-200 (time + cost savings) | 16-32x |
| Traction | ~$6.30 | ~$200-500 (compounding intelligence) | 32-79x |
| Scale | ~$6.30 | ~$500-1,000 (near-complete coverage) | 79-159x |

The per-agent DA cost stays roughly constant (each agent produces similar
volumes), but the value created per agent increases with network size
because collective intelligence improves. This means the ROI improves
at every stage of growth.

---

## 5. Defensibility and Moats

Network effects are the strongest form of competitive moat in technology.
tiagent builds four reinforcing moats:

### Data moat

Every day that tiagent agents operate, they collectively produce learning
data that accumulates on Celestia DA. A competitor launching today would
face a cold-start problem: their agents have no shared intelligence, while
tiagent's network has months or years of accumulated knowledge.

This moat deepens daily. At 10,000 agents producing 100 KB/day each,
the network accumulates roughly 1 GB of new learning data per day. After
one year, that is 365 GB of structured, quality-gated agent intelligence
that no competitor can replicate without building an equivalent user base.

The data moat is particularly strong because the learning data has
diminishing marginal cost but increasing marginal value. The 100,000th
playbook might be cheap to produce, but it fills a gap that makes the
entire corpus meaningfully more complete.

### Protocol moat (first-mover advantage)

tiagent defines the namespace structure, blob encoding format, and
learning artifact schema for agent intelligence on Celestia DA. These
choices become de facto standards as the network grows.

Future agent frameworks that want to participate in the same learning
network must adopt tiagent's schema---or build a competing, incompatible
network from scratch. History shows that protocol-level first movers in
open ecosystems (TCP/IP, HTTP, ERC-20) tend to define the standard that
everyone else adopts.

### Ecosystem moat

tiagent does not exist in isolation. It integrates with:

- **TraceCommons**: Cross-framework learning (tiagent traces accessible
  to Claude Code, Codex, and other agents, and vice versa)
- **MCP**: Standard tool protocol with 97M+ monthly SDK downloads
- **IronClaw**: WASM/TEE sandboxed execution for untrusted agent code

Each integration creates switching costs. A developer using tiagent with
TraceCommons and MCP tools has invested configuration, learned workflows,
and built automation that does not transfer to a competing framework.

### Open-source moat

Paradoxically, being open-source strengthens the moat rather than
weakening it. Here is why:

1. **Community contributions compound.** External contributors add
   language support, tool integrations, and platform compatibility that
   the core team could not build alone. Every contribution makes tiagent
   harder to compete with.

2. **Trust.** Developers trust open-source tools more than proprietary
   ones, especially for AI agents that have access to their codebases.
   This trust advantage accelerates adoption relative to closed-source
   competitors.

3. **Forking is not a threat.** A fork can copy the code, but it cannot
   copy the network. The learning data on Celestia DA belongs to the
   namespace, not the codebase. A fork would start with zero collective
   intelligence.

---

## 6. Marketing Multiplier Effect

tiagent is not just a product---it is a marketing vehicle for Celestia.
Every tiagent success story is inherently a Celestia success story,
because the network effect that makes tiagent special runs on Celestia DA.

### Organic storytelling

The stories that tiagent generates are naturally compelling:

- "Developer builds production app 3x faster using tiagent---powered by
  Celestia DA" (headline for a case study)
- "Open-source coding agent gets smarter every time someone new installs
  it---learning data shared via Celestia" (Hacker News submission)
- "My AI coding agent solved a bug in 30 seconds because another
  developer's agent encountered the same issue last week on a different
  continent" (Twitter thread)

These are not manufactured marketing narratives. They are natural outcomes
of the product working as designed. And each one positions Celestia as
critical infrastructure for AI agents.

### Conference and community presence

AI agent development is the hottest topic in the developer conference
circuit. Talks about coding agents draw standing-room-only audiences at
every major conference. tiagent provides Celestia with a presence at:

- **AI/ML conferences**: NeurIPS, ICML, AI Engineer Summit
- **Developer conferences**: Strange Loop, RustConf, PyCon, JSConf
- **Blockchain conferences**: Modular Summit, EthCC, Cosmoverse
- **Meetups and workshops**: Local developer communities worldwide

Every tiagent talk is implicitly a Celestia talk, because explaining how
cross-agent learning works requires explaining Celestia DA.

### Reaching non-blockchain developers

This is the strategic leverage that makes tiagent unique as an ecosystem
growth strategy. The target audience is not "Celestia developers" or even
"blockchain developers." The target audience is **every developer who
writes code**---approximately 30 million people worldwide.

The vast majority of these developers have never interacted with a
blockchain. tiagent reaches them where they already are: writing code,
using coding agents, looking for better tools. Celestia enters their
awareness not as "a blockchain" but as "the infrastructure that makes
their coding agent smarter."

This is the same playbook that made AWS successful: developers adopted
AWS not because they cared about cloud infrastructure, but because it
made their applications easier to build. Celestia, via tiagent, becomes
infrastructure that developers use without needing to understand or care
about its blockchain nature.

### Viral mechanics

The network effect creates a natural viral loop:

1. Developer A uses tiagent and sees measurable improvement over time.
2. Developer A tells Developer B: "My agent keeps getting better."
3. Developer B asks how. Developer A explains the network effect.
4. Developer B installs tiagent. The network becomes slightly smarter.
5. Developer A's agent improves slightly (new data from Developer B).
6. Both developers now have incentive to recruit Developer C.

The key insight: **users are incentivized to recruit other users** because
each new user genuinely makes their own experience better. This is the
same dynamic that drove WhatsApp, Slack, and GitHub to massive scale.

---

## 7. Comparison to Other Ecosystem Growth Strategies

Celestia has several options for growing its developer ecosystem. Here
is how tiagent compares to the alternatives:

| Strategy | Typical Cost | Time to Measurable Impact | Developer Reach | Sustainability |
|---|---|---|---|---|
| Hackathons | $100-250K per event | 2-6 months | Hundreds per event | Low (one-time engagement) |
| Developer grants (individual) | $50-500K per grant | 6-12 months | Tens of developers | Medium (project may stall) |
| Ecosystem fund | $10-100M | 1-3 years | Thousands | High (portfolio approach) |
| Developer advocacy / DevRel | $500K-1M/year | 6-12 months | Thousands | Medium (stops when funding stops) |
| Documentation + tutorials | $100-200K | 3-6 months | Thousands | High (evergreen) |
| **tiagent grant** | **$200K** | **6-12 months** | **Potentially millions** | **Very High (self-reinforcing)** |

### Why tiagent is highest-leverage

1. **Market size.** Hackathons, grants, and DevRel target blockchain
   developers (approximately 30,000-50,000 active Celestia ecosystem
   developers). tiagent targets all software developers (approximately
   30 million). The addressable market is 600-1000x larger.

2. **Self-reinforcing growth.** Every other strategy requires ongoing
   investment to maintain momentum. tiagent's network effects mean that
   growth compounds without additional funding once the flywheel is
   spinning. The $200K grant is a one-time catalyst, not an ongoing cost.

3. **DA demand generation.** No other ecosystem growth strategy directly
   creates DA demand. Hackathons produce demo projects. Grants produce
   niche tools. tiagent produces daily, recurring blob submissions from
   every active user. This is real economic activity on the Celestia
   network.

4. **Narrative leverage.** "Celestia powers the world's first collectively
   intelligent coding agent" is a more compelling narrative than "Celestia
   funded 20 hackathon projects" or "Celestia gave grants to 5 rollup
   teams." The tiagent story is unique, memorable, and differentiated.

### Return on investment comparison

| Investment | Amount | Expected 2-Year DA Revenue | Developer Acquisition Cost | Narrative Value |
|---|---|---|---|---|
| 10 hackathons | $2M | ~$0 (demos, not production) | ~$2,000/developer | Low (generic) |
| 20 individual grants | $2M | Variable, often $0 | ~$10,000/developer | Medium |
| Ecosystem fund | $50M | Spread across portfolio | ~$5,000/developer | High |
| **tiagent** | **$200K** | **$76K-$7.6M** (at 1K-100K agents) | **<$1/developer** (at scale) | **Very High** |

The developer acquisition cost comparison is striking. Traditional
ecosystem strategies cost thousands of dollars per developer because
they require active outreach, incentives, and support. tiagent's network
effects mean that after the initial investment, growth is organic---
driven by the product being genuinely better than alternatives. The
marginal cost of acquiring the 10,000th user approaches zero.

---

## 8. Risk and Mitigation

### Risk: Adoption is slow

**Scenario**: Developers do not switch from Claude Code / Cursor / Codex
to tiagent quickly enough for network effects to materialize.

**Mitigation**: tiagent works as a standalone agent without any Celestia
integration. The self-improvement loops (inner loop: per-execution
feedback, middle loop: cross-task learning) operate entirely locally.
A developer gets measurable value from tiagent even if they are the only
user in the world. The network effect is a bonus, not a requirement.

This means tiagent can grow at its own pace. Even slow adoption produces
value for early users and accumulates learning data for later users.
There is no "minimum viable network size" below which the product is
useless.

**Downside protection**: At worst, Celestia funds a good open-source
coding agent that uses DA for storage. At best, it funds a platform
with transformative network effects.

### Risk: Competitors copy the approach

**Scenario**: After seeing tiagent's traction, Claude Code or Cursor adds
cross-user learning, potentially using a competing DA layer.

**Mitigation**: Three factors protect tiagent's position:

1. **Data moat compounds daily.** By the time a competitor launches a
   similar feature, tiagent's network will have months of accumulated
   learning data. The competitor starts at zero.

2. **First-mover in namespace and schema design.** tiagent defines the
   standard for agent learning data on Celestia. Competitors must either
   adopt this standard (strengthening tiagent's ecosystem) or create an
   incompatible alternative (fragmenting the market).

3. **Open-source trust.** Claude Code and Cursor are proprietary.
   Developers are increasingly wary of sharing their coding patterns
   with closed-source companies. tiagent's open-source, verifiable
   learning layer is a structural trust advantage.

4. **Ecosystem integrations.** TraceCommons, MCP, and IronClaw
   partnerships create switching costs that a copycat feature cannot
   replicate overnight.

### Risk: DA costs spike

**Scenario**: Celestia DA pricing increases significantly, making per-
agent costs prohibitive.

**Mitigation**: Three layers of protection:

1. **Small artifacts.** Most learning data is deltas, not full snapshots.
   A routing weight update is 1-5 KB, not megabytes. Even at 10x current
   pricing, the per-agent cost would be $0.50-5.00/day---still well
   within the value created.

2. **Adaptive publication.** tiagent's quality gates filter out low-value
   artifacts before publication. Only novel, high-quality learning data
   reaches DA. This means volume scales with value, not with raw activity.

3. **Celestia's roadmap.** Celestia's scaling roadmap (128 MB blocks today,
   targeting 1 TB/s with Fibre/V8) explicitly aims to reduce per-byte
   costs over time. The long-term trend is toward cheaper DA, not more
   expensive DA.

### Risk: Learning data quality degrades

**Scenario**: As the network grows, low-quality or adversarial learning
data dilutes the collective intelligence.

**Mitigation**: tiagent implements multiple quality filters:

1. **Local quality gates.** Each agent evaluates learning artifacts before
   publication. Only artifacts that meet novelty and substance thresholds
   are submitted.

2. **Reputation weighting.** Agents that consistently produce high-quality
   artifacts receive higher weight in the collective intelligence.
   Agents that produce noise are deprioritized.

3. **Verifiable provenance.** Every artifact on Celestia DA has an
   inclusion proof. Adversarial data can be traced to its source and
   excluded.

4. **HDC fingerprinting.** Hyperdimensional computing fingerprints
   detect duplicate or near-duplicate artifacts, preventing the corpus
   from being flooded with redundant data.

---

## 9. Long-Term Strategic Value

### For Celestia

tiagent represents something Celestia does not currently have: a consumer-
facing application with demand-side network effects that generates
sustained, growing DA demand. Today, Celestia's DA consumers are
primarily rollups---infrastructure projects used by other infrastructure
projects. tiagent brings DA demand from a fundamentally different source:
individual developers and their coding agents.

This matters because diverse demand sources make the Celestia network
more resilient. If DA demand comes only from rollups, Celestia's economic
activity is correlated with rollup adoption cycles. If DA demand also
comes from millions of coding agents, the demand base is broader and
more stable.

### For the broader AI agent ecosystem

tiagent, built on Celestia, demonstrates that public verifiable
infrastructure can power AI agent coordination at scale. This proof of
concept has implications far beyond coding agents:

- Research agents that share findings through DA
- Trading agents that publish strategy performance data
- Customer service agents that share resolution playbooks
- DevOps agents that share infrastructure optimization patterns

Each of these represents a new category of DA demand. tiagent is the
beachhead---the first demonstration that the pattern works. If it
succeeds, it opens the door to an entire ecosystem of DA-powered agent
applications.

### The $200K question

Celestia's most recent funding round was $100M (Series C, September 2024,
led by Bain Capital Crypto). Against that context, the tiagent grant
request of $200K is 0.2% of one fundraising round.

For that 0.2%, Celestia gets:

- The first application with demand-side network effects in its ecosystem
- A path to millions of non-blockchain developers using Celestia DA daily
- DA demand that could scale to $76M/year at mass adoption
- A compelling, differentiated narrative for AI agent infrastructure
- An open-source project that the community can build on and extend
- A marketing vehicle that positions Celestia at every AI developer
  conference

The question is not whether $200K is a lot of money. The question is
whether any other $200K investment in the Celestia ecosystem has the
potential to generate this kind of return. We believe the answer is no.

---

## Summary

| Property | tiagent | Every Other Coding Agent |
|---|---|---|
| Value curve | Superlinear (network effects) | Linear (isolated) |
| Learning | Local + cross-agent via Celestia DA | None or proprietary |
| Growth | Self-reinforcing flywheel | Marketing-dependent |
| Moats | Data + protocol + ecosystem + open-source | Brand only |
| DA demand | Growing linearly with users | None |
| Developer reach | 30M+ (all developers) | N/A for Celestia |
| Grant cost | $200K (one-time catalyst) | N/A |

tiagent is not just another coding agent. It is a network---a collectively
intelligent system that gets smarter with every user, runs on public
verifiable infrastructure, and generates real economic activity for
Celestia. The $200K grant is the spark that lights a self-reinforcing
flywheel. Once spinning, it does not need additional funding to keep
going. It needs users, and the product is designed to attract them.
