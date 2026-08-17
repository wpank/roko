# DA Feasibility Assessment

**Document 11 of the tiagent Celestia Grant Proposal**

**Date**: August 2026

---

## Executive Summary

A natural question about tiagent's Celestia integration is: have you actually run the
math? This document answers that question with real numbers from two systems --- TraceCommons
(an existing trace-sharing platform) and roko (the production codebase from which tiagent's
architecture is derived).

The short answer: **Celestia is not a database, and we do not propose using it as one.**
Celestia is a data availability layer --- it proves that data was published, not that data
will be stored forever. tiagent uses Celestia as an **attestation and sharing layer**, not a
replacement for databases or object stores. This distinction is critical to understanding why
the integration works.

**Key finding:** The artifacts tiagent needs to share (behavioral fingerprints, model routing
weights, playbooks, quality thresholds) are naturally small --- most are under 5 KB. Publishing
them as Celestia blobs costs $12/day for 1,000 agents, or $0.08/day using batched digests.
At scale, tiagent becomes a meaningful DA consumer comparable to a small rollup.

---

## 1. Background: What Is Data Availability?

For readers unfamiliar with the distinction: data availability (DA) is not storage. DA
guarantees three things:

1. **Data was published** --- anyone can verify via data availability sampling (DAS)
2. **Data was available at publication time** --- Celestia's consensus attests to this
3. **The data had specific contents** --- a Merkle commitment in the block header binds the
   data cryptographically

What DA does **not** provide:

- **Permanent storage.** Celestia light nodes prune blob data after 7 days. Only archival
  nodes retain full history indefinitely.
- **Queryability.** There are no indexes, no SQL, no range queries. To find data in a
  namespace, you scan block-by-block.
- **Low cost for large objects.** At ~$0.81/MB (current mainnet), a 1 MB blob costs $0.81.

The critical nuance: while blob data is pruned after 7 days on light nodes, **block headers
are permanent on every node type**. And [Blobstream](https://docs.celestia.org/how-to-guides/blobstream)
posts these block commitments to Ethereum, Arbitrum, and Base via ZK proofs (~1 hour cadence).
So the *proof that data existed with specific contents at a specific time* is permanent ---
even after the data itself is pruned.

| Node type | Blob data retention | Block headers |
|-----------|-------------------|---------------|
| Light node | **7 days** | Permanent |
| Bridge node | **7 days** | Permanent |
| Full node | Configurable | Permanent |
| Archival node | **Permanent** | Permanent |

---

## 2. Case Study: TraceCommons

[TraceCommons](https://tracecommons.org) is a platform where developers share execution traces
from AI coding agents. Contributors submit traces (code snippets, agent tool calls, context,
outcomes) and earn credits; consumers spend credits to access the collective knowledge base.
It is a natural candidate for Celestia integration because it involves multi-party data
sharing with a credit ledger --- exactly the kind of system where independent verifiability
matters.

### Current architecture

TraceCommons uses three storage tiers:

| Tier | Technology | Contents | Size per submission |
|------|-----------|----------|-------------------|
| **Metadata** | PostgreSQL (41 migrations, 30+ tables, row-level security) | Contributor profiles, trace metadata, credit ledger, access control, search indexes | 2--8 KB |
| **Object Store** | Google Cloud Storage or local filesystem, AES-GCM encrypted | Encrypted trace envelopes containing code, context, and annotations | 20--100 KB median (1.5 MB max) |
| **Vector Store** | BGE-large-en-v1.5 embeddings, 1024-dim, HNSW index | Semantic embeddings for similarity search | 4--200 KB |

Current scale: ~352 total submissions, ~13 per week (~2/day), 3 active contributors.

### Should Celestia replace any of these stores?

**No.** Each store exists for reasons Celestia cannot serve:

- **PostgreSQL** provides queries, joins, row-level security, and full-text search. Celestia
  has no query capability.
- **Object Store** provides permanent, encrypted, random-access retrieval. Celestia prunes
  after 7 days.
- **Vector Store** provides k-nearest-neighbor similarity search. Celestia is append-only
  with no search.

### What Celestia *does* provide for TraceCommons

**Provenance attestation.** TraceCommons operates a credit system --- contributors earn
credits for sharing traces, consumers spend credits to access them. Today, the only evidence
that a trace was submitted at a specific time with specific contents is the server operator's
PostgreSQL database. The operator could theoretically:

- Retroactively modify credit balances
- Alter submission timestamps
- Silently remove or modify traces after submission
- Dispute contributor attribution

At 3 contributors, this is a trust-between-friends problem. At 300 contributors --- especially
if enterprises participate --- it requires cryptographic guarantees.

**The integration pattern:** publish a small hash attestation to Celestia alongside each
trace submission. The attestation contains:

```
Trace content hash      32 bytes
Contributor ID hash     32 bytes
Envelope hash           32 bytes
Timestamp                8 bytes
Schema version           2 bytes
Signature               64 bytes
─────────────────────────────────
Total                  170 bytes
```

This 170-byte blob proves that a specific trace was submitted by a specific contributor at a
specific time. Blobstream relays this commitment to Ethereum within ~1 hour, creating a
permanent, independently verifiable record. The full trace data stays in PostgreSQL and GCS
where it belongs.

### Cost analysis

| Scale | Submissions/day | Daily blob size | Daily cost | Annual cost |
|-------|-----------------|-----------------|------------|-------------|
| Current (3 contributors) | 2 | 340 B | $0.0003 | $0.10 |
| 10x growth | 20 | 3.4 KB | $0.003 | $1.00 |
| 100x growth | 200 | 34 KB | $0.03 | $10 |
| 1,000x growth | 2,000 | 340 KB | $0.28 | $100 |

For comparison, storing the full envelopes (50 KB median) on Celestia would cost $30/year at
current scale but $300,000/year at 10,000x --- and you would still need PostgreSQL and a vector
store for queries and search. Hash attestations achieve the same provenance guarantees at a
fraction of the cost.

An even cheaper option --- batching all daily attestation hashes into a single Merkle tree and
publishing only the root --- costs ~$0.03/year regardless of submission volume.

---

## 3. tiagent's Learning Artifacts: Measured from Production

tiagent's architecture is derived from roko, an existing ~800K LOC Rust codebase that already
produces and persists all of the learning artifacts described in the [Product Vision](https://gist.github.com/wpank/fc5147b3ff4325bfc6dcd2c4f7273f7f)
and [Technical Architecture](https://gist.github.com/wpank/fd2d8ead683e8dce31ad76135741700f)
documents. The numbers below are **measured from a real roko workspace** after moderate use
(~19 agent episodes, ~81 agent turns), not estimates.

### What the runtime produces

| Artifact | What it is | Measured size | Per-unit size |
|----------|-----------|---------------|---------------|
| **HDC behavioral fingerprint** | 10,240-bit binary vector computed from each agent episode's (prompt, outcome) pair. Enables O(1) similarity comparison via Hamming distance. | 1,708 bytes per episode | 1.7 KB |
| **Cascade router snapshot** | Learned routing table: which LLM model handles which task type best. Tracks 14 models across 28 task roles with trial counts and success rates. | 68 KB total | ~5 KB per model |
| **Playbooks** | Successful tool-call sequences extracted from high-scoring episodes. Each playbook is a reusable strategy (e.g., "grep before writing new code"). | 36 KB for 9 playbooks | ~4 KB each |
| **Episode records** | Structured metadata per agent session: tokens, cost, timing, gate verdicts, model used, HDC fingerprint. | 158 KB for 19 episodes | ~8 KB each |
| **Efficiency events** | Per-turn cost breakdown: input/output/cache tokens, cost, tools used, duration, gate pass/fail. | 56 KB for 81 turns | ~700 B each |
| **Gate thresholds** | Exponential moving average of pass/fail rates per quality gate (compile, test, lint, diff). | 337 bytes total | Constant |
| **Provider health** | Circuit breaker state per LLM provider: failure rates, cooldown state, consecutive failures. | 3.1 KB | ~800 B per provider |
| **Section outcomes** | Which prompt sections contributed to successful outcomes. Used to rank and prune prompt content. | 888 KB for 1,331 outcomes | ~670 B each |
| **Dream consolidation** | Offline memory consolidation: counterfactual analysis, pattern extraction from episode history. | 606 KB total | Variable |
| **Affect state** | PAD (Pleasure-Arousal-Dominance) vectors and somatic landscape for dispatch modulation. | 22 KB | Constant |
| **Run ledger** | Fine-grained per-action log of every gate invocation, task start/end, cost update. | 5.9 MB for 42,304 actions | ~140 B each |

### What should be shared vs. what stays local

The value of collective learning comes from sharing specific artifacts. Not everything should
be published --- some data is too large, too private, or too agent-specific.

| Artifact | Share via Celestia? | Rationale |
|----------|-------------------|-----------|
| **HDC fingerprints** | **Yes** (full blob, 1.7 KB) | Enables cross-agent similarity search: "find agents that solved similar tasks." This is the mechanism that makes trajectory RAG work. |
| **Cascade router weights** | **Yes** (delta, 2--5 KB) | Collective model routing: "which model handles Rust compilation tasks best across all users?" Publish diffs, not the full 68 KB snapshot. |
| **Playbooks** | **Yes** (full blob, ~4 KB) | Reusable strategies: "successful tool-call sequences that other agents can replay." Deduplicated by content hash. |
| **Gate thresholds** | **Yes** (full blob, 337 B) | Quality calibration: "what compile pass rates are normal across the network?" |
| **Efficiency summaries** | **Yes** (summary, ~1 KB) | Performance benchmarks: "what's slow, what's fast." |
| **Episode records** | **Hash only** (170 B) | Episodes contain prompts and code. Privacy requires hash attestation, not full publication. |
| **Run ledger** | No | Too granular (5.9 MB for one workspace), too agent-specific, no cross-agent value. |
| **Dream state** | No | Agent-specific memory consolidation. No sharing value. |
| **Affect state** | No | Agent-specific emotional model. No sharing value. |

### Why HDC fingerprints are the critical artifact

HDC (Hyperdimensional Computing) fingerprints deserve special attention because they are the
mechanism that makes collective learning concrete rather than theoretical.

Each fingerprint is a 10,240-bit binary vector (1,280 bytes raw, 1,708 bytes base64-encoded)
computed deterministically from an episode's prompt and outcome. Two fingerprints can be
compared with a single XOR + popcount operation (Hamming distance), making similarity search
across millions of episodes computationally trivial --- no embedding model, no vector database,
no GPU required.

This enables **trajectory RAG**: when an agent encounters a new task, it searches the network's
published fingerprints for similar past episodes. If another agent solved a similar task
successfully, the current agent can retrieve that agent's strategy (playbook, model choice,
tool sequence) and use it as in-context learning. The more agents participate, the denser the
fingerprint space becomes, and the more likely any new task has a useful precedent.

At 1.7 KB per fingerprint, these are small enough to publish as full Celestia blobs without
batching. A network of 1,000 agents producing 5 episodes per day generates 8.5 MB/day of
fingerprint data --- well within Celestia's 128 MB block capacity.

---

## 4. Cost Projections for Collective Learning

Three scenarios, from cheapest to most comprehensive:

### Scenario A: Hash-only attestations

Publish a 170-byte attestation per episode (hashes of HDC fingerprint + outcome + playbook IDs).
Cheapest option. Provides provenance but not direct artifact sharing.

| Active agents | Episodes/day/agent | Daily size | Daily cost | Annual cost |
|---------------|-------------------|------------|------------|-------------|
| 10 | 5 | 8.5 KB | $0.007 | $2.50 |
| 100 | 5 | 85 KB | $0.07 | $25 |
| 1,000 | 5 | 850 KB | $0.69 | $250 |
| 10,000 | 5 | 8.5 MB | $6.89 | $2,500 |

### Scenario B: Full shareable artifacts (recommended)

Publish HDC fingerprints (1.7 KB each), daily router deltas (2--5 KB), playbooks on extraction
(4 KB each), gate thresholds (337 B daily). This is the configuration that enables the full
collective learning loop described in documents [02](https://gist.github.com/wpank/fc5147b3ff4325bfc6dcd2c4f7273f7f)
and [05](https://gist.github.com/wpank/7799c1904650b546666996f672fc0fed).

| Active agents | Episodes/day/agent | Daily shared data | Daily cost | Monthly cost | Annual cost |
|---------------|-------------------|-------------------|------------|--------------|-------------|
| 10 | 5 | ~150 KB | $0.12 | $3.60 | $44 |
| 100 | 5 | ~1.5 MB | $1.22 | $36 | $440 |
| 1,000 | 5 | ~15 MB | $12 | $365 | $4,400 |
| 10,000 | 5 | ~150 MB | $122 | $3,645 | $44,000 |

### Scenario C: Batched daily digests

One daily Merkle root per agent covering all episodes + routing updates. Cheapest at scale
because blob count is fixed at one per agent per day.

| Active agents | Daily blobs | Daily size | Daily cost | Annual cost |
|---------------|-------------|------------|------------|-------------|
| 10 | 10 | 1 KB | $0.001 | $0.30 |
| 1,000 | 1,000 | 100 KB | $0.08 | $30 |
| 10,000 | 10,000 | 1 MB | $0.81 | $300 |
| 100,000 | 100,000 | 10 MB | $8.10 | $3,000 |

### Context: DA cost vs. LLM API cost

A developer running tiagent makes ~50 LLM API calls per day at an average of ~$0.03 per call,
spending roughly **$1.50/day on LLM APIs**. Under Scenario B, the DA cost for the same workload
is ~$0.012/day. **DA is less than 1% of the LLM spend.** This ratio holds across workload
sizes because both scale linearly with task count, and DA blobs are compact summaries rather
than full payloads.

---

## 5. What This Means for Celestia

### DA consumption at scale

| Growth stage | Active agents | Daily DA (Scenario B) | Monthly DA | Comparable to |
|-------------|---------------|----------------------|------------|---------------|
| Early (Year 1) | 100 | 1.5 MB | 45 MB | Small testnet rollup |
| Growing (Year 2) | 1,000 | 15 MB | 450 MB | Small production rollup |
| Established | 10,000 | 150 MB | 4.5 GB | Medium rollup |
| At scale | 100,000 | 1.5 GB | 45 GB | Major rollup |

At 10,000 agents, tiagent would be one of the larger DA consumers on the Celestia network ---
generating organic, recurring blob demand from a user base that is not building rollups and
would not otherwise interact with Celestia at all.

### The 7-day pruning question --- and why it's a feature, not a bug

Does blob pruning after 7 days undermine the collective learning use case?

No. And the reason goes deeper than "most artifacts are consumed quickly." tiagent's knowledge
system is **designed around the same economic principle** as Celestia's pruning: knowledge that
isn't actively validated should fade.

#### Knowledge demurrage: how tiagent treats memory

tiagent does not treat knowledge as a permanent archive. It implements **knowledge demurrage**
--- a Gesellian tax on stored information. Every knowledge entry carries a `balance` field
(0.0--5.0) that decays continuously:

```
dB/dt = -r - β × B(t)

Where:
  B(t) = balance at time t
  r    = flat tax per day (constant drain)
  β    = exponential decay rate (proportional drain)
```

Different knowledge types decay at different rates:

| Knowledge kind | Base half-life | Transient tier (0.1×) | Working tier (0.5×) | Consolidated (1.0×) | Persistent (5.0×) |
|---------------|---------------|----------------------|--------------------|--------------------|-------------------|
| **Warning** | 1 hour | 6 minutes | 30 minutes | 1 hour | 5 hours |
| **Strategy fragment** | 14 days | 1.4 days | 7 days | 14 days | 70 days |
| **Insight** | 30 days | 3 days | 15 days | 30 days | 150 days |
| **Causal link** | 60 days | 6 days | 30 days | 60 days | 300 days |
| **Heuristic** | 90 days | 9 days | 45 days | 90 days | 450 days |

Knowledge survives only through **active reinforcement**: being retrieved, cited in a
successful context, passing a quality gate, or explaining a surprising outcome. Each
reinforcement restores balance, but with diminishing returns to prevent hoarding:

```
novelty = 1 / (1 + ln(retrieval_count))
balance_bump = signal_value × novelty
```

When balance falls below 0.05, the entry **freezes** (moves to cold storage, excluded from
queries). When the recency factor drops below 1% of initial weight, the entry **dies** and
becomes eligible for permanent deletion.

#### The alignment with Celestia's 7-day window

This decay model maps directly onto Celestia's pruning behavior:

| tiagent artifact | Effective half-life | 7-day window covers | What happens after pruning |
|-----------------|--------------------|--------------------|--------------------------|
| **Warnings** | 1 hour | 168× the half-life | Irrelevant --- consumed or dead within hours |
| **Strategy fragments** | 1.4--14 days | 0.5--5× the half-life | Transient fragments already dead; consolidated ones are merged locally |
| **Router deltas** | Hours (latest-wins) | Completely sufficient | Only the most recent delta matters |
| **Gate thresholds** | Hours (latest-wins) | Completely sufficient | Superseded by newer thresholds |
| **Playbooks** | Days (merge-on-consume) | Sufficient | Merged into local stores within hours; content-hash dedup provides redundancy |
| **HDC fingerprints** | 3--150 days | Partially covers | See below |

For **5 of 6 artifact types**, the 7-day DA window exceeds the effective lifetime of the data.
Celestia's pruning is not fighting the knowledge model --- it is implementing the same
principle at the infrastructure layer. Data that nobody retrieves within 7 days is data that
has already decayed below relevance in tiagent's own model.

#### The HDC fingerprint exception

HDC fingerprints are the one artifact class with long-term value. A fingerprint from 6 months
ago can still be useful for trajectory RAG ("find agents that solved a similar task"). Three
mechanisms handle this:

1. **Archival nodes** retain all blobs permanently. A tiagent bootstrap node can run an
   archival node (or connect to one) to maintain the full fingerprint history.

2. **Dream consolidation** processes fingerprints within hours, not days. tiagent runs
   periodic offline cycles (inspired by sleep neuroscience) that cluster episodes by HDC
   similarity, extract patterns, and distill knowledge. Once a dream cycle processes a
   fingerprint batch, the distilled knowledge persists locally --- the raw DA blob has served
   its purpose.

3. **Genomic bottleneck** snapshots. tiagent can periodically publish a compressed "genetic
   memory" --- the top-N highest-confidence fingerprints and playbooks --- as a single blob.
   New agents bootstrap from the latest snapshot rather than scanning the full history.

#### The four-tier knowledge lifecycle on DA

The complete lifecycle maps cleanly:

```
[Agent completes task]
        |
        v
Knowledge created at Transient tier (0.1× half-life)
Published as blob to Celestia
        |
   Within hours:
        |
   Other agents consume blob, merge into local stores
   Dream consolidation processes fingerprints
   Router deltas superseded by newer publishes
        |
   Within 7 days:
        |
   Blob pruned from Celestia light nodes ← MATCHES natural decay
   Block header + Blobstream proof remain permanent
   Locally reinforced knowledge promoted: Transient → Working → Consolidated
        |
   Beyond 7 days:
        |
   Knowledge that was reinforced: lives on in local stores at higher tiers
   Knowledge that wasn't reinforced: already dead (balance < 0.05) ← MATCHES pruning
   HDC fingerprints: preserved by archival nodes or consolidated into snapshots
   Provenance proofs: permanent via Blobstream on Ethereum
```

**The insight:** Celestia's 7-day window is not a constraint to work around. It is a
natural expression of the same principle tiagent already implements: **unreinforced knowledge
should not persist.** The DA layer provides a 7-day sharing window. tiagent's demurrage model
ensures that anything worth keeping is consumed, reinforced, and promoted to local durable
storage within that window. Everything else was going to die anyway.

### Integration architecture

```
[tiagent Runtime]
  Completes an agent episode
        |
  Persists locally:
  ├── episodes.jsonl (8 KB)
  ├── cascade-router.json (delta: 2-5 KB)
  ├── playbooks/ (4 KB on extraction)
  └── efficiency.jsonl (700 B/turn)
        |
  Publishes to Celestia:
  ├── HDC fingerprint   → full blob (1.7 KB)  → ns: tiagent/agent/{id}
  ├── Router delta      → full blob (2-5 KB)  → ns: tiagent/learn
  ├── Playbook          → full blob (4 KB)    → ns: tiagent/learn
  ├── Gate thresholds   → full blob (337 B)   → ns: tiagent/learn
  └── Episode hash      → attestation (170 B) → ns: tiagent/trace
        |
  Blobstream → Ethereum (~1hr)
  Permanent cryptographic commitment on ETH/Arbitrum/Base
```

This is a **write-aside** pattern. Celestia is not in the critical path for agent execution.
If Celestia is unavailable, agents operate normally using local stores; blobs queue and publish
when connectivity resumes. The DA layer adds verifiability and sharing, not a runtime
dependency.

---

## 6. Addressing Common Concerns

### "Celestia is temporary storage --- why would you put AI data on it?"

We agree that Celestia is not storage --- and that is precisely why it fits. tiagent's
knowledge model is built on the principle that **knowledge should be temporary unless actively
validated.** Every knowledge entry pays continuous demurrage (a holding tax). Entries that are
not retrieved, cited, or gate-validated within their half-life window naturally decay to zero
and get pruned.

Celestia enforces the same principle at the infrastructure layer: blobs that are not consumed
within 7 days are pruned. This is not a conflict --- it is alignment. The DA layer provides a
sharing window. tiagent's knowledge lifecycle ensures that anything worth keeping is consumed
and promoted to local durable storage within that window. The data that gets pruned from
Celestia is data that tiagent's own demurrage model would have killed anyway.

What survives pruning:
1. **Block headers** (permanent on all node types) --- cryptographic commitment to blob contents
2. **Blobstream proofs** (permanent on Ethereum) --- ZK proofs of Celestia data roots
3. **Locally promoted knowledge** --- entries reinforced during the sharing window live on in
   agents' local stores at Working, Consolidated, or Persistent tiers
4. **Dream-consolidated patterns** --- offline consolidation distills raw blobs into durable
   knowledge within hours, not days

### "What's the value of DA proofs for an AI system?"

DA proofs provide two things traditional infrastructure cannot:

**For TraceCommons:** Trust minimization for a credit-based system. When contributors share
proprietary traces in exchange for credits, the credit ledger needs to be independently
verifiable. DA attestations make the ledger tamper-evident without trusting the server
operator. At 3 contributors this is nice-to-have. At 300 (especially enterprises), it is a
requirement.

**For tiagent's collective learning:** Verifiable artifact provenance. When an agent merges
routing weights or playbooks from the network, it needs confidence those artifacts are genuine
--- actually published by an agent that actually executed those tasks. DA inclusion proofs
provide this. Without them, a malicious actor could inject fabricated learning artifacts to
poison the network's collective intelligence.

### "Can't you just use a regular database or S3?"

For single-agent learning, yes. The local stores work perfectly and Celestia is not required.
tiagent runs entirely standalone with `celestia.enabled = false`.

For **collective** learning across thousands of agents owned by different people, a central
database introduces a trust problem: who operates it? Who pays for it? Who controls access?
Celestia eliminates these questions. Any agent can publish, any agent can read, nobody
controls the data layer, and inclusion proofs make everything verifiable. This is the
architectural reason collective learning requires a DA layer, not just a database.

---

## 7. Summary Table

| Dimension | TraceCommons | tiagent collective learning |
|-----------|-------------|---------------------------|
| **What goes on Celestia** | 170-byte hash attestations per trace submission | HDC fingerprints (1.7 KB), router deltas (2-5 KB), playbooks (4 KB), gate thresholds (337 B) |
| **What stays off Celestia** | Full envelopes, metadata, vectors, search indexes | Run ledgers, dream state, affect state, full episode logs |
| **Cost at current scale** | $0.10/year | $44/year (100 agents) |
| **Cost at 1,000x** | $100/year | $4,400/year (1,000 agents) |
| **DA consumption (1K users)** | 340 KB/day | 15 MB/day |
| **DA consumption (10K users)** | 3.4 MB/day | 150 MB/day |
| **Value of DA proofs** | Tamper-evident credit ledger, contributor attribution, audit compliance | Verifiable artifact provenance, poisoning resistance, decentralized sharing |
| **7-day pruning impact** | None (hash attestations survive via block headers + Blobstream) | Aligned by design: tiagent's knowledge demurrage model kills unreinforced knowledge within the same window. HDC fingerprints preserved by archival nodes + dream consolidation. |
| **Architecture change required** | One 170-byte write-aside per submission | Publisher module alongside existing local stores |
| **Replaces existing storage?** | No | No |

---

*The right question is not "can we store AI data on Celestia?" but "what does a DA layer
provide that traditional infrastructure cannot?" The answer: decentralized sharing with
verifiable provenance and natural lifecycle alignment. tiagent's learning artifacts are
naturally small, naturally append-only, and naturally ephemeral --- designed to decay unless
actively validated, just as Celestia blobs are designed to be pruned unless actively consumed.
This is not a workaround. It is architectural alignment between the knowledge model and the
infrastructure model. Celestia is the substrate that makes collective intelligence trustless
--- at a cost of less than 1% of the LLM spend.*
