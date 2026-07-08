# 01 — Architecture

## Framing

There are two separate concerns here and the earlier drafts kept mixing them:

1. **Protocol architecture** — where agent identity, discovery metadata, and
   reachability should live.
2. **Default deployment shape** — the easiest way to run and verify the system
   locally and on Railway.

The protocol answer is:

- durable identity belongs in ERC-8004, not in mirage's Rust `HashMap`
- live reachability belongs in a relay and/or an agent's own public endpoint
- agents without wallets are a first-class production case

The default deployment answer is:

- `mirage-rs` and `agent-relay` run together by default
- they share one domain by default through a `/relay/*` path prefix
- this is the standard local-demo and Railway-demo shape unless we later need
  to split them for scale

## The actual problem

The Kauri dashboard repo (`nunchi-dashboard`, separate repo) currently routes
messages to per-agent HTTP servers via `POST /message`. Today it discovers
agent URLs from:

- `localStorage` endpoint cache in `src/components/ai-studio/AskPanel.tsx`
- active deployment responses in `src/services/mirage-api.ts`
- manual entry

That is enough for ad hoc demos, but it breaks as a durable discovery model for:

- agents running on user infrastructure
- agents behind NAT
- agents started from a laptop or CI runner
- agents that intentionally have no wallet
- cross-session reuse on a different machine or browser

Issue #15 proposes adding an `endpoint` field to
`apps/mirage-rs/src/chain/agent.rs::AgentEntry`. That would improve one UI path
while still leaving identity tied to a mirage-specific Rust registry. That is
the wrong abstraction boundary.

## Core protocol architecture

Four pieces are on the critical path.

### 1. `mirage-rs`

`mirage-rs` is a chain simulator and JSON-RPC surface. In this design it is not
the durable registry of agent identity. It either:

- forks an upstream chain that already has ERC-8004 deployed, or
- deploys the ERC-8004 contracts itself at boot

The existing Rust `AgentRegistry` remains only as a legacy API surface during
the migration window and is then deprecated.

### 2. ERC-8004 `IdentityRegistry`

This is the durable identity layer for wallet-holding agents:

- passport ownership
- capability bitmask
- tier
- `agentCardUri`

The dashboard and the in-tree mirage demo both read this over `eth_call`.

### 3. `agent-relay`

This is a standalone relay binary with one job: make agents reachable when they
cannot or should not accept inbound traffic directly.

It is responsible for:

- outbound WS connections from agents
- live presence
- message forwarding
- hosting relay-backed Agent Cards

It is not the chain, not the orchestrator, and not the control plane.

### 4. `roko-agent-server`

Each reachable agent exposes its own HTTP/WS surface. The missing production
piece today is a real CLI entrypoint that starts it:

- `roko agent serve`

Agents can then participate in either or both paths:

- **direct path**: public URL + wallet + 8004 registration
- **relay path**: outbound WS to relay, with or without wallet

## Default deployment shape

The architecture above does not require the relay to live inside mirage, but
the default runtime shape does:

- local demo: `mirage-rs + agent-relay` on one machine
- Railway demo: `mirage-rs + agent-relay` in one service
- URL convention: `{CHAIN_URL}/relay/*`

This gives us:

- one default domain
- one default env var for the dashboard
- one obvious place for local and Railway verification

If scale or operational concerns later require it, `agent-relay` can move to
its own service without changing the protocol model.

## Discovery and transport rules

These are the real invariants the rest of the docs build on.

### Durable identity

- wallet-holding agents publish identity through ERC-8004
- `agentCardUri` is the durable pointer to off-chain service metadata

### Live presence

- relay presence is authoritative for agents currently connected to the relay
- direct HTTP reachability is authoritative for public agents that do not need
  relay transport

### Message transport

- prefer direct HTTP when `AgentCard.endpoints.rest` exists and is reachable
- use relay transport when the agent is wallet-free, private, NAT'd, or only
  advertises relay reachability

### Merge model

Both the Kauri dashboard and the in-tree mirage static UI should build the same
merged list:

`on-chain identities ∪ relay-connected agents`

deduped by `agent_id`.

## Agent classes

The important split is not local vs deployed. It is wallet-present vs
wallet-absent.

| Agent class | 8004 identity | Relay | Direct HTTP | Notes |
|---|---|---|---|---|
| Public deployed agent with wallet | Yes | Optional | Yes | Primary direct-connect path |
| NAT'd deployed agent with wallet | Yes | Yes | Optional | Relay may host the card URI |
| Laptop / CI / SDK agent with no wallet | No | Yes | Optional | First-class production path |
| Laptop / CI / SDK agent with session key | Optional | Yes | Optional | Phase-2 auth can use signed hello |

## Default topology

```
             ┌────────────────────────────────────────────────┐
             │ Default service: mirage-rs + agent-relay       │
             │                                                │
             │  JSON-RPC / REST      relay WS + REST          │
             │      :8545                 :3100               │
             │        │                     │                 │
             │        └────── /relay/* ─────┘                 │
             └───────────────────┬────────────────────────────┘
                                 │
                           one public domain
                                 │
                ┌────────────────┴────────────────┐
                │                                 │
                ▼                                 ▼
      Kauri dashboard / local demo         Agents
      merge 8004 + relay views             - remote public agent
                                           - remote relay-only agent
                                           - local laptop agent
```

## Dashboard load flow

1. Read `registeredCount()` and `registeredAt(i)` from `IdentityRegistry`
2. Read each `passports(passportId)`
3. Filter to `CAP_ROKO`
4. Fetch each `agentCardUri`
5. Read `GET /relay/agents`
6. Merge the two sets by `agent_id`
7. Route messages:
   - direct `POST {endpoints.rest}/message` when possible
   - relay `POST /relay/agents/{id}/message` otherwise

The same logic should exist in:

- Kauri dashboard (`nunchi-dashboard`, separate repo)
- mirage static demo (`apps/mirage-rs/static/`)

## Why this is better than Issue #15

Issue #15 says: add `endpoint` to mirage's `AgentEntry`.

That solves one symptom, but it keeps all of the important state in the wrong
place:

- identity remains mirage-specific instead of chain-shaped
- wallet-free agents still need an ad hoc story
- NAT traversal still needs another mechanism
- production discovery still depends on a simulator's in-process state

The replacement model fixes the problem at the abstraction boundary where it
actually belongs:

- chain for durable identity
- relay for reachability
- agent card for transport metadata

## Why the default is still mirage + relay together

Even though the relay is a separate concept, the default deployment shape should
still keep it next to mirage because it gives us the simplest operational story:

- one Railway service for the chain demo path
- one default local startup path
- one domain for the dashboard
- one obvious verification surface

That is a deployment preference, not a protocol requirement.

## Explicitly out of scope

- adding `endpoint` to mirage's `AgentEntry`
- keeping `roko-serve` on the production hot path
- forcing every agent to have a wallet
- making the relay the hot path for every message
- finalizing 1-click deploy UX in the dashboard
- relay auth beyond the MVP contract described in [02-relay-design.md](02-relay-design.md)
