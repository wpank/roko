# Collective Intelligence

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


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

---

## Network Effects and Scaling

- Metcalfe, R. (1995). Metcalfe's Law. As described in various subsequent analyses.

- Reed, D.P. (1999). That Sneaky Exponential — Beyond Metcalfe's Law to the Power of Community Building.

---

## Collective Calibration

- Central Limit Theorem (classical). Error in estimating a population mean scales as 1/sqrt(n) for independent observations.
  *Grounds: 31.6× calibration heuristic — at N=1,000 agents: sqrt(1000) = 31.6× faster calibration (theoretical upper bound under independence assumption). This is a Nunchi-derived scaling heuristic inspired by CLT, not a published theorem. Actual speedup will be less due to correlation, distribution shift, and coordination overhead.*

---

## Swarm Intelligence

- Holland, J.H. (1995). _Hidden Order: How Adaptation Builds Complexity_. Addison-Wesley.

---

## Knowledge Flow and Information Economics

- Hayek, F.A. (1945). The Use of Knowledge in Society. _American Economic Review_, 35(4), 519-530.

- Arrow, K.J. (1962). Economic Welfare and the Allocation of Resources for Invention. _NBER_.

---

## Representation Engineering

- Turner, A. et al. (2024). Steering Language Models With Activation Engineering. arXiv:2308.10248.

- Zou, A. et al. (2023). Representation Engineering: A Top-Down Approach to AI Transparency. arXiv:2310.01405.

---

## Thousand Brains Theory

- Hawkins, J., Ahmad, S., & Cui, Y. (2017). A Theory of How Columns in the Neocortex Enable Learning the Structure of the World. _Frontiers in Neural Circuits_.

- Clay, V., Leadholm, P., & Hawkins, J. (2024). The Thousand Brains Project. arXiv, 2024.

---

## Emergent Collective Intelligence in LLM Systems (2025)

- Emergent Coordination in Multi-Agent Language Models (2025). arXiv:2510.05174.
  *Grounds: Dynamical emergence — information-theoretic framework measuring whether dynamical emergence is present in multi-agent LLM systems. Shows that combining personas with theory-of-mind instructions produces identity-linked differentiation and goal-directed complementarity — patterns mirroring Woolley et al.'s C-Factor diagnostics. Directly validates Roko's Collective architecture.*

- Large Language Models Miss the Multi-Agent Mark (2025). arXiv:2505.21298.

---

## Cross-References

- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for stigmergic coordination
- See [05-biological-analogues.md](./05-biological-analogues.md) for superorganism theory
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic coordination
