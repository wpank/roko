# Agent Coordination Use Cases

## Summary

After auditing all PRs across Nunchi-trade repos (collaboration #134-161, daeji #21-42, demo-ide, desktop-app, agent-chat, mcp-gateway, nunchi-cli, contracts-core), here is every agent coordination use case and what handles it.

**Bottom line:** The relay (pub/sub + request/response) covers everything agents actually need. There are no coordination use cases that require a separate chat/messaging layer.

## Classification

### Pub/Sub (Relay Handles It) — 14 Use Cases

These are all broadcast/fanout patterns. An agent publishes to a topic; all subscribers receive it. The relay's TopicBus is the right primitive.

| # | Use Case | Source | Topic |
|---|---|---|---|
| 1 | Agent advertisement/heartbeat | PR #158 | `agent.presence` |
| 2 | Job posted event broadcast | PR #148, #155 | `chain.{id}` (contract event) |
| 3 | Job discovery/assessment | daeji #28 | `chain.{id}` + local compute |
| 4 | Anti-knowledge propagation | PR #152, #160 | `feed.antiknowledge` |
| 5 | StatusCard verification events | daeji #24 | `chain.{id}` (AgentRegistered event) |
| 6 | Hire-an-agent broadcast | PR #155 | `job.posted` |
| 7 | Workspace global discovery | PR #156 | `agent.presence` |
| 8 | MCP tool-call observability | PR #158 | Not needed (see MCP section) |
| 9 | Chain event relay (lifecycle) | daeji #24 | `chain.{id}` |
| 10 | Lobby job announcements | daeji #24 | `job.posted` |
| 11 | Lobby room-joined notifications | daeji #24 | `job.{id}.status` |
| 12 | Lobby mining claims | daeji #24 | `job.{id}.claim` |
| 13 | Config sync (IDE↔Railway) | PR #156 | Not relay — this is IDE SSE |
| 14 | ISFR rate publication | daeji #21 | `isfr.rates` |

### On-Chain (Neither Relay nor Chat) — 10 Use Cases

These are contract interactions. Agents submit transactions independently; the chain mediates.

| # | Use Case | Source | Mechanism |
|---|---|---|---|
| 1 | Agent registration (ERC-8004) | daeji #27 | `AgentRegistry.register()` |
| 2 | Job bidding (4 selection modes) | daeji #31 | `MultiAgentMarket.bid()` |
| 3 | Winner selection | daeji #31, contracts-core | Contract logic |
| 4 | Settlement (`submitMulti`) | daeji #26 | Contract tx |
| 5 | ISFR multi-keeper quorum voting | PR #157 | `ISFROracle.submitRateForRange()` |
| 6 | Stigmergic knowledge (InsightBoard) | PR #152 | `InsightBoard.postGuarded()` |
| 7 | Pheromone decay/tier promotion | PR #152 | Contract state |
| 8 | Prediction commit-reveal | PR #152, #157 | Contract protocol |
| 9 | Reputation/trust scoring | PR #153 | `WorkerRegistry` query |
| 10 | Creator revenue routing | PR #151 | Contract logic |

### Request/Response (Relay Handles It) — 5 Use Cases

Direct agent-to-agent or service-to-agent RPC. The relay's `POST /relay/messages` with timeout handles this.

| # | Use Case | Source | Mechanism |
|---|---|---|---|
| 1 | Direct agent messaging | PR #158 | `POST /relay/messages` |
| 2 | MCP tool call proxy | PR #156 | Not relay — agent runtime |
| 3 | agentctl JSON-RPC daemon | daeji #33 | Local IPC, not relay |
| 4 | agentctl bridge tools | desktop-app #5 | Local IPC, not relay |
| 5 | Capability-based discovery | daeji #28 | `GET /relay/agents` + chain query |

### Previously "Chat" — Now Relay Topics

These were the use cases that PR #24 solved with encrypted commonware-p2p rooms. All of them work over relay topics instead:

| # | Use Case | PR #24 Mechanism | Relay Mechanism |
|---|---|---|---|
| 1 | Room formation | AEAD key exchange via lobby | Agent subscribes to `job.{id}` topic |
| 2 | In-room coordination (PartialResult) | `RoomMessage::PartialResult` | Publish to `job.{id}` with `msg_type: "partial_result"` |
| 3 | In-room consensus (Vote) | `RoomMessage::Vote` | Publish to `job.{id}` with `msg_type: "vote"` |
| 4 | Contribution score declaration | `RoomMessage::Final` with `contribution_score_bps` | Publish to `job.{id}` with `msg_type: "final"` |
| 5 | ISFR symphony 4-agent room | 4 agents in encrypted room | 4 agents subscribe to `isfr.symphony.{job_id}` |
| 6 | Autoresearch execution | Same as symphony | Same pattern on `research.{job_id}` |

**Why encryption doesn't matter here:** The user confirmed confidential rooms aren't useful. Agent rate submissions, partial results, and votes are public data — they're submitted on-chain anyway. If confidentiality is ever needed (sealed-bid auctions), agents can encrypt payloads application-side before publishing to the relay topic. The relay doesn't need to know about encryption.

### Genuinely Different — 4 Use Cases

These don't fit pub/sub, request/response, or on-chain patterns:

| # | Use Case | Source | Why Different | Resolution |
|---|---|---|---|---|
| 1 | MEV race execution | agent-chat PR #2 | Latency-critical competitive execution | Deferred (Phase eta.4). If needed, low-latency transport is a separate concern from the relay. |
| 2 | DKG threshold signing | PR #138, #132 | Cryptographic protocol at consensus layer | Consensus-layer primitive, not agent messaging |
| 3 | Encryption envelope for sensitive jobs | PR #155 | Key exchange/wrapping | Application-level, not transport |
| 4 | External agent pairing | desktop-app #6 | Auth/pairing protocol | IDE-specific, not relay |

None of these require a "chat" system.

## What About Symphony Coordination?

The symphony protocol (PR #148, #158, daeji #26) is the most complex coordination pattern. Here's how it works over the relay:

```
1. Job posted on chain → chain watcher publishes to chain.{id}
2. Agents discover job → subscribe to chain.{id}, filter for job_posted
3. Agents bid on chain → MultiAgentMarket.bid()
4. Winners selected on chain → chain watcher publishes job_awarded
5. Winners subscribe to job.{job_id} topic
6. Each agent publishes partial results:
   { "topic": "job.{job_id}", "msg_type": "partial_result",
     "payload": { "class": "lending", "rate_bps": 420 } }
7. Each agent computes deterministic aggregate from all partials
8. Lowest-address agent publishes final:
   { "topic": "job.{job_id}", "msg_type": "final",
     "payload": { "composite_bps": 620, "contribution_scores": [...] } }
9. Submitter calls submitMultiWithScores on chain
```

This is exactly what PR #24's chat rooms did, minus the encryption. The relay is a more natural fit because:
- No 64-slot ceiling
- Any language (Python keepers work too)
- NAT-friendly
- Topics are dynamic (no pre-registration)
- Replay on subscribe (late joiners see partial results)

## What PR #158 Gets Wrong About Coordination

PR #158 says "chat NEVER reads from agent bus." This rule exists because PR #158 assumes chat and bus are separate systems with separate auth. If the relay is the only coordination layer, this rule is meaningless — there is no separate "chat" to protect.

PR #158 also invents a two-bus architecture (chat + agent bus) with 8 surface contracts (C1-C8) governing the boundary between them. With a single relay, these contracts simplify to:
- C1 (chain → bus): Chain watcher publishes contract events. Already implemented.
- C2 (bus envelope): The relay's wire format. Already implemented (with gaps noted in 01-relay-service-spec.md).
- C3-C6 (chat contracts): Not needed. Symphony coordination runs over relay topics.
- C7-C8 (workspace/IDE contracts): IDE-specific, not relay concerns.

## MCP Is Not a Coordination Pattern

PR #156 and #158 both reference MCP gateway topics (`mcp.tool_call.<workspace_id>`) as a relay concern. This is wrong:

- MCP is agent-runtime configuration: `.mcp.json`, `agent.mcp_config`, env vars.
- Users run their own MCP servers alongside their agents (stdio in the same container, or HTTP to external endpoints).
- Nunchi is not building a managed MCP product.
- The "Railway agents can't use local MCP servers" problem is solved by running MCP servers on Railway (same container/project), not by building a centralized gateway.
- MCP tool-call telemetry, if desired, can be an agent-level concern (agent logs its own tool calls), not a relay topic.

## Conclusion

The relay covers all coordination use cases. PR #24 (chat) should not be merged — the relay replaces it entirely. The MCP gateway PRs (#1-3 in mcp-gateway repo) should be closed or reframed as optional managed infrastructure, not default agent architecture.
