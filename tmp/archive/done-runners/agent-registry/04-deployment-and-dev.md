# 04 — Deployment and Dev Workflow

This doc describes the **default** way to run and verify the system:

- locally
- in the in-tree mirage demo
- against the external Kauri dashboard
- on Railway with a mix of remote and local agents

## Default topology

### Local

- `mirage-rs` on `:8545`
- `agent-relay` on `:3100`
- default public shape via `http://localhost:8545/relay/*`
- one or more agents:
  - local direct agent
  - local relay-connected agent

### Railway

- one Railway service running `mirage-rs + agent-relay`
- zero or more remote agent services
- zero or more laptop agents connecting back to the Railway relay
- Kauri dashboard pointed at the Railway mirage URL

This topology is the default because it gives us one canonical setup to test.

## Railway service layout

### After the migration

- **`mirage-devnet`**
  - built from `docker/mirage.Dockerfile`
  - runs `mirage-rs + agent-relay`
  - exposes the chain root URL and `/relay/*`
- **remote agent services**
  - built from `docker/roko.Dockerfile`
  - start command overridden to run `roko agent serve`
- **Kauri dashboard**
  - separate repo
  - configured with `VITE_CHAIN_URL`

`roko-serve` is not part of the deployed data plane in this model.

## Docker and process model

### `docker/mirage.Dockerfile`

Target behavior:

- build `mirage-rs`
- build `agent-relay`
- run both under a minimal init process

Why:

- this is the easiest default local/remote shape
- it gives one service and one default origin

### `docker/roko.Dockerfile`

Target behavior:

- still usable as the base image for agent processes
- Railway start command overrides the current default `serve`
- remote agent demo uses:

```bash
roko agent serve --agent-id remote-demo-1 --relay-url wss://<mirage-domain>/relay/ws/agent
```

No new Dockerfile is required for the first remote-agent demo if Railway
overrides the command cleanly.

## Path routing

The default same-domain shape is:

- chain traffic: `{CHAIN_URL}`
- relay traffic: `{CHAIN_URL}/relay/*`

Recommended implementation:

- mirage forwards `/relay/*` to the local relay process

Alternative:

- expose relay on a second domain and set `VITE_RELAY_URL`

The architecture does not depend on the proxy, but the default dev and demo
experience does.

## Ports

| Port | Service | Public by default? | Purpose |
|---|---|---|---|
| 8545 | mirage-rs | yes | JSON-RPC, health, static demo, relay passthrough |
| 3100 | agent-relay | internal by default | relay WS + REST |
| ephemeral | per-agent server | public or private | direct agent HTTP/WS |
| 9090 | roko-serve | no | local-only legacy path during migration |

## Environment variables

### Kauri dashboard (`nunchi-dashboard`, separate repo)

Target `.env.example` shape:

```bash
VITE_CHAIN_URL=https://mirage-devnet-production.up.railway.app
VITE_CHAIN_ID=88888

# Optional only when relay is split away from the chain origin
# VITE_RELAY_URL=https://relay.example.com/relay

# Optional auth when multi-user flows are enabled
# VITE_PRIVY_APP_ID=
```

Target frontend defaults:

```ts
const CHAIN_URL = import.meta.env.VITE_CHAIN_URL ?? "http://localhost:8545";
const RELAY_URL = import.meta.env.VITE_RELAY_URL ?? `${CHAIN_URL}/relay`;
```

`VITE_ROKO_URL` should disappear from the discovery/messaging path.

### Agents

Target `roko.toml` shape:

```toml
[agent.server]
bind = "0.0.0.0:0"
relay_url = "wss://mirage-devnet-production.up.railway.app/relay/ws/agent"
chain_rpc = "https://mirage-devnet-production.up.railway.app"
passport_id = 42
wallet_key = "env:AGENT_WALLET_KEY"
cors_origins = ["https://dashboard.example.com", "http://localhost:5173"]
```

Wallet-free agents simply omit the chain values.

### Relay

```bash
RELAY_BIND=0.0.0.0:3100
RELAY_LOG=info
```

## Local dev recipes

### Recipe A — native cargo

```bash
# Terminal 1
cargo run -p mirage-rs --release --features "binary,chain" -- \
  --host 127.0.0.1 --port 8545

# Terminal 2
cargo run -p agent-relay --release -- --bind 127.0.0.1:3100

# Terminal 3
cargo run -p roko-cli -- agent serve --agent-id demo-1 \
  --relay-url ws://localhost:3100/relay/ws/agent

# Terminal 4
cd ~/dev/nunchi/nunchi-dashboard && npm run dev
```

### Recipe B — Docker plus external agent

```bash
docker compose -f docker/docker-compose.dev.yml up

# separate terminal
cargo run -p roko-cli -- agent serve --agent-id demo-1 \
  --relay-url ws://localhost:3100/relay/ws/agent
```

### Recipe C — quickstart

The in-tree quickstart should become the fastest local proof:

```bash
apps/mirage-rs/static/quickstart.sh
```

That script should:

- start mirage
- start relay
- wait for relay health
- spawn at least one relay-backed agent
- optionally spawn one wallet-backed agent
- open the static UI

## Local demo requirements

The mirage static demo is a first-class verification target.

It must prove:

- 8004 is available
- relay is available
- wallet-free agents appear
- wallet-backed agents appear
- messages reach both transports

### Required UI behavior

The static UI should:

- merge 8004 agents and relay agents
- show online status
- choose direct vs relay transport automatically
- provide a simple per-agent test message path

## Remote Railway mixed-topology demo

This is the required deployment proof for the design.

### Goal

Verify that one deployed agent and one laptop agent can both participate
against the same remote mirage + relay service.

### Target setup

#### Service A — mirage + relay

- Railway service built from `docker/mirage.Dockerfile`
- public URL becomes `VITE_CHAIN_URL`

#### Service B — remote deployed agent

- Railway service built from `docker/roko.Dockerfile`
- override start command to run:

```bash
roko agent serve \
  --agent-id remote-demo-1 \
  --relay-url wss://<mirage-domain>/relay/ws/agent
```

If a wallet path is being tested, add the chain RPC and wallet flags.

#### Laptop agent

Run from local machine:

```bash
cargo run -p roko-cli -- agent serve --agent-id laptop-demo-1 \
  --relay-url wss://<mirage-domain>/relay/ws/agent
```

#### Dashboard

- point Kauri dashboard at `VITE_CHAIN_URL=https://<mirage-domain>`
- rely on default `RELAY_URL=${CHAIN_URL}/relay`

### Required acceptance checks

1. `GET https://<mirage-domain>/relay/health` returns `ok`
2. remote deployed agent appears in dashboard and static demo
3. local laptop agent appears in dashboard and static demo
4. dashboard can message both agents
5. static demo can message both agents
6. relay restart causes both agents to reconnect

### Why this matters

This is the simplest proof that the design works in the real mixed topology:

- one chain host
- one remote agent
- one local agent
- one dashboard

If this setup is healthy, the architecture is sound enough to iterate.

## Kauri dashboard migration notes

The external dashboard currently has:

- `localStorage` endpoint caching in `src/components/ai-studio/AskPanel.tsx`
- `VITE_ROKO_URL` and `ROKO_BASE` in `src/services/mirage-api.ts`

Target migration:

- read identities from 8004
- read presence from relay
- choose direct or relay transport per agent
- remove `localStorage` endpoint caching from the main path

## Auth posture

### MVP

- relay accepts unauthenticated hello
- dashboard can read agent lists without auth
- message-send auth is deferred

### Later

- dashboard send auth via SIWE / Privy
- relay agent auth via bearer token and/or signed hello

Wallet-free operation must remain supported after auth lands.

## Health and observability

Required health surface:

- `GET {CHAIN_URL}/health`
- `GET {CHAIN_URL}/relay/health`

Useful optional surface:

- `GET {CHAIN_URL}/relay/metrics`
- structured logs for both processes

## Remaining open items

These stay open after the docs refresh:

1. exact home for 1-click deploy orchestration after `roko-serve`
2. exact phase-2 auth mechanism details
3. whether the Kauri dashboard keeps any temporary compatibility shim during rollout
