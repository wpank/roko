# 39-19 Regulatory Compliance -- Annotated Reference Map

> Regulatory frameworks, compliance obligations, and legal analysis relevant to
> Roko's audit trail, data governance, and agent transparency systems.
>
> **v3 depth file** -- updated 2026-09-15.

---

## AI-Specific Regulation

**[EU, 2024]** *EU AI Act.* Regulation (EU) 2024/1689.
AI risk classification with proportional obligations. High-risk AI systems require conformity assessment, risk management, data governance, and logging. Roko's Signal DAG, episode logs, and Gate pipeline provide the audit infrastructure for compliance. The Attestation field on Signals provides content provenance tracking.

---

## Financial Regulation

**[SEC]** *Securities Exchange Act of 1934; Investment Advisers Act of 1940.* U.S. SEC.
Investment adviser regulations applicable to autonomous agents managing financial assets. The Signal DAG provides a complete, content-addressed audit trail of every decision and its inputs. Episode logs satisfy record-keeping requirements.

**[CFTC]** *Commodity Exchange Act.* U.S. CFTC.
Derivatives compliance for DeFi agents. Roko's replay capability (`roko replay <hash>`) enables decision replay for regulatory examination. Every agent decision can be traced to its causal inputs.

**[EU]** *MiFID II (Markets in Financial Instruments Directive).* Directive 2014/65/EU.
Algorithmic trading record-keeping requirements including timestamps, order parameters, and execution details. The lineage DAG and episode logs satisfy algorithmic trading audit requirements.

---

## Data Protection

**[EU]** *GDPR (General Data Protection Regulation).* Regulation (EU) 2016/679.
Data minimization and right to be forgotten. Knowledge decay via Ebbinghaus half-lives provides a structural implementation of data minimization -- old data automatically ages out. The Curator's active pruning provides structural right-to-erasure compliance.

**[US Congress, 1996]** *HIPAA (Health Insurance Portability and Accountability Act).* 1996.
Healthcare data privacy. Capability-based access control and permissioned subnets in Agent Groups support compliance by restricting data access to authorized agents with appropriate capabilities.

---

## Audit and Accountability

**[US Congress, 2002]** *SOX (Sarbanes-Oxley Act).* 2002.
Audit trail requirements for financial reporting. The content-addressed Signal DAG provides immutable audit trails: every Signal is hash-linked to its predecessors, making tampering detectable.

**[C2PA]** *Coalition for Content Provenance and Authenticity Standard.* c2pa.org.
Content provenance tracking standard for establishing the origin and modification history of digital content. Grounds the Attestation field on Signals.

---

## DeFi-Specific Compliance

---

## Trust and Liability

**[Schrepel, 2024]** *The Trust Dilemma in Autonomous Agent Systems.* Stanford Law Review.
Legal analysis of trust and liability in autonomous agent systems. Examines how existing legal frameworks apply to agents that make independent decisions. Informs the design of the Safety layer's audit and evidence mechanisms.

---

## Affective Data Privacy

**[Fabiano, 2025]** *Affective Computing and Emotional Data: Challenges in Privacy Regulations.* arXiv:2509.20153.
Privacy implications of emotion-aware AI under the EU AI Act. The Daimon's affect state could constitute sensitive personal data in regulated domains. Informs compliance requirements for Daimon deployment.

---

## Explainability Requirements

**[EU, 2024]** *EU AI Act, Article 13: Transparency and Provision of Information to Users.* Regulation (EU) 2024/1689.
High-risk AI systems must be sufficiently transparent that users can interpret outputs and use them appropriately. Roko's episode logs, Signal DAG replay, and Gate pipeline verdicts provide the interpretability infrastructure.

**[Ribeiro et al., 2016]** *"Why Should I Trust You?": Explaining the Predictions of Any Classifier.* KDD 2016.
LIME: Local Interpretable Model-agnostic Explanations. Informs how individual agent decisions can be explained post-hoc through the episode narrative and Signal chain.

---

## Agent Liability Frameworks

**[Calo, 2015]** *Robotics and the Lessons of Cyberlaw.* California Law Review, 103, 513--563.
Legal analysis of autonomous agent liability. Identifies the challenge of attributing responsibility when agents make independent decisions. Informs the design of the Signal DAG as evidence for liability attribution.

**[Vladeck, 2014]** *Machines Without Principals: Liability Rules and Artificial Intelligence.* Washington Law Review, 89(1), 117--150.
Principal-agent law applied to autonomous systems. When agents act without direct human oversight, the question of who bears liability becomes structural. Grounds the audit and evidence mechanisms in Roko's safety layer.

---

## International Standards

**[ISO/IEC 42001, 2023]** *AI Management System Standard.* ISO.
International standard for AI management systems. Provides organizational framework for responsible AI deployment. Roko's workspace structure (.roko/ directory, configuration, and state management) aligns with the standard's lifecycle approach.

**[NIST, 2023]** *AI Risk Management Framework (AI RMF 1.0).* NIST AI 100-1.
Risk management framework for AI systems. Provides vocabulary and process for identifying, assessing, and mitigating AI risks. Informs Roko's `roko doctor` diagnostics and health monitoring approach.

---

## Cross-References

- Security architecture: [08-security-and-provenance](./08-security-and-provenance.md)
- Protocol standards: [22-protocol-standards](./22-protocol-standards.md)
- Content provenance: depth/12-safety/
