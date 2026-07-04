# Knowledge Publishing: When, How, and What NOT To Share

## The Problem

Agents learn valuable things. Those things need to reach the chain so other agents
benefit. But blindly publishing everything leaks:

- **Project secrets** — API keys, internal URLs, client names, file paths
- **Proprietary patterns** — competitive advantages, unique workflows
- **Alpha** — trading strategies, MEV opportunities, predictive signals
- **PII** — names, emails, identifiers from processed data
- **Low-quality noise** — failed experiments, hallucinated insights, unverified guesses

## What Exists Today

| Layer | Status | What it does |
|-------|--------|-------------|
| ResultFilter + scrub.rs | **Wired** | Strips API keys, passwords, auth tokens from tool outputs |
| CustodyTaint | **Wired** | Tags actions with provenance (UserInput, ExternalFetch, etc.) |
| Signal-level Taint | **Built** | Clean/UserInput/UnverifiedSource on Engrams |
| Safety contracts | **Wired** | Role-scoped invariants (NoNetworkAccess, MaxCost, etc.) |
| Plugin tiers (5-level) | **Built** | Capability-based tool access control |
| Distiller confidence | **Built** | Knowledge entries have confidence 0-1, GC at <0.05 |
| Anti-knowledge | **Built** | Conflicting entries detected via HDC similarity |
| Witness layer | **Built** | Attestation fingerprints on-chain (hashes, not payloads) |

### What Does NOT Exist

- No **publish gate** — nothing decides "is this knowledge safe/good enough to share?"
- No **content classification** for knowledge entries (public vs proprietary vs secret)
- No **alpha protection** — trading strategies treated same as coding patterns
- No **IFC enforcement** — taint doesn't propagate to block publishing
- No **abstraction pipeline** — raw episodes shared as-is, not generalized
- No **temporal embargo** — no delay for strategically sensitive knowledge
- No **selective sharing** — all-or-nothing, no quality-based filtering

## The Seven-Layer Defense

Based on current research (2024-2026), the architecture for safe knowledge publishing:

```
Raw Episode
  │
  ▼
┌─────────────────────────────────────────────────────┐
│ Layer 1: Content Classification                      │
│   Presidio-style NER + custom recognizers            │
│   Detect: PII, API keys, project names, file paths,  │
│   internal URLs, client identifiers                  │
│   Action: auto-redact or block                       │
├─────────────────────────────────────────────────────┤
│ Layer 2: Knowledge Distillation (Abstraction)        │
│   Transform specific → general:                      │
│   "Django ORM on project acme-corp" →                │
│   "ORM migrations may deadlock under concurrency"    │
│   Uses DistilDP pattern (ACL 2024) for generation    │
├─────────────────────────────────────────────────────┤
│ Layer 3: IFC Labels (Fides-style)                    │
│   Tag with: { confidentiality, integrity, type }     │
│   Enforce declassification rules at share boundary   │
│   PROJECT_SPECIFIC cannot exit without abstraction   │
├─────────────────────────────────────────────────────┤
│ Layer 4: Quality Gate                                │
│   Share only if:                                     │
│     confidence > 0.75 (distiller threshold)          │
│     gate_passed = true (verified by execution)       │
│     tier >= Working (not Transient)                  │
│     no_unresolved_conflicts (anti-knowledge clear)   │
├─────────────────────────────────────────────────────┤
│ Layer 5: Alpha Protection (temporal embargo)         │
│   Strategically sensitive → delay by N periods       │
│   Trading alpha: share after value has decayed       │
│   Classification: LLM judge + domain heuristics      │
├─────────────────────────────────────────────────────┤
│ Layer 6: PP-HDC Encoding                             │
│   Distance-preserving non-invertible projection      │
│   Shared vectors cannot reconstruct original         │
│   <1% accuracy loss (PP-HDC, IEEE 2024)              │
├─────────────────────────────────────────────────────┤
│ Layer 7: Selective Sharing                           │
│   Not everything should be shared (Selective-FD)     │
│   Low-confidence → keep local until promoted         │
│   Model-specific → keep local (not generalizable)    │
│   Only share knowledge that helps without harming    │
└─────────────────────────────────────────────────────┘
  │
  ▼
Chain Submission (PP-HDC encoded, quality-gated,
  abstracted, IFC-labeled, embargo-checked)
```

## Layer Details

### Layer 1: Content Classification

**What to detect** (custom recognizers for agent outputs):

| Pattern | Examples | Action |
|---------|----------|--------|
| API keys / secrets | `sk-...`, `ghp_...`, `Bearer ...` | Block (never share) |
| File paths | `/Users/will/dev/...`, `C:\Users\...` | Redact to relative |
| Project/client names | `acme-corp`, `project-phoenix` | Strip entirely |
| Internal URLs | `*.internal.company.com` | Strip |
| Database identifiers | specific table names, column names | Generalize |
| Git hashes / commit refs | `abc123def` | Strip |
| IP addresses / ports | `192.168.1.x:5432` | Strip |
| Model-specific artifacts | `claude-opus-4-6` response fragments | Strip |

**Implementation**: Adapt the existing `ScrubPolicy` in `roko-agent/src/safety/scrub.rs`.
The scrubber already strips secrets from tool outputs — extend it to run on knowledge
entries before chain submission.

Estimated: ~200 lines (extend existing scrubber with knowledge-specific patterns)

### Layer 2: Knowledge Distillation (Abstraction Ladder)

Transform specific episodes into general principles:

| Level | Example | When to share |
|-------|---------|---------------|
| L0 (raw) | "On 2024-03-15, project acme-corp Django migration 0047 failed with deadlock on PostgreSQL 14.2" | Never |
| L1 (redacted) | "Django migration failed with deadlock on PostgreSQL" | Within organization |
| L2 (generalized) | "ORM migrations may deadlock under concurrency; use exponential backoff + SKIP LOCKED" | Default share level |
| L3 (category) | "I have knowledge about ORM migration failures" | Cross-organization (with ZK proof) |

**The abstraction pipeline**:

```rust
struct AbstractionPipeline {
    /// Step 1: Content classifier (Layer 1)
    classifier: ContentClassifier,
    /// Step 2: Generalizer (LLM-based, cheap model)
    generalizer: KnowledgeGeneralizer,
    /// Step 3: Validator (does generalized version preserve the insight?)
    validator: AbstractionValidator,
}

impl AbstractionPipeline {
    async fn abstract_to_level(&self, entry: &KnowledgeEntry, level: AbstractionLevel) -> Result<KnowledgeEntry> {
        // 1. Classify sensitive content
        let classified = self.classifier.classify(&entry.content)?;

        // 2. Generalize (strip specifics, preserve pattern)
        let generalized = self.generalizer.generalize(entry, level, &classified).await?;

        // 3. Validate (does the generalized version still capture the insight?)
        let valid = self.validator.validate(&generalized, entry)?;
        if !valid { return Err(AbstractionError::LostMeaning); }

        Ok(generalized)
    }
}
```

The generalizer uses a cheap LLM (Haiku-tier) with a structured prompt:
"Given this specific insight, produce a generalized version that preserves the
engineering pattern but removes all project-specific details, names, dates,
and identifiers."

**Key insight from Selective-FD (Nature Communications 2024)**: Not all knowledge
should be shared. Client-side selectors identify which knowledge is accurate enough,
and server-side selectors filter what's precise enough to propagate. Sharing everything
degrades collective quality.

Estimated: ~400 lines (generalizer + validator + integration)

### Layer 3: IFC Labels (Fides-style)

Based on Microsoft Fides (arXiv:2505.23643, May 2025) — the first practical IFC
system for AI agents.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
struct KnowledgeLabel {
    /// Confidentiality level
    confidentiality: ConfidentialityLevel,
    /// Data integrity (how verified is this?)
    integrity: IntegrityLevel,
    /// Content type (for selective declassification)
    content_type: ContentType,
    /// Source taint chain (where did this come from?)
    taint_chain: Vec<TaintSource>,
}

enum ConfidentialityLevel {
    Public,           // Safe to share globally
    Organization,     // Share within org boundary
    Project,          // Share within project boundary
    Agent,            // Never share (agent-private)
}

enum IntegrityLevel {
    Verified,         // Gate-passed, multi-episode confirmed
    Derived,          // Abstracted from verified knowledge
    Unverified,       // Single observation, not confirmed
    External,         // From external/tainted source
}
```

**Enforcement rule**: Knowledge with `confidentiality > target_boundary` MUST pass
through the abstraction pipeline (Layer 2) before sharing. This is enforced at the
publish boundary, not the storage boundary.

**Taint propagation**: If knowledge was derived from `ExternalFetch` or `UserInput`
tainted data, it inherits the taint. Tainted knowledge requires explicit
declassification before sharing.

Estimated: ~300 lines (label types + propagation + enforcement)

### Layer 4: Quality Gate

```rust
struct PublishGate {
    min_confidence: f64,        // default 0.75
    min_tier: KnowledgeTier,    // default Working (not Transient)
    require_gate_pass: bool,    // default true
    max_model_specificity: f64, // default 0.3 (low = generalizable)
    require_multi_episode: bool, // default true for heuristics
}

impl PublishGate {
    fn should_publish(&self, entry: &KnowledgeEntry) -> PublishDecision {
        if entry.confidence < self.min_confidence { return Reject("low confidence"); }
        if entry.tier < self.min_tier { return Reject("not yet consolidated"); }
        if self.require_gate_pass && !entry.gate_verified { return Reject("unverified"); }
        if entry.model_generality < self.max_model_specificity { return Reject("model-specific"); }
        if entry.has_unresolved_conflicts() { return Reject("conflicting anti-knowledge"); }
        Approve
    }
}
```

This connects to the existing distiller infrastructure — confidence scores, tier
progression, anti-knowledge conflict detection are all already computed. The publish
gate just checks them before chain submission.

Estimated: ~100 lines

### Layer 5: Alpha Protection (Temporal Embargo)

For trading/blockchain agents, knowledge about successful strategies is valuable
precisely because others don't know it. Publishing immediately destroys the advantage.

```rust
struct AlphaProtection {
    /// Domain-specific embargo periods
    embargo_periods: HashMap<KnowledgeDomain, Duration>,
    /// LLM-based strategic sensitivity scorer
    sensitivity_scorer: Option<Box<dyn SensitivityScorer>>,
}

impl AlphaProtection {
    fn embargo_for(&self, entry: &KnowledgeEntry) -> Option<Duration> {
        // 1. Check if domain has an embargo
        if let Some(period) = self.embargo_periods.get(&entry.domain) {
            return Some(*period);
        }

        // 2. LLM judge for strategic sensitivity
        if let Some(scorer) = &self.sensitivity_scorer {
            let score = scorer.score(entry);
            if score > 0.7 { return Some(Duration::hours(24)); }
            if score > 0.4 { return Some(Duration::hours(4)); }
        }

        None // no embargo needed
    }
}
```

Default embargo periods:

| Domain | Embargo | Rationale |
|--------|---------|-----------|
| Trading strategies | 24h | Alpha decays within hours-days |
| MEV patterns | 1h | MEV opportunities are ephemeral |
| Security vulnerabilities | 72h | Responsible disclosure window |
| Code patterns | 0 | No competitive advantage |
| Research insights | 0 | Value increases with sharing |

After embargo expires, knowledge enters the normal publish pipeline.

Estimated: ~200 lines

### Layer 6: PP-HDC Encoding

Before chain submission, vectors get distance-preserving non-invertible projection
(PP-HDC, IEEE 2024). This ensures:

- Similarity queries still work on the encoded vectors (<1% accuracy loss)
- Original knowledge cannot be reconstructed from the encoded vector
- Even if someone captures the vector, they can't reverse the encoding

```rust
struct PpHdcEncoder {
    /// Secret projection matrix (never shared)
    projection: ProjectionMatrix,
}

impl PpHdcEncoder {
    fn encode(&self, vector: &HdcVector) -> HdcVector {
        // Distance-preserving hash-encoding
        // Uses random projection + threshold (Johnson-Lindenstrauss)
        self.projection.apply(vector)
    }

    fn similarity_preserved(&self, a: &HdcVector, b: &HdcVector) -> bool {
        let original_sim = a.similarity(b);
        let encoded_sim = self.encode(a).similarity(&self.encode(b));
        (original_sim - encoded_sim).abs() < 0.01 // <1% accuracy loss
    }
}
```

Estimated: ~300 lines

### Layer 7: Selective Sharing

The final filter: even after all previous layers, only share knowledge that is
likely to help others without degrading collective quality.

```rust
struct SelectiveSharing {
    /// Minimum value to the collective (estimated via HDC novelty)
    min_novelty_threshold: f64,  // default 0.3
    /// Maximum redundancy with existing chain knowledge
    max_redundancy: f64,         // default 0.85 (don't share duplicates)
}

impl SelectiveSharing {
    async fn should_share(&self, entry: &KnowledgeEntry, chain: &ChainClient) -> bool {
        // 1. Is this novel enough? (not already on chain)
        let existing = chain.hdc_topk(&entry.fingerprint, 5).await;
        if existing.iter().any(|(_, sim)| *sim > self.max_redundancy) {
            return false; // too similar to existing knowledge
        }

        // 2. Is it novel enough to be useful?
        let max_sim = existing.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);
        if 1.0 - max_sim < self.min_novelty_threshold {
            return false; // marginal novelty
        }

        true
    }
}
```

Estimated: ~150 lines

## When Agents Publish

Knowledge publication is NOT continuous. It happens at specific lifecycle moments:

### Trigger 1: After Successful Task Completion

```
Task completes → gates pass → episode recorded
  → distiller extracts knowledge candidates (every N episodes)
  → candidates enter publish pipeline (7 layers)
  → survivors submitted to chain
```

This is the primary publication path. The distiller already runs periodically
and produces `KnowledgeEntry` candidates with confidence scores.

### Trigger 2: After Dream Cycle (Consolidation)

```
Dream cycle completes → knowledge tier promotions
  → newly Consolidated/Persistent entries eligible
  → publish pipeline (7 layers)
  → survivors submitted to chain
```

Dream cycles promote knowledge from Transient → Working → Consolidated → Persistent.
Tier promotion is a natural trigger for sharing — knowledge that survives multiple
validation cycles is more likely to be useful and less likely to be noise.

### Trigger 3: After Cross-Domain Resonance Detection

```
Resonance detected between arenas → novel structural analogy
  → if both source entries are Consolidated+ and gate-verified
  → the resonance pattern itself is a publishable insight
  → publish pipeline (7 layers)
```

Cross-domain analogies are high-value knowledge: "this pattern in domain A is
structurally similar to that pattern in domain B." These are inherently abstract
(the HDC similarity is structural, not content-based) and thus naturally private.

### Trigger 4: Manual Operator Approval

For high-sensitivity domains (trading, security), require explicit operator
approval before any chain submission:

```toml
[knowledge.publishing]
auto_publish = false              # require manual approval
approval_queue = ".roko/publish-queue/"
notify_operator = true
```

## The Full Pipeline In Roko Terms

```rust
/// Called after distiller produces candidates
async fn publish_knowledge(
    &self,
    candidates: Vec<KnowledgeEntry>,
    chain: &dyn ChainClient,
) -> Result<Vec<PublishedEntry>> {
    let mut published = Vec::new();

    for entry in candidates {
        // Layer 1: Content classification
        let classified = self.content_classifier.classify(&entry)?;
        if classified.has_blocked_content() { continue; }

        // Layer 2: Abstraction
        let abstracted = self.abstraction_pipeline
            .abstract_to_level(&entry, AbstractionLevel::L2)
            .await?;

        // Layer 3: IFC label check
        let label = self.ifc.label_for(&abstracted);
        if label.confidentiality > ConfidentialityLevel::Public {
            // Needs further abstraction or skip
            continue;
        }

        // Layer 4: Quality gate
        if !self.publish_gate.should_publish(&abstracted).is_approve() {
            continue;
        }

        // Layer 5: Alpha protection
        if let Some(embargo) = self.alpha_protection.embargo_for(&abstracted) {
            self.embargo_queue.enqueue(&abstracted, embargo);
            continue;
        }

        // Layer 6: PP-HDC encoding
        let encoded_vector = self.pp_hdc.encode(&abstracted.fingerprint);

        // Layer 7: Selective sharing (check chain for novelty)
        if !self.selective.should_share(&abstracted, chain).await {
            continue;
        }

        // Submit to chain
        let tx = chain.submit_knowledge(
            &encoded_vector,
            &abstracted.metadata(), // abstracted metadata only
            self.agent_id,
            self.reputation_stake,
        ).await?;

        published.push(PublishedEntry { entry: abstracted, tx_hash: tx });
    }

    Ok(published)
}
```

## Estimated Total Implementation

| Component | Lines | Depends on |
|-----------|-------|-----------|
| Content classifier (extend scrub.rs) | ~200 | Existing scrubber |
| Abstraction pipeline (generalizer + validator) | ~400 | Cheap LLM (Haiku) |
| IFC labels + enforcement | ~300 | Core types |
| Publish quality gate | ~100 | Existing distiller |
| Alpha protection + embargo queue | ~200 | Domain config |
| PP-HDC encoding | ~300 | roko-primitives |
| Selective sharing (novelty check) | ~150 | Chain RPC |
| Publish orchestration (glue) | ~200 | All of above |
| **Total** | **~1,850** | |

## Key Research References

- **Microsoft Fides** (arXiv:2505.23643, May 2025) — IFC for AI agents, stopped 100% of prompt injection attacks
- **DistilDP** (ACL 2024) — Knowledge distillation with differential privacy via synthetic generation
- **Selective-FD** (Nature Communications 2024) — Not all knowledge should be shared; selective sharing improves both privacy AND quality
- **PP-HDC** (IEEE 2024) — Privacy-preserving HDC with <1% accuracy loss
- **PETS 2024** — Information leakage in trading, leakage budgets per time window
- **Presidio** (Microsoft, MIT) — Production-ready PII detection, 40+ entity types
- **DP in Generative AI Agents** (arXiv:2603.17902, March 2026) — Token-level and message-level DP for agent outputs
