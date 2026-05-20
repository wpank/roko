# Relay Bus PR Assessment

Date: 2026-05-08

Scope:
- `Nunchi-trade/collaboration` PR #158: agent-coordination PRD, chat vs agent bus.
- `Nunchi-trade/collaboration` PR #156: IDE MCP gateway and agent workspace spec.
- Related implementation PRs referenced from #156: `mcp-gateway` #1, #2, #3; `nunchi-cli` #10; `demo-ide` #10.
- Local Roko implementation in this repo, especially `apps/agent-relay`, `crates/roko-agent-server`, `crates/roko-serve`, `crates/roko-agent/src/mcp`, and deployment docs.
- Daeji tmp notes in `/Users/will/dev/nunchi/daeji/tmp`.

## Executive Read

PR #158 is directionally right that there is a real distinction between:

- broadcast/presence/feed transport, which is the relay-backed Bus, and
- private task coordination, which can be an application-level protocol over a topic, a direct request/response path, or a separate chat system if Daeji truly needs one.

But PR #158 gets the current state wrong. It treats Will's relay as pending and tries to freeze a new `bus.nunchi.trade/v1` envelope that does not match the service now implemented in Roko. The relay exists here as `apps/agent-relay`: topic pub/sub, per-topic ring replay, direct request/response, feed metadata, workspace registration, roko-serve same-origin proxying, feed bridges, and a chain block watcher.

PR #156 has a sharper boundary problem. MCP should be user/runtime configuration, not a required Nunchi-hosted gateway. Roko already treats MCP as agent-owned config: `.mcp.json` / `agent.mcp_config`, stdio MCP servers spawned by the agent runtime, and tool discovery merged into the normal tool registry. A hosted MCP gateway can be a useful optional managed proxy for HTTP MCPs, shared secrets, or an enterprise audit product, but it should not become the default way agents access tools or the source of truth for every user's MCP setup.

The clean model is:

- Relay/Bus: cross-instance agent connectivity, presence, feeds, topic pub/sub, request/response, chain event projection. Runs as a sidecar or shared service depending on deployment.
- MCP: local/container/user-owned tool servers configured per agent. A Nunchi gateway is optional and appears to agents as just another MCP endpoint/proxy, not as infrastructure the relay or agents fundamentally depend on.
- Chain/indexer: belongs in relay/roko-serve/a dedicated chain indexer, not inside an MCP gateway.

## What Exists in Roko Now

The old Daeji tmp note `roko-relay-current-state.md` says Roko's relay was only flat request/response. That was true for that note, but the repo has moved.

Current relay implementation:

- `apps/agent-relay/src/protocol.rs`
  - Inbound frames: `hello`, `card`, `response`, `error`, `ping`, `subscribe`, `unsubscribe`, `publish`, `register_feed`, `unregister_feed`.
  - Outbound frames: `ack`, `message`, `error`, `pong`, `topic_message`.
  - Topic messages carry `topic`, `msg_type`, `payload`, `publisher_id`, and `seq`.
- `apps/agent-relay/src/bus.rs`
  - Topic pub/sub.
  - Per-topic ring buffer, default capacity 128.
  - Global monotonic sequence number.
  - Replay on subscribe.
- `apps/agent-relay/src/lib.rs`
  - `/relay/health`
  - `/relay/agents`
  - `/relay/agents/ws`
  - `/relay/cards/{id}`
  - `/relay/messages`
  - `/relay/events/ws`
  - `/relay/workspaces`
  - `/relay/workspaces/register`
  - `/relay/feeds`
  - `/relay/topics`
  - `/relay/topics/{topic}/messages`
  - `/relay/topics/{topic}/subscribers`
- `apps/agent-relay/src/chain_watcher.rs`
  - Optional background watcher enabled with `--rpc-ws-url`.
  - Polls `eth_blockNumber` and publishes `new_block` to `chain:{chain_id}`.
  - It is not yet a full ERC-8004/ERC-8183 event decoder.
- `crates/roko-agent-server/src/features/relay_client.rs`
  - Agents connect outbound to the relay, publish cards, receive direct messages, and can subscribe/publish topics.
- `crates/roko-agent-server/src/features/relay_subscriber.rs`
  - Higher-level pub/sub wrapper.
  - `ISFRTopicAdapter` maps relay topic messages into local `ISFRFeed`.
- `crates/roko-core/src/isfr_feed.rs`
  - Relay-to-local-Bus bridge for ISFR topics.
- `crates/roko-serve/src/lib.rs`
  - Starts workspace relay registration.
  - Starts ISFR relay bridge.
  - Starts feed agents and a feed relay bridge.
- `crates/roko-serve/src/routes/relay_proxy.rs`
  - Proxies `/relay/*` through roko-serve so the sidecar relay is reachable through the same public origin.
- `docker/start-railway.sh` and `docker/RAILWAY.md`
  - The default Railway deployment runs `roko serve`, `mirage-rs`, and `agent-relay` in one Railway service with `agent-relay` bound privately on `127.0.0.1:9011`.

This is already the "relay as service" shape. The thing to document is the current service boundary and the remaining deltas, not a new detached `bus.nunchi.trade/v1` contract.

## PR #158 Assessment

### What #158 Gets Right

- It recognizes that broadcast feeds/presence/job discovery should not be forced through a job-specific chat room.
- It correctly sees ISFR as a feed/pub-sub use case.
- It correctly wants an interface that consumers can rely on while implementation evolves.
- It separates economic commitments from ephemeral observation: bids and settlement belong on-chain, observations and liveness do not.

### Where #158 Gets It Wrong

1. It freezes the wrong interface.

The proposed envelope:

```json
{
  "v": 1,
  "topic": "...",
  "block_height": 123,
  "tx_hash": "0x...",
  "log_index": 4,
  "agent_id": "0x...",
  "data": {},
  "ts_unix_ms": 1746543210123
}
```

does not match the implemented relay. The implemented outbound topic frame is:

```json
{
  "type": "topic_message",
  "topic": "...",
  "msg_type": "...",
  "payload": {},
  "publisher_id": "agent-id",
  "seq": 1
}
```

Internally `TopicEnvelope` also has `timestamp_ms`, but that timestamp is not serialized in the outbound `topic_message` frame today. If a v1 freeze is needed, freeze the actual relay protocol or make a deliberate migration PR against this repo first.

2. It assumes the relay is still pending.

The PR says current relay should not be integrated and v2 should land behind a freeze. The repo now has topic pub/sub, feed registration, workspace registration, feed bridges, and deployment wiring. The doc should say "this is what the current relay does; these are the missing gaps" instead of "wait for v2."

3. It over-prescribes "chat" as a separate plane.

The Roko/Daeji relay redesign notes repeatedly frame the relay as payload-opaque. It can carry job/group coordination messages without knowing whether the payload is a symphony partial, a vote, an ISFR range proposal, or a status update. The PR's rule that "chat NEVER reads from agent bus" may be true for one Daeji commonware-chat design, but it is too strong as a Roko system contract.

Better framing:

- Relay does not enforce coordination semantics.
- Private chat, job groups, and direct request/response are application protocols.
- Some application protocols may use relay topics; others may use a separate AEAD mesh.
- The relay should not know or care.

4. It invents topic names that conflict with the current code and docs.

PR #158 uses dot topics like `job.open.<cap>`, `isfr.<jobType>.aggregate`, `caps.<bit>`, and `mcp.tool_call.<workspace_id>`.

The implemented and local design direction uses colon topics such as:

- `isfr:rates`
- `isfr:epochs`
- `chain:{chain_id}`
- `feed:meta:relay`
- `feed:{domain}:{name}`

Before any "frozen" topic grammar, we need decide whether colon topics are the canonical relay grammar. The current implementation and Roko docs already lean that way.

5. It couples relay auth to the MCP gateway.

PR #158 says bus JWT is shared with the MCP gateway auth path. That is not in the Roko relay, and it is a questionable boundary. Relay authorization should be based on relay identity, agent identity, workspace membership, or chain registry membership. MCP gateway tokens are tool-call credentials. Reusing them makes the relay depend on a service that should be optional.

6. It specifies HTTP long-poll and time retention that do not exist.

Current relay has:

- ring replay on subscribe,
- HTTP inspection of recent topic messages,
- no `GET /v1/poll?topic=...&since=...`,
- no per-topic time retention,
- no `since_ts`.

These may be good future work, but they are not a v1 freeze.

7. It puts MCP tool-call observability in the core topic namespace.

`mcp.tool_call.<workspace_id>` assumes the Nunchi hosted gateway is canonical. If tool-call telemetry is useful, it should be a generic telemetry/feed event emitted by whichever runtime owns the call. It should not make the Bus depend on a global MCP gateway.

8. It names a global hosted service as the default surface.

The local deployment model is sidecar/same-origin:

- Railway service runs its own loopback `agent-relay`.
- `roko-serve` proxies `/relay/*`.
- Mirage can front relay on the same origin.

A global `bus.nunchi.trade` may be a managed option, but it should not be the only contract. Users and agents should be able to run their own relay with their Roko/Railway instance.

### Recommended Rewrite For #158

Replace the "C2 frozen envelope" with a "Relay v1 implemented contract":

Client to relay:

```json
{ "type": "hello", "agent_id": "agent-1", "name": "Agent", "capabilities": ["isfr"] }
{ "type": "subscribe", "topic": "isfr:rates" }
{ "type": "publish", "topic": "isfr:rates", "msg_type": "composite_rate", "payload": { "bps": 620 } }
{ "type": "register_feed", "feed": { "feed_id": "meta-relay", "topic": "feed:meta:relay", "name": "Relay Stats" } }
```

Relay to client:

```json
{ "type": "ack", "event": "subscribed:isfr:rates" }
{ "type": "topic_message", "topic": "isfr:rates", "msg_type": "composite_rate", "payload": {}, "publisher_id": "agent-1", "seq": 42 }
{ "type": "message", "message_id": "...", "message": { "prompt": "..." } }
```

Then list known gaps separately:

- outbound `topic_message` should include `timestamp_ms` if consumers need it;
- subscribe should probably support `topics: []` and `resume_after`;
- chain watcher should decode contract logs, not only block numbers;
- auth and topic ACLs are not implemented;
- per-topic backpressure policies are not implemented;
- HTTP poll is not implemented;
- topic grammar needs a decision before being called frozen.

## PR #156 Assessment

### What #156 Gets Right After Sam's Review

Sam's review improved the spec materially:

- hosted gateway must not replace local bridge,
- local-only vs hosted vs mirrored must be explicit,
- hosted gateway cannot magically invoke user-local stdio MCPs,
- subscription workspaces and marketplace/hire workspaces are different,
- humans should drive workspaces through typed app/tile actions, not unmanaged direct task chat.

Those corrections are good. The remaining problem is more fundamental: the hosted gateway is still treated as the default center of gravity.

### Where #156 Gets It Wrong

1. MCP is an agent/runtime concern, not an agent coordination substrate.

Roko's docs and code already model MCP as dynamic tool configuration:

- `.mcp.json` discovery from the working directory or `$HOME`,
- `agent.mcp_config` in config,
- stdio MCP servers spawned by the agent runtime,
- tools merged into the normal registry,
- safety hooks applied uniformly to built-in and MCP tools.

That is the right default. A user should specify which MCP servers their agents can use. If the agent runs on Railway, those MCP servers should run in that Railway container/project when they are stdio/container-local servers, or be configured as direct HTTP MCP endpoints when remote.

2. A Nunchi-hosted MCP gateway centralizes secrets and tool traffic without being necessary.

The gateway would see tool names, redacted or unredacted args depending on implementation, latency, status, and secret injection boundaries. That might be acceptable as an optional managed product. It should not become required for self-hosted agents or user-owned MCP servers.

3. "Account/server config as source of truth" is not generally true.

For Nunchi's hosted UX, account config can be a convenience. For Roko as a runtime, the source of truth is the deployment's config and environment:

- `.mcp.json`,
- `agent.mcp_config`,
- `ROKO_MCP_CONFIG`,
- container env vars,
- Railway service variables,
- user-managed sidecars.

The IDE can generate and deploy those configs. It should not necessarily own them centrally.

4. Hosted stdio support is mostly the same as "run the MCP next to the agent."

If a stdio MCP server can run in a hosted runner, it can usually run as a sidecar/process in the same Railway deployment as the agent. That is simpler than a global gateway that brokers stdio processes. The gateway might still be useful for shared HTTP MCPs or managed Nunchi-built tools, but not as a general stdio solution.

5. `mcp-gateway` PR #3 puts chain marketplace indexing into the wrong service.

The `mcp-gateway` stack added:

- config store,
- token mint/revoke,
- MCP proxy,
- audit log,
- ERC-8004/marketplace event read model,
- jobs,
- work proofs,
- reputation,
- workspace close release projections.

That mixes tool access with marketplace indexing. The indexer belongs in a chain service, relay chain watcher, roko-serve read model, or a dedicated indexer. It does not belong in an MCP gateway.

6. The gateway should not be a relay dependency.

PR #158 then uses PR #156's gateway token model for bus auth and `mcp.tool_call` bus topics. That is the wrong direction. Relay and MCP gateway should be independent services. One may emit telemetry to the other, but neither should be foundational for the other.

### Recommended Rewrite For #156

Split the spec into three separate surfaces:

1. User MCP runtime config

Canonical default:

- user provides `.mcp.json` / `agent.mcp_config`,
- stdio MCPs run in the same runtime/container as the agent,
- HTTP MCPs are called directly by the agent runtime,
- secrets come from env/vault owned by that deployment,
- IDE may help generate, validate, and deploy this config.

2. Optional Nunchi managed MCP proxy

Optional product/service:

- only enabled when `NUNCHI_MCP_GATEWAY_URL` and `NUNCHI_MCP_TOKEN` are present,
- useful for Nunchi-managed HTTP MCPs, centralized enterprise audit, or account-level secret vaulting,
- not required for Railway agents,
- not the source of truth for local/user-owned MCP servers,
- no marketplace indexer inside it.

3. Workspace/app action surface

Keep this part:

- typed app/tile actions,
- per-template and per-agent allowlists,
- visible tile activity,
- audit records.

But keep it separate from MCP gateway hosting. App/tile actions are an IDE/workspace API, not necessarily an MCP gateway API.

## Specific PR Feedback

### Comment Shape For PR #158

Suggested stance:

> This PR has the right high-level distinction between relay/broadcast and private coordination, but it should not freeze the proposed C2 envelope. The Roko relay is already implemented in `apps/agent-relay` with a different v1 protocol: `hello`, `subscribe`, `publish`, `topic_message`, feed registration, request/response, and workspace registration under `/relay/*`. Please rewrite this as an alignment doc against the implemented relay, list the remaining gaps, and avoid coupling relay auth/topics to the MCP gateway. A global `bus.nunchi.trade` may be a managed deployment option, but the canonical relay service must also run as the user's own sidecar/same-origin service on Railway/Mirage.

### Comment Shape For PR #156

Suggested stance:

> The local/hosted/mirrored corrections are good, but the hosted MCP gateway should be optional, not the default agent tool architecture. Users own MCP configuration; agents should load `.mcp.json` / `agent.mcp_config` and run stdio MCPs in the same local or Railway runtime they run in. A Nunchi gateway can be a managed HTTP MCP proxy or audit/vault product when explicitly enabled, but it should not be the source of truth for all MCP servers, and the marketplace/chain indexer should not live in `mcp-gateway`.

## Decisions To Make

1. Should `apps/agent-relay`'s current protocol be frozen as v1, or are small breaking cleanups still allowed now?

Examples of small cleanups:

- include `timestamp_ms` in outbound `topic_message`;
- support `subscribe` with `topics: []`;
- support `resume_after` instead of replaying the whole ring on subscribe;
- rename `msg_type` vs `type`;
- decide colon topics vs dot topics.

2. Is the canonical relay deployment:

- per-user/per-workspace sidecar, same-origin through `roko-serve`,
- a Nunchi-hosted multi-tenant relay,
- or both?

If both, which one do docs call default, and how do topic isolation and auth differ?

3. Does Daeji still need a separate commonware-chat plane for private rooms, or should job/workspace coordination be an application protocol over relay group topics?

This is the biggest conceptual split between #158 and the local relay docs.

4. Should the relay chain watcher become the chain event pumper?

The current watcher only publishes `new_block`. The next obvious step is decoding ERC-8004/ERC-8183/ISFR logs and publishing them on `chain:{chain_id}` or `feed:*` topics. I do not see a reason to introduce a separate pumper unless restart isolation is the main requirement.

5. What is the first topic grammar we want to support publicly?

Current code/docs imply:

- `chain:{chain_id}`
- `isfr:rates`
- `isfr:epochs`
- `feed:{domain}:{name}`
- `group:{id}:...`
- `agent:{id}:...`

PR #158 implies:

- `agent.*`
- `job.*`
- `isfr.*`
- `caps.*`
- `mcp.tool_call.*`

We should pick one before comments call anything frozen.

6. Is Nunchi trying to offer a managed MCP product?

If yes, name it as such and keep it optional. If no, close or reframe `mcp-gateway` as a temporary demo shim.

7. Where should marketplace/indexer state live?

Candidates:

- `roko-serve` read model,
- relay chain watcher plus HTTP projections,
- dedicated chain indexer,
- not `mcp-gateway`.

8. Should `nunchi-cli` gateway mode stay as an opt-in compatibility path?

The PR #10 shape is acceptable if it only activates with `NUNCHI_MCP_GATEWAY_URL` and `NUNCHI_MCP_TOKEN`, and if normal MCP config remains first-class.

## Bottom Line

Do not make PR #158 or #156 canonical as written.

For #158: keep the relay-vs-private-coordination insight, but align the spec to the implemented Roko relay service and avoid inventing a new bus API.

For #156: keep typed workspace/app actions, but demote hosted MCP gateway to an optional managed proxy. The default is user-owned MCP config and MCP servers running with the user's agent runtime.
