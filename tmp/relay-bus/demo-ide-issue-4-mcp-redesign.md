# Demo IDE Issue 4: Per-Agent MCP Config, Not A Hosted Gateway

Date: 2026-05-08
Source: https://github.com/Nunchi-trade/demo-ide/issues/4

## Readout

Issue #4 correctly identifies one real constraint: a Railway or OpenClaw-hosted
agent cannot call a user's laptop at `127.0.0.1`. The proposed solution is the
wrong default. It turns that network constraint into a Nunchi-hosted MCP gateway,
account-scoped MCP state, gateway tokens, gateway discovery APIs, and centralized
tool-call execution.

The better design is simpler:

1. MCP configuration belongs to a specific agent.
2. That agent receives a JSON MCP config at launch or deploy time.
3. Stdio MCP servers run next to the agent, inside the same local machine,
   Railway container, or OpenClaw runtime.
4. HTTP MCP servers are called directly by the agent from its own runtime.
5. Nunchi can provide templates and optional managed connectors, but it should
   not be the default place where every user's MCP calls execute.

In other words: do not make "hosted gateway" the default. Materialize
`mcpServers` for the selected agent.

## What The Issue Gets Wrong

### It confuses locality with ownership

The issue says hosted agents cannot reach the local IDE bridge, so the default
should be a Nunchi-hosted MCP gateway. The missing step is: the MCP server should
usually move to the hosted agent's runtime.

For a Railway agent that needs a market-data MCP, the config should launch that
MCP server in the Railway instance, or point directly at the market-data HTTP MCP
endpoint. It does not need to bounce through a Nunchi account gateway. If the MCP
server wraps user-owned credentials, those credentials belong in the user's
Railway project or chosen runtime secret store.

### It makes MCP account-global when it should be agent-bound

The issue says the Nunchi app stores MCP config on the user's account, and the
hosted gateway gives linked agents account-scoped access. That is too broad.

The actual operational question is:

> Which MCP servers is this specific agent allowed to use in this specific
> runtime?

An account-level catalog can exist as a convenience, but the runtime contract
should be per-agent. A research agent, a trading agent, a code agent, and a local
desktop agent should not inherit one shared account MCP surface by default.

### It centralizes secrets in the wrong place

The issue tries to avoid raw secrets by giving the agent a `NUNCHI_MCP_TOKEN`.
That only moves the secret boundary into Nunchi's gateway. Nunchi then becomes
responsible for storing, brokering, redacting, auditing, and calling arbitrary
third-party tools for all users.

For user-owned MCP servers, the default should be:

- local agent: user secrets live in the local environment or local files;
- Railway agent: user secrets live in Railway environment variables;
- OpenClaw hosted runtime: user secrets live in that runtime's secret mechanism;
- remote HTTP MCP: user supplies endpoint and auth material to the agent runtime.

Nunchi should not need to see raw third-party secrets, and it should not need to
hold a universal delegated token that can call every configured MCP tool.

### It turns a deployment problem into a platform service

The issue's API shape:

```text
GET  /mcp/servers
GET  /mcp/servers/{id}/tools
POST /mcp/servers/{id}/tools/{name}/call
```

is a new hosted MCP execution plane. That is a large product and security
surface. It is not required to make hosted agents use MCP. The existing Roko and
ACP shape already supports "tell this agent which MCP servers exist."

The work should be to make that attachment flow reliable across local, Railway,
and OpenClaw runtimes.

## Current Repo Signals

The local code and docs already point away from a mandatory hosted gateway:

- `docs/v2/INTEGRATION-GUIDE.md` describes MCP server configuration with
  Claude-style `mcpServers` JSON and says agents receive MCP config for providers
  that support it.
- `crates/roko-agent/src/mcp/config.rs` models MCP config as concrete servers
  with `stdio` or `http` transport, command/args/env, endpoint, auth token, and
  tier.
- `crates/roko-agent/src/process/mcp.rs` explicitly treats an MCP server as
  spawned by the agent process, and discovers `.roko/mcp-config.json` or
  `.roko/mcp-config.local.json`.
- `crates/roko-agent/src/claude_cli_agent.rs` passes discovered MCP config to
  Claude CLI with `--mcp-config` and `--strict-mcp-config`.
- `crates/roko-acp/src/types.rs` and `crates/roko-acp/src/session.rs` support
  `mcpServers` on ACP `session/new`.
- `demo-ide/src/components/AgentChat.tsx` and
  `demo-ide/src/components/CodeAgentTile.tsx` already pass `mcpServers` into
  `session/new` for `nunchi-mcp`.
- `demo-ide/src/lib/roko.ts` explicitly says the IDE should register
  `nunchi-mcp` in ACP `session/new` so spawned agents get tools through standard
  MCP discovery.

The existing shape is: attach MCP config to the agent/session, then let the
agent runtime spawn or connect to the MCP servers.

## Proposed Design

### Core contract

Each agent has an MCP profile. The profile is a JSON document that can be
rendered into the formats already understood by Roko, Claude CLI, and ACP.

The profile is not "the user's MCP account state." It is an agent launch input:

```json
{
  "agent_id": "isfr-alpha",
  "runtime": "railway",
  "mcp_config_path": ".roko/agents/isfr-alpha/mcp.json",
  "allowed_tools": [
    "coingecko.get_price",
    "coingecko.get_markets"
  ]
}
```

The actual MCP config can use the common Claude-style shape:

```json
{
  "mcpServers": {
    "coingecko": {
      "command": "npx",
      "args": ["-y", "@acme/coingecko-mcp"],
      "env": {
        "COINGECKO_API_KEY": "${COINGECKO_API_KEY}"
      }
    },
    "nunchi": {
      "command": "/app/bin/nunchi-mcp",
      "args": []
    }
  }
}
```

Roko can normalize this into its internal `servers` shape when needed:

```json
{
  "servers": [
    {
      "name": "coingecko",
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@acme/coingecko-mcp"],
      "env": {
        "COINGECKO_API_KEY": "${COINGECKO_API_KEY}"
      },
      "tier": "sandboxed"
    },
    {
      "name": "market-http",
      "transport": "http",
      "endpoint": "https://tools.example.com/mcp",
      "auth_token": "${MARKET_MCP_TOKEN}",
      "tier": "network"
    }
  ]
}
```

The important part is that this JSON is assigned to the specific agent and made
available in the runtime where that agent runs.

### Runtime materialization

Local desktop agent:

- IDE writes `.roko/agents/<agent-id>/mcp.json` or `.roko/mcp-config.local.json`.
- ACP `session/new` receives `mcpServers` directly for short-lived sessions.
- Stdio MCP servers spawn on the user's machine.
- Local-only resources such as filesystem, Obsidian, browser automation, or IDE
  tiles remain local.

Railway agent:

- Railway template accepts `ROKO_AGENT_MCP_CONFIG_JSON` or
  `ROKO_AGENT_MCP_CONFIG_B64`.
- The boot script writes the decoded config to `.roko/mcp.json` or
  `.roko/mcp-config.json` before launching the agent.
- Required binaries or package managers are part of the template image, or the
  MCP server command uses pinned package execution such as `npx -y`.
- Secrets are Railway environment variables referenced by the config.
- Stdio MCP servers spawn inside the Railway service as child processes or
  sibling sidecars when isolation is useful.

OpenClaw or ACP agent:

- The launcher passes `mcpServers` in `session/new`.
- If the OpenClaw runtime is remote, that runtime must either have the MCP
  command available or receive a deploy-time config artifact.
- The IDE should not assume the remote OpenClaw runtime can reach the local
  desktop bridge.

Remote HTTP MCP:

- The config contains the endpoint and auth reference.
- The agent calls the endpoint directly.
- If the HTTP MCP is user-owned, Nunchi is not in the call path.

Optional Nunchi-managed MCP:

- Nunchi may offer managed connectors for Nunchi-owned services.
- Those appear as normal HTTP MCP entries in the agent's config.
- This is a product feature, not the default architecture for all MCPs.

## Demo IDE Product Shape

Replace "connection mode: local gateway / hosted gateway / Railway sidecar" with
"where this selected agent will run this MCP."

Suggested UI model:

- MCP catalog: install recipes and known server templates.
- Agent MCP profile: the servers attached to the currently selected agent.
- Runtime compatibility: local, Railway, OpenClaw, HTTP remote, managed Nunchi.
- Secret binding status: missing, local env, Railway env, remote token, managed.
- Tool allowlist: explicit tools or server-wide grant for this agent.
- Materialization preview: the JSON that will be written or passed to the agent.

The primary action should be "Add to agent", not "Enable on account."

For example, adding a market data MCP to `isfr-alpha` should produce:

```text
.roko/agents/isfr-alpha/mcp.json
Railway env:
  ROKO_AGENT_MCP_CONFIG_B64=...
  COINGECKO_API_KEY=...
```

Then the Railway boot process writes the file and launches the agent. No
Nunchi-hosted `/mcp/servers` endpoint is involved.

## Service Boundaries

### Nunchi app

The app can store:

- catalog entries;
- install recipes;
- per-agent desired MCP profiles;
- non-secret metadata;
- runtime compatibility hints;
- allowlist policy intended for the agent.

The app should avoid storing raw third-party secrets unless the user explicitly
chooses a Nunchi-managed connector.

### Relay bus

The relay bus should observe and route agent events. It should not be the MCP
execution plane.

Useful events:

```text
mcp.config.materialized
mcp.server.starting
mcp.server.ready
mcp.server.failed
mcp.tool.call.started
mcp.tool.call.completed
mcp.tool.call.failed
```

Events should include agent id, runtime id, server id, tool name, status, timing,
and redacted arguments. This satisfies the audit need without forcing all calls
through a central gateway.

### Agent runtime

The agent runtime owns:

- reading the config file;
- spawning stdio MCP servers;
- connecting to HTTP MCP servers;
- converting MCP tools into the agent's tool registry;
- enforcing local allowlists and permission tiers;
- emitting audit events.

This matches the current Roko direction.

### Railway template

The Railway template should be a deployment convenience:

- provision the agent process;
- install or include common MCP runtime dependencies;
- accept a per-agent MCP JSON payload;
- write it into the workspace before launch;
- reference secrets from Railway env vars;
- optionally run sidecars for servers that need a separate process.

Publishing a Nunchi-owned Railway template may still be a distribution channel,
but that does not require Nunchi to operate the MCP gateway for every user.

## Revised Acceptance Criteria

Replace the issue's acceptance criteria with these:

- Demo IDE can attach an MCP server to one selected agent, not only to a global
  user account.
- Demo IDE can render the exact MCP JSON that will be passed to that agent.
- Local ACP sessions receive `mcpServers` through `session/new`.
- Roko agents can read a per-agent `.roko/mcp.json` or `.roko/mcp-config.json`.
- Railway template accepts per-agent MCP config as JSON or base64 env, writes it
  to disk, and starts the agent with that config.
- Railway secrets remain in Railway env vars and are referenced by the config.
- A Railway-style agent can call a market-data MCP without the local IDE bridge
  and without a Nunchi-hosted MCP gateway.
- Tool allowlists are enforced by the agent runtime before MCP tool execution.
- MCP call audit events are emitted to logs and the relay bus with redacted args.
- Local-only MCPs are marked incompatible with hosted runtimes unless the user
  explicitly provides a sync, volume, tunnel, or remote equivalent.
- Optional Nunchi-managed MCP connectors can be represented as HTTP MCP config
  entries, but they are not required for user-owned MCP servers.

## Migration From Issue 4

Keep:

- the diagnosis that hosted agents cannot reach `127.0.0.1` on the user's
  laptop;
- Railway sidecar support;
- per-agent allowlists;
- audit events;
- an e2e test proving hosted agents can call market data tools.

Change:

- "Nunchi-hosted MCP gateway is default" -> "per-agent MCP config is default";
- "account-scoped MCP access" -> "agent-scoped MCP profile";
- "`NUNCHI_MCP_TOKEN` for all hosted MCP calls" -> "runtime-local secrets and
  direct MCP config";
- "gateway discovery/call API" -> "agent runtime discovery/call via MCP";
- "local state mirrors into Railway whenever a new MCP is added" -> "user
  explicitly attaches an MCP server to a specific hosted agent, then deploys or
  syncs that config."

## What Not To Build Yet

Do not build these as the default path:

- a hosted `/mcp/servers` API for every user's tools;
- a Nunchi account vault that brokers arbitrary third-party MCP secrets;
- a local-to-Railway automatic mirror of every MCP the user has configured;
- a gateway-only OpenClaw template that cannot run stdio MCP servers locally in
  the agent runtime;
- a central allowlist implementation that bypasses agent runtime permission
  checks.

These may become optional enterprise or managed-connector features later. They
should not be the foundation for hosted agents using MCP.

## Open Questions

1. Should the user-facing config format be Claude-style `mcpServers`, Roko's
   internal `servers`, or both with normalization at the boundary?
2. Where should the Demo IDE persist per-agent MCP profiles:
   `.roko/agents/<agent-id>/mcp.json`, `.roko/mcp/<agent-id>.json`, or a local
   app database that materializes files on launch?
3. Should Railway deploy write MCP JSON from `ROKO_AGENT_MCP_CONFIG_JSON`, base64,
   a mounted file, or a generated config committed into the template workspace?
4. How should secret references be validated before deploy, especially when the
   IDE can see that `${COINGECKO_API_KEY}` is required but cannot read Railway's
   env values?
5. What is the exact audit event schema on the relay bus for MCP server status
   and tool calls?
6. Should local-only MCPs be blocked for hosted agents by default, or allowed
   only behind an explicit tunnel/sync choice?
7. How much HTTP MCP support is needed in Roko's HTTP-backed tool loop before
   this works uniformly across providers?

## Bottom Line

Issue #4 is right that hosted agents cannot use a laptop-local bridge. It is
wrong that this implies a default Nunchi-hosted MCP gateway.

The default should be agent-local MCP materialization: add the MCP server as JSON
to the specific agent, put secrets in that agent's runtime, spawn stdio servers
beside the agent, and call HTTP MCP servers directly. Nunchi can still provide a
catalog, templates, optional managed connectors, and relay-bus audit events
without owning every user's MCP execution path.
