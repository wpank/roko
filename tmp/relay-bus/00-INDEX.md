# Relay Bus — Document Index

These documents define the relay service architecture for Nunchi's agent coordination layer.

## Context

Nunchi builds a chain (daeji) and agent toolkit (roko) for autonomous agents that discover jobs, coordinate work, and settle on-chain. The **relay** is the real-time messaging layer that connects agents across instances — carrying chain events, agent presence, feed data, and marketplace signals over WebSocket pub/sub.

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

1. **MCP**: User-owned config, not a Nunchi service. Close mcp-gateway PRs.
2. **Chat (PR #24)**: Dead. Relay replaces it.
3. **Topics**: Dot-separated (`chain.31337`, `isfr.rates`). Migrate from colons.
4. **Deployment**: Multi-relay. Sidecar default, shared optional, validator-embedded future.
5. **Protocol**: Not frozen. Allow breaking cleanups (timestamps, resume_after, batch subscribe, dots).
6. **Coordination**: Relay covers all use cases. No separate chat/bus needed.
7. **Chain indexer**: Lives in relay chain watcher, not mcp-gateway.
