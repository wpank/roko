# 39-08 Security and Provenance -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for agent safety, adversarial robustness, capability-based
> security, and content provenance in Roko's safety layer.
>
> **v3 depth file** -- updated 2026-09-15. Added ActPlane, NeuroTaint, VIGIL, DReST.

---

## Agent Security Frameworks

**[Debenedetti et al., 2025]** *Defeating Prompt Injections by Design.* arXiv.

**[OWASP, 2025]** *Top 10 for LLM Applications.* 2025.

**[OWASP, 2025b]** *Agentic Security Initiative Top 10.* 2025.

**[Bai et al., 2022]** *Constitutional AI.* arXiv:2212.08073.

---

## Safe Interruptibility

**[Omohundro, 2008]** *The Basic AI Drives.* Proceedings of AGI 2008.

**[Cohen, 1987]** *Computer Viruses: Theory and Experiments.* Computers & Security, 6(1), 22--35.

---

## Adversarial Robustness

**[Zhang et al., 2025]** *CVaR-Constrained Policy Optimization for Safe Reinforcement Learning.* 2025.

**[Kaspersky, 2026]** *OpenClaw: 512 Vulnerabilities.* 2026.

**[Chuang et al., 2024]** *TEE.Fail.* 2024.

---

## Content Provenance

**[C2PA]** *Coalition for Content Provenance and Authenticity.* c2pa.org.

**[W3C, 2022]** *Decentralized Identifiers (DIDs) v1.0.* W3C Recommendation.

---

## Capability-Based Security

**[Dennis & Van Horn, 1966]** *Programming Semantics for Multiprogrammed Computations.* Communications of the ACM, 9(3), 143--155.

---

## Agent Security Benchmarks

**[Koohestani, 2025]** *AgentGuard: Runtime Verification of AI Agents.* arXiv:2509.23864.

**[Bühler et al., 2025]** *AgentBound: Securing Execution Boundaries of AI Agents.* arXiv:2510.21236.

**[Xing et al., 2025]** *MCP-Guard.* arXiv preprint; Findings of ACL 2026.

---

## Safe Reinforcement Learning

**[Berkenkamp et al., 2017]** *Safe Model-based RL with Stability Guarantees.* NeurIPS 2017. arXiv:1705.08551.

**[Schulman et al., 2015]** *Trust Region Policy Optimization.* ICML 2015. arXiv:1502.05477.

**[Alshiekh et al., 2018]** *Safe RL via Shielding.* AAAI 2018. arXiv:1708.08611.

---

## Formal Verification (2024--2025)

**[Dalrymple et al., 2024]** *Towards Guaranteed Safe AI.* arXiv:2405.06624.

**[Sbai, 2025]** *Model Checking Deep Neural Networks.* Frontiers in Computer Science.

**[Odersky et al., 2026]** *Tracking Capabilities for Safer Agents.* arXiv:2603.00991. Best Paper ACM CAIS 2026.
Capture checking as complement to Roko's capability system.

---

## 2026 Additions: ActPlane, NeuroTaint, VIGIL, DReST

**[Zheng et al., 2026]** *ActPlane: Programmable OS-Level Policy Enforcement for Agent Harnesses.* arXiv:2606.25189.
eBPF kernel-level enforcement for agent tool calls. Deployable substrate for the tool cooldown/isolation policies in `roko-agent/safety`.

**[Cai et al., 2026]** *Ghost in the Agent: Redefining Information Flow Tracking for LLM Agents.* arXiv:2604.23374.

**[Li et al., 2026]** *VIGIL: SMT Behavioral Specs.* arXiv:2606.26524.

---

## Cross-References

- Regulatory compliance: [19-regulatory-compliance](./19-regulatory-compliance.md)
- ERC-8004 identity: [22-protocol-standards](./22-protocol-standards.md)
- Trust-origin lattice: depth/12-safety/
