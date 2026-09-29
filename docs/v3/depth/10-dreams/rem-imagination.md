# REM Imagination: Counterfactual Reasoning and Creative Recombination

> **v3 depth file** -- `/docs/v3/depth/10-dreams/rem-imagination.md`
> Canonical source: v1 `docs/v1/10-dreams/03-rem-imagination.md`
> Implementation: `crates/roko-dreams/src/imagination.rs`, `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `ImaginationMode`, counterfactual synthesis, affect-weighted
> creativity mode selection are live

---

## 1. What REM Imagination Does

REM (Rapid Eye Movement) imagination is the second phase of every dream cycle.
Where NREM replay strengthens and tests existing memories, REM imagination goes
further: it generates **genuinely novel hypotheses** by recombining elements from
different episodes, simulating counterfactual histories, and applying structured
creativity frameworks.

The biological analogy is REM sleep, during which the prefrontal cortex
(executive control) is suppressed while associative cortex remains active. This
creates a state where the brain can combine memories in ways that waking cognition
would inhibit (Hobson & Schredl 2011). Walker & van der Helm (2009, Psychological
Bulletin) showed that REM specifically depotentiates the emotional charge of
memories -- "overnight therapy."

---

## 2. Three Creativity Modes (Boden 2004)

The REM phase operates in three creativity modes, following Margaret Boden's
(2004, "The Creative Mind") taxonomy:

### 2.1 Combinational Creativity

Combine elements from unrelated episodes to discover unexpected similarities.
This is the weakest form of creativity but produces the most reliable results.

**Prompt structure**:
```
Given episode A about [topic X] and episode B about [topic Y],
what structural patterns do they share?
```

**Example**: Noticing that gas price spikes and governance vote deadlines share
the same timing pattern -- a structural similarity invisible when each domain
is considered in isolation.

### 2.2 Exploratory Creativity

Traverse the boundaries of existing strategy spaces, pushing parameters to
extremes. This tests the limits of current heuristics.

**Prompt structure**:
```
Your current heuristic says [X]. What happens at the extreme of this rule?
When would it break?
```

**Example**: Testing whether a "always retry failed tasks 3 times" heuristic
still works at 10 retries. The exploration discovers that beyond 5 retries,
the same error repeats identically, wasting compute.

### 2.3 Transformational Creativity

Violate fundamental assumptions of existing strategies to generate genuinely
novel approaches. This is the strongest and most speculative form.

**Prompt structure**:
```
What if [core assumption] were false? What strategy would you use instead?
```

**Example**: Imagining that compilation errors are actually test failures -- what
would the response strategy be? This assumption violation can reveal that the
agent's error-classification heuristics are too rigid.

---

## 3. Pearl's Structural Causal Model Framework

The REM phase implements counterfactual reasoning via Pearl's (2009, Causality)
three-level SCM framework:

### Level 1: Association

What correlates with what in the episode data? This is purely observational.

**Computational approach**: Scan the episode batch for co-occurrence patterns.
When event A appears in an episode, does event B appear more frequently? HDC
cosine similarity between episode vectors identifies candidates.

### Level 2: Intervention

What would happen if the agent had taken a different action? This is the do()
operator from Pearl's calculus.

**Computational approach**: Construct modified episode variants where one
variable is changed while others remain constant. Run the modified variant
through the agent's current heuristic set and predict the outcome.

### Level 3: Counterfactual

Given what actually happened, what would have happened if conditions had been
different? This requires reasoning about specific instances.

**Computational approach**: Take a specific failed episode. Identify the
decision point where the failure originated (using causal mode replay from
NREM). Construct a counterfactual variant where that decision was different.
Ask the LLM to reason through the consequences.

---

## 4. Counterfactual Fault Lines (Byrne 2005)

Byrne's (2005, The Rational Imagination) "fault lines" guide which
counterfactuals the agent explores first:

| Priority | Fault Line | Rationale |
|----------|------------|-----------|
| 1 | **Controllable actions** | Things the agent could have done differently |
| 2 | **Recent actions** | Temporally proximate decisions |
| 3 | **Abnormal actions** | Decisions that deviated from the agent's usual patterns |

Epstude & Roese (2008, Personality and Social Psychology Review) provide the
functional theory:

- **Upward counterfactuals** ("what if I had done better?") drive
  self-improvement
- **Downward counterfactuals** ("what if I had done worse?") serve as rehearsal
  for future threats

The REM phase generates both types. Upward counterfactuals become strategy
improvement hypotheses. Downward counterfactuals become threat rehearsal
scenarios (see [threat-simulation.md](threat-simulation.md)).

---

## 5. Emotional Depotentiation

During REM processing, the emotional charge of replayed memories decreases.
Walker & van der Helm (2009) showed that REM sleep reduces emotional arousal
by 0.3--0.5 units per cycle on a 0--1 scale:

```
post_dream_arousal = pre_dream_arousal - depotentiation_delta
depotentiation_delta in [0.3, 0.5] per cycle
```

The depotentiation delta scales with current arousal:

```
depotentiation_delta = 0.3 + (pre_dream_arousal - 0.5).max(0.0) * 0.4
```

High-arousal agents experience more depotentiation -- the agent "needs it more."
An agent at arousal 1.0 experiences delta 0.5; an agent at arousal 0.5
experiences delta 0.3.

This is the computational implementation of "overnight therapy": the agent
remembers what happened but no longer feels it as strongly. The informational
content of the memory is preserved while the emotional charge is reduced.

---

## 6. Model Selection for REM

The REM phase uses a **Sonnet-class model** (T1) because creative recombination
requires genuine reasoning. The CascadeRouter handles selection:

```rust
fn dream_phase_model(phase: &DreamPhase, router: &CascadeRouter) -> ModelConfig {
    match phase {
        DreamPhase::NremReplay { .. } => router.select_model(InferenceTier::T0),
        DreamPhase::RemImagination { .. } => router.select_model(InferenceTier::T1),
        DreamPhase::Integration { .. } => ModelConfig::None,
    }
}
```

If the T1 model is unavailable or the budget is exhausted, graceful degradation:
- T1 to T0 fallback for REM (lower quality but still functional)
- T0 to skip for NREM (defer the dream entirely)

---

## 7. Creativity Mode Selection by Affect

The agent's emotional state influences which creativity mode is favored:

| PAD Dimension | Effect on REM |
|--------------|--------------|
| High Arousal (A > 0.7) | More REM time for emotional depotentiation |
| Negative Pleasure (P < -0.3) | Combinational mode favored (safer, more reliable) |
| High Pleasure (P > 0.5) | Exploratory mode favored (more risk-tolerant) |
| Low Dominance (D < -0.3) | Threat simulation emphasis |

This creates a feedback loop: the agent's emotional state shapes its dreams,
which shape its emotional state through depotentiation, which shapes future
dreams.

---

## 8. Hypothesis Staging

REM-generated hypotheses enter the staging buffer at confidence 0.20--0.30.
Each hypothesis includes:

- The generation mode (combinational, exploratory, transformational, Pearl SCM
  level)
- The source episodes that contributed
- The hypothesis content (natural language)
- An HDC vector for similarity queries
- Whether the hypothesis contradicts existing knowledge

Hypotheses that contradict existing knowledge receive a -0.05 confidence
penalty. Hypotheses generated by transformational creativity also receive a
-0.05 penalty (most speculative mode). Multiple source episodes provide a
+0.02 per source bonus (max +0.10).

---

## 9. Academic Citations

| Paper | How It Informs REM Imagination |
|-------|-------------------------------|
| Boden (2004), The Creative Mind | Three creativity modes taxonomy |
| Pearl (2009), Causality | Three-level SCM for counterfactual reasoning |
| Byrne (2005), The Rational Imagination | Fault lines for counterfactual exploration priority |
| Epstude & Roese (2008), PSPR | Upward/downward counterfactual functional theory |
| Walker & van der Helm (2009), Psychological Bulletin | REM emotional depotentiation ("overnight therapy") |
| Hobson & Schredl (2011) | Prefrontal suppression enables creative recombination |
| Revonsuo (2000), Behavioral and Brain Sciences | Threat simulation during REM |
| Simonton (2010) | BVSR: blind variation and selective retention |
| Hafner et al. (2025), DreamerV3, Nature | Dream-based consolidation as architectural inspiration |
| McClelland et al. (1995), CLS theory | Fast/slow memory bridged by sleep replay |

---

## 10. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | REM is Phase 2 |
| [nrem-replay.md](nrem-replay.md) | NREM replay outputs feed REM |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Staging buffer receives REM hypotheses |
| [hdc-counterfactual-synthesis.md](hdc-counterfactual-synthesis.md) | HDC operations for counterfactual blending |
| [threat-simulation.md](threat-simulation.md) | REM sub-mode for adversarial dreaming |
| [divergence-and-alpha.md](divergence-and-alpha.md) | Affective divergence through emotional processing |
