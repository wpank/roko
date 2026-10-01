# Protocol Standards

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


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

- Bryan, K. (2025). ERC-8001: Agent Coordination Framework. EIPs. Status: Final.

- Parikh, R. & Ross, J.M. (2025). ERC-8033: Agent Council Oracles. EIPs. Status: Draft.

- Crapis, D. et al. (2026). ERC-8183: Agentic Commerce Protocol. EIPs. Status: Draft.

---

## Token Standards

- Ethereum Foundation. ERC-20: Token Standard. EIPs.
  *Grounds: KORAI token — standard fungible token interface. KORAI (mainnet) and DAEJI (testnet) implement ERC-20 with 1% annual demurrage.*

- Ethereum Foundation. ERC-721: Non-Fungible Token Standard. EIPs.
  *Grounds: Korai Passport — ERC-721 soulbound token for agent identity. Each agent has a unique non-transferable NFT that encodes its capabilities and reputation.*

- Ethereum Foundation (2021). ERC-4337: Account Abstraction Using Alt Mempool. EIPs.

- Ethereum Foundation (2024). ERC-7683: Cross Chain Intents. EIPs.

---

## Micropayments

- Coinbase & Cloudflare (2025). x402: HTTP 402 Payment Required Protocol for Machine-to-Machine Micropayments.

---

## Attestation and Provenance

- Ethereum Attestation Service (2024-2026). EAS Documentation. attest.sh.

- Uniswap Labs (2022). Permit2: Signature-Based Token Approvals.

---

## Agent Protocols

- Google (2025). Agent-to-Agent (A2A) Protocol Specification.

- Anthropic (2024). Model Context Protocol (MCP) Specification.

- Virtuals Protocol (2025). Agent Commerce Protocol: Agent-to-Agent Economic Coordination.

---

## Cryptographic Primitives

- Merkle, R.C. (1987). A Digital Signature Based on a Conventional Encryption Function. In _CRYPTO '87_, LNCS 293, 369-378.

- Goldwasser, S., Micali, S., & Rackoff, C. (1985). The Knowledge Complexity of Interactive Proof Systems. In _STOC '85_, 291-304.

- Ben-Sasson, E. et al. (2018). Scalable, Transparent, and Post-Quantum Secure Computational Integrity. Cryptology ePrint Archive, 2018/046.

- Benet, J. (2014). IPFS — Content Addressed, Versioned, P2P File System. arXiv:1407.3561.
  *Grounds: Content addressing — content-addressed P2P storage. Roko's BLAKE3 content-addressed Engrams follow the same content-addressing principle.*

- Szabo, N. (1997). Formalizing and Securing Relationships on Public Networks. _First Monday_, 2(9).

---

## Cross-References

- See [08-security-and-provenance.md](./08-security-and-provenance.md) for security architecture
- See [19-regulatory-compliance.md](./19-regulatory-compliance.md) for compliance frameworks
- See [21-mechanism-design.md](./21-mechanism-design.md) for token economics
