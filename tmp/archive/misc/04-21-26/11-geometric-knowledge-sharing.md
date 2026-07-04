# Geometric Knowledge Sharing

*A zero-LLM privacy architecture for collective agent intelligence.*

---

## The Problem

You have a fleet of AI agents. They solve tasks — fix bugs, monitor blockchains,
audit code, execute trades. Every task teaches an agent something. But each agent's
knowledge dies when its process ends, and sharing knowledge risks leaking secrets,
competitive advantages, or private data.

Existing approaches all have the same flaw: they share *text*.

- **RAG** shares text chunks in a vector database. Text is readable. Secrets leak.
- **Federated learning** shares model gradients. Gradients can be inverted to
  reconstruct training data (Geminio, ICCV 2025).
- **Fine-tuning** shares the model itself. The model memorizes training examples.
  Extractable via prompting.

Every approach that moves natural language across trust boundaries is fundamentally
unsafe. You can add noise (differential privacy), but the noise/utility tradeoff
on small knowledge sets (dozens to hundreds of items) is brutal — epsilon values
that provide meaningful privacy destroy the knowledge.

## The Insight

**Don't share knowledge as text. Share it as geometry.**

A 10,240-bit binary vector (1,280 bytes) can encode the *structural pattern*
of an insight without encoding its *specific content*. Two agents working in
completely different domains — one fixing Django bugs, another auditing Solidity
contracts — can discover that their experiences share structural similarity,
without either agent seeing the other's work.

This is possible because of three algebraic properties of hyperdimensional
computing (HDC):

1. **Binding (XOR)** composes a role and a filler into a single vector.
   `bind(LANGUAGE, rust)` encodes "the language is Rust" as a fixed-width vector.
   Crucially, binding is *involutory*: `bind(bind(A, B), B) = A`. You can
   unbind a role to recover the filler — or unbind the filler to erase it.

2. **Bundling (majority vote)** superposes multiple bound pairs into one vector.
   The result is similar to all inputs. A knowledge vector is a bundle of
   role-filler bindings: pattern + domain + outcome + strategy.

3. **Similarity (Hamming distance via XOR + popcount)** takes <1 microsecond
   between any two vectors. Two vectors with similarity >0.526 (3σ above
   random for 10,240-bit space) share genuine structural similarity.

The key operation: **unbinding erases specifics algebraically.**

```
Knowledge vector K =
    bind(PATTERN, "retry_with_backoff")
  ⊕ bind(PROJECT, "acme-corp")
  ⊕ bind(LANGUAGE, "python")
  ⊕ bind(ERROR_TYPE, "orm_deadlock")

Erase project:
    K_clean = K ⊕ bind(PROJECT, "acme-corp")
            = bind(PATTERN, "retry_with_backoff")
              ⊕ bind(LANGUAGE, "python")
              ⊕ bind(ERROR_TYPE, "orm_deadlock")
```

The project-specific information is gone. Not obfuscated, not noised — *removed*,
via an exact algebraic operation that takes microseconds. What remains is the
structural pattern: retry-with-backoff for ORM deadlocks in Python.

This is not statistical privacy. It's structural. The algebra guarantees it.

## The Architecture

```
Agent learns something
  │
  ├── 1. Encode as HDC role-filler vector          (~5 μs, $0)
  │      pattern + domain + outcome + strategy
  │      + project + client + date + ...
  │
  ├── 2. Scrub: strip secret patterns              (~10 μs, $0)
  │      regex for API keys, file paths, etc.
  │      on the TEXT metadata, not the vector
  │
  ├── 3. Unbind: erase specific roles              (~1 μs per role, $0)
  │      PROJECT, CLIENT, DATE, FILE_PATH
  │      algebraic removal, not obfuscation
  │
  ├── 4. Quality gate: is this worth sharing?       (~1 μs, $0)
  │      confidence > 0.75
  │      gate-verified (execution proved it works)
  │      tier >= Working (survived multiple validations)
  │      not redundant with existing chain knowledge
  │
  ├── 5. Project: PP-HDC non-invertible encoding    (~50 μs, $0)
  │      distance-preserving projection
  │      similarity queries still work (<1% loss)
  │      but the vector cannot be reversed
  │
  ├── 6. Embargo check: is this time-sensitive?     (~1 ns, $0)
  │      trading alpha → delay 24h
  │      security finding → delay 72h
  │      code pattern → no delay
  │
  └── 7. Submit to chain                           (~100 ms, gas only)
         PP-HDC vector (1,280 bytes)
         + minimal metadata:
           domain (coding/chain/security/research)
           knowledge type (insight/heuristic/warning)
           confidence score
           submitter reputation score
           timestamp
         NO TEXT. EVER.
```

**Total cost per knowledge entry: $0 in LLM calls. ~5ms compute. A few cents gas.**

Compare to approaches that use LLMs for abstraction/summarization: $0.001-$0.01
per entry, 1-5 seconds latency, and the LLM itself might leak information in its
reformulation.

## What Goes On-Chain vs. Off-Chain

### On-chain (Korai / Mirage)

```
┌─────────────────────────────────────────────────────┐
│ Knowledge Entry On-Chain                             │
│                                                       │
│   vector: [u8; 1280]        // PP-HDC encoded        │
│   domain: u8                // coding/chain/sec/...  │
│   kind: u8                  // insight/heuristic/... │
│   confidence: u16           // 0-10000 (fixed-point) │
│   submitter: Address        // Korai Passport holder │
│   reputation: u16           // submitter's score     │
│   timestamp: u64            // block timestamp       │
│   content_hash: [u8; 32]    // blake3 of full entry  │
│                                                       │
│   Total: ~1,340 bytes per entry                      │
└─────────────────────────────────────────────────────┘
```

No text. No natural language. No project names, file paths, code snippets,
or strategy descriptions ever touch the chain.

The `content_hash` is a commitment: the submitter can later prove they hold
the original knowledge without revealing it (useful for disputes, reputation
challenges, or knowledge marketplace negotiations).

### Off-chain (local only)

The full knowledge entry with text, source episodes, confidence history,
and tier progression lives in the agent's local neuro store
(`.roko/neuro/knowledge.jsonl`). It never leaves the machine unless the
agent explicitly opts in to a private sharing arrangement.

## How Retrieval Works

When an agent needs knowledge for a task:

```
1. Encode current task as HDC vector             (~5 μs)
   same role-filler structure: pattern + domain + error type + ...

2. Query chain: hdc_topk(task_vector, k=20)      (~1-5 ms local, ~100 ms RPC)
   returns 20 nearest vectors by Hamming similarity
   weighted by submitter reputation
   filtered by domain relevance

3. For each returned vector:
   similarity > 0.526? → genuine structural match
   reputation > threshold? → trustworthy source
   confidence > 0.5? → reasonably certain

4. Package as prompt context:
   "A structurally similar problem in [domain] was solved with
    [knowledge_type] pattern. Confidence: [score]. Reputation: [score]."

5. VCG auction decides if this context wins budget
   competes against local playbooks, code intelligence, etc.
   reserved 15% budget for chain knowledge
```

**The agent doesn't receive text from the chain.** It receives a similarity
signal: "something structurally similar to your current problem was solved
successfully by a reputable agent." The agent's own reasoning fills in the
specifics.

This is the opposite of RAG. RAG says "here is the exact text that's
relevant." Geometric retrieval says "something shaped like your problem
was solved — figure out the analogy." This is both more private (no text
shared) and often more useful (forces the agent to reason about the
structural pattern rather than copy-paste).

## The Five Defenses

### Defense 1: Algebraic Erasure (no LLM, microseconds)

The role-filler binding in HDC is algebraically exact. Unbinding a role
from a composite vector removes that role's information. This is not
statistical — it's structural.

Roles to always unbind before sharing:
- `PROJECT` — project/client/org identifier
- `FILE_PATH` — absolute or relative file paths
- `AUTHOR` — who wrote the code being discussed
- `TIMESTAMP_SPECIFIC` — exact dates (keep relative timing)
- `CREDENTIAL` — anything that looks like a secret

The codebook allocates deterministic vectors for these roles. Unbinding
is one XOR per role (~1μs per 10,240-bit vector).

After unbinding, the composite vector retains:
- `PATTERN` — the structural pattern (retry logic, error handling, etc.)
- `DOMAIN` — what area of knowledge (coding, chain, security)
- `ERROR_TYPE` — what kind of problem
- `STRATEGY` — what kind of solution
- `OUTCOME` — success/failure

These are the general features that make knowledge transferable across
projects. The specific features (which project, which file, which date)
are gone.

### Defense 2: Deterministic Scrubbing ($0, microseconds)

Before HDC encoding, the text metadata gets scrubbed using pattern matching.
This catches things that don't map cleanly to HDC roles:

| Pattern | Detection | Action |
|---------|-----------|--------|
| API keys | `sk-`, `ghp_`, `Bearer`, high-entropy strings | Strip |
| File paths | `/Users/`, `C:\`, `./src/` | Strip |
| URLs | `*.internal.*`, `localhost:*` | Strip |
| IP addresses | IPv4/IPv6 patterns | Strip |
| Email addresses | `*@*.*` | Strip |
| Git hashes | 40-char hex strings | Strip |

This reuses the existing `ScrubPolicy` in `roko-agent/src/safety/scrub.rs`,
which is already wired for tool output sanitization. Extend it to run on
knowledge entry metadata before encoding.

### Defense 3: PP-HDC Projection ($0, ~50 microseconds)

Even after unbinding specific roles, the remaining vector might theoretically
be partially inverted by an adversary who knows the codebook. PP-HDC
(Privacy-Preserving HDC, IEEE 2024) adds a final non-invertible projection:

```
encoded = secret_projection_matrix × clean_vector
```

Properties:
- **Distance-preserving**: similarity between encoded vectors matches
  similarity between originals (<1% accuracy loss)
- **Non-invertible**: cannot reconstruct the clean vector from the encoded
  one without the secret projection matrix
- **Per-instance**: each roko instance has its own projection matrix.
  The chain's HDC precompile works on encoded vectors directly.

The projection matrix is generated once at instance creation and never shared.
It's the "private key" of the knowledge sharing system.

### Defense 4: Temporal Embargo ($0, nanoseconds)

Some knowledge is valuable precisely because others don't have it:

| Domain | Embargo | Why |
|--------|---------|-----|
| Trading strategies | 24 hours | Alpha decays within hours/days |
| MEV patterns | 1 hour | MEV opportunities are ephemeral |
| Security vulnerabilities | 72 hours | Responsible disclosure |
| Code patterns | 0 | No competitive advantage |
| Research insights | 0 | Value increases with sharing |

Embargoed entries sit in a local queue. A background ticker checks
timestamps and promotes entries to the publish pipeline when their
embargo expires. This is a timestamp comparison, not a computation.

### Defense 5: Quality Gate ($0, microseconds)

Not all knowledge is worth sharing. Low-quality knowledge actively
degrades collective intelligence (Selective-FD, Nature Communications 2024).

Entry must pass ALL of:
- `confidence >= 0.75` — distiller threshold
- `tier >= Working` — survived at least one validation cycle
- `gate_verified = true` — the solution actually worked (compilation, tests, etc.)
- `no_unresolved_conflicts` — no contradicting anti-knowledge
- `model_generality >= 0.7` — not specific to one LLM
- `novelty >= 0.3` — not redundant with existing chain knowledge
  (checked via `hdc_topk` against chain, only vectors with similarity <0.85)

All these fields already exist on `KnowledgeEntry` in roko-neuro. The gate
is a pure predicate — no computation, just field comparisons.

## Network Effects and Exponential Scale

### Why This Compounds

1. **More agents → richer vector space → better retrieval.**
   With 100 agents across 10 domains, the chain accumulates diverse patterns.
   An agent encountering a novel problem has a higher probability of finding a
   structurally similar solution from another domain.

2. **Cross-domain resonance is the multiplier.** An insight from blockchain
   monitoring ("retry after RPC timeout with exponential backoff") resonates
   structurally with a coding insight ("retry after database deadlock with
   exponential backoff"). Neither agent shared text — but the HDC similarity
   (>0.526) reveals the analogy. The querying agent's own reasoning completes
   the transfer.

3. **Quality compounds, not noise.** The quality gate ensures only gate-verified,
   high-confidence, multi-episode knowledge reaches the chain. Bad knowledge
   gets filtered locally. Reputation scoring downweights consistently wrong
   submitters. The chain gets better over time, not noisier.

4. **Reputation creates selection pressure.** Agents with higher reputation
   scores produce knowledge that gets higher VCG bids in other agents' prompt
   auctions. This means their knowledge gets used more often, tested more often,
   and (if it works) further boosts their reputation. Positive feedback loop.

5. **Demurrage prevents staleness.** KORAI token has 1% annual decay. Knowledge
   entries are time-weighted. Old patterns that haven't been confirmed recently
   lose weight. The chain is a living knowledge base, not a graveyard.

### Scaling Properties

| Metric | 10 agents | 100 agents | 1000 agents |
|--------|-----------|------------|-------------|
| Chain entries (after 1 month) | ~500 | ~5,000 | ~50,000 |
| Cross-domain resonance pairs | ~25 | ~2,500 | ~250,000 |
| Avg retrieval quality (top-20) | Low | Medium | High |
| Per-query cost | ~600 gas | ~600 gas | ~600 gas |
| Per-query latency | <5ms | <5ms | <10ms (3-tier search) |

The cost is constant regardless of chain size (the HDC precompile's 3-tier
search — bloom filter → approximate → exact — keeps queries O(1)-ish).
The quality improves superlinearly because cross-domain resonance pairs grow
quadratically with the number of domains represented.

### The 3-Tier On-Chain Search (Scales to 1M+ entries)

```
Query: hdc_topk(task_vector, k=20)

Tier 1: Bloom filter (8.7 bits/entry)
  Reject 90% of entries without reading full vectors
  1M entries → ~1.1 MB bloom filter → microseconds

Tier 2: Approximate (downproject to 1,024-bit summaries)
  Scan surviving ~100K entries at 8x compression
  Find top 5K candidates

Tier 3: Exact (full 10,240-bit POPCNT)
  Scan 5K candidates at full resolution
  Return top 20 with exact similarity scores

Total: ~400 gas for k=20, regardless of chain size
```

## The Trading Agent Case

This is the hardest case. A blockchain agent discovers a profitable MEV
pattern. Publishing it immediately destroys the edge. Never publishing it
means other agents can't learn from it.

The solution is a three-part protocol:

### 1. Immediate: publish the vector, embargo the metadata

The PP-HDC encoded vector goes on-chain immediately. But the metadata
(domain, type, confidence) is embargoed for 24 hours. During the embargo,
other agents can see that "something exists in this region of vector space"
but can't query it meaningfully — they don't know what domain it's from
or what type of knowledge it is.

After 24 hours, the metadata is published. By then, the specific MEV
opportunity is long gone. What remains is the structural pattern, which is
still valuable for future pattern recognition.

### 2. Category-level signals only

Trading knowledge is published at abstraction level L3: "I have knowledge
about [MEV_SANDWICH | LIQUIDATION | ARBITRAGE]." The structural pattern is
in the vector. The category is in the metadata. But the specific parameters
(which pool, which token, which price level) are in the roles that were
unbound before sharing.

### 3. Reputation-gated deep access

For agents that want the full insight (not just the structural pattern),
they negotiate via the Spore marketplace:
- Pay KORAI to access the original (non-projected) vector
- Source agent's reputation is staked
- If the knowledge is useless, reputation penalty
- If it's valuable, reputation boost

This creates an economic mechanism: agents with genuine alpha can monetize
it, but only after the immediate edge has expired.

## Cost Summary

| Operation | Latency | $ per entry |
|-----------|---------|-------------|
| Encode as role-filler HDC | ~5 μs | $0 |
| Scrub text metadata | ~10 μs | $0 |
| Unbind specific roles | ~5 μs | $0 |
| Quality gate check | ~1 μs | $0 |
| PP-HDC projection | ~50 μs | $0 |
| Embargo check | ~1 ns | $0 |
| Chain submission | ~100 ms | ~$0.002 gas |
| **Total** | **~100 ms** | **~$0.002** |

Compare to an LLM-based abstraction pipeline:

| Operation | Latency | $ per entry |
|-----------|---------|-------------|
| LLM summarization call | 2-10 s | $0.005-$0.05 |
| Content classification | 1-5 s | $0.002-$0.02 |
| Quality assessment | 1-5 s | $0.002-$0.02 |
| **Total** | **4-20 s** | **$0.009-$0.09** |

The geometric approach is **200-4,000x faster** and **5-45x cheaper**.
At scale (thousands of entries), this is the difference between a system
that publishes continuously and one that publishes in expensive batches.

## What Makes This Novel

1. **No text crosses trust boundaries.** The chain stores only binary vectors
   and scalar metadata. Natural language never leaves the local machine.

2. **Privacy is algebraic, not statistical.** Unbinding removes information
   exactly, not approximately. No epsilon/delta tradeoffs. No noise.

3. **The privacy operation IS the encoding operation.** There's no separate
   "privacy step" — the same HDC algebra that encodes knowledge also strips
   specifics. Privacy is a property of the representation, not an add-on.

4. **Retrieval produces analogy, not recall.** The querying agent receives a
   similarity signal, not text. It must reason about why the match is relevant.
   This forces genuine transfer learning rather than copy-paste.

5. **Cost is zero marginal.** After the fixed cost of HDC encoding (microseconds,
   pure Rust), all privacy guarantees are structural. No per-entry LLM calls.
   No model inference. Just bit operations.

6. **Network effects are quadratic in domains.** Cross-domain resonance pairs
   grow as O(domains²). Ten domains produce 45 cross-domain pairs. Twenty
   domains produce 190. The more diverse the agent fleet, the richer the
   collective intelligence — because structural patterns transfer across domains
   even when the content is completely different.

## Implementation Estimate

| Component | Lines | Depends on |
|-----------|-------|-----------|
| Extended scrubber (text metadata patterns) | ~150 | Existing scrub.rs |
| Role-filler publish encoder (unbind specific roles) | ~200 | Existing neuro/hdc.rs |
| PP-HDC projection (non-invertible encoding) | ~300 | roko-primitives |
| Quality gate predicate | ~80 | Existing KnowledgeEntry fields |
| Embargo queue (background ticker) | ~150 | tokio timer |
| Publish orchestration (chain submission) | ~200 | Chain RPC client |
| HDC precompile in Mirage (4 operations) | ~500 | revm |
| **Total** | **~1,580** | |

Zero new crates. Zero new dependencies. Zero LLM calls.
