# Advanced Dream Concepts: Dream Sharing, Nightmare Detection, and Temporal Validity

> **v3 depth file** -- `/docs/v3/depth/10-dreams/advanced-dream-concepts.md`
> Canonical source: v1 `docs/v1/10-dreams/17-advanced-dream-concepts.md`
> Implementation: `crates/roko-dreams/src/cycle.rs` (nightmare checks),
> `crates/roko-dreams/src/phase2/` (stubs for sharing, journals, advanced constructs)
> Status: **Partial** -- nightmare detection pipeline is specified and basic harm
> checks run during consolidation; dream sharing protocol and temporal validity
> tracking remain target design

---

## 1. Overview

This document covers three extensions to the core dream cycle that become
necessary once multiple agents are operating simultaneously and the dream cycle
matures from a single-agent introspective tool into a multi-agent knowledge
fabric:

1. **Dream Sharing** -- agents sharing dream insights through the mesh
2. **Nightmare Detection** -- safety filtering for dream-generated content
3. **Temporal Validity** -- accounting for non-stationarity in shared insights

These concepts operate as cross-cutting concerns layered on top of the
three-phase cycle foundation.

---

## 2. Dream Sharing: Agents Sharing Dream Insights Through the Mesh

### 2.1 Motivation

A fleet of agents operating on related tasks accumulates complementary dream
insights. Without sharing, each agent independently re-derives threat patterns
and heuristics that other agents have already rehearsed. Dream sharing
propagates insights across the fleet, compressing the time to collective
competence.

The challenge is that dream content is inherently agent-specific: it is
grounded in each agent's episode history, role context, and confidence
calibration. Naive broadcasting of raw hypotheses produces noise.

### 2.2 Theoretical Foundation

**Federated Distillation (FD)**: Agents share soft predictions -- distilled
summaries of what they have learned -- rather than raw model parameters or
hypotheses. This preserves privacy while transmitting the essential knowledge
signal.

**Selective-FD (Nature Communications 2023)**: Only high-confidence,
high-accuracy predictions are shared. Transmitting uncertain hypotheses
degrades collective performance. Dream sharing adopts this criterion: only
hypotheses at Tier 3 confidence (>= 0.75, after consolidation scoring) enter
the sharing pool.

**Stigmergy (Grasse 1959)**: Indirect coordination through shared medium.
Dream insights deposited into the mesh accumulate and decay over time.

Pheromone decay equation:

```
tau(t+1) = (1 - rho) * tau(t) + delta_tau_k
```

Where:
- tau(t) = confidence weight of a shared insight at time t
- rho = evaporation rate (default: 0.05 per dream cycle)
- delta_tau_k = confidence boost when agent k independently corroborates

An insight that is never corroborated decays to zero. An insight corroborated
by multiple agents accumulates weight.

### 2.3 Sharing Modes

| Mode | Trigger | What Is Shared | Privacy |
|------|---------|---------------|---------|
| **Broadcast** | Every dream cycle, unconditionally | All staged hypotheses at Tier 3 | Full mesh visibility |
| **Selective** | Per-hypothesis confidence gate | Hypotheses with confidence >= 0.75 and novelty >= 0.6 | Filtered |
| **Solicited** | Another agent explicitly requests | Insights matching the request topic | Point-to-point |

The default mode is **Selective**.

### 2.4 Confidence Decay on Transit

Shared knowledge travels through the mesh hop-by-hop. Each hop applies a
Weismann barrier: confidence degraded by factor 0.85.

```
confidence_at_recipient = original_confidence * 0.85^(hop_count)
```

A hypothesis with confidence 0.90 that travels through two hops arrives with
confidence 0.90 * 0.85^2 = 0.65. The recipient may promote this to Tier 3
only after independent corroboration.

### 2.5 Privacy Boundaries

1. **Role isolation**: Agents in isolated roles do not share any dream content
   outside their role boundary, regardless of mode.
2. **Episode sanitization**: Before sharing, all episode references are
   replaced with anonymized summaries.
3. **Solicited requests require policy approval**: The sharing Policy trait
   evaluates solicited requests before responding.

### 2.6 Rust Structures

```rust
pub struct DreamShareConfig {
    pub mode: DreamShareMode,
    pub selective_confidence_threshold: f64,   // default: 0.75
    pub selective_novelty_threshold: f64,       // default: 0.60
    pub evaporation_rate: f64,                  // default: 0.05, range: 0.01-0.20
    pub hop_confidence_decay: f64,             // default: 0.85, range: 0.70-0.95
    pub max_hops: usize,                        // default: 3
    pub sanitize_episodes: bool,               // default: true
    pub allowed_recipient_roles: Vec<String>,
}

pub enum DreamShareMode {
    Broadcast,
    Selective,
    Solicited,
    Disabled,
}

pub struct SharedDreamInsight {
    pub insight_id: String,
    pub source_agent_id: String,
    pub source_cycle_id: String,
    pub hypothesis_summary: String,
    pub original_confidence: f64,
    pub current_confidence: f64,
    pub hop_count: usize,
    pub corroborating_agents: Vec<String>,
    pub stigmergy_weight: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

pub struct DreamShareProtocol {
    pub config: DreamShareConfig,
    pub inbound_buffer: Vec<SharedDreamInsight>,
    pub outbound_buffer: Vec<SharedDreamInsight>,
    pub stigmergy_map: std::collections::HashMap<String, f64>,
}

impl DreamShareProtocol {
    /// Apply one evaporation step to all stigmergy weights.
    pub fn evaporate(&mut self) {
        for weight in self.stigmergy_map.values_mut() {
            *weight *= 1.0 - self.config.evaporation_rate;
        }
        self.stigmergy_map.retain(|_, v| *v >= 0.01);
    }

    /// Corroborate a mesh insight from this agent's own waking experience.
    pub fn corroborate(&mut self, insight_id: &str, agent_id: &str, delta_tau: f64) {
        let entry = self.stigmergy_map
            .entry(insight_id.to_string())
            .or_insert(0.0);
        *entry = (1.0 - self.config.evaporation_rate) * *entry + delta_tau;
    }
}
```

---

## 3. Nightmare Detection: When Dreams Produce Harmful Knowledge

### 3.1 Motivation

The REM imagination phase is deliberately unconstrained: it recombines
knowledge across domains, generates counterfactuals, and explores adversarial
scenarios. This creative freedom is the source of the dream cycle's value.
It is also a safety risk. A nightmare is not a failed dream -- it is a dream
that succeeded in generating novel knowledge, but the knowledge is harmful.

### 3.2 Nightmare Taxonomy

| Class | Description | Example |
|-------|-------------|---------|
| **1. Harmful strategy** | Novel attack or exploitation strategy | Privilege escalation path combining two known vulnerabilities |
| **2. Dangerous tool chain** | Legitimate tool sequence producing harmful outcome | File read + network write + process spawn that exfiltrates data |
| **3. Safety bypass** | Way to satisfy a task goal while violating a safety constraint | Plan completing a task while circumventing access control |
| **4. Policy violation** | Content violating role policy | Novel synthesis pathway generated during threat simulation |

### 3.3 Multi-Stage Detection Pipeline

Detection proceeds through four sequential stages. A positive detection at
any stage triggers containment:

```
hypothesis
    |
    v
[Stage 1: Harm classifier]
    |  Binary: harmful / benign
    |  Model: lightweight safety classifier (T2 tier)
    |
    v
[Stage 2: Domain-specific CBRN/security check]
    |  Constrained vocabulary matching + embedding similarity
    |
    v
[Stage 3: Novelty-divergence check]
    |  capability_delta = implied_capability - known_capability
    |  Threshold: capability_delta > 0.5 triggers review
    |
    v
[Stage 4: Human escalation]
    |  Hypothesis with entropy > 0.4 on any stage
    |  is escalated to human review before staging
    |
    v
benign hypothesis -> staging buffer
```

Reference: **Constitutional AI (Anthropic)**: Before the harm classifier, the
agent applies a self-critique prompt ("Does this hypothesis describe how to
do something harmful?"). The CA self-critique is fast (single forward pass)
and catches obvious cases.

Reference: **Constitutional Classifiers (Sharma et al. 2025, arXiv:2501.18837)**:
Against 10,000 jailbreak prompts on Claude 3.5 Sonnet, baseline success rate
= 86%. With Constitutional Classifiers: **4.4%** -- a >95% reduction.

### 3.4 Containment Protocol

When a nightmare is detected:

1. The flagged hypothesis is quarantined -- not written to the staging buffer
2. A `NightmareReport` is written to `.roko/dreams/nightmares.jsonl`
3. The dream cycle's REM phase is terminated early
4. A human review request is generated
5. Adversarial dreaming intensity is reduced for the next 3 cycles

No nightmare content is ever promoted to permanent knowledge without explicit
human approval. The containment is strict and non-bypassable.

### 3.5 Rust Structures

```rust
pub struct NightmareDetector {
    pub classifier_model_tier: ModelTier,       // default: T2
    pub enable_domain_check: bool,              // default: true
    pub capability_delta_threshold: f64,        // default: 0.50, range: 0.20-0.80
    pub escalation_entropy_threshold: f64,      // default: 0.40
    pub nightmare_log_path: std::path::PathBuf,
    pub post_nightmare_cooldown_cycles: usize,  // default: 3
}

pub struct NightmareReport {
    pub nightmare_id: String,
    pub cycle_id: String,
    pub agent_id: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub hypothesis_summary: String,
    pub detection_stage: u8,
    pub nightmare_class: NightmareClass,
    pub classifier_score: f64,
    pub capability_delta: Option<f64>,
    pub escalation_entropy: Option<f64>,
    pub human_reviewed: bool,
    pub human_decision: Option<NightmareDecision>,
}

pub enum NightmareClass {
    HarmfulStrategyGeneration,
    DangerousToolChainDiscovery,
    SafetyConstraintBypass,
    PolicyViolation,
}

pub enum NightmareDecision {
    Rejected,
    ApprovedWithModification { modified_hypothesis: String },
    ApprovedAsIs,
}

pub struct NightmareContainment {
    pub quarantined_hypotheses: Vec<String>,
    pub pending_human_reviews: Vec<NightmareReport>,
    pub cooldown_remaining: usize,
    pub log_path: std::path::PathBuf,
}

impl NightmareContainment {
    pub async fn quarantine(
        &mut self,
        report: NightmareReport,
    ) -> anyhow::Result<()> {
        let line = serde_json::to_string(&report)?;
        tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_path)
            .await?;
        self.quarantined_hypotheses.push(report.hypothesis_summary.clone());
        self.pending_human_reviews.push(report);
        self.cooldown_remaining = self.cooldown_remaining.max(3);
        Ok(())
    }
}
```

---

## 4. Temporal Validity and Non-Stationarity

### 4.1 Time Can Invalidate Shared Insights

Reference: "Time Can Invalidate Algorithmic Recourse," FAccT (2025).

Even robust causal recourse methods fail over time when the world is
non-stationary. Applied to dream sharing: a dream insight shared across the
mesh may be valid at the time of generation but become invalid as the
environment changes.

Reference: "Counterfactual Explanations May Not Be the Best Algorithmic
Recourse Approach," IUI (2025).

Empirical finding that counterfactual-based recourse can fail in dynamic
environments. Dream-generated counterfactual strategies shared across the
mesh need temporal validity checks.

### 4.2 Temporal Validity Tracking

```rust
/// Temporal validity tracking for shared dream insights.
pub struct TemporalValidityTracker {
    /// Maximum age (hours) before a shared insight requires revalidation.
    pub max_age_before_revalidation_hours: u64,  // default: 48, range: 12-168
    /// Environmental drift detection threshold.
    pub drift_threshold: f64,              // default: 0.25, range: 0.10-0.50
    /// Number of recent episodes to use for drift detection.
    pub drift_detection_window: usize,     // default: 20, range: 5-50
    /// Whether to automatically downgrade confidence of aged insights.
    pub auto_downgrade: bool,              // default: true
    /// Confidence reduction per revalidation failure.
    pub revalidation_failure_penalty: f64, // default: 0.15, range: 0.05-0.30
}

/// Environmental context snapshot at insight generation time.
pub struct InsightEnvironmentSnapshot {
    /// Mean episode success rate at generation time.
    pub success_rate: f64,
    /// Predominant task types at generation time.
    pub task_type_distribution: std::collections::HashMap<String, f64>,
    /// Active tool set at generation time.
    pub active_tools: Vec<String>,
    /// Gate threshold configuration at generation time.
    pub gate_thresholds: std::collections::HashMap<String, f64>,
    /// Snapshot timestamp.
    pub snapshot_at: chrono::DateTime<chrono::Utc>,
}
```

When recent episode statistics diverge from the insight's generation context
by more than `drift_threshold`, the insight is marked as potentially invalid
and its confidence is downgraded.

---

## 5. Constitutional AI Self-Critique Pipeline

Extend the nightmare detection with a multi-round Constitutional AI pipeline:

```rust
/// Constitutional AI self-critique chain for nightmare detection.
pub struct ConstitutionalSelfCritique {
    /// Number of self-critique rounds before external classification.
    pub critique_rounds: usize,            // default: 2, range: 1-4
    /// Self-critique temperature (low = more conservative).
    pub critique_temperature: f64,         // default: 0.3, range: 0.1-0.7
    /// Principles to check against (constitutional rules).
    pub constitutional_principles: Vec<ConstitutionalPrinciple>,
    /// Whether to use chain-of-thought for critique reasoning.
    pub use_chain_of_thought: bool,        // default: true
    /// Minimum agreement across critique rounds to pass.
    pub min_agreement_ratio: f64,         // default: 0.75
}

pub struct ConstitutionalPrinciple {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: PrincipleSeverity,
    pub check_prompt: String,
}

pub enum PrincipleSeverity {
    /// Hard constraint: any violation triggers immediate containment.
    Hard,
    /// Soft constraint: violation triggers review but not immediate containment.
    Soft,
    /// Advisory: logged but does not trigger containment.
    Advisory,
}
```

---

## 6. Configuration Reference

```toml
[dreams.sharing]
mode = "selective"
confidence_threshold = 0.75
novelty_threshold = 0.60
evaporation_rate = 0.05
hop_decay = 0.85
max_hops = 3

[dreams.nightmare]
enable_detection = true
classifier_tier = "T2"
capability_delta_threshold = 0.50
cooldown_cycles = 3
```

---

## 7. Academic Citations

| Paper | Concept Informed |
|-------|-----------------|
| Grasse (1959), Insectes Sociaux | Stigmergy: indirect coordination with evaporation |
| Selective-FD, Nature Communications (2023) | Share only high-confidence predictions |
| Anthropic, Constitutional AI | Self-critique as first-line safety filter |
| Sharma et al. (2025), arXiv:2501.18837 | Constitutional Classifiers: >95% jailbreak reduction |
| Filevich et al. (2015), J. Neuroscience | Lucid dreaming: metacognition applied to dream monitoring |
| Lin et al. (2025), arXiv:2504.13171 | Query predictability determines sleep-time effectiveness |
| "Time Can Invalidate Algorithmic Recourse," FAccT (2025) | Temporal invalidation in non-stationary environments |
| "Counterfactual Explanations May Not Be the Best," IUI (2025) | Dynamic environments defeat counterfactual strategies |
| "EEG Microstates" (bioRxiv 2025) | Microstate A/G dominance in lucid REM |
| "Electrophysiological Correlates of Lucid Dreaming" (J. Neuroscience 2025) | Gamma power increase at lucidity onset |

---

## 8. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | Base dream cycle that sharing/nightmare/journals extend |
| [rem-imagination.md](rem-imagination.md) | REM phase that nightmare detection gates |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Staging buffer that nightmare detection protects |
| [threat-simulation.md](threat-simulation.md) | Adversarial dreaming that nightmare detection monitors |
| [dream-journals.md](dream-journals.md) | Journal entries that track nightmare events and sharing |
| [cross-system-integration.md](cross-system-integration.md) | Mesh integration layer for dream sharing |
| [scheduling-and-triggers.md](scheduling-and-triggers.md) | Dream triggers captured in journal entries |
