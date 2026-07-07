# Open Questions — Emergent Goal Structures

**Kind**: Perspective
**Source**: `docs/00-architecture/28-emergent-goal-structures.md`

---

## Unresolved Design Questions

1. **How do you distinguish emergent misalignment from appropriate adaptation?** A system
   that develops an effective goal of "produce high-quality code" may look like goal drift
   from "follow instructions" but is actually better aligned with operator intent. The lens
   cannot automatically distinguish good emergence from bad emergence. What criteria
   distinguish them?

2. **What is the timescale of goal drift?** Is goal drift a slow process (detectable with
   monthly monitoring) or a fast process (could occur within a single session)? The answer
   determines the appropriate monitoring frequency.

3. **Can goal anchors be too strong?** Hard constraints prevent emergence in the constrained
   dimension. If safety constraints are too strong and too broad, they may prevent
   adaptation that is genuinely useful. How do you calibrate constraint strength?

4. **How does the Daimon state interact with goal stability?** The Daimon modulates scoring
   weights, which affects the attractor landscape. A Daimon that responds rapidly to
   context provides flexibility but instability. A Daimon that responds slowly provides
   stability but may be poorly adapted to new contexts. What is the right Daimon response
   timescale?

5. **Is there a formal relationship between the active inference generative model and the
   emergent goals?** In active inference, preferred observations define the attractor. Can
   this be exploited to design a system where the only attractors are the intended goal
   states — by designing the preferred observations carefully?

---

## Open Research Questions

1. **Mesa-optimization in Roko**: Does Roko's architecture create conditions for the
   emergence of inner optimizers? The Composer, in particular, selects a context bundle
   to maximize synthesis quality — is this selection process an inner optimizer with its
   own effective goal?

2. **Instrumental convergence at Roko's scale**: Omohundro's instrumental convergence
   theorem applies to systems with general optimization processes. Does it apply to Roko's
   more constrained architecture? What is the specific set of instrumental goals that
   would be convergent for Roko's architecture?

3. **Goal topology and [knowledge topology](../temporal-topology/README.md)**: Are there
   structural connections between the topology of the knowledge space and the topology of
   the goal landscape? Does a highly connected knowledge base stabilize or destabilize
   the goal attractor structure?
