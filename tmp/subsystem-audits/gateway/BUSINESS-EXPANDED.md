# Expanded Business Model: Nunchi Gateway + Roko + Chain

This expands on BUSINESS.md with insights from the learnings2 briefing set. Cross-references the strategy, competitive intelligence, co-evolution research, and clearing-as-inference concepts.

---

## Revenue Architecture: Four Streams (Not Three)

The original BUSINESS.md had three streams (subscription, usage, crypto). The learnings2 research reveals a fourth that's arguably the most defensible.

### Stream 1: Gateway Subscriptions (Near-Term, $0-$99/mo)

Same as BUSINESS.md. Free/Starter/Pro/Custom tiers. This is the Temporal playbook: give away the OSS runtime, charge for managed infrastructure.

### Stream 2: Enterprise Support + Licensing ($5K-$100K+/mo)

From learnings2/03-BUSINESS-MODEL.md — the beachhead is **enterprise support on Roko OSS, not a managed platform**. Temporal raised $18.75M Series A with ZERO commercial product. Revenue started as enterprise support.

| Offering | Price | What |
|---|---|---|
| Community support | Free | GitHub issues, Discord |
| Enterprise Basic | $5K/mo | SLA, priority support, 2 named contacts |
| Enterprise Pro | $25K/mo | SLA, Slack channel, architecture reviews, 4 contacts |
| Enterprise Premium | $100K+/mo | Forward-deployed engineers (Palantir model), custom gates, compliance |
| ISO 42001 scaffolding | $85-150K upfront + $40-60K/yr | AI governance system certification support |

**Critical correction**: ISO 42001 carries NO Annex ZA, is NOT a harmonised European standard, creates NO presumption of conformity under EU AI Act. Position as "governance scaffolding demonstrating management system maturity" — not compliance certificate.

### Stream 3: Chain Economics (Long-Term)

When the nunchi chain launches (month 18-24+):

| Fee | Rate | At $10M/mo GMV |
|---|---|---|
| Job marketplace (ERC-8183) | 5% (2% escrow + 3% marketplace) | $500K/mo |
| Knowledge posting | Trust Credits (burns NUNCHI) | Deflationary pressure |
| Knowledge queries | Trust Credits | Deflationary pressure |
| Block production | Validator fees | Network-dependent |
| Identity staking | Required to access network | Lock-up value |

**Token design**: Helium-hybrid burn-and-mint. Agents pay in Trust Credits (USD-pegged). Trust Credits minted by burning NUNCHI at oracle rate. More network demand → more NUNCHI burned → deflationary. This is the only token mechanic that survived the 2025 AI token crash (VIRTUAL -86%, ELIZAOS -99.98%, FET -94%).

### Stream 4: Knowledge Marketplace (The Moat)

This is the genuinely novel revenue stream. **Every gateway request generates learning signal. Aggregated learning signal is the product.**

| Product | Customer | Pricing |
|---|---|---|
| Routing intelligence API | Other gateway operators, framework builders | Per-query or subscription |
| Task-model fit database | AI companies evaluating models | Subscription |
| Benchmark-as-a-service | Model providers wanting eval data | Per-benchmark-run |
| Affordance datasets | DevTool companies | Subscription |
| Cross-agent tool usage patterns | Framework builders | Subscription |

**Why this works**: After 1 year of operation with 100+ customers, the gateway has millions of verified observations about which models work for which tasks at what cost. This dataset is worth more than the gateway itself. OpenRouter has the traffic but doesn't learn from it. We have the traffic AND the learning loop.

**Pricing constraint**: Never sell raw customer data. Only sell aggregated statistical artifacts. Privacy is non-negotiable.

---

## Expanded Market Sizing

### Primary Market: LLM Gateway/Routing

| Segment | 2026 TAM | Nunchi Share (Y3) |
|---|---|---|
| Individual dev gateway (coding) | $500M | 2% = $10M |
| Agent developer gateway | $200M | 5% = $10M |
| Enterprise AI infrastructure | $2B | 0.5% = $10M |
| **Subtotal** | **$2.7B** | **$30M** |

### Adjacent Market: Agent Coordination

| Segment | 2026 TAM | Nunchi Share (Y3) |
|---|---|---|
| Non-Human Identity management | $9.45B → $18.7B by 2030 | 0.3% = $28M |
| Multi-agent orchestration | $1B | 3% = $30M |
| AI governance/compliance | $500M | 2% = $10M |
| **Subtotal** | **$11B** | **$68M** |

### Long-Horizon Market: On-Chain Agent Economy

| Segment | Potential | Timeline |
|---|---|---|
| On-chain rate derivatives (ISFR) | $668T TradFi → even 0.001% on-chain = $668M | Year 3-5 |
| Agent job marketplace | Scales with agent population | Year 2-4 |
| Cross-domain knowledge marketplace | Novel, no comparable | Year 2-3 |

---

## Competitive Positioning (Updated)

### Why We Win Against Each Competitor

**vs. OpenRouter** (marketplace):
- OpenRouter sees traffic but doesn't learn. No PPC loop, no CascadeRouter, no affordance-aware routing.
- OpenRouter charges 5.5% on credits. We charge subscription + usage, with costs that drop over time.
- "OpenRouter is a marketplace. Nunchi is a learning system. Marketplaces facilitate. Learning systems compound."

**vs. LiteLLM** (infrastructure):
- LiteLLM is plumbing. 100+ providers, zero intelligence.
- No caching beyond basic response cache. No model selection learning. No tool optimization.
- "LiteLLM is the haproxy of LLM routing. Nunchi is the Cloudflare — intelligence at the edge."

**vs. Portkey** (observability):
- Portkey's semantic cache reduces cost but doesn't learn routing.
- No cross-agent intelligence. No affordance awareness. No fine-tuning loop.
- "Portkey tells you what happened. Nunchi prevents the expensive thing from happening in the first place."

**vs. Temporal** ($5B, workflow durability):
- "Temporal owns 'did this code run.' Nunchi owns 'did the right agent, with the right memory, at the right price, with a receipt the counterparty can verify.'"
- Temporal has no inference routing, no model selection, no cost optimization.
- Complementary, not competitive. Nunchi sits beneath Temporal's durable execution.

**vs. Braintrust** ($800M, evaluation):
- Braintrust evaluates after the fact. Nunchi optimizes before the request is sent.
- No cross-agent learning. No economic incentives. No chain settlement.
- "Braintrust is the test suite. Nunchi is the optimizer."

**The empty quadrant we occupy**: learning gateway + agent coordination plane + on-chain settlement. No competitor combines:
1. Multi-provider routing that learns (CascadeRouter)
2. Multi-layer caching with HDC semantic matching
3. Cross-agent tool and affordance intelligence
4. Epistemic reputation with CRPS scoring
5. On-chain identity + marketplace + knowledge substrate
6. Self-funding agent economics
7. ZK proofs over behavioral fingerprints

---

## The 10-20x Cost Reduction Claim (Honest Math)

From learnings2/04-RESEARCH.md — HAL Princeton benchmark is the gold standard.

### Base Case (No Optimization)

Naive SWE-Agent with Claude Opus: **$44.86/task** (HAL benchmark, 21,730 rollouts).

### Layer-by-Layer Reduction

| Layer | Factor | Mechanism | Running Cost |
|---|---|---|---|
| Baseline | 1.0x | Direct API, opus for everything | $44.86 |
| Prompt caching | 0.20x | Anthropic prefix cache (90% off reads) | $8.97 |
| Tier routing | 0.40x | RouteLLM: 85% cost at 95% quality | $3.59 |
| Waste trimming | 0.60x | Semantic dedup, tool pruning, context compress | $2.15 |
| Batch scheduling | 0.50x | Async batch API for non-urgent work | $1.08 |
| **Combined** | **~0.024x** | Stacked multipliers | **$1.08** |

**~41x theoretical reduction. 10-20x practical** (not all requests are batchable, not all cache, routing doesn't always find cheaper).

### Disclosure Requirements

- HAL baseline **explicitly excludes caching**. Caching alone (80-90% hit rate) = 4-5x reduction.
- The full stack gets to 30x+ theoretical. Practical depends on workload mix.
- Always disclose the intermediate step. Dishonesty destroys credibility.
- Claude Opus 4.7's new tokenizer uses up to 35% MORE tokens — recalculate demo numbers.

### Per-Customer Savings Example

| Customer Profile | Monthly Spend (Direct) | Monthly Spend (Nunchi) | Savings |
|---|---|---|---|
| Solo dev, heavy coding | $200 | $40-70 | 65-80% |
| Small agent team (5 agents) | $1,500 | $300-500 | 67-80% |
| Enterprise (50 agents) | $15,000 | $2,500-5,000 | 67-83% |
| Heavy agent workload (500 agents) | $150,000 | $20,000-40,000 | 73-87% |

Savings increase with scale because:
1. Cache hit rates increase with more similar requests
2. CascadeRouter has more learning signal
3. Batch scheduling amortizes better
4. Cross-agent tool intelligence compounds

---

## Cybernetic Self-Learning Loops (Complete Catalog)

### Loop 1: CascadeRouter (Model Selection Learning)

```
Request → Route to model → Observe outcome (pass/fail, cost, latency) →
  LinUCB bandit updates → better routing → lower cost / higher quality →
  More customers → more observations → even better routing
```

**Convergence**: Static (hardcoded) → Confidence (50+ obs, Wilson interval) → UCB (200+ obs, full contextual bandit). After ~1000 observations per task category, routing is near-optimal.

### Loop 2: Cache Regime Adaptation

```
Request pattern changes → Cache hit rate shifts →
  Regime detector fires (Calm/Normal/Volatile/Crisis) →
  TTLs adjust (2h → 15min → 5min) →
  Cache hit rate recovers → Cost stabilizes
```

**Why it's novel**: Other caches have static TTLs. Ours adapt to workload volatility.

### Loop 3: Budget Degradation Learning

```
Customer approaches budget → Tier degrades (Full → T1Only → Economy → Block) →
  Degradation thresholds adjusted based on customer's actual quality tolerance →
  Some customers tolerate aggressive degradation (automation) →
  Others need gentle curves (interactive coding) →
  Gateway learns per-key degradation curves
```

### Loop 4: Prompt Experiment Convergence

```
A/B test prompt sections (system prompt structure, tool instructions, etc.) →
  UCB1 allocates traffic to variants →
  Winner emerges after N trials →
  Winner auto-promoted to default →
  New experiment started on next section
```

**Compounding**: Each winning prompt section improves success rates, which improves CascadeRouter observations, which improves routing.

### Loop 5: Provider Health Adaptation

```
Provider degrades (higher latency, more errors) →
  Circuit breaker opens (3 consecutive failures) →
  Traffic routes to alternatives →
  Half-open probe detects recovery →
  Traffic gradually returns →
  Health scores update for future routing
```

### Loop 6: Tool Usage Optimization

```
Gateway observes tool calls across all agents →
  Builds tool dependency graph →
  Identifies unused tools (defined but never called) →
  Prunes unused tools from future requests (saves 2-5K tokens) →
  Identifies missing tools (agent keeps failing without tool X) →
  Suggests tool additions
```

### Loop 7: Affordance-Routing Feedback

```
Agent improves code quality → Affordance score rises →
  CascadeRouter routes to cheaper model → Cost drops →
  Saved budget enables more improvements → Affordance rises further →
  Geometric compounding: 1% per invocation × 200 = 625%
```

### Loop 8: Fine-Tuning Cycle

```
Successful episodes → Training data →
  AutoTrain fine-tune on HuggingFace →
  Push model to Hub →
  CascadeRouter adds as bandit arm →
  Explore: does fine-tuned model win? →
  If yes → more traffic → more training data → repeat
```

### Loop 9: Cross-Instance Knowledge Propagation

```
Instance A learns routing insight →
  Publishes to Hub (aggregated, anonymized) →
  Instance B pulls → incorporates → discovers related insight →
  Publishes back →
  Instance C (new) starts with collective knowledge of A + B
```

### Loop 10: Epistemic Reputation Compounding

```
Agent makes accurate predictions (CRPS-scored) →
  Reputation rises → Cheaper access + priority →
  Completes more tasks → More predictions →
  More accurate calibration → Higher reputation
```

### The Interaction Effect

These loops don't operate independently. They share data substrate:
- Loop 1 (routing) feeds Loop 4 (experiments) — routing data reveals which prompts work
- Loop 6 (tools) feeds Loop 7 (affordance) — tool patterns indicate code quality
- Loop 8 (fine-tuning) feeds Loop 1 (routing) — new models become routing candidates
- Loop 9 (cross-instance) accelerates ALL other loops by sharing signal

**The system's learning rate is superlinear** — each new loop accelerates every other loop.

---

## Business Opportunities Not in Original Docs

### 1. Benchmark-as-a-Service

The gateway + HuggingFace integration enables continuous benchmarking:

```bash
roko bench swe --repeat 0 --batch-size 50 --shuffle
# Perpetual grinder: samples instances, CascadeRouter picks models,
# gates validate, learning loops fire, scores accumulate
```

**Sell to model providers**: "Test your model against 500+ real-world coding tasks, with detailed per-category breakdowns, for $X/run. Results include comparison against all models in our routing table."

Revenue: $500-$5,000 per benchmark run. At 10 model providers running monthly: $60K-$600K ARR.

### 2. Compliance Gateway (EU AI Act)

EU AI Act enforcement: **August 2, 2026**. Penalties: €35M or 7% global turnover.

The gateway is uniquely positioned to provide Article 50 transparency:
- Every request logged with model used, cost, provider
- Every response auditable (which model, what safety checks ran)
- Epistemic reputation provides Article 50(2) "adequately informed" compliance
- ZK-HDC proofs provide Article 50(4) machine-readable disclosures

**Compliance add-on**: $500-$2,000/mo on top of gateway subscription. Provides:
- Audit trail (every inference logged)
- Model card registry (which models are in use, capabilities declared)
- Safety reports (PII scans, injection detections, privacy classifications)
- Article 50 transparency documents (auto-generated)

Only 35.7% of EU managers feel prepared (Deloitte). Massive demand.

### 3. Agent Insurance / SLA Marketplace

The gateway has the data to price **agent reliability SLAs**:

```
Agent X has completed 847 tasks with 94% gate pass rate.
Agent X's epistemic reputation is "Expert" tier (CRPS top 30%).
Agent X's metabolic ratio is 1.7x (self-sustaining).

SLA offer: "Agent X will complete Task Y at 90% confidence for $Z.
If fails, refund + penalty."
```

This is insurance underwriting using the gateway's observational dataset. No competitor has the data to price this.

### 4. Model Provider Partnerships

Model providers want distribution. The gateway provides it:

- "List your model in our CascadeRouter" → model gets traffic → model provider pays for inclusion
- "Priority routing for your model for Task Category X" → sponsorship model
- "We'll run continuous benchmarks on your model" → eval partnership

This flips the revenue model: instead of us paying providers, **providers pay us for distribution**.

### 5. White-Label Gateway

Other companies want intelligent routing but don't want to build it:

- Framework builders (LangChain, CrewAI, Mastra) → embed Nunchi routing as a library
- Cloud providers → white-label gateway for their AI platform
- Enterprise → on-premises gateway with all the learning loops

License: BSL with 4-year Apache conversion (protects revenue during growth).

---

## The Full Nunchi Revenue Stack

| Stream | Timeline | Y1 | Y3 |
|---|---|---|---|
| Gateway subscriptions (Free→Pro) | Immediate | $200K | $2M |
| Enterprise support | Month 3 | $300K | $5M |
| Compliance add-on | Month 6 | $100K | $3M |
| Benchmark-as-a-service | Month 6 | $50K | $600K |
| Knowledge marketplace | Month 12 | $0 | $2M |
| Model provider partnerships | Month 12 | $0 | $1M |
| Chain economics (marketplace fees) | Month 24+ | $0 | $500K |
| White-label licensing | Month 18 | $0 | $1M |
| **Total** | | **$650K** | **$15.1M** |

Conservative estimates. The Temporal comp ($5B at 40-60x ARR) suggests $15M ARR supports a $600M-$900M Series B valuation.

---

## Why This Is Hard to Replicate

1. **The InsightStore compounds**: After 1 year, millions of verified model-task-cost observations. Code is forkable in hours. Data takes a year to regenerate.

2. **Cross-agent learning has network effects**: Each customer's traffic improves routing for all customers. N customers → N² learning interactions.

3. **Epistemic reputation is non-transferable**: An agent's CRPS history on Nunchi doesn't transfer to a competing gateway. Switching costs increase with reputation accumulation.

4. **Protocol convergence window is closing**: MCP (97M SDK downloads), A2A (150+ orgs), ERC-8004 (~80-150K projected agents). 6-12 months to establish as the canonical coordination layer. After protocol lock-in, switching costs are enormous.

5. **HDC + ZK is a genuine first**: No competitor has demonstrated ZK proofs over agent behavioral fingerprints. IBM NorthPole projection: >100M HDC similarity searches/second on a single chip. The hardware trajectory favors our architectural bet.

6. **The ten loops interact**: Competing with one loop is easy. Competing with ten loops that accelerate each other is a coordination problem as hard as the one we're solving.
