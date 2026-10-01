# Security and Provenance

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


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

- Debenedetti, E. et al. (2025). Defeating Prompt Injections by Design. arXiv, 2025.

- OWASP (2025). Top 10 for LLM Applications. 2025.

- OWASP (2025). Agentic Security Initiative Top 10. 2025.

- Bai, Y. et al. (2022). Constitutional AI: Harmlessness from AI Feedback. arXiv:2212.08073.

---

## Safe Interruptibility

- Orseau, L. & Armstrong, S. (2016). Safely Interruptible Agents.

- Omohundro, S.M. (2008). The Basic AI Drives. _Proceedings of AGI_, 2008.

---

## Formal Undecidability

- Cohen, F. (1987). Computer Viruses: Theory and Experiments. _Computers & Security_, 6(1), 22-35.

---

## Adversarial Robustness

- Zhang, Q. et al. (2025). CVaR-Constrained Policy Optimization for Safe Reinforcement Learning with CVaR Constraints. 2025.


---

## TEE and Hardware Security

- Chuang et al. (2024). TEE.Fail. 2024.

---

## Content Provenance

- C2PA (Content Provenance and Authenticity). Coalition for Content Provenance and Authenticity. c2pa.org.
  *Grounds: Forensic AI — content provenance standard for tracking the origin and modification history of digital content. Grounds the Attestation field on Engrams: cryptographic proof of origin for every piece of agent-generated content.*

- W3C. Decentralized Identifiers (DIDs) v1.0. W3C Recommendation, 2022.

---

## Capability-Based Security

- Dennis, J.B. & Van Horn, E.C. (1966). Programming Semantics for Multiprogrammed Computations. _Communications of the ACM_, 9(3), 143-155.

---

## Agent-Specific Security Benchmarks

- Chen, J. et al. (2025). AgentGuard: Runtime Verification of AI Agents. arXiv:2509.23864.

- Bühler et al. (2025). AgentBound: Securing Execution Boundaries of AI Agents for AI Agents. arXiv:2510.21236.

- Rodriguez, A. et al. (2025). MCP-Guard: A Multi-Stage Defense-in-Depth Framework for Securing Model Context Protocol in Agentic AI. arXiv:2508.10991.

---

## Safe Reinforcement Learning

- Berkenkamp, F. et al. (2017). Safe Model-based Reinforcement Learning with Stability Guarantees. _NeurIPS_, 2017. arXiv:1705.08551.

- Schulman, J. et al. (2015). Trust Region Policy Optimization. _ICML_, 2015. arXiv:1502.05477.

- Alshiekh, M. et al. (2018). Safe Reinforcement Learning via Shielding. _AAAI_, 2018. arXiv:1708.08611.

---

## Formal Verification for AI Safety (2024-2025)


- Towards Guaranteed Safe AI (2024). A Framework for Ensuring Robust and Reliable AI Systems. arXiv:2405.06624.

- Model Checking Deep Neural Networks (2025). _Frontiers in Computer Science_, 2025.

---

## Cross-References

- See [19-regulatory-compliance.md](./19-regulatory-compliance.md) for EU AI Act, SEC/CFTC, and compliance frameworks
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC-8004 agent identity
- See topic [03-harness](../04-verification/INDEX.md) for full Harness layer design
