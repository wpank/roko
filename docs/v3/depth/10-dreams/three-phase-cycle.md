# The Three-Phase Dream Cycle

> **v3 depth file** -- `/docs/v3/depth/10-dreams/three-phase-cycle.md`
> Canonical source: v1 `docs/v1/10-dreams/01-three-phase-cycle.md`
> Implementation: `crates/roko-dreams/src/cycle.rs`, `crates/roko-dreams/src/runner.rs`
> Status: **Wired** -- DreamCycle, DreamRunner, heartbeat scheduler, and checkpoint restore are live

---

## 1. Overview

Every dream cycle in Roko consists of three sequential phases, inspired by
biological mammalian sleep:

1. **NREM Replay** -- Re-processing past episodes with controlled mutations to
   consolidate memory and extract cross-episode patterns.
2. **REM Imagination** -- Generating counterfactual scenarios and novel strategies
   through creative recombination of existing knowledge.
3. **Integration** -- Evaluating dream outputs, staging validated hypotheses, and
   promoting them to permanent knowledge.

These phases are not metaphorical labels. Each phase implements a distinct
computational process with its own academic foundations, model requirements, and
output types. The three-phase structure is grounded in Complementary Learning
Systems (CLS) theory (McClelland et al. 1995) and the specific consolidation
mechanisms documented by Diekelmann & Born (2010, Psychological Review).

---

## 2. Phase 1: NREM Replay

**Purpose**: Strengthen significant memories, weaken irrelevant ones, and extract
cross-episode patterns.

**Biological basis**: During non-REM sleep (particularly slow-wave sleep stages
N2-N3), the hippocampus replays compressed versions of recent experiences at
accelerated timescales. Sharp-wave ripples coordinate the transfer of episodic
memories from hippocampus to neocortex (Buzsaki 1989; Ji & Wilson 2007). The
replay is not veridical -- it includes minor mutations that test whether patterns
hold under perturbation.

**Computational implementation**: The NREM replay phase selects episodes from the
episode log using the Mattar & Daw (2018, Nature Neuroscience) utility formula:

```
Utility(episode) = Gain x Need x (1 / spacing_penalty)
```

Where:
- **Gain** measures how much the agent's behavior would improve by better
  processing this episode. Episodes where the outcome was surprising (the
  agent's prediction error was large) have high gain.
- **Need** measures how often the agent encounters situations similar to this
  episode. Episodes from frequently encountered task types have high need.
- **Spacing penalty** implements spacing effects (Cepeda et al. 2006) --
  recently replayed episodes are penalized to prevent over-rehearsal.

Selected episodes are replayed with two types of mutation:

| Mutation Type | Description | Research Basis | Frequency |
|---------------|-------------|----------------|-----------|
| **Perturbation** | Key values within the episode are shifted within a plausible range | Perturbed replay from hippocampal studies | 30% of replays |
| **Bidirectional replay** | Episodes are replayed both forward (cause to effect) and backward (effect to cause) | Ambrose et al. (2016), Reverse Replay | All replays |

The replay phase uses a **Haiku-class model** (cheap, fast) because it primarily
involves pattern matching and comparison against existing knowledge, not creative
generation.

> **Full detail**: [nrem-replay.md](nrem-replay.md)

---

## 3. Phase 2: REM Imagination

**Purpose**: Generate novel strategies and counterfactual hypotheses through
creative recombination of existing knowledge.

**Biological basis**: During REM sleep, the brain enters a state of high-level
cortical activation without sensory input. The prefrontal cortex (executive
control) is suppressed while associative cortex is active, enabling novel
combinations of memories that would be inhibited during waking (Hobson & Schredl
2011). Walker & van der Helm (2009, Psychological Bulletin) demonstrated that
REM sleep specifically reduces the emotional charge of traumatic memories --
"overnight therapy."

**Computational implementation**: The REM phase operates in three creativity
modes, following Boden's (2004) taxonomy:

| Creativity Mode | Operation | Example |
|----------------|-----------|---------|
| **Combinational** | Combine elements from unrelated episodes to discover unexpected similarities | Noticing that gas price spikes and governance vote deadlines share the same timing pattern |
| **Exploratory** | Traverse the boundaries of existing strategy spaces, pushing parameters to extremes | Testing whether a "always retry failed tasks 3 times" heuristic still works at 10 retries |
| **Transformational** | Violate fundamental assumptions of existing strategies | Imagining that compilation errors are actually test failures -- what would the response be? |

The REM phase also implements **counterfactual reasoning** via Pearl's (2009)
three-level structural causal model (SCM) framework:

1. **Association** (Level 1): What correlates with what in the episode data?
2. **Intervention** (Level 2): What would happen if the agent had taken a
   different action?
3. **Counterfactual** (Level 3): Given what actually happened, what would have
   happened if conditions had been different?

Byrne's (2005, The Rational Imagination) "fault lines" guide which
counterfactuals the agent explores first:
- **Controllable actions**: Things the agent could have done differently
  (highest priority)
- **Recent actions**: Temporally proximate decisions (second priority)
- **Abnormal actions**: Decisions that deviated from the agent's usual patterns
  (third priority)

**Emotional depotentiation**: During REM processing, Walker & van der Helm (2009)
showed that the emotional charge of memories decreases by 0.3--0.5 units per
cycle (on a 0--1 arousal scale):

```
post_dream_arousal = pre_dream_arousal - depotentiation_delta
depotentiation_delta in [0.3, 0.5] per cycle
```

The REM phase uses a **Sonnet-class model** (more capable, more expensive) because
creative recombination requires genuine reasoning.

> **Full detail**: [rem-imagination.md](rem-imagination.md)

---

## 4. Phase 3: Integration

**Purpose**: Evaluate outputs from NREM replay and REM imagination, stage
validated hypotheses, and promote them to permanent knowledge.

**Biological basis**: The integration phase corresponds to the brief waking
periods between sleep cycles (interspersed micro-arousals) and the
consolidation that occurs during the transition between NREM and REM. During
these transitions, the brain evaluates which memories have been sufficiently
strengthened and which should decay (Stickgold & Walker 2013).

**Computational implementation**: Integration is a **pure computation phase** --
no LLM call is required. It operates on the outputs from NREM and REM.

### The Staging Buffer

Dream-generated hypotheses enter a staging buffer at confidence level 0.20--0.30.
This is the "maybe" zone -- the hypothesis is interesting enough to record but
not validated enough to act on.

### Confidence Ladder

Hypotheses climb a confidence ladder through waking validation:

| Confidence Range | Status | What Happens |
|-----------------|--------|--------------|
| 0.20--0.30 | **Staged** | Hypothesis just entered from a dream. No action taken yet. |
| 0.30--0.50 | **Partially Validated** | Some waking evidence supports the hypothesis. |
| 0.50--0.70 | **Strongly Supported** | Multiple confirmations. Agent acts on it tentatively. |
| >= 0.70 | **Promoted** | Hypothesis written to permanent NeuroStore. |

### Promotion to Permanent Knowledge

When a hypothesis reaches confidence >= 0.70, the integration phase promotes it:

1. Written to NeuroStore as a `KnowledgeEntry` with `source: "dream"` provenance
2. If it represents a new heuristic, also written to the playbook store
3. Staging buffer entry updated to `status: 'promoted'`
4. Promotion event emitted for downstream listeners

### What Integration Produces

| Output | Description | Destination |
|--------|-------------|-------------|
| Promoted insights | Validated hypotheses that became permanent knowledge | NeuroStore |
| Updated confidence | Existing knowledge entries whose confidence was adjusted | NeuroStore |
| Emotional depotentiation | Reduced arousal scores for processed episodes | Daimon PAD update |
| Meta-patterns | Cross-episode structural similarities from HDC clustering | NeuroStore |
| Dream report | A structured `DreamCycleReport` | `.roko/dreams/dream-{timestamp}.json` |

> **Full detail**: [consolidation-and-staging.md](consolidation-and-staging.md)

---

## 5. Dream Cycle State Machine

The dream cycle progresses through a deterministic state machine:

```
IDLE -> NREM_REPLAY -> REM_IMAGINATION -> INTEGRATION -> IDLE
```

Each state transition is logged. The agent cannot be interrupted mid-phase (the
current phase runs to completion before any transition). Between full dream
cycles, the agent may enter a micro-consolidation mode where only the most urgent
single replay is processed.

```rust
pub enum DreamPhase {
    /// No dream in progress. Agent is in waking mode.
    Idle,
    /// NREM replay: re-processing past episodes with mutations.
    NremReplay {
        episodes_to_replay: usize,
        episodes_replayed: usize,
    },
    /// REM imagination: counterfactual generation and creative recombination.
    RemImagination {
        counterfactuals_to_generate: usize,
        counterfactuals_generated: usize,
    },
    /// Integration: evaluating outputs, staging, promoting.
    Integration {
        hypotheses_to_evaluate: usize,
        hypotheses_evaluated: usize,
    },
}
```

---

## 6. Resource Allocation Across Phases

Each phase has different computational requirements:

| Phase | Model Tier | Context Window | Typical Duration | Cost Profile |
|-------|-----------|----------------|------------------|-------------|
| **NREM Replay** | Haiku-class (T0) | Minimal | 60--120 seconds for 10 episodes | Low (~$0.001/episode) |
| **REM Imagination** | Sonnet-class (T1) | Full | 120--300 seconds for 3--5 counterfactuals | Medium (~$0.01/counterfactual) |
| **Integration** | None (pure computation) | N/A | < 5 seconds | Negligible |

The asymmetry is deliberate. NREM replay is cheap pattern matching -- a fast,
inexpensive model suffices. REM imagination requires genuine reasoning and
creativity -- a more capable model is worth the cost. Integration is arithmetic
and database operations -- no model needed at all.

The CascadeRouter (see `crates/roko-learn/src/cascade_router.rs`) handles model
selection for each phase.

---

## 7. Concurrent Execution

Dream phases execute sequentially within a single dream cycle, but multiple
aspects of NREM replay can run concurrently:

- Multiple episodes can be replayed in parallel (each replay is independent)
- Replays do not share state until the aggregation step at the end of NREM
- REM imagination is sequential (each counterfactual builds on previous context)
- Integration is a single-threaded batch operation

The `DreamCycle` implementation in `crates/roko-dreams/src/cycle.rs` manages
this concurrency.

---

## 8. Biological Fidelity: Extended Sleep Architecture

The three-phase cycle (NREM, REM, Integration) maps cleanly to biological sleep
but omits transition phases. Diekelmann & Born (2010) and the WSCL computational
model (Skenderi et al. 2024) both demonstrate that the transitions between phases
are computationally active periods where the brain/model switches processing
modes.

In Roko, the hypnagogia engine (see [hypnagogia-engine.md](hypnagogia-engine.md))
serves as the WAKE to NREM transition -- a brief creative exploration phase that
runs before the structured NREM/REM/Integration phases begin.

```rust
/// Extended dream cycle with transition phases.
pub enum ExtendedDreamPhase {
    Idle,
    /// Transition: waking to sleep. Hypnagogia engine runs here.
    HypnagogicTransition {
        fragments_generated: usize,
        fragments_retained: usize,
    },
    NremReplay { episodes_to_replay: usize, episodes_replayed: usize },
    NremToRemTransition,
    RemImagination { counterfactuals_to_generate: usize, counterfactuals_generated: usize },
    Integration { hypotheses_to_evaluate: usize, hypotheses_evaluated: usize },
    /// Transition: sleep to waking. Metacognitive dream review.
    HypnopompicTransition { insights_summarized: usize },
}
```

---

## 9. Micro-Consolidation

Short idle gaps (2--5 minutes) between tasks can support micro-consolidation --
a single high-priority episode replay without the full dream cycle. Unlike a
full NREM, REM, Integration sequence, micro-consolidation replays only the
highest-utility episode (by Mattar-Daw scoring) and does not enter the REM
imagination phase.

Micro-consolidation is triggered when the agent detects an idle gap that meets
the minimum duration but falls short of the full `idle_threshold_mins` required
for a complete dream cycle. Because it skips REM and integration, it cannot
generate new hypotheses or promote knowledge -- it can only reinforce existing
episodic memory.

```rust
pub struct MicroConsolidation {
    pub min_idle_secs: u64,           // default: 120, range: 60-300
    pub max_micro_replays: usize,     // default: 1, range: 1-3
    pub can_stage: bool,              // default: false
    pub model_tier: ModelTier,        // default: T0 (Haiku-class)
}
```

---

## 10. Academic Citations

| Paper | Contribution to Dream Cycle |
|-------|----------------------------|
| McClelland et al. (1995), Psychological Review, CLS theory | Fast episodic to slow semantic transfer via sleep replay |
| Diekelmann & Born (2010), Psychological Review | NREM/REM consolidation mechanisms and transition phases |
| Mattar & Daw (2018), Nature Neuroscience | Utility formula for replay prioritization |
| Ji & Wilson (2007), Nature Neuroscience | Hippocampal sharp-wave ripple replay during SWS |
| Buzsaki (1989) | Sharp-wave ripple coordination model |
| Ambrose et al. (2016) | Bidirectional replay in hippocampal circuits |
| Cepeda et al. (2006) | Spacing effects in memory consolidation |
| Pearl (2009), Causality | Three-level SCM framework for counterfactuals |
| Boden (2004) | Three creativity modes: combinational, exploratory, transformational |
| Walker & van der Helm (2009), Psychological Bulletin | REM emotional depotentiation ("overnight therapy") |
| Hobson & Schredl (2011) | Prefrontal suppression during REM enables creative recombination |
| Stickgold & Walker (2013) | Sleep-dependent memory triage |
| Byrne (2005), The Rational Imagination | Fault lines for counterfactual exploration priority |
| Epstude & Roese (2008), PSPR | Functional theory of upward/downward counterfactuals |
| WSCL (Skenderi et al. 2024) | Computational validation of three-phase wake-sleep architecture |

---

## 11. Cross-References

| Document | Relevance |
|----------|-----------|
| [nrem-replay.md](nrem-replay.md) | Full detail on Mattar-Daw utility, four replay modes, HDC clustering |
| [rem-imagination.md](rem-imagination.md) | Pearl SCM, Boden creativity, emotional depotentiation |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Staging buffer, confidence ladder, promotion |
| [dream-evolution.md](dream-evolution.md) | Fourth phase: memetic selection and strategy evolution |
| [sleep-time-compute.md](sleep-time-compute.md) | Computational economics and budget allocation |
| [scheduling-and-triggers.md](scheduling-and-triggers.md) | Scheduling logic and trigger conditions |
| [hypnagogia-engine.md](hypnagogia-engine.md) | WAKE-to-NREM transition creative engine |
