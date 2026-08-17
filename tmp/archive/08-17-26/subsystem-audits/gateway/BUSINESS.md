# Nunchi Gateway: Business Model & Economics

## The Opportunity

AI agent developers and individual coding devs currently face a choice:
1. **Subscription** (Codex $200/mo, Claude Code Max $100-$800/mo) — simple but expensive, inflexible
2. **Direct API** — full control but complex (manage keys, handle rate limits, no caching, no routing)
3. **Existing gateways** (OpenRouter, LiteLLM, Portkey) — middleware tax, limited intelligence

Nunchi can offer something none of them do: a gateway that **learns** which models work best for which tasks, automatically routes to the cheapest option that meets quality requirements, and gets cheaper over time through cybernetic self-optimization.

## Competitive Landscape (April 2026)

### Marketplace/Passthrough

| Service | Pricing Model | Markup | Differentiator |
|---|---|---|---|
| OpenRouter | Credit purchase fee (5.5%) | 0% on tokens | 300+ models, convenience |
| TokenMix | Below-market passthrough | -3 to -8% | Volume aggregation |
| HuggingFace | Pass-through | 0% | HF model ecosystem |

### Observability-First

| Service | Pricing Model | Base Cost | Differentiator |
|---|---|---|---|
| Portkey | Subscription + pass-through | $49/mo Pro | Semantic caching, fallbacks |
| Helicone | Subscription + pass-through | $79/mo Pro | Observability, tracing |
| Braintrust | Freemium | Free (beta) | Tracing + eval |

### Infrastructure

| Service | Pricing Model | Base Cost | Differentiator |
|---|---|---|---|
| LiteLLM | Open source / enterprise license | $250/mo Basic | 100+ providers, self-host |
| Bifrost | Open source / enterprise | Free core | Sub-100µs overhead |
| Cloudflare AI Gateway | Freemium | $5/mo | CDN integration, unified billing |

### What's Missing From All of Them

None of these gateways:
- **Learn from usage** — they don't get smarter about which model to pick over time
- **Adapt budgets** — they enforce static limits, not dynamic degradation curves
- **Optimize prompts** — no A/B experiments, no winner promotion
- **Detect behavioral patterns** — loop detection, convergence, drift
- **Run batch opportunistically** — no automatic batch routing for non-urgent work
- **Provide agent-native features** — tool pruning, context compression, playbook injection

The bardo gateway already does most of this. Roko's CascadeRouter does the learning. Combining them creates something unique.

## Pricing Strategy

### Model: Hybrid Subscription + Usage

```
┌─────────────────────────────────────────────────────────┐
│                    nunchi gateway                        │
│                                                         │
│  Free Tier         Starter          Pro         Custom  │
│  ──────────        ───────          ───         ──────  │
│  $0/mo             $29/mo           $99/mo      Custom  │
│  100K tokens/day   10M tokens/day   100M/day    ∞       │
│  3 models          All models       All models  All     │
│  No caching        L1 cache         3-layer     Custom  │
│  No batch          Batch API        Batch+prio  SLA     │
│  Community         Email            Priority    Slack   │
│  1 API key         5 keys           25 keys     ∞       │
│  No analytics      Basic analytics  Full suite  Custom  │
│                                                         │
│  Overage: $2.50 / M tokens (blended)                    │
│  Crypto: Per-request USDC micropayments (any tier)      │
└─────────────────────────────────────────────────────────┘
```

### Why This Beats $200/mo Codex

For a heavy coding developer spending ~$200/mo on Codex:

| Cost Component | Codex ($200/mo) | Nunchi Pro ($99/mo) |
|---|---|---|
| Simple tasks (autocomplete, explain) | GPT-5.x (expensive) | Route to DeepSeek/Gemini Flash ($0.14-0.60/M) |
| Hard tasks (architecture, debug) | GPT-5.x | Route to Opus/GPT-5.4 (best quality) |
| Repeated questions | Full price | Semantic cache (30-70% hit rate) |
| Non-urgent work (review, docs) | Full price | Batch API (50% off) |
| System prompt overhead | Full price every request | Prefix cache (90% off reads) |
| Learning over time | No improvement | CascadeRouter learns → cheaper routing |

**Estimated effective cost**: $40-70/mo in actual provider spend for $200/mo equivalent usage, depending on workload mix and cache hit rates.

**Margin at $99/mo subscription**:
- Provider cost: ~$40-70/mo (after caching + routing)
- Gross margin: $29-59/mo per customer (30-60%)
- Infrastructure: ~$500/mo amortized over first 50 customers = $10/customer
- **Net margin: $19-49/mo per customer** at 50 customers

### The Agent Developer Angle

Agent builders need something different than coding devs:

| Need | Value |
|---|---|
| Multi-model routing | CascadeRouter picks cheapest model per task type |
| Budget enforcement | 4-tier degradation prevents runaway costs |
| Batch for non-urgent | Automatic 50% savings on async work |
| Tool optimization | Prune unused tools, compress schemas (2-5K tokens/req saved) |
| Loop detection | Catch agent loops before they burn $50 |
| Per-agent analytics | Know which agent costs what |
| Self-funding mode | Agents earn revenue → pay for own inference |

These are features no competitor offers because they're not built by agent developers.

## Revenue Projections

### Conservative (Year 1)

| Metric | Month 3 | Month 6 | Month 12 |
|---|---|---|---|
| Free users | 100 | 500 | 2,000 |
| Starter ($29) | 10 | 50 | 200 |
| Pro ($99) | 5 | 25 | 100 |
| MRR | $785 | $3,925 | $15,700 |
| Token cost (est.) | $400 | $2,000 | $8,000 |
| Infra cost | $500 | $800 | $1,500 |
| Gross profit | -$115 | $1,125 | $6,200 |

**Break-even**: ~Month 5 with 30 Starter + 10 Pro customers.

### Optimistic (Year 1, with network effects)

If roko's self-hosting story drives adoption (agent devs try roko → need gateway → pay for gateway):

| Metric | Month 6 | Month 12 |
|---|---|---|
| Pro ($99) | 100 | 500 |
| MRR | $9,900 | $49,500 |
| Gross margin (55%) | $5,445 | $27,225 |

## Unit Economics Deep Dive

### How We Get Cheaper Than Direct API

Five compounding cost reduction layers:

```
Raw provider cost:                     $100.00 (baseline)
After intelligent routing (30% cheap): -$25.00 → $75.00
After semantic cache (30% hit rate):   -$22.50 → $52.50
After prefix cache (system prompts):   -$15.75 → $36.75
After batch API (20% non-urgent):      -$3.68  → $33.07
After tool pruning (token savings):    -$3.31  → $29.76

Effective cost: $29.76 on $100.00 of naive spend
Savings: 70.2%
```

These are conservative estimates. Bardo gateway's cache analytics showed real-world savings of 40-85% on agent swarm workloads.

### Volume Discount Path

At scale (100+ customers), we can negotiate:
- **Anthropic**: Enterprise tier pricing (25-50% volume discount)
- **OpenAI**: Committed-use agreements (similar discounts)
- **DeepSeek**: Already has 50-75% off-peak discounts we can pass through
- **Aggregated HuggingFace**: Their free tier + provider competition means near-zero cost for many models

Even without volume discounts, the caching + routing stack alone provides enough margin.

## Crypto Micropayment Track (MPP)

For autonomous agents that need to pay programmatically:

### How It Works
1. Agent calls gateway, gets HTTP 402 with USDC quote
2. Agent signs ERC-3009 `transferWithAuthorization` (no gas needed — relay pays)
3. Agent retries with `X-Payment` header containing the signed authorization
4. Gateway verifies signature off-chain, serves the request
5. USDC transfer settles asynchronously on Base

### Session Mode (Optimized)
1. Agent pre-funds a session with one signed USDC deposit
2. Subsequent requests draw from session balance (no per-request signing)
3. Session auto-closes after TTL, refunding unused balance

### Spread / Markup
Default 20% over raw provider cost. Tiered discounts based on usage volume:
- New: 20% | Regular (5+ sessions): 18% | Power (25+): 15% | Trusted (100+): 12% | Sovereign (500+): 8%

### Why Crypto for Agents
- **No human in the loop**: Agents can pay without a credit card, Stripe subscription, or approval flow
- **Metered exactly**: Pay for what you use, no overages, no unused capacity
- **Composable**: Any smart contract can call the gateway — DAOs, autonomous agents, on-chain protocols
- **Auditable**: Every payment is on-chain

## Competitive Moats

### 1. Cybernetic Learning Loop
The gateway gets cheaper over time. No competitor does this:
```
Usage → CascadeRouter observes (pass/fail, cost, latency) →
  LinUCB bandit updates → better routing → lower cost →
  customer saves more → more usage → more learning → ...
```

### 2. Agent-Native Features
Built by agent builders for agent builders: tool pruning, loop detection, convergence, context compression, playbook injection. These don't exist in generic gateway products.

### 3. Open Source Core + Hosted Service
`roko-gateway` is open source (self-host for free). Nunchi hosted service adds: multi-tenant, managed caching infra, volume discounts, SLA, crypto payments. Same model as LiteLLM but with intelligence.

### 4. HuggingFace Integration
Dynamic model discovery → CascadeRouter explores new models automatically → fine-tuning loop closes → models improve → published back to Hub → network effects.

### 5. Roko Ecosystem
Gateway integrates with roko's full stack: orchestrator, learning, safety, knowledge. No other gateway has an agent framework feeding it signal.

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Providers cut prices (margin squeeze) | High | Caching + routing savings compound independently of provider pricing |
| OpenRouter adds learning features | Medium | They're a marketplace, not an agent platform. Different DNA. |
| Anthropic/OpenAI offer native routing | Medium | They won't route across each other. Multi-provider is our edge. |
| Low conversion from free → paid | Medium | Free tier is genuinely useful. Agent devs hit limits fast. |
| Crypto regulatory risk | Low | USDC on Base is compliant. Stripe fallback for traditional billing. |
| Volume discount negotiations fail | Low | Caching alone provides sufficient margin. |
