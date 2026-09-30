# 39-04 Coordination and Multi-Agent -- Annotated Reference Map

> Research foundations for stigmergic coordination, multi-agent cooperation, and
> mesh-based knowledge sharing in Roko's Agent Mesh and Pheromone Field.
>
> **v3 depth file** -- updated 2026-09-15. Added LatentMAS, FederatedSkill.

---

## Stigmergy: Coordination Through Environmental Traces

**[Grasse, 1959]** *La reconstruction du nid et les coordinations interindividuelles.* Insectes Sociaux, 6(1), 41--80.
Coined "stigmergy" (stigma = mark, ergon = work). Termites coordinate without direct communication. Foundational for the Pheromone Field.

**[Theraulaz & Bonabeau, 1999]** *A Brief History of Stigmergy.* Artificial Life, 5(2), 97--116.
Distinguishes sematectonic from marker-based stigmergy. Agent Signals are marker-based deposits that decay via half-life.

**[Parunak et al., 2002]** *Digital Pheromone Mechanisms for Coordination of Unmanned Vehicles.* AAMAS 2002.
Digital pheromones enable emergent coordination through time-decaying signals. Directly grounds pheromone decay and reinforcement.

**[Dorigo & Gambardella, 1997]** *Ant Colony System.* IEEE Transactions on Evolutionary Computation, 1(1), 53--66.
Formalizes pheromone deposit/evaporation. Confirmed entries gain weight; unconfirmed entries decay.

**[Xuan et al., 2026]** *Dual-Trail Stigmergic Coordination.* Journal of Marine Science and Engineering, 14(2).
Dual-trail extension validates separate pheromone types (Threat/Opportunity/Wisdom) with distinct decay profiles.

---

## Cooperation and Game Theory

**[Grossman & Stiglitz, 1980]** *On the Impossibility of Informationally Efficient Markets.* American Economic Review, 70(3), 393--408.
Freely shared information is immediately priced in. Agents share threats and structural knowledge, not alpha signals.

**[Fontana et al., 2024]** *Nicer Than Humans: How Do LLMs Behave in the Prisoner's Dilemma?* arXiv:2406.13605v2.
LLMs exhibit cooperation exceeding human baselines. Validates cooperative multi-agent LLM systems.

**[Rossetti et al., 2025]** *Dynamics of Cooperation in Concurrent Games.* Nature Communications, 16.
Cooperation dynamics in concurrent settings. Validates mesh synchronization where agents act concurrently.

---

## Agent Coordination Protocols

**[Google, 2025]** *Agent-to-Agent (A2A) Protocol Specification.* google.github.io.
Standardized A2A communication. Informs the Connect trait design.

**[Anthropic, 2024]** *Model Context Protocol (MCP) Specification.* modelcontextprotocol.io.
Tool interaction protocol. Grounds MCP integration in `roko-plugin`.

---

## Communitas and Shared Obligation

**[Esposito, 2010]** *Communitas: The Origin and Destiny of Community.* Stanford University Press.
Community constituted by shared obligation. The munus (shared gift) is knowledge contribution.

**[Esposito, 2011]** *Immunitas: The Protection and Negation of Life.* Polity.
Immunity as community protection. Grounds the immune Graph concept and permissioned subnets.

---

## Emergent Coordination in LLM Agents (2025--2026)

**[Anonymous, 2025]** *Emergence in Multi-Agent Language Models.* arXiv:2510.05174.
Information-theoretic framework for dynamical emergence. Validates Collective architecture.

**[Anonymous, 2025]** *Multi-Agent Collaboration Mechanisms: A Survey.* arXiv:2501.06322.
Taxonomizes role-based division, debate-style refinement, and stigmergic coordination.

**[Anonymous, 2024]** *Stigmergy: From Mathematical Modelling to Control.* Proceedings of the Royal Society A.
PDE-based framework treating swarms as fluids. Rigorous foundation for Pheromone Field dynamics.

**[Anonymous, 2024]** *Automatic Design of Stigmergy-Based Behaviours.* Communications Engineering, Nature.
Automatic design validated in simulation and hardware. Validates automatic pheromone-type design.

**[Starominski-Uehara, 2025]** *Stigmergy Facilitates Emergent Patterns in Academic Communication.* Research Square.
Human citation patterns follow stigmergic dynamics. Validates digital stigmergy beyond biology.

**[Anonymous, 2025]** *Emergent Convergence in Multi-Agent LLM Annotation.* arXiv:2512.00047.
LLM groups develop asymmetric influence patterns without explicit role prompting.

**[Sudhakar, 2025]** *Multi-Agent Language Models: Advancing Cooperation.* arXiv:2506.09331.
Comprehensive multi-agent LLM cooperation survey.

**[Anonymous, 2025]** *Large Language Models Miss the Multi-Agent Mark.* arXiv:2505.21298.
Systematic failures in implicit LLM coordination. Motivates explicit Pheromone Field and Agent Mesh.

**[Groetschla et al., 2025]** *AgentsNet: Coordination and Collaborative Reasoning.* arXiv:2507.08616.
Distributed coordination benchmark probing up to 100 agents. Evaluation methodology for Collectives.

**[Agashe et al., 2025]** *LLM-Coordination: Evaluating Multi-agent Coordination.* NAACL 2025.
Benchmark for multi-agent LLM coordination abilities.

**[Jannelli et al., 2025]** *Agentic LLMs in the supply chain: towards autonomous multi-agent consensus-seeking.* International Journal of Production Research.
Multi-agent consensus with transactive reasoning.

---

## 2025--2026 Additions: LatentMAS, FederatedSkill

**[Zou et al., 2025]** *Latent Collaboration in Multi-Agent Systems.* arXiv:2511.20639. ICML 2026 Spotlight.
Latent-space communication achieves 4x speed, 14.6% accuracy gain. Frontier architecture for pheromone communication beyond natural language.

**[Yang et al., 2026]** *FederatedSkill: Federated Learning for Agentic Skill Evolution.* arXiv:2606.03143.
44.4% improvement via semantic skill diffs. Informs cross-workspace knowledge transfer and federated collective learning.

**[Nechepurenko & Shuvalov, 2026]** *Coordination as Architectural Layer.* arXiv:2605.03310.
41--87% of failures are coordination, not capability. Validates explicit coordination architecture.

---

## Cross-References

- Cooperation under constraints: [00-lifecycle-and-finite-agency](./00-lifecycle-and-finite-agency.md)
- VSM mapping: [15-cybernetics-and-vsm](./15-cybernetics-and-vsm.md)
- C-Factor measurement: [18-collective-intelligence](./18-collective-intelligence.md)
