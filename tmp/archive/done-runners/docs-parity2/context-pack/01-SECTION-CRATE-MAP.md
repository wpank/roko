# Section-to-Crate Map

This table maps each docs section to its target crate(s), priority, and group.

| Batch | Section | Crate(s) | Priority | Group | Dependencies |
|-------|---------|----------|----------|-------|--------------|
| DP00 | 00-architecture | roko-core | P0 | core | none |
| DP01 | 01-orchestration | roko-orchestrator,roko-cli | P0 | core | DP00 |
| DP02 | 02-agents | roko-agent | P0 | core | DP00 |
| DP03 | 03-composition | roko-compose | P0 | core | DP00 |
| DP04 | 04-verification | roko-gate | P0 | core | DP00 DP01 |
| DP05 | 05-learning | roko-learn | P0 | core | DP00 |
| DP06 | 06-neuro | roko-neuro,roko-primitives | P1 | extensions | DP00 DP05 |
| DP07 | 07-conductor | roko-conductor | P1 | extensions | DP00 DP01 |
| DP08 | 08-chain | roko-chain | P2 | phase2 | DP00 |
| DP09 | 09-daimon | roko-daimon | P2 | phase2 | DP00 |
| DP10 | 10-dreams | roko-dreams | P2 | phase2 | DP00 |
| DP11 | 11-safety | roko-agent | P0 | safety-iface | DP00 DP02 |
| DP12 | 12-interfaces | roko-cli,roko-serve,roko-agent-server | P0 | safety-iface | DP00 DP01 DP02 |
| DP13 | 13-coordination | roko-orchestrator | P1 | infra | DP00 DP01 |
| DP14 | 14-identity-economy | roko-chain | P2 | phase2 | DP00 DP08 |
| DP15 | 15-code-intelligence | roko-index,roko-mcp-code,roko-lang-rust,roko-lang-typescript,roko-lang-go | P1 | infra | DP00 |
| DP16 | 16-heartbeat | roko-runtime | P1 | infra | DP00 DP01 |
| DP17 | 17-lifecycle | roko-agent,roko-runtime | P1 | infra | DP00 DP02 |
| DP18 | 18-tools | roko-std,roko-agent | P1 | infra | DP00 DP02 |
| DP19 | 19-deployment | roko-cli | P1 | infra | DP00 DP12 |
| DP20 | 20-technical-analysis | cross-cutting | P2 | phase2 | DP00 DP04 DP05 |

## Groups

- **core** (P0): DP00-DP05 — Architecture, orchestration, agents, composition, verification, learning
- **extensions** (P1): DP06-DP07 — Neuro, conductor
- **safety-iface** (P0): DP11-DP12 — Safety, interfaces
- **infra** (P1): DP13, DP15-DP19 — Coordination, code-intel, heartbeat, lifecycle, tools, deployment
- **phase2** (P2): DP08-DP10, DP14, DP20 — Chain, daimon, dreams, identity-economy, technical-analysis

## Execution Order

1. Core foundation (DP00-DP05)
2. Safety + interfaces (DP11-DP12)
3. Extensions + infra (DP06-DP07, DP13, DP15-DP19)
4. Phase 2+ stubs (DP08-DP10, DP14, DP20)
