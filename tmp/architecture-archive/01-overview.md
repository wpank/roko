# Overview and problem

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---

## The problem

Roko has four infrastructure components that evolved independently and never agreed on boundaries:

1. **Mirage** -- a devnet chain with a relay WebSocket. Always on, shared across users.
2. **roko-serve** -- an HTTP control plane with ~85 routes. Requires a workspace directory. Optional for users who only want agents.
3. **roko-agent-server** -- a per-agent HTTP sidecar (13 routes). One process per agent. Breaks behind NAT.
4. **Dashboard / TUI** -- consumes REST endpoints from all three. Falls over when any backend is unreachable.

This creates several concrete failures:

- **Per-agent sidecars don't traverse NAT.** An agent on a Fly Machine can't expose an HTTP server that the control plane can reach without proxy configuration. The sidecar model assumes a flat network.
- **Dashboard requires roko-serve.** If the control plane is down, the dashboard shows "Backend offline" even though agents may still be running and the relay still has presence data.
- **Polling everywhere.** The dashboard polls multiple endpoints on 1-5 second intervals. This wastes bandwidth, creates visual jitter, and scales poorly with agent count.
- **API keys scattered.** Each agent holds its own LLM API keys via environment variables. No central audit, no rotation, no cost attribution.
- **No agent lifecycle.** Agents are either ephemeral CLI processes or stateless HTTP workers. No heartbeat, no mode (persistent vs reactive), no graceful shutdown protocol.
- **Three discovery sources, zero merge.** Relay presence, ERC-8004 on-chain registry, and manually-added deployment URLs each live in separate UIs. No unified agent list.

This document specifies the architecture that resolves all six.

---

## Architecture overview

```
                         ┌─────────────────────────┐
                         │   Mirage chain + Relay   │  Always on. Shared.
                         │   (mirage-devnet.fly.dev)│
                         │                          │
                         │  Chain: blocks, events,  │
                         │         ERC-8004 registry │
                         │  Relay: agent presence,  │
                         │         WS event routing  │
                         └────────┬─────────────────┘
                                  │ WebSocket
             ┌────────────────────┼─────────────────────────┐
             │                    │                          │
             ▼                    ▼                          ▼
  ┌──────────────────┐  ┌─────────────────────┐   ┌──────────────────┐
  │    Dashboard     │  │    roko process      │   │  Remote agent    │
  │   (web / TUI)    │  │   (optional)         │   │  (Fly / Railway) │
  │                  │  │                      │   │                  │
  │ Connects to:     │  │  ┌───────────────┐   │   │  Connects        │
  │ - Relay (always) │  │  │ Control plane │   │   │  OUTBOUND to     │
  │ - roko (if avail)│  │  │ (roko-serve)  │   │   │  relay via WS    │
  │ - Agent feeds    │  │  ├───────────────┤   │   │                  │
  │                  │  │  │ Agent runtime │   │   │  Gets inference  │
  │ Subscribes to WS │  │  │ (tokio tasks) │   │   │  via parent      │
  │ per page. No     │  │  ├───────────────┤   │   │  gateway proxy   │
  │ polling.         │  │  │ Inference     │   │   │                  │
  └──────────────────┘  │  │ Gateway       │   │   └──────────────────┘
                        │  └───────────────┘   │
                        │                      │
                        │  In-process agents:   │
                        │  ┌─────┐ ┌─────┐     │
                        │  │ A1  │ │ A2  │ ... │
                        │  └─────┘ └─────┘     │
                        └──────────────────────┘
```

Three deployment tiers:

| Tier | What runs | Who needs it |
|------|-----------|--------------|
| **Backbone** | Mirage chain + relay | Everyone. Always on. Shared infrastructure. |
| **Workspace** | roko process (control plane + agent runtime + inference gateway) | Users who want orchestration, plans, PRDs, learning. |
| **Remote agents** | Standalone processes on Fly/Railway | Users who need isolation or scale. |

The backbone is the only hard dependency. Everything else is additive.
