# 39-08 Security and Provenance -- Annotated Reference Map

> Research foundations for agent safety, adversarial robustness, capability-based
> security, and content provenance in Roko's safety layer.
>
> **v3 depth file** -- updated 2026-09-15. Added ActPlane, NeuroTaint, VIGIL, DReST.

---

## Agent Security Frameworks

**[Debenedetti et al., 2025]** *Defeating Prompt Injections by Design.* arXiv.
Capability-based authorization separating control flow from data flow. Grounds the permission model in `roko-agent/safety`.

**[OWASP, 2025]** *Top 10 for LLM Applications.* 2025.
Memory poisoning ranked high. Knowledge decay and tier validation serve as structural defenses.

**[OWASP, 2025b]** *Agentic Security Initiative Top 10.* 2025.
Agent-specific threats: confused deputy, privilege escalation, tool misuse. Grounds safety layer design.

**[Bai et al., 2022]** *Constitutional AI.* arXiv:2212.08073.
Constitutional constraints as structural guarantees. Informs Policy trait design.

---

## Safe Interruptibility

**[Omohundro, 2008]** *The Basic AI Drives.* Proceedings of AGI 2008.
Instrumental convergence: self-preservation and resource acquisition. Informs pathological behavior prevention.

**[Cohen, 1987]** *Computer Viruses: Theory and Experiments.* Computers & Security, 6(1), 22--35.
Perfect detection is undecidable. Defense must be structural: decay, validation, provenance tracking.

---

## Adversarial Robustness

**[Zhang et al., 2025]** *CVaR-Constrained Policy Optimization for Safe Reinforcement Learning.* 2025.
Tail risk management via CVaR constraints. Informs safety-bounded optimization.

**[Kaspersky, 2026]** *OpenClaw: 512 Vulnerabilities.* 2026.
512 vulnerabilities including 8 critical in a competing framework. Validates security-first approach.

**[Chuang et al., 2024]** *TEE.Fail.* 2024.
TEE broken under $1,000. TEE is one layer, not sole defense. Grounds multi-layer security.

---

## Content Provenance

**[C2PA]** *Coalition for Content Provenance and Authenticity.* c2pa.org.
Content provenance standard. Grounds the Attestation field on Signals.

**[W3C, 2022]** *Decentralized Identifiers (DIDs) v1.0.* W3C Recommendation.
Agent identity standard for decentralized identification.

---

## Capability-Based Security

**[Dennis & Van Horn, 1966]** *Programming Semantics for Multiprogrammed Computations.* Communications of the ACM, 9(3), 143--155.
Authority as unforgeable tokens. Historical grounding for the capability system.

---

## Agent Security Benchmarks

**[Koohestani, 2025]** *AgentGuard: Runtime Verification of AI Agents.* arXiv:2509.23864.
Safety evaluation via tool use testing. Informs safety gate design.

**[Bühler et al., 2025]** *AgentBound: Securing Execution Boundaries of AI Agents.* arXiv:2510.21236.
Tool substitution attack prevention. Grounds secure tool binding.

**[Xing et al., 2025]** *MCP-Guard.* arXiv preprint; Findings of ACL 2026.
MCP output sanitization validation. Grounds output screening.

---

## Safe Reinforcement Learning

**[Berkenkamp et al., 2017]** *Safe Model-based RL with Stability Guarantees.* NeurIPS 2017. arXiv:1705.08551.
Lyapunov stability for safe exploration. Grounds safety-constrained learning.

**[Schulman et al., 2015]** *Trust Region Policy Optimization.* ICML 2015. arXiv:1502.05477.
Conservative policy updates. Informs bounded strategy changes.

**[Alshiekh et al., 2018]** *Safe RL via Shielding.* AAAI 2018. arXiv:1708.08611.
Runtime shields override unsafe actions. Grounds pre/post safety checks.

---

## Formal Verification (2024--2025)

**[Dalrymple et al., 2024]** *Towards Guaranteed Safe AI.* arXiv:2405.06624.
World model + safety spec + verifier = quantitative guarantees. Maps to NeuroStore + Policy + Gate.

**[Sbai, 2025]** *Model Checking Deep Neural Networks.* Frontiers in Computer Science.
Temporal logic (LTL, CTL) for neural network verification.

**[Odersky et al., 2026]** *Tracking Capabilities for Safer Agents.* arXiv:2603.00991. Best Paper ACM CAIS 2026.
Capture checking as complement to Roko's capability system.

---

## 2026 Additions: ActPlane, NeuroTaint, VIGIL, DReST

**[Zheng et al., 2026]** *ActPlane: Programmable OS-Level Policy Enforcement for Agent Harnesses.* arXiv:2606.25189.
eBPF kernel-level enforcement for agent tool calls. Deployable substrate for the tool cooldown/isolation policies in `roko-agent/safety`.

**[Cai et al., 2026]** *Ghost in the Agent: Redefining Information Flow Tracking for LLM Agents.* arXiv:2604.23374.
Semantic taint tracking extending classical IFC models to LLM outputs. Complements the TaintTracker trust-origin lattice.

**[Li et al., 2026]** *VIGIL: SMT Behavioral Specs.* arXiv:2606.26524.
SMT-based behavioral specifications for agent verification. Complement to the gate pipeline's structural checks.

---

## Cross-References

- Regulatory compliance: [19-regulatory-compliance](./19-regulatory-compliance.md)
- ERC-8004 identity: [22-protocol-standards](./22-protocol-standards.md)
- Trust-origin lattice: depth/12-safety/
