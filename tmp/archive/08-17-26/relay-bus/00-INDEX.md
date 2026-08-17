# Relay Bus — Document Index

> **What is this?** The relay bus is roko's event distribution system. It has two
> buses: an in-process `EventBus` (`roko-runtime`) for local pub/sub within a single
> roko process, and a WebSocket relay (`apps/agent-relay`, proxied via
> `roko-agent-server`) for cross-instance agent connectivity. These docs capture
> design decisions made in May 2026 and migration plans that resulted from auditing
> two collaboration PRs (#156, #158) against the actual implementation.
>
> **Status (2026-08-13):** The 9 decisions in `05-decisions.md` are settled and
> authoritative. The colon-to-dot topic migration (decision #3) is decided but
> **not yet implemented** -- the codebase still uses colon-separated topics
> (`isfr:rates`, `chain:{id}`, etc.). The `ISFRFeed.map_topic()` shim in
> `roko-core/src/isfr_feed.rs` still converts colons to dots at the boundary.
>
> Last updated: 2026-08-13

These documents define the relay service architecture for Nunchi's agent coordination layer.

## Context

Nunchi builds a chain (daeji) and agent toolkit (roko) for autonomous agents that discover jobs, coordinate work, and settle on-chain. The **relay** is the real-time messaging layer that connects agents across instances -- carrying chain events, agent presence, feed data, and marketplace signals over WebSocket pub/sub.

Two PRs in the collaboration repo proposed designs that didn't align with the existing implementation:
- PR #156 proposed a Nunchi-hosted MCP gateway as default agent infrastructure
- PR #158 proposed a new bus envelope schema that doesn't match the built relay

These documents correct the record, spec the actual relay, and make decisions.

## Documents

| # | Document | What It Covers |
|---|---|---|
| [00](00-INDEX.md) | This index | Navigation |
| [01](01-relay-service-spec.md) | **Relay Service Spec** | Architecture, deployment models (sidecar/shared/multi-relay), wire protocol, topic namespace, what the relay does and doesn't do, current implementation, gaps |
| [02](02-validator-embedded-relay.md) | **Validator-Embedded Relay** | Two embedding modes (minimal chain projector vs full supervised task), library+binary design, validator incentives, comparison with PR #24's chat approach |
| [03](03-coordination-use-cases.md) | **Coordination Use Cases** | All 42 use cases audited across Nunchi repos, classified by pattern (pub/sub, on-chain, request/response, chat-replacement, other). Conclusion: relay covers everything. |
| [04](04-topic-grammar.md) | **Topic Grammar** | Decision: dots not colons. Industry survey (NATS, MQTT, Kafka, RabbitMQ, Redis, etc.), URL safety, wildcard readiness, migration plan |
| [05](05-decisions.md) | **Decisions** | Nine settled decisions: MCP gateway closed, chat PR dead, dots, multi-relay deployment, protocol not frozen yet, PR #158/#156 dispositions, chain indexer location, relay sufficiency |

## Pre-existing Documents

| Document | What It Covers |
|---|---|
| [pr-156-158-assessment.md](pr-156-158-assessment.md) | Original detailed assessment of PRs #156 and #158 against the actual relay implementation |
| [demo-ide-issue-4-mcp-redesign.md](demo-ide-issue-4-mcp-redesign.md) | MCP gateway redesign notes from demo-ide issue #4 |

## Key Decisions Summary

All 9 decisions below are **settled**. Implementation status is noted in parentheses.

1. **MCP**: User-owned config, not a Nunchi service. Close mcp-gateway PRs. (Settled; roko uses `.mcp.json` / `agent.mcp_config`)
2. **Chat (PR #24)**: Dead. Relay replaces it. (Settled; PR not merged)
3. **Topics**: Dot-separated (`chain.31337`, `isfr.rates`). Migrate from colons. (Decided but **not yet migrated** -- colons still in codebase)
4. **Deployment**: Multi-relay. Sidecar default, shared optional, validator-embedded future. (Settled; sidecar is current default)
5. **Protocol**: Not frozen. Allow breaking cleanups (timestamps, resume_after, batch subscribe, dots). (Settled; protocol still unfrozen)
6. **PR #158**: Rewrite to align with actual relay. (Settled)
7. **PR #156**: Strip MCP gateway, keep workspace surface. (Settled)
8. **Chain indexer**: Lives in relay chain watcher, not mcp-gateway. (Settled; chain watcher exists but only publishes `new_block`)
9. **Coordination**: Relay covers all 42 use cases. No separate chat/bus needed. (Settled)
