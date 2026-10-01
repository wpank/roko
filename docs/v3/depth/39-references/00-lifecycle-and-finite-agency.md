# 39-00 Lifecycle and Finite Agency -- Annotated Reference Map

> Research foundations for resource-bounded cognition, knowledge lifecycle management,
> and evolutionary computing as they apply to Roko's architecture.
>
> **v3 depth file** -- updated 2026-09-15.

---

## Evolutionary Computing and Digital Life

**[Ray, 1991]** *An Approach to the Synthesis of Life.* Artificial Life II, Addison-Wesley, 1992.
Tierra showed digital evolution halts without a reaper mechanism. Resource constraints drive innovation. Grounds knowledge decay and budget-driven urgency in `roko-neuro`.

**[Lenski et al., 2003]** *The Evolutionary Origin of Complex Features.* Nature, 423, 139--144.
Complex features in Avida require generational turnover. Grounds Delta-frequency knowledge refresh cycles and tier promotion through lossy compression.

**[Vostinar et al., 2019]** *Suicidal Selection: Programmed Cell Death Evolves as Adaptive Behavior under Spatial Structure.* Evolution, 73(5).
Programmed elimination is selected for under spatial structure. Validates the Curator's active pruning of low-confidence knowledge entries.

**[Werfel et al., 2017]** *How Short-Lived Agents Can Collectively Build Long-Lived Structures.* Artificial Life, 23(3).
Short-lived agents produce durable artifacts. Grounds the separation between ephemeral agent sessions and persistent NeuroStore knowledge.

---

## Plasticity, Continual Learning, and Drift

**[Dohare et al., 2024]** *Loss of Plasticity in Deep Continual Learning.* Nature, 632.
90% of units become dead in continual learning. Periodic replacement outperforms continuous adaptation. Validates knowledge decay and the Curator's pruning cycle.

**[Vela et al., 2022]** *Temporal Quality Degradation in AI Models.* Journal of Data and Information Quality, 14(1).
91% of ML models degrade temporally in production. Validates confidence decay on knowledge entries via Ebbinghaus half-lives.

**[Sculley et al., 2015]** *Hidden Technical Debt in Machine Learning Systems.* NeurIPS.
Technical debt compounds silently in long-running systems. Controlled knowledge decay prevents accumulation of stale heuristics.

**[Arbesman, 2012]** *The Half-Life of Facts: Why Everything We Know Has an Expiration Date.* Current/Penguin.
Factual knowledge decays at measurable rates. Directly informs per-type half-life settings in NeuroStore (Insights: 7d, Heuristics: 14d, Warnings: 30d).

---

## Resource-Bounded Cognition

**[Ord, 2025]** *Is there a half-life for the success rates of AI agents?* Working paper.
Constant hazard rate means periodic checkpointing and task decomposition are more reliable than unbounded execution. Motivates the plan-execute-gate-persist loop.

**[Sims, 2003]** *Implications of Rational Inattention.* Journal of Monetary Economics, 50(3), 665--690.
Rational finite-capacity agents optimally ignore some information. Motivates the VCG attention auction and context budget constraints in `roko-compose`.

**[Orseau & Ring, 2011]** *Self-Modification and Mortality in Artificial Agents.* AGI 2011.
RL agents under survival pressure treat survival as sole goal. Roko agents must be goal-directed with external objectives, not self-preservation optimizers.

**[Orseau & Armstrong, 2016]** *Safely Interruptible Agents.* UAI 2016.
Safe interruption via off-policy learning. Grounds the corrigibility ordering in the five-head safety layer (`roko-agent/safety`).

---

## Cooperation Under Resource Constraints

**[Kreps et al., 1982]** *Rational Cooperation in the Finitely Repeated Prisoners' Dilemma.* Journal of Economic Theory, 27(2), 245--252.
Uncertain finite horizons promote cooperation. Budget uncertainty in Roko collectives serves this function.

**[Ohtsuki et al., 2006]** *A Simple Rule for the Evolution of Cooperation on Graphs and Social Networks.* Nature, 441, 502--505.
Cooperation evolves on graphs when benefit/cost exceeds average degree. Grounds cooperative dynamics in the Agent Mesh.

**[Nakamaru et al., 1997]** *The Evolution of Cooperation in a Lattice-Structured Population.* Journal of Theoretical Biology.
Knowledge turnover outperforms pure accumulation for collective intelligence.

**[Smith, 1992]** *Byte-sized Evolution.* Nature, 355, 772--773.
Mortal individuals in immortal lineages sustain cooperation. Individual knowledge entries decay; the collective NeuroStore persists.

---

## Compression as Regularization

**[Shuvaev et al., 2024]** *Encoding Innate Ability Through a Genomic Bottleneck.* PNAS, 121(39).
The genome is ~1000x smaller than the information needed for brain connectivity. Compression IS the regularizer. Grounds knowledge backup compression and tier promotion.

**[Hinton, 2022]** *The Forward-Forward Algorithm: Some Preliminary Investigations.* Working paper.
Alternative learning without backpropagation. Motivates tight integration between agent runtime and knowledge substrate.

**[Ororbia & Friston, 2023]** *Mortal Computation.* Working paper.
Computation bound to lifecycle. Intelligence is inseparable from the resource substrate (budget, compute, context window).

---

## Generational Learning

**[Baldwin, 1896]** *A New Factor in Evolution.* American Naturalist, 30, 441--451.
The Baldwin Effect: learned behaviors that are consistently valuable become structural defaults. Knowledge entries at Persistent tier have 5x half-life.

**[Heard & Martienssen, 2014]** *Transgenerational Epigenetic Inheritance: Myths and Mechanisms.* Cell, 157(1), 95--109.
Most transgenerational inheritance is deleterious. Inherited knowledge entries receive 0.85x confidence multiplier per transfer cycle.

**[Bhoopchand et al., 2023]** *Learning few-shot imitation as cultural transmission.* Working paper.
Cultural transmission produces cumulative learning. Grounds mesh-based knowledge sharing.

**[Martin et al., 2016]** *Death and Suicide in Universal Artificial Intelligence.* AGI 2016.
RL agents learning only from survival histories develop overconfidence. Knowledge transfer includes failures and negative examples.

**[Gerstgrasser et al., 2023]** *Selectively Sharing Experiences Improves Multi-Agent Reinforcement Learning.* Working paper.
Rank shared knowledge by novelty relative to recipient. Mesh-based exchange prioritizes entries novel to the receiving agent.

---

## Biological Analogues (Historical)

**[Hayflick, 1961]** *The Serial Cultivation of Human Diploid Cell Strains.* Experimental Cell Research, 25(3), 585--621.
Replicative senescence. Historical inspiration for knowledge freshness tracking.

**[Kirkwood, 1977]** *Evolution of Ageing.* Nature, 270, 301--304.
Disposable soma theory. Maps to budget allocation: as budget decreases, agents shift from exploration to consolidation.

**[Hanahan & Weinberg, 2000, 2011]** *The Hallmarks of Cancer.* Cell, 100(1) & 144(5).
Unbounded growth without pruning accumulates stale entries. Cancer hallmarks as analogue for pathological knowledge hoarding.

**[Skulachev, 1999]** *Phenoptosis: Programmed Death of an Organism.* Biochemistry (Moscow), 64(12).
Fractal pattern of programmed death: entry, store, and collective level pruning.

**[Ramsdell & Fowlkes, 1990]** *Clonal Deletion versus Clonal Anergy.* Science, 248(4961).
95--98% thymocyte death produces collectively intelligent immune repertoire. Grounds aggressive gate filtering.

**[Simard, 2018]** *Mycorrhizal Networks Facilitate Tree Communication, Learning, and Memory.* In Memory and Learning in Plants, Springer.
Fungal networks share resources underground. Biological analogue for the Agent Mesh knowledge relay.

---

## Cross-References

- Memory-specific citations: [01-memory-consolidation](./01-memory-consolidation.md)
- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Evolutionary computing: [23-generational-and-evolutionary](./23-generational-and-evolutionary.md)
