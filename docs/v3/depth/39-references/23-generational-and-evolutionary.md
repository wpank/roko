# 39-23 Generational and Evolutionary -- Annotated Reference Map

> Research foundations for evolutionary computing, quality-diversity algorithms,
> memetic evolution, and generational knowledge transfer in Roko's EvoSkills
> and knowledge evolution systems.
>
> **v3 depth file** -- updated 2026-09-15. Added DGM.

---

## Digital Evolution

**[Ray, 1991]** *An Approach to the Synthesis of Life.* Artificial Life II, Addison-Wesley, 1992.
Tierra: resource pressure drives digital evolution. 300+ genotypes emerged only with finite lifespans. Grounds knowledge decay as the mechanism that prevents knowledge calcification. (See also [00-lifecycle](./00-lifecycle-and-finite-agency.md).)

**[Lenski et al., 2003]** *The Evolutionary Origin of Complex Features.* Nature, 423, 139--144.
LTEE: complex features require generational turnover. Tier promotion (Episode to Insight to Heuristic to Playbook) produces generalization through lossy compression. (See also [00-lifecycle](./00-lifecycle-and-finite-agency.md).)

---

## Genomic Bottleneck

**[Shuvaev et al., 2024]** *Encoding Innate Ability Through a Genomic Bottleneck.* PNAS, 121(39).
The genome is ~1000x smaller than brain connectivity information, yet organisms have innate behaviors. Compression (~2,000 gene limit) transfers better than raw knowledge. Grounds knowledge backup compression via the bottleneck principle.

---

## Cultural and Vertical Transmission

**[Bhatt et al., 2023]** *Few-Shot Imitation as Cultural Transmission.* Working paper.
Cultural transmission produces cumulative learning across agent populations. Few-shot imitation enables cross-agent learning.

**[Bourahla et al., 2022]** *Vertical Transmission Enables Agents to Exceed Performance Ceilings.* Working paper.
Inter-generational knowledge transfer enables exceeding individual ceilings. Grounds NeuroStore backup/restore as the mechanism for vertical knowledge inheritance.

**[Perez et al., 2024]** *Pure Imitation Leads to Stagnation.* AGI 2024.
Novelty requires mixing inheritance and exploration. Pure imitation without innovation leads to convergence. Motivates the 15% contrarian retrieval mandate and anti-proletarianization.

---

## Quality-Diversity Algorithms

**[Mouret & Clune, 2015]** *Illuminating Search Spaces by Mapping Elites (MAP-Elites).* arXiv:1504.04909.
Quality-diversity algorithm maintaining high-performing diverse solutions across feature space. Grounds the EvoSkills skill library -- maintaining a diverse repertoire of strategies, not just the single best.

**[GVU, 2024]** *Genome Value Update: Quality-Diversity for Robust Multi-Strategy Evolution.* Working paper.
Quality-diversity framework for evolving robust multi-strategy systems. Extends MAP-Elites with genome-level value tracking.

---

## Architecture Search

**[Hu et al., 2025]** *Automated Design of Agentic Systems (ADAS).* ICLR 2025.
Meta-agent searching the agent architecture space. Discovers novel building blocks and compositions. Roko's composable trait system provides the search space that ADAS-style optimization operates over. (See also [06-self-learning](./06-self-learning-systems.md).)

---

## 2026 Addition: DGM

**[DGM, 2026]** *Darwin Godel Machine: Open-Ended Evolution of Self-Improving Agents.* arXiv:2505.22954.
Open-ended self-improvement through evolutionary architecture search. Combines evolutionary search (variation) with self-verification (selection) to produce agents that improve their own architecture. Validates Roko's meta-learning loop where the learning process itself evolves, and directly informs the R04 meta-agent lineage scope.

**[Sakana AI, 2025]** *Darwin Godel Machine.* arXiv:2505.22954.
The same DGM paper from Sakana AI. Agents that can modify their own code and verify the improvement. Key constraint: self-improvement must be verifiable. Maps to Roko's Gate pipeline as the verification mechanism for architecture changes.

---

## Memetic Evolution

**[Dawkins, 1976]** *The Selfish Gene.* Oxford University Press.
Memes as units of cultural transmission. Knowledge entries are the agent ecosystem's memes -- they replicate, mutate, and compete for context window space.

**[Blackmore, 1999]** *The Meme Machine.* Oxford University Press.
Memetic evolution: cultural units compete for replication. Knowledge entries in the NeuroStore are memes competing for retrieval and confirmation.

**[Hull, 1988]** *Science as a Process.* University of Chicago Press.
Evolutionary epistemology: science as evolution with replicators (ideas) and interactors (scientists). Knowledge entries are replicators; agents are interactors.

---

## Formal Evolutionary Theory

**[Price, 1970]** *Selection and Covariance.* Nature, 227, 520--521.
Price equation: trait change = covariance between trait and fitness. Provides a framework for analyzing which knowledge entries persist -- entries correlated with task success are selected for.

**[Fisher, 1930]** *The Genetical Theory of Natural Selection.* Clarendon Press.
Fisher's fundamental theorem: rate of fitness increase equals genetic variance. Diversity drives improvement. Grounds the diversity maintenance mandate in EvoSkills.

**[Taylor & Jonker, 1978]** *Evolutionary Stable Strategies and Game Dynamics.* Mathematical Biosciences, 40(1--2), 145--156.
Replicator dynamics for strategy frequency changes. Grounds NeuroStore strategy selection: successful strategies replicate (higher confidence), unsuccessful ones decline (decay).

**[Wright, 1932]** *The Roles of Mutation, Inbreeding, Crossbreeding, and Selection in Evolution.* Proc. Sixth International Congress on Genetics, 1, 356--366.
Fitness landscapes. The Somatic Landscape in the Daimon is an affective analogy: an 8-dimensional k-d tree over strategy space where terrain encodes past outcomes.

**[Hinton & Nowlan, 1987]** *How Learning Can Guide Evolution.* Complex Systems, 1.
Learning smooths fitness landscapes. Individual task-level learning guides collective knowledge evolution by identifying promising search regions before evolutionary selection commits.

---

## Metacognitive Self-Improvement

**[Liu & van der Schaar, 2025]** *Truly Self-Improving Agents Require Intrinsic Metacognitive Learning.* ICML 2025. arXiv:2506.05109.
Intrinsic metacognition required for genuine self-improvement beyond surface-level prompt tuning. Validates the meta-cognition step in Roko's cognitive loop.

---

## Cross-References

- Lifecycle foundations: [00-lifecycle-and-finite-agency](./00-lifecycle-and-finite-agency.md)
- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Biological analogues: [05-biological-analogues](./05-biological-analogues.md)
