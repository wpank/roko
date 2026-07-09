# 06 — Decisions and Context

This is the shortest way to recover the current stance of the design.

## Firm decisions

### 1. Issue #15 identified the right UX problem but the wrong fix

Do not add `endpoint` to mirage's Rust `AgentEntry` as the long-term answer.

### 2. `mirage-rs` is a chain host, not the durable agent registry

Its legacy Rust registry can exist during migration, but the target durable
identity model is ERC-8004.

### 3. `agent-relay` is a standalone component

It is not conceptually part of the chain. It is co-deployed with mirage by
default because that is the best local and Railway operating shape.

### 4. Wallet-free agents are first-class in production

They are not a dev-only shortcut.

### 5. Direct transport is primary; relay is selective

Use direct HTTP when the agent is public and advertises it. Use relay when the
agent is relay-first, wallet-free, or private.

### 6. The target contract ABI is defined in `05-contracts-and-identity.md`

In particular, the target agent-card update method is:

`updateAgentCardUri(uint256,string)`

Current helper code must eventually match that target.

### 7. `roko agent serve` is mandatory

Without a real production caller for `roko-agent-server`, the rest of the
design is just a paper architecture.

### 8. The default runtime shape is mirage + relay on one origin

That is the standard way to run:

- local dev
- in-tree demo
- Railway demo

### 9. Two verification surfaces matter

- **Kauri dashboard** in the external `nunchi-dashboard` repo
- **mirage static demo** inside this repo

Both must be able to discover and message agents through the same model.

### 10. Remote Railway mixed-topology verification is required

The design is not considered proven until:

- one remote deployed agent works
- one local laptop agent works
- both connect to the same remote relay
- both are reachable from the dashboard and the static demo

### 11. `roko-serve` is deprecated, not deleted

It can remain for local TUI and compatibility paths during migration, but it is
not part of the target discovery or messaging data plane.

## Key scenarios

### A. Public deployed agent with wallet

- agent owns a passport
- agent hosts its card directly
- dashboard reads card URI from 8004
- dashboard sends direct `POST /message`

### B. Deployed agent without wallet

- agent connects to relay
- relay hosts the card
- dashboard discovers it via relay
- dashboard messages it through relay

### C. Laptop agent without wallet

- agent runs locally
- agent connects outbound to local or remote relay
- same relay-based discovery path as B

### D. Mixed Railway verification

- remote mirage + relay on Railway
- remote deployed agent on Railway
- local laptop agent against the same remote relay
- Kauri dashboard pointed at the Railway mirage URL
- mirage static demo also pointed at the same origin

## Deferred items

These are intentionally not on the critical path.

| Deferred item | Reason |
|---|---|
| final relay auth design | not needed to prove the discovery model |
| 1-click deploy UX redesign | separate product and control-plane problem |
| full `roko-serve` deletion | depends on later StateHub migration work |
| non-stub reputation / validation registries | not required for discovery or messaging |
| standalone relay service by default | only needed if the one-origin default stops being good enough |

## Relevant files in this repo

| Area | Path |
|---|---|
| legacy mirage agent registry | `apps/mirage-rs/src/chain/agent.rs` |
| mirage HTTP layer | `apps/mirage-rs/src/http_api/` |
| mirage entrypoint | `apps/mirage-rs/src/main.rs` |
| relay target location | `apps/agent-relay/` |
| agent server | `crates/roko-agent-server/src/lib.rs` |
| current registration helper | `crates/roko-agent-server/src/registration.rs` |
| CLI entrypoint | `crates/roko-cli/src/main.rs` |
| legacy control plane | `crates/roko-serve/` |
| mirage Docker image | `docker/mirage.Dockerfile` |
| roko Docker image | `docker/roko.Dockerfile` |
| local demo launcher | `apps/mirage-rs/static/quickstart.sh` |
| local demo UI | `apps/mirage-rs/static/` |

## Relevant files in the Kauri dashboard repo

The dashboard repo is separate, but these are the paths that matter:

| Area | Path |
|---|---|
| current endpoint cache logic | `src/components/ai-studio/AskPanel.tsx` |
| current split base URLs | `src/services/mirage-api.ts` |
| current constants | `src/services/constants.ts` |
| env template | `.env.example` |

## FAQ

### Why not just put the URL in mirage's agent registry?

Because it solves discovery as a mirage-specific UI cache problem instead of as
a durable identity + reachability problem.

### Why keep the relay next to mirage if they are conceptually separate?

Because the default runtime shape should be the easiest thing to run and the
easiest thing to demo. One origin is the cleanest default.

### Why require both the Kauri dashboard and the in-tree static demo?

Because one repo is the real web UI and the other is the fastest review loop.
We need both.

### Why insist on the Railway mixed-topology demo?

Because a localhost-only success case does not prove that local and deployed
agents can coexist through the same remote discovery surface.
