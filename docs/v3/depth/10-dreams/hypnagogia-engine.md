# The Hypnagogia Engine: Four-Layer Creative Onset

> **v3 depth file** -- `/docs/v3/depth/10-dreams/hypnagogia-engine.md`
> Canonical source: v1 `docs/v1/10-dreams/07-hypnagogia-engine.md`
> Implementation: `crates/roko-dreams/src/hypnagogia.rs`
> Status: **Wired** -- `HypnagogiaEngine` runs before the structured NREM/REM/Integration
> phases; entries counted in `DreamCycleReport::hypnagogia_entries_count`

---

## 1. What Hypnagogia Is

Hypnagogia is the transitional state between waking and sleep -- the liminal
threshold where executive control has loosened but not collapsed. Lacaux et al.
(2021, Science Advances) demonstrated that subjects in the hypnagogic state
solved 83% of creative problems versus 30% for fully awake subjects, replicating
the legendary "Edison technique."

In Roko, the **hypnagogia engine** is a four-layer creative onset system that
operates at the transition into a dream cycle. Before the structured phases
begin, the hypnagogia engine runs a brief (~30--60 second) creative exploration
designed to produce genuinely novel associations.

---

## 2. The Four Layers

### Layer 1: Thalamic Gate (Anti-Correlated Retrieval)

The thalamic gate retrieves knowledge entries from NeuroStore that are
**maximally dissimilar** to the agent's current focus. This is anti-correlated
retrieval: instead of finding what is similar, find what is maximally different.

Implementation: Compute the HDC centroid of the agent's recent episodes, then
query NeuroStore for entries with the lowest Hamming similarity to that centroid.

**Purpose**: Surface forgotten or neglected knowledge that the agent's waking
cognition would never retrieve. These are the "spectral traces" (see
[hauntology-in-dreams.md](hauntology-in-dreams.md)).

### Layer 2: Executive Loosener (Elevated Temperature)

The executive loosener increases the LLM's sampling temperature during
hypnagogia:

| Parameter | Waking Value | Hypnagogia Value |
|-----------|-------------|-----------------|
| Temperature | 0.7 | 1.3 |
| top_p | 0.90 | 0.95 |

The elevated temperature mimics the biological suppression of prefrontal
executive control during sleep onset. The model produces more diverse, less
predictable outputs -- some of which will be noise, but some will be genuine
creative leaps.

### Layer 3: Dali Interrupt (Short Associative Fragments)

The Dali Interrupt generates 50--100 token associative fragments from the
anti-correlated seed material. These fragments are deliberately short and
incomplete -- they capture the onset of an idea, not its development.

Named after Salvador Dali's technique of dozing with a key in his hand: he
would fall asleep, the key would drop, the clang would wake him, and he would
paint whatever image was in his mind at that liminal moment.

### Layer 4: Homuncular Observer (Structured Scoring)

The observer evaluates each fragment against three criteria using a low
temperature (T=0.4) for conservative judgment:

| Criterion | Weight | Threshold |
|-----------|--------|-----------|
| **Novelty** | 0.4 | HDC distance from existing knowledge centroid > 0.30 |
| **Relevance** | 0.3 | At least one connection to current task domain |
| **Coherence** | 0.3 | Logically self-consistent (no contradictions within the fragment) |

Fragments scoring above the composite threshold are retained as seeds for the
REM phase. Below-threshold fragments are discarded.

---

## 3. Composite Scoring

The composite score combines all three criteria:

```
composite = novelty * 0.4 + relevance * 0.3 + coherence * 0.3
```

The novelty weighting is highest because the primary purpose of hypnagogia is to
produce associations that waking cognition cannot. A perfectly relevant,
perfectly coherent fragment that is not novel has no value -- the agent already
knows it.

---

## 4. Stochastic Resonance

The hypnagogia engine uses controlled noise injection following Gammaitoni et al.
(1998, "Stochastic Resonance," Reviews of Modern Physics). The noise level is
calibrated so that weak but genuine patterns are amplified:

- Too little noise: only obvious patterns detected (exploitation trap)
- Too much noise: signal lost in randomness (exploration waste)
- Optimal noise: weak patterns amplified, novel connections surfaced

The elevated temperature in Layer 2 serves as the noise source. The Homuncular
Observer in Layer 4 serves as the signal detector.

---

## 5. Integration with Dream Cycle

Hypnagogic fragments that score above threshold feed into the REM phase as seed
material for counterfactual generation:

```
Waking State
  -> Hypnagogia (transition phase)
    -> Thalamic Gate: anti-correlated HDC retrieval from Neuro
    -> Executive Loosener: elevated temperature (T=1.3, top_p=0.95)
    -> Dali Interrupt: 50-100 token associative fragments
    -> Homuncular Observer: structured scoring (T=0.4)
  -> Dream Cycle (NREM -> REM -> Integration)
```

The `DreamCycleReport` tracks `hypnagogia_entries_count` -- the number of
fragments retained from the hypnagogia phase.

---

## 6. Kluver Form Constants (TUI Rendering)

Heinrich Kluver (1926) catalogued four geometric hallucination patterns at the
edge of sleep: tunnels/funnels, spirals, lattices/honeycombs, and cobwebs. These
arise from the visual cortex's self-organizing dynamics.

The TUI dream portal renders these form constants using braille characters
(U+2800--U+28FF) as background patterns during the hypnagogia phase. Over the
phosphene background, text fragments from NeuroStore surface and dissolve.

---

## 7. Academic Citations

| Paper | Contribution |
|-------|-------------|
| Lacaux et al. (2021), Science Advances | 83% creative problem solving in hypnagogic state |
| Kluver (1926) | Four geometric hallucination form constants |
| Gammaitoni et al. (1998), Reviews of Modern Physics | Stochastic resonance theory |
| Hobson & Schredl (2011) | Executive control suppression during sleep onset |
| Filevich et al. (2015), J. Neuroscience | Metacognitive monitoring of dream states |

---

## 8. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | Hypnagogia precedes the three-phase cycle |
| [rem-imagination.md](rem-imagination.md) | REM consumes hypnagogic fragments as seeds |
| [hdc-counterfactual-synthesis.md](hdc-counterfactual-synthesis.md) | Anti-correlated HDC retrieval |
| [divergence-and-alpha.md](divergence-and-alpha.md) | Hypnagogia as the primary creative divergence mechanism |
| [hauntology-in-dreams.md](hauntology-in-dreams.md) | Spectral traces surfaced by anti-correlation |
| [inner-worlds-and-rendering.md](inner-worlds-and-rendering.md) | TUI rendering of hypnagogic state |
