# L4 — Orchestration Layer

> User-facing entry points: the CLI and the HTTP/WebSocket server.

**Status**: Shipping
**Crates**: `roko-cli`, `roko-serve`
**Depends on**: [L3 Harness](04-L3-harness.md)
**Used by**: Nothing within Roko (this is the top of the stack)
**Last reviewed**: 2026-04-19

---

## TL;DR

L4 is where operators and users interact with Roko. `roko-cli` provides a command-line
interface for starting, configuring, and inspecting agents. `roko-serve` exposes a
REST and WebSocket API for programmatic access. Both layers are thin shells over L3 —
they parse user input, call L3, and format the result.

---

## `roko-cli`

`roko-cli` provides the `roko` binary. Key commands:

```
roko agent spawn --config agent.toml       # start a new agent
roko agent list                            # list running agents
roko agent status --id <id>                # health, metrics, recent ticks
roko agent consolidate --id <id>           # trigger manual Delta pass
roko agent stop --id <id>                  # graceful shutdown

roko tick run --config agent.toml          # run a single tick (for testing)
roko substrate inspect --path ./agent.db   # browse Substrate contents

roko config validate agent.toml           # lint the configuration
roko config generate --profile minimal    # generate a starter config
```

Full CLI reference: see
[`guides/cli-reference.md`](../../guides/cli-reference.md).

---

## `roko-serve`

`roko-serve` exposes a REST + WebSocket API:

```
POST /agents                               # spawn a new agent
GET  /agents                               # list all agents
GET  /agents/{id}/status                   # health + metrics
POST /agents/{id}/stimulus                 # inject a Pulse
WS   /agents/{id}/stream                  # live stream of Pulses and Engrams
POST /agents/{id}/consolidate              # trigger Delta
DELETE /agents/{id}                        # stop and remove

GET  /engrams?agent={id}&kind={kind}      # query Substrate
GET  /engrams/{id}                         # single Engram by ID
```

Authentication: Bearer token, configurable in `roko-serve` config.

---

## L4's Thin-Shell Principle

L4 crates contain minimal logic. They do:
- Parse command-line arguments or HTTP request bodies
- Validate inputs (using `roko-core` validators)
- Call `Orchestrator` methods at L3
- Format the result for the output channel

They do **not**:
- Make routing decisions
- Manipulate Engrams directly
- Call any L2 crate directly

This keeps L4 testable (thin shells are easy to mock), portable (the same CLI can
be used against a remote `roko-serve`), and auditable (all state changes flow through L3).

---

## Configuration and the Config File

The `agent.toml` configuration file is the primary API for configuring agent behavior.
It is parsed at L4 and passed to L3's `TickContextBuilder`. Key sections:

```toml
[agent]
id   = "my-agent"
name = "Research Agent"

[substrate]
backend = "sled"
path    = "./data/agent.db"

[speeds.gamma]
confidence_threshold = 0.85
max_context_tokens   = 4096

[model.gamma]
provider = "openai"
model    = "gpt-4o-mini"

[cross_cuts]
neuro  = true
daimon = true
dreams = true
```

Full configuration reference: see
[`operations/configuration/README.md`](../../operations/configuration/README.md).

---

## See also

- [L3 Harness](04-L3-harness.md) — the layer L4 calls
- [Dependency Rules](06-dependency-rules.md) — L4 is the top; it may import any lower layer
- [guides/cli-reference.md](../../guides/cli-reference.md) — full CLI documentation
- [operations/configuration](../../operations/configuration/README.md) — config reference
