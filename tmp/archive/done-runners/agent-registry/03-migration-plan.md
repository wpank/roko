# 03 — Migration Plan

This plan is organized around the real critical path, not around every
possible cleanup.

## Critical path

The smallest end-to-end path that proves the design works is:

1. mirage boots with target ERC-8004 contracts
2. relay accepts agent connections
3. `roko agent serve` exists
4. an agent can appear via relay and/or 8004
5. Kauri dashboard and mirage static demo can both message that agent
6. the same system works locally and on Railway

Everything else is secondary.

## High-level change summary

| Area | Main change |
|---|---|
| `contracts/` | add target ERC-8004 `IdentityRegistry` and stub companion registries |
| `apps/mirage-rs/` | fork-or-deploy 8004, keep default `/relay/*` runtime shape, deprecate legacy registry |
| `apps/agent-relay/` | add standalone relay binary |
| `crates/roko-agent-server/` | add relay client and target chain registration support |
| `crates/roko-cli/` | add `roko agent serve` |
| `apps/mirage-rs/static/` | update in-tree demo to merge 8004 + relay |
| `nunchi-dashboard` repo | replace endpoint cache / `VITE_ROKO_URL` split with 8004 + relay merge |

## Phase 1 — Target ERC-8004 on mirage

**Goal:** mirage exposes the target `IdentityRegistry` interface.

Work:

- add `contracts/src/IdentityRegistry.sol`
- add `contracts/src/ReputationRegistry.sol` and `ValidationRegistry.sol` stubs
- add deterministic deploy script
- make `mirage-rs` fork if available, otherwise deploy at boot
- stop expanding `apps/mirage-rs/src/chain/agent.rs`

Deliverable:

- `registeredCount()` returns `0`
- target `updateAgentCardUri(uint256,string)` is part of the contract surface

## Phase 2 — Relay binary

**Goal:** a standalone relay exists and can route messages.

Work:

- add `apps/agent-relay/`
- implement WS hello, message forwarding, and card hosting
- add smoke tests for hello -> message -> response

Deliverable:

- `GET /relay/health` returns `ok`
- `POST /relay/agents/{id}/message` works against a test agent

## Phase 3 — `roko agent serve`

**Goal:** the per-agent server is actually runnable in production.

This is the biggest hidden gap in the current codebase. Today
`AgentServer::builder()` is effectively only exercised in tests.

Work:

- add `roko agent serve`
- wire dispatcher, LLM backend, knowledge store, and feature flags
- expose agent ID, owner, bind address, relay URL, and optional chain settings

Deliverable:

- `roko agent serve --agent-id demo-1` binds a local port and answers `/health`

## Phase 4 — Agent relay client and chain registration

**Goal:** one agent binary can participate in both discovery paths.

Work:

- add relay client inside `roko-agent-server`
- reconnect and resend hello on failure
- add target chain registration path
- make wallet optional and silent when absent
- align the agent registration helper to the target ABI from
  [05-contracts-and-identity.md](05-contracts-and-identity.md)

Deliverable:

- relay-only agent shows up in `GET /relay/agents`
- wallet agent updates its `agentCardUri` on-chain

## Phase 5 — Default mirage runtime shape

**Goal:** `mirage-rs + agent-relay` becomes the default local and Railway stack.

Work:

- build relay alongside mirage in `docker/mirage.Dockerfile`
- add default entrypoint that runs both
- add default `/relay/*` forwarding from mirage to relay

Deliverable:

- one service exposes chain RPC and relay routes together

## Phase 6 — Kauri dashboard migration

**Goal:** the external dashboard repo stops depending on endpoint cache and
`roko-serve`.

Repo:

- external repo: `nunchi-dashboard` (Kauri dashboard)

Work:

- replace `localStorage` endpoint lookup in `src/components/ai-studio/AskPanel.tsx`
- replace `VITE_ROKO_URL` split in `src/services/mirage-api.ts`
- add 8004 reader, relay client, and merged agent hook
- route direct when a public rest endpoint exists, else relay

Deliverable:

- dashboard works with `VITE_CHAIN_URL` only in the default same-domain setup

## Phase 7 — Local demo and quickstart

**Goal:** this repo ships its own fast end-to-end proof.

Work:

- update `apps/mirage-rs/static/quickstart.sh`
- start mirage + relay
- spawn at least one wallet-free demo agent
- optionally spawn one wallet agent
- update the static UI to merge 8004 + relay and send test messages

Deliverable:

- a reviewer can run the in-tree demo and verify the system without cloning the
  dashboard repo

## Phase 8 — Remote Railway mixed-topology verification

**Goal:** verify the design in the deployment shape that actually matters.

This is a required verification phase, not a nice-to-have.

### Target setup

- **Service A:** `mirage-rs + agent-relay` on Railway from `docker/mirage.Dockerfile`
- **Service B:** one remote agent container on Railway using `docker/roko.Dockerfile`
  with a start command that runs `roko agent serve`
- **Local agent:** one laptop agent connecting to the remote relay
- **Dashboard:** Kauri dashboard pointed at the Railway mirage URL

### Required verification matrix

1. remote deployed wallet or relay-backed agent appears
2. local laptop agent appears through the same remote relay
3. dashboard can message both
4. mirage static demo can also see and message both
5. restart of relay service causes reconnect and recovery

### Why this phase is mandatory

It proves the design is not only a localhost trick:

- remote + local agents can coexist
- relay URL conventions actually work
- default Docker and Railway wiring is sound

## Phase 9 — Deprecate `roko-serve`

**Goal:** remove `roko-serve` from the production path without deleting the crate.

Work:

- mark crate and README deprecated
- stop deploying it to Railway
- keep local-dev TUI compatibility

Deliverable:

- no production discovery or messaging path depends on `roko-serve`

## Issue #15 resolution text

This is the text that should eventually go back to the issue:

> The issue identified a real UX gap, but adding `endpoint` to mirage's Rust
> `AgentEntry` would solve it in the wrong layer. The replacement design moves
> durable identity to ERC-8004 and live reachability to a relay, with
> `roko-agent-server` exposing the actual per-agent transport surface. See
> `tmp/agent-registry/` for the design notes and migration plan.

## What moves where

| Existing responsibility | New home |
|---|---|
| durable agent identity | ERC-8004 |
| live agent presence | relay |
| per-agent message execution | `roko-agent-server` |
| local demo verification | mirage static UI + quickstart |
| production web verification | Kauri dashboard |
| orchestration / plans / PRDs | not part of this migration's hot path |

## Ordering summary

```
P1  8004 contracts on mirage
P2  relay binary
P3  roko agent serve
P4  agent relay client + chain registration
P5  default mirage runtime shape
P6  Kauri dashboard migration
P7  local demo
P8  remote Railway mixed-topology verification
P9  roko-serve deprecation
```

Dependencies:

- P1, P2, P3 can proceed in parallel
- P4 depends on P2 + P3 and part of P1
- P5 depends on P2
- P6 depends on P1 + P2 + P4 + P5
- P7 depends on P1 + P2 + P3 + P4 + P5
- P8 depends on P5 + P6 and ideally P7
- P9 follows once P6 and P8 pass

## Main risks

| Risk | Mitigation |
|---|---|
| Docs and code drift on the target 8004 ABI | treat [05-contracts-and-identity.md](05-contracts-and-identity.md) as the target contract definition and update code to match it |
| Relay becomes a dumping ground for control-plane features | keep its scope limited to presence, card hosting, and transport |
| Dashboard migration stalls because it lives in another repo | keep the in-tree mirage demo as a parallel verification surface |
| Railway success hides local failures or vice versa | require both Phase 7 and Phase 8 before calling the design proven |
