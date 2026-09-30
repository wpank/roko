# 39-22 Protocol Standards -- Annotated Reference Map

> Blockchain protocol standards, agent identity specifications, agent-to-agent
> communication protocols, and interoperability frameworks relevant to Roko's
> on-chain and inter-agent subsystems.
>
> **v3 depth file** -- updated 2026-09-15. Added A2A v1.0, MCP to Linux Foundation.

---

## Agent Identity

**[Bryan, 2024]** *ERC-8004: Agent Identity.* Ethereum EIPs.
Agent identity standard. ERC-721 soulbound with capabilityList bitmask, domainStakes, reputationTracks, teeAttestation, systemPromptHash (ventriloquist defense), tier classification, and slashHistory. Grounds the Korai Passport in `roko-chain`.

**[Bryan, 2025]** *ERC-8001: Agent Coordination Framework.* Ethereum EIPs (Final).
Protocol-level primitives for on-chain multi-agent interaction. Provides the coordination substrate for the Identity Economy.

**[Parikh & Ross, 2025]** *ERC-8033: Agent Council Oracles.* Ethereum EIPs (Draft).
On-chain oracle mechanism for multi-agent council decisions. Enables verified group consensus outcomes.

**[Crapis et al., 2026]** *ERC-8183: Agentic Commerce Protocol.* Ethereum EIPs (Draft).
Protocol for agent-to-agent economic transactions. Provides the wire format for autonomous value exchange.

---

## Token Standards

**[ERC-20]** *ERC-20: Token Standard.* Ethereum EIPs.
Standard fungible token interface. KORAI and DAEJI implement ERC-20 with 1% annual demurrage.

**[ERC-721]** *ERC-721: Non-Fungible Token Standard.* Ethereum EIPs.
NFT standard. Korai Passport is ERC-721 soulbound -- non-transferable identity tokens.

**[ERC-4337, 2021]** *ERC-4337: Account Abstraction Using Alt Mempool.* Ethereum EIPs.
Smart contract wallets with custom validation logic, gas sponsorship, and batched transactions. Enables agent account abstraction.

**[ERC-7683, 2024]** *ERC-7683: Cross Chain Intents.* Ethereum EIPs.
Standardized cross-chain intent format. Enables agents to express and execute cross-chain transaction intents.

---

## Agent Communication Protocols

**[Google, 2025]** *Agent-to-Agent (A2A) Protocol Specification v1.0.* google.github.io.
Standardized agent-to-agent communication protocol. Discovery via Agent Cards, task lifecycle management (submitted/working/input-required/completed/failed), streaming via SSE, and push notifications. Informs the Connect trait design and the relay adapter in `roko-connectivity`. The v1.0 specification (April 2025) formalized task states and multi-turn interactions.

**[Anthropic, 2024; Linux Foundation, 2025]** *Model Context Protocol (MCP) Specification.* modelcontextprotocol.io.
Tool interaction protocol for LLMs. Standardizes resources, tools, prompts, and sampling between hosts and servers. Roko implements MCP client in `roko-agent` and MCP server in `roko-mcp-code`. The Linux Foundation adopted MCP governance in late 2025, establishing it as an industry standard alongside A2A.

---

## Micropayments

**[Coinbase & Cloudflare, 2025]** *x402: HTTP 402 Payment Required Protocol.* 2025.
Machine-to-machine micropayments at < $0.001/transaction. Sub-second USDC settlement on Base. Enables the self-funding economic cycle: agent earns from knowledge, spends on compute, produces value, earns more. Grounds the payments subsystem in `roko-chain` and `roko-serve`.

---

## Attestation and Provenance

**[EAS, 2024--2026]** *Ethereum Attestation Service.* attest.sh.
On-chain attestation for Signal verification. Provides infrastructure for the Attestation field on Signals -- cryptographic proof of origin.

**[Uniswap Labs, 2022]** *Permit2: Signature-Based Token Approvals.* 2022.
Gasless token approvals for agent transactions. Enables efficient token authorization without on-chain gas costs per approval.

**[Virtuals Protocol, 2025]** *Agent Commerce Protocol.* 2025.
Agent-to-agent economic coordination protocol for autonomous value exchange between agent systems.

---

## Cryptographic Primitives

**[Merkle, 1987]** *A Digital Signature Based on a Conventional Encryption Function.* CRYPTO '87, LNCS 293, 369--378.
Merkle trees for content-addressed verification. Ancestor of BLAKE3 content hashing used throughout Roko's Signal DAG.

**[Goldwasser, Micali & Rackoff, 1985]** *The Knowledge Complexity of Interactive Proof Systems.* STOC '85, 291--304.
Zero-knowledge proofs. Grounds privacy-preserving knowledge verification without revealing the knowledge itself.

**[Ben-Sasson et al., 2018]** *Scalable, Transparent, and Post-Quantum Secure Computational Integrity (STARKs).* Cryptology ePrint 2018/046.
Post-quantum transparent proofs without trusted setup. Potential future proving system for agent computation verification.

**[Benet, 2014]** *IPFS -- Content Addressed, Versioned, P2P File System.* arXiv:1407.3561.
Content-addressed P2P storage. Same content-addressing principle as BLAKE3 Signals.

**[Szabo, 1997]** *Formalizing and Securing Relationships on Public Networks.* First Monday, 2(9).
Smart contracts as self-enforcing digital agreements. The Policy trait enforces agent behavior constraints as computational contracts.

---

## Agent Data Exchange

**[Song et al., 2025]** *Agent Data Protocol (ADP).* arXiv:2510.24702.
Structured protocol for agent data exchange. Provides wire format for structured data between agents.

---

## Cross-References

- Security architecture: [08-security-and-provenance](./08-security-and-provenance.md)
- Regulatory compliance: [19-regulatory-compliance](./19-regulatory-compliance.md)
- Token economics: [21-mechanism-design](./21-mechanism-design.md)
