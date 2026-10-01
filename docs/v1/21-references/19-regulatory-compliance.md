# Regulatory Compliance

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


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

---

## Financial Services Regulation

- U.S. Securities and Exchange Commission (SEC). Securities Exchange Act of 1934; Investment Advisers Act of 1940.

- U.S. Commodity Futures Trading Commission (CFTC). Commodity Exchange Act.
  *Grounds: Derivatives compliance — agents operating with DeFi derivatives must consider CFTC jurisdiction. Forensic AI replay enables demonstrating that agent decisions followed programmatic rules.*

- European Union. MiFID II (Markets in Financial Instruments Directive). Directive 2014/65/EU.

---

## Data Privacy

- European Union. GDPR (General Data Protection Regulation). Regulation (EU) 2016/679.

- U.S. Congress. HIPAA (Health Insurance Portability and Accountability Act). 1996.

---

## Financial Reporting

- U.S. Congress. SOX (Sarbanes-Oxley Act). 2002.

---

## Content Provenance

- C2PA. Coalition for Content Provenance and Authenticity Standard. c2pa.org.
  *Grounds: Content provenance — industry standard for tracking content origin and modification history. Grounds the Attestation field on Engrams. Cross-referenced in [08-security-and-provenance.md](./08-security-and-provenance.md).*

---

## DeFi Compliance


- Schrepel, T. (2024). The Trust Dilemma in Autonomous Agent Systems. _Stanford Law Review_.

---

## Cross-References

- See [08-security-and-provenance.md](./08-security-and-provenance.md) for security architecture
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC standards
