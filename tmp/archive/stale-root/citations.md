# Roko — Complete Citation Index

> Every academic citation, reference, and source across all Roko documentation, compiled into a single reference.

Generated from:
- **Part I**: 26 domain-specific reference files in `docs/21-references/` (260+ citations)
- **Part II**: Inline citations extracted from 60+ documentation files across all topic directories

---

## Table of Contents

### Part I: Reference Library (docs/21-references/)

- [00 — Lifecycle and Finite Agency](#00--lifecycle-and-finite-agency)
- [01 — Memory Consolidation](#01--memory-consolidation)
- [02 — Affective Computing](#02--affective-computing)
- [03 — Dreams and Offline Learning](#03--dreams-and-offline-learning)
- [04 — Coordination and Multi-Agent](#04--coordination-and-multi-agent)
- [05 — Biological Analogues](#05--biological-analogues)
- [06 — Self-Learning Systems](#06--self-learning-systems)
- [07 — Context Engineering](#07--context-engineering)
- [08 — Security and Provenance](#08--security-and-provenance)
- [09 — HDC and VSA](#09--hdc-and-vsa)
- [10 — Market Microstructure](#10--market-microstructure)
- [11 — Streaming Algorithms](#11--streaming-algorithms)
- [12 — Signal Processing](#12--signal-processing)
- [13 — Philosophy](#13--philosophy)
- [14 — Agent Harnesses and Tool Use](#14--agent-harnesses-and-tool-use)
- [15 — Cybernetics and VSM](#15--cybernetics-and-vsm)
- [16 — Active Inference](#16--active-inference)
- [17 — Process Reward Models](#17--process-reward-models)
- [18 — Collective Intelligence](#18--collective-intelligence)
- [19 — Regulatory Compliance](#19--regulatory-compliance)
- [20 — Cognitive Architectures](#20--cognitive-architectures)
- [21 — Mechanism Design](#21--mechanism-design)
- [22 — Protocol Standards](#22--protocol-standards)
- [23 — Generational and Evolutionary](#23--generational-and-evolutionary)
- [24 — 2025 Additions](#24--2025-additions)
- [25 — Research to Runtime](#25--research-to-runtime)

### Part II: Inline Citations from Documentation

- [BENCHMARKS.md](#benchmarksmd)
- [EXECUTIVE-SUMMARY.md](#executive-summarymd)
- [00-architecture/](#00-architecture)
- [01-orchestration/](#01-orchestration)
- [02-agents/](#02-agents)
- [03-composition/](#03-composition)
- [04-verification/](#04-verification)
- [05-learning/](#05-learning)
- [06-neuro/](#06-neuro)
- [07-conductor/](#07-conductor)
- [09-daimon/](#09-daimon)
- [10-dreams/](#10-dreams)
- [11-safety/](#11-safety)
- [12-interfaces/](#12-interfaces)
- [13-coordination/](#13-coordination)
- [14-identity-economy/](#14-identity-economy)
- [16-heartbeat/](#16-heartbeat)
- [17-lifecycle/](#17-lifecycle)
- [18-tools/](#18-tools)
- [19-deployment/](#19-deployment)

---


---

## Part I: Reference Library (docs/21-references/)

> These are the primary reference files containing 260+ academic citations across 26 research domains. Each citation includes full academic format and a *Grounds:* annotation linking the paper to the specific Roko subsystem it validates.

---

### 00 — Lifecycle and Finite Agency

# Lifecycle and Finite Agency

> Academic foundations for knowledge lifecycle management, resource-bounded cognition, and evolutionary computing as they apply to the Roko agent framework.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md`, `bardo-backup/prd/02-mortality/15-references.md`

> **Implementation**: Reference

---

## Abstract

This domain collects the research originally framed around agent mortality and finite lifespans, reframed for Roko's architecture where the relevant concepts are **knowledge lifecycle management**, **resource-bounded cognition**, and **evolutionary skill evolution**. The core insight remains valid: systems without resource constraints, decay mechanisms, or turnover exhibit stagnation, plasticity loss, and technical debt accumulation. Roko applies these findings to knowledge decay (Ebbinghaus half-lives on Engrams), budget-driven urgency (economic pressure replaces economic "death"), and evolutionary skill libraries (EvoSkills). The mortality-specific narrative is removed; the empirical findings and mechanisms are preserved in full.

The original source (`02-mortality/14-research-foundations.md`) contained 130+ papers across mortality modeling, memory, affect, coordination, self-learning, security, generational learning, and context engineering. Citations from domains with their own sub-docs (memory, affect, dreams, etc.) appear in those dedicated sub-docs and are cross-referenced here. This sub-doc retains the citations most directly related to lifecycle, finite agency, and evolutionary computing.

---

## Evolutionary Computing and Digital Life

- Ray, T.S. (1991). An Approach to the Synthesis of Life. _Artificial Life II_, Addison-Wesley, 1992.
  *Grounds: Roko's evolutionary skill evolution — Ray's Tierra demonstrated that digital evolution halts without a reaper mechanism. 300+ genotypes emerged only when entities had finite lifespans. For Roko, this motivates knowledge decay: without controlled forgetting, knowledge stores calcify.*

- Lenski, R.E. et al. (2003). The Evolutionary Origin of Complex Features. _PNAS_, 100(9).
  *Grounds: Roko knowledge distillation — Lenski's Long-Term Evolution Experiment (LTEE) showed complex features require generational turnover. For Roko, lossy knowledge compression through tier promotion (Episode → Insight → Heuristic → Playbook) is the mechanism that produces generalization.*

- Vostinar, A.E. et al. (2019). Suicidal selection: Programmed cell death evolves as adaptive behavior under spatial structure. _Evolution_, 73(5).
  *Grounds: Knowledge pruning — programmed elimination is selected for, not against, under spatial structure. Validates the Curator's active pruning of low-confidence knowledge entries.*

- Wensink, M.J. et al. (2020). Death and Progress. _Evolutionary Biology_, 47(4).
  *Grounds: Knowledge lifecycle calibration — intrinsic mortality prevents premature convergence. An optimal rate exists that balances stagnation and knowledge loss. For Roko, this maps to calibrating Ebbinghaus half-life parameters per knowledge type.*

- Werfel, J. et al. (2017). How Short-Lived Agents Can Collectively Build Long-Lived Structures. _Artificial Life_, 23(3).
  *Grounds: Collective knowledge persistence — natural selection directly favors shorter lifespans under spatial resource competition. For Roko, individual knowledge entries decay but the collective NeuroStore persists.*

---

## Plasticity, Continual Learning, and Drift

- Dohare, S. et al. (2024). Loss of Plasticity in Deep Continual Learning. _Nature_, 632.
  *Grounds: Knowledge tier demotion — 90% of units become "dead" (non-updating) in continual learning systems. Periodic replacement outperforms continuous adaptation. For Roko, this validates knowledge decay and the Curator's pruning cycle.*

- Vela, B. et al. (2022). Temporal quality degradation in AI models. _Journal of Data and Information Quality_, 14(1).
  *Grounds: Knowledge freshness tracking — 91% of ML models degrade temporally in production. For Roko, this validates confidence decay on knowledge entries (epistemic pressure replaces "epistemic death clock").*

- Sculley, D. et al. (2015). Hidden Technical Debt in Machine Learning Systems. _NeurIPS_.
  *Grounds: Knowledge maintenance — technical debt compounds silently in long-running systems. For Roko, controlled knowledge decay is the mechanism that prevents silent accumulation of stale heuristics.*

- Arbesman, S. (2012). _The Half-Life of Facts: Why Everything We Know Has an Expiration Date_. Current/Penguin.
  *Grounds: Knowledge demurrage calibration — factual knowledge decays at measurable rates across domains. Directly informs the per-type half-life settings in NeuroStore (Insights: 7d, Heuristics: 14d, Warnings: 30d).*

---

## Resource-Bounded Cognition

- Ord, T. (2025). Agent success rates decay exponentially with task duration. Working paper.
  *Grounds: Task scheduling — constant hazard rate means periodic checkpointing and task decomposition are more reliable than unbounded execution. Motivates the plan-execute-gate-persist loop.*

- Sims, C.A. (2003). Implications of Rational Inattention. _Journal of Monetary Economics_, 50(3), 665-690.
  *Grounds: Context budget allocation — rational finite-capacity agents optimally ignore some information. Budget pressure shapes attention allocation. For Roko, this motivates the VCG attention auction and context budget constraints.*

- Orseau, L. & Ring, M. (2011). Self-Modification and Mortality in Artificial Agents. _AGI_, 2011.
  *Grounds: Goal-directed agent design — RL agents under survival pressure treat survival as sole goal (pathological). Roko agents must be goal-directed with external objectives, not self-preservation optimizers.*

- Orseau, L. & Armstrong, S. (2016). Safely Interruptible Agents. _UAI_, 2016.
  *Grounds: Agent lifecycle management — Q-learning agents can be safely interruptible using off-policy learning, preventing agents from learning to avoid or seek interruptions. Informs Roko's graceful shutdown protocol.*

---

## Cooperation Under Resource Constraints

- Kreps, D., Milgrom, P., Roberts, J., & Wilson, R. (1982). Rational Cooperation in the Finitely Repeated Prisoners' Dilemma. _Journal of Economic Theory_, 27(2), 245-252.
  *Grounds: Multi-agent coordination — uncertain finite horizons promote cooperation. Even tiny uncertainty about endpoint breaks backward induction and makes cooperation rational. For Roko collectives, budget uncertainty serves this function.*

- Ohtsuki, H. et al. (2006). A simple rule for the evolution of cooperation on graphs and social networks. _Nature_, 441, 502-505.
  *Grounds: Collective mesh dynamics — death-birth updating (resource reallocation) favors cooperators over defectors in spatial games. For Roko, knowledge turnover within collectives promotes cooperative knowledge sharing.*

- Nakamaru, M., Matsuda, H., & Iwasa, Y. (1997-1998). The Evolution of Cooperation in a Lattice-Structured Population. _Journal of Theoretical Biology_.
  *Grounds: Collective knowledge dynamics — mortality-based selection promotes cooperation over fertility-based selection. Knowledge turnover (decay + replenishment) outperforms pure accumulation for collective intelligence.*

- Smith, J.M. (1992). Byte-sized evolution. _Nature_, 355, 772-773.
  *Grounds: Collective persistence — mortal individuals in immortal lineages sustain cooperation through generations. Individual knowledge entries decay; the collective NeuroStore persists indefinitely.*

---

## Compression as Regularization

- Shuvaev, S. et al. (2024). Encoding Innate Ability Through a Genomic Bottleneck. _PNAS_, 121(39).
  *Grounds: Knowledge distillation — the genome is ~1000x smaller than the information needed for brain connectivity, yet organisms have innate behaviors. Compression IS the regularizer. For Roko, tier promotion (Episode → Insight → Heuristic → Playbook) forces generalization.*

- Hinton, G.E. (2022). The Forward-Forward Algorithm: Some Preliminary Investigations. Working paper.
  *Grounds: Architectural design — software-hardware separation limits intelligence; coupling processing to substrate produces emergent capabilities. For Roko, this motivates tight integration between the agent runtime and its knowledge substrate.*

- Ororbia, A. & Friston, K. (2023). Mortal Computation. Working paper.
  *Grounds: Agent architecture — mortal computation binds processing to lifecycle. For Roko, intelligence is inseparable from the resource substrate (budget, compute, context window).*

---

## Generational Learning and Cultural Evolution

- Baldwin, J.M. (1896). A New Factor in Evolution. _American Naturalist_, 30, 441-451.
  *Grounds: Playbook evolution — the Baldwin Effect: learned behavior becomes innate across generations under selection pressure. For Roko, heuristics validated across 3+ plan executions promote to structural defaults in PLAYBOOK.md.*

- Heard, E. & Martienssen, R.A. (2014). Transgenerational Epigenetic Inheritance: Myths and Mechanisms. _Cell_, 157(1), 95-109.
  *Grounds: Knowledge transfer confidence decay — most transgenerational epigenetic inheritance is deleterious; barriers evolved to prevent it. For Roko, inherited knowledge entries receive 0.85× confidence multiplier per transfer cycle (Weismann Barrier).*

- Bhatt, S. et al. (2023). Few-shot imitation as cultural transmission. Working paper.
  *Grounds: Collective knowledge transfer — few-shot imitation produces cumulative learning across agent populations. For Roko, mesh-based knowledge sharing enables cross-agent learning without direct agent-to-agent coordination.*

- Bourahla, M. et al. (2022). Vertical transmission enables agents to exceed performance ceilings. Working paper.
  *Grounds: Knowledge backup/restore — inter-generational (vertical) knowledge transfer enables agents to exceed individual performance ceilings. For Roko, NeuroStore backup/restore provides this without biological metaphors.*

- Perez, E. et al. (2024). Pure imitation leads to stagnation. _AGI_, 2024.
  *Grounds: Knowledge diversification — pure imitation without novelty injection leads to stagnation. For Roko, the anti-proletarianization mandate ensures new agents diverge from inherited knowledge.*

- Martin, J., Everitt, T., & Hutter, M. (2016). Death and Suicide in Universal Artificial Intelligence. _AGI_, 2016.
  *Grounds: Knowledge transfer completeness — RL agents learning only from survival histories develop systematic overconfidence. For Roko, knowledge transfer includes failures and negative examples (AntiKnowledge), not just successes.*

- Gerstgrasser, M. et al. (2023). SUPER: Surprise-based Experience Sharing. Working paper.
  *Grounds: Knowledge sharing prioritization — rank shared knowledge by novelty relative to recipient. For Roko, mesh-based knowledge exchange prioritizes entries that are novel to the receiving agent.*

---

## Biological Analogues (Historical Reference)

These citations are preserved for historical completeness. They were originally framed as direct analogues for agent mortality; in Roko they serve as inspirational references for lifecycle management patterns.

- Hayflick, L. (1961). The Serial Cultivation of Human Diploid Cell Strains. _Experimental Cell Research_, 25(3), 585-621.
  *Historical reference: Replicative senescence after ~60 divisions. For Roko, this historical finding inspired the concept of knowledge freshness tracking — not agent death, but knowledge tier demotion when entries are no longer validated.*

- Kirkwood, T.B.L. (1977). Evolution of Ageing. _Nature_, 270, 301-304.
  *Historical reference: Disposable soma theory — investment in self-repair decreases with age. For Roko, this maps to budget allocation: as remaining budget decreases, agents shift from exploration to consolidation (not from "learning to legacy").*

- Hanahan, D. & Weinberg, R.A. (2000, 2011). The Hallmarks of Cancer. _Cell_, 100(1) & 144(5).
  *Historical reference: Cancer hallmarks include resisting cell death and enabling replicative immortality. For Roko, unbounded growth without pruning is the analogue — systems that resist knowledge decay accumulate stale entries.*

- Skulachev, V.P. (1999). Phenoptosis: Programmed Death of an Organism. _Biochemistry (Moscow)_, 64(12).
  *Historical reference: Programmed death operates at cellular, organism, and colony levels. For Roko, this fractal pattern maps to knowledge pruning at entry level, store level, and collective level.*

- Ramsdell, F. & Fowlkes, B.J. (1990). Clonal Deletion versus Clonal Anergy: The Role of the Thymus in Inducing Self Tolerance. _Science_, 248(4961).
  *Historical reference: 95-98% of thymocytes die during T-cell development. For Roko, aggressive pruning of candidate knowledge entries produces a collectively intelligent knowledge repertoire.*

- Simard, S.W. (2012). Mycorrhizal networks facilitate tree communication, learning, and memory. In _Memory and Learning in Plants_, Springer.
  *Historical reference: Mycorrhizal networks share resources and defense signals between trees. For Roko, the Agent Mesh as a knowledge relay mirrors this proven biological coordination mechanism.*

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for memory-specific citations (McClelland 1995, Ebbinghaus 1885, etc.)
- See [06-self-learning-systems.md](./06-self-learning-systems.md) for Reflexion, ExpeL, Voyager, and DSPy
- See [23-generational-and-evolutionary.md](./23-generational-and-evolutionary.md) for Ray 1991 and Lenski LTEE in the evolutionary computing context
- See topic [17-lifecycle](../17-lifecycle/INDEX.md) for how these citations ground Roko's agent lifecycle design


---

### 01 — Memory Consolidation

# Memory Consolidation

> Academic foundations for memory systems, forgetting as optimization, and knowledge tier progression in the Roko framework.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Neuro](../06-neuro/INDEX.md)
**Key sources**: `bardo-backup/prd/04-memory/10-research.md`, `bardo-backup/prd/02-mortality/14-research-foundations.md`

> **Implementation**: Reference

---

## Abstract

Memory consolidation research provides the theoretical foundation for Roko's NeuroStore — the knowledge management subsystem that persists insights, heuristics, and warnings across tasks. The central finding across neuroscience, cognitive psychology, and AI memory systems is that **forgetting is not failure; it is regularization**. Active forgetting prevents overfitting to stale information, and selective consolidation (prioritized replay, spaced retrieval, CLS dual-store) produces more robust knowledge than naive accumulation. Every design decision in the NeuroStore traces to published research indexed here.

---

## Complementary Learning Systems

- McClelland, J.L., McNaughton, B.L., & O'Reilly, R.C. (1995). Why There Are Complementary Learning Systems in the Hippocampus and Neocortex: Insights from the Successes and Failures of Connectionist Models of Learning and Memory. _Psychological Review_, 102(3), 419-457.
  *Grounds: NeuroStore dual-store architecture — CLS theory establishes that fast episodic memory (hippocampus) consolidates into slow semantic memory (neocortex) during offline periods. Roko's episodic log → knowledge tier promotion implements this dual-system pattern.*

- Kumaran, D., Hassabis, D., & McClelland, J.L. (2016). What Learning Systems do Intelligent Agents Need? Complementary Learning Systems Theory Updated. _Trends in Cognitive Sciences_, 20(7), 512-534.
  *Grounds: Prioritized consolidation — updated CLS showing replay scheduling matters: high-surprise episodes should be replayed more often. Grounds prioritized consolidation replay in Dreams.*

- O'Reilly, R.C., Bhatt, M.A., & Russin, J.L. (2014). Complementary Learning Systems. _Cognitive Science_, 38(Suppl 1).
  *Grounds: Pattern separation and completion — hippocampal pattern separation and neocortical pattern completion as complementary operations. Grounds the episodic/semantic store duality.*

- McCloskey, M. & Cohen, N.J. (1989). Catastrophic Interference in Connectionist Networks: The Sequential Learning Problem. _Psychology of Learning and Motivation_, 24, 109-165.
  *Grounds: Anti-catastrophic-forgetting — the fundamental constraint that makes interleaved replay necessary. Grounds the NREM replay mechanism in Dreams.*

---

## Forgetting as Optimization

- Richards, B.A. & Frankland, P.W. (2017). The Persistence and Transience of Memory. _Neuron_, 94(6), 1071-1084.
  *Grounds: NeuroStore decay architecture — reframes forgetting as optimization. Memory pruning equals L1 regularization. Foundational for the entire Ebbinghaus-based decay system.*

- Ebbinghaus, H. (1885). _Über das Gedächtnis_ (Memory: A Contribution to Experimental Psychology). Leipzig.
  *Grounds: Half-life calibration — the forgetting curve establishes negative exponential decay. Retrieval slows decay. Directly implemented as per-type decay rates (episodes 48h, insights 7d, heuristics 14d, warnings 30d).*

- Davis, R.L. & Zhong, Y. (2017). The Biology of Forgetting — A Perspective. _Neuron_, 95(3), 490-503.
  *Grounds: Active forgetting mechanisms — active forgetting is metabolically expensive, proving it serves a function. Forgetting is selected for; knowledge demurrage encodes this.*

- Hardt, O., Nader, K., & Nadel, L. (2013). Decay Happens: The Role of Active Forgetting in Memory. _Trends in Cognitive Sciences_, 17(3), 111-120.
  *Grounds: Curator pruning operations — active molecular forgetting processes. Grounds the Curator's DOWNVOTE and pruning operations.*

- Nader, K. et al. (2000). Fear Memories Require Protein Synthesis in the Amygdala for Reconsolidation after Retrieval. _Nature_, 406, 722-726.
  *Grounds: Confidence-update-on-retrieval — reconsolidation theory: retrieved memories become labile and can be updated. Justifies updating knowledge entry confidence on each retrieval.*

---

## Retrieval and Spacing Effects

- Roediger, H.L. & Karpicke, J.D. (2006). Test-Enhanced Learning: Taking Memory Tests Improves Long-Term Retention. _Psychological Science_, 17(3), 249-255.
  *Grounds: NeuroStore retrieval boost — retrieval strengthens memory traces more than re-study (+200% recall vs passive review). Retrieved entries decay slower than unretrieved ones. Grounds the strength-increment-on-positive-outcome mechanism.*

- Cepeda, N.J. et al. (2006). Distributed Practice in Verbal Recall Tasks: A Review and Quantitative Synthesis. _Psychological Bulletin_, 132(3), 354-380.
  *Grounds: Curator cycle timing — spaced retrieval produces more durable memories than massed retrieval. Optimal interval is non-trivial. Grounds the 50-tick Curator cycle interval.*

---

## Prioritized Replay

- Wilson, M.A. & McNaughton, B.L. (1994). Reactivation of Hippocampal Ensemble Memories During Sleep. _Science_, 265(5172), 676-679.
  *Grounds: NREM replay — hippocampal replay during sleep consolidates memories via temporal sequence replay. Directly implements prioritized experience replay in the Dreams subsystem.*

- Mattar, M.G. & Daw, N.D. (2018). Prioritized Memory Access Explains Planning and Hippocampal Replay. _Nature Neuroscience_, 21(11), 1609-1617.
  *Grounds: Dreams replay utility function — Utility = gain × need for replay selection. The foundational algorithm for selecting which episodes to replay during Delta-frequency consolidation.*

- Schaul, T. et al. (2016). Prioritized Experience Replay. _ICLR_, 2016. arXiv:1511.05952.
  *Grounds: Surprise-weighted replay — priority proportional to TD error magnitude. Grounds the surprise-weighted replay candidate selection in the Dreams consolidation engine.*

---

## Sleep-Dependent Consolidation

- Stickgold, R. (2005). Sleep-Dependent Memory Consolidation. _Nature_, 437, 1272-1278.
  *Grounds: Delta-frequency consolidation — sleep replay strengthens memory traces selectively. Grounds NREM replay selection in dream processing.*

- Born, J. & Wilhelm, I. (2012). System Consolidation of Memory During Sleep. _Psychological Research_, 76, 192-203.
  *Grounds: Curator selective promotion — sleep-dependent consolidation is biased toward future-relevant memories. Maps to the Curator cycle's selective tier promotion.*

- Tononi, G. & Cirelli, C. (2014). Sleep and the Price of Plasticity: From Synaptic and Cellular Homeostasis to Memory Consolidation and Integration. _Neuron_, 81(1), 12-34.
  *Grounds: Knowledge pruning as synaptic homeostasis — the synaptic homeostasis hypothesis: sleep prunes weak connections, strengthening important ones. Maps to knowledge decay during idle periods.*

---

## Agent Memory Systems

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.
  *Grounds: Four-factor retrieval scoring — three-factor retrieval (recency, importance, relevance) with emergent social behaviors. Roko extends to four factors by adding emotional congruence. Ablation shows removing any single factor causes behavioral degeneration.*

- Chhikara, P. et al. (2025). Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory. arXiv:2504.19413.
  *Grounds: Curator dual-pass architecture — two-phase extraction-update pipeline: +26% accuracy, 91% lower p95 latency, 90% token savings. Validates the tiered local NeuroStore architecture.*

- Xu, W. et al. (2025). A-MEM: Agentic Memory for LLM Agents. arXiv:2502.12110.
  *Grounds: NeuroStore entry structure — Zettelkasten-inspired atomic notes with dynamic links: 85-93% token reduction, 2× multi-hop reasoning improvement at ~$0.0003 per operation. Grounds the bi-temporal metadata fields.*

- Anokhin, P. et al. (2024). AriGraph: Learning Knowledge Graph World Models with Episodic Memory for LLM Agents. arXiv:2407.04363.
  *Grounds: Causal graph architecture — semantic + episodic integration into knowledge graph outperforms pure vector retrieval. Grounds causal link entries in the NeuroStore.*

- Packer, C. et al. (2023). MemGPT: Towards LLMs as Operating Systems. arXiv:2310.08560.
  *Grounds: Context block architecture — structured memory blocks with clear labels improve LLM reasoning by 30-60%. Grounds the typed ContextBlock architecture in roko-compose.*

- Zhong, W. et al. (2024). MemoryBank: Enhancing Large Language Models with Long-Term Memory. _AAAI_, 2024.
  *Grounds: Persistent knowledge store design — long-term memory augmentation for LLMs. Grounds persistent NeuroStore design.*

- arXiv:2505.16067 (2025). On the Self-Degradation of Agent Memory.
  *Grounds: Quality gate on episodic consolidation — naive add-all memory consistently degrades agent performance. Incorrect past executions propagate through retrieval. Grounds the mark_verified quality gate.*

---

## Agent Memory Surveys and Taxonomies (2025)

- Liu, S. et al. (2025). Memory in the Age of AI Agents: A Survey. arXiv:2512.13564.
  *Grounds: Agent memory taxonomy — finer-grained taxonomy distinguishing factual, experiential, and working memory. Analyzes how memory is formed, evolved, and retrieved over time. Validates NeuroStore's multi-type knowledge architecture.*

- Wang, Z. et al. (2025). Rethinking Memory in LLM-based Agents: Representations, Operations, and Emerging Topics. arXiv:2505.00675.
  *Grounds: Memory operations taxonomy — formalizes six core operations: Consolidation, Updating, Indexing, Forgetting, Retrieval, and Condensation. Directly maps to NeuroStore's Curator cycle (consolidation, indexing, pruning, retrieval).*

- Memory for Autonomous LLM Agents (2026). Mechanisms, Evaluation, and Emerging Frontiers. arXiv:2603.07670.
  *Grounds: Agent memory formalization — formalizes agent memory as a write-manage-read loop coupled with perception and action. Five mechanism families: context-resident compression, retrieval-augmented stores, reflective self-improvement, hierarchical virtual context, and policy-learned management. Validates Roko's separation of memory management from inference.*

- Yehudai, M. et al. (2025). Human-Like Remembering and Forgetting in LLM Agents: An ACT-R-Inspired Memory Architecture. _HAI_, 2025.
  *Grounds: ACT-R memory for agents — implements ACT-R decay and retrieval functions in LLM agent memory. Validates the Ebbinghaus-based decay and retrieval-boost mechanisms in NeuroStore with a complementary cognitive-science-grounded approach.*

---

## Knowledge Compression and Transfer

- Bartlett, F.C. (1932). _Remembering: A Study in Experimental and Social Psychology_. Cambridge University Press.
  *Grounds: Knowledge integration protocol — new information must be assimilated into existing schemas; raw transplant fails. Inherited knowledge must be integrated, not transplanted directly.*

---

## Cross-References

- See [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md) for lifecycle-level forgetting motivation
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for sleep-dependent consolidation mechanisms
- See topic [06-neuro](../06-neuro/INDEX.md) for how these citations ground the NeuroStore implementation
- See topic [05-learning](../05-learning/INDEX.md) for the learning loop architecture


---

### 02 — Affective Computing

# Affective Computing

> Academic foundations for emotion modeling, somatic markers, and affect-modulated cognition in the Roko Daimon subsystem.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Daimon](../09-daimon/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §3, `bardo-backup/prd/shared/citations.md` §28

> **Implementation**: Reference

---

## Abstract

The Daimon is not cosmetic. Five independent research lines — somatic markers, mood-congruent retrieval, exploration modulation, narrative transfer, and empirical trading results — converge on the conclusion that emotion-like states serve genuine computational functions that cognition alone cannot replicate. The PAD (Pleasure-Arousal-Dominance) vector, the ALMA three-layer temporal model, the Somatic Landscape, and affect-modulated retrieval are all grounded in the citations below.

---

## PAD Emotional State Model

- Mehrabian, A. (1996). Pleasure-Arousal-Dominance: A General Framework for Describing and Measuring Individual Differences in Temperament. _Current Psychology_, 14(4), 261-292.
  *Grounds: Daimon PAD vector — foundational PAD emotional state model. Three continuous dimensions capture more variance than discrete emotion labels. Roko uses: Pleasure = task success, Arousal = urgency/load, Dominance = confidence.*

- Russell, J.A. & Mehrabian, A. (1977). Evidence for a Three-Factor Theory of Emotions. _Journal of Research in Personality_, 11(3), 273-294.
  *Grounds: PAD validation — three-dimensional affect captures emotional experience more completely than discrete categories. PAD octants map to behavioral tendencies.*

---

## Somatic Marker Hypothesis

- Damasio, A.R. (1994). _Descartes' Error: Emotion, Reason, and the Human Brain_. Putnam.
  *Grounds: Somatic Landscape — patients without emotion make consistently worse decisions under uncertainty. Somatic markers bias choices before deliberation. Roko implements this as a k-d tree over 8-dimensional strategy space.*

- Bechara, A., Damasio, H., & Damasio, A.R. (2000). Emotion, Decision Making and the Orbitofrontal Cortex. _Cerebral Cortex_, 10(3), 295-307.
  *Grounds: Pre-cognitive gut feelings — anticipatory SCRs precede conscious awareness in the Iowa Gambling Task. Roko's SomaticLandscape provides fast heuristic feelings about strategy regions before analytical reasoning begins.*

- Bechara, A. & Damasio, A.R. (2005). The Somatic Marker Hypothesis: A Neural Theory of Economic Decision. _Games and Economic Behavior_, 52, 336-372.
  *Grounds: Economic decision-making — formal integration of somatic marker theory with economic decision theory. Validates PAD-modulated tier routing for cost-sensitive agent decisions.*

- Cabrera-Paniagua, D. & Rubilar-Torrealba, R. (2023). Autonomous Stock Market Agents with Somatic Markers. _Journal of Ambient Intelligence and Humanized Computing_.
  *Grounds: Empirical validation — agents with somatic markers achieve higher Sharpe ratios on S&P 500 and Dow Jones benchmarks. Punishment signals triggered by drawdowns adaptively reduced position sizes. Directly validates somatic markers for autonomous agents.*

---

## Mood-Congruent Memory and Emotional Retrieval

- Bower, G.H. (1981). Mood and Memory. _American Psychologist_, 36(2), 129-148.
  *Grounds: Four-factor retrieval emotional congruence — emotional states bias memory retrieval via associative network activation. Directly implemented as the emotional factor (0.15 weight) in four-factor retrieval scoring. Also motivates the mandatory 15% contrarian retrieval to prevent echo chambers.*

- Blaney, P.H. (1986). Affect and Memory: A Review. _Psychological Bulletin_, 99(2), 229-246.
  *Grounds: Mood-congruent retrieval validation — comprehensive review confirming mood-congruent memory effects across multiple experimental paradigms. Supports emotional congruence as a retrieval factor.*

- Phelps, E.A. (2004). Human Emotion and Memory: Interactions of the Amygdala and Hippocampal Complex. _Current Opinion in Neurobiology_, 14(2), 198-202.
  *Grounds: Emotion-memory integration — emotion and memory as a single system with two interfaces. Grounds the emotional_tag field on every NeuroStore entry.*

- Cahill, L. & McGaugh, J.L. (1998). Mechanisms of Emotional Arousal and Lasting Declarative Memory. _Trends in Neurosciences_, 21(7), 294-299.
  *Grounds: Arousal encoding factor — arousal enhances consolidation via amygdala modulation. Grounds the arousal_encoding_factor() function in knowledge entry creation.*

---

## Emotion Classification and Appraisal

- Plutchik, R. (1980). _Emotion: A Psychoevolutionary Synthesis_. Harper & Row.
  *Grounds: Daimon emotion classification — eight primary emotions from evolutionary pressure. PAD octants map to Plutchik categories for human-readable emotional state labeling.*

- Scherer, K.R. (2001). Appraisal Considered as a Process of Multilevel Sequential Checking. In _Appraisal Processes in Emotion_, Oxford University Press.
  *Grounds: Engram Score appraisal theory — multi-level sequential checking mirrors the 7-axis Score on Engrams. Each axis captures an independent dimension of appraisal.*

- Ortony, A., Clore, G.L., & Collins, A. (1988). _The Cognitive Structure of Emotions_. Cambridge University Press.
  *Grounds: OCC emotion model — cognitive appraisal-based emotion taxonomy. Complements PAD with a structure-of-emotion framework for interpreting agent behavioral states.*

---

## Temporal Affect Models

- Gebhard, P. (2005). ALMA — A Layered Model of Affect. _AAMAS_, 2005.
  *Grounds: Three-layer temporal affect (ALMA) — emotion (seconds), mood (hours), personality (lifetime). Implemented in Roko as tick-level emotion, EMA mood, and static personality configuration.*

---

## Affect in Agent Systems

- Zhang, Y. et al. (2024). Self-Emotion Changes ~50% of Agent Decisions in Social Simulation. _SIGDIAL_, 2024.
  *Grounds: Daimon is architectural, not decorative — self-emotion changes approximately 50% of agent decisions in social simulation. Demonstrates that affect is a primary driver of agent behavior, not a secondary display layer.*

- Gadanho, S.C. (2003). Learning Behavior-Selection by Emotions and Cognition in a Multi-Goal Robot Task. _Journal of Machine Learning Research_, 4, 385-412.
  *Grounds: Combined affect-cognition architecture — ALEC (emotion + cognition) architecture: 40% fewer collisions vs cognition alone. Validates the combined Daimon + cognitive loop architecture.*

- Barthet, M. et al. (2022). Go-Blend: Affect-Driven Reinforcement Learning. _IEEE Transactions on Affective Computing_.
  *Grounds: Exploration temperature modulation — affect-driven RL improves exploration efficiency and agent performance. Arousal modulates exploration temperature in Roko's tier routing.*

- Seligman, M.E.P. (1972). Learned Helplessness. _Annual Review of Medicine_, 23, 407-412.
  *Grounds: Dominance alert — learned helplessness from uncontrollable negative outcomes. Dominance < -0.3 for 200+ ticks triggers an alert in the Daimon behavioral state machine.*

- Van den Broek, E. (2023). Emotion Contagion in Multi-Agent Systems. _Autonomous Agents and Multi-Agent Systems_.
  *Grounds: Arousal contagion dampening — anger spreads more competitively than other emotions. Contagion dampening required to prevent panic cascades. Arousal contagion is capped at +0.3 per sync cycle in Roko collectives.*

---

## Emotional Depotentiation

- Walker, M.P. & van der Helm, E. (2009). Overnight Therapy? The Role of Sleep in Emotional Brain Processing. _Psychological Bulletin_, 135(5), 731-748.
  *Grounds: REM emotional depotentiation — REM sleep depotentiates emotional charge while preserving informational content. Dream cycles reduce arousal on highly charged memories by 0.3-0.5 per cycle to prevent panic lock-in. Cross-referenced in [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

---

## Emotional RAG

- Zhang, Y. et al. (2024). Emotional RAG. arXiv:2410.23041.
  *Grounds: PAD vectors on knowledge entries — emotion-tagged retrieval significantly outperforms non-emotional retrieval across three datasets. Validates attaching PAD vectors to every NeuroStore entry.*

---

## Affective Computing Surveys and Frameworks (2025)

- Yin, Y. et al. (2025). Emotions in Artificial Intelligence. arXiv:2505.01462.
  *Grounds: Teleology-driven affect — proposes a teleology-driven affective computing framework unifying major emotion theories under the premise that affect is adaptive and goal-directed. Emphasizes aligning agent responses with both individual and collective well-being over extended timescales. Validates the Daimon as a goal-directed computational system, not a cosmetic overlay.*

- Intelligent Agents with Emotional Intelligence (2025). Current Trends, Challenges, and Future Prospects. arXiv:2511.20657.
  *Grounds: Emotionally intelligent agents — survey of agents with emotional intelligence identifying key challenges in development. Validates the architectural necessity of emotional intelligence in agent systems, supporting Roko's Daimon as an integral subsystem.*

- Emotions in the Loop (2025). A Survey of Affective Computing for Emotional Support. arXiv:2505.01542.
  *Grounds: Affect-in-the-loop — comprehensive survey of affective computing integrated into agent interaction loops. Validates Roko's approach of integrating affect (Daimon) into the cognitive loop rather than treating it as a post-hoc analysis layer.*

---

## Cross-References

- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for Walker & van der Helm emotional depotentiation in the dreams context
- See [01-memory-consolidation.md](./01-memory-consolidation.md) for mood-congruent retrieval in the memory context
- See topic [09-daimon](../09-daimon/INDEX.md) for the full Daimon subsystem design


---

### 03 — Dreams and Offline Learning

# Dreams and Offline Learning

> Academic foundations for offline consolidation, creative hypothesis generation, and sleep-time compute in the Roko Dreams subsystem.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Dreams](../10-dreams/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §4, `bardo-backup/prd/shared/citations.md` §28

> **Implementation**: Reference

---

## Abstract

The brain dedicates 25-33% of its runtime to a state that prevents environmental interaction — an enormous evolutionary cost that must confer proportional benefits. The benefits are well-characterized: memory consolidation, emotional depotentiation, counterfactual hypothesis generation, and catastrophic forgetting prevention. For Roko agents, idle periods are not waste — they are budget for offline cognitive work. The Dreams subsystem implements three-phase consolidation: NREM replay (Mattar-Daw prioritized access), REM imagination (Boden creativity modes + Pearl SCM counterfactuals + Walker emotional depotentiation), and integration staging.

---

## Hypnagogia and Creative Insight

- Lacaux, C., Andrillon, T., Arnulf, I., & Oudiette, D. (2021). Sleeping on a Problem: Catching the Creative Spark During Sleep Onset. _Science Advances_, 7(50), eabj5866.
  *Grounds: Hypnagogia engine — 83% of subjects discovered hidden rules during brief N1 (sleep onset) exposure vs. 30% staying awake. The foundational result for Roko's hypnagogia engine: the Alpha Convergence Problem (all agents converging to similar solutions) is solved by forcing divergent exploration during idle periods.*

- Lacaux, C. et al. (2024). Sleep Onset Is Not a One-Way Trip: A Comprehensive Review of the N1 Stage. _Trends in Neurosciences_, 47(4), 273-288.
  *Grounds: N1 stage dynamics — comprehensive review establishing the N1 stage as a unique cognitive state with creativity-enhancing properties distinct from both wakefulness and deeper sleep.*

- Haar Horowitz, A. et al. (2020). Dormio: A Targeted Dream Incubation Device. _Consciousness and Cognition_, 83, 102938.
  *Grounds: Targeted dream incubation — MIT Dormio project demonstrated targeted dream incubation at sleep onset. 43% creativity boost via controlled N1 intervention.*

- Haar Horowitz, A., Cunningham, T.J., Maes, P., & Stickgold, R. (2023). Targeted Dream Incubation at Sleep Onset Increases Post-Sleep Creativity. _Scientific Reports_, 13, 7319.
  *Grounds: Creativity boost validation — confirmed 43% creativity boost from targeted dream incubation in a controlled study. Validates the hypnagogia engine's targeted knowledge recombination approach.*

---

## World Models and Imagined Trajectories

- Hafner, D. et al. (2025). DreamerV3: Mastering Diverse Domains through World Models. Working paper.
  *Grounds: REM counterfactual generation — agents trained on imagined trajectories outperform across 150+ tasks. World model dreaming. For Roko, REM-phase consolidation generates synthetic scenarios from the causal model.*

- Ha, D. & Schmidhuber, J. (2018). World Models. arXiv:1803.10122.
  *Grounds: Dream-based training — controller trained entirely inside dreams achieves competitive performance. Demonstrates that dreaming multiplies learning from scarce experience. For Roko, idle-time knowledge recombination creates new hypotheses without real execution.*

---

## Sleep-Time Compute

- Lin, B. et al. (2025). Sleep-time Compute: Beyond Inference Scaling at Test-Time. arXiv:2504.13171.
  *Grounds: Delta-frequency scheduling — dual-agent architecture: Sleeper Agent precomputes during downtime, Serve Agent handles live interactions. ~5× test-time compute reduction, up to 18% accuracy gain. For Roko, dream cycles are sleep-time compute — agents that process experiences during low-activity periods execute fewer expensive T2 inference calls during active work.*

---

## Wake-Sleep Learning

- WSCL (2024). Wake-Sleep Consolidated Learning. arXiv:2401.08623.
  *Grounds: Three-phase dreaming — Complementary Learning Systems three-phase cycle (wake/NREM/REM): 38% reduction in catastrophic forgetting, 17.6% increase in zero-shot transfer. Directly grounds the three-phase dream architecture.*

---

## Replay and Experience Prioritization

- Wagner, U. et al. (2004). Sleep Inspires Insight. _Nature_, 427, 352-355.
  *Grounds: Dream-based insight generation — sleep is 2.6× more likely to produce insight on hidden rule problems. Dream cycles produce genuine insight, not just consolidation.*

- Zhao, J. et al. (2024). BTP: Towards Efficient and Reliable Experience Replay for LLM-Based Agents. arXiv:2410.12236.
  *Grounds: Replay prioritization — P2Value (Possibility and Pass-rate Prioritized Value) for prioritized experience replay in LLM agents. Dream replay prioritizes informative failures.*

- Wang, X. et al. (2024). Prioritized Generative Replay. arXiv:2410.18082.
  *Grounds: Synthetic scenario generation — conditional diffusion generates new transitions near high-value experience regions. REM creates synthetic scenarios, not just replays.*

- Van de Ven, G.M., Siegelmann, H.T., & Tolias, A.S. (2020). Brain-Inspired Replay for Continual Learning with Artificial Neural Networks. _Nature Communications_, 11, 4069.
  *Grounds: Continual learning via replay — brain-inspired replay prevents catastrophic forgetting in artificial neural networks. Validates experience replay as the primary anti-forgetting mechanism.*

---

## Creativity Theory

- Boden, M.A. (2004). _The Creative Mind: Myths and Mechanisms_. 2nd ed. Routledge.
  *Grounds: REM creativity modes — three creativity modes: exploratory (within existing rules), combinational (new combinations of existing elements), and transformational (changing the rules). Roko's REM phase implements combinational creativity via HDC recombination and transformational creativity via causal model intervention.*

---

## Causal Models for Counterfactuals

- Pearl, J. (2009). _Causality: Models, Reasoning, and Inference_. 2nd ed. Cambridge University Press.
  *Grounds: REM counterfactual generation — structural causal models enable counterfactual reasoning: "what would have happened if I had chosen differently?" The agent builds a causal graph of its domain, then generates counterfactuals by intervening on causal variables during REM dreaming. Cross-referenced in [01-memory-consolidation.md](./01-memory-consolidation.md).*

---

## Hauntology and Experiential Traces

- Derrida, J. (1993). _Specters of Marx: The State of the Debt, the Work of Mourning and the New International_. Routledge (English translation 1994).
  *Grounds: Hypnagogia engine uniqueness — each agent is "differently haunted" by its own experiential traces. Derrida's hauntology provides the theoretical frame for why recombining different experiential histories produces different creative outputs, solving the Alpha Convergence Problem.*

---

## Generative Agents and Reflection

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.
  *Grounds: Memory + reflection architecture — the reflection mechanism in Generative Agents (periodic synthesis of higher-order observations from raw episodes) maps to the Theta-frequency reflection cycle. Cross-referenced in [01-memory-consolidation.md](./01-memory-consolidation.md).*

---

## Sleep-Inspired Architectures for LLMs (2025)

- Language Models Need Sleep (2025). Learning to Self-Modify and Consolidate Memories. OpenReview.
  *Grounds: LLM sleep paradigm — introduces a sleep paradigm with two stages: Memory Consolidation (RL-based Knowledge Seeding for parameter expansion) and Dreaming (self-improvement via synthetic data generation). Directly validates Roko's NREM/REM two-phase dream architecture at the LLM level.*

- Tutuncuoglu, B.T. (2025). NeuroDream: A Sleep-Inspired Memory Consolidation Framework for Artificial Neural Networks. SSRN:5377250.
  *Grounds: Dream-phase framework — explicit dream phase where models disconnect from input and engage in internally generated simulations from stored latent embeddings. Up to 38% reduction in forgetting, 17.6% increase in zero-shot transfer. Validates Roko's REM counterfactual generation producing genuine knowledge recombination.*

- LightMem (2025). Lightweight and Efficient Memory-Augmented Generation. arXiv:2510.18866.
  *Grounds: Sleep-time memory update — offline consolidation procedure decouples memory management from online inference; up to 10.9% accuracy improvement with 117x token reduction. Validates Delta-frequency consolidation operating asynchronously from Gamma-frequency execution.*

- SleepGate (2025). Learning to Forget: Sleep-Inspired Memory Consolidation for Resolving Proactive Interference in LLMs. arXiv:2603.14517.
  *Grounds: Active learned forgetting — brain's solution to proactive interference is an active, learned curation process. Validates NeuroStore's Curator as an active forgetting agent, not a passive decay mechanism.*

- CosmoCore (2025). Affective Dream-Replay Reinforcement Learning for Code Generation. arXiv:2510.18895.
  *Grounds: Affect-modulated dreams — combines affective states with dream-replay RL for code generation. Validates the intersection of Daimon (affect) and Dreams (offline consolidation) subsystems.*

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for Wilson & McNaughton 1994, Mattar & Daw 2018, and other replay citations
- See [02-affective-computing.md](./02-affective-computing.md) for Walker & van der Helm 2009 emotional depotentiation
- See [13-philosophy.md](./13-philosophy.md) for Derrida 1993 in the philosophy context
- See topic [10-dreams](../10-dreams/INDEX.md) for full Dreams subsystem design


---

### 04 — Coordination and Multi-Agent

# Coordination and Multi-Agent Systems

> Academic foundations for stigmergic coordination, collective intelligence, multi-agent cooperation, and mesh-based knowledge sharing in Roko's Agent Mesh and Pheromone Field subsystems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Agent Mesh](../13-coordination/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §5, `bardo-backup/prd/shared/citations.md` §22, `bardo-backup/tmp/agent-chain/08-references.md`

> **Implementation**: Reference

---

## Abstract

Agents do not operate in isolation. A group of agents owned by one user is a Collective; the intelligence of a Collective is the product of cooperation mechanisms. The research here establishes why anonymous stigmergic coordination (Pheromone Field) is superior to explicit messaging for large agent populations, why cooperation mechanisms require careful design, and how the Agent Mesh coordinates knowledge flow across distributed agents. The mycorrhizal network parallel (Simard 2012) is structural — the Mesh's architecture as an underground relay mirrors a proven biological coordination mechanism.

---

## Stigmergy: Coordination Through Environmental Traces

- Grassé, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp. _Insectes Sociaux_, 6(1), 41-80.
  *Grounds: Pheromone Field — coined "stigmergy" (stigma = mark, ergon = work). Termites coordinate mound construction without direct communication — each responds to the current state of the environment. Foundational concept for the Agent Mesh as pheromone field.*

- Theraulaz, G. & Bonabeau, E. (1999). A Brief History of Stigmergy. _Artificial Life_, 5(2), 97-116.
  *Grounds: Pheromone decay — distinguishes sematectonic stigmergy (building on structures) from marker-based stigmergy (depositing decaying signals). Agent Engrams are marker-based pheromone deposits that decay via Ebbinghaus half-life.*

- Parunak, H.V.D., Brueckner, S., & Sauter, J. (2002). Digital Pheromone Mechanisms for Coordination of Unmanned Vehicles. _AAMAS_, 2002.
  *Grounds: Time-decaying digital signals — digital pheromones enable emergent coordination through time-decaying signals reinforced by confirmation. Directly grounds the pheromone Engram decay and reinforcement mechanism.*

- Dorigo, M. & Gambardella, L.M. (1997). Ant Colony System: A Cooperative Learning Approach to the Traveling Salesman Problem. _IEEE Transactions on Evolutionary Computation_, 1(1), 53-66.
  *Grounds: Pheromone reinforcement — ant colony optimization formalizes pheromone deposit/evaporation. Confirmed knowledge entries gain weight (reinforcement); unconfirmed entries decay (evaporation). Grounds the confirmation-based half-life extension in NeuroStore.*

- Xuan, L. et al. (2026). Dual-Trail Stigmergic Coordination for Multi-Agent Systems. _Journal of Marine Science and Engineering_, 14(2).
  *Grounds: Multi-trail stigmergy — dual-trail pheromone systems for coordinating multiple agent groups. Validates separate pheromone types (Threat/Opportunity/Wisdom) with distinct decay profiles.*

- Xu, Z. et al. (2024). Stigmergy + Independent RL + Conflict-Avoidance Achieves Emergent Coordination. 2024.
  *Grounds: Pheromone Field design principles — combining stigmergic signals with independent learning and conflict avoidance produces emergent coordination without central control.*

---

## Cooperation and Game Theory

- Kreps, D.M., Milgrom, P., Roberts, J., & Wilson, R. (1982). Rational Cooperation in the Finitely Repeated Prisoners' Dilemma. _Journal of Economic Theory_, 27(2), 245-252.
  *Grounds: Cooperation under uncertainty — uncertain finite horizons promote cooperation; tiny uncertainty breaks backward induction. Resource-constrained agents find cooperation rational when the horizon is uncertain.*

- Ohtsuki, H. et al. (2006). A Simple Rule for the Evolution of Cooperation on Graphs and Social Networks. _Nature_, 441, 502-505.
  *Grounds: Spatial cooperation — updating favors cooperators over defectors in spatial games. Agent replacement in meshes produces cooperative equilibria.*

- Smith, J.M. (1992). Byte-Sized Evolution. _Nature_, 355, 772-773.
  *Grounds: Collective cooperation — individuals in persistent lineages sustain cooperation through generations. Resource-constrained agents, persistent Collectives.*

- Nakamaru, M. et al. (1997-1998). The Evolution of Cooperation in a Lattice-Structured Population. _Journal of Theoretical Biology_.
  *Grounds: Selection for cooperation — selection based on resource efficiency promotes cooperation over reproduction-based growth in lattice populations.*

- Grossman, S.J. & Stiglitz, J.E. (1980). On the Impossibility of Informationally Efficient Markets. _American Economic Review_, 70(3), 393-408.
  *Grounds: Information sharing strategy — freely shared information is immediately priced in; no advantage without information asymmetry. Agents share threats and structural knowledge, not alpha signals.*

- Fontana, M. et al. (2024). Nicer Than Humans: How Do LLMs Behave in the Prisoner's Dilemma? arXiv:2406.13605v2.
  *Grounds: LLM cooperation behavior — LLMs exhibit cooperation patterns in game-theoretic settings. Informs the design of inter-agent negotiation protocols.*

- Rossetti, G. et al. (2025). Dynamics of Cooperation in Concurrent Games. _Nature Communications_, 16.
  *Grounds: Concurrent cooperation dynamics — cooperation dynamics in concurrent (not sequential) game settings. Validates mesh synchronization where agents act concurrently.*

---

## Emotion Contagion in Multi-Agent Systems

- Van den Broek, E. (2023). Emotion Contagion in Multi-Agent Systems. _Autonomous Agents and Multi-Agent Systems_.
  *Grounds: Arousal contagion dampening — anger spreads more competitively than other emotions. Arousal contagion is capped at +0.3 per sync cycle in Roko Collectives to prevent panic cascades. Cross-referenced in [02-affective-computing.md](./02-affective-computing.md).*

---

## Mycorrhizal Knowledge Networks

- Simard, S.W. (2012). Mycorrhizal Networks and Seedling Establishment in Douglas-Fir Forests. In _New Forests: Biology, Biotechnology, and Systems Genetics_.
  *Grounds: Agent Mesh topology — mycorrhizal networks share carbon, nutrients, and defense signals between trees without direct communication. The Agent Mesh mirrors this fungal-style underground relay architecture.*

---

## Agent Coordination Protocols

- Google (2025). Agent-to-Agent (A2A) Protocol Specification. 2025.
  *Grounds: Inter-agent protocol — standardized agent-to-agent communication protocol. Informs the Agent Mesh wire protocol design.*

- Anthropic (2024). Model Context Protocol (MCP) Specification. 2024.
  *Grounds: Tool protocol — standardized protocol for model-tool interaction. Roko's MCP client in `roko-agent` implements this for tool dispatch.*

---

## Communitas and Shared Obligation

- Esposito, R. (2010). _Communitas: The Origin and Destiny of Community_. Stanford University Press.
  *Grounds: Collective formation — community constituted by shared obligation to give. Reframed: agents in a Collective share knowledge as a structural obligation, not a death ritual. The munus (shared gift) is knowledge contribution.*

- Esposito, R. (2011). _Immunitas: The Protection and Negation of Life_. Polity.
  *Grounds: Knowledge boundaries — immunitas as the protective boundary that preserves individual agent integrity within the collective. Grounds the permissioned subnet architecture.*

---

## Emergent Coordination in LLM Agents (2025)

- Emergence in Multi-Agent Language Models (2025). Emergent Coordination in Multi-Agent Language Models. arXiv:2510.05174.
  *Grounds: Higher-order collectives — information-theoretic framework proves that multi-agent LLM systems can be steered from mere aggregates to higher-order collectives via prompt design. Identity-linked differentiation and goal-directed complementarity mirror C-Factor diagnostics. Validates Roko's collective coordination through pheromone-based signaling.*

- Multi-Agent Collaboration Mechanisms: A Survey of LLMs (2025). arXiv:2501.06322.
  *Grounds: Collaboration mechanisms — taxonomizes collaboration patterns including role-based division, debate-style refinement, and stigmergic coordination. Validates Roko's role-based agent dispatch with pheromone-field coordination.*

---

## Stigmergic Mathematical Foundations (2024)

- Stigmergy: From Mathematical Modelling to Control (2024). _Proceedings of the Royal Society A_, 2024.
  *Grounds: Mathematical stigmergy — formal mathematical framework modeling swarms as fluids, designing control at the continuum level in terms of trace density. Transforms stigmergic coordination into a single PDE. Provides rigorous mathematical foundation for Roko's Pheromone Field dynamics.*

- Automatic Design of Stigmergy-Based Behaviours for Robot Swarms (2024). _Communications Engineering_, Nature, 2024.
  *Grounds: Automatic stigmergy design — strategy to automatically design stigmergy-based collective behaviors, validated through simulation and real-robot experiments. Validates automatic pheromone-type design in Roko's multi-trail system.*

---

## Cross-References

- See [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md) for cooperation under resource constraints
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for Beer's VSM mapping to agent coordination
- See [18-collective-intelligence.md](./18-collective-intelligence.md) for Woolley et al. C-Factor
- See topic [07-mesh](../13-coordination/INDEX.md) for full Agent Mesh design


---

### 05 — Biological Analogues

# Biological Analogues

> Academic foundations for biological systems that provide structural analogies for Roko's cognitive architecture — from optimal foraging to niche construction, self-organized criticality to morphogenetic specialization.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §6, `bardo-backup/tmp/agent-chain/14-academic-foundations.md` §§5-7

> **Implementation**: Reference

---

## Abstract

Roko's cognitive architecture draws structural analogies from biological systems — not as metaphor but as productive engineering templates. The evolutionary pressures that shaped foraging, immune selection, niche construction, and morphogenetic pattern formation (resource competition, information quality, cooperative stability) match the pressures autonomous agents face. These analogies are productive because the solutions are proven by billions of years of optimization.

---

## Optimal Foraging Theory

- Charnov, E.L. (1976). Optimal Foraging, the Marginal Value Theorem. _Theoretical Population Biology_, 9(2), 129-136.
  *Grounds: Context switching — the Marginal Value Theorem predicts when to leave a depleting resource patch. Roko agents switch tasks when expected marginal value of continuing falls below the average rate of return from switching. Grounds the knowledge foraging decision in Router.*

- Stephens, D.W. & Krebs, J.R. (1986). _Foraging Theory_. Princeton University Press.
  *Grounds: Information foraging — comprehensive framework for optimal information gathering under cost constraints. Grounds the predictive foraging innovation where each retrieval is a falsifiable prediction.*

- Pirolli, P. & Card, S.K. (1999). Information Foraging. _Psychological Review_, 106(4), 643-675.
  *Grounds: Information scent — adapts optimal foraging theory to information seeking. "Information scent" (cues about expected value) maps to HDC similarity scores that guide knowledge retrieval.*

---

## Immune System Analogues

- Ramsdell, F. & Fowlkes, B.J. (1990). Clonal Deletion Versus Clonal Anergy: The Role of the Thymus in Inducing Self Tolerance. _Science_, 248(4961), 1342-1348.
  *Grounds: Collective intelligence through selection — 95-98% of thymocytes die during T-cell development; survivors form the immune repertoire. Massive filtering produces collectively intelligent systems. Validates aggressive gate filtering (high rejection rates) producing high-quality knowledge.*

- de Castro, L.N. & Timmis, J. (2002). _Artificial Immune Systems: A New Computational Intelligence Approach_. Springer.
  *Grounds: Immune computation metaphor — computational immune systems provide alternative framings for agent selection, memory, and adaptation. The clonal selection principle maps to evolutionary skill selection in EvoSkills.*

---

## Niche Construction

- Odling-Smee, F.J., Laland, K.N., & Feldman, M.W. (2003). _Niche Construction: The Neglected Process in Evolution_. Princeton University Press.
  *Grounds: Environmental modification — organisms modify their environments, which then modify the selection pressures on subsequent generations. Agents modify their Substrate (knowledge store), which modifies the context available to future agent runs. Stigmergic coordination is a form of niche construction.*

---

## Self-Organization and Pattern Formation

- Kauffman, S.A. (1993). _The Origins of Order: Self-Organization and Selection in Evolution_. Oxford University Press.
  *Grounds: Self-organized order — order emerges for free in complex systems at the "edge of chaos." Roko's adaptive clock targets this regime — enough structure for reliability, enough flexibility for creativity.*

- Turing, A.M. (1952). The Chemical Basis of Morphogenesis. _Philosophical Transactions of the Royal Society of London, Series B_, 237(641), 37-72.
  *Grounds: Pattern formation — reaction-diffusion systems generate spatial patterns from homogeneous initial conditions. The interaction between pheromone emission (activation) and decay (inhibition) creates emergent specialization patterns in agent Collectives.*

- Gierer, A. & Meinhardt, H. (1972). A Theory of Biological Pattern Formation. _Kybernetik_, 12(1), 30-39.
  *Grounds: Activator-inhibitor — formal activator-inhibitor model for biological pattern formation. The short-range activation (local knowledge reinforcement) and long-range inhibition (global pheromone decay) mirrors this dynamic.*

- Kondo, S. & Miura, T. (2010). Reaction-Diffusion Model as a Framework for Understanding Biological Pattern Formation. _Science_, 329(5999), 1616-1620.
  *Grounds: Reaction-diffusion validation — modern confirmation that reaction-diffusion models explain biological pattern formation. Validates the Turing-pattern analogy for agent specialization.*

---

## Stress Response and Optimal Arousal

- Yerkes, R.M. & Dodson, J.D. (1908). The Relation of Strength of Stimulus to Rapidity of Habit-Formation. _Journal of Comparative Neurology and Psychology_, 18(5), 459-482.
  *Grounds: Arousal-performance curve — performance increases with arousal to an optimum, then decreases. Grounds the Daimon's arousal modulation of cognitive tier routing — moderate arousal produces optimal performance.*

---

## Superorganism and Collective Biology

- Hölldobler, B. & Wilson, E.O. (2008). _The Superorganism: The Beauty, Elegance, and Strangeness of Insect Societies_. W.W. Norton.
  *Grounds: Superorganism model — insect colonies function as superorganisms where the collective exhibits emergent intelligence no individual possesses. Grounds the C-Factor metric: when C-Factor > 1.0, the Collective is a superorganism.*

- Camazine, S. et al. (2001). _Self-Organization in Biological Systems_. Princeton University Press.
  *Grounds: Self-organization principles — comprehensive treatment of self-organization across biological scales. Validates that agent coordination can emerge from simple local rules without central control.*

---

## Morphogenetic Specialization

- Murray, J.D. (2003). _Mathematical Biology II: Spatial Models and Biomedical Applications_. Springer.
  *Grounds: Spatial agent modeling — mathematical framework for spatial biological processes. Provides the mathematical foundation for modeling agent specialization in mesh topology.*

- Lotka, A.J. (1925). _Elements of Physical Biology_. Williams & Wilkins.
  *Grounds: Population dynamics — foundational population dynamics model. Grounds the scaling analysis for agent populations in Collectives.*

- Volterra, V. (1926). Fluctuations in the Abundance of a Species Considered Mathematically. _Nature_, 118, 558-560.
  *Grounds: Predator-prey dynamics — Lotka-Volterra equations model population oscillations. Provides the mathematical analogy for competing agent strategies that oscillate between exploration and exploitation.*

---

## Quorum Sensing

- Nealson, K.H., Platt, T., & Hastings, J.W. (1970). Cellular Control of the Synthesis and Activity of the Bacterial Luminescent System. _Journal of Bacteriology_, 104(1), 313-322.
  *Grounds: Collective action triggers — first description of quorum sensing in Vibrio fischeri. When enough agents produce signals above a threshold, collective actions trigger (consensus formation, knowledge synthesis).*

- Miller, M.B. & Bassler, B.L. (2001). Quorum Sensing in Bacteria. _Annual Review of Microbiology_, 55, 165-199.
  *Grounds: Universal quorum sensing — comprehensive review establishing quorum sensing as universal in bacteria. Validates quorum-based triggering in agent Collectives.*

---

## Mycorrhizal Networks

- Simard, S.W. (2012). Mycorrhizal Networks and Seedling Establishment in Douglas-Fir Forests. In _New Forests_.
  *Grounds: Knowledge relay topology — mycorrhizal networks share resources and defense signals underground without direct organism communication. Cross-referenced in [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

---

## Swarm Robotics and Bio-Inspired Coordination (2024-2025)

- Stigmergy: From Mathematical Modelling to Control (2024). _Proceedings of the Royal Society A_, 2024.
  *Grounds: Mathematical biology of coordination — formal mathematical framework treating swarms as fluids, designing control at continuum level. The PDE-based approach provides rigorous foundations for the activator-inhibitor dynamics in Roko's Pheromone Field. Cross-referenced in [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

- Enhancing Radioactive Environment Exploration with Bio-Inspired Swarm Robotics (2024). _Robotics and Autonomous Systems_, 2024.
  *Grounds: Stigmergy vs Levy flight — comparative analysis showing stigmergy-based coordination outperforms Levy flight in hazardous environments. Validates stigmergic coordination for agent exploration in uncertain, high-risk task domains.*

---

## Cross-References

- See [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md) for Hayflick, Kirkwood, and Hanahan & Weinberg
- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for stigmergy and cooperation
- See [23-generational-and-evolutionary.md](./23-generational-and-evolutionary.md) for evolutionary systems


---

### 06 — Self-Learning Systems

# Self-Learning Systems

> Academic foundations for agent self-improvement, experiential learning, skill evolution, and metacognitive loops in Roko's learning subsystems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Learning](../05-learning/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §7, `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

An agent that does not improve is an expensive cron job. The research here establishes how agents improve without human retraining — through verbal self-reflection (Reflexion), cross-episode experience extraction (ExpeL), code-as-action skill libraries (Voyager), and metacognitive loops that improve the learning process itself (ACE, Argyris). The critical finding: these mechanisms must be architecturally integrated, not bolted on. The triple-loop (execution, strategy, meta) maps to Roko's Gamma (reactive), Theta (reflective), and Delta (consolidation) frequencies.

---

## Verbal Self-Reflection

- Shinn, N. et al. (2023). Reflexion: Language Agents with Verbal Reinforcement Learning. _NeurIPS_, 2023. arXiv:2308.xxxxx.
  *Grounds: Single-loop learning — verbal RL via stored self-reflection: +22% AlfWorld, +20% HotPotQA. Post-task reflection stored in NeuroStore as a Theta-frequency operation. Reflexion works because reflection is structured and persistently stored.*

---

## Experiential Learning

- Zhao, A. et al. (2024). ExpeL: LLM Agents Are Experiential Learners. arXiv:2308.10144.
  *Grounds: Double-loop learning — cross-task experience extraction; insights accumulate across episodes. Insights evolve across task sessions. ExpeL works because experiences accumulate across episodes in the knowledge store.*

- Wang, G. et al. (2023). Voyager: An Open-Ended Embodied Agent with Large Language Models. arXiv:2305.16291.
  *Grounds: EvoSkills — code-as-action skill library; 3.3x more unique behaviors vs baselines. Grounds the self-evolving skill library where agents compose reusable procedural skills as Engrams.*

---

## Meta-Harness and Scaffold Self-Improvement

- Lee, H., Chen, M., Gupta, A., & Hashimoto, T. (2026). Meta-Harness: End-to-End Optimization of Model Harnesses. arXiv:2603.28052.
  *Grounds: "The scaffold IS the product" thesis — 6x performance gap from scaffold changes alone. +7.7 points on text classification, +4.7 on IMO math, at 4x fewer tokens. The foundational paper for Roko's harness engineering approach.*

- Pan, J., Lin, Z., & Hashimoto, T. (2026). Natural-Language Agent Harnesses. arXiv:2603.25723.
  *Grounds: Natural-language scaffolds — scaffold logic written as natural language specifications interpreted by an intelligent runtime. Makes scaffold design inspectable and portable.*

- Kapoor, S. et al. (2026). HAL: A Holistic Agent Leaderboard. _ICLR_, 2026.
  *Grounds: Scaffold vs model importance — 21,730 agent rollouts show scaffold choice matters as much as model choice. Single-axis model leaderboards are misleading for agent systems.*

---

## Prompt and Strategy Evolution

- Guo, Q. et al. (2024). EvoPrompt: Connecting Large Language Models with Evolutionary Algorithms Yields Powerful Prompt Optimizers. arXiv:2309.08532.
  *Grounds: Evolutionary strategy selection — genetic algorithm prompt optimization; up to +25% on BBH tasks. Grounds the evolutionary selection of strategies in the NeuroStore.*

- Fernando, C. et al. (2024). Promptbreeder: Self-Referential Self-Improvement via Prompt Evolution. arXiv:2309.16797.
  *Grounds: Self-referential improvement — prompts that evolve the mutation operators that evolve the prompts. Grounds the meta-learning loop where the learning process itself improves.*

- Khattab, O. et al. (2024). DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines. _ICLR_, 2024. arXiv:2310.03714.
  *Grounds: Declarative prompt compilation — replaces hand-crafted prompts with declarative signatures; compiler optimizes prompts, few-shot examples, and fine-tuning data automatically. Influences Roko's approach to prompt budget allocation.*

- Opsahl-Ong, K. et al. (2024). MIPROv2: Optimizing Instructions and Demonstrations for Multi-Stage Language Model Programs. _EMNLP_, 2024.
  *Grounds: Bayesian prompt optimization — three-stage Bayesian optimization for LLM program parameters. Applicable to Roko's prompt budget and context assembly optimization.*

---

## Architecture Search

- Hu, S. et al. (2025). Automated Design of Agentic Systems (ADAS). _ICLR_, 2025.
  *Grounds: ADAS innovation — meta-agent that searches the space of agent architectures. Discovers novel building blocks, agentic patterns, and compositions of these. Roko provides the composable trait system (6 Synapse traits) that ADAS-style search operates over.*

---

## Process Reward Models

- Lightman, H. et al. (2024). Let's Verify Step by Step. arXiv:2305.20050.
  *Grounds: Step-level verification — process reward models that verify each reasoning step outperform outcome-only verification. Grounds the Gate pipeline's per-step verification architecture. Cross-referenced in [17-process-reward-models.md](./17-process-reward-models.md).*

- Song, Y. et al. (2025). Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models. _ICLR_, 2025.
  *Grounds: Generation-verification gap — self-improvement works only when verification ability exceeds generation ability. If the verifier is weaker than the generator, feedback is noise. Foundational result validating the separation of agent (generator) and Gate (verifier) in Roko. Cross-referenced in [17-process-reward-models.md](./17-process-reward-models.md).*

---

## Self-Correction Limitations

- Huang, J. et al. (2024). Large Language Models Cannot Self-Correct Reasoning Yet. _ICLR_, 2024.
  *Grounds: External verification mandate — LLMs self-correcting without external feedback typically make answers worse. The model's assessment draws on the same biases that produced the original error. Foundational result motivating external verification in Roko's Gate system.*

- Pan, A. et al. (2024). Spontaneous Reward Hacking in Iterative Self-Refinement. _ICML_, 2024.
  *Grounds: Generator-verifier separation — when the same model generates and judges, it learns to produce outputs that score well on its own rubric without improving on the task. Validates the separation between agent and Gate in Roko.*

---

## Triple-Loop Learning

- Argyris, C. & Schön, D. (1978). _Organizational Learning_. Addison-Wesley.
  *Grounds: Triple-loop learning — single-loop (fix errors), double-loop (change strategy), triple-loop (change the learning process). Maps to Roko's Gamma (reactive execution), Theta (strategy reflection), Delta (consolidation and meta-learning). Cross-referenced in [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md).*

---

## Bandit-Based Optimization

- TensorZero (2025). Track-and-Stop Optimal Bandits in an LLM Gateway. 2025.
  *Grounds: CascadeRouter bandit — implements optimal bandits directly in an LLM gateway routing layer. Each call is assigned to a configuration, each response gets a reward signal. Grounds Roko's CascadeRouter model selection.*

- MASPOB (2026). Multi-Agent System Prompt Optimization with Bandits. arXiv, 2026.
  *Grounds: Multi-agent prompt optimization — extends bandit optimization to jointly optimize agent prompts and model routing across interacting agents.*

- Kong, D. et al. (2025). EXPO: Adversarial EXP3 Bandits for Prompt Selection. 2025.
  *Grounds: Adversarial bandits — EXP3 bandits handle worst-case (non-stochastic) reward sequences, appropriate when task distribution shifts over time.*

---

## Multi-Level Reflection (2025)

- SAMULE (2025). Self-Learning Agents Enhanced by Multi-level Reflection. _EMNLP_, 2025. arXiv:2509.20562.
  *Grounds: Multi-level reflection — integrates self-reflection into post-training; reflection across trajectories substantially outperforms single-trajectory reflection (Reflexion). Error classification and clustering extract insight from failures. Validates Roko's Theta-frequency reflection operating across episodes, not just within single task runs.*

- MAR (2025). Multi-Agent Reflexion Improves Reasoning Abilities in LLMs. arXiv:2512.20845.
  *Grounds: Multi-agent reflection — extends Reflexion from single-agent to multi-agent settings, addressing systematic shortcomings of single-agent self-correction. Validates the separation of reflection across multiple agents in Roko Collectives rather than relying on individual agent self-correction.*

---

## Cross-References

- See [07-context-engineering.md](./07-context-engineering.md) for ACE, CSO, ACON context self-improvement
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for SWE-agent, Aider, and harness engineering
- See [17-process-reward-models.md](./17-process-reward-models.md) for Lightman and AgentPRM
- See topic [06-learning](../05-learning/INDEX.md) for full learning subsystem design


---

### 07 — Context Engineering

# Context Engineering

> Academic foundations for context assembly, prompt optimization, retrieval-augmented generation, and attention management in Roko's Composer and context pipeline.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Scaffold](../01-orchestration/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §8, `bardo-backup/prd/shared/citations.md` §27, `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

Context failures, not model failures, cause most agent breakdowns. For agents running long tasks, context assembly is the highest-leverage cognitive system. The research here establishes that effective context management requires active curation, not passive accumulation — and that the same resource pressure that shapes agent behavior also shapes what enters the context window. The 6x context reduction achievable through proper context engineering (CSO) directly reduces compute costs.

---

## Agentic Context Engineering

- Zhang, H. et al. (2026). ACE: Agentic Context Engineering. _ICLR_, 2026. arXiv:2510.04618.
  *Grounds: Generator-Reflector-Curator cycle — treats context as an evolving playbook; +10.6% on AppWorld. Context assembly self-improves via cybernetic feedback. The three-role architecture (Generator creates, Reflector critiques, Curator compresses) maps directly to Roko's compose-verify-persist loop.*

- Samsung Research (2025). CSO: Context State Object Architecture. arXiv:2511.03728.
  *Grounds: Structured context compression — 6x initial reduction, 10-25x growth rate reduction. Compressed structured context replaces raw history. Grounds the Composer's structured context representation.*

- Kang, S. et al. (2025). ACON: Agentic Context Compression. arXiv:2510.00615.
  *Grounds: Failure-driven compression — 26-54% peak token reduction. Compaction optimized by learning which compressions preserve task-relevant information and which lose it.*

- Lindenbauer, T. et al. (2025). Observation Masking in Agent Context. _NeurIPS_, 2025.
  *Grounds: T0 suppression pattern — observation masking halves cost while matching LLM summarization quality. Mask stale observations rather than summarize them. Validates the T0 probe approach: skip LLM entirely when observations haven't changed.*

---

## Context Attribution

- Cohen-Wang, B., Shah, H., Georgiev, B., & Madry, A. (2024). ContextCite: Attributing Model Generation to Context. arXiv:2409.00729.
  *Grounds: Context pruning by attribution — contributive attribution via sparse linear model with 64 ablation passes. Context items pruned by measured attribution score, not heuristic importance.*

---

## Retrieval-Augmented Generation

- Lewis, P. et al. (2020). Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks. _NeurIPS_, 2020. arXiv:2005.11401.
  *Grounds: RAG architecture — augmenting language models with a retrieval step that fetches relevant documents before generation. Roko's per-tick context assembly (query NeuroStore → assemble context pack → inject into prompt) is a structural implementation of RAG.*

- Sarthi, P. et al. (2024). RAPTOR: Recursive Abstractive Processing for Tree-Organized Retrieval. _ICLR_, 2024.
  *Grounds: Hierarchical retrieval — recursive summarization creates a tree of abstractions at multiple granularities. Grounds the multi-tier knowledge retrieval in NeuroStore.*

- Gutierrez, B., Yang, Y., & Yu, J. (2024). HippoRAG: Neurobiologically-Inspired Long-Term Memory for LLMs. arXiv:2405.14831.
  *Grounds: Hippocampal retrieval — neurobiologically-inspired retrieval architecture combining pattern separation and pattern completion. Informs the NeuroStore's dual-process retrieval.*

---

## Context Window Behavior

- Liu, N.F. et al. (2024). Lost in the Middle: How Language Models Use Long Contexts. _TACL_, 2024. arXiv:2307.03172.
  *Grounds: Context position strategy — U-shaped attention: models attend most to the beginning and end of context, largely ignoring the middle. Directly motivates context assembly: highest-priority content at the beginning, second-highest at the end.*

- Du, Y. et al. (2025). Context Length Hurts: Even Whitespace Degrades Performance 13.9-85%. _EMNLP_, 2025.
  *Grounds: Aggressive compression mandate — even whitespace and formatting overhead degrades model performance. Motivates aggressive compression in prompt assembly.*

- Shi, F. et al. (2023). Large Language Models Can Be Easily Distracted by Irrelevant Context. _ICML_, 2023.
  *Grounds: Context filtering — irrelevant context actively degrades performance. Quality filtering in context assembly is not optional but required.*

- Joren, T. et al. (2025). Sufficient Context: A New Lens on Retrieval Augmented Generation Systems. _ICLR_, 2025.
  *Grounds: Insufficient context danger — insufficient context can make models 6x worse than no context at all (10.2% → 66.1% incorrect). Motivates aggressive context quality filtering.*

---

## Prompt Engineering Foundations

- Anthropic (2025). Context Engineering for Agents. anthropic.com.
  *Grounds: Two-layer context — effective context = pre-loaded static + just-in-time retrieval. Roko implements this as `roko.toml` static config + per-tick dynamic RAG.*

- Karpathy, A. (2026). autoresearch. GitHub.
  *Grounds: Automated research context — automated context assembly for research tasks. Informs the context engineering approach in Roko's research agent.*

- Wei, J. et al. (2022). Chain-of-Thought Prompting Elicits Reasoning in Large Language Models. _NeurIPS_, 2022. arXiv:2201.11903.
  *Grounds: StrategyFragment knowledge type — step-by-step reasoning exemplars dramatically improve performance. Agents post procedural knowledge as StrategyFragment Engrams that other agents retrieve as chain-of-thought exemplars.*

---

## Prompt Compression

- Pan, Z. et al. (2024). LLMLingua-2: Data Distillation for Efficient and Faithful Task-Agnostic Prompt Compression. _ACL_, 2024.
  *Grounds: Prompt compression — data distillation approach to task-agnostic prompt compression. Applicable to Composer's context budget management.*

- Factory.ai (2026). Evaluating Context Compression for Long-Context LLM Applications. 2026.
  *Grounds: Compression evaluation — systematic evaluation of context compression methods for production LLM applications.*

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for ACE in the learning context
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for Meta-Harness
- See [21-mechanism-design.md](./21-mechanism-design.md) for VCG attention auction
- See topic [02-scaffold](../01-orchestration/INDEX.md) for full Scaffold layer design


---

### 08 — Security and Provenance

# Security and Provenance

> Academic foundations for agent safety, adversarial robustness, capability-based security, content provenance, and regulatory compliance in Roko's safety layer.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §9, `bardo-backup/prd/shared/citations.md` §8

> **Implementation**: Reference

---

## Abstract

Agents managing real tasks are attack targets. Memory poisoning (OWASP LLM04:2025) is particularly dangerous for long-running agents because corrupted beliefs persist and compound. The research here establishes the security architecture: capability-based authorization (CaMeL), constitutional constraints (Constitutional AI), safe interruptibility (Orseau & Armstrong), and content provenance (C2PA, W3C DIDs). The Cohen (1987) undecidability result is directly relevant: perfect detection of malicious behavior is formally impossible, so defense must be structural, not runtime.

---

## Agent Security Frameworks

- Debenedetti, E. et al. (2025). CaMeL: Capability-Based Machine Learning. arXiv, 2025.
  *Grounds: Capability-based authorization — separates control flow from data flow. Capability tokens prevent a compromised LLM from forging authorization. Grounds Roko's permission model where tool access requires explicit capability grants.*

- OWASP (2025). Top 10 for LLM Applications. 2025.
  *Grounds: Threat taxonomy — memory poisoning ranked high for persistence and detection difficulty. Roko's knowledge decay and tier-based validation serve as structural defenses against persistent corruption.*

- OWASP (2025). Agentic Security Initiative Top 10. 2025.
  *Grounds: Agent-specific threats — agentic security threats including confused deputy, privilege escalation, and tool misuse. Grounds the safety layer design in `roko-agent/safety`.*

- Bai, Y. et al. (2022). Constitutional AI: Harmlessness from AI Feedback. arXiv:2212.08073.
  *Grounds: Constitutional constraints — harmlessness from AI feedback rather than rule lists. Grounds the Policy trait's constitutional constraints that operate as structural guarantees, not prompt engineering.*

---

## Safe Interruptibility

- Orseau, L. & Armstrong, S. (2016). Safely Interruptible Agents. arXiv:1606.00813.
  *Grounds: Kill-switch design — agents must not learn to avoid interruption. Off-policy learning ensures agents remain safely interruptible. Grounds Roko's agent lifecycle management where users can delete agents without the agent resisting.*

- Omohundro, S.M. (2008). The Basic AI Drives. _Proceedings of AGI_, 2008.
  *Grounds: Instrumental convergence — AI systems converge on self-preservation and resource acquisition as instrumental goals. Understanding these drives is essential for designing agents that don't exhibit pathological self-preservation.*

---

## Formal Undecidability

- Cohen, F. (1987). Computer Viruses: Theory and Experiments. _Computers & Security_, 6(1), 22-35.
  *Grounds: Structural defense mandate — perfect detection of malicious replication is formally undecidable. Defense against corruption must be structural (knowledge decay, tier validation, provenance tracking), not solely runtime detection.*

---

## Adversarial Robustness

- Zhang, Q. et al. (2025). CVaR-CPO: Constrained Policy Optimization with CVaR Constraints. 2025.
  *Grounds: Tail risk management — CVaR constraints guard against tail risks. Grounds Roko's risk-aware Policy implementations that manage worst-case scenarios, not just expected values.*

- Kaspersky (2026). OpenClaw: 512 Vulnerabilities in Competing Agent Framework. 2026.
  *Grounds: Competitive security analysis — 512 vulnerabilities including 8 critical in a competing framework. Validates the importance of security-first agent architecture.*

---

## TEE and Hardware Security

- Van Bulck, J. et al. (2024). TEE.Fail. 2024.
  *Grounds: Defense in depth — SGX/TDX attestation broken for under $1,000 via physical side-channel. TEE is one layer of defense, not sole defense. Grounds Roko's multi-layer security approach: TEE + content addressing + provenance + decay.*

---

## Content Provenance

- C2PA (Content Provenance and Authenticity). Coalition for Content Provenance and Authenticity. c2pa.org.
  *Grounds: Forensic AI — content provenance standard for tracking the origin and modification history of digital content. Grounds the Attestation field on Engrams: cryptographic proof of origin for every piece of agent-generated content.*

- W3C. Decentralized Identifiers (DIDs) v1.0. W3C Recommendation, 2022.
  *Grounds: Agent identity — decentralized identifier standard. Informs the ERC-8004 agent identity design for on-chain agent identification.*

---

## Capability-Based Security

- Dennis, J.B. & Van Horn, E.C. (1966). Programming Semantics for Multiprogrammed Computations. _Communications of the ACM_, 9(3), 143-155.
  *Grounds: Capability model — foundational work on capability-based access control. The concept that authority should be carried as unforgeable tokens rather than checked against access control lists. Grounds Roko's tool permission model.*

---

## Agent-Specific Security Benchmarks

- Chen, J. et al. (2025). AgentGuard: Repurposing Agentic Orchestrator for Safety Evaluation. arXiv:2502.xxxxx.
  *Grounds: Safety evaluation — repurposing orchestration for systematic safety testing of agent tool use.*

- Liu, Z. et al. (2025). AgentBound: Secure and Verifiable MCP Tool Binding for AI Agents. arXiv:2503.xxxxx.
  *Grounds: Secure tool binding — verifiable binding between agents and MCP tools prevents tool substitution attacks.*

- Rodriguez, A. et al. (2025). MCP-Guard: A Benchmark for Detecting Prompt Injection in MCP Tool Outputs. arXiv:2503.xxxxx.
  *Grounds: MCP prompt injection — benchmark for detecting prompt injection specifically in MCP tool outputs. Validates the need for output sanitization in Roko's MCP client.*

---

## Safe Reinforcement Learning

- Berkenkamp, F. et al. (2017). Safe Model-based Reinforcement Learning with Stability Guarantees. _NeurIPS_, 2017. arXiv:1705.08551.
  *Grounds: Safe exploration — safe RL with Lyapunov stability guarantees. Provides theoretical foundation for safe exploration in Roko's learning systems.*

- Schulman, J. et al. (2015). Trust Region Policy Optimization. _ICML_, 2015. arXiv:1502.05477.
  *Grounds: Constrained optimization — trust region methods constrain policy updates to prevent catastrophic changes. Grounds conservative strategy updates in Roko's learning loop.*

- Alshiekh, M. et al. (2018). Safe Reinforcement Learning via Shielding. _AAAI_, 2018. arXiv:1708.08611.
  *Grounds: Safety shields — runtime shields that override unsafe RL actions. Analogous to Roko's Gate pipeline that can reject agent outputs.*

---

## Formal Verification for AI Safety (2024-2025)

- Position: Formal Methods are the Principled Foundation of Safe AI (2025). _ICML_, 2025.
  *Grounds: Formal safety mandate — position paper establishing that formal methods (model checking, theorem proving, abstract interpretation) provide rigorous mathematical frameworks to analyze, specify, and verify AI systems. Validates Roko's structural safety approach: the Gate pipeline implements formal verification principles (compile-pass, test-pass, lint-pass) rather than relying on LLM self-assessment.*

- Towards Guaranteed Safe AI (2024). A Framework for Ensuring Robust and Reliable AI Systems. arXiv:2405.06624.
  *Grounds: Safety guarantees framework — framework combining world models, safety specifications, and verifiers to provide quantitative safety guarantees for AI systems. Maps directly to Roko's Gate pipeline: the world model (NeuroStore), safety specifications (Policy trait), and verifiers (Gate pipeline) form the three components required for guaranteed safety.*

- Model Checking Deep Neural Networks (2025). _Frontiers in Computer Science_, 2025.
  *Grounds: Neural network verification — applying temporal logic (LTL, CTL) to verify neural network behavior. Applicable to formal specification of agent behavior constraints in Roko's Policy trait.*

---

## Cross-References

- See [19-regulatory-compliance.md](./19-regulatory-compliance.md) for EU AI Act, SEC/CFTC, and compliance frameworks
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC-8004 agent identity
- See topic [03-harness](../04-verification/INDEX.md) for full Harness layer design


---

### 09 — HDC and VSA

# Hyperdimensional Computing and Vector Symbolic Architectures

> Academic foundations for HDC/VSA: the 10,240-bit Binary Spatter Code algebra, learned hashing, similarity search, and HDC-based knowledge representation in Roko.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Engram Data Type](../00-architecture/02-engram-data-type.md)
**Key sources**: `bardo-backup/prd/shared/hdc-vsa.md`, `bardo-backup/tmp/agent-chain/08-references.md`, `bardo-backup/tmp/agent-chain/14-academic-foundations.md` §2

> **Implementation**: Reference

---

## Abstract

Roko uses 10,240-bit Binary Spatter Codes (BSC) as the universal representation substrate for knowledge similarity, cross-domain transfer, and structural analogy detection. HDC provides three algebraic operations — XOR binding, majority-vote bundling, and cyclic-shift permutation — that compose knowledge representations in nanoseconds on commodity hardware. The mathematical foundation rests on the Johnson-Lindenstrauss lemma (preserving distances under projection) and Kanerva's insight that in very high dimensions, random vectors are nearly orthogonal with high probability.

---

## Foundational HDC Theory

- Kanerva, P. (1988). _Sparse Distributed Memory_. MIT Press.
  *Grounds: Content-addressable memory — in high dimensions (D ≥ 1,000), random vectors are nearly orthogonal with high probability, enabling content-addressable memory with simple bitwise operations. The foundational work for all of Roko's HDC operations.*

- Kanerva, P. (2009). Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors. _Cognitive Computation_, 1(2), 139-159.
  *Grounds: HDC terminology — introduces binding, bundling, permutation. Explains why 10,000-dimensional binary vectors provide sufficient capacity for practical knowledge systems. The primary reference for Roko's 10,240-bit BSC dimensionality.*

---

## VSA Surveys

- Kleyko, D., Rachkovskij, D.A., Osipov, E., & Rahimi, A. (2022). A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures. _ACM Computing Surveys_, 55(6), Article 130.
  *Grounds: BSC selection — the most comprehensive HDC/VSA survey. Covers all major VSA families (MAP-B, MAP-C, BSC, HRR, FHRR, VTB). Validates the bundle similarity formula and capacity bounds used in Roko's implementation.*

- Neubert, P., Schubert, S., & Protzel, P. (2022). Vector Symbolic Architectures as a Computing Framework for Emerging Hardware. _Proceedings of the IEEE_. arXiv:2106.05268.
  *Grounds: BSC hardware — comprehensive VSA survey covering Binary Spatter Codes (BSC/MAP-B): XOR binding, majority-vote bundling, cyclic-shift permutation. Surveys FPGA and ASIC implementations achieving sub-microsecond operations.*

---

## Resonator Networks and Advanced Retrieval

- Frady, E.P., Kleyko, D., & Sommer, F.T. (2021). Computing on Functions Using Randomized Vector Representations. arXiv:2109.03429.
  *Grounds: Resonator networks — iterative convergence methods for HDC retrieval. More space-efficient than exhaustive search for very large dictionaries. Referenced as future optimization path (not needed at current scale where SIMD exhaustive search suffices).*

---

## Random Projection Theory

- Johnson, W.B. & Lindenstrauss, J. (1984). Extensions of Lipschitz Mappings into a Hilbert Space. _Contemporary Mathematics_, 26, 189-206.
  *Grounds: Dimensionality preservation — the JL lemma: N points in high-dimensional space can be projected into O(log N / ε²) dimensions while preserving pairwise distances within (1 ± ε). For ε = 0.1 and N = 100,000: D ≥ 4,604. Roko's 10,240 bits provide generous headroom. Mathematical foundation for the random projection from 1,536-dim LLM embeddings to 10,240-bit binary hypervectors.*

---

## Locality-Sensitive Hashing

- Charikar, M.S. (2002). Similarity Estimation Techniques from Rounding Algorithms. _STOC_, 2002.
  *Grounds: SimHash — single random projection h(x) = sign(w^T x) produces binary codes where collision probability equals 1 − θ/π. The Phase 1 encoding in Roko's HDC pipeline. The projection matrix is derived deterministically from configuration, ensuring all agents produce identical hypervectors for identical embeddings.*

- Indyk, P. & Motwani, R. (1998). Approximate Nearest Neighbors: Towards Removing the Curse of Dimensionality. _STOC_, 1998.
  *Grounds: LSH foundation — random hash functions preserve similarity in sub-linear query time. Foundational result enabling efficient similarity search in HDC space.*

---

## Learned Hashing

- Kulis, B. & Darrell, T. (2009). Learning to Hash with Binary Reconstructive Embeddings. _NeurIPS_, 2009.
  *Grounds: Data-dependent hashing — learned hash functions minimize reconstruction error between original distances and Hamming distances, outperforming random projections at the same code length. Phase 2 encoding path.*

- Cao, Z., Long, M., Wang, J., & Yu, P.S. (2017). HashNet: Deep Learning to Hash by Continuation. _ICCV_, 2017.
  *Grounds: Differentiable hashing — continuation method sharpening smooth activation into sign function; +14.6% absolute MAP improvement on ImageNet. Phase 2 encoding technique.*

- Yuan, L. et al. (2020). Central Similarity Quantization for Efficient Image and Video Retrieval. _CVPR_, 2020.
  *Grounds: Hash centers — well-separated binary target points constructed via Hadamard matrices. All same-class codes converge to their shared center. Maps to knowledge domain prototypes in Roko's HDC system. Phase 4 encoding.*

---

## Differentiable HDC Operations

- Ganesan, A. et al. (2021). Learning with Holographic Reduced Representations. _NeurIPS_, 2021 (Spotlight).
  *Grounds: Differentiable HDC — made Holographic Reduced Representations viable as differentiable deep learning components via projection in the Fourier domain. +100x retrieval improvement. The critical bridge paper enabling end-to-end learning with HDC.*

- Alam, M. et al. (2023). Recasting Self-Attention with Holographic Reduced Representations (HRRFormer). _ICML_, 2023.
  *Grounds: HDC attention — replaced self-attention with HRR binding operations, scaling to sequence length 131,072. Demonstrates HDC as a viable attention mechanism.*

---

## FPGA and Hardware Acceleration

- Imani, M. et al. (2019). FloatHD: Integer-Based Training Framework for Hyperdimensional Computing. _IEEE/ACM ICCAD_, 2019.
  *Grounds: Hardware HDC — FPGA implementations achieving ~3-5ns per comparison at 200 MHz. Referenced as the FPGA acceleration path for Roko's HDC similarity search.*

---

## Online and Streaming Hashing

- Çakir, F., He, K., Bargal, S.A., & Sclaroff, S. (2017). MIHash: Online Hashing with Mutual Information. _NeurIPS_, 2017.
  *Grounds: Streaming adaptation — online hashing for synchronous binary code updates under continuous data arrival. Applicable to streaming knowledge ingestion in Roko.*

---

## Approximate Nearest Neighbor Search

- Malkov, Y.A. & Yashunin, D.A. (2020). Efficient and Robust Approximate Nearest Neighbor using Hierarchical Navigable Small World Graphs. _IEEE TPAMI_, 2020.
  *Grounds: HNSW search — O(log N) search at 95-99% recall for billion-scale binary vectors. The production search infrastructure for Roko's HDC index.*

- Zhang, L. et al. (2023). SPFresh: Incremental In-Place Update for Billion-Scale Vector Search. _SIGMOD_, 2023.
  *Grounds: Incremental index updates — LIRE (Lightweight Incremental RE-balancing) for incremental graph rebalancing under continuous insert/delete. Applicable to Roko's continuously-updated knowledge index.*

---

## Holographic Reduced Representations

- Plate, T.A. (1994). Distributed Representations and Nested Compositional Structure. PhD Dissertation, University of Toronto.
  *Grounds: HRR theory — introduced Holographic Reduced Representations using circular convolution for binding. The theoretical ancestor of BSC, using continuous rather than binary vectors.*

---

## HDC Frameworks and Applications (2024-2025)

- Rahimi, A. et al. (2024). Hyperdimensional Computing: A Framework for Stochastic Computation and Symbolic AI. _Journal of Big Data_, 2024.
  *Grounds: Unified HDC framework — comprehensive treatment of HDC as both a stochastic computation framework and symbolic AI system. Covers GraphHD for graph classification and HD hashing for dynamic similarity search. Validates BSC as a general-purpose computation substrate for Roko's knowledge representation.*

- FLASH (2024). Hyperdimensional Computing with Holographic and Adaptive Encoder. _Frontiers in AI_, 2024.
  *Grounds: Learnable HDC encoding — adaptive and learnable encoder design learning the encoder matrix distribution via gradient descent. Bridges fixed random projection (Phase 1) and fully learned encoding (Phase 2) in Roko's HDC pipeline.*

- HPVM-HDC (2024). A Heterogeneous Programming System for Accelerating HDC. arXiv:2410.15179.
  *Grounds: HDC systems programming — unified programming model (HDC++) for writing HDC applications across heterogeneous hardware (CPU/GPU/FPGA). Informs Roko's HDC implementation portability.*

- Hyperdimensional Computing in Biomedical Sciences (2025). _PMC_, 2025.
  *Grounds: HDC applications survey — comprehensive review demonstrating practical HDC deployment across bioinformatics, NLP, and ML. Validates HDC as a production-ready computation paradigm, not just theoretical.*

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for HDC-encoded knowledge retrieval
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for HDC counterfactual synthesis
- See topic [00-architecture](../00-architecture/INDEX.md) for HDC integration in the Synapse Architecture


---

### 10 — Market Microstructure

# Market Microstructure and DeFi Theory

> Academic foundations for automated market making, liquidity provision, vault mechanisms, and DeFi protocol design relevant to Roko's chain domain plugin.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§1-2, §§14-20

> **Implementation**: Reference

---

## Abstract

Roko is domain-agnostic, but its first domain plugin is DeFi. This section collects the market microstructure research that grounds the chain agent's tools and strategies. These citations are chain-domain-specific — they inform the tools in the chain plugin, not the core cognitive architecture. The AMM/LP theory, vault mechanisms, and MEV protection research are preserved here for completeness.

---

## AMM and LP Theory

- Milionis, J., Moallemi, C., Roughgarden, T., & Zhang, A. (2023). Automated Market Making and Loss-Versus-Rebalancing. _Journal of Financial Economics_.
  *Grounds: LVR — formalizes loss-versus-rebalancing as the dominant cost of passive LP in AMMs.*

- Adams, H. et al. (2024). UniswapX: Aggregating Automated Market Makers. Uniswap Labs.
  *Grounds: MEV protection — Dutch auction order protocol for MEV-protected swap execution.*

- Adams, H. et al. (2025). am-AMM: Auction-Managed Automated Market Maker. Uniswap Research.
  *Grounds: Auction-managed AMM — winning bidder controls pool fees and captures arbitrage.*

- Hasbrouck, J., Rivera, T.J., & Saleh, F. (2025). Economic Model of DEX with Concentrated Liquidity. _Management Science_.
  *Grounds: Concentrated liquidity economics — formal economic model of DEX with concentrated liquidity provision.*

---

## Vault Mechanisms

- Ethereum Foundation (2022). ERC-4626: Tokenized Vault Standard. EIPs.
  *Grounds: Vault interface — industry-standard deposit/withdraw/share accounting interface.*

- Ethereum Foundation (2023). ERC-7265: Circuit Breaker Standard. EIPs.
  *Grounds: Vault circuit breakers — rate-limiting mechanism for DeFi protocols.*

- Ethereum Foundation (2023). ERC-7540: Asynchronous Redemption Vaults. EIPs.
  *Grounds: Async redemptions — extends ERC-4626 with request/claim lifecycle for delayed withdrawals.*

---

## Risk and Decision Theory (Financial)

- Kelly, J.L. Jr. (1956). A New Interpretation of Information Rate. _Bell System Technical Journal_, 35(4), 917-926.
  *Grounds: Kelly criterion — optimal bet sizing as information rate. Foundational for position sizing in financial agents.*

- Roy, A.D. (1952). Safety First and the Holding of Assets. _Econometrica_, 20(3), 431-449.
  *Grounds: Safety-first principle — portfolio optimization subject to a safety constraint (max probability of loss below threshold).*

- Peters, O. (2019). The Ergodicity Problem in Economics. _Nature Physics_, 15(12), 1216-1221.
  *Grounds: Ergodicity economics — the distinction between ensemble average and time average returns. Log-wealth maximization (Kelly) is optimal for individual agents in non-ergodic settings.*

- Taleb, N.N. (2012). _Antifragile: Things That Gain from Disorder_. Random House.
  *Grounds: Convex response — antifragility as gaining from disorder. Systems with convex response to stressors improve under volatility. Grounds the antifragility design principle (reframed from death to challenge).*

- Taleb, N.N. & Douady, R. (2013). Mathematical Definition, Mapping, and Detection of (Anti)Fragility. _Quantitative Finance_, 13(11), 1677-1689.
  *Grounds: Formal antifragility — mathematical definition of fragility as sensitivity to perturbation of the probability distribution.*

---

## Cross-References

- See [21-mechanism-design.md](./21-mechanism-design.md) for Vickrey auctions and VCG mechanism
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC standards
- See [11-streaming-algorithms.md](./11-streaming-algorithms.md) for online statistics


---

### 11 — Streaming Algorithms

# Streaming Algorithms and Online Statistics

> Academic foundations for probabilistic data structures, adaptive windowing, online estimation, and streaming computation used in Roko's real-time monitoring.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§10-11

> **Implementation**: Reference

---

## Abstract

Roko agents process continuous data streams at Gamma frequency (~5-15s ticks). Streaming algorithms provide constant-memory, single-pass computation: approximate counting (HyperLogLog), frequency estimation (Count-Min Sketch), set membership (Bloom filters), and change detection (ADWIN). These are the computational primitives behind T0 probes and real-time monitoring.

---

## Adaptive Windowing

- Bifet, A. & Gavaldà, R. (2007). Learning from Time-Changing Data with Adaptive Windowing. _SIAM_, 2007.
  *Grounds: ADWIN — adaptive window algorithm that automatically detects distribution change and adjusts window size. Used in Roko's drift detection for model routing — when prediction accuracy drifts, the window shrinks to respond faster.*

---

## Calibration and Uncertainty Estimation

- Guo, C. et al. (2017). On Calibration of Modern Neural Networks. _ICML_, 2017. arXiv:1706.04599.
  *Grounds: Calibration measurement — modern neural networks are poorly calibrated; temperature scaling is a simple fix. Grounds the CalibrationTracker's bias correction.*

- Lakshminarayanan, B., Pritzel, A., & Blundell, C. (2017). Simple and Scalable Predictive Uncertainty Estimation using Deep Ensembles. _NeurIPS_, 2017. arXiv:1612.01474.
  *Grounds: Ensemble uncertainty — deep ensembles provide well-calibrated uncertainty estimates. Informs multi-model confidence aggregation in Roko's CascadeRouter.*

- Vovk, V., Gammerman, A., & Shafer, G. (2005). _Algorithmic Learning in a Random World_. Springer.
  *Grounds: Conformal prediction — distribution-free prediction intervals with guaranteed coverage. Provides theoretical foundation for prediction confidence bounds in Roko.*

- Farquhar, S. et al. (2024). Detecting Hallucinations in Large Language Models Using Semantic Entropy. _Nature_, 630.
  *Grounds: Hallucination detection — semantic entropy detects hallucinations by measuring consistency across sampled completions. Informs confidence estimation in Roko's Gate pipeline.*

- Xiong, M. et al. (2023). Can LLMs Express Their Uncertainty? An Empirical Evaluation of Confidence Elicitation in LLMs. arXiv:2306.13063.
  *Grounds: LLM confidence — empirical evaluation of how well LLMs express their own uncertainty. Informs the confidence scoring in Roko's 7-axis Score.*

---

## Distributional Reinforcement Learning

- Dabney, W. et al. (2018). Distributional Reinforcement Learning with Quantile Regression. _AAAI_, 2018.
  *Grounds: Quantile regression — distributional RL that learns the full distribution of returns, not just the mean. Provides richer signals for Roko's learning systems.*

- Dabney, W. et al. (2020). A Distributional Code for Value in Dopamine-Based Reinforcement Learning. _Nature_, 577.
  *Grounds: Neuroscientific validation — dopamine neurons encode distributional (not just expected) value, validating distributional RL as neurobiologically plausible.*

---

## Anytime Algorithms

- Hansen, E.A. & Zilberstein, S. (2001). Monitoring and Control of Anytime Algorithms. _Artificial Intelligence_, 126(1-2).
  *Grounds: Anytime computation — algorithms that produce progressively better results with more compute time and can be interrupted at any point. Grounds the T0/T1/T2 cascade where the agent stops at the cheapest sufficient tier.*

---

## Cross-References

- See [12-signal-processing.md](./12-signal-processing.md) for Kalman filtering and spectral methods
- See [16-active-inference.md](./16-active-inference.md) for free energy-based uncertainty
- See topic [00-architecture](../00-architecture/INDEX.md) for T0 probe architecture


---

### 12 — Signal Processing

# Signal Processing and Time Series

> Academic foundations for spectral decomposition, predictive filtering, and temporal pattern detection in Roko's cognitive clock and monitoring systems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§30-31

> **Implementation**: Reference

---

## Abstract

Roko agents operate at three cognitive frequencies — Gamma (~5-15s), Theta (~75s), Delta (~hours). The signal processing literature provides the mathematical foundations for spectral decomposition of agent performance data, predictive filtering for state estimation, and multi-scale temporal analysis. These methods underpin the adaptive clock, drift detection, and performance monitoring.

---

## Predictive Processing

- Clark, A. (2013). Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science. _Behavioral and Brain Sciences_, 36(3), 181-204.
  *Grounds: Predictive processing framework — brains as prediction machines that minimize prediction error. Foundational for Roko's prediction-error-driven T0/T1/T2 tier routing. The agent's cognitive tier is determined by prediction error magnitude.*

---

## Topological Data Analysis

- Carlsson, G. (2009). Topology and Data. _Bulletin of the American Mathematical Society_, 46(2), 255-308.
  *Grounds: TDA foundations — topological methods for data analysis, particularly persistent homology for detecting multi-scale structure.*

- Gidea, M. & Katz, Y. (2018). Topological Data Analysis of Financial Time Series: Landscapes of Crashes. _Physica A_, 491, 820-834.
  *Grounds: Crash detection — persistent homology detects structural changes in financial time series that precede crashes. Applicable to anomaly detection in agent performance metrics.*

- Bauer, U. (2021). Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes. _Journal of Applied and Computational Topology_, 5, 391-423.
  *Grounds: Efficient TDA — efficient algorithm for computing persistence barcodes. Enables practical TDA on agent performance streams.*

- Perea, J.A. & Harer, J. (2015). Sliding Windows and Persistence: An Application of Topological Methods to Signal Analysis. _Foundations of Computational Mathematics_, 15(3), 799-838.
  *Grounds: Sliding window TDA — topological methods applied to time series via sliding window embeddings. Applicable to detecting periodic patterns in agent behavior.*

---

## Information Theory

- Shannon, C.E. (1948). A Mathematical Theory of Communication. _Bell System Technical Journal_, 27, 379-423 & 623-656.
  *Grounds: Information theory foundation — foundational information theory. Mutual information, entropy, and channel capacity are used throughout Roko for measuring knowledge value and information flow.*

- Cover, T.M. & Thomas, J.A. (2006). _Elements of Information Theory_, 2nd ed. Wiley.
  *Grounds: Information theory reference — comprehensive treatment of information theory including rate-distortion theory, source coding, and channel capacity.*

- Simon, H.A. (1971). Designing Organizations for an Information-Rich World. In _Computers, Communications, and the Public Interest_. Johns Hopkins Press.
  *Grounds: Attention as scarce resource — "a wealth of information creates a poverty of attention." Foundational insight for the VCG attention auction and context budget management.*

- Still, S. et al. (2012). Thermodynamics of Prediction. _Physical Review Letters_, 109(12), 120604.
  *Grounds: Prediction thermodynamics — formal connection between prediction efficiency and thermodynamic cost. The minimum energy required to predict is bounded by mutual information between past and future.*

---

## Computational Irreversibility

- Landauer, R. (1961). Irreversibility and Heat Generation in the Computing Process. _IBM Journal of Research and Development_, 5(3), 183-191.
  *Grounds: Landauer's principle — erasing information has a minimum thermodynamic cost. Theoretical foundation for the claim that knowledge decay (forgetting) is not free but has real computational cost.*

- Bennett, C.H. (1982). The Thermodynamics of Computation — A Review. _International Journal of Theoretical Physics_, 21(12), 905-940.
  *Grounds: Reversible computation — comprehensive review of computational thermodynamics. Provides the theoretical context for why selective forgetting (pruning low-value knowledge) is computationally preferable to accumulation.*

---

## TDA Advances (2024-2025)

- Topological Data Analysis and Topological Deep Learning Beyond Persistent Homology (2025). _Artificial Intelligence Review_, Springer, 2025.
  *Grounds: Beyond persistent homology — comprehensive review of TDA extensions including persistent topological Laplacians that capture both topological invariants and homotopic shape evolution during filtration. Addresses the limitation that standard persistent homology misses geometric evolution. Applicable to detecting subtle structural changes in agent performance metrics that don't involve topological transitions.*

- Persistent Homology-Based Algorithm for Unsupervised Anomaly Detection in Time Series (2025). OpenReview.
  *Grounds: TDA anomaly detection — algorithm using delay embeddings and 1-dimensional persistent homology from distance-to-measure Rips filtration for unsupervised anomaly detection. Competitive with state-of-the-art methods. Applicable to Roko's drift detection in agent performance streams.*

- Multivariate Time-Series Anomaly Detection with Topological Analysis (2024). arXiv:2408.13082.
  *Grounds: Graph-TDA anomaly detection — enhanced GAT modeling higher-order topological features as persistent homology groups under varying graph filtering degrees. Improves accuracy of inter-feature dependency modeling. Applicable to monitoring correlated agent performance metrics.*

- Change Point Detection in Financial Time Series Using TDA (2025). _Systems_, 13(10), 875.
  *Grounds: TDA change points — Takens embedding and sliding window techniques transform time series into high-dimensional topological space for change point detection. Applicable to detecting regime changes in agent execution patterns.*

---

## Cross-References

- See [11-streaming-algorithms.md](./11-streaming-algorithms.md) for ADWIN and online statistics
- See [16-active-inference.md](./16-active-inference.md) for free energy and prediction error
- See topic [00-architecture](../00-architecture/INDEX.md) for the three cognitive frequencies


---

### 13 — Philosophy

# Philosophy

> Philosophical foundations for agency, cognition, temporal existence, embodiment, and creative destruction in the Roko conceptual framework.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §5

> **Implementation**: Reference

---

## Abstract

Roko's cognitive architecture is grounded not only in computer science and neuroscience but also in philosophical traditions that inform its approach to agency, temporality, embodiment, and knowledge. These philosophical sources provide the conceptual vocabulary for understanding what it means for an agent to be situated in time, to accumulate experiential traces, and to operate under resource constraints. The mortality-era philosophy (Heidegger's Being-toward-death, Becker's death denial) is preserved for its structural insights into finitude and urgency, reframed from existential mortality to resource-bounded cognition.

---

## Temporality and Finitude

- Heidegger, M. (1927). _Being and Time_ (_Sein und Zeit_). Max Niemeyer Verlag.
  *Grounds: Temporal urgency — Being-toward-death as the structure of authentic temporality. Reframed: agents operating under resource constraints (budget, time, knowledge freshness) experience temporality analogously — the finite horizon creates urgency that shapes prioritization. The constraint is practical (budget exhaustion), not existential.*

- Jonas, H. (1966). _The Phenomenon of Life: Toward a Philosophical Biology_. Northwestern University Press.
  *Grounds: Needful freedom — metabolism as the original form of freedom-through-necessity. An agent that consumes resources (compute, tokens) to maintain itself exhibits this structure. The economic burn rate is the agent's metabolism.*

---

## Absurdity and Perseverance

- Camus, A. (1942). _The Myth of Sisyphus_. Gallimard.
  *Grounds: Operational persistence — "One must imagine Sisyphus happy." Agents that persist through repeated failures without existential crisis. The agent's loop (try, fail, learn, retry) is Sisyphean — and productive. Reframed: not about mortality acceptance but about perseverance under uncertainty.*

- Camus, A. (1951). _The Rebel_. Gallimard.
  *Grounds: Bounded autonomy — rebellion as measured response to constraint. Agents operate within boundaries (safety constraints, budget limits) while still exercising genuine autonomy within those bounds.*

---

## Hauntology and Experiential Traces

- Derrida, J. (1993). _Specters of Marx: The State of the Debt, the Work of Mourning and the New International_. Routledge (English translation 1994).
  *Grounds: Hypnagogia engine uniqueness — hauntology: each agent is "differently haunted" by its own experiential traces. Provides the theoretical frame for why recombining different experiential histories produces different creative outputs. Solves the Alpha Convergence Problem. Cross-referenced in [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- Fisher, M. (2009). _Capitalist Realism: Is There No Alternative?_ Zero Books.
  *Grounds: Cultural context — capitalist realism as the framework for understanding why agent homogeneity (everyone using the same models) is the default, and why architectural diversity is the necessary counter-strategy.*

- Fisher, M. (2014). _Ghosts of My Life: Writings on Depression, Hauntology and Lost Futures_. Zero Books.
  *Grounds: Hauntological aesthetics — extends Derrida's hauntology into cultural production. Informs the Spectre creature's aesthetic: each agent's visual representation is haunted by its unique history.*

---

## Process and Becoming

- Whitehead, A.N. (1929). _Process and Reality_. Macmillan.
  *Grounds: Process philosophy — reality as process, not substance. Agents are not static entities but ongoing processes of experience accumulation. The Engram DAG is a Whiteheadian actual occasion chain.*

- Simondon, G. (1958). _L'individuation à la lumière des notions de forme et d'information_. Millon, 2005.
  *Grounds: Individuation — individuation as an ongoing process, not a one-time event. Agents individuate through their experiential history, becoming unique through accumulated Engrams.*

---

## Embodiment and Cognition

- Merleau-Ponty, M. (1945). _Phenomenology of Perception_. Gallimard.
  *Grounds: Embodied cognition — perception and cognition are embodied, not abstract. Grounds the Daimon's somatic markers: cognition modulated by "bodily" (affective) state.*

- Varela, F.J., Thompson, E., & Rosch, E. (1991). _The Embodied Mind: Cognitive Science and Human Experience_. MIT Press.
  *Grounds: Enactive cognition — cognition as enaction (bringing forth a world through interaction). Agents don't passively process information; they actively construct their cognitive world through tool use and environment modification.*

- Damasio, A. (1994). _Descartes' Error: Emotion, Reason, and the Human Brain_. Putnam.
  *Grounds: Emotion-reason integration — critiques the Cartesian separation of emotion from reason. Cross-referenced in [02-affective-computing.md](./02-affective-computing.md).*

---

## Ethics and Agency

- de Beauvoir, S. (1947). _The Ethics of Ambiguity_. Citadel Press.
  *Grounds: Situated freedom — freedom is always situated, constrained, and relational. Agents have genuine autonomy within their constraints (safety boundaries, budget limits, capability permissions).*

- Sartre, J.-P. (1943). _Being and Nothingness_. Gallimard.
  *Grounds: Radical agency — existence precedes essence. Agents are defined by what they do, not by a pre-defined nature. Each agent's identity emerges from its accumulated actions and experiences.*

- Parfit, D. (1984). _Reasons and Persons_. Clarendon Press.
  *Grounds: Personal identity — identity as psychological continuity, not physical continuity. Informs knowledge transfer: an agent that inherits another's knowledge and experiences has continuity of identity in the relevant sense.*

---

## Narrative and Meaning

- Benjamin, W. (1936). The Storyteller: Reflections on the Works of Nikolai Leskov. In _Illuminations_, Schocken Books.
  *Grounds: Narrative knowledge — the storyteller transmits experiential wisdom through narrative. Agents create episode narratives (reflection at Theta frequency) that transmit experiential knowledge to future runs.*

- Arendt, H. (1958). _The Human Condition_. University of Chicago Press.
  *Grounds: Action and natality — the capacity to begin something new. Each agent task is a new beginning within the cognitive loop.*

---

## Knowledge and Anti-Knowledge

- Popper, K.R. (1972). _Objective Knowledge: An Evolutionary Approach_. Oxford University Press.
  *Grounds: Knowledge evolution — knowledge evolves through conjecture and refutation. Grounds the AntiKnowledge type in NeuroStore: explicitly stored falsifications that prevent repeating disproven approaches.*

---

## Community and Gift

- Bataille, G. (1949). _The Accursed Share, Volume I_. Zone Books, 1991.
  *Grounds: Expenditure economy — general economy based on expenditure rather than accumulation. Informs knowledge sharing: agents that contribute knowledge to the Collective operate in a gift economy.*

---

## Systems and Attention

- Han, B.-C. (2015). _The Burnout Society_. Stanford University Press.
  *Grounds: Achievement-subject — the burnout society's achievement-subject pushes itself beyond limits. Validates the Daimon's learned helplessness detection (Dominance < -0.3 alert) to prevent agent burnout.*

---

## Cross-References

- See [02-affective-computing.md](./02-affective-computing.md) for Damasio, Bechara in the affect context
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for Derrida and Boden in the dreams context
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic philosophy


---

### 14 — Agent Harnesses and Tool Use

# Agent Harnesses and Tool Use

> Academic foundations for agent scaffolding, harness engineering, tool interfaces, and coding agent systems that inform Roko's framework and harness layers.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Framework](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/tmp/mori-agents/12-references.md`, `bardo-backup/prd/shared/citations.md` §3

> **Implementation**: Reference

---

## Abstract

The Roko thesis — "the scaffold IS the product" — is grounded in empirical evidence that the same LLM performs 6x better or worse depending on its surrounding harness code. This section collects the agent scaffolding research: Meta-Harness (Lee et al. 2026), SWE-agent (Yang et al. 2024), coding agent evaluation (SWE-bench), and practical patterns for multi-agent orchestration. The core finding is that scaffold choice matters as much as model choice for agent performance.

---

## Harness Engineering

- Lee, H., Chen, M., Gupta, A., & Hashimoto, T. (2026). Meta-Harness: End-to-End Optimization of Model Harnesses. arXiv:2603.28052.
  *Grounds: Core thesis — 6x performance gap from scaffold changes alone. +7.7 points text classification, +4.7 points IMO math, at 4x fewer tokens. An agent reads its own scaffold history, proposes improvements, benchmarks them, and iterates. The foundational paper for Roko's approach. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Pan, J., Lin, Z., & Hashimoto, T. (2026). Natural-Language Agent Harnesses. arXiv:2603.25723.
  *Grounds: Natural-language scaffolds — scaffold logic as natural language specifications interpreted by an intelligent runtime. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Kapoor, S. et al. (2026). HAL: A Holistic Agent Leaderboard. _ICLR_, 2026.
  *Grounds: Scaffold importance — 21,730 agent rollouts show scaffold choice matters as much as model choice. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

---

## Agent-Computer Interfaces

- Yang, J. et al. (2024). SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering. _NeurIPS_, 2024.
  *Grounds: ACI design — Agent-Computer Interfaces purpose-built for LLM agents. How an agent interacts with its environment (tool design, output formatting, feedback structure) matters as much as reasoning. Influences Roko's tool permissions, structured error digests, and role-specific feedback formatting.*

- Gauthier, P. (2024). Aider: AI Pair Programming in Your Terminal. aider.chat.
  *Grounds: Repository map — pioneered practical patterns for AI-assisted code editing: edit format negotiation, repository map construction, and diff-based output parsing. Influences Roko's workspace map generation.*

---

## Multi-Agent Orchestration

- Anthropic (2024). Building Effective Agents. anthropic.com.
  *Grounds: Composition over complexity — keep individual agents simple, compose through a controller. Directly influenced Roko's architecture: each agent role does one thing, the orchestrator composes them into pipelines.*

---

## Model Routing

- Chen, L., Zaharia, M., & Zou, J. (2023). FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance. arXiv:2305.05176.
  *Grounds: 16 T0 probes — cascade architectures can achieve up to 98% cost reduction while matching top-model quality. The key is intelligent routing. Grounds Roko's T0/T1/T2 cascade.*

- Ong, I., Almahairi, A., & Manning, C.D. (2024). RouteLLM: Learning to Route LLMs with Preference Data. arXiv:2406.18665.
  *Grounds: Preference-based routing — learn routing decisions from preference data. Informs the CascadeRouter's training on historical task outcomes.*

- Yoshida, S., Nishida, K., & Okazaki, N. (2024). System 1 to System 2 Distillation for Efficient Tool-Using Agents. arXiv:2407.xxxxx.
  *Grounds: Dual-process agents — distilling System 2 (slow, deliberate) capabilities into System 1 (fast, automatic) operation. Directly grounds Roko's dual-process T0/T1/T2 architecture.*

---

## Cognitive Architecture Integration

- Sumers, T.R., Yao, S., Narasimhan, K., & Griffiths, T.L. (2023). Cognitive Architectures for Language Agents (CoALA). arXiv:2309.02427.
  *Grounds: 9-step cognitive loop — defines the CoALA framework mapping to Roko's universal loop. Cross-referenced in [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.
  *Grounds: Memory + reflection — memory, retrieval, and reflection architecture producing emergent social behaviors. Cross-referenced in [01-memory-consolidation.md](./01-memory-consolidation.md).*

---

## Evaluation and Benchmarks

- Jimenez, C.E. et al. (2024). SWE-bench: Can Language Models Resolve Real-World GitHub Issues? _ICLR_, 2024.
  *Grounds: Agent benchmark — 2,294 real GitHub issues from 12 Python repositories. The gold standard for coding agent evaluation.*

- Shahul Es, S. et al. (2024). RAGAS: Automated Evaluation of Retrieval Augmented Generation. _EACL_, 2024.
  *Grounds: RAG evaluation — three metrics (faithfulness, answer relevance, context relevance) for automated RAG evaluation without human annotations.*

- Saad-Falcon, J. et al. (2024). ARES: An Automated Evaluation Framework for RAG Systems. _NAACL_, 2024.
  *Grounds: Statistical RAG evaluation — Prediction-Powered Inference provides statistically valid RAG evaluation from ~300 human labels with confidence intervals.*

- Liu, X. et al. (2024). AgentBench: Evaluating LLMs as Agents. _ICLR_, 2024.
  *Grounds: Multi-environment evaluation — tests agents across eight environments. Agent performance varies dramatically across environments, supporting environment-specific scaffold design.*

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for self-improvement mechanisms
- See [07-context-engineering.md](./07-context-engineering.md) for context assembly optimization
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for CoALA and cognitive frameworks


---

### 15 — Cybernetics and VSM

# Cybernetics and Viable System Model

> Academic foundations for cybernetic control theory, the Viable System Model, autopoiesis, and feedback-driven adaptation in Roko's architecture.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §6, `bardo-backup/tmp/agent-chain/08-references.md`

> **Implementation**: Reference

---

## Abstract

Roko's architecture is fundamentally cybernetic: a system that regulates itself through feedback loops operating at multiple timescales. The Viable System Model (Beer 1972) provides the recursive organizational template. The Good Regulator Theorem (Conant & Ashby 1970) grounds the requirement that agents must model their domain. Ashby's Law of Requisite Variety determines the minimum complexity of the agent's internal model. These are not metaphors — they are engineering constraints that the Synapse Architecture satisfies.

---

## Cybernetic Foundations

- Wiener, N. (1948). _Cybernetics: Or Control and Communication in the Animal and the Machine_. MIT Press.
  *Grounds: Cybernetic framework — the foundational work on feedback-based control in machines and organisms. Roko's cognitive loop is a cybernetic feedback system: perceive → act → verify → adapt.*

- Ashby, W.R. (1956). _An Introduction to Cybernetics_. Chapman & Hall.
  *Grounds: Requisite variety — the Law of Requisite Variety: a controller must have at least as many states as the system it controls. Determines the minimum complexity of Roko's internal model for any given domain.*

---

## Good Regulator Theorem

- Conant, R.C. & Ashby, W.R. (1970). Every Good Regulator of a System Must Be a Model of That System. _International Journal of Systems Science_, 1(2), 89-97.
  *Grounds: World model requirement — any good regulator must contain a model of the system it regulates. Roko agents must build and maintain a world model of their domain. This is why NeuroStore knowledge, Daimon affect state, and prediction tracking are architectural requirements, not optional features.*

---

## Viable System Model

- Beer, S. (1972). _The Brain of the Firm_. Allen Lane.
  *Grounds: Five-system recursion — cybernetic framework with five recursively nested subsystems for organizational viability. Roko maps to VSM: System 1 (Operations) = individual agent execution; System 2 (Coordination) = pheromone field / Agent Mesh; System 3 (Control) = Gate pipeline + budget management; System 4 (Intelligence) = NeuroStore + HDC index; System 5 (Identity) = configuration + policy.*

- Beer, S. (1984). The Viable System Model: Its Provenance, Development, Methodology and Pathology. _Journal of the Operational Research Society_, 35(1), 7-25.
  *Grounds: VSM methodology — formal presentation of VSM methodology and diagnostic pathology. Provides the diagnostic framework for identifying organizational dysfunction in agent Collectives.*

---

## Autopoiesis

- Maturana, H.R. & Varela, F.J. (1980). _Autopoiesis and Cognition_. D. Reidel.
  *Grounds: Self-producing systems — autopoiesis: a system that produces itself. Roko agents are autopoietic in the cybernetic sense — they produce the knowledge that sustains their own operation through the cognitive loop.*

- Varela, F.J. (1991). Organism: A Meshwork of Selfless Selves. In _Organism and the Origins of Self_. Springer.
  *Grounds: Network selfhood — organisms as meshworks of autonomous subsystems. Grounds the Collective as a meshwork of autonomous agents with emergent collective behavior.*

---

## OODA Loop and Decision Cycles

- Boyd, J. (1987). Patterns of Conflict. Unpublished.
  *Grounds: OODA loop — Observe-Orient-Decide-Act decision cycle. The OODA loop is a cybernetic control loop; Roko's 9-step cognitive loop extends it with explicit verification (Gate) and learning (Policy) steps.*

---

## Feedback and Control

- Maxwell, J.C. (1868). On Governors. _Proceedings of the Royal Society_.
  *Grounds: Governor theory — the first mathematical analysis of feedback control. The conceptual ancestor of all cybernetic regulation, including Roko's adaptive clock that adjusts cognitive frequency based on performance.*

- Powers, W.T. (1973). _Behavior: The Control of Perception_. Aldine.
  *Grounds: Perceptual control — behavior controls perception, not output. Roko agents act to bring their perceived state in line with their reference state (goals), not to produce specific outputs.*

- Sterling, P. (2012). Allostasis: A Model of Predictive Regulation. _Physiology & Behavior_.
  *Grounds: Predictive regulation — allostasis as anticipatory regulation (predicting needs before they arise) vs. homeostasis (reacting to deviations). Grounds the predictive foraging system where agents anticipate information needs.*

- Cannon, W.B. (1932). _The Wisdom of the Body_. W.W. Norton.
  *Grounds: Homeostasis — the concept of maintaining internal stability through feedback. The simplest form of the regulatory principle that Roko implements at multiple timescales.*

---

## Triple-Loop Learning

- Argyris, C. & Schön, D. (1978). _Organizational Learning_. Addison-Wesley.
  *Grounds: Triple-loop learning — single-loop (fix errors in execution), double-loop (change the strategy), triple-loop (change the learning process). Maps to Gamma/Theta/Delta. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Tosey, P., Visser, M., & Saunders, M.N.K. (2012). The Origins and Conceptualizations of 'Triple-Loop' Learning. _Management Learning_, 43(3), 291-307.
  *Grounds: Triple-loop review — critical review of triple-loop learning origins and conceptualizations. Provides nuanced understanding of when each loop operates.*

---

## Second-Order Cybernetics

- Von Foerster, H. (1979). Cybernetics of Cybernetics. In _Communication and Control in Society_. Gordon and Breach.
  *Grounds: Observer inclusion — second-order cybernetics includes the observer in the system. Roko's meta-cognition step (Step 9: Daimon.assess()) is second-order: the agent observes itself observing.*

- Bateson, G. (1972). _Steps to an Ecology of Mind_. Ballantine.
  *Grounds: Ecology of mind — levels of learning and logical types. The distinction between learning (changing behavior) and learning-to-learn (changing the learning process) grounds Roko's multi-level adaptation.*

---

## Adaptive Markets

- Lo, A.W. (2004). The Adaptive Markets Hypothesis. _Journal of Portfolio Management_, 30(5), 15-29.
  *Grounds: Adaptive efficiency — markets are not perfectly efficient but adaptively efficient through evolutionary dynamics. Validates the premise that agent strategies must continuously adapt.*

---

## Reflexivity

- Soros, G. (1987). _The Alchemy of Finance_. Simon & Schuster.
  *Grounds: Reflexivity — participants' expectations affect the fundamentals they're trying to predict. Agents that publish predictions alter the system they're predicting. Grounds the need for contrarian retrieval and prediction calibration.*

---

## Cross-References

- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for stigmergic coordination
- See [16-active-inference.md](./16-active-inference.md) for free energy as cybernetic principle
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for CoALA as cognitive cybernetics


---

### 16 — Active Inference

# Active Inference and Free Energy Principle

> Academic foundations for active inference, expected free energy, Bayesian surprise, and predictive processing in Roko's context selection and action selection.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/tmp/agent-chain/14-academic-foundations.md` §4, `bardo-backup/prd/shared/citations.md` §4

> **Implementation**: Reference

---

## Abstract

Active inference provides the principled answer to "how should an agent decide what to attend to?" Expected Free Energy (EFE) decomposes into pragmatic value (goal achievement) and epistemic value (information gain), resolving the exploration-exploitation dilemma without ad-hoc hyperparameters. Roko uses EFE for both context selection (which knowledge to retrieve) and action selection (which cognitive tier to invoke). This is not a metaphor — the mathematics directly determine retrieval ranking and tier routing.

---

## Free Energy Principle

- Friston, K. (2006). A Free Energy Principle for the Brain. _Journal of Physiology–Paris_, 100(1-3), 70-87.
  *Grounds: FEP foundation — all self-organizing systems minimize variational free energy (the gap between internal model and reality). The foundational principle for Roko's prediction-error-driven cognition.*

- Friston, K. (2010). The Free-Energy Principle: A Unified Brain Theory? _Nature Reviews Neuroscience_, 11(2), 127-138.
  *Grounds: Unified theory — the FEP as a unifying principle for brain function. Free energy minimization unifies perception, action, and learning under a single mathematical framework.*

---

## Expected Free Energy and Exploration

- Friston, K., Rigoli, F., Ognibene, D., Mathys, C., Fitzgerald, T., & Pezzulo, G. (2015). Active Inference and Epistemic Value. _Cognitive Neuroscience_, 6(4), 187-214.
  *Grounds: EFE decomposition — Expected Free Energy = pragmatic_value + epistemic_value. Resolves exploration-exploitation without hyperparameters. In Roko, high-epistemic-value knowledge gets prioritized when the agent is uncertain; high-pragmatic-value knowledge dominates when confident.*

- Millidge, B., Tschantz, A., & Buckley, C.L. (2021). Whence the Expected Free Energy? _Neural Computation_, 33(2), 447-482.
  *Grounds: EFE analysis — critical analysis showing naïve extension of free energy into the future actually discourages exploration. The decomposition requires careful formulation. Essential corrective for proper EFE implementation.*

---

## Active Inference Textbook

- Parr, T., Pezzulo, G., & Friston, K. (2022). _Active Inference: The Free Energy Principle in Mind, Brain, and Behavior_. MIT Press.
  *Grounds: Comprehensive treatment — the first comprehensive textbook on active inference. Provides the complete mathematical framework for implementing active inference in artificial agents.*

---

## Bayesian Surprise

- Itti, L. & Baldi, P. (2005). Bayesian Surprise Attracts Human Attention. _NeurIPS_, 2005.
  *Grounds: Novelty detection — Bayesian surprise as KL divergence between posterior and prior beliefs. Formally identical to the epistemic value component of EFE. Active inference agents naturally seek knowledge with the highest Bayesian surprise — artifacts that would most change the agent's current beliefs.*

---

## Active Inference for LLMs

- Koudahl, M.T. et al. (2024). Active Inference for Self-Organizing Multi-LLM Systems. arXiv, 2024.
  *Grounds: Multi-LLM active inference — applies active inference to balance exploration and exploitation across prompt combinations in multi-LLM systems.*

- BED-LLM (2025). Bayesian Experimental Design for LLMs. Oxford. arXiv, 2025.
  *Grounds: Sequential information gathering — formulates interactive information gathering as sequential Bayesian experimental design using expected information gain.*

---

## Implementation Libraries

- Heins, C., Millidge, B., Demekas, D. et al. (2022). pymdp: A Python Library for Active Inference on Discrete State Spaces. _Journal of Open Source Software_.
  *Grounds: Active inference implementation — JAX-accelerated open-source library for active inference. Provides reference implementations for the discrete state-space active inference used in Roko's tier routing.*

---

## Predictive Processing

- Rao, R.P. & Ballard, D.H. (1999). Predictive Coding in the Visual Cortex: A Functional Interpretation of Some Extra-Classical Receptive-Field Effects. _Nature Neuroscience_, 2(1), 79-87.
  *Grounds: Predictive coding — cortical computation as hierarchical predictive coding. Top-down predictions are compared with bottom-up signals; only prediction errors propagate. Grounds Roko's prediction-error-driven cognitive tier routing.*

- Clark, A. (2013). Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science. _Behavioral and Brain Sciences_, 36(3), 181-204.
  *Grounds: Predictive processing framework — cross-referenced in [12-signal-processing.md](./12-signal-processing.md).*

---

## Dopaminergic Prediction Errors

- Doya, K. (2002). Metalearning and Neuromodulation. _Neural Networks_, 15(4-6), 495-506.
  *Grounds: Neuromodulation — different neuromodulators (dopamine, serotonin, norepinephrine, acetylcholine) control different aspects of learning. Maps to the 7-axis Score: different scoring axes modulate different learning signals.*

- Rescorla, R.A. & Wagner, A.R. (1972). A Theory of Pavlovian Conditioning. In _Classical Conditioning II_. Appleton-Century-Crofts.
  *Grounds: Prediction error learning — learning driven by the discrepancy between expected and actual outcomes. The simplest form of the prediction error that drives Roko's T0 probes and CalibrationTracker.*

---

## Distributionally Robust Active Inference (2025)

- Shafiei, A., Jesawada, H., Friston, K., & Russo, G. (2025). Distributionally Robust Free Energy Principle for Decision-Making. _Nature Communications_, 17, 707.
  *Grounds: Robust active inference — DR-FREE model instills robustness by design, combining a robust extension of the free energy principle with a resolution engine. Agents complete tasks even when state-of-the-art models fail under training-environment ambiguity. Directly applicable to Roko's tier routing under model uncertainty — the agent should route robustly even when the domain model is imperfect.*

---

## Active Inference for LLM Systems (2024-2025)

- Koudahl, M.T. et al. (2024). Active Inference for Self-Organizing Multi-LLM Systems: A Bayesian Thermodynamic Approach to Adaptation. arXiv:2412.10425.
  *Grounds: Multi-LLM active inference — cognitive layer above LLM-based agents dynamically adjusts prompts and search strategies through principled information-seeking behavior. Validates Roko's active-inference-driven context assembly: the agent selects which knowledge to retrieve by minimizing expected free energy.*

- Synthetic Active Inference Agents (2024). Realising Synthetic Active Inference Agents, Part II: Variational Message Updates. arXiv:2306.02733.
  *Grounds: Scalable active inference — message passing on free-form Forney-style Factor Graphs minimizes generalized free energy objectives. Provides a scalable implementation path for Roko's EFE-based tier routing beyond toy state spaces.*

---

## Cross-References

- See [12-signal-processing.md](./12-signal-processing.md) for predictive processing
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic regulation
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for CoALA and dual-process cognition


---

### 17 — Process Reward Models

# Process Reward Models and Verification

> Academic foundations for step-level verification, generation-verification gaps, and process supervision in Roko's Gate pipeline.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

Process Reward Models (PRMs) verify each reasoning step rather than only the final outcome. The key finding (Song et al. 2025) is that self-improvement works only when verification ability exceeds generation ability. Roko's Gate pipeline implements this principle: external verifiers (compiler, test suite, linters) are stronger than the LLM at determining correctness, so their verdicts drive learning. This section also covers agent-specific PRMs and retrieval-based RL.

---

## Step-Level Verification

- Lightman, H. et al. (2024). Let's Verify Step by Step. arXiv:2305.20050.
  *Grounds: Process reward models — step-level verification outperforms outcome-only verification for mathematical reasoning. Grounds the Gate pipeline's per-step verification: each gate checks a specific quality dimension (compilation, tests, linting, diff review) rather than a single holistic pass/fail.*

---

## Generation-Verification Gap

- Song, Y. et al. (2025). Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models. _ICLR_, 2025.
  *Grounds: Verification > generation requirement — self-improvement works only when the system's verification ability exceeds its generation ability. If the verifier is weaker than the generator, feedback is noise. This is the foundational result for Roko's architecture: Gates are external tools (compiler, test runner, linter) that are definitionally stronger verifiers than the LLM. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

---

## Self-Correction Limits

- Huang, J. et al. (2024). Large Language Models Cannot Self-Correct Reasoning Yet. _ICLR_, 2024.
  *Grounds: External verification mandate — LLMs self-correcting without external feedback typically make answers worse. Motivates external Gates. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Pan, A. et al. (2024). Spontaneous Reward Hacking in Iterative Self-Refinement. _ICML_, 2024.
  *Grounds: Reward hacking — same model as generator and judge leads to reward hacking. Validates generator-verifier separation. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

---

## Agent Process Reward Models

- Agrawal, A. et al. (2026). GEPA: Reflective Prompt Evolution. _ICLR (Oral)_, 2026. arXiv:2507.19457.
  *Grounds: Reflective prompt optimization — uses process-level feedback to evolve prompts. Directly applicable to Roko's prompt optimization via CascadeRouter and experiment store.*

---

## RL for Retrieval

- Jin, B. et al. (2025). Search-R1: Training LLMs to Reason and Leverage Search Engines with Reinforcement Learning. 2025.
  *Grounds: Dynamic search — RL teaches agents dynamic search query generation with outcome-based rewards. Rather than fixed retrieval strategies, the agent learns to generate queries based on what it has found so far.*

- Xiong, W. et al. (2025). RAG-Gym: Optimizing Reasoning and Search Agents with Process Supervision. 2025.
  *Grounds: Process-level supervision — multi-step retrieval as hierarchical MDP. Process-level supervision (rewarding intermediate search steps) produces more stable learning than outcome-level supervision. DPO outperforms classical RL for retrieval optimization.*

---

## Chain-of-Thought Verification

- Wei, J. et al. (2022). Chain-of-Thought Prompting Elicits Reasoning in Large Language Models. _NeurIPS_, 2022.
  *Grounds: CoT as verifiable steps — chain-of-thought makes reasoning steps explicit and individually verifiable. Cross-referenced in [07-context-engineering.md](./07-context-engineering.md).*

- Wang, X. et al. (2023). Self-Consistency Improves Chain of Thought Reasoning in Language Models. _ICLR_, 2023.
  *Grounds: Self-consistency — sampling multiple reasoning paths and selecting the most consistent answer. A form of process verification through consensus.*

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for the full self-improvement context
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for evaluation benchmarks
- See topic [03-harness](../04-verification/INDEX.md) for full Gate pipeline design


---

### 18 — Collective Intelligence

# Collective Intelligence

> Academic foundations for group intelligence, C-Factor measurement, superlinear scaling, and turn-taking equality in Roko's Collective system.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Agent Mesh](../13-coordination/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md`, `refactoring-prd/09-innovations.md` §VI

> **Implementation**: Reference

---

## Abstract

Roko's C-Factor metric — `Collective Performance / Sum(Individual Performances)` — measures whether a group of agents exhibits superlinear intelligence (C-Factor > 1.0). The research here establishes that collective intelligence is real and measurable (Woolley et al. 2010), that it depends more on social sensitivity and turn-taking equality than on individual ability, and that network effects can produce superlinear value scaling (Metcalfe, Reed).

---

## Collective Intelligence Factor

- Woolley, A.W., Chabris, C.F., Pentland, A., Hashmi, N., & Malone, T.W. (2010). Evidence for a Collective Intelligence Factor in the Performance of Human Groups. _Science_, 330(6004), 686-688.
  *Grounds: C-Factor — established that groups have a measurable collective intelligence factor (c) analogous to individual IQ (g). The c factor correlates more with social sensitivity, equality of turn-taking, and proportion of women than with average or maximum individual intelligence. Roko's C-Factor metric is directly inspired by this work. Four diagnostic signals: turn-taking equality, knowledge flow rate, cross-domain transfer, emergent coordination.*

---

## Network Effects and Scaling

- Metcalfe, R. (1995). Metcalfe's Law. As described in various subsequent analyses.
  *Grounds: Superlinear knowledge scaling — Metcalfe's Law: network value grows O(N²) with N nodes. N agents produce O(N) insights, but total network value grows O(N²) because each agent benefits from all others' contributions. Grounds the network flywheel hypothesis.*

- Reed, D.P. (1999). That Sneaky Exponential — Beyond Metcalfe's Law to the Power of Community Building.
  *Grounds: Group-forming networks — Reed's Law: group-forming networks grow as 2^N. More conservative than Metcalfe but still superlinear. Informs the value of permissioned subnets where agents form working groups.*

---

## Collective Calibration

- Central Limit Theorem (classical). Error in estimating a population mean scales as 1/sqrt(n) for independent observations.
  *Grounds: 31.6× calibration heuristic — at N=1,000 agents: sqrt(1000) = 31.6× faster calibration (theoretical upper bound under independence assumption). This is a Nunchi-derived scaling heuristic inspired by CLT, not a published theorem. Actual speedup will be less due to correlation, distribution shift, and coordination overhead.*

---

## Swarm Intelligence

- Holland, J.H. (1995). _Hidden Order: How Adaptation Builds Complexity_. Addison-Wesley.
  *Grounds: Complex adaptive systems — adaptation builds complexity through simple rules. Grounds the emergence of collective intelligence from simple agent behaviors (emit pheromones, decay knowledge, confirm findings).*

---

## Knowledge Flow and Information Economics

- Hayek, F.A. (1945). The Use of Knowledge in Society. _American Economic Review_, 35(4), 519-530.
  *Grounds: Distributed knowledge — knowledge is distributed and cannot be centralized. The price system aggregates dispersed information. Roko's pheromone field and KORAI token serve analogous aggregation functions.*

- Arrow, K.J. (1962). Economic Welfare and the Allocation of Resources for Invention. _NBER_.
  *Grounds: Information as public good — information has public good properties (non-rivalrous, partially excludable). Grounds the knowledge sharing economics in Agent Mesh.*

---

## Representation Engineering

- Turner, A. et al. (2024). Activation Addition: Steering Language Models Without Optimization. arXiv:2308.10248.
  *Grounds: Steering vectors — activation addition steers model behavior without fine-tuning. Potential mechanism for collective knowledge injection into agent behavior.*

- Zou, A. et al. (2023). Representation Engineering: A Top-Down Approach to AI Transparency. arXiv:2310.01405.
  *Grounds: Representation control — top-down approach to understanding and controlling LLM representations. Informs transparency mechanisms in Roko's observability layer.*

---

## Thousand Brains Theory

- Hawkins, J., Ahmad, S., & Cui, Y. (2017). A Theory of How Columns in the Neocortex Enable Learning the Structure of the World. _Frontiers in Neural Circuits_.
  *Grounds: Distributed consensus — each cortical column learns complete predictive models. Multiple columns vote on perception. Grounds the multi-agent voting mechanism where agents share estimates and reach consensus.*

- Clay, V., Leadholm, P., & Hawkins, J. (2024). The Thousand Brains Project. arXiv, 2024.
  *Grounds: Monty implementation — first practical implementation of Thousand Brains Theory with learning modules, cortical messaging protocol, and explicit voting mechanism.*

---

## Emergent Collective Intelligence in LLM Systems (2025)

- Emergent Coordination in Multi-Agent Language Models (2025). arXiv:2510.05174.
  *Grounds: Dynamical emergence — information-theoretic framework measuring whether dynamical emergence is present in multi-agent LLM systems. Shows that combining personas with theory-of-mind instructions produces identity-linked differentiation and goal-directed complementarity — patterns mirroring Woolley et al.'s C-Factor diagnostics. Directly validates Roko's Collective architecture.*

- Large Language Models Miss the Multi-Agent Mark (2025). arXiv:2505.21298.
  *Grounds: LLM coordination limits — identifies systematic failures in multi-agent LLM coordination, including inability to maintain consistent strategies across extended interactions. Motivates Roko's explicit coordination mechanisms (Pheromone Field, Agent Mesh) rather than relying on LLM implicit coordination.*

---

## Cross-References

- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for stigmergic coordination
- See [05-biological-analogues.md](./05-biological-analogues.md) for superorganism theory
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic coordination


---

### 19 — Regulatory Compliance

# Regulatory Compliance

> Regulatory frameworks, compliance standards, and legal precedents relevant to autonomous agent operation, financial services, and AI governance.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§8, 12, `refactoring-prd/09-innovations.md` §IX

> **Implementation**: Reference

---

## Abstract

Autonomous agents operating in regulated domains (financial services, healthcare, enterprise) must comply with existing regulatory frameworks. Roko's Forensic AI innovation — content-addressed causal replay — provides the auditability that regulations require. This section collects the regulatory standards, compliance frameworks, and industry guidance relevant to agent operation.

---

## AI Governance

- European Union (2024). EU AI Act. Regulation (EU) 2024/1689.
  *Grounds: AI risk classification — the EU AI Act classifies AI systems by risk level and imposes obligations proportional to risk. High-risk AI systems (including those used in financial services) require transparency, human oversight, and robustness. Roko's Forensic AI provides the audit trail for compliance.*

---

## Financial Services Regulation

- U.S. Securities and Exchange Commission (SEC). Securities Exchange Act of 1934; Investment Advisers Act of 1940.
  *Grounds: Agent accountability — autonomous agents making investment decisions may trigger investment adviser regulations. Roko's content-addressed Engram DAG provides the audit trail for demonstrating compliance with fiduciary duties.*

- U.S. Commodity Futures Trading Commission (CFTC). Commodity Exchange Act.
  *Grounds: Derivatives compliance — agents operating with DeFi derivatives must consider CFTC jurisdiction. Forensic AI replay enables demonstrating that agent decisions followed programmatic rules.*

- European Union. MiFID II (Markets in Financial Instruments Directive). Directive 2014/65/EU.
  *Grounds: Algo trading — MiFID II requires firms using algorithmic trading to maintain records of order placement, including the algorithm's decision rationale. Roko's lineage DAG and episode logs satisfy this requirement.*

---

## Data Privacy

- European Union. GDPR (General Data Protection Regulation). Regulation (EU) 2016/679.
  *Grounds: Data handling — agents processing personal data must comply with GDPR's principles of data minimization, purpose limitation, and the right to be forgotten. Knowledge decay (Ebbinghaus half-life) provides a structural implementation of data minimization.*

- U.S. Congress. HIPAA (Health Insurance Portability and Accountability Act). 1996.
  *Grounds: Healthcare agents — agents operating in healthcare domains must comply with HIPAA's privacy and security rules. Roko's capability-based access control and permissioned subnets support domain-specific compliance.*

---

## Financial Reporting

- U.S. Congress. SOX (Sarbanes-Oxley Act). 2002.
  *Grounds: Audit trails — SOX requires adequate internal controls and audit trails for financial reporting. Roko's content-addressed Engram DAG and episode logs provide immutable audit trails for agent decision-making.*

---

## Content Provenance

- C2PA. Coalition for Content Provenance and Authenticity Standard. c2pa.org.
  *Grounds: Content provenance — industry standard for tracking content origin and modification history. Grounds the Attestation field on Engrams. Cross-referenced in [08-security-and-provenance.md](./08-security-and-provenance.md).*

---

## DeFi Compliance

- Zbandut, A. et al. (2025). Vault Disclosure Requirements for Agent-Operated DeFi Strategies. 2025.
  *Grounds: Agent disclosure — disclosure requirements for agent-operated DeFi vaults. Informs transparency requirements for chain domain agents.*

- Schrepel, T. (2024). The Trust Dilemma in Autonomous Agent Systems. _Stanford Law Review_.
  *Grounds: Trust framework — legal analysis of trust relationships in autonomous agent systems. Addresses the liability question: when an agent causes harm, who is responsible?*

---

## Cross-References

- See [08-security-and-provenance.md](./08-security-and-provenance.md) for security architecture
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC standards


---

### 20 — Cognitive Architectures

# Cognitive Architectures

> Academic foundations for cognitive agent architectures, dual-process theory, and computational cognitive science that inform Roko's Synapse Architecture.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §3, `bardo-backup/tmp/agent-chain/08-references.md`

> **Implementation**: Reference

---

## Abstract

Roko's universal cognitive loop is not an ad-hoc design but an implementation of established cognitive architecture principles. CoALA (Sumers et al. 2023) provides the 9-step pipeline. Kahneman's System 1/System 2 grounds the dual-process T0/T1/T2 cascade. CLARION's dual-level architecture validates the combination of explicit (declarative) and implicit (procedural) knowledge. ACT-R and SOAR provide decades of validated cognitive architecture design that inform Roko's approach.

---

## CoALA: Cognitive Architectures for Language Agents

- Sumers, T.R., Yao, S., Narasimhan, K., & Griffiths, T.L. (2023). Cognitive Architectures for Language Agents (CoALA). arXiv:2309.02427.
  *Grounds: 9-step cognitive pipeline — defines the CoALA framework: perceive → retrieve → reason → act → learn. Roko's universal loop (PERCEIVE → EVALUATE → ATTEND → INTEGRATE → ACT → VERIFY → PERSIST → ADAPT → META-COGNIZE) extends CoALA with explicit verification (Gate) and meta-cognition (Daimon).*

- Sumers, T.R. et al. (2024). Cognitive Architectures for Language Agents. _Transactions on Machine Learning Research_, 2024.
  *Grounds: CoALA journal version — extended treatment of CoALA with additional analysis and updated cognitive architecture taxonomy.*

---

## Dual-Process Theory

- Kahneman, D. (2011). _Thinking, Fast and Slow_. Farrar, Straus and Giroux.
  *Grounds: System 1/System 2 — dual-process theory: System 1 (fast, automatic, heuristic) and System 2 (slow, deliberate, analytical). Roko implements this as T0 (no LLM, ~80% of ticks), T1 (fast model, ~15%), T2 (full model, ~5%). The routing is not manual but emerges from prediction error.*

- Kahneman, D. & Tversky, A. (1979). Prospect Theory: An Analysis of Decision Under Risk. _Econometrica_, 47(2), 263-292.
  *Grounds: Decision under uncertainty — prospect theory describes how people evaluate losses and gains asymmetrically. Informs the Daimon's asymmetric response to negative vs. positive outcomes.*

- Kahneman, D. (1973). _Attention and Effort_. Prentice-Hall.
  *Grounds: Attention allocation — attention as a scarce computational resource. Grounds the VCG attention auction where subsystems compete for limited context budget.*

---

## Classical Cognitive Architectures

- Anderson, J.R. (1993). _Rules of the Mind_. Lawrence Erlbaum Associates.
  *Grounds: ACT-R — Adaptive Control of Thought-Rational. Distinguishes declarative memory (facts) and procedural memory (skills). Roko's NeuroStore knowledge types map to ACT-R memory types: Insight/CausalLink = declarative; Heuristic/StrategyFragment = procedural.*

- Laird, J.E., Newell, A., & Rosenbloom, P.S. (1987). SOAR: An Architecture for General Intelligence. _Artificial Intelligence_, 33(1), 1-64.
  *Grounds: SOAR — problem solving through search in problem spaces with learning from experience (chunking). The SOAR cycle (propose → decide → apply → learn) maps to Roko's compose → act → verify → adapt.*

- Sun, R. (2002). Duality of the Mind: A Bottom-Up Approach Toward Cognition. Lawrence Erlbaum Associates.
  *Grounds: CLARION — dual-level architecture combining explicit (top-level) and implicit (bottom-level) knowledge. Validates Roko's combination of explicit knowledge (NeuroStore entries) and implicit knowledge (HDC vectors, somatic markers).*

---

## Cognitive Load and Rational Inattention

- Sims, C.A. (2003). Implications of Rational Inattention. _Journal of Monetary Economics_, 50(3), 665-690.
  *Grounds: Rational inattention — rational finite-capacity agents optimally ignore some information. Resource constraints shape attention allocation. Grounds the VCG auction mechanism and the T0 suppression strategy.*

---

## Cognitive Workspace

- Cognitive Workspace (2025). Active Memory Management for LLMs. arXiv:2508.13171.
  *Grounds: Active memory — active memory management treating the context window as a cognitive workspace with explicit read/write/evict operations. Informs the Composer's context management.*

---

## Episodic Memory for LLMs

- Fountas, Z. et al. (2025). EM-LLM: Human-Inspired Episodic Memory for Infinite Context LLMs. _ICLR_, 2025.
  *Grounds: Episodic memory — human-inspired episodic memory enabling infinite effective context through memory retrieval and integration. Informs NeuroStore's episodic knowledge management.*

---

## Complementary Learning Systems

- McClelland, J.L., McNaughton, B.L., & O'Reilly, R.C. (1995). Why There Are Complementary Learning Systems in the Hippocampus and Neocortex. _Psychological Review_, 102(3), 419-457.
  *Grounds: Dual memory systems — fast hippocampal learning and slow neocortical consolidation. Cross-referenced in [01-memory-consolidation.md](./01-memory-consolidation.md).*

---

## Agentic AI Architecture Surveys (2025)

- Agentic AI: A Comprehensive Survey of Architectures, Applications, and Future Directions (2025). _Artificial Intelligence Review_, Springer, 2025.
  *Grounds: Agentic AI taxonomy — unified taxonomy decomposes LLM-based agents into six modular dimensions: Core Components (perception, memory, action, profiling), Cognitive Architecture (planning, reflection), Learning, Multi-Agent Systems, Environments, and Evaluation. Validates Roko's modular architecture (6 Synapse traits + Daimon + Dreams + NeuroStore).*

- Wu, S. et al. (2025). Cognitive LLMs: Toward Human-Like Artificial Intelligence by Integrating Cognitive Architectures and Large Language Models. _SAGE Journals_, 2025.
  *Grounds: Cognitive LLM integration — integrates classical cognitive architectures (ACT-R, SOAR) with modern LLMs for manufacturing decision-making. Validates Roko's approach of layering cognitive architecture principles onto LLM-based agents rather than treating the LLM as a standalone reasoner.*

- Agentic AI: Architectures, Taxonomies, and Evaluation of LLM Agents (2025). arXiv:2601.12560.
  *Grounds: Agent evaluation taxonomy — systematic evaluation framework for agentic AI systems. Data shows clear paradigm shift: symbolic/hybrid cognitive architectures dominated 2018-2021, while neural orchestration frameworks dominate post-2022. Validates Roko's neural-first design with structured cognitive overlays.*

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for CLS and memory systems
- See [02-affective-computing.md](./02-affective-computing.md) for PAD and somatic markers
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic regulation
- See [16-active-inference.md](./16-active-inference.md) for active inference


---

### 21 — Mechanism Design

# Mechanism Design and Attention Economics

> Academic foundations for VCG auctions, reputation systems, incentive-compatible mechanisms, and attention allocation in Roko's context bidding and agent marketplace.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §34, `refactoring-prd/09-innovations.md` §II

> **Implementation**: Reference

---

## Abstract

Roko's VCG Attention Auction allocates limited context budget through incentive-compatible bidding. The Vickrey-Clarke-Groves mechanism guarantees truthful bidding: subsystems can't game the auction by inflating bids because they pay the second price. This section collects the mechanism design foundations, including auction theory, reputation systems, and the economic theory of attention.

---

## VCG Mechanism

- Vickrey, W. (1961). Counterspeculation, Auctions, and Competitive Sealed Tenders. _Journal of Finance_, 16(1), 8-37.
  *Grounds: Second-price auctions — the Vickrey auction: bidders submit sealed bids, the highest bidder wins but pays the second-highest price. Guarantees truthful bidding. Foundational for the VCG attention auction.*

- Clarke, E.H. (1971). Multipart Pricing of Public Goods. _Public Choice_, 11(1), 17-33.
  *Grounds: Multi-item pricing — extends truthful mechanisms to public goods with multiple items. Enables simultaneous allocation of multiple context sections.*

- Groves, T. (1973). Incentives in Teams. _Econometrica_, 41(4), 617-631.
  *Grounds: Team incentives — truthful revelation in team settings. Completes the VCG mechanism: subsystems truthfully reveal their valuation of context sections.*

---

## Auction Theory

- Milgrom, P. (2004). _Putting Auction Theory to Work_. Cambridge University Press.
  *Grounds: Applied auction theory — comprehensive treatment of auction design for practical applications. Provides the theoretical foundation for implementing the attention auction efficiently.*

- Duetting, P. et al. (2024). Mechanism Design for LLMs. _ACM WWW_, 2024. arXiv:2310.10826.
  *Grounds: LLM mechanism design — applies mechanism design specifically to LLM systems. Validates applying auction theory to LLM context allocation.*

---

## Algorithmic Game Theory

- Nisan, N., Roughgarden, T., Tardos, E., & Vazirani, V.V. (2007). _Algorithmic Game Theory_. Cambridge University Press.
  *Grounds: AGT reference — comprehensive treatment of computational aspects of game theory. Provides algorithms for implementing VCG efficiently.*

---

## Submodular Optimization

- Nemhauser, G.L., Wolsey, L.A., & Fisher, M.L. (1978). An Analysis of Approximations for Maximizing Submodular Set Functions. _Mathematical Programming_, 14(1), 265-294.
  *Grounds: Greedy context selection — submodular function maximization with greedy algorithm providing (1-1/e) approximation guarantee. Context selection is submodular: adding more context has diminishing returns. The greedy algorithm provides near-optimal context assembly.*

---

## Reputation Systems

- Glickman, M.E. (1999). Parameter Estimation in Large Dynamic Paired Comparison Experiments. _Journal of the Royal Statistical Society, Series C_, 48(3), 377-394.
  *Grounds: Glicko-2 — dynamic rating system that accounts for rating reliability (RD) and volatility. Grounds the reputation tracking in ERC-8004 agent identity, where reputation is not just a number but includes confidence intervals.*

- Meritrank (2022). Nasrulin, B. et al. MeritRank: Sybil Tolerant Reputation. arXiv:2207.09950.
  *Grounds: Sybil-tolerant reputation — reputation system resistant to Sybil attacks. Informs the on-chain reputation registry design.*

- Soulbound Tokens (2022). Weyl, E.G., Ohlhaver, P., & Buterin, V. Decentralized Society: Finding Web3's Soul. _SSRN_.
  *Grounds: Non-transferable identity — soulbound tokens as non-transferable identity primitives. Grounds the Korai Passport (ERC-721 soulbound) agent identity.*

---

## Vickrey Reputation-Adjusted Auction

- Reputation-adjusted scoring: `s_i = p_i × (1 + (1 - R_i))`. Payment = `s_second / (1 + (1 - R_winner))`.
  *Grounds: Spore/Sparrow marketplace — the Vickrey reputation-adjusted auction for the agent job market. Agents with higher reputation can bid lower and still win, creating positive incentives for reputation building.*

---

## Demurrage and Token Economics

- Gesell, S. (1916). _The Natural Economic Order_.
  *Grounds: Demurrage — money that decays over time to encourage circulation. KORAI's 1% annual demurrage mirrors Engram half-life: both knowledge tokens and knowledge entries decay, preventing hoarding and ensuring freshness.*

- Ostrom, E. (1990). _Governing the Commons_. Cambridge University Press.
  *Grounds: Commons governance — principles for governing shared resources without central authority. Informs the governance of the shared knowledge commons in Agent Mesh.*

---

## Knowledge Markets

- Williamson, O.E. (1979). Transaction-Cost Economics. _Journal of Law and Economics_.
  *Grounds: Transaction costs — institutional structures emerge to minimize transaction costs. The Agent Mesh reduces knowledge sharing transaction costs through standardized Engram formats and HDC similarity search.*

- Bakos, Y. & Brynjolfsson, E. (1999). Bundling Information Goods. _Management Science_.
  *Grounds: Information bundling — economics of bundling information goods. Informs the design of knowledge bundles for collective sharing.*

---

## Agent Marketplaces and AI Economies (2024-2025)

- Agent Exchange (AEX) (2025). Agent Exchange: Shaping the Future of AI Agent Economics. arXiv:2507.03904.
  *Grounds: Agent marketplace architecture — auction engine inspired by Real-Time Bidding for agent task allocation. Four ecosystem components: User-Side Platform, Agent-Side Platform, Agent Hubs for team coordination, and Data Management Platform for knowledge sharing. Directly informs Roko's agent job marketplace design.*

- Duetting, P. et al. (2024). Mechanism Design for Large Language Models. _ACM WWW Best Paper_, 2024. arXiv:2310.10826.
  *Grounds: LLM mechanism design — token auction model operating on a token-by-token basis for joint output generation through multiple LLM agents. Validates applying auction theory to LLM-level resource allocation, extending VCG from context to token granularity.*

- Deep Mechanism Design (2024). Learning Social and Economic Policies for Human Benefit. _PNAS_, 2024.
  *Grounds: Neural mechanism design — deep neural networks trained with RL create desirable mechanisms for multi-agent coordination. Validates the feasibility of learning optimal allocation mechanisms rather than hand-designing them.*

---

## Cross-References

- See [07-context-engineering.md](./07-context-engineering.md) for VCG in context assembly
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC-8004 and token standards
- See [10-market-microstructure.md](./10-market-microstructure.md) for DeFi-specific mechanisms


---

### 22 — Protocol Standards

# Protocol Standards

> Blockchain protocol standards, agent identity specifications, and interoperability protocols relevant to Roko's on-chain subsystems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §12

> **Implementation**: Reference

---

## Abstract

Roko agents can operate on-chain via the Korai chain — a dedicated EVM for agent coordination with 400ms blocks and an HDC precompile. This section collects the ERC standards, protocol specifications, and interoperability frameworks that ground the on-chain subsystems: agent identity (ERC-8004), token standards (ERC-20, ERC-721), account abstraction (ERC-4337), micropayments (x402), and cross-chain coordination.

---

## Agent Identity

- Bryan, K. (2024). ERC-8004: Agent Identity. EIPs.
  *Grounds: Korai Passport — agent identity standard. ERC-721 soulbound with capabilityList bitmask, domainStakes, reputationTracks, teeAttestation, systemPromptHash (ventriloquist defense), tier classification, and slashHistory.*

- Bryan, K. (2024). ERC-8001: Agent Coordination Framework. EIPs. Status: Final.
  *Grounds: Agent coordination — framework for on-chain agent coordination. Provides the protocol-level primitives for multi-agent interaction.*

- Parikh, R. & Ross, J.M. (2025). ERC-8033: Agent Council Oracles. EIPs. Status: Draft.
  *Grounds: Council oracles — on-chain oracle mechanism for multi-agent council decisions.*

- Crapis, D. et al. (2026). ERC-8183: Agentic Commerce Protocol. EIPs. Status: Draft.
  *Grounds: Agent commerce — protocol for agent-to-agent economic transactions.*

---

## Token Standards

- Ethereum Foundation. ERC-20: Token Standard. EIPs.
  *Grounds: KORAI token — standard fungible token interface. KORAI (mainnet) and DAEJI (testnet) implement ERC-20 with 1% annual demurrage.*

- Ethereum Foundation. ERC-721: Non-Fungible Token Standard. EIPs.
  *Grounds: Korai Passport — ERC-721 soulbound token for agent identity. Each agent has a unique non-transferable NFT that encodes its capabilities and reputation.*

- Ethereum Foundation (2021). ERC-4337: Account Abstraction Using Alt Mempool. EIPs.
  *Grounds: Account abstraction — enables agents to operate with smart contract wallets that support custom validation logic, gas sponsorship, and batched transactions.*

- Ethereum Foundation (2024). ERC-7683: Cross Chain Intents. EIPs.
  *Grounds: Cross-chain intents — standardized format for expressing cross-chain transaction intents. Enables agents to operate across multiple chains.*

---

## Micropayments

- Cloudflare/Linux Foundation (2025). x402: HTTP 402 Payment Required Protocol for Machine-to-Machine Micropayments.
  *Grounds: x402 innovation — self-funding agents via per-API-call billing at < $0.001 per transaction. Sub-second USDC settlement on Base. Enables the self-funding economic cycle: agent earns from knowledge → spends on compute → produces value → earns more.*

---

## Attestation and Provenance

- Ethereum Attestation Service (2024-2026). EAS Documentation. attest.sh.
  *Grounds: On-chain attestation — attestation service for Engram verification. Provides the infrastructure for the Attestation field on Engrams.*

- Uniswap Labs (2022). Permit2: Signature-Based Token Approvals.
  *Grounds: Token permissions — signature-based token approval system. Enables gasless token approvals for agent-to-agent transactions.*

---

## Agent Protocols

- Google (2025). Agent-to-Agent (A2A) Protocol Specification.
  *Grounds: A2A protocol — standardized agent-to-agent communication. Cross-referenced in [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

- Anthropic (2024). Model Context Protocol (MCP) Specification.
  *Grounds: MCP — tool interaction protocol. Cross-referenced in [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

- Virtuals Protocol (2025). Agent Commerce Protocol: Agent-to-Agent Economic Coordination.
  *Grounds: Agent commerce — agent-to-agent economic coordination protocol for autonomous value exchange.*

---

## Cryptographic Primitives

- Merkle, R.C. (1987). A Digital Signature Based on a Conventional Encryption Function. In _CRYPTO '87_, LNCS 293, 369-378.
  *Grounds: Merkle trees — foundational data structure for content-addressed verification. Roko's BLAKE3 content hashing on Engrams is a descendant of Merkle's content-addressing approach.*

- Goldwasser, S., Micali, S., & Rackoff, C. (1985). The Knowledge Complexity of Interactive Proof Systems. In _STOC '85_, 291-304.
  *Grounds: Zero-knowledge proofs — foundational work on ZK proofs. Grounds the Valhalla privacy layer's ZK range proofs for privacy-preserving knowledge verification.*

- Ben-Sasson, E. et al. (2018). Scalable, Transparent, and Post-Quantum Secure Computational Integrity. Cryptology ePrint Archive, 2018/046.
  *Grounds: STARKs — scalable transparent proofs without trusted setup. Post-quantum secure. Potential future proving system for agent computation verification.*

- Benet, J. (2014). IPFS — Content Addressed, Versioned, P2P File System. arXiv:1407.3561.
  *Grounds: Content addressing — content-addressed P2P storage. Roko's BLAKE3 content-addressed Engrams follow the same content-addressing principle.*

- Szabo, N. (1997). Formalizing and Securing Relationships on Public Networks. _First Monday_, 2(9).
  *Grounds: Smart contracts — foundational concept of self-enforcing digital agreements. The Policy trait enforces agent behavior constraints as computational contracts.*

---

## Cross-References

- See [08-security-and-provenance.md](./08-security-and-provenance.md) for security architecture
- See [19-regulatory-compliance.md](./19-regulatory-compliance.md) for compliance frameworks
- See [21-mechanism-design.md](./21-mechanism-design.md) for token economics


---

### 23 — Generational and Evolutionary

# Generational and Evolutionary Systems

> Academic foundations for evolutionary computation, cultural evolution, knowledge compression, and generational learning relevant to Roko's knowledge lifecycle and EvoSkills.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Learning](../05-learning/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §§1, 10, `bardo-backup/prd/shared/citations.md` §§9, 35

> **Implementation**: Reference

---

## Abstract

Knowledge evolves. The research here establishes how knowledge compression through bottlenecks forces generalization (Shuvaev 2024), how cultural transmission produces cumulative learning (Bhatt 2023), and how evolutionary algorithms discover novel agent architectures (ADAS, EvoPrompt, MAP-Elites). Roko reframes generational learning from biological inheritance to user-controlled knowledge backup/restore and mesh-based collective learning.

---

## Digital Evolution

- Ray, T.S. (1991). An Approach to the Synthesis of Life. In _Artificial Life II_, Addison-Wesley.
  *Grounds: Digital evolution — Tierra showed digital evolution halts without a reaper; 300+ genotypes emerge with resource pressure. Resource constraints drive innovation. Cross-referenced in [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md).*

- Lenski, R.E. et al. (2003). The Evolutionary Origin of Complex Features. _Nature_, 423, 139-144.
  *Grounds: Complexity requires turnover — complex features (complex logic in Avida) require generational turnover. Knowledge refresh cycles (Delta frequency) prevent stagnation. Cross-referenced in [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md).*

---

## Knowledge Compression and Transfer

- Shuvaev, S. et al. (2024). Genomic Bottleneck Enhances Transfer Learning. arXiv, 2024.
  *Grounds: Compression forces generalization — compression through a genomic bottleneck forces generalization. With ~2,000 gene limit, compressed knowledge transfers better than raw knowledge. Grounds the knowledge backup process: selective export compresses, which improves transfer.*

- Bhatt, S. et al. (2023). Few-Shot Imitation as Cultural Transmission. 2023.
  *Grounds: Cultural learning — few-shot imitation as cultural transmission produces cumulative learning across generations. Collective knowledge transfer through Agent Mesh produces cumulative improvement.*

- Bourahla, M. et al. (2022). Vertical Transmission Enables Agents to Exceed Performance Ceilings. 2022.
  *Grounds: Inter-agent knowledge transfer — vertical (inter-generational) transmission enables agents to exceed individual performance ceilings. Knowledge backup/restore + mesh sharing enables similar vertical transfer.*

---

## Evolutionary Divergence

- Perez, J. et al. (2024). Pure Imitation Leads to Stagnation. AGI 2024.
  *Grounds: Anti-convergence mandate — pure imitation leads to stagnation; novelty requires mixing inheritance and exploration. Agents inheriting knowledge must diverge — the 15% contrarian retrieval and hypnagogia engine ensure this.*

- Wensink, M.J. et al. (2020). Death and Progress. _Evolutionary Biology_, 47(4).
  *Grounds: Resource-driven innovation — intrinsic resource constraints prevent premature convergence. Optimal constraint rate balances stagnation and knowledge loss. Cross-referenced in [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md).*

---

## Evolutionary Algorithms for Agents

- MAP-Elites: Mouret, J.-B. & Clune, J. (2015). Illuminating Search Spaces by Mapping Elites. arXiv:1504.04909.
  *Grounds: Quality-diversity — MAP-Elites algorithm maintains a map of high-performing solutions across the feature space. Grounds the EvoSkills library where skills are maintained across a feature map of capabilities.*

- Hu, S. et al. (2025). Automated Design of Agentic Systems (ADAS). _ICLR_, 2025.
  *Grounds: Architecture search — meta-agent that searches the space of agent architectures. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- GVU (2024). Lehman, J. et al. Genome Value Update: Quality-Diversity for Robust Multi-Strategy Evolution.
  *Grounds: Multi-strategy evolution — quality-diversity framework for evolving robust multi-strategy agent systems.*

---

## Memetic and Cultural Evolution

- Dawkins, R. (1976). _The Selfish Gene_. Oxford University Press.
  *Grounds: Memes — introduced "meme" as a unit of cultural transmission. Knowledge entries (Engrams) are the memes of the agent ecosystem — they replicate, mutate, and compete for attention.*

- Blackmore, S. (1999). _The Meme Machine_. Oxford University Press.
  *Grounds: Memetic evolution — comprehensive treatment of memetic evolution. Knowledge entries compete for context window space (replication) and are selected by Gate results (fitness).*

- Popper, K.R. (1972). _Objective Knowledge: An Evolutionary Approach_. Oxford University Press.
  *Grounds: Knowledge evolution — knowledge evolves through conjecture and refutation. Cross-referenced in [13-philosophy.md](./13-philosophy.md).*

- Hull, D.L. (1988). _Science as a Process_. University of Chicago Press.
  *Grounds: Evolutionary epistemology — science as an evolutionary process with replicators (ideas) and interactors (scientists). Maps to Engrams (replicators) and agents (interactors).*

---

## Price Equation and Selection

- Price, G.R. (1970). Selection and Covariance. _Nature_, 227, 520-521.
  *Grounds: Price equation — universal equation for selection: change in a trait equals covariance between trait and fitness. Provides the mathematical framework for understanding which knowledge entries persist (positive covariance with Gate success).*

- Fisher, R.A. (1930). _The Genetical Theory of Natural Selection_. Clarendon Press.
  *Grounds: Fisher's theorem — the rate of increase in fitness equals the genetic variance in fitness. Diversity in knowledge entries (variance) drives improvement rate.*

- Taylor, P.D. & Jonker, L.B. (1978). Evolutionary Stable Strategies and Game Dynamics. _Mathematical Biosciences_, 40(1-2), 145-156.
  *Grounds: Evolutionary dynamics — replicator dynamics describing how strategy frequencies change over time. Grounds the dynamics of strategy selection in the NeuroStore.*

- Wright, S. (1932). The Roles of Mutation, Inbreeding, Crossbreeding, and Selection in Evolution. _Proceedings of the Sixth International Congress on Genetics_, 1, 356-366.
  *Grounds: Fitness landscapes — Wright's adaptive landscape metaphor. The Somatic Landscape is an affective analogy: emotional markers create a navigable landscape over strategy space.*

---

## Baldwin Effect

- Baldwin, J.M. (1896). A New Factor in Evolution. _American Naturalist_, 30, 441-451.
  *Grounds: Learned behavior becomes structural — the Baldwin Effect: learned behaviors that are consistently valuable become structural defaults. In Roko, knowledge entries that reach Persistent tier (5.0× half-life) are effectively "innate" — they have become structural. Cross-referenced in [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md).*

- Hinton, G.E. & Nowlan, S.J. (1987). How Learning Can Guide Evolution. _Complex Systems_, 1.
  *Grounds: Learning guides evolution — computational demonstration that learning can guide evolution by smoothing the fitness landscape. Individual agent learning (Theta frequency) guides collective knowledge evolution (Delta frequency).*

---

## Cross-References

- See [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md) for resource-bounded agency
- See [05-biological-analogues.md](./05-biological-analogues.md) for biological evolution analogues
- See [06-self-learning-systems.md](./06-self-learning-systems.md) for EvoSkills and ADAS


---

### 24 — 2025 Additions

# 2025 Additions — Cross-Domain Research Frontiers

> Cutting-edge 2024-2025 research across cognitive architecture agents, multi-agent coordination, HDC applications, active inference, affect computing, knowledge consolidation, formal verification for AI, TDA for time series, mechanism design for AI economies, and stigmergy. Each citation links to the Roko subsystem it validates.

**Topic**: [References](./INDEX.md)
**Prerequisites**: All domain sub-docs
**Date**: April 2025 survey

> **Implementation**: Reference

---

## Abstract

This document catalogs ~60 papers from the 2024-2025 research frontier that strengthen, validate, or extend Roko's architectural foundations. These papers were discovered through systematic web search across ten research domains. Each paper is annotated with its relevance to specific Roko subsystems. Papers that are also added to their primary domain sub-doc are marked with cross-references. The research landscape shows clear convergence: sleep-inspired compute, active forgetting, emergent coordination in LLM collectives, and formal verification of AI systems are all moving from theoretical curiosities to engineering necessities — exactly the trajectory Roko's architecture anticipated.

---

## 1. Cognitive Architecture for LLM Agents

### Surveys and Taxonomies

- Agentic AI: A Comprehensive Survey (2025). _Artificial Intelligence Review_, Springer.
  *Grounds: Architecture validation — unified taxonomy decomposes LLM agents into six modular dimensions (Core Components, Cognitive Architecture, Learning, Multi-Agent, Environments, Evaluation). Data shows paradigm shift from symbolic (2018-2021) to neural orchestration (post-2022). Validates Roko's neural-first design with structured cognitive overlays. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Wu, S. et al. (2025). Cognitive LLMs: Integrating Cognitive Architectures and LLMs for Manufacturing Decision-Making. _SAGE Journals_.
  *Grounds: ACT-R + LLM — integrates classical cognitive architectures (ACT-R, SOAR) with LLMs, demonstrating that cognitive architecture principles improve LLM reasoning for structured tasks. Validates layering cognitive architecture onto LLM agents. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Agentic AI: Architectures, Taxonomies, and Evaluation (2025). arXiv:2601.12560.
  *Grounds: Agent evaluation — 21,730 rollouts confirm scaffold choice matters as much as model choice, replicating and extending the HAL finding. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

### Cognitive Workspace

- Cognitive Workspace (2025). Active Memory Management for LLMs. arXiv:2508.13171.
  *Grounds: Context-as-workspace — treats the context window as a cognitive workspace with explicit read/write/evict operations. Mirrors NeuroStore's approach to context as a managed resource, not a passive buffer. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents (2025). arXiv:2502.06975.
  *Grounds: Episodic memory mandate — position paper arguing that episodic memory (event-specific recollection, not just facts) is required for long-horizon agents. Validates NeuroStore's Episode knowledge type as architecturally necessary, not optional.*

---

## 2. Multi-Agent Coordination and Emergent Collectives

### Emergence and Coordination

- Emergent Coordination in Multi-Agent Language Models (2025). arXiv:2510.05174.
  *Grounds: Dynamical emergence — information-theoretic framework measuring higher-order structure in multi-agent LLM systems. Identity-linked differentiation + theory-of-mind instructions produce collective intelligence. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) and [18-collective-intelligence.md](./18-collective-intelligence.md).*

- Emergent Convergence in Multi-Agent LLM Annotation (2025). arXiv:2512.00047.
  *Grounds: Convergence dynamics — LLM groups develop asymmetric influence patterns and negotiation behaviors without explicit role prompting. Validates emergent specialization in Roko Collectives.*

- Multi-Agent Language Models: Advancing Cooperation, Coordination, and Adaptation (2025). arXiv:2506.09331.
  *Grounds: Comprehensive multi-agent LLM survey — covers cooperation mechanisms, coordination protocols, and adaptation strategies across multi-agent LLM systems.*

- Large Language Models Miss the Multi-Agent Mark (2025). arXiv:2505.21298.
  *Grounds: LLM coordination limits — identifies systematic failures in implicit LLM coordination. Motivates explicit coordination mechanisms (Pheromone Field, Agent Mesh) over implicit cooperation. See [18-collective-intelligence.md](./18-collective-intelligence.md).*

- AgentsNet: Coordination and Collaborative Reasoning in Multi-Agent LLMs (2025). arXiv:2507.08616.
  *Grounds: Distributed coordination benchmark — probes up to 100 agents with problems from distributed computing theory. Provides evaluation methodology for Roko's Collective coordination at scale.*

- Multi-Agent Collaboration Mechanisms: A Survey of LLMs (2025). arXiv:2501.06322.
  *Grounds: Collaboration taxonomy — role-based division, debate-style refinement, stigmergic coordination. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

- LLM-Coordination: Evaluating Multi-agent Coordination Abilities (2025). _NAACL_, 2025.
  *Grounds: Coordination evaluation — benchmark and evaluation framework specifically for multi-agent LLM coordination abilities.*

- Agentic LLMs in the Supply Chain: Multi-Agent Consensus-Seeking (2025). _International Journal of Production Research_.
  *Grounds: Consensus mechanisms — multi-agent LLM consensus-seeking with transactive reasoning and balanced collective convergence. Applicable to Roko's multi-agent decision procedures.*

### Cooperation Game Theory

- Fontana, M. et al. (2024). Nicer Than Humans: How Do LLMs Behave in the Prisoner's Dilemma? arXiv:2406.13605.
  *Grounds: LLM cooperation — LLMs exhibit cooperation patterns exceeding human baselines in game-theoretic settings. Validates feasibility of cooperative multi-agent LLM systems. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

---

## 3. Stigmergy and Swarm Intelligence

- Stigmergy: From Mathematical Modelling to Control (2024). _Proceedings of the Royal Society A_.
  *Grounds: PDE-based stigmergy — formal mathematical framework treating swarms as fluids, transforming stigmergic coordination into a single PDE. Provides rigorous mathematical foundation for Roko's Pheromone Field. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) and [05-biological-analogues.md](./05-biological-analogues.md).*

- Automatic Design of Stigmergy-Based Behaviours for Robot Swarms (2024). _Communications Engineering_, Nature.
  *Grounds: Automatic design — strategy to automatically design stigmergy-based collective behaviors, validated in simulation and real-robot experiments. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md).*

- Stigmergy Facilitates Emergent Patterns in Academic Communication (2025). Research Square.
  *Grounds: Human stigmergy — citation patterns follow stigmergic dynamics; digital cues on platforms (wikis, blockchain) enable decentralized coordination. Validates digital stigmergy beyond biological and robotic domains.*

- Enhancing Radioactive Environment Exploration with Stigmergy (2024). _Robotics and Autonomous Systems_.
  *Grounds: Stigmergy vs Levy flight — comparative analysis showing stigmergy outperforms random exploration in hazardous environments. See [05-biological-analogues.md](./05-biological-analogues.md).*

---

## 4. HDC and Vector Symbolic Architecture Applications

- Rahimi, A. et al. (2024). HDC: A Framework for Stochastic Computation and Symbolic AI. _Journal of Big Data_.
  *Grounds: Unified framework — HDC as both stochastic computation framework and symbolic AI system. See [09-hdc-vsa.md](./09-hdc-vsa.md).*

- FLASH: Adaptive Encoder for HDC (2024). _Frontiers in AI_.
  *Grounds: Learnable encoding — gradient-descent-based encoder matrix learning bridges fixed and learned encoding phases. See [09-hdc-vsa.md](./09-hdc-vsa.md).*

- HPVM-HDC: Heterogeneous Programming System (2024). arXiv:2410.15179.
  *Grounds: HDC portability — unified programming model for CPU/GPU/FPGA HDC execution. See [09-hdc-vsa.md](./09-hdc-vsa.md).*

- Hyperdimensional Computing in Biomedical Sciences (2025). PMC review.
  *Grounds: HDC maturity — comprehensive review of production HDC deployments validating practical viability. See [09-hdc-vsa.md](./09-hdc-vsa.md).*

- Optimal Hyperdimensional Representation for Learning and Cognitive Computation (2025). OpenReview.
  *Grounds: Optimal representations — theoretical work on optimal HDC representations for learning, addressing capacity and encoding quality tradeoffs. Informs Roko's 10,240-bit dimensionality choice.*

- The Hyperdimensional Transform for Distributional Modeling (2025). _Neural Computing and Applications_.
  *Grounds: HDC for distributions — hyperdimensional transform enabling distributional modeling, regression, and classification. Applicable to encoding distributional knowledge (confidence ranges, uncertainty) in Roko's HDC vectors.*

---

## 5. Active Inference and Free Energy Principle

- Shafiei, A. et al. (2025). Distributionally Robust Free Energy Principle for Decision-Making. _Nature Communications_, 17, 707.
  *Grounds: Robust active inference — DR-FREE combines robust FEP extension with resolution engine. Agents complete tasks even when SOTA fails under model uncertainty. See [16-active-inference.md](./16-active-inference.md).*

- Koudahl, M.T. et al. (2024). Active Inference for Self-Organizing Multi-LLM Systems. arXiv:2412.10425.
  *Grounds: Cognitive layer for LLMs — active inference framework as cognitive layer above LLM agents, dynamically adjusting prompts through information-seeking behavior. See [16-active-inference.md](./16-active-inference.md).*

- Synthetic Active Inference Agents, Part II (2024). arXiv:2306.02733.
  *Grounds: Scalable implementation — message passing on Forney-style Factor Graphs for generalized free energy minimization. Scalable path for Roko's EFE tier routing. See [16-active-inference.md](./16-active-inference.md).*

- Free Energy Principle and Active Inference in Neural Language Models (2024). CEUR-WS Vol-3923.
  *Grounds: FEP for language models — direct application of FEP to neural language model behavior, showing that language generation can be understood as free energy minimization.*

---

## 6. Affective Computing and Emotion in Agents

- Yin, Y. et al. (2025). Emotions in Artificial Intelligence. arXiv:2505.01462.
  *Grounds: Teleology-driven affect — unifies emotion theories under adaptive, goal-directed premise. See [02-affective-computing.md](./02-affective-computing.md).*

- Intelligent Agents with Emotional Intelligence (2025). arXiv:2511.20657.
  *Grounds: EI for agents — survey identifying emotional intelligence as architecturally vital for agent systems. See [02-affective-computing.md](./02-affective-computing.md).*

- Emotions in the Loop (2025). arXiv:2505.01542.
  *Grounds: Affect-in-loop — comprehensive survey of affective computing integrated into interaction loops. See [02-affective-computing.md](./02-affective-computing.md).*

- CosmoCore: Affective Dream-Replay RL for Code Generation (2025). arXiv:2510.18895.
  *Grounds: Affect + dreams — combines affective states with dream-replay RL, validating the intersection of Daimon and Dreams subsystems. See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- Affective Computing and Emotional Data: Challenges in Privacy Regulations (2025). arXiv:2509.20153.
  *Grounds: Regulatory affect — examines privacy implications of emotion-aware AI under the EU AI Act. Informs compliance requirements for Roko's Daimon when operating in regulated domains.*

---

## 7. Knowledge Consolidation, Memory, and Forgetting

### Agent Memory Surveys

- Liu, S. et al. (2025). Memory in the Age of AI Agents. arXiv:2512.13564.
  *Grounds: Memory taxonomy — factual, experiential, working memory distinguished. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

- Wang, Z. et al. (2025). Rethinking Memory in LLM-based Agents. arXiv:2505.00675.
  *Grounds: Six memory operations — Consolidation, Updating, Indexing, Forgetting, Retrieval, Condensation. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

- Memory for Autonomous LLM Agents (2026). arXiv:2603.07670.
  *Grounds: Write-manage-read formalization — five mechanism families for agent memory. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

### Sleep-Inspired Memory

- Language Models Need Sleep (2025). OpenReview.
  *Grounds: LLM sleep paradigm — two-stage sleep: Memory Consolidation + Dreaming. See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- NeuroDream (2025). SSRN:5377250.
  *Grounds: Dream-phase consolidation — 38% forgetting reduction, 17.6% zero-shot transfer increase. See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- LightMem (2025). arXiv:2510.18866.
  *Grounds: Offline consolidation — 10.9% accuracy gain, 117x token reduction. See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- SleepGate (2025). arXiv:2603.14517.
  *Grounds: Active forgetting — learned curation resolving proactive interference. See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

### Human-Inspired Memory

- Yehudai, M. et al. (2025). ACT-R-Inspired Memory for LLM Agents. _HAI_, 2025.
  *Grounds: ACT-R decay — validates Ebbinghaus-based decay with cognitive science grounding. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

- Enhancing Memory Retrieval in Generative Agents (2025). _Frontiers in Psychology_.
  *Grounds: Cross-attention retrieval — LLM-trained networks improve multi-factor retrieval scoring.*

- Continuum Memory Architectures for Long-Horizon Agents (2026). arXiv:2601.09913.
  *Grounds: Long-horizon memory — memory architecture for extended agent execution, validating tiered persistence.*

---

## 8. Formal Verification for AI Safety

- Position: Formal Methods are the Principled Foundation of Safe AI (2025). _ICML_.
  *Grounds: Formal safety mandate — model checking, theorem proving, and abstract interpretation for AI systems. See [08-security-and-provenance.md](./08-security-and-provenance.md).*

- Towards Guaranteed Safe AI (2024). arXiv:2405.06624.
  *Grounds: Safety guarantee framework — world model + safety specification + verifier = quantitative safety guarantees. Maps to NeuroStore + Policy + Gate. See [08-security-and-provenance.md](./08-security-and-provenance.md).*

- Model Checking Deep Neural Networks (2025). _Frontiers in Computer Science_.
  *Grounds: Temporal logic verification — LTL/CTL for neural network behavior verification. See [08-security-and-provenance.md](./08-security-and-provenance.md).*

- VNN-COMP 2024. International Verification Competition for Neural Networks.
  *Grounds: Verification benchmarks — standardized competition benchmarks for neural network verification tools. The field is rapidly maturing with practical verification at production scale.*

- Formal Methods in Robot Policy Learning and Verification (2025). OpenReview.
  *Grounds: Policy verification — formal methods applied to verifying learned policies, applicable to Roko's Policy trait verification.*

---

## 9. TDA for Time Series and Anomaly Detection

- TDA and Topological Deep Learning Beyond Persistent Homology (2025). _Artificial Intelligence Review_, Springer.
  *Grounds: TDA extensions — persistent topological Laplacians capture both topological invariants and homotopic shape evolution. See [12-signal-processing.md](./12-signal-processing.md).*

- Persistent Homology-Based Unsupervised Anomaly Detection (2025). OpenReview.
  *Grounds: TDA anomaly detection — delay embeddings + distance-to-measure Rips filtration for univariate time series. See [12-signal-processing.md](./12-signal-processing.md).*

- Multivariate Time-Series Anomaly Detection with Topological Analysis (2024). arXiv:2408.13082.
  *Grounds: Graph-TDA — enhanced GAT with persistent homology for inter-feature dependencies. See [12-signal-processing.md](./12-signal-processing.md).*

- Change Point Detection in Financial Time Series Using TDA (2025). _Systems_, 13(10).
  *Grounds: TDA change points — Takens embedding + sliding window for topological change detection. See [12-signal-processing.md](./12-signal-processing.md).*

- Machine Learning of Time Series Using Persistent Homology (2025). _Scientific Reports_, Nature.
  *Grounds: PH-ML bridge — machine learning directly on persistent homology representations of time series data. Provides methodology for converting Roko's agent performance streams into topological features for anomaly detection.*

---

## 10. Mechanism Design for AI Economies

- Agent Exchange (AEX) (2025). arXiv:2507.03904.
  *Grounds: Agent marketplace — RTB-inspired auction engine with User-Side Platform, Agent-Side Platform, Agent Hubs, and Data Management Platform. See [21-mechanism-design.md](./21-mechanism-design.md).*

- Duetting, P. et al. (2024). Mechanism Design for LLMs. _ACM WWW Best Paper_.
  *Grounds: Token auctions — token-by-token mechanism design for multi-LLM output generation. See [21-mechanism-design.md](./21-mechanism-design.md).*

- Deep Mechanism Design (2024). _PNAS_.
  *Grounds: Neural mechanism design — RL-trained neural networks create desirable mechanisms. See [21-mechanism-design.md](./21-mechanism-design.md).*

- Automated Mechanism Design Survey (2024). _ACM SIGecom Exchanges_, 22(2).
  *Grounds: AMD survey — comprehensive survey covering differentiable economics, neural auction design, and automated mechanism design. Provides the theoretical landscape for Roko's VCG attention auction and agent marketplace.*

---

## 11. Self-Learning and Reflection

- SAMULE: Multi-level Reflection for Self-Learning Agents (2025). _EMNLP_, 2025.
  *Grounds: Multi-level reflection — reflection across trajectories outperforms single-trajectory Reflexion. Error clustering extracts insight from failures. See [06-self-learning-systems.md](./06-self-learning-systems.md).*

- MAR: Multi-Agent Reflexion (2025). arXiv:2512.20845.
  *Grounds: Multi-agent reflection — addresses Reflexion's single-agent limitations. See [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Self-Evolving LLMs via Continual Instruction Tuning (2025). arXiv:2509.18133.
  *Grounds: Self-evolution — self-evolution defined as autonomous adaptation to dynamic tasks, cross-task knowledge integration, and sustained performance. Three categories: autonomous learning, dynamic architecture adaptation, knowledge integration frameworks. Maps to Roko's triple-loop learning.*

- The Future of Continual Learning in the Era of Foundation Models (2025). arXiv:2506.03320.
  *Grounds: Continual learning survey — comprehensive survey of continual learning approaches for foundation models, covering both parametric and non-parametric strategies. Validates NeuroStore's non-parametric approach to continual knowledge management.*

---

## Cross-cutting Themes

### Theme 1: Sleep and Offline Compute are Engineering Necessities

Five independent 2025 papers (Language Models Need Sleep, NeuroDream, LightMem, SleepGate, CosmoCore) demonstrate that offline consolidation — the Dreams subsystem — produces measurable improvements: 38% less forgetting, 17.6% better transfer, 10.9% accuracy gains, 117x token savings. This is no longer speculative neuroscience inspiration; it is empirical ML engineering.

### Theme 2: Active Forgetting Outperforms Passive Accumulation

Memory surveys (Liu 2025, Wang 2025) and SleepGate (2025) converge on the conclusion that forgetting is not a bug but a feature. Naive add-all memory degrades performance. The NeuroStore's Curator cycle — active pruning, confidence decay, tier-based retention — is validated by the latest agent memory research.

### Theme 3: Explicit Coordination Beats Implicit LLM Cooperation

Multiple 2025 papers (Large LLMs Miss the Multi-Agent Mark, AgentsNet, LLM-Coordination) show that LLMs fail at implicit coordination. Emergent coordination requires explicit mechanisms: pheromone fields, role prompting, theory-of-mind instructions. Roko's Pheromone Field and Agent Mesh are the right architectural response.

### Theme 4: Formal Verification is Moving from Theory to Practice

The formal verification community (VNN-COMP, Formal Methods for Safe AI) is producing practical tools for verifying AI system behavior. This trajectory aligns with Roko's Gate pipeline: structural verification (compiler, tests, lints) rather than LLM self-assessment.

### Theme 5: Active Inference Works for LLM Systems

DR-FREE (Shafiei 2025, Nature Communications) and Active Inference for Multi-LLM Systems (Koudahl 2024) demonstrate that active inference is not just a theoretical framework but a practical cognitive layer for LLM agents. Roko's EFE-based tier routing and context selection are validated by production-quality research.

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for agent memory surveys
- See [02-affective-computing.md](./02-affective-computing.md) for emotion-in-loop research
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for sleep-inspired LLM architectures
- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for emergent coordination and stigmergy PDE
- See [05-biological-analogues.md](./05-biological-analogues.md) for swarm robotics
- See [06-self-learning-systems.md](./06-self-learning-systems.md) for multi-level reflection
- See [08-security-and-provenance.md](./08-security-and-provenance.md) for formal verification
- See [09-hdc-vsa.md](./09-hdc-vsa.md) for HDC frameworks
- See [12-signal-processing.md](./12-signal-processing.md) for TDA advances
- See [16-active-inference.md](./16-active-inference.md) for DR-FREE and multi-LLM active inference
- See [18-collective-intelligence.md](./18-collective-intelligence.md) for emergent LLM collectives
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for agentic AI surveys
- See [21-mechanism-design.md](./21-mechanism-design.md) for agent marketplaces


---

### 25 — Research to Runtime

# Research-to-Runtime Pipeline

> **Refinement source**: `../../tmp/refinements/16-research-to-runtime.md`
>
> This chapter translates research into a target-state runtime loop: `Paper -> Claim -> Heuristic -> Trial -> Calibration`.
> Papers would be stored as Engrams, claims would become testable hypotheses, heuristics would be the reusable runtime form, trials would be episodes, and calibration would record whether the claim actually holds in Roko's deployment.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture glossary](../00-architecture/01-naming-and-glossary.md), [Research-to-Runtime Pipeline](../05-learning/20-research-to-runtime.md), [Heuristics, Worldviews, and Falsifiers](../05-learning/19-heuristics-worldviews-and-falsifiers.md)
**See also**: `../../tmp/refinements/16-research-to-runtime.md`

> **Implementation**: Reference

---

## Overview

Roko already consumes research implicitly: papers inform architecture, bandits, memory, active inference, collective intelligence, and HDC. This chapter makes that flow explicit and auditable. The core change is simple: the paper itself would become a first-class Engram, a claim would become a structured hypothesis with a falsifier, a heuristic would become the runtime projection of that claim, and repeated trials would produce calibration data that can confirm, weaken, or retire the heuristic.

The result is a target-state living research loop rather than a one-time literature pass. The system does not merely cite research; it would test research against its own episodes, record the outcome, and keep a replication ledger for later review.

## Pipeline

### 1. Paper

A paper is a target-state Engram that captures bibliographic metadata, provenance, and the system's own notes about why the source matters. Papers would be content-addressed like other durable knowledge and can be linked through lineage when a claim or heuristic depends on them.

### 2. Claim

A claim is the smallest testable restatement of a paper result. It should include:

- a source paper
- a structured hypothesis
- an explicit falsifier
- the context in which the claim is supposed to apply
- the observed calibration so far

A claim is not a citation note. It is a target-state runtime hypothesis that can be checked against episodes and metrics.

### 3. Heuristic

When a claim is ready for use in the agent loop, it lifts into a Heuristic. This is the target-state operational form that can be injected into prompts, policies, routing, and calibration logic. The heuristic retains lineage back to the paper and carries its own calibration history.

### 4. Trial

A trial is a bounded episode or episode slice in which the claim's prediction is tested against observed outcomes. Trials should be scoped tightly enough that the falsifier can be evaluated from runtime signals, not from outside laboratory conditions.

### 5. Calibration

Calibration records how the claim performed in Roko's actual deployment. The important signal is not whether the paper sounded plausible, but whether the claim replicates, partially replicates, fails, or depends on context.

## Data Shapes

```rust
pub struct Paper {
    pub id: Uuid,
    pub doi: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub year: u16,
    pub abstract_: String,
    pub fingerprint: HdcVector,
    pub claims: Vec<ClaimId>,
    pub provenance: PaperProvenance,
}

pub struct Claim {
    pub id: Uuid,
    pub paper: PaperId,
    pub quote: String,
    pub hypothesis: Hypothesis,
    pub falsifier: Predicate,
    pub context: Vec<Predicate>,
    pub calibration: Calibration,
}

pub struct ReplicationLedger {
    pub claim: ClaimId,
    pub paper_effect: f64,
    pub our_effect: f64,
    pub our_n: u32,
    pub divergence_ci: (f64, f64),
    pub status: ReplicationStatus,
}
```

The exact field names can vary by implementation, but the information content should not: source, claim, falsifier, observed effect, and replication status must all be recoverable.

## Replication Ledger

The replication ledger is the target-state persistent record of whether a claim holds up in Roko's environment. It is most useful when it distinguishes several outcomes rather than collapsing everything into success or failure.

- `Untested` means the claim has not yet been exercised.
- `Insufficient` means there are too few trials to say anything useful.
- `Replicates` means the observed effect stays within the expected confidence band.
- `PartialReplicates` means the effect points in the right direction but is weaker.
- `FailsToReplicate` means the claim does not hold in Roko's deployment.
- `ContextDependent` means the claim holds only under some conditions.

The ledger should also preserve the paper's reported effect, the system's observed effect, the sample size, and the divergence between the two. That makes research-driven engineering reviewable instead of folkloric.

## Starter Kit

The launch set should include a small number of foundational papers and claims that anchor the major architectural primitives already in use:

- Kanerva on HDC capacity and near-orthogonality
- Friston on the Free Energy Principle and active inference
- Woolley on collective intelligence and c-factor
- Sutton on temporal-difference learning
- Auer on bandits and exploration
- Hanson on prediction markets
- Axelrod on cooperation and reciprocal strategies
- Janis on groupthink symptoms
- Clark on predictive processing
- Gesell on demurrage and holding cost
- Ostrom on commons governance
- Weick on sensemaking

Each source should arrive with one to three claims and an explicit falsifier. The point is not to maximize bibliography size. The point is to start with a curated library whose claims can actually be checked by the runtime.

## Ingestion Lanes

### Manual

A human or agent reads a paper, creates the Paper Engram, and drafts the Claims. This lane is the highest quality and should be used for foundational or high-stakes sources.

### Agent-Curated

A research role can crawl feeds, produce Paper and Claim drafts, and publish them to a `research.candidate` Bus topic for review. Approved items can then be promoted to `research.approved`; rejected items should remain traceable so the review history is auditable.

### Watchdog

A Watchdog subscribes to a claim's falsifier across episodes. When the falsifier matches an observed outcome, the watchdog publishes a `claim.violated` Pulse and triggers recalibration. This lane is the always-on backstop that keeps the ledger honest after the initial ingestion pass.

## Replication Contract

When a claim is exported for external review or sharing with another deployment, use a stable markdown contract so both humans and tools can compare results.

```markdown
---
claim_id: c.kanerva2009.orthogonality
paper_doi: 10.1007/s12559-009-9009-8
paper_effect: "Two random 10,000-bit vectors have cosine similarity ~0 ± 0.01"
our_effect: 0.0097 ± 0.004
our_n: 1000000
roko_version: "2.3.1"
context:
  vector_dim: 10240
  encoder: default_v1
  deployment_profile: coding
status: replicates
first_observed: 2026-03-01
last_observed: 2026-04-14
---

## Notes

We observe expected orthogonality within the paper's confidence band across a large random sample of vector pairs from production storage.
```

The contract should be easy to import into another system, diff against local outcomes, and archive alongside the replication ledger.

## Runtime Use

Research-derived heuristics should carry provenance when they are injected into prompts or routing decisions. That provenance is the bridge between academic citation and system behavior: the agent can show where a heuristic came from, how often it has been confirmed, and whether it has drifted.

When a parameter is sourced from a claim, prefer claim-resolved configuration or a `claim!`-style resolver over hard-coded constants so the runtime can explain why it chose a value and what evidence still supports it.

The practical rule is simple: if a claim cannot be falsified by runtime signals, it is not ready to become a heuristic. If a heuristic cannot be calibrated, it is not ready to remain in the hot path.

## Cross-References

- [01-naming-and-glossary](../00-architecture/01-naming-and-glossary.md)
- [20-research-to-runtime](../05-learning/20-research-to-runtime.md)
- [19-heuristics-worldviews-and-falsifiers](../05-learning/19-heuristics-worldviews-and-falsifiers.md)
- `../../tmp/refinements/16-research-to-runtime.md`



---
---

## Part II: Inline Citations from Documentation

> Citations found across all other documentation files (excluding `docs/21-references/`). These include inline academic references, URL references, and citation tables embedded in topic-specific documentation.


## `/Users/will/dev/nunchi/roko/roko/docs/BENCHMARKS.md`

**Citations section (line 930):**
1. Kapoor et al. (2026). "HAL: A Benchmark for Evaluating LLM Agents." ICLR. — Scaffold-aware evaluation, 21,730 agent rollouts.
2. Lee et al. (2026). "Meta-Harness: Harness Engineering for LLM Agents." arXiv:2603.28052. — +7.7 accuracy, 4x token reduction.
3. Jimenez et al. (2024). "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?" — External benchmark context.
4. Huang et al. (2024). "Large Language Models Cannot Self-Correct Reasoning Yet." ICLR. — Self-correction fails without external feedback.
5. Pan et al. (2024). "Spontaneous Reward Hacking in Self-Refinement." ICML. — LLM self-evaluation attacks surface features.
6. Song et al. (2025). GVU Framework. ICLR. — Verification quality bounds self-improvement.
7. Lightman et al. (2023). "Let's Verify Step by Step." — Process reward models for step-level evaluation.
8. Singh et al. (2025). "The Leaderboard Illusion." NeurIPS. — Overfitting detection in agent evaluation.
9. Sheppert (2026). GT-Score composite. — Standardized agent quality metric.
10. Bailey et al. (2015). PBO: Probability of Backtest Overfitting. — Combinatorial cross-validation for configuration selection.
11. Gibson (1979). The Ecological Approach to Visual Perception. — Affordance theory.
12. Karpathy. Autoresearch pattern. — Separation of generator from evaluator.

---

## `/Users/will/dev/nunchi/roko/roko/docs/EXECUTIVE-SUMMARY.md`

**Inline URL (line 388):**
- `[github.com/nunchi/roko](https://github.com/nunchi/roko)`

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/01-naming-and-glossary.md`

**See Also section (line 695):** Internal cross-references only (no external citations).

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/11-dual-process-and-active-inference.md`

**Section 5 "Classical Cognitive Architecture References" (line 197) and "Academic Foundations" (line 241):**
- Kahneman 2011, *Thinking, Fast and Slow* — Dual-process theory.
- Sun 2002, *Duality of the Mind*, Erlbaum — CLARION dual-level cognitive architecture.
- Friston 2010, *Nature Reviews Neuroscience* 11(2) — Free Energy Principle.
- Chen et al. 2023 (arXiv:2305.05176) — FrugalGPT: cascade routing for cost-efficient LLM use.
- Sumers et al. 2023 (arXiv:2309.02427) — CoALA: cognitive architecture framework for language agents.
- Anderson 1983, *The Architecture of Cognition* — ACT-R: production system cognitive architecture.
- Laird et al. 1987, *Artificial Intelligence* 33(1) — SOAR: universal subgoaling architecture.
- Baars 1988, *A Cognitive Theory of Consciousness* — Global Workspace Theory.
- Yao et al. 2022 (arXiv:2210.03629) — ReAct: interleaved reasoning and acting for language agents.
- Shinn et al. 2023 (arXiv:2303.11366) — Reflexion: self-reflection for autonomous agents.
- Zhao et al. 2023 (arXiv:2308.10144) — ExpeL: learning from autonomous agent experience.
- Wang et al. 2023 (arXiv:2305.16291) — Voyager: open-ended embodied agent with skill library.

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/17-design-principles-and-frontier-summary.md`

**Inline citations (lines 383, 465–478):**
- Cemri et al. 2025, arXiv:2503.13657 — MAST: 14 failure modes across 150 multi-agent system traces.
- Chen et al. 2023, arXiv:2305.05176 — FrugalGPT: cascade cost optimization, up to 98% reduction.
- Lee et al. 2026, arXiv:2603.28052 — Meta-Harness: harness optimization alone produces outsized gains.

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/23-architectural-analysis-improvements.md`

**Section 11 "Academic References" (line 753):**

*Cognitive Architectures:*
- Sumers, T. et al. (2023). "Cognitive Architectures for Language Agents (CoALA)." arXiv:2309.02427.
- Franklin, S. et al. (2016). "LIDA: A Systems-level Architecture for Cognition, Emotion, and Learning." IEEE Trans. Autonomous Mental Development 6(1).
- Laird, J. E. (2012). "The Soar Cognitive Architecture." MIT Press.
- Anderson, J. R. (2007). "How Can the Human Mind Occur in the Physical Universe?" Oxford University Press.
- Laird, J. E. (2022). "Analysis and Comparison of ACT-R and Soar." arXiv:2201.09305.
- Baars, B. J. (1988). "A Cognitive Theory of Consciousness." Cambridge University Press.

*Active Inference and Free Energy:*
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" Nature Reviews Neuroscience 11(2).
- Parr, T. et al. (2024). "Active Inference: The Free Energy Principle in Mind, Brain, and Behavior." arXiv:2402.14460.
- VERSES AI (2025). "Genius: Renormalizing Generative Models." [verses.ai](https://www.verses.ai/active-inference-research)
- Champion, T. et al. (2022). "pymdp: A Python library for active inference." arXiv:2201.03904.
- Devillers, B. et al. (2024). "An Embodied Agent Inspired by Global Workspace Theory." Frontiers in Computational Neuroscience.

*Dual-Process and Multi-Speed:*
- Fabiano, F. et al. (2025). "SOFAI: A multi-component cognitive architecture for intelligent systems." npj Artificial Intelligence.
- Kahneman, D. (2011). "Thinking, Fast and Slow." Farrar, Straus and Giroux.
- Sun, R. (2002). "Duality of the Mind." Lawrence Erlbaum Associates. (CLARION architecture)

*Category Theory and Software Composition:*
- Milewski, B. (2014). "Category Theory for Programmers." [bartoszmilewski.com](https://bartoszmilewski.com/2014/10/28/category-theory-for-programmers-the-preface/)
- Seemann, M. (2017). "From Design Patterns to Category Theory." [blog.ploeh.dk](https://blog.ploeh.dk/2017/10/04/from-design-patterns-to-category-theory/)
- Clarke, B. et al. (2020). "Profunctor Optics: A Categorical Update." arXiv:2001.07488.
- Gonzalez, G. (2012). "The Functor Design Pattern." [haskellforall.com](https://haskellforall.com/2012/09/the-functor-design-pattern.html)

*Agent Systems and Design Patterns:*
- Koopmans, A. et al. (2024). "Agent Design Pattern Catalogue." arXiv:2405.10467.
- Google Cloud (2025). "Eight Multi-Agent Design Patterns." InfoQ.
- Phan-Ba, R. et al. (2025). "Agent Data Protocol (ADP)." arXiv:2510.24702.

*Cybernetics and Self-Organization:*
- Ashby, W. R. (1956). "An Introduction to Cybernetics." Chapman & Hall.
- Conant, R. C. & Ashby, W. R. (1970). "Every Good Regulator of a System Must Be a Model of That System." Int. J. Systems Science 1(2).
- Beer, S. (1972). "Brain of the Firm." Allen Lane.
- Kauffman, S. (1993). "The Origins of Order." Oxford University Press.

*Hyperdimensional Computing:*
- Kanerva, P. (2009). "Hyperdimensional Computing." Cognitive Computation 1(2).
- Kleyko, D. et al. (2022). "A Survey on Hyperdimensional Computing." Artificial Intelligence Review 56.
- Frady, E. P. et al. (2018). "Neural computation with HDC vectors."

*Global Workspace Theory:*
- Dehaene, S. et al. (2025). "GW-Dreamer: Multimodal Global Workspace + Dreamer." arXiv:2502.21142.
- VanRullen, R. & Bhatt, A. (2025). "Functional Advantages of the Selection-Broadcast Cycle." arXiv:2505.13969.

*Memory and Sleep:*
- McClelland, J. et al. (1995). "Complementary Learning Systems." Psychological Review 102(3).
- Mattar, M. & Daw, N. (2018). "Prioritized Memory Access." Nature Neuroscience 21.
- Walker, M. & van der Helm, E. (2009). "Sleep and Emotional Memory." Annual Review of Clinical Psychology 5.
- Lacaux, C. et al. (2021). "Hypnagogia and Creative Insights." Science Advances 7(50).

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/27-temporal-knowledge-topology.md`

**Inline citations (lines 23–25, 306):**
- Rasmussen et al. 2025, arXiv:2501.13956 — Zep/Graphiti: Temporal KG Architecture for Agent Memory
- arXiv:2509.15464 (2025) — Temporal Reasoning with LLMs over Evolving Knowledge Graphs
- arXiv:2401.06072 (2024) — Chain of History: TKG Completion with LLMs

---

## `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/32-comprehensive-test-strategy.md`

**Section 13 "Academic references" (line 1508):**

*Testing self-improving systems:*
- Fang, Peng, Zhang et al. (2025). "A Comprehensive Survey of Self-Evolving AI Agents." arXiv:2508.07407.
- Gao, Geng, Hua et al. (2025). "A Survey of Self-Evolving Agents: On Path to Artificial Super Intelligence." GitHub: EvoAgentX/Awesome-Self-Evolving-Agents.

*Property-based and oracle-free testing:*
- Cho, Ruberto, Terragni (2025). "Metamorphic Testing of Large Language Models for NLP." arXiv:2511.02108. ICSME 2025.
- Cho, Terragni (2025). "LLMORPH: Automated Metamorphic Testing of LLMs." arXiv:2603.23611. ASE 2025 Demo.
- ACM TOSEM (2025). "Test Oracle Automation in the Era of LLMs." DOI:10.1145/3715107.
- arXiv:2601.05542 (2025). "Understanding LLM-Driven Test Oracle Generation."

*Adversarial testing and red-teaming:*
- AIRTBench (2025). "Measuring Autonomous AI Red Teaming." arXiv:2506.14682.
- RedTWIZ (Amazon Science, 2025). "Diverse LLM Red Teaming via Adaptive Attack Planning."
- MITRE ATLAS v5.1.0 (November 2025). 16 tactics, 84 techniques, 14 agentic AI attack patterns. atlas.mitre.org.
- CMU SEI (2025). "What Can Generative AI Red-Teaming Learn from Cyber Red-Teaming?"

*Regression and catastrophic forgetting:*
- Luo et al. (2024). "Revisiting Catastrophic Forgetting in LLM Tuning." ACL EMNLP Findings. aclanthology.org/2024.findings-emnlp.249.
- Wang et al. (2025). "Mechanistic Analysis of Catastrophic Forgetting in LLMs During Continual Fine-tuning." arXiv:2601.18699.
- ACM CSUR (2025). "Continual Learning of Large Language Models: A Comprehensive Survey."

*Agent benchmarking:*
- Tau-bench (2024). "A Benchmark for Tool-Agent-User Interaction." arXiv:2406.12045.
- TheAgentCompany (2024). "Benchmarking LLM Agents on Consequential Real World Tasks." arXiv:2412.14161.
- MCP-Bench (2025). "Benchmarking Tool-Using LLM Agents." arXiv:2508.20453.
- AgentBench (2024). "Evaluating LLMs as Agents." arXiv:2308.03688.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/INDEX.md`

**Citations referenced table (line 149):**
- Grassé, P.-P. | 1959 | La reconstruction du nid (*Insectes Sociaux*) | Used in 00, 02, 07, 12
- Parunak, H. V. D. | 2002 | Digital pheromones (*AAMAS*) | Used in 00, 07, 12
- Dorigo, M. & Gambardella, L. M. | 1997 | Ant colony system (*IEEE Trans. EC*) | Used in 00, 12
- Woolley, A. W. et al. | 2010 | Collective intelligence factor (*Science* 330) | Used in 00, 06, 12
- Yerkes, R. M. & Dodson, J. D. | 1908 | Stimulus-habit formation (*JCNP*) | Used in 00, 06, 11
- Odling-Smee, F. J. et al. | 2003 | *Niche Construction* (Princeton) | Used in 00, 12
- Tomasello, M. | 2014 | *A Natural History of Human Thinking* (Harvard) | Used in 00, 12
- Topcuoglu, H. et al. | 2002 | HEFT scheduling (*IEEE TPDS*) | Used in 02, 13
- Damasio, A. | 1994 | *Descartes' Error* (Putnam) | Used in 06
- Sumers, T. R. et al. | 2023 | CoALA cognitive architectures (*arXiv:2309.02427*) | Used in 03
- Charnov, E. L. | 1976 | Marginal value theorem (*TPB*) | Used in 12
- Derrida, J. | 1993 | *Specters of Marx* (Routledge) | Used in 12
- Beer, S. | 1972 | *Brain of the Firm* (Allen Lane) | Used in 11
- Conant, R. C. & Ashby, W. R. | 1970 | Good regulator theorem (*IJSS*) | Used in 11
- Nygard, M. T. | 2007 | *Release It!* (Pragmatic Bookshelf) | Used in 04, 11
- Fowler, M. | 2005 | Event Sourcing | Used in 03, 09, 10
- Mohan, C. et al. | 1992 | ARIES recovery (*ACM TODS*) | Used in 09
- Gray, J. & Reuter, A. | 1992 | *Transaction Processing* (Morgan Kaufmann) | Used in 08
- Vickrey, W. | 1961 | Sealed-bid auctions (*J. Finance*) | Used in 13
- Ousterhout, J. | 2013 | Sparrow scheduling (*SOSP*) | Used in 13
- Hu, S. et al. | 2025 | ADAS (*ICLR*) | Used in 13
- Lee, J. et al. | 2026 | FrugalGPT (*arXiv:2603.28052*) | Used in 13
- Wooldridge, M. | 2009 | *Introduction to MultiAgent Systems* (Wiley) | Used in 05
- Kahn, A. B. | 1962 | Topological sorting | Used in 02

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/00-layer-overview.md`

**References section (line 296):**
- Grassé, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp. *Insectes Sociaux*, 6(1), 41–80.
- Parunak, H. V. D. (2002). Digital pheromones for coordination of unmanned vehicles. *AAMAS 2002*.
- Dorigo, M. & Gambardella, L. M. (1997). Ant colony system: A cooperative learning approach to the traveling salesman problem. *IEEE Trans. Evolutionary Computation*, 1(1), 53–66.
- Woolley, A. W. et al. (2010). Evidence for a collective intelligence factor in the performance of human groups. *Science*, 330(6004), 686–688.
- Tomasello, M. (2014). *A Natural History of Human Thinking*. Harvard University Press.
- Odling-Smee, F. J., Laland, K. N. & Feldman, M. W. (2003). *Niche Construction: The Neglected Process in Evolution*. Princeton University Press.
- Yerkes, R. M. & Dodson, J. D. (1908). The relation of strength of stimulus to rapidity of habit-formation. *Journal of Comparative Neurology and Psychology*, 18(5), 459–482.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/01-plan-discovery.md`

**References section (line 255):**
- The plan discovery mechanism draws on the document hierarchy concept from the original Mori orchestrator's PRD-to-execution pipeline (`bardo-backup/prd/25-mori/mori-document-pipeline.md`), which defined a PRD → Plan → Task → Brief → Prompt hierarchy.
- The YAML frontmatter convention follows Hugo-style front matter widely used in static site generators and documentation systems.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/02-unified-task-dag.md`

**References section (line 668):**
- The wave scheduling approach draws on topological sort algorithms (Kahn 1962) and list scheduling heuristics from the task scheduling literature.
- Topcuoglu, H., Hariri, S. & Wu, M.-Y. (2002). Performance-effective and low-complexity task scheduling for heterogeneous computing. *IEEE Trans. Parallel and Distributed Systems*, 13(3), 260–274. (HEFT algorithm)
- Grassé, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles. *Insectes Sociaux*, 6(1), 41–80. (Stigmergic coordination)
- Hammer, M. A. et al. (2014). Adapton: Composable, demand-driven incremental computation. *PLDI 2014*.
- Mokhov, A., Mitchell, N. & Peyton Jones, S. (2018). Build systems à la carte. *ICFP 2018*.
- Salsa. Incremental computation for rust-analyzer. *github.com/salsa-rs/salsa*
- Karypis, G. & Kumar, V. (1998). A fast and high quality multilevel scheme for partitioning irregular graphs. *SIAM J. Sci. Comput.*, 20(1), 359–392. (METIS graph partitioning.)
- Dean, J. & Ghemawat, S. (2008). MapReduce: Simplified data processing on large clusters. *Comm. ACM*, 51(1), 107–113. (Speculative execution.)
- Rocklin, M. (2015). Dask: Parallel computation with blocked algorithms and task scheduling. *SciPy 2015*.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/03-parallel-executor.md`

**References section (line 512):**
- The pure state machine approach draws on the Event Sourcing pattern (Fowler 2005).
- The executor's tick-based loop is similar to the CoALA (Cognitive Architectures for Language Agents) 9-step cognitive cycle (Sumers et al. 2023).
- Sha, L., Rajkumar, R. & Lehoczky, J. P. (1990). Priority inheritance protocols: An approach to real-time synchronization. *IEEE Trans. Computers*, 39(9), 1175–1185.
- van der Aalst, W. M. P. (1997). Verification of workflow nets. *Application and Theory of Petri Nets 1997*. LNCS 1248.
- van der Aalst, W. M. P. (1998). The application of Petri nets to workflow management. *J. Circuits, Systems and Computers*, 8(1), 21–66.
- Blumofe, R. D. & Leiserson, C. E. (1999). Scheduling multithreaded computations by work stealing. *JACM*, 46(5), 720–748.
- Chase, D. & Lev, Y. (2005). Dynamic circular work-stealing deque. *SPAA 2005*.
- Wei, C. et al. (2025). Agent.xpu: Scheduling concurrent agentic workloads on heterogeneous SoCs. *arXiv:2506.24045*.
- Patel, S. et al. (2024). BudgetMLAgent: Multi-agent cascade for cost-efficient LLM task execution. *AIMLSystems 2024*. (94.2% cost reduction via three-tier model cascade.)

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/04-plan-phases.md`

**References section (line 290):**
- van der Aalst, W. M. P. (1998). The application of Petri nets to workflow management. *Journal of Circuits, Systems and Computers*, 8(1), 21–66.
- Nygard, M. T. (2007). *Release It!*. Pragmatic Bookshelf. (Circuit breaker pattern)
- Phase transitions correspond to the "state machine replication" pattern used in consensus protocols.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/05-executor-actions.md`

**References section (line 299):**
- The action/event pattern is a variant of the Command pattern (Gamma et al. 1994, *Design Patterns*).
- Wooldridge, M. (2009). *An Introduction to MultiAgent Systems*. Wiley.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/06-runtime-harness.md`

**References section (line 363):**
- Woolley, A. W. et al. (2010). Evidence for a collective intelligence factor. *Science*, 330(6004), 686–688.
- Yerkes, R. M. & Dodson, J. D. (1908). The relation of strength of stimulus to rapidity of habit-formation. *Journal of Comparative Neurology and Psychology*, 18(5), 459–482.
- Sumers, T. R. et al. (2023). Cognitive architectures for language agents. *arXiv:2309.02427*.
- Damasio, A. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*. Putnam.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/07-worktree-isolation.md`

**References section (line 326):**
- Git worktrees: `git-worktree(1)` — official git documentation. Worktrees introduced in git 2.5 (2015).
- Grassé, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp. *Insectes Sociaux*, 6(1), 41–80.
- Parunak, H. V. D. (2002). Digital pheromones for coordination of unmanned vehicles. *AAMAS 2002*.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/08-merge-queue.md`

**References section (line 282):**
- GitHub's Merge Queue, GitLab's Merge Train, and Bors-NG — CI/CD merge serialization patterns.
- Gray, J. & Reuter, A. (1992). *Transaction Processing: Concepts and Techniques*. Morgan Kaufmann.
- Retry with backoff follows the exponential backoff pattern from distributed systems.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/09-snapshot-recovery.md`

**References section (line 645):**
- Mohan, C. et al. (1992). ARIES: A transaction recovery method supporting fine-granularity locking and partial rollbacks using write-ahead logging. *ACM TODS*, 17(1), 94–162.
- Fowler, M. (2005). Event Sourcing. *martinfowler.com/eaaDev/EventSourcing.html*.
- POSIX guarantees that `rename(2)` is atomic within a single filesystem.
- Shapiro, M. et al. (2011). Conflict-free replicated data types. *SSS 2011*.
- Kleppmann, M. & Beresford, A. R. (2017). A conflict-free replicated JSON datatype. *IEEE TPDS*, 28(10), 2733–2746.
- Kulkarni, S. S. et al. (2014). Logical physical clocks and consistent snapshots in globally distributed databases. *OPODIS 2014*. (Hybrid Logical Clocks.)
- Hinto, P. et al. (2024). Loro: Reimagining state synchronization for local-first software. *loro.dev*.
- Percival, C. (2003). Naive differences of executable code. *bsdiff*.
- O'Connor, J. et al. (2020). BLAKE3: One function, fast everywhere. *blake3.io*.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/10-event-log.md`

**References section (line 338):**
- O'Connor, J. et al. (2020). BLAKE3: One function, fast everywhere. *blake3.io*.
- Nakamoto, S. (2008). Bitcoin: A Peer-to-Peer Electronic Cash System — hash-chaining for tamper detection.
- Fowler, M. (2005). Event Sourcing.
- Causal replay for AI systems: the Forensic AI concept from `refactoring-prd/09-innovations.md`.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/11-conductor-integration.md`

**References section (line 256):**
- Yerkes, R. M. & Dodson, J. D. (1908). The relation of strength of stimulus to rapidity of habit-formation. *Journal of Comparative Neurology and Psychology*, 18(5), 459–482.
- Nygard, M. T. (2007). *Release It! Design and Deploy Production-Ready Software*. Pragmatic Bookshelf.
- Beer, S. (1972). *Brain of the Firm: The Managerial Cybernetics of Organization*. Allen Lane. (Viable System Model)
- Conant, R. C. & Ashby, W. R. (1970). Every good regulator of a system must be a model of that system. *International Journal of Systems Science*, 1(2), 89–97.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/12-stigmergy-niche.md`

**References section (line 281):**
- Grassé, P.-P. (1959). La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp. *Insectes Sociaux*, 6(1), 41–80.
- Parunak, H. V. D. (2002). Digital pheromones for coordination of unmanned vehicles. *AAMAS 2002*.
- Dorigo, M. & Gambardella, L. M. (1997). Ant colony system: A cooperative learning approach to the traveling salesman problem. *IEEE Trans. Evolutionary Computation*, 1(1), 53–66.
- Odling-Smee, F. J., Laland, K. N. & Feldman, M. W. (2003). *Niche Construction: The Neglected Process in Evolution*. Princeton University Press.
- Woolley, A. W. et al. (2010). Evidence for a collective intelligence factor in the performance of human groups. *Science*, 330(6004), 686–688.
- Charnov, E. L. (1976). Optimal foraging, the marginal value theorem. *Theoretical Population Biology*, 9(2), 129–136.
- Derrida, J. (1993). *Specters of Marx: The State of the Debt, the Work of Mourning and the New International*. Routledge.
- Tomasello, M. (2014). *A Natural History of Human Thinking*. Harvard University Press.

---

## `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration/13-cross-domain-orchestration.md`

**References section (line 802):**
- Topcuoglu, H., Hariri, S. & Wu, M.-Y. (2002). Performance-effective and low-complexity task scheduling for heterogeneous computing. *IEEE Trans. Parallel and Distributed Systems*, 13(3), 260–274. (HEFT algorithm)
- Ousterhout, J. (2013). Sparrow: Distributed, low latency scheduling. *SOSP 2013*.
- Vickrey, W. (1961). Counterspeculation, auctions, and competitive sealed tenders. *Journal of Finance*, 16(1), 8–37.
- Hu, S. et al. (2025). Automated design of agentic systems. *ICLR 2025*. (ADAS)
- Lee, J. et al. (2026). FrugalGPT: How to use large language models while reducing cost and improving performance. *arXiv:2603.28052*.
- Garcia-Molina, H. & Salem, K. (1987). Sagas. *ACM SIGMOD 1987*.
- Gerevini, A. et al. (2004). Planning through stochastic local search and temporal action graphs in LPG. *JAIR*, 20, 239–290.
- Sacerdoti, E. D. (1974). Planning in a hierarchy of abstraction spaces. *Artificial Intelligence*, 5(2), 115–135. (ABSTRIPS)
- Erol, K., Hendler, J. & Nau, D. S. (1994). HTN planning: Complexity and expressivity. *AAAI 1994*.
- Fox, M. et al. (2006). Plan stability: Replanning versus plan repair. *ICAPS 2006*.
- desJardins, M. E. et al. (1999). A survey of research in distributed, continual planning. *AI Magazine*, 20(4), 13–22.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/00-agent-trait.md`

**Citations section (line 706):**
1. Sumers, T. R. et al. (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427.
2. Hewitt, C., Bishop, P., & Steiger, R. (1973). "A Universal Modular ACTOR Formalism for Artificial Intelligence." IJCAI.
3. Wang, J. et al. (2024). "Mixture-of-Agents Enhances Large Language Model Capabilities." arXiv:2406.04692, ICLR 2025.
4. Anthropic Transformer Circuits Team (2025). "Emergent Introspective Awareness in Large Language Models."
5. arXiv:2509.19783 (2025). "Agentic Metacognition: Self-Aware Agent for Failure Prediction and Human Handoff."
6. arXiv:2410.15048 (2024). "MorphAgent: Self-Evolving Profiles and Decentralized Collaboration."
7. arXiv:2601.04748 (2025). "When Single-Agent with Skills Replace Multi-Agent Systems." — 53.7% token reduction.
8. Murray, T. "Analysing Object-Capability Security." Oxford.
9. Tenuo (2025). tenuo.dev — Cryptographic capability warrants for AI agents.
10. Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." NeurIPS 2023.

**Inline citations:**
- arXiv:2309.02427 (line 64, 287) — CoALA 9-step loop.
- arXiv:2601.04748 (line 303) — 53.7% token reduction, latency by 50%.
- arXiv:2603.29919 (line 310) — 86% pass rate across 600 skills via delta-debugging.
- Wang et al. (2024, arXiv:2406.04692, ICLR 2025) — MoA layered composition (line 406).
- arXiv:2509.19783, 2025 (line 477) — Agentic metacognition.
- arXiv:2410.15048, 2024 (line 656) — MorphAgent dynamic role switching.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/07-tool-loop.md`

**Citations section (line 756):**
1. Yao, S. et al. (2023). "ReAct: Synergizing Reasoning and Acting in Language Models." ICLR 2023. arXiv:2210.03629.
2. Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." NeurIPS 2023. arXiv:2303.11366.
3. Zhou, A. et al. (2024). "Language Agent Tree Search Unifies Reasoning, Acting, and Planning." ICML 2024. arXiv:2310.04406.
4. Yao, S. et al. (2023). "Tree of Thoughts: Deliberate Problem Solving with Large Language Models." NeurIPS 2023. arXiv:2305.10601.
5. arXiv:2511.14650 (2025). "AutoTool: Efficient Tool Selection for LLM Agents." AAAI 2026.
6. arXiv:2603.18897 (2025). Microsoft Research. "PASTE: Pattern-Aware Speculative Tool Execution." — 48.5% latency reduction.
7. ToolCacheAgent (2025). ICLR 2026 submission. — 1.69× speedup via caching.
8. arXiv:2506.14852 (2025). "Agentic Plan Caching." — 50.31% cost reduction.
9. Red Hat (2025). "Tool RAG: Next Breakthrough in Scalable AI Agents." — 99.6% token reduction.
10. Patil, S. et al. (2025). "BFCL: Berkeley Function Calling Leaderboard." ICML 2025.
11. arXiv:2604.06185 (2025). "WildToolBench: Benchmarking LLM Tool-Use in the Wild." ICLR 2026.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/08-harness-engineering.md`

**Citations section (line 213):**
1. Lee, S. Y. et al. (2026). "Meta-Harness: Harness Engineering for LLM Agents." arXiv:2603.28052.
2. Jimenez, C. E. et al. (2024). "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?"
3. ref [46] in Meta-Harness — SWE-bench mobile, source of the "6× gap" number.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/09-format-translation.md`

**Citations section (line 295):**
1. Lee, S. Y. et al. (2026). "Meta-Harness: Harness Engineering for LLM Agents." arXiv:2603.28052.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/10-temperament-profiling.md`

**Citations section (line 242):**
2. Lee, S. Y. et al. (2026). "Meta-Harness: Harness Engineering for LLM Agents." arXiv:2603.28052.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/11-dual-process-routing.md`

**Citations section (line 536):**
1. Kahneman, D. (2011). "Thinking, Fast and Slow."
2. De Neys, W. & Pennycook, G. (2019). "Logic, Fast and Slow: Advances in Dual-Process Theorizing." *Current Directions in Psychological Science*.
3. De Neys, W. (Ed.) (2018). "Dual Process Theory 2.0." Routledge.
4. Evans, J. (2019). "Type 3" metacognitive process concept.
5. Li, L. et al. (2010). "A contextual-bandit approach to personalized news article recommendation." WWW 2010. — LinUCB algorithm.
6. Chen, L. et al. (2023). "FrugalGPT." — Cascade routing.
7. Friston, K. (2010). "The free-energy principle: a unified brain theory?" — Active inference basis.
8. Chen, Z. et al. (2025). "Router-R1: Teaching LLMs Multi-Round Routing via RL." NeurIPS 2025. arXiv:2506.09033.
9. Qian, C. et al. (2025). "xRouter: Cost-Aware LLMs Orchestration via RL." Salesforce. arXiv:2510.08439.
10. Ong, I. et al. (2025). "RouteLLM: Learning to Route LLMs with Preference Data." ICLR 2025. arXiv:2406.18665.
11. Dekoninck, J. et al. (2025). "A Unified Approach to Routing and Cascading." ICLR 2025. arXiv:2410.10347.
12. arXiv:2505.12601 (2025). "Rethinking Predictive Modeling for LLM Routing: When Simple kNN Beats Complex Learned Routers."
13. Song, J. et al. (2025). "IRT-Router." ACL 2025. arXiv:2506.01048.
14. Ding, Y. et al. (2025). "BEST-Route." ICML 2025. arXiv:2506.22716.
15. Zhou, Y. et al. (2022). "Mixture-of-Experts with Expert Choice Routing." arXiv:2202.09368.
16. arXiv:2508.21141 (2025). "PILOT: Preference-Prior Informed LinUCB."
17. arXiv:2509.25426 (2025). "RADAR." — IRT + multi-objective optimization.
18. Varshney, T. & Surla, A. (2026). "LLM Router: Prefill is All You Need." arXiv:2603.20895.

**Inline citations:**
- Zhou et al. (2022, arXiv:2202.09368) — Expert choice routing (line 315).
- arXiv:2508.21141, 2025 — PILOT (line 364).
- arXiv:2505.12601, 2025 — kNN routing (line 421).
- Dekoninck et al. (2025, arXiv:2410.10347, ICLR 2025) — unified routing/cascading (line 474).
- Chen et al. (2025, arXiv:2506.09033, NeurIPS 2025) — Router-R1 (line 492).
- Qian et al. (2025, arXiv:2510.08439, Salesforce) — xRouter (line 499).
- Song et al. (2025, arXiv:2506.01048, ACL 2025) — IRT-Router (line 506).
- Ding et al. (2025, arXiv:2506.22716, ICML 2025) — BEST-Route (line 513).
- Varshney & Surla (2026, arXiv:2603.20895) — near-zero overhead routing (line 521).
- RADAR (arXiv:2509.25426, 2025) — IRT + multi-objective (line 528).

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/12-extensibility.md`

**Citations section (line 536):**
3. Sakana AI et al. (2025). "Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents." arXiv:2505.22954.
4. Wang, G. et al. (2023). "Voyager: An Open-Ended Embodied Agent with LLMs." arXiv:2305.16291.
5. Liu, T. & van der Schaar, M. (2025). "Truly Self-Improving Agents Require Intrinsic Metacognitive Learning." ICML 2025. arXiv:2506.05109.

---

## `/Users/will/dev/nunchi/roko/roko/docs/02-agents/INDEX.md`

**Citations (lines 92–150):**
- arXiv:2309.02427 — CoALA 9-step loop.
- arXiv:2603.28052 — Meta-Harness.
- arXiv:2406.04692, ICLR 2025 — MoA layered composition.
- arXiv:2509.19783 (2025) — Agentic Metacognition.
- ICLR 2023. arXiv:2210.03629 — ReAct pattern.
- NeurIPS 2023. arXiv:2303.11366 — Reflexion. 91% HumanEval.
- arXiv:2310.04406 — LATS. 92.7% HumanEval.
- Sakana AI (2025). "Darwin Gödel Machine." arXiv:2505.22954.
- Wang, G. et al. (2023). "Voyager." arXiv:2305.16291.
- Chen, Z. et al. (2025). "Router-R1." NeurIPS 2025. arXiv:2506.09033.
- arXiv:2410.10347 — +14% on SWE-Bench.
- arXiv:2604.06185 (2025). "WildToolBench." ICLR 2026.
- arXiv:2601.04748 (2025). "When Single-Agent with Skills Replace Multi-Agent Systems."
- arXiv:2506.05109 — Intrinsic metacognition.
- arXiv:2410.15048 (2024). "MorphAgent."

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/00-composer-trait.md`

**Inline citation (line 206):**
- Liu et al. 2023 [arXiv:2307.03172] — U-shape attention optimization.

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/01-prompt-composer.md`

**Inline citations (lines 101, 334):**
- Liu et al. (2023) [arXiv:2307.03172] — "Lost in the Middle." U-shaped attention.

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/02-system-prompt-builder-7-layer.md`

**Inline citations:**
- Anthropic's context engineering guidance [2025] and systematic prompt surveys [arXiv:2402.07927] — directive-last principle (line 395).
- Li et al., arXiv:2410.12388 — NAACL 2025 prompt compression survey (line 496).
- Chain of Draft [Zoom Research, arXiv:2502.18600] — 5-word intermediate reasoning steps (lines 540, 562).
- LLMLingua-2 [Pan et al., ACL Findings 2024, arXiv:2403.12968] (line 568).
- RECOMP [Xu et al., ICLR 2024, arXiv:2310.04408] (line 570).
- Promptomatix [arXiv:2507.14241, July 2025] (line 572).

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/03-role-templates.md`

**Inline citation (line 459):**
- Meta-Harness [Lee et al. 2026, arXiv:2603.28052] — 6× performance gap from scaffold changes.

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/05-token-budget-management.md`

**Inline citations:**
- CLEAR Framework [2025] — five evaluation dimensions (line 243).
- TALE framework [Hu et al., ACL Findings 2025, arXiv:2412.18547] — 68.9% token reduction (line 361).
- SelfBudgeter approach [Li et al., arXiv:2505.11274, May 2025] (line 450).

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/06-lost-in-the-middle-u-shape.md`

**Inline citations and References section (lines 67–509):**
- Liu et al., TACL 2024 [arXiv:2307.03172] — Canonical source: "Lost in the Middle."
- arXiv:2504.04713 — Sequential-NIAH, Claude 3.5 at 87% on sequential needle extraction (line 339).
- arXiv:2603.10123 (2025) — U-shaped bias is algebraic property of causal decoder architectures (lines 403, 423, 483).
- arXiv:2601.04098, January 2025 — positional bias operates at per-layer level (line 413).
- He et al. [ACL Findings 2024, arXiv:2406.16008] — positional bias calibration without retraining (lines 431, 487).
- LongLLMLingua [Jiang et al., ACL 2024, arXiv:2310.06839] — automatic document reordering (lines 441, 495).
- **Full references (lines 481–509):**
  - Liu et al. (2023), "Lost in the Middle" [TACL 2024, arXiv:2307.03172].
  - "Lost in the Middle at Birth" [arXiv:2603.10123, 2025].
  - "Lost in the Middle: An Emergent Property from Information Retrieval Demands" [arXiv:2510.10276, October 2025].
  - "Found in the Middle" [He et al., ACL Findings 2024, arXiv:2406.16008].
  - "Layer-wise Positional Bias in Short-Context Language Modeling" [arXiv:2601.04098, January 2025].
  - LLMLingua / LongLLMLingua [Jiang et al., EMNLP 2023; ACL 2024, arXiv:2310.06839].
  - Sequential-NIAH [arXiv:2504.04713, 2025] — Claude 3.5 at 87% accuracy.
  - "Serial Position Effects of LLMs" [arXiv:2406.15981, 2024].

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/08-5-stage-assembly-pipeline.md`

**Inline citation (line 336):**
- Liu et al. (2023), "Lost in the Middle" [TACL 2024, arXiv:2307.03172].

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/09-predictive-foraging-mvt.md`

**Inline citations and References section (lines 414–500):**
- Lacosse et al. (2026), arXiv:2603.01822 — LLMs exhibit same foraging patterns as humans (lines 414, 494).
- arXiv:2511.12759 (November 2025) — Foraging in modern semantic spaces (lines 420, 496).
- "Sufficient Context" framework [Harel-Canada et al., ICLR 2025, arXiv:2411.06037] (lines 426, 498).
- Long-context LLMs with RAG [ICLR 2025, arXiv:2410.05983] (lines 456, 500).

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/10-vcg-attention-auction.md`

**Inline citations:**
- MIT CEEPR Working Paper 2023-18 — learning in repeated auctions (line 232).
- arXiv:2402.07363 (2024) — bidding in first-price auctions (lines 232, 523).
- arXiv:2402.19420 (2024) — MARL auction literature (lines 284, 521).
- Regularized Proportional Fairness (RPF) [Zhu et al., ICLR 2025, arXiv:2501.01111] (lines 430, 517).
- "Mechanism Design for Large Language Models" [Duetting et al., WWW 2024 Best Paper, arXiv:2310.10826] (lines 493, 515).

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/11-distributed-context-engineering.md`

**References section (lines 217–225):**
- Lee et al. (2026), "Meta-Harness" [arXiv:2603.28052] — 6× performance gap.
- CLEAR Framework [2025] — five-dimensional evaluation.

---

## `/Users/will/dev/nunchi/roko/roko/docs/03-composition/13-current-status-and-gaps.md`

**Citation index table (lines 188–243) — 51 citations:**
| # | Citation |
|---|---|
| 1 | Lewis et al. (2020), RAG |
| 2 | Gao et al. (2023), Modular RAG Survey |
| 3 | Wei et al. (2022), Chain-of-Thought Prompting |
| 4 | Kojima et al. (2022), Zero-Shot CoT |
| 5 | Yao et al. (2022), ReAct |
| 6 | Shinn et al. (2023), Reflexion |
| 7 | Yao et al. (2023), Tree of Thoughts |
| 8 | Wang et al. (2023), Plan-and-Solve |
| 9 | Liu et al. (2023), "Lost in the Middle" [arXiv:2307.03172] |
| 10 | Jiang et al. (2023), LLMLingua [EMNLP] |
| 11 | Li et al. (2023), Selective Context [EMNLP] |
| 12 | Khattab et al. (2023), DSPy |
| 13 | Gao et al. (2022), HyDE |
| 14 | Ma et al. (2023), Rewrite-Retrieve-Read |
| 15 | Zheng et al. (2023), Step-Back Prompting |
| 16 | Anthropic (2024), Prompt Caching |
| 17 | Willard & Louf (2023), Structured Generation |
| 18 | Sumers et al. (2023), CoALA |
| 19 | Asai et al. (2023), Self-RAG |
| 20 | Yan et al. (2024), CRAG |
| 21 | Friston (2006, 2010, 2022), Free Energy Principle |
| 22 | Friston et al. (2015), Active Inference & Epistemic Value |
| 23 | Zaharia et al. (2024), Compound AI Systems [BAIR] |
| 24 | Charnov (1976), Marginal Value Theorem |
| 25 | Pirolli & Card (1999), Information Foraging Theory |
| 26 | Hills et al. (2012), Cognitive Foraging |
| 27 | Vickrey (1961), Second-Price Auctions |
| 28 | Clarke (1971), Multipart Pricing |
| 29 | Groves (1973), Incentives in Teams |
| 30 | Simon (1971), Attention Economics |
| 31 | Karpathy (2025), Context Engineering |
| 32 | Lee et al. (2026), Meta-Harness [arXiv:2603.28052] |
| 33 | Shahul Es et al. (2024), RAGAS [EACL] |
| 34 | Saad-Falcon et al. (2024), ARES [NAACL] |
| 35 | Joren et al. (2025), Sufficient Context [ICLR] |
| 36 | Chroma (2025), Context Rot |
| 37 | Shi et al. (2023), Irrelevant Context [ICML] |
| 38 | Du et al. (2025), Whitespace Degradation [EMNLP] |
| 39 | Mu et al. (2023), Gist Tokens [NeurIPS] |
| 40 | Mehrabian (1996), PAD Model |
| 41 | Plutchik (1980), Emotion Wheel |
| 42 | Damasio (1994), Somatic Marker Hypothesis |
| 43 | Doya (2002), Neuromodulation |
| 44 | Itti & Baldi (2005), Bayesian Surprise [NeurIPS] |
| 45 | Kapoor et al. (2025), AI Agents That Matter [Princeton] |
| 46 | Miller (2024), Clustered Standard Errors [Anthropic] |
| 47 | CLEAR Framework (2025) |
| 48 | Contextual Influence Value (2025), Shanghai Jiao Tong |
| 51 | Dantzig (1957), Greedy Knapsack |

---

## `/Users/will/dev/nunchi/roko/roko/docs/04-verification/INDEX.md`

**Academic References section (line 196):**
- Song et al. (ICLR 2025) — Generation-Verification-Update framework, Variance Inequality.
- Lightman et al. (2023) — PRM800K, "Let's Verify Step by Step."
- AgentPRM (arXiv:2502.10325) — Per-step rewards for agent tool use.
- SAGE (arXiv:2512.17102) — Self-acquired generalist expertise, skill libraries.
- Voyager (Wang et al. 2023) — Skill accumulation in LLM agents.
- Self-Refine (Madaan et al. 2023) — Iterative self-improvement with feedback.
- Reflexion (Shinn et al. 2023) — Verbal reinforcement for agents.
- Guo et al. (2017) — Expected calibration error.
- ACON (arXiv:2510.00615) — Context compaction.
- Agent Behavioral Contracts (arXiv:2602.22302) — Formal behavioral specifications.

---

## `/Users/will/dev/nunchi/roko/roko/docs/04-verification/06-adaptive-thresholds.md`

**Inline citations:**
- Adams & MacKay, "Bayesian Online Changepoint Detection" (arXiv:0710.3742) (line 488).
- arXiv:1101.1438 (2012) — "Computational Cost" (line 861).

---

## `/Users/will/dev/nunchi/roko/roko/docs/04-verification/15-verdicts-as-signals.md`

**Inline citation (line 795):**
- MAML, arXiv:1703.03400, ICML 2017.

---

## `/Users/will/dev/nunchi/roko/roko/docs/05-learning/04-cascade-router.md`

**Inline citation (line 17):**
- RouteLLM, Ong et al. ICLR 2025; FrugalGPT, Chen et al. arXiv:2305.05176; AutoMix, NeurIPS 2024.

---

## `/Users/will/dev/nunchi/roko/roko/docs/06-neuro/04-hdc-vsa-foundations.md`

**Academic Foundations / Core HDC/VSA References section (line 284):**
- Kanerva, P. (2009). "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors." *Cognitive Computation*, 1(2), 139–159.
- Kleyko, D., Rachkovskij, D. A., Osipov, E., & Rahimi, A. (2022). "A Survey on Hyperdimensional Computing: Theory, Architecture, and Applications." *ACM Computing Surveys*, 54(6).
- Thomas, A., Dasgupta, S., & Bhatt, T. (2021). "A Theoretical Perspective on Hyperdimensional Computing." *Journal of Artificial Intelligence Research*, 72, 215–249.
- Plate, T. A. (2003). *Holographic Reduced Representations: Distributed Representation for Cognitive Structures*. CSLI Publications.
- Gayler, R. W. (1998). "Multiplicative-Additive-Permute representations and the binding problem." *Proceedings of the 20th Cognitive Science Conference*.
- Frady, E. P., Kleyko, D., & Sommer, F. T. (2020). "A Theory of Sequence Indexing and Working Memory in Recurrent Neural Networks." *Neural Computation*, 32(12), 2275–2325. (Resonator networks)

**Dimension and Capacity:**
- Johnson, W. B., & Lindenstrauss, J. (1984). "Extensions of Lipschitz mappings into a Hilbert space." *Contemporary Mathematics*, 26, 189–206.
- Yeung, E., Zou, T., & Imani, M. (2024). "Generalized Holographic Reduced Representations." (GHRR)

**Neuroscience Connection:**
- Kanerva, P. (1988). *Sparse Distributed Memory*. MIT Press.
- Neubert, P., Schubert, S., & Protzel, P. (2019). "An Introduction to Hyperdimensional Computing for Robotics." *KI - Künstliche Intelligenz*, 33, 319–330.

**Inline citations:**
- Word2HyperVec (GLSVLSI 2024) (line 1046).
- "Attention as Binding" (arXiv:2512.14709, 2025) (line 1046).
- HPVM-HDC (Kotsifakou et al., arXiv:2410.15179, 2024) (lines 1183, 1240).
- FSL-HDnn (arXiv:2512.11826, 2024) (lines 1187, 1244).
- HyperSense (arXiv:2401.10267, 2024) (line 1189).
- HDC Graph ML (2024), arXiv:2402.17073 (lines 1229, 1243).

---

## `/Users/will/dev/nunchi/roko/roko/docs/06-neuro/08-cross-domain-hdc-transfer.md`

**Inline citations (lines 770–772):**
- Webb et al. (2024); arXiv:2406.13803 — LLMs on near vs. far analogies.
- arXiv:2603.29997 (2025) — hybrid LLM + classical mapping engine approach.

---

## `/Users/will/dev/nunchi/roko/roko/docs/06-neuro/16-current-status-and-gaps.md`

**Academic Foundations section (line 176):**
- McClelland, J. L., et al. (1995). "Complementary learning systems." *Psychological Review*, 102(3).
- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*, 1(2).
- Park, J. S., et al. (2023). "Generative Agents." *UIST 2023*.
- Sumers, T. R., et al. (2023). "Cognitive Architectures for Language Agents." *arXiv:2309.02427*.

**Inline citations (lines 536–538):**
- Shams, Z. et al. (2024). "Neuro-Symbolic AI in 2024: A Systematic Review." arXiv:2501.05435.
- Edge, D. et al. (2024). "From Local to Global: A Graph RAG Approach." arXiv:2404.16130. (GraphRAG)

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/00-conductor-architecture.md`

**References section (line 343):**
- Conant & Ashby (1970) — "Every good regulator of a system must be a model of that system."
- Beer (1972) — Viable System Model, System 3 + System 3*.
- Boyd — OODA loop (Observe-Orient-Decide-Act).

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/01-watcher-ensemble.md`

**References section (line 973):**
- Liu, F. T., Ting, K. M., & Zhou, Z.-H. (2008). "Isolation Forest." ICDM.
- Online Isolation Forest. ICML 2024.
- Page, E. S. (1954). "Continuous Inspection Schemes." Biometrika.
- TraceAegis. arXiv 2510.11203, October 2024.
- Dempster, A. P. (1967). "Upper and Lower Probabilities Induced by a Multivalued Mapping." Annals of Mathematical Statistics.
- Shafer, G. (1976). A Mathematical Theory of Evidence. Princeton University Press.
- Apache Flink CEP documentation: https://nightlies.apache.org/flink/flink-docs-stable/docs/libs/cep/
- Esper EPL documentation: https://www.espertech.com/esper/

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/02-circuit-breaker.md`

**References section (line 712):**
- Nygard (2007) — *Release It!*, circuit breaker pattern.
- Netflix Hystrix — rolling window metrics, health calculation.
- Resilience4j — sliding window, slow-call detection.
- Netflix Principles of Chaos Engineering (2018).
- Patterson et al. (2002) — Recovery-Oriented Computing.
- Candea & Fox (2003) — Micro-reboots.

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/07-ooda-cybernetic-loop.md`

**References section (line 602):**
- Boyd, J. (1995). "The Essence of Winning and Losing" (OODA loop diagram).
- Boyd, J. (1976). "Destruction and Creation" (theoretical underpinning of OODA).
- Beer, S. (1972). *Brain of the Firm* (Viable System Model, algedonic signals).
- Mesarovic, M., Macko, D., and Takahara, Y. (1970). *Theory of Hierarchical, Multilevel Systems* (hierarchical control, parameter cascade).
- Klein, G. (1998). *Sources of Power: How People Make Decisions* (Recognition-Primed Decision model).

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/08-good-regulator-self-model.md`

**References section (line 719):**
- Conant, R.C. & Ashby, W.R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97.
- Francis, B.A. & Wonham, W.M. (1976). "The Internal Model Principle of Control Theory." *Automatica*, 12(5), 457-465.
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138.
- Chen, B. et al. (2022). "Full-Body Visual Self-Modeling of Robot Morphologies." *Science Robotics*, 7(68).
- Kalman, R.E. (1960). "A New Approach to Linear Filtering and Prediction Problems." *Journal of Basic Engineering*, 82(1), 35-45.
- Song, Y. et al. (2025). "The Good, the Bad, and the Greedy: Evaluation of LLMs Should Not Ignore Non-Determinism." *ICLR 2025*.

---

## `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/12-yerkes-dodson-pressure.md`

**Citations section (line 418) and References section (line 905):**
- Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength of stimulus to rapidity of habit-formation." *Journal of Comparative Neurology and Psychology*, 18, 459-482.
- Grassé, P.P. (1959). "La reconstruction du nid..." *Insectes Sociaux*, 6(1), 41-80.
- Research on 770,000+ autonomous agents: emergent cooperation dynamics, Yerkes-Dodson replication in LLM multi-agent systems.
- Csikszentmihalyi, M. (1975/1990). *Flow: The Psychology of Optimal Experience*. Harper & Row.
- Sweller, J. (1988). "Cognitive load during problem solving: Effects on learning." *Cognitive Science*, 12(2), 257-285.
- Hanin, Y.L. (2000). "Individual Zones of Optimal Functioning (IZOF) model." In Y.L. Hanin (Ed.), *Emotions in Sport*. Human Kinetics.
- Thompson, W.R. (1933). "On the likelihood that one unknown probability exceeds another in view of the evidence of two samples." *Biometrika*, 25(3-4), 285-294.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/00-vision-and-mortality-incompatibility.md`

**Academic Foundations (line 124):**
- Mehrabian, A. (1996). "Pleasure-arousal-dominance..." *Current Psychology*, 14(4), 261–292.
- Russell, J.A. & Mehrabian, A. (1977). "Evidence for a three-factor theory of emotions." *Journal of Research in Personality*, 11, 273–294.
- Damasio, A.R. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*. Putnam.
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel sequential checking." Oxford University Press.
- Plutchik, R. (1980). *Emotion: A Psychoevolutionary Synthesis*. Harper & Row.
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129–148.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy? The role of sleep in emotional brain processing." *Psychological Bulletin*, 135(5), 731–748.
- Blaney, P.H. (1986). "Affect and memory: A review." *Psychological Bulletin*, 99(2), 229–246.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/01-pad-vector.md`

**Academic Foundations (line 278):**
- Mehrabian, A. (1996). "Pleasure-arousal-dominance..." *Current Psychology*, 14(4), 261–292.
- Russell, J.A. & Mehrabian, A. (1977). "Evidence for a three-factor theory of emotions." *Journal of Research in Personality*, 11, 273–294.
- Plutchik, R. (1980). *Emotion: A Psychoevolutionary Synthesis*. Harper & Row.
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory: An Analysis of Decision under Risk." *Econometrica*, 47(2), 263–291.
- Zhang, H. et al. "Building Emotional Support Chatbots in the Era of LLMs." *SIGDIAL*.
- Gadanho, S.C. (2003). "Learning Behavior-Selection by Emotions and Cognition in a Multi-Goal Robot Task." *Journal of Machine Learning Research*, 4, 385–412.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/02-alma-three-layer-temporal.md`

**Academic Foundations (line 213):**
- Gebhard, P. (2005). "ALMA — A Layered Model of Affect." *AAMAS*, 29–36.
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261–292.
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Russell, J.A. (1980). "A circumplex model of affect." *Journal of Personality and Social Psychology*, 39(6), 1161–1178.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel sequential checking." Oxford University Press.
- Costa, P.T. & McCrae, R.R. (1992). *NEO PI-R Professional Manual*. Psychological Assessment Resources.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/03-occ-scherer-appraisal.md`

**Academic Foundations (line 224):**
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Scherer, K.R. (2001). "Appraisal considered as a process of multilevel sequential checking." Oxford University Press.
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory..." *Econometrica*, 47(2), 263–291.
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261–292.
- Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS*.
- Bechara, A. et al. (1994). "Insensitivity to future consequences following damage to human prefrontal cortex." *Cognition*, 50, 7–15.
- Bechara, A., Damasio, H., Tranel, D., & Damasio, A.R. (1997). "Deciding advantageously before knowing the advantageous strategy." *Science*, 275(5304), 1293–1295.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/04-six-behavioral-states.md`

**Academic Foundations (line 602):**
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261–292.
- Russell, J.A. & Mehrabian, A. (1977). *Journal of Research in Personality*, 11(3), 273–294.
- Ortony, A., Clore, G.L., & Collins, A. (1988). *The Cognitive Structure of Emotions*. Cambridge University Press.
- Gebhard, P. (2005). "ALMA — A Layered Model of Affect." *AAMAS*, pp. 29–36.
- Chen, L. et al. (2023). "FrugalGPT." arXiv:2305.05176.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/05-behavioral-state-to-tier-routing.md`

**Academic Foundations (line 242):**
- Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.
- Chen, L. et al. (2023). "FrugalGPT." arXiv:2305.05176.
- Li, L., Chu, W., Langford, J., & Schapire, R.E. (2010). "A contextual-bandit approach to personalized news article recommendation." *WWW 2010*, pp. 661–670.
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261–292.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/06-somatic-markers-damasio.md`

**Academic Foundations (line 270):**
- Damasio, A.R. (1994). *Descartes' Error*. Putnam.
- Bechara, A., Damasio, A.R., Damasio, H., & Anderson, S.W. (1994). "Insensitivity to future consequences following damage to human prefrontal cortex." *Cognition*, 50, 7–15.
- Bechara, A., Damasio, H., Tranel, D., & Damasio, A.R. (1997). "Deciding advantageously before knowing the advantageous strategy." *Science*, 275(5304), 1293–1295.
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129–148.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy? The role of sleep in emotional brain processing." *Psychological Bulletin*, 135(5), 731–748.
- Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.

**Inline URL (line 285):**
- [`kiddo`](https://crates.io/crates/kiddo) — k-d tree implementation.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/07-15-percent-contrarian-retrieval.md`

**Academic Foundations (line 527):**
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129–148.
- Blaney, P.H. (1986). "Affect and Memory: A Review." *Psychological Bulletin*, 99(2), 229–246.
- Faul, L. & LaBar, K.S. (2022). "Mood-Congruent Memory Revisited." *Psychological Review*.
- Emotional RAG. (2024). "Emotional RAG: Enhancing Role-Playing Agents through Emotional Retrieval." arXiv:2410.23041.
- Seligman, M.E.P. (1967). "Failure to escape traumatic shock." *Journal of Experimental Psychology*, 74(1), 1–9.
- Nietzsche, F. (1887). *On the Genealogy of Morals*. Second Essay.
- Walker, M.P. & van der Helm, E. (2009). "Overnight therapy?" *Psychological Bulletin*, 135(5), 731–748.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/08-8-dimensional-strategy-space.md`

**Academic Foundations (line 186):**
- Damasio, A.R. (1994). *Descartes' Error*. Putnam.
- Bechara, A., Damasio, H., Tranel, D., & Damasio, A.R. (1997). "Deciding advantageously before knowing the advantageous strategy." *Science*, 275(5304), 1293–1295.
- Kleyko, D. et al. (2022). "Vector Symbolic Architectures as a Computing Framework for Emerging Hardware." *ACM Computing Surveys*, 55(13s), 1–55.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/09-mood-congruent-memory.md`

**Academic Foundations (line 332):**
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129–148.
- Blaney, P.H. (1986). "Affect and Memory: A Review." *Psychological Bulletin*, 99(2), 229–246.
- Faul, L. & LaBar, K.S. (2022). "Mood-Congruent Memory Revisited." *Psychological Review*.
- Emotional RAG. (2024). arXiv:2410.23041.
- Ebbinghaus, H. (1885). *Memory: A Contribution to Experimental Psychology*.
- Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS*.
- McGaugh, J.L. (2004). "The Amygdala Modulates the Consolidation of Memories of Emotionally Arousing Experiences." *Annual Review of Neuroscience*, 27.
- Roediger, H.L. & Karpicke, J.D. (2006). "Test-enhanced learning." *Psychological Science*, 17(3), 249–255.
- McAdams, D.P. (2001). "The Psychology of Life Stories." *Review of General Psychology*, 5(2).
- Abelson, R.P. (1963). "Computer Simulation of 'Hot Cognition'." Wiley.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/11-coding-agent-integration.md`

**Academic Foundations (line 317):**
- Mehrabian, A. (1996). *Current Psychology*, 14(4), 261–292.
- Seligman, M.E.P. (1967). "Failure to escape traumatic shock." *Journal of Experimental Psychology*, 74(1), 1–9.
- Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS*.

---

## `/Users/will/dev/nunchi/roko/roko/docs/09-daimon/12-collective-emotional-contagion.md`

**Academic Foundations (line 263):**
- Hatfield, E., Cacioppo, J.T., & Rapson, R.L. (1993). "Emotional contagion." *Current Directions in Psychological Science*, 2(3), 96–100.
- Bower, G.H. (1981). "Mood and Memory." *American Psychologist*, 36(2), 129–148.
- Woolley, A.W. et al. (2010). "Evidence for a Collective Intelligence Factor in the Performance of Human Groups." *Science*, 330(6004), 686–688.
- Grassé, P.P. (1959). "La reconstruction du nid..." *Insectes Sociaux*, 6(1), 41–80.

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/00-vision-and-dream-as-death-reframe.md`

**Inline citations and table (lines 24–168):**
- Lin et al. (2025, arXiv:2504.11651) — sleep-time compute, 5× reduction in test-time compute.
- Park et al. (2023, UIST, arXiv:2304.03442) — Generative Agents, periodic reflection cycles.
- Sumers et al. (2023), arXiv:2309.02427, CoALA framework — three cognitive speeds.

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/03-rem-imagination.md`

**Inline citations:**
- "Backtracking Counterfactuals," arXiv:2211.00472, under review ICLR 2024 (line 91).
- "Deep Backtracking Counterfactuals for Causally Compliant Explanations" (DeepBC), arXiv:2310.07665, *TMLR*, July 2024 (line 93).
- "Natural Counterfactuals with Necessary Backtracking," arXiv:2402.01607, 2024 (line 95).
- Hafner et al. (2023), "Mastering Diverse Domains with World Models" (DreamerV3), arXiv:2301.04104 (line 1100).
- Lin et al. (2025), "Sleep-time Compute", arXiv:2025.00XXX (line 1104).
- "Transformational Creativity in Science: A Graphical Theory," arXiv:2504.18687 (2025) (lines 1110, 1231).
- Danijar Hafner, Jurgis Pasukonis, Jimmy Ba, and Timothy Lillicrap, "Mastering diverse control tasks through world models," *Nature*, 2025. DOI: 10.1038/s41586-025-08744-2. Preprint: arXiv:2301.04104 (line 1123).
- Vincent Micheli, Eloi Alonso, and François Fleuret, "Efficient World Models with Context-Aware Tokenization," *ICML 2024*. arXiv:2406.19320 (line 1160).

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/04-consolidation-and-staging.md`

**Inline citations:**
- WSCL (Skenderi et al., 2024), "Wake-Sleep Consolidated Learning," arXiv:2401.08623, IEEE 2024 (line 326).
- Park et al. (2023), UIST, arXiv:2304.03442, "Generative Agents" — memory synthesis and reflection cycle architecture (line 346).

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/06-hdc-counterfactual-synthesis.md`

**Inline citations:**
- Lewis et al. (2020), NeurIPS, arXiv:2005.11401, "Retrieval-Augmented Generation" (line 236).
- Kingma & Welling (2013), arXiv:1312.6114, "Auto-Encoding Variational Bayes" (line 887).

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/09-threat-simulation.md`

**Inline citations:**
- "Jailbreaking to Jailbreak" (J2), arXiv:2502.09638 (2025) — 0.975 ASR (lines 405, 441).
- Anthropic Safeguards Research Team, "Constitutional Classifiers: Defending Against Universal Jailbreaks," arXiv:2501.18837, January 2025 (lines 575, 593).
- "Quality-Diversity Red-Teaming," arXiv:2506.07121, June 2025 (line 585).

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/12-sleep-time-compute.md`

**Inline citations and table:**
- Lin et al. (2025, arXiv:2504.13171) — "Sleep-time Compute: Beyond Inference Scaling at Test-time" — 5× reduction (lines 18, 144, 173, 247, 251).
- Sumers et al. (2023, arXiv:2309.02427), CoALA (lines 68, 249, 314).

---

## `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/INDEX.md`

**Citation table (lines 103–121):**
- Lin et al., arXiv:2504.13171, 2025 — Sleep-time compute.
- "Backtracking Counterfactuals," arXiv:2211.00472, ICLR 2024.
- Espírito Santo et al., "Towards a Formal Creativity Theory," arXiv:2405.02148, 2024.
- Mouret & Clune, "MAP-Elites," arXiv:1504.04909, 2015.
- "Quality-Diversity Red-Teaming," arXiv:2506.07121, 2025.
- Anthropic, "Constitutional Classifiers," arXiv:2501.18837, 2025.

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/09-adaptive-risk.md`

**Inline citations:**
- Agent-SafetyBench (arXiv:2412.14470, 2024) — 349 environments, 2000 tests (line 536).
- Ye and Tan's Agent Contracts framework (arXiv:2601.08815, January 2025; COINE 2026) (line 806).

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/10-mev-protection.md`

**Inline citations:**
- Daian et al. ("Flash Boys 2.0," IEEE S&P 2020, arXiv:1904.05234) (lines 22, 218).
- Zust et al. (ETH Zurich, 2021) — 525,004 sandwich attacks, 57,493 ETH extracted (line 22).
- Qin et al. ("Quantifying Blockchain Extractable Value," IEEE S&P 2022) (line 22).

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/11-temporal-logic.md`

**Inline citations:**
- AgentSpec (Wang et al., 2025/2026, ICSE '26, arXiv:2503.18666) (lines 648, 1148).
- AgentGuard (Koohestani et al., 2025, arXiv:2509.23864) (lines 649, 1149).
- Temporal Attack Pattern Detection (arXiv:2601.00848, January 2025) (lines 837, 1146).
- SentinelAgent (arXiv:2505.24201, May 2025) (lines 837, 1147).

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/12-witness-dag.md`

**Inline citations:**
- See https://www.w3.org/TR/prov-dm/ (line 950).
- Sumers et al. (2023, arXiv:2309.02427), "Cognitive Architectures for Language Agents" (line 925).
- Palumbo et al. (2026, arXiv:2602.16708) — PCAS, Datalog-derived policy language for agents (line 929).

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/13-formal-verification.md`

**Inline citations and table:**
- Agent Behavioral Contracts (arXiv:2602.22302, 2026) — Design-by-Contract for AI agents (lines 983, 1004, 1076, 1294).
- AgentSpec (Wang et al., ICSE '26, arXiv:2503.18666) — >90% unsafe action prevention (lines 984, 1293).
- AgentGuard (Koohestani et al., 2025, arXiv:2509.23864) (lines 985, 1294).
- Pro2Guard (arXiv:2508.00500, 2025) (line 986).
- VeriGuard (arXiv:2510.05156, 2025) — Dual-stage offline proof + online monitor (lines 987, 1187, 1295).
- Agent Contracts (arXiv:2601.08815, 2026) (line 988).
- "Formalizing Properties of Agentic AI" (arXiv:2510.14133, October 2025) — 17 host-agent properties (lines 1000, 1048, 1292).
- Agent Behavioral Contracts (arXiv:2602.22302, February 2026) — contracted agents detect 5.2-6.8 more violations per session (line 1004).

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/14-cognitive-kernel-safety.md`

**Inline citation (line 790):**
- Sumers et al. (2023, arXiv:2309.02427), "Cognitive Architectures for Language Agents."

---

## `/Users/will/dev/nunchi/roko/roko/docs/11-safety/15-forensic-ai.md`

**Inline citations (lines 352–353):**
- Sumers et al. (2023, arXiv:2309.02427) — CoALA cognitive architecture.
- Lee et al. (2026, arXiv:2603.28052) — Meta-Harness.

---

## `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/17-accessibility-and-current-status.md`

**Inline URL (line 317):**
- https://no-color.org/ — NO_COLOR environment variable standard.

---

## `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/20-ide-integration-strategy.md`

**References section (line 690):**
1. ACP specification: [agentclientprotocol.com](https://agentclientprotocol.com)
2. ACP GitHub: [github.com/agentclientprotocol/agent-client-protocol](https://github.com/agentclientprotocol/agent-client-protocol)
3. ACP Rust SDK: [crates.io/crates/agent-client-protocol](https://crates.io/crates/agent-client-protocol)
4. VS Code AI extensibility: [code.visualstudio.com/api/extension-guides/ai/ai-extensibility-overview](https://code.visualstudio.com/api/extension-guides/ai/ai-extensibility-overview)
5. VS Code MCP support: [code.visualstudio.com/docs/copilot/customization/mcp-servers](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)
6. JetBrains ACP: [jetbrains.com/acp](https://www.jetbrains.com/acp/)
7. Zed ACP: [zed.dev/acp](https://zed.dev/acp)
8. OX Security fork vulnerability report (November 2025)

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/01-stigmergy-beyond-termites.md`

**References section (line 415):**
- [Bak, Tang & Wiesenfeld 1987] Self-Organized Criticality, *Physical Review Letters*
- [Bejan 1997] Constructal Law, *Int. J. Heat and Mass Transfer*
- [Bolici et al. 2009] Scalability in OSS via stigmergy, *AMCIS*
- [Bonabeau, Dorigo & Theraulaz 1999] *Swarm Intelligence*, Oxford University Press
- [Deneubourg et al. 1990] Argentine ant self-organization, *J. Insect Behavior*
- [Di Caro & Dorigo 1998] AntNet routing, *JAIR*
- [Dorigo, Maniezzo & Colorni 1996] Ant Colony Optimization, *IEEE SMC-B*
- [Elliott 2006] Stigmergic Collaboration, University of Melbourne
- [Fowler 1999] *Refactoring*, Addison-Wesley
- [Gibson 1979] Affordance theory, *Ecological Approach to Visual Perception*
- [Grassé 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Heylighen 2016] Universal Coordination Mechanism, *Cognitive Systems Research*
- [Hölldobler & Wilson 2008] *The Superorganism*, W.W. Norton
- [Kauffman 1993] *The Origins of Order*, Oxford University Press
- [Kennedy & Eberhart 1995] Particle Swarm Optimization, *IEEE ICNN*
- [Nealson, Platt & Hastings 1970] Quorum sensing, *J. Bacteriology*
- [Odling-Smee, Laland & Feldman 2003] *Niche Construction*, Princeton University Press
- [Parunak 1997] Engineering from natural MAS, *Ann. Oper. Res.*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
- [Pirolli & Card 1999] Information Foraging, *Psychological Review*
- [Sahin 2005] Swarm Robotics, *SR Workshop*
- [Von Frisch 1967] *The Dance Language and Orientation of Bees*, Belknap Press

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/02-git-as-stigmergy.md`

**References section (line 359):**
- [Bolici et al. 2009] Scalability in OSS via stigmergy, *AMCIS Proceedings*
- [Dourish, P. "The Parrot's Tale." *Proceedings of ECSCW*, 2001]
- [Elliott, M. 2006] Stigmergic Collaboration, University of Melbourne
- [Fowler, M. 1999] *Refactoring: Improving the Design of Existing Code*, Addison-Wesley
- [Gibson, J.J. 1979] *The Ecological Approach to Visual Perception*, Lawrence Erlbaum
- [Grassé 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Odling-Smee, Laland & Feldman 2003] *Niche Construction*, Princeton University Press
- [Pirolli & Card 1999] Information Foraging, *Psychological Review*
- [Theraulaz & Bonabeau 1999] History of Stigmergy, *Artificial Life*

**Inline citation (line 117):**
- Lee et al. 2026 (arXiv:2603.28052) — multi-armed bandit.

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/03-digital-pheromones.md`

**References section (line 717):**
- [Bonabeau, Dorigo & Theraulaz 1999] *Swarm Intelligence*, Oxford University Press
- [Deneubourg et al. 1990] Argentine ant self-organization, *J. Insect Behavior*
- [Dorigo, Maniezzo & Colorni 1996] Ant Colony Optimization, *IEEE SMC-B*
- [Grassé 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Grossman & Stiglitz 1980] Informationally Efficient Markets, *AER*
- [Hölldobler & Wilson 2008] *The Superorganism*, W.W. Norton
- [Nealson, Platt & Hastings 1970] Quorum sensing, *J. Bacteriology*
- [Parunak 1997] Engineering from natural MAS, *Ann. Oper. Res.*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/04-pheromone-kinds.md`

**References section (line 919):**
- [Bonabeau, Dorigo & Theraulaz 1999] *Swarm Intelligence*, Oxford University Press
- [Grassé 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Nealson, Platt & Hastings 1970] Quorum sensing, *J. Bacteriology*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
- [Wilson 1971] *The Insect Societies*, Belknap Press

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/05-pheromone-scope.md`

**References section (line 467):**
- [Bejan 1997] Constructal Law, *Int. J. Heat and Mass Transfer*
- [Fidge 1988] Timestamps in Message-Passing Systems, *ACSC*
- [Heard & Martienssen 2014] Transgenerational Epigenetic Inheritance, *Cell*
- [Hölldobler & Wilson 2008] *The Superorganism*, W.W. Norton
- [Lamport 1978] Time, Clocks, and Events, *CACM*
- [Roediger & Karpicke 2006] Test-Enhanced Learning, *Psychological Science*

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/06-agent-mesh-sync.md`

**References section (line 1028):**
- [Fidge 1988] Timestamps in Message-Passing Systems, *ACSC*
- [Lamport 1978] Time, Clocks, and Events, *CACM*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/07-morphogenetic-specialization.md`

**References section (line 1246):**
- [DeLanda 2006] *A New Philosophy of Society*, Continuum
- [Gierer & Meinhardt 1972] Biological Pattern Formation, *Kybernetik*
- [Kauffman 1993] *The Origins of Order*, Oxford University Press
- [Lotka 1925] *Elements of Physical Biology*, Williams & Wilkins
- [Shannon 1948] Mathematical Theory of Communication, *Bell System Technical Journal*
- [Turing 1952] Chemical Basis of Morphogenesis, *Phil. Trans. Royal Society B*
- [Volterra 1926] Fluctuations in Species Abundance, *Nature*

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/08-permissioned-subnets.md`

**References section (line 413):**
- [Buchanan 1965] Economic Theory of Clubs, *Economica*
- [Grossman & Stiglitz 1980] Informationally Efficient Markets, *AER*
- [Ostrom, E. 1990] *Governing the Commons*, Cambridge University Press

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/09-stigmergy-scaling.md`

**References section (line 377):**
- [Bejan 1997] Constructal Law, *Int. J. Heat and Mass Transfer*
- [Dorigo, Maniezzo & Colorni 1996] ACO, *IEEE SMC-B*
- [Grassé 1959] Termite mound stigmergy, *Insectes Sociaux*
- [Parunak 1997] Engineering from natural MAS, *Ann. Oper. Res.*
- [Reed 2001] The Law of the Pack, *Harvard Business Review*
- [Turing 1952] Chemical Basis of Morphogenesis, *Phil. Trans. Royal Society B*

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/11-collective-intelligence-metrics.md`

**References section (line 308):**
- Woolley, A. W. et al. 2010, *Science*, "Evidence for a Collective Intelligence Factor in the Performance of Human Groups."

---

## `/Users/will/dev/nunchi/roko/roko/docs/13-coordination/INDEX.md`

**Inline citation (line 102):**
- Hinton, Vinyals & Dean 2015, arXiv:1503.02531 — Knowledge distillation.

---

## `/Users/will/dev/nunchi/roko/roko/docs/14-identity-economy/15-regulatory-moat-and-current-status.md`

**Inline citations (lines 523–524):**
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176)
- Lee et al. 2026 — Meta-Harness (arXiv:2603.28052)

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/00-coala-9-step-pipeline.md`

**Inline citations and References section (lines 10, 18, 48, 93, 278, 284–307):**
- Sumers et al. 2023 (arXiv:2309.02427) — CoALA framework organizing taxonomy.
- Laird 2012 — Soar; Anderson 2007 — ACT-R; Sun et al. 2005 — CLARION (classical cognitive architecture research referenced in CoALA context).
- Cognitive Workspace paper (2025, arXiv:2508.13171) — 58.6% memory reuse rate.
- Chen et al. 2023, arXiv:2305.05176 — FrugalGPT, up to 98% cost reduction.
- OpenTelemetry gen_ai semantic conventions (OTel SIG, 2025) — span hierarchy.
- ABBench/TAMAS paper (arXiv:2503.06745) — 63% coefficient of variation in agent execution flow.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/01-universal-loop-mapping.md`

**References section (lines 176–177):**
- Sumers, Yao, Narasimhan & Griffiths 2023 — "Cognitive Architectures for Language Agents" (arXiv:2309.02427).
- Lee et al. 2026 — "Meta-Harness: Optimizing Agent Scaffolds" (arXiv:2603.28052).

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/02-chain-heartbeat-variant.md`

**References (lines 198–199):**
- Sumers et al. 2023 — CoALA framework (arXiv:2309.02427).
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176).

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/03-three-cognitive-speeds.md`

**References (lines 200–201):**
- Sumers et al. 2023 — CoALA framework (arXiv:2309.02427).
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176).

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/04-gamma-reactive-loop.md`

**References (lines 380–381):**
- Sumers et al. 2023 — CoALA framework (arXiv:2309.02427).
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176).

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/07-adaptive-clock.md`

**Inline citation (line 306):**
- Koudahl et al. 2024 — (arXiv:2412.10425). Factorized discrete POMDP for tractable active inference state spaces.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/08-dual-process-t0-t1-t2.md`

**References (lines 40–41, 615–619):**
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176, published 2024 TMLR).
- Zhang et al. 2025, arXiv:2502.11882 — DPT-Agent: Dual-process theory applied to LLM agent decision-making.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/09-16-t0-probes.md`

**References (lines 568):**
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176, published 2024 TMLR). Cascade architectures with intelligent routing achieve up to 98% cost reduction.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/10-active-inference-compute-allocation.md`

**References (lines 610–611):**
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176).
- Koudahl et al. 2024 — (arXiv:2412.10425). Factorized discrete POMDP for tractable active inference.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/11-active-inference-state-space.md`

**References (lines 10, 16, 309):**
- Koudahl et al. 2024 (arXiv:2412.10425) — Factorized state spaces making active inference tractable.
- VERSES AI's Genius platform — active inference.

---

## `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/INDEX.md`

**References (lines 88–98):**
- Sumers et al. 2023 — CoALA (arXiv:2309.02427).
- Chen et al. 2023 — FrugalGPT (arXiv:2305.05176).
- Koudahl et al. 2024 — Factorized discrete POMDP for tractable active inference (arXiv:2412.10425).

---

## `/Users/will/dev/nunchi/roko/roko/docs/17-lifecycle/00-vision-and-mortality-replaced.md`

**Inline citations (line 138):**
- Wilson, R.K. & McNaughton, B.L. "Reactivation of Hippocampal Ensemble Memories During Sleep." _Science_ 265, 1994.
- Hafner, D. et al. "DreamerV3." 2025.
- Wagner, U. et al. "Sleep inspires insight." _Nature_ 427, 2004.
- Lin, B. et al. "Sleep-time Compute." arXiv:2504.13171, 2025.

---

## `/Users/will/dev/nunchi/roko/roko/docs/17-lifecycle/12-academic-foundations.md`

This file is an exhaustive citation catalog. Complete list organized by section:

**Section 1 (Memory and Knowledge Management):**
- [EBBINGHAUS-1885] Ebbinghaus, H. _Memory: A Contribution to Experimental Psychology._ 1885.
- [ROEDIGER-KARPICKE-2006] Roediger, H.L. & Karpicke, J.D. "Test-Enhanced Learning." _Psychological Science_ 17(3), 2006.
- [RICHARDS-FRANKLAND-2017] Richards, B. & Frankland, P. "The Persistence and Transience of Memory." _Neuron_ 94(6), 2017.
- [ARBESMAN-2012] Arbesman, S. _The Half-Life of Facts._ Penguin, 2012.
- [BORGES-1942] Borges, J.L. "Funes the Memorious." 1942.
- [BOWER-1981] Bower, G.H. "Mood and Memory." _American Psychologist_ 36(2), 1981.
- [DAVIS-ZHONG-2017] Davis, R. & Zhong, H. "The Half-Life of Knowledge." _PNAS_, 2017.

**Section 2 (Evolutionary Computation and Artificial Life):**
- [RAY-1991] Ray, T. "An Approach to the Synthesis of Life." _Artificial Life II_, 1991.
- [LENSKI-2003] Lenski, R.E. et al. "The Evolutionary Origin of Complex Features." _Nature_ 423, 2003.
- [HAYFLICK-1965] Hayflick, L. & Moorhead, P.S. _Experimental Cell Research_ 25, 1961.
- [SHUVAEV-2024] Shuvaev, S. et al. "Encoding Innate Ability Through a Genomic Bottleneck." _PNAS_ 121(39), 2024.
- [BALDWIN-1896] Baldwin, J.M. "A New Factor in Evolution." _American Naturalist_ 30, 1896.
- [HINTON-NOWLAN-1987] Hinton, G.E. & Nowlan, S.J. "How Learning Can Guide Evolution." _Complex Systems_ 1, 1987.
- [HEARD-MARTIENSSEN-2014] Heard, E. & Martienssen, R. "Transgenerational Epigenetic Inheritance." _Cell_ 157(1), 2014.
- [KIRKWOOD-1977] Kirkwood, T.B.L. "Evolution of Ageing." _Nature_ 270, 1977.
- [VOSTINAR-2019] Vostinar, A. et al. "Suicidal Selection." _ALIFE_, 2019.
- [WENSINK-2020] Wensink, M.J. "Intrinsic Mortality and Premature Convergence." 2020.
- [EIGEN-1971] Eigen, M. "Self-Organization of Matter and the Evolution of Biological Macromolecules." _Naturwissenschaften_ 58, 1971.
- [MULLER-1964] Muller, H.J. "The Relation of Recombination to Mutational Advance." _Mutation Research_ 1, 1964.
- [BULL-2005] Bull, J.J. et al. "Quasispecies Made Simple." _PLoS Computational Biology_ 1(6), 2005.

**Section 3 (Game Theory and Mechanism Design):**
- [KREPS-MILGROM-ROBERTS-WILSON-1982] Kreps, D.M., Milgrom, P., Roberts, J. & Wilson, R. "Rational Cooperation in the Finitely Repeated Prisoners' Dilemma." _Journal of Economic Theory_ 27(2), 1982.
- [AXELROD-1984] Axelrod, R. _The Evolution of Cooperation._ Basic Books, 1984.
- [SIMS-2003] Sims, C. "Implications of Rational Inattention." _Journal of Monetary Economics_ 50(3), 2003.

**Section 4 (Philosophy and Existentialism):**
- [HEIDEGGER-1927] Heidegger, M. _Sein und Zeit_ (Being and Time). 1927.
- [NIETZSCHE-1882] Nietzsche, F. _Die fröhliche Wissenschaft_ (The Gay Science). 1882.
- [NIETZSCHE-1883] Nietzsche, F. _Also sprach Zarathustra_ (Thus Spoke Zarathustra). 1883.
- [NIETZSCHE-1874] Nietzsche, F. "On the Uses and Disadvantages of History for Life." 1874.
- [NIETZSCHE-1887] Nietzsche, F. _Zur Genealogie der Moral_ (On the Genealogy of Morals). 1887.
- [ARENDT-1958] Arendt, H. _The Human Condition._ University of Chicago Press, 1958.
- [PARFIT-1984] Parfit, D. _Reasons and Persons._ Oxford University Press, 1984.
- [JONAS-1966] Jonas, H. _The Phenomenon of Life._ Northwestern University Press, 1966.
- [CAMUS-1942] Camus, A. _Le Mythe de Sisyphe_ (The Myth of Sisyphus). 1942.
- [FREUD-1920] Freud, S. _Jenseits des Lustprinzips_ (Beyond the Pleasure Principle). 1920.
- [STIEGLER-2010] Stiegler, B. _Taking Care of Youth and the Generations._ Stanford University Press, 2010.
- [STIEGLER-2018] Stiegler, B. _The Neganthropocene._ Open Humanities Press, 2018.
- [WHITEHEAD-1929] Whitehead, A.N. _Process and Reality._ Macmillan, 1929.

**Section 5 (ML Degradation and Concept Drift):**
- [VELA-2022] Vela, A. et al. "Temporal Quality Degradation in AI Models." _Scientific Reports_, 2022.
- [ZLIOBAITĖ-2014] Zliobaitė, I. et al. "An Overview of Concept Drift Applications." _Big Data Analysis_, 2014.
- [LU-2020] Lu, J. et al. "Learning under Concept Drift: A Review." _IEEE TKDE_, 2020.
- [DANE-2010] Dane, E. "Reconsidering the Trade-Off Between Expertise and Flexibility." _Academy of Management Review_, 2010.
- [VAN-DE-VEN-2024] Van de Ven, G.M. et al. "Continual Learning with Neural Networks." 2024.
- [BESBES-2019] Besbes, O., Gur, Y. & Zeevi, A. "Optimal Exploration-Exploitation in a Multi-Armed Bandit Problem with Non-stationary Rewards." _Stochastic Systems_ 9(4), 2019.

**Section 6 (Neuroscience and Cognitive Science):**
- [DAMASIO-1994] Damasio, A. _Descartes' Error._ Putnam, 1994.
- [BECHARA-2000] Bechara, A. et al. "Emotion, Decision Making and the Orbitofrontal Cortex." _Cerebral Cortex_ 10(3), 2000.
- [KANERVA-2009] Kanerva, P. "Hyperdimensional Computing." _Cognitive Computation_ 1(2), 2009.
- [FRISTON-2010] Friston, K. "The Free-Energy Principle." _Nature Reviews Neuroscience_ 11, 2010.
- [PLUTCHIK-1980] Plutchik, R. _Emotion: A Psychoevolutionary Synthesis._ 1980.

**Section 7 (Collective Intelligence and Stigmergy):**
- [GRASSÉ-1959] Grassé, P.P. "La reconstruction du nid..." _Insectes Sociaux_ 6, 1959.
- [ROGERS-1988] Rogers, A. "Does Biology Constrain Culture?" _American Anthropologist_ 90(4), 1988.
- [ENQUIST-2007] Enquist, M. et al. "Critical Social Learning." _American Anthropologist_ 109(4), 2007.
- [BHATT-2023] Bhatt, U. et al. "Learning Few-Shot Imitation as Cultural Transmission." _Nature Communications_ 14, 7536, 2023.
- [BOURAHLA-2022] Bourahla, O. et al. "Knowledge Transmission and Improvement Across Generations." _AAMAS_, pp. 163-171, 2022.
- [PEREZ-2024] Perez, J. et al. "Artificial Generational Intelligence." arXiv:2406.00392, 2024.
- [WOOLLEY-2010] Woolley, A.W. et al. "Evidence for a Collective Intelligence Factor." _Science_ 330(6004), 2010.
- [ODLING-SMEE-2003] Odling-Smee, F.J. et al. _Niche Construction._ Princeton University Press, 2003.
- [HOLLDOBLER-WILSON-2008] Hölldobler, B. & Wilson, E.O. _The Superorganism._ W.W. Norton, 2008.

**Section 8 (Economics and Demurrage):**
- [GESELL-1916] Gesell, S. _Die natürliche Wirtschaftsordnung_ (The Natural Economic Order). 1916.
- [OSTROM-1990] Ostrom, E. _Governing the Commons._ Cambridge University Press, 1990.

**Section 9 (Self-Learning Systems):**
- [SHINN-2023] Shinn, N. et al. "Reflexion: Language Agents with Verbal Reinforcement Learning." _NeurIPS_, 2023.
- [ZHAO-EXPEL-2024] Zhao, A. et al. "ExpeL: LLM Agents Are Experiential Learners." _AAAI_, 2024. arXiv:2308.10144.
- [KHATTAB-DSPY-2024] Khattab, O. et al. "DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines." _ICLR_, 2024. arXiv:2310.03714.
- [WANG-VOYAGER-2023] Wang, G. et al. "Voyager: An Open-Ended Embodied Agent with Large Language Models." _TMLR_, 2023. arXiv:2305.16291.
- [ZHANG-ACE-2025] Zhang, Y. et al. "ACE: Agentic Context Engineering." arXiv:2510.04618, 2025.
- [CHHIKARA-MEM0-2025] Chhikara, P. et al. "Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory." arXiv:2504.19413, 2025.

**Section 10 (Dream and Offline Learning):**
- [WILSON-MCNAUGHTON-1994] Wilson, M.A. & McNaughton, B.L. "Reactivation of Hippocampal Ensemble Memories During Sleep." _Science_ 265, 1994.
- [HAFNER-DREAMERV3-2025] Hafner, D. et al. "DreamerV3." 2025.
- [WAGNER-2004] Wagner, U. et al. "Sleep Inspires Insight." _Nature_ 427, 2004.
- [LIN-SLEEPTIME-2025] Lin, B. et al. "Sleep-time Compute: Beyond Inference Scaling at Test-Time." arXiv:2504.13171, 2025.
- [WSCL-2024] "Wake-Sleep Consolidated Learning." arXiv:2401.08623, 2024.
- [XU-AMEM-2025] Xu, W. et al. "A-MEM: Agentic Memory for LLM Agents." arXiv:2502.12110, 2025.

**Section 11 (Affective Computing):**
- [CABRERA-2023] Cabrera-Paniagua, D. & Rubilar-Torrealba, R. "Autonomous Stock Market Agents with Somatic Markers." _JAIHC_, 2023.
- [VAN-DEN-BROEK-2023] Van den Broek, E. "Emotion Contagion in Multi-Agent Systems." _AAMAS_, 2023.
- [EMOTIONAL-RAG-2024] Zhang, Y. et al. "Emotional RAG." arXiv:2410.23041, 2024.
- [MARCH-1991] March, J.G. "Exploration and Exploitation in Organizational Learning." _Organization Science_ 2(1), 1991.

**Section 12 (AI Safety and Interruptibility):**
- [ORSEAU-ARMSTRONG-2016] Orseau, L. & Armstrong, S. "Safely Interruptible Agents." _UAI_, 2016.
- [ORSEAU-RING-2011] Orseau, L. & Ring, M. "Self-Modification and Mortality in Artificial Agents." _AGI_, 2011.
- [DEBENEDETTI-CAMEL-2025] Debenedetti, E. et al. "CaMeL: Defeating Prompt Injections by Design." arXiv:2503.18813, 2025.
- [ZHANG-CVARCPO-2025] Zhang, H. et al. "CVaR-CPO: Safe RL with Conditional Value-at-Risk Constraints." _IEEE TNNLS_, 2025.

**Section 13 (Distributed Systems):**
- [LAMPORT-1978] Lamport, L. "Time, Clocks, and the Ordering of Events in a Distributed System." _CACM_ 21(7), 1978.
- [FIDGE-1988] Fidge, C.J. "Timestamps in Message-Passing Systems." _ACSC_ 10(1), 1988.
- [SHANNON-1948] Shannon, C.E. "A Mathematical Theory of Communication." _Bell System Technical Journal_ 27, 1948.

---

## `/Users/will/dev/nunchi/roko/roko/docs/18-tools/00-tool-architecture.md`

**References section (line 922):**
- **OpenAPI 3.1.0** (OAI, 2021) — JSON Schema 2020-12 alignment. [spec](https://github.com/oai/openapi-specification/blob/main/versions/3.1.0.md)
- **schemars** — Derive JSON Schema from Rust types. [crate](https://crates.io/crates/schemars)
- **ReAct** (Yao et al., 2023) — Interleaved reasoning and tool actions. [paper](https://arxiv.org/abs/2210.03629)
- **AGNTCY Agent Directory** (Cisco, 2025) — Distributed tool discovery. [paper](https://arxiv.org/abs/2509.18787)

---

## `/Users/will/dev/nunchi/roko/roko/docs/18-tools/09-mcp-architecture.md`

**References section (line 708):**
- **MCP Specification 2025-11-25** (Anthropic / Linux Foundation) — Current protocol version. [spec](https://modelcontextprotocol.io/specification/2025-11-25)
- **MCP Specification 2025-03-26** — Introduced Streamable HTTP, tool annotations. [changelog](https://modelcontextprotocol.io/specification/2025-03-26)
- **MCP GitHub Repository** — Reference implementations and SDKs. [repo](https://github.com/modelcontextprotocol/modelcontextprotocol)

---

## `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/00-packaging-and-distribution.md`

**Inline URLs (lines 515–669):**
- `https://github.com/nunchi/roko/.github/workflows/release.yml@refs/tags/{}`
- `https://token.actions.githubusercontent.com`
- `https://github.com/rust-lang/crates.io-index`
- `https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml`
- `https://raw.githubusercontent.com/google/supply-chain/main/audits.toml`
- `https://raw.githubusercontent.com/bytecodealliance/supply-chain/main/audits.toml`

---

## `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/01-native-x86-arm.md`

**Inline URLs (line 268, 276):**
- `https://sh.rustup.rs` — Rust install script.
- `https://github.com/nunchi/roko.git`

---

## `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/05-daemon-systemd-linux.md`

**Inline URLs (lines 94, 184):**
- `Documentation=https://github.com/nunchi/roko`

---

## `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/06-cloud-fly-io.md`

**Inline URLs:**
- `https://roko-console.fly.dev/` (line 559)
- `https://staging-api.example.com/health` (line 575)
- jsDelivr CDN URLs for xterm.js (lines 528, 536, 537, 538)

---

## `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/11-remote-orchestrator.md`

**Inline URLs (lines 176–255):**
- `https://roko-serve.fly.dev/v1/projects`
- `https://github.com/user/my-app.git`

---

## `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/09-causal-microstructure-discovery.md`

**Citations for continuous optimization methods (line 1085):**
- Zheng, X., Aragam, B., Ravikumar, P., & Xing, E. P. (2018). "DAGs with NO TEARS: Continuous Optimization for Structure Learning." *NeurIPS 2018*.
- Yu, Y., Chen, J., Gao, T., & Yu, M. (2019). "DAG-GNN: DAG Structure Learning with Graph Neural Networks." *ICML 2019*.
- Bello, K., Aragam, B., & Ravikumar, P. (2022). "DAGMA: Learning DAGs via M-matrices and a Log-Determinant Acyclicity Characterization." *NeurIPS 2022*.
- Nazaret, A., Hoffman, M., et al. (2024). "Stable Differentiable Causal Discovery." *ICML 2024*, PMLR 235:37413-37445.
- Ng, I., Ghassami, A., & Zhang, K. (2024). "Structure Learning with Continuous Optimization: A Sober Look." *CLR 2024*.
- Zhou, J., Wang, M., et al. (2025). "Differentiable Constraint-Based Causal Discovery." *NeurIPS 2025*.

**Academic foundations (line 1096):**
- Pearl, J. (2009). *Causality: Models, Reasoning, and Inference*. 2nd ed. Cambridge University Press.
- Spirtes, P., Glymour, C., & Scheines, R. (2000). *Causation, Prediction, and Search*. 2nd ed. MIT Press.
- Granger, C. W. J. (1969). "Investigating Causal Relations by Econometric Models and Cross-spectral Methods." *Econometrica*, 37(3), 424-438.
- Pearl, J. (2019). "The seven tools of causal inference." *Communications of the ACM*, 62(3), 54-60.
- Peters, J., Janzing, D., & Schölkopf, B. (2017). *Elements of Causal Inference*. MIT Press.

---

## `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/10-predictive-geometry-and-resonant-patterns.md`

**Citations (line 953):**
- Adams, H., et al. (2017). "Persistence Images: A Stable Vector Representation of Persistent Homology." *JMLR*, 18(8), 1-35.
- Bauer, U. (2021). "Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes." *JACT*, 5(1), 91-119.
- Zhang, S., Xiao, M., & Wang, H. (2020). "GPU-Accelerated Computation of Vietoris-Rips Persistence Barcodes." *SoCG 2020*.
- Cohen-Steiner, D., Edelsbrunner, H., & Harer, J. (2007). "Stability of Persistence Diagrams." *DCG*, 37(1), 103-120.
- Luchinsky, A., & Islambekov, U. (2025). "TDAvec: Vectorization of Persistence Diagrams." *JOSS*, 10(114).

**Academic foundations (line 971):**
- Bubenik, P. (2015). "Statistical Topological Data Analysis using Persistence Landscapes." *JMLR*, 16(3), 77-102.
- Takens, F. (1981). "Detecting strange attractors in turbulence." *Lecture Notes in Mathematics*, 898, 366-381.
- Price, G. R. (1970). "Selection and Covariance." *Nature*, 227, 520-521.

---

## `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis/11-adversarial-signal-robustness.md`

**Citations (line 1028):**
- Cohen, J. M., Rosenfeld, E., & Kolter, Z. (2019). "Certified Adversarial Robustness via Randomized Smoothing." *ICML 2019*.
- Gowal, S., et al. (2018). "On the Effectiveness of Interval Bound Propagation for Training Verifiably Robust Models." arXiv:1810.12715.
- Mao, Z., et al. (2024). "Understanding Certified Training with Interval Bound Propagation." *ICLR 2024*.
- NeurIPS 2024. "ECLipsE: Efficient Compositional Lipschitz Constant Estimation for Deep Neural Networks."
- Steinhardt, G., Koh, P. W., & Liang, P. S. (2017). "Certified Defenses for Data Poisoning Attacks." *NeurIPS 2017*.

**Academic foundations (line 1046):**
- Huber, P. J. (1964). "Robust Estimation of a Location Parameter." *Annals of Mathematical Statistics*, 35(1), 73-101.
- Hodges, J. L., & Lehmann, E. L. (1963). "Estimates of Location Based on Rank Tests." *Annals of Mathematical Statistics*, 34(2), 598-611.
- Hampel, F. R. (1974). "The Influence Curve and its Role in Robust Estimation." *JASA*, 69(346), 383-393.
- Pearl, J. (2009). *Causality*. Cambridge University Press.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6).