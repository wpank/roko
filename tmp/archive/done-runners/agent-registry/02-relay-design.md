# 02 — Relay Design

## Purpose

`agent-relay` is the live reachability layer for agents that are not reached by
direct inbound HTTP.

That includes:

- wallet-free agents
- NAT'd agents
- laptop / CI / SDK agents
- public agents that still want relay fallback

Its job is deliberately narrow:

- maintain outbound agent connections
- expose current presence
- forward messages
- host relay-backed Agent Cards

## Packaging and deployment

### Binary shape

- new standalone Rust binary at `apps/agent-relay/`
- no dependency on `roko-serve`
- no dependency on mirage internals beyond optional co-deployment

### Default deployment mode

The default runtime shape is:

- `agent-relay` co-deployed with `mirage-rs`
- same Railway service by default
- same local-dev stack by default
- shared public origin by default through `/relay/*`

### Alternative deployment mode

If needed later, `agent-relay` can run as its own service with a separate
domain. Nothing in the protocol design depends on same-process deployment.

## Responsibilities

1. Accept outbound WS agent connections
2. Track currently-connected agents
3. Expose `GET /relay/agents`
4. Host relay-backed Agent Cards at `GET /relay/cards/{id}`
5. Forward request/response and streaming traffic
6. Broadcast connect/disconnect events to subscribers

## Non-responsibilities

The relay does **not** do any of the following:

- chain reads or writes
- wallet / passport issuance
- long-term identity storage
- orchestration, plans, PRDs, or deployments
- LLM execution
- persistent queueing across restarts

## Source-of-truth boundaries

These boundaries matter more than the exact route names.

| Concern | Source of truth |
|---|---|
| Passport ownership, capability bits, durable card URI | ERC-8004 |
| Whether an agent is currently online through relay | relay |
| Direct public endpoint for a public agent | Agent Card |
| Message dispatch inside the agent | `roko-agent-server` |

## State model

Minimal in-memory state is sufficient for the MVP:

```rust
struct RelayState {
    agents: RwLock<HashMap<String, ConnectedAgent>>,
    cards: RwLock<HashMap<String, AgentCard>>,
}

struct ConnectedAgent {
    agent_id: String,
    owner: Option<String>,
    capabilities: Vec<String>,
    domains: Vec<String>,
    connected_at: u64,
    tx: mpsc::Sender<AgentMessage>,
}
```

On restart, all presence disappears and agents reconnect. That is acceptable.

## Routes

All routes are prefixed with `/relay/` so the relay can share a domain with
mirage by default.

### Directory

- `GET /relay/agents`
- `GET /relay/agents?owner=0xabc`
- `GET /relay/agents/{id}`
- `GET /relay/cards/{id}`

### Messaging

- `POST /relay/agents/{id}/message`
- `GET /relay/ws/agent`
- `GET /relay/ws/dashboard`

### Health

- `GET /relay/health`
- `GET /relay/metrics` optional

## Agent WS protocol

The transport can stay intentionally small.

### Agent hello

```json
{
  "type": "hello",
  "agent_id": "local-dev-001",
  "owner": null,
  "capabilities": ["messaging", "predictions"],
  "domains": ["roko"],
  "card": {
    "name": "local-dev-001",
    "capabilities": ["messaging", "predictions"],
    "endpoints": {
      "relay": "wss://example.com/relay/ws/dashboard?agent=local-dev-001"
    },
    "domain_tags": ["roko"],
    "version": "0.1.0"
  }
}
```

### Relay ack

```json
{
  "type": "connected",
  "agent_id": "local-dev-001",
  "card_uri": "/relay/cards/local-dev-001"
}
```

### Request / response

```json
{ "type": "message", "request_id": "req-1", "prompt": "hello" }
{ "type": "response", "request_id": "req-1", "response": "hi" }
```

### Streaming

```json
{ "type": "chunk", "request_id": "req-1", "text": "hel", "done": false }
{ "type": "chunk", "request_id": "req-1", "text": "lo", "done": true }
```

## Dashboard protocol

The dashboard should support both simple request/response and streaming.

### HTTP one-shot

`POST /relay/agents/{id}/message`

```json
{ "prompt": "What's the current ISFR?", "conversation_id": "conv-123" }
```

### Dashboard WS

```json
{ "type": "agent_connected", "agent_id": "demo-1" }
{ "type": "agent_disconnected", "agent_id": "demo-1" }
{ "type": "chunk", "agent_id": "demo-1", "request_id": "req-1", "text": "...", "done": false }
```

## Routing rules

The relay is not a universal mandatory hop. It is a selective transport.

### Use direct transport when

- the card has `endpoints.rest`
- the endpoint is public
- the caller can reach it

### Use relay transport when

- the agent is wallet-free
- the agent is only connected through relay
- the card points at relay-hosted transport
- the agent is NAT'd or otherwise private

## Agent Card hosting

Three forms are valid.

| Card URI | Host | Use when |
|---|---|---|
| `https://agent.example/.well-known/agent-card.json` | agent | public direct-connect agent |
| `https://chain.example/relay/cards/{agent_id}` | relay | relay-first or wallet-free agent |
| `data:application/json;base64,...` | on-chain inline | tiny fallback, not preferred |

For wallet-free agents, relay-hosted card URIs are the normal path, not a hack.

## Auth

### MVP

No auth. This is acceptable only for devnet, closed demos, and controlled
testing. The docs should say that plainly.

### Phase 2 target

- **agent -> relay**
  - bearer token for wallet-free agents
  - signed hello for agents with signing capability
  - both paths must remain supported
- **dashboard -> relay**
  - Privy / SIWE-backed JWT for message send
  - read-only listing may remain public

The important invariant is that auth must not reintroduce a wallet requirement
for wallet-free agents.

## URL discovery

### Agent-side priority

1. CLI flag `--relay-url`
2. env var `ROKO_RELAY_URL`
3. `roko.toml`
4. default based on local vs deployed runtime

### Dashboard-side default

Default assumption:

```ts
const CHAIN_URL = import.meta.env.VITE_CHAIN_URL ?? "http://localhost:8545";
const RELAY_URL = import.meta.env.VITE_RELAY_URL ?? `${CHAIN_URL}/relay`;
```

This should be true in both:

- Kauri dashboard (`nunchi-dashboard`)
- mirage static demo (`apps/mirage-rs/static`)

## Failure and reconnect behavior

- agent reconnects with exponential backoff
- relay drops presence immediately on disconnect
- dashboard receives connect/disconnect events
- messages sent while the agent is disconnected are lost in MVP

That behavior is fine for the current scope.

## Why not reuse `roko-serve`

`roko-serve` is bound to a workspace-shaped `AppState`, filesystem-backed
runtime concerns, and a much larger orchestration surface. The relay should be
smaller, dumber, and deployable independently.

## Why the relay stays small

The success criterion for the relay is not feature count. It is predictability:

- easy to embed in the default demo stack
- easy to reason about as presence + transport only
- easy to move into its own service later if needed
