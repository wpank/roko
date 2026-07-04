# Korai Narrative: The Full Picture and What's Missing

## The Vision (How It Should Work)

Every roko agent, before it does anything, assembles a prompt. That prompt assembly
is a **market** — 8 cognitive subsystems (Neuro, Daimon, Playbooks, Research, Task,
Code Intelligence, Oracles, Iteration Memory) bid for context window space via a
VCG auction. The winning sections get included; losers get dropped.

Korai is supposed to be the **global knowledge layer** that the Neuro bidder queries.
The flow:

```
Agent gets a task
  → fingerprint task as 10,240-bit HDC vector
  → query Korai chain: hdc_topk(task_fingerprint, k=20)
  → chain returns 20 most similar knowledge entries
      (weighted by submitter reputation, filtered by access)
  → Neuro bidder packages these as prompt sections
  → VCG auction decides which knowledge entries win budget
  → agent runs with globally-informed context
  → outcome logged → episode fingerprinted
  → successful patterns published back to Korai
  → next agent's query returns better results
```

This is the flywheel: agents query → learn → publish → other agents query better
results → learn more → publish more → ...

The chain ensures:
- **Persistence** — knowledge survives process death
- **Reputation-gating** — bad knowledge from low-rep agents gets downweighted
- **Economic incentives** — agents pay KORAI to query, earn KORAI for useful contributions
- **Demurrage** — stale knowledge decays (1% annual token decay mirrors neuro store half-lives)
- **HDC precompile** — sub-millisecond similarity search on-chain (~400 gas for top-20)

## What Actually Exists Today

### Built and Working

| Component | Status | Where |
|-----------|--------|-------|
| HDC primitives (10,240-bit vectors) | Working | roko-primitives |
| Per-episode fingerprinting | Wired in orchestrate.rs | episodes.jsonl |
| VCG auction for prompt composition | Wired | roko-compose/src/auction.rs |
| 8 attention bidders | Wired | roko-compose/src/prompt.rs |
| Section effectiveness learning | Wired | roko-learn/src/section_effect.rs |
| Thompson-like bidder learning | Wired | roko-compose/src/auction.rs |
| Neuro knowledge store (local) | Built | roko-neuro/src/knowledge_store.rs |
| Neuro HDC encoder | Built | roko-neuro/src/hdc.rs |
| Cross-domain resonance detection | Built | roko-primitives/src/codebook.rs |
| Episode clustering (k-medoids) | Built | roko-learn/src/hdc_clustering.rs |
| Resonant pattern dynamics | Built | roko-learn/src/resonant_patterns.rs |
| Somatic markers (k-d tree) | Built | roko-daimon/src/somatic_ta.rs |
| Korai chain contracts (Rust) | Built | roko-chain/src/ (52 tests) |
| Mirage EVM simulator | Built | mirage-rs (141 tests) |
| HDC precompile spec | Spec'd | docs/08-chain/03-hdc-on-chain-precompile.md |
| Agent Registry (soulbound) | Built | roko-chain/src/agent_registry.rs |
| Reputation Registry (7-domain) | Built | roko-chain/src/reputation_registry.rs |
| Marketplace (Spore) | Built | roko-chain/src/marketplace.rs |
| ISFR (fact registry) | Built | roko-chain/src/isfr.rs |
| SystemPromptBuilder (9 layers) | Wired | roko-compose/src/system_prompt_builder.rs |
| Complexity-adaptive budgets | Wired | roko-compose/src/budget.rs |

### Built But NOT Connected

| Gap | What's missing | Blocks |
|-----|---------------|--------|
| **Neuro → prompt** | Knowledge store never queried at compose time | Neuro bidder has nothing to bid with |
| **Episode clustering** | k-medoids never called on accumulated episodes | No cluster-level reasoning |
| **Resonance detection** | Never called at dispatch time | No cross-domain transfer |
| **Somatic markers** | Never populated from episodes | No "gut feeling" fast-path |
| **Chain → Neuro** | No RPC client connecting to Korai/Mirage | Can't query on-chain knowledge |
| **Neuro → Chain** | No publish path from local knowledge to chain | Can't contribute knowledge |
| **HDC precompile** | Not implemented in Mirage | Can't run similarity on-chain |
| **Reputation → bid weight** | Reputation scores don't inform VCG auction | All knowledge weighted equally |
| **PP-HDC privacy** | Not implemented | Can't safely share vectors |

### NOT Built

| Component | What needs building | Est. effort |
|-----------|-------------------|-------------|
| HDC precompile in Mirage | revm custom precompile for bind/similarity/topk | ~500 lines |
| Chain RPC client in roko-agent | Query Korai from dispatch path | ~400 lines |
| Knowledge publish pipeline | Episode → distill → encode → submit to chain | ~600 lines |
| PP-HDC hash-encoding | Distance-preserving non-invertible projection | ~300 lines |
| Arena framework (roko bench) | Benchmark harness generating training signal | ~1,400 lines |
| HuggingFace integration (roko-hf) | Dataset loading, model discovery, fine-tuning | ~1,500 lines |

## The Gap Analysis: What's Actually Missing

### Gap 1: The Neuro Bidder Is Empty

The VCG auction runs every time a prompt is built. The Neuro bidder (`AttentionBidder::Neuro`)
exists and competes. But it has **nothing to bid with** because:

1. The local neuro store (`roko-neuro/src/knowledge_store.rs`) is built but never queried
   at compose time
2. There's no chain query path — `hdc_topk` doesn't exist yet in Mirage
3. The `build_learned_context()` in orchestrate.rs (line ~13762) builds skills, playbook rules,
   and patterns — but NOT neuro knowledge entries

**Fix**: At dispatch time, encode the task as an HDC vector, query the local neuro store
(and eventually Korai) for similar entries, package them as prompt sections tagged
`AttentionBidder::Neuro`, and let the VCG auction decide if they're worth including.

### Gap 2: No Chain Query Path

The `ChainClient` trait exists in roko-chain but has no live implementation. There's no
code that:
1. Connects to a Korai node (or Mirage) via JSON-RPC
2. Calls `hdc_topk(query_vector, k)` on the precompile
3. Deserializes the returned knowledge entries
4. Feeds them into the prompt composition pipeline

**Fix**: Implement a `MirageChainClient` that connects to the local Mirage instance.
When Korai goes live, swap the RPC endpoint — same interface.

### Gap 3: No Knowledge Publish Path

Agents learn things (successful episodes, playbooks, insights) but never publish them.
There's no code that:
1. Distills episode outcomes into knowledge entries
2. Encodes them as HDC vectors
3. Submits them to Korai (or Mirage) with reputation stake
4. Other agents can then query these entries

**Fix**: After `record_completed_run()`, check if the episode is "publishable" (high
confidence, novel, gate-passed). If so, encode and submit to chain.

### Gap 4: Episode Clustering Never Runs

`k_medoids` is implemented and tested. But orchestrate.rs never calls it. Without
clustering:
- No cluster-level features for CascadeRouter
- No curriculum learning (can't identify weak spots)
- No medoid-based compression for chain publishing

**Fix**: Run clustering every N episodes (configurable). Persist clusters. Use cluster
membership as a CascadeRouter context feature.

### Gap 5: Reputation Doesn't Inform Auction

The reputation registry tracks 7-domain scores with EMA + decay. But when knowledge
entries come back from a chain query, they're all weighted equally. High-reputation
agents' knowledge should bid higher in the VCG auction.

**Fix**: When packaging chain-retrieved knowledge into prompt sections, set
`bid_value = base_relevance × submitter_reputation[domain]`. The VCG auction
naturally handles the rest — higher-reputation knowledge wins more often.

### Gap 6: No HDC Precompile in Mirage

The precompile is spec'd (docs/08-chain/03-hdc-on-chain-precompile.md) with 4 operations
and gas costs. But Mirage doesn't implement it yet. Without the precompile, there's no
way to run HDC similarity queries on-chain.

**Fix**: Implement the 4 precompile operations in Mirage's revm instance:
- `hdc_similarity(a, b)` → XOR + popcount → ~50 gas
- `hdc_topk(query, k)` → iterate stored vectors → ~400 gas for k=20
- `hdc_bind(a, b)` → XOR → ~30 gas
- `hdc_bundle(vectors)` → majority vote → ~30 + 5N gas

### Gap 7: PP-HDC Not Implemented

Vectors shared on-chain are currently raw. XOR binding is invertible — anyone can
unbind and recover the original. For privacy-preserving sharing, PP-HDC hash-encoding
is needed (<1% accuracy loss per IEEE 2024 paper).

**Fix**: Implement distance-preserving non-invertible projection in roko-primitives.
All vectors get hash-encoded before chain submission.

## How To Make It Measurably Better

The question isn't just "does it work" — it's "does querying Korai produce measurably
better agent outcomes than not querying?"

### Measurement Framework

The section effectiveness registry already tracks this! If Neuro-sourced sections
are included in prompts and those prompts lead to gate passes, the lift is measurable:

```
lift = pass_rate(neuro_included) - pass_rate(neuro_excluded)
```

With the VCG auction, this happens naturally:
1. Some prompts include Neuro knowledge (because the auction selected it)
2. Some prompts exclude it (because other bidders won)
3. The effectiveness registry computes the lift
4. If lift > 0.05, the bidder's Thompson posterior shifts up
5. If lift < -0.02, it shifts down
6. Over time, the system learns whether chain knowledge actually helps

### The Arena As Measurement Instrument

SWE-bench (and other arenas) provide controlled evaluation:

```
Batch A: 50 SWE-bench instances, Neuro bidder disabled (no chain knowledge)
Batch B: 50 SWE-bench instances, Neuro bidder enabled (chain knowledge injected)

Compare: pass rates, cost, latency
```

The arena framework makes this a fair A/B test because:
- Same instances
- Same models (CascadeRouter with same weights)
- Only difference: whether Neuro knowledge is in the prompt

If chain knowledge improves SWE-bench pass rate by even 2-3 percentage points,
that's a measurable signal. The section effectiveness registry captures it
automatically.

### The Compound Effect

Over multiple arena batches:
1. Batch 1: Chain is empty. Neuro bidder has nothing. Baseline score.
2. Batch 2: Successful episodes from Batch 1 published to chain. Neuro bidder
   now has a few entries. Slight improvement.
3. Batch N: Chain has hundreds of entries from prior batches. Neuro bidder
   consistently wins VCG slots. Score plateaus at a higher level.

The score-over-time curve IS the measurement. If it's flat, the chain isn't
helping. If it rises, the chain is providing genuine value.

## The Full Narrative

**Korai is a persistent, reputation-gated, HDC-indexed knowledge commons for agents.**

Without Korai: each agent starts from scratch. Knowledge dies with the process.
Learning is local. Playbooks help but only within one instance.

With Korai: every agent's experience compounds into a shared knowledge base.
An agent solving a Django issue on SWE-bench publishes what it learned. The next
agent working on a similar issue queries Korai and gets that insight — adjusted
for the submitter's reputation — injected into its prompt via the VCG auction.

The HDC encoding makes this efficient (1,280 bytes per entry, sub-ms similarity),
the reputation system makes it trustworthy (bad knowledge from low-rep agents is
downweighted), the VCG auction makes it fair (knowledge competes for budget on
merit, not seniority), and the demurrage makes it fresh (stale knowledge decays).

The arena framework provides the measurement instrument: does chain knowledge
actually improve agent performance? Section effectiveness tracking gives the answer
per-section, per-role, per-arena.

The HuggingFace integration adds the outer loop: successful episodes become
fine-tuning data, fine-tuned models become new CascadeRouter arms, and the whole
thing compounds across model quality AND knowledge quality simultaneously.

**One sentence**: Korai turns roko from a system that learns alone into a system
that learns collectively, and the VCG auction ensures that collectively-learned
knowledge is provably better than not having it.
