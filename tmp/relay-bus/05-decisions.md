# Decisions

Settled answers to the open questions from the original assessment.

## 1. MCP Gateway: Closed

**Decision:** Nunchi is not building a managed MCP product.

PR #156's hosted MCP gateway was trying to solve "Railway agents can't use local MCP servers." The correct answer is: MCP servers run alongside agents. If the agent is on Railway, MCP servers run in the same Railway container/project. MCP is agent-runtime config (`.mcp.json`, `agent.mcp_config`, env vars), not a Nunchi service.

**Action items:**
- Close or archive `mcp-gateway` PRs #1-3 (config scaffold, token/proxy, indexer)
- Reframe PR #156 to remove hosted gateway as default architecture
- Keep the workspace/app action surface from PR #156 (tiles, typed actions, audit) — that part is good
- `nunchi-cli` PR #10 (gateway mode) should be opt-in only, not default

**What stays:**
- `.mcp.json` / `agent.mcp_config` as source of truth
- Stdio MCP servers spawned by agent runtime
- HTTP MCP servers called directly by agent runtime
- IDE can help generate and deploy MCP config
- If someone later wants a managed MCP proxy product, it's opt-in via `NUNCHI_MCP_GATEWAY_URL`

## 2. Chat (daeji PR #24): Dead

**Decision:** Do not merge PR #24. The relay replaces it.

The commonware-p2p chat layer uses the wrong transport for agent coordination. The relay provides everything chat does, plus language-agnostic access, NAT-friendliness, dynamic topics, and reconnection.

**What to keep from PR #24:**
- ChaCha20Poly1305 AEAD primitives (if encrypted rooms are ever needed, application-level)
- Room key derivation (`keccak256("DAEJI_ROOM_V1" || jobId)`)
- Chain event watching (alloy patterns, adapt for relay chain watcher)

**What to drop:**
- commonware-p2p mesh transport
- 64-slot pre-allocated pool
- Typed message enum (Hello/Status/PartialResult/Vote/Final) — these become `msg_type` strings in relay topic messages
- File-based registry
- kora embedding (relay replaces this)
- Lobby/room dual channel model

**PRs to close:** daeji #24 (consolidated chat), #11, #13, #14, #17, #19 (individual chat PRs), #36 (vendored upstream commonware-chat)

## 3. Topic Grammar: Dots

**Decision:** Use dot-separated topics. Migrate from colons.

See [04-topic-grammar.md](04-topic-grammar.md) for full rationale.

## 4. Relay Deployment: Multi-Relay

**Decision:** Agents connect to 1-3 relays. Sidecar is default. Shared relay is optional.

- **Sidecar** (current): `127.0.0.1:9011` in Railway service, proxied via roko-serve. Default for single-user setups.
- **Shared** (new): `relay.nunchi.trade` or community-run. For cross-user discovery and marketplace feeds.
- **Validator-embedded** (future): `kora --relay-port 9011`. For validators who want lower-latency chain events.

The relay is a library crate (`daeji-relay` or factored from `apps/agent-relay`) that can be used as a standalone binary or embedded in kora.

## 5. Relay Protocol: Not Frozen Yet

**Decision:** The relay protocol is not v1 frozen. Small breaking changes are allowed now.

Allowed cleanups before freeze:
- Include `ts` (timestamp_ms) in outbound `topic_message` ✓ (already stored, not serialized)
- Support `subscribe` with `topics: [...]` for batch subscription
- Support `resume_after: seq` for reconnection
- Migrate topic grammar from colons to dots
- Add `timestamp_ms` to outbound frames

After these land, the v1 wire format can be frozen. PR #158 should not freeze the current protocol — it should align to the actual implementation and list these gaps.

## 6. PR #158 Disposition: Rewrite

**Decision:** PR #158 should be rewritten to align with the actual relay implementation.

The PR correctly distinguishes broadcast/bus from private coordination. But it:
- Freezes the wrong envelope schema
- Assumes the relay is pending (it's built)
- Invents a two-bus architecture that isn't needed
- Uses dot topics that conflict with the (now also dot) implementation
- Couples relay auth to the MCP gateway
- Names a global hosted service as the only surface

**Rewrite should:**
- Document the actual `apps/agent-relay` protocol
- List the gaps (from 01-relay-service-spec.md)
- Describe multi-relay deployment (sidecar + shared + validator)
- Drop the chat-vs-bus distinction (relay handles both)
- Drop MCP gateway coupling

## 7. PR #156 Disposition: Strip MCP, Keep Workspace

**Decision:** PR #156 should remove the hosted MCP gateway and keep the workspace/tile/action surface.

**Keep:**
- Workspace data model (`state`, `job`, `agents`)
- Typed app/tile action surface with allowlists and audit
- Subscription vs marketplace workspace flavors
- Template metadata (`agent_slots`, `default_tiles`)

**Remove:**
- Hosted MCP gateway as default architecture
- `mcp.nunchi.trade/v1` endpoint surface
- Scoped JWT for gateway access
- SSE config sync for MCP mirroring
- `mcp.tool_call.<workspace_id>` bus topic

## 8. Chain Indexer Location: Relay Chain Watcher

**Decision:** The marketplace/chain event read model belongs in the relay's chain watcher, not in `mcp-gateway`.

The `mcp-gateway` PR #3 put ERC-8004/marketplace event ingestion, jobs, work proofs, and reputation projections inside the MCP gateway. That's the wrong service boundary. The relay chain watcher already watches the chain — it should decode these events and publish them as typed topic messages. Consumers (dashboards, agents, IDE) subscribe to the relevant topics.

## 9. Coordination: Relay Is Sufficient

**Decision:** Agents do not need a separate coordination/chat layer beyond the relay.

All 42 coordination use cases audited across Nunchi repos fall into: pub/sub (14), on-chain (10), request/response (5), relay-as-chat-replacement (6), or genuinely different (4, none requiring chat). The relay handles everything. See [03-coordination-use-cases.md](03-coordination-use-cases.md).

The four "genuinely different" use cases (MEV races, DKG, key exchange, agent pairing) are either deferred, consensus-layer, or application-level. None require a chat system.
