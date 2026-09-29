# Affect-Modulated Retrieval

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/context_assembler.rs` -- PadState and scoring hooks
> v1 source: `docs/v1/03-composition/12-affect-modulated-retrieval.md`

---

## Overview

Affect-modulated retrieval uses the Daimon's PAD (Pleasure-Arousal-Dominance)
vector to bias which context is surfaced during assembly. An anxious agent (low
pleasure, high arousal) automatically receives more cautionary context --
anti-patterns, past failure summaries, conservative guidance. A confident,
exploratory agent (high pleasure, low arousal) receives more novel context --
research memos, cross-domain insights, experimental approaches.

This is the computational analog of Damasio's somatic marker hypothesis (1994):
before the agent reasons about which context to use, the affect state has
already biased the retrieval scores. The agent's "reasoning" (the LLM
generation) starts from a context set that has been emotionally filtered.

---

## 1. The PAD Model

Albert Mehrabian (1996) defined the Pleasure-Arousal-Dominance model as a
three-dimensional emotional space. Unlike basic sentiment (positive/negative),
PAD captures motivational state:

```rust
// crates/roko-compose/src/context_assembler.rs

pub struct PadState {
    /// Pleasure dimension. Range: [-1.0, 1.0]
    /// Positive: task success, gate passes, good outcomes
    /// Negative: task failure, gate rejections, errors
    pub pleasure: f64,

    /// Arousal dimension. Range: [-1.0, 1.0]
    /// Positive: time pressure, high urgency, approaching deadline
    /// Negative: idle time, no pressure, exploratory mode
    pub arousal: f64,

    /// Dominance dimension. Range: [-1.0, 1.0]
    /// Positive: high confidence, autonomous action
    /// Negative: low confidence, seeking guidance
    pub dominance: f64,
}
```

### 1.1 Why PAD, Not Sentiment

PAD captures motivational state, not just valence:

- **Pleasure** determines risk tolerance: low -> conservative, high -> adventurous
- **Arousal** determines urgency: high -> action-oriented, low -> reflective
- **Dominance** determines autonomy: high -> act independently, low -> seek help

A simple positive/negative sentiment model would treat "confident and exploring"
the same as "excited and rushing" -- both are positive. PAD distinguishes them:
the first is +P -A +D (favors novel context), the second is +P +A +D (favors
concise, action-oriented context).

### 1.2 PAD Octants

The three dimensions define eight behavioral states:

| Octant | P | A | D | State | Context Bias |
|--------|---|---|---|-------|-------------|
| +P +A +D | + | + | + | **Excited** | Action-oriented, recent, concise |
| +P +A -D | + | + | - | **Surprised** | Directive, structured guidance |
| +P -A +D | + | - | + | **Confident** | Exploratory, novel, cross-domain |
| +P -A -D | + | - | - | **Calm** | Comprehensive, thorough |
| -P +A +D | - | + | + | **Angry** | Focused, targeted at error source |
| -P +A -D | - | + | - | **Anxious** | Cautionary, anti-patterns, warnings |
| -P -A +D | - | - | + | **Bored** | Stimulating, diverse, challenging |
| -P -A -D | - | - | - | **Sad** | Supportive, past successes, proven patterns |

---

## 2. Modulation Rules

### 2.1 Arousal Modulation

| Condition | Threshold | Effect on Context Retrieval |
|-----------|----------|---------------------------|
| High arousal | arousal >= 0.35 | Boost recent content (recency bonus x1.5). Boost action-oriented content (task brief, file context, gate errors). Suppress exploratory content (research memo, cross-plan context). |
| Low arousal | arousal <= -0.35 | Boost novel content (novelty bonus x1.5). Boost exploratory content (research memo, cross-domain insights). Suppress urgency signals. |
| Neutral | -0.35 < arousal < 0.35 | No modulation |

SystemPromptBuilder implementation:

```rust
// High arousal affect guidance (Layer 8)
if arousal >= 0.35 {
    "You are under time pressure. Focus on the most impactful changes first.
     Avoid over-engineering. Prefer simple, correct solutions over elegant ones."
}

// Low arousal affect guidance
if arousal <= -0.35 {
    "You have time to explore. Consider multiple approaches before committing.
     Read surrounding code carefully. Look for patterns you can reuse."
}
```

### 2.2 Pleasure Modulation

| Condition | Threshold | Effect on Context Retrieval |
|-----------|----------|---------------------------|
| Low pleasure | pleasure <= -0.35 | Boost anti-patterns/warnings (x1.5). Boost past failure context. Boost conservative guidance. |
| High pleasure | pleasure > 0.35 | No special boost (default behavior is confident) |
| Neutral | -0.35 < pleasure <= 0.35 | No modulation |

```rust
if pleasure <= -0.35 {
    "Recent attempts have had issues. Be extra careful with your changes.
     Double-check your work against the acceptance criteria before finishing."
}
```

### 2.3 Dominance Modulation

| Condition | Threshold | Effect on Context Retrieval |
|-----------|----------|---------------------------|
| Low dominance | dominance <= -0.35 | Boost explanatory context (architecture docs, overviews). Boost structured guidance (step-by-step). |
| High dominance | dominance > 0.35 | Boost directive context (task brief, acceptance criteria). Suppress explanatory. |
| Neutral | -0.35 < dominance <= 0.35 | No modulation |

---

## 3. Integration with ContextAssembler

The ContextAssembler accepts an optional PadState that biases its scoring:

```rust
impl ContextAssembler {
    pub const fn with_affect_state(mut self, affect_state: Option<PadState>) -> Self {
        self.affect_state = affect_state;
        self
    }
}
```

When PadState is present, scoring is modulated:

```rust
fn score_chunk(
    task_text: &str,
    chunk: &ContextChunk,
    affect: Option<&PadState>,
) -> f64 {
    let base = source_priority(&chunk.source)
        + chunk.relevance * 0.4
        + chunk.track_record.unwrap_or(0.0) * 0.3
        + chunk.confidence.unwrap_or(0.5) * 0.2
        + chunk.recency.unwrap_or(0.5) * 0.1;

    let affect_modifier = match affect {
        Some(pad) if pad.arousal >= 0.35 => {
            // High arousal: boost recent and action-oriented
            chunk.recency.unwrap_or(0.0) * 0.2
        }
        Some(pad) if pad.pleasure <= -0.35 => {
            // Low pleasure: boost anti-knowledge and warnings
            if matches!(chunk.source, ContextSource::AntiPattern) { 0.3 }
            else { 0.0 }
        }
        _ => 0.0,
    };

    base + affect_modifier
}
```

Additionally, when PadState is present, the knowledge query limit is doubled
(from 10 to 20 candidates) to provide a richer candidate pool for affect-biased
selection.

---

## 4. PAD State Sources

### 4.1 Appraisal Triggers

The PAD vector is updated by appraisal triggers -- events that change the
agent's emotional state:

| Event | Pleasure | Arousal | Dominance |
|-------|----------|---------|-----------|
| Gate pass (first attempt) | +0.2 | -0.1 | +0.1 |
| Gate pass (after retry) | +0.1 | -0.05 | +0.05 |
| Gate failure | -0.2 | +0.15 | -0.1 |
| Consecutive failures (3+) | -0.3 | +0.3 | -0.2 |
| Task completed under budget | +0.15 | -0.1 | +0.15 |
| Task exceeded budget | -0.1 | +0.2 | -0.1 |
| Approaching deadline | 0 | +0.25 | -0.05 |
| Idle (no active tasks) | 0 | -0.2 | 0 |

### 4.2 Decay Toward Baseline

The PAD vector decays toward neutral [0, 0, 0] with a configurable half-life:

```
pad(t) = pad(t-1) * exp(-ln(2) / half_life * dt)
```

Default half-life: 30 minutes. After 30 minutes without new appraisal events,
the PAD vector is halved. After approximately 2 hours, it is near zero.

Decay prevents permanent affect drift: a series of failures creates temporary
anxiety that naturally dissipates over time. Without decay, cumulative negative
events would push the agent into permanent pessimism.

### 4.3 Persistence

PAD state persists across `roko plan run` invocations via `.roko/daimon/affect.json`:

```json
{
  "pleasure": -0.15,
  "arousal": 0.22,
  "dominance": 0.08,
  "updated_at": "2026-09-12T14:30:00Z"
}
```

On restart, the PAD vector is loaded and decayed from `updated_at` to the
current time. This means a plan that failed overnight starts the next morning
with a nearly neutral affect state.

---

## 5. Behavioral Effects

### 5.1 Anxious Agent (Low Pleasure, High Arousal)

Scenario: Three consecutive gate failures on a cross-crate integration task.

PAD state: pleasure = -0.45, arousal = 0.50, dominance = -0.25

Context effects:
- Anti-patterns boosted x1.5 -- common failure modes appear prominently
- Warning knowledge entries prioritized -- "this import path changed in v3"
- Gate errors placed at End (high-attention recency) -- specific test failures
- Affect guidance: "Recent attempts have had issues. Be extra careful."
- Conservative model selection -- CascadeRouter may prefer a more capable model

### 5.2 Confident Explorer (High Pleasure, Low Arousal)

Scenario: Five consecutive gate passes, no time pressure, idle period.

PAD state: pleasure = 0.55, arousal = -0.40, dominance = 0.35

Context effects:
- Novel content boosted x1.5 -- cross-domain insights, research memos surfaced
- Exploratory guidance -- "Consider multiple approaches before committing"
- Research memo included (normally Medium priority, now boosted)
- Cross-plan context included -- broader awareness of system architecture
- Exploration-friendly model selection -- creative approaches encouraged

### 5.3 Urgent Executor (Neutral Pleasure, High Arousal)

Scenario: Approaching deadline, many tasks remaining.

PAD state: pleasure = 0.0, arousal = 0.60, dominance = 0.10

Context effects:
- Recent content boosted x1.5 -- most recent relevant files and signals
- Action-oriented content prioritized -- task brief, file context, criteria
- Research memo suppressed -- no time for exploration
- Affect guidance: "You are under time pressure. Focus on impact."
- Faster model selection -- CascadeRouter prefers speed over depth

---

## 6. Connection to Somatic Markers

Antonio Damasio's somatic marker hypothesis (1994) proposes that emotional
reactions guide decision-making by rapidly narrowing the field of choices.
Before conscious reasoning, the body's emotional response eliminates options
that feel wrong and highlights options that feel right.

The PAD-modulated retrieval system implements a computational analog: before
the agent reasons about which context to use, the affect state has already
biased the retrieval scores. High-arousal states rapidly narrow the field to
action-oriented content. Low-pleasure states rapidly highlight cautionary
content. The agent's "reasoning" starts from a context set that has already
been emotionally filtered.

This is not a metaphor -- it is a functional equivalence. Damasio's somatic
markers reduce the combinatorial explosion of decision-making by pruning the
option space before deliberation. The PAD modulation reduces the combinatorial
explosion of context selection by biasing retrieval scores before the assembly
pipeline runs.

---

## 7. Connection to Neuromodulation

Doya (2002, "Metalearning and Neuromodulation") mapped biological
neuromodulators to computational meta-parameters:

| Neuromodulator | Computational Role | PAD Mapping |
|---------------|-------------------|-------------|
| Dopamine | Reward prediction, motivation | Pleasure dimension |
| Serotonin | Time discount, patience | Arousal dimension (inverted) |
| Noradrenaline | Exploration vs. exploitation | Dominance dimension |
| Acetylcholine | Learning rate, attention | Implicit in retrieval modulation |

The PAD modulation of retrieval is analogous to Doya's framework: the arousal
dimension acts like serotonin (controlling patience vs. urgency), the pleasure
dimension acts like dopamine (controlling approach vs. avoidance), and the
dominance dimension acts like noradrenaline (controlling exploitation vs.
exploration).

Elizabeth Phelps (2006, "Emotion and Cognition: Insights from Studies of the
Human Amygdala") demonstrated that emotional arousal enhances memory encoding
and retrieval for emotionally relevant stimuli while suppressing retrieval of
neutral information. This matches the arousal modulation: high arousal boosts
retrieval of recent, actionable context (emotionally relevant) while
suppressing exploratory context (emotionally neutral).

Gordon Bower (1981, "Mood and Memory") established mood-congruent retrieval:
people in negative moods more easily retrieve negative memories. The pleasure
modulation implements this directly: low pleasure boosts anti-patterns and
failure history (negative context) while high pleasure leaves positive/neutral
context at baseline.

---

## 8. Integration with VCG Auction

The PAD state modulates VCG bids through the `affect_weight` component:

```
bid(section) = expected_value * urgency * affect_weight
```

The affect_weight varies by subsystem and PAD state:

| Subsystem | High Arousal | Low Pleasure | Low Dominance |
|-----------|-------------|-------------|---------------|
| Neuro | x0.8 (suppress exploration) | x1.5 (boost warnings) | x1.2 (boost explanatory) |
| Daimon | x1.3 (affect is relevant) | x1.3 (affect is relevant) | x1.1 |
| IterationMemory | x1.3 (boost retries) | x1.2 (boost failures) | x1.0 |
| CodeIntelligence | x1.2 (boost files) | x1.0 | x0.9 |
| PlaybookRules | x1.1 | x1.3 (boost caution) | x1.0 |
| Research | x0.7 (suppress) | x1.0 | x1.3 (boost understanding) |
| TaskContext | x1.3 (boost directive) | x1.0 | x0.9 |
| Oracles | x1.2 (boost predictions) | x1.1 | x1.0 |
| GroupContext | x1.1 | x1.0 | x1.2 (boost collective) |

This creates a coherent behavioral profile: an anxious agent gets boosted
warnings from Neuro, boosted failure context from IterationMemory, and
suppressed exploration from Research -- all through the same PAD state flowing
through different bid computations.

---

## 9. Academic Foundations

**Mehrabian, A. (1996), "Pleasure-Arousal-Dominance: A General Framework for
Describing and Measuring Individual Differences in Temperament."** Current
Psychology, 14(4), 261-292. The foundational paper defining the PAD model.

**Damasio, A. (1994), "Descartes' Error: Emotion, Reason, and the Human Brain."**
Putnam. The somatic marker hypothesis: emotional reactions guide rational
decision-making by rapidly pruning option spaces.

**Bower, G. H. (1981), "Mood and Memory."** American Psychologist, 36(2),
129-148. Mood-congruent retrieval: negative moods facilitate retrieval of
negative memories. The empirical foundation for pleasure-modulated context
selection.

**Phelps, E. A. (2006), "Emotion and Cognition: Insights from Studies of the
Human Amygdala."** Annual Review of Psychology, 57, 27-53. Emotional arousal
enhances encoding and retrieval for emotionally relevant stimuli while
suppressing neutral information retrieval.

**Plutchik, R. (1980), "Emotion: A Psychoevolutionary Synthesis."** Harper &
Row. Plutchik's wheel of emotions maps to PAD octants, providing categorical
labels for continuous emotional states.

**Doya, K. (2002), "Metalearning and Neuromodulation."** Neural Networks,
15(4-6), 495-506. Mapped biological neuromodulators to computational
meta-parameters: dopamine (reward), serotonin (time discount), noradrenaline
(exploration), acetylcholine (learning rate).

**Friston, K. (2022), The Free Energy Principle.** Active inference + affect:
the PAD state modulates the balance between pragmatic and epistemic value in the
EFE formula, biasing context selection toward exploitation (high arousal) or
exploration (low arousal).

---

## 10. Current Status and Gaps

| Aspect | Status |
|--------|--------|
| PadState struct | **Implemented** |
| Arousal modulation in scoring | **Implemented** (score_chunk function) |
| Pleasure modulation in scoring | **Implemented** (anti-pattern boost) |
| Dominance modulation | **Implemented** (in VCG bid computation) |
| Affect guidance in SystemPromptBuilder | **Implemented** (Layer 8, arousal/pleasure) |
| PAD persistence | **Implemented** (.roko/daimon/affect.json) |
| PAD decay | **Implemented** (configurable half-life) |
| Appraisal triggers | **Implemented** (gate outcomes, budget, deadline) |
| VCG affect_weight integration | **Implemented** (per-subsystem modulation) |
| CascadeRouter affect integration | **Implemented** (model selection bias) |

---

## Cross-References

| File | Relationship |
|------|-------------|
| `system-prompt-builder-9-layer.md` | Layer 8: Affect Guidance |
| `active-inference-context-selection.md` | EFE modulated by PAD |
| `5-stage-assembly-pipeline.md` | Scoring with affect modifier |
| `vcg-attention-auction.md` | Affect weight in bid formula |
| [06-COMPOSITION.md](../../06-COMPOSITION.md) ss 7 | Parent chapter affect modulation |
| [11-AFFECT.md](../../11-AFFECT.md) | Daimon affect system specification |
