# Cross-System Integration

> **v3 depth file** -- `/docs/v3/depth/10-dreams/cross-system-integration.md`
> Canonical source: v1 `docs/v1/10-dreams/15-cross-system-integration.md`
> Implementation: `crates/roko-dreams/src/cycle.rs`, `crates/roko-dreams/src/runner.rs`,
> with integration points in `roko-neuro`, `roko-learn`, `roko-compose`, `roko-agent`,
> `roko-daimon`, `roko-gate`
> Status: **Wired** -- all primary integration points (Neuro, Daimon, Learn, Compose,
> Gate, Orchestrator, Supervisor) are live in the dream cycle

---

## 1. Overview

Dreams do not exist in isolation. The dream subsystem is a cognitive cross-cut --
it touches every layer of the Roko architecture, injecting learned knowledge,
emotional modulation, and creative insights into the agent's waking operation.

In the two-fabric model, Dreams consume Substrate scans for completeness and
Bus subscriptions for reactivity, so Delta-speed consolidation can wake on
`substrate.engram.stored` instead of fixed polling. This document maps every
integration point between the dream subsystem and other Roko subsystems,
documenting the data flows, channels, and trait implementations that connect
dreams to the rest of the cognitive architecture.

### Reactive Input / Output Model

Dreams use both durable storage and live transport:

| Direction | Fabric | Channel | Purpose |
|-----------|--------|---------|---------|
| Substrate to Dreams | Substrate | Query / scan | Completeness pass over durable Signals during consolidation |
| Bus to Dreams | Bus | `substrate.engram.stored` | Pulse-triggered Delta wakeup when new durable material lands |
| Dreams to Substrate | Substrate | `put()` | Persist consolidated Signals and staged knowledge |
| Dreams to Bus | Bus | `engram.promoted`, `neuro.insight.promoted` | Broadcast promotion so Neuro and Compose can refresh without re-querying |

---

## 2. Dreams x Neuro (Knowledge Store)

The Neuro subsystem (`roko-neuro`) is the agent's persistent knowledge base --
episodes, insights, heuristics, warnings, causal links. Dreams are the primary
mechanism for transforming raw episodic memory into durable semantic knowledge.

### Data Flow: Neuro to Dreams

| Data | Direction | Purpose |
|------|-----------|---------|
| Episodes since last dream | Neuro to NREM Replay | Raw material for replay prioritization via Mattar-Daw |
| Causal edges with confidence | Neuro to REM Imagination | SCM counterfactual generation |
| Knowledge entries by tier | Neuro to Integration | Existing knowledge for deduplication and confidence updates |
| Embedding vectors | Neuro to HDC synthesis | Source material for counterfactual blending |

The `DreamEngine::schedule()` method reads episodes from the Neuro store to
determine whether enough unprocessed material has accumulated:

```rust
let episodes_path = self.workdir.join(".roko").join("episodes.jsonl");
let episodes = block_on(EpisodeLogger::read_all_lossy(&episodes_path)).ok()?;
let last_report = load_latest_dream_report(&self.report_dir()).ok().flatten();
let cutoff = last_report
    .as_ref()
    .and_then(|report| report.processed_through.or(Some(report.started_at)));

let recent: Vec<&Episode> = episodes
    .iter()
    .filter(|episode| cutoff.is_none_or(|ts| episode.timestamp > ts))
    .collect();
```

### Data Flow: Dreams to Neuro

| Data | Direction | Purpose |
|------|-----------|---------|
| `InsightRecord` entries | NREM Replay to Neuro | Discovered patterns, cross-episode correlations |
| `CounterfactualHypothesis` entries | REM Imagination to Neuro | Hypotheses staged at confidence 0.20-0.30 |
| Confidence updates | Integration to Neuro | Promoted insights with updated confidence |
| Deprecated entries | Integration to Neuro | Entries with reduced confidence after re-evaluation |
| Playbook revisions | Integration to Neuro | New or updated strategy entries |

The `TierProgression` system classifies dream-generated insights into tiers:

| Tier | Confidence Range | Typical Source |
|------|-----------------|----------------|
| T0 (Observation) | 0.00-0.19 | Raw episode fragments |
| T1 (Hypothesis) | 0.20-0.39 | REM counterfactual outputs |
| T2 (Emerging Pattern) | 0.40-0.59 | Cross-episode consolidation |
| T3 (Validated Insight) | 0.60-0.79 | Gate-validated discoveries |
| T4 (Established Knowledge) | 0.80-1.00 | Repeatedly confirmed, multi-dream validated |

### Bidirectional Feedback Loop

```
Episodes accumulate in Neuro
  -> Dreams replay and consolidate episodes
    -> Insights written to Neuro at T1-T2
      -> Promotion Pulses broadcast on the Bus
      -> Waking experience validates or refutes
        -> Validated insights promoted to T3+
          -> T3+ insights influence future dream replay priority
            -> Better dreams -> better insights -> cycle continues
```

---

## 3. Dreams x Daimon (Affect Engine)

The Daimon is the agent's affect engine -- it maintains a PAD
(Pleasure-Arousal-Dominance) emotional state vector.

### Emotional Context for Dreams

| PAD Dimension | Dream Effect |
|--------------|-------------|
| High Arousal (A > 0.7) | More REM time for emotional depotentiation |
| Negative Pleasure (P < -0.3) | Replay prioritizes failure episodes |
| Low Dominance (D < -0.3) | Threat simulation emphasis in REM |
| High Pleasure (P > 0.5) | Exploratory creativity mode favored |

### Emotional Depotentiation

The most important dream-Daimon interaction during REM:

```
pre_dream_arousal = daimon.pad().arousal
// ... REM processing ...
post_dream_arousal = pre_dream_arousal - depotentiation_delta

// Typical depotentiation: 0.3-0.5 per dream cycle
depotentiation_delta = 0.3 + (pre_dream_arousal - 0.5).max(0.0) * 0.4
```

This implements Walker & van der Helm's "overnight therapy" finding: REM
sleep depotentiates emotional charge while preserving informational content.

### Daimon State Updates from Dreams

| Update | Source | Effect |
|--------|--------|--------|
| Arousal reduction | REM depotentiation | PAD arousal decreases by 0.3-0.5 |
| Pleasure adjustment | Dream discoveries | Positive discoveries increase P |
| Dominance recalibration | Threat simulation | Successful rehearsal increases D |

---

## 4. Dreams x Learning Subsystem (roko-learn)

### Episode Logger Integration

The `EpisodeLogger` records agent turns. Dreams consume these as raw material:

```rust
let episodes = Arc::new(EpisodeLogger::new(
    self.workdir.join(".roko").join("episodes.jsonl"),
));
```

The `processed_through` timestamp in the dream report allows subsequent
cycles to skip already-processed episodes.

### Playbook Store Integration

Dreams can generate playbook revisions. The `PlaybookStore` is passed
directly to the `DreamCycle`:

```rust
let playbooks_root = self.workdir.join(".roko").join("learn").join("playbooks");
let playbooks = Arc::new(PlaybookStore::new(playbooks_root));
let mut cycle = DreamCycle::new(episodes, knowledge, playbooks, dispatcher);
```

### Pattern Discovery Integration

| Component | Role in Dreams |
|-----------|---------------|
| `PatternMiner` | Trigram mining across episodes with FNV-1a hashing |
| `CrossEpisodeConsolidator` | K-medoids clustering over HDC episode vectors |
| `EpisodeView` trait | Abstracts episode access for pattern mining |

### CascadeRouter Integration

Dream consolidation can update the `CascadeRouter`:

| Dream Output | Router Update |
|-------------|--------------|
| Model A outperforms B for task type X | Increase Model A weight for type X |
| Consistent gate failures with Model C | Decrease Model C weight for type Y |
| Novel strategy discovery with Model D at high temperature | Note Model D effectiveness for creative tasks |

---

## 5. Dreams x Context Engineering (roko-compose)

The context engineering subsystem assembles prompts using the
`SystemPromptBuilder`. Dreams integrate in two ways.

### Dream Context Injection

When an agent's context is assembled for a waking task:

| Context Layer | Dream Contribution |
|--------------|--------------------|
| Knowledge context | Recently promoted dream insights (T3+) |
| Emotional context | Post-dream PAD state (with depotentiation) |
| Strategy context | Dream-generated playbook revisions that have been validated |
| Warning context | Threat simulation results from dream REM phase |

### Dream Prompt Assembly

The dream consolidation process itself requires prompt assembly:

```rust
let dispatcher: Arc<dyn AgentDispatcher> =
    Arc::new(self.config.agent.build_agent(&self.workdir));
let mut cycle = DreamCycle::new(episodes, knowledge, playbooks, dispatcher);
cycle.run().await
```

Dreams typically use a cheaper, faster model (e.g., Haiku) with `bare_mode = true`
(no tool use) and `effort = "low"` for cost efficiency.

---

## 6. Dreams x Gate Pipeline (roko-gate)

### Gate Results Feed Dreams

Gate results from waking tasks are recorded as episode data:

| Gate Signal | Dream Use |
|------------|-----------|
| Compile gate failures | Replay with emphasis on error patterns |
| Test gate failures | Replay with emphasis on reasoning chains |
| Clippy warnings | Consolidate into coding heuristics |
| Diff gate anomalies | Replay to understand scope creep |

### Dreams Update Gate Thresholds

The adaptive gate threshold system uses EMA per rung. Dream consolidation
can propose threshold adjustments based on cross-episode analysis:

- Consistent false positives: threshold may be relaxed
- Consistent misses: threshold may be tightened
- Threshold proposals enter at low confidence and require waking validation

---

## 7. Dreams x Agent Mesh (Coordination)

In multi-agent deployments, Dreams interact with the Mesh through ordinary
fabrics: `MeshSubstrate` replicates durable dream outputs, and `MeshBus`
publishes live Pulses.

### Knowledge Sharing via Mesh

| Sharing Direction | Mechanism | Content |
|------------------|-----------|---------|
| Agent to Mesh | `MeshSubstrate.put()` + `MeshBus.publish()` | High-confidence insights (T3+), promotion Pulses |
| Mesh to Agent | `MeshBus` subscription + `MeshSubstrate` query | Peer insights at reduced confidence (x0.85 per hop) |

### Collective Dream Patterns

When multiple agents dream about similar episodes, the mesh identifies
collective patterns:

1. Agent A discovers pattern P1
2. Agent B independently discovers pattern P2
3. Mesh detects P1 and P2 are semantically similar (HDC cosine)
4. Both agents receive a "collective confirmation" signal

### Pheromone Field Integration

| Pheromone Signal | Dream Effect |
|-----------------|-------------|
| High threat pheromone (> 0.7) | Prioritize threat simulation in REM |
| Low activity pheromone | Extend NREM consolidation |
| Knowledge pheromone spike | Prioritize integration of mesh insights |

---

## 8. Dreams x Orchestrator (Plan Execution)

### Scheduling Coordination

The dream scheduler coordinates with the plan executor to find idle windows.
Dreams never interrupt active tasks. The orchestrator calls
`dream_runner.schedule_next()` after each task completion.

### Plan Feedback Loop

| Dream Output | Plan Effect |
|-------------|-------------|
| Recurring failure pattern | Flag similar pending tasks for re-planning |
| New heuristic for task type X | Enrich context for future tasks of type X |
| Discovered dependency between task outcomes | Suggest DAG edge additions |

---

## 9. Dreams x Fleet Coordination

```rust
/// Fleet-level dream coordination.
pub struct FleetDreamCoordinator {
    /// Whether to stagger dream cycles across fleet members.
    pub stagger_cycles: bool,             // default: true
    /// Minimum stagger interval between any two agents dreaming (minutes).
    pub min_stagger_mins: u64,            // default: 10, range: 5-60
    /// Whether to aggregate fleet dream insights for collective trend analysis.
    pub aggregate_insights: bool,         // default: true
    /// Collective insight confidence boost.
    pub collective_confirmation_boost: f64, // default: 0.15, range: 0.05-0.30
    /// Minimum agents confirming a pattern for collective boost.
    pub min_confirming_agents: usize,     // default: 2, range: 2-5
}
```

Staggered scheduling ensures continuous waking coverage: dream insights from
early dreamers propagate to later dreamers' waking contexts before they sleep.

---

## 10. Dreams x Configuration System

```toml
[dreams]
auto_dream = true
idle_threshold_mins = 15
min_episodes_for_dream = 5
scheduled_cron = "0 0 */4 * * * *"
episode_count_trigger = 50
quality_gain = 0.75
quality_penalty = 1.25
budget_fraction = 0.15
batch_size = 10

[dreams.agent]
command = "claude"
model = "claude-haiku-4-5-20251001"
bare_mode = true
effort = "low"
timeout_ms = 120000

[dreams.privacy]
nrem_provider = "local"
rem_provider = "api"
hypnagogia_provider = "api"

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

## 11. Event Flow Diagram

The complete event flow for a dream cycle touching all subsystems:

```
 1. Orchestrator detects idle gap after task completion
 2. DreamRunner reads episodes from EpisodeLogger (roko-learn)
 3. DreamRunner scans Substrate for unconsolidated durable Signals
 4. If batch threshold met, dream cycle fires

 5. NREM Phase:
    a. PatternMiner discovers trigram patterns (roko-learn)
    b. CrossEpisodeConsolidator clusters via K-medoids (roko-learn)
    c. Mattar-Daw utility scores prioritize episodes
    d. Replay generates InsightRecords

 6. REM Phase:
    a. Daimon provides PAD context
    b. Neuro provides causal graph for SCM counterfactuals
    c. Hypnagogia fragments seed creative generation
    d. Counterfactual hypotheses generated
    e. Emotional depotentiation applied

 7. Integration Phase:
    a. Insights staged in Neuro at T1-T2 confidence
    b. Hypotheses staged at 0.20-0.30 confidence
    c. Playbook revisions written to PlaybookStore
    d. Gate threshold proposals generated
    e. CascadeRouter updates proposed
    f. Promotion Pulses published for downstream refresh

 8. Post-Dream:
    a. DreamCycleReport persisted to .roko/dreams/
    b. Daimon PAD updated (arousal reduced)
    c. Orchestrator notified -> resumes task execution
    d. High-confidence insights shared to Mesh (if connected)
    e. Oneirography generates dream image (if configured)
```

---

## 12. Integration Summary Table

| Subsystem | Layer | Direction | Key Data |
|-----------|-------|-----------|----------|
| **Neuro** (Knowledge) | L1 | Bidirectional | Substrate scans + promotion Pulses; insights to Neuro |
| **Daimon** (Affect) | L1 | Bidirectional | PAD context to dreams; depotentiation to Daimon |
| **Learn** (Episodes) | L1 | Neuro-mediated | Episodes, playbooks, patterns, routing |
| **Compose** (Context) | L2 | Dreams to Context | Post-dream insights injected; Pulses refresh enrichment |
| **Gate** (Validation) | L3 | Bidirectional | Gate results to dreams; threshold updates to gates |
| **Mesh** (Coordination) | L4 | Bidirectional | Dream insights to Mesh; peer insights to Dreams |
| **Orchestrator** (Plans) | L4 | Bidirectional | Idle windows to dreams; feedback to plans |
| **Hypnagogia** (Creativity) | L0/L1 | Unidirectional | Hypnagogic fragments to dream seeds |
| **Supervisor** (Process) | L0 | Supervisor to Dreams | Lifecycle, cancellation, resource limits |
| **Oneirography** (Art) | L1/L2 | Dreams to Art | DreamCycleReport to image generation |

---

## 13. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | Dream cycle structure |
| [nrem-replay.md](nrem-replay.md) | NREM consuming episodes from EpisodeLogger |
| [rem-imagination.md](rem-imagination.md) | REM using Neuro causal graphs |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Integration writing results to Neuro |
| [hypnagogia-engine.md](hypnagogia-engine.md) | Hypnagogia fragments feeding dream seeds |
| [sleep-time-compute.md](sleep-time-compute.md) | Compute budget constraining dream inference |
| [scheduling-and-triggers.md](scheduling-and-triggers.md) | Scheduling coordinated with orchestrator |
| [oneirography.md](oneirography.md) | Art generation consuming DreamCycleReport |
| [dream-routing-advice.md](dream-routing-advice.md) | Routing advice feeding CascadeRouter |
