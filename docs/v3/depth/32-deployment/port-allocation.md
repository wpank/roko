# 32-deployment/15 -- Port Allocation

> Service port assignments, the default port rationale, Fly.io internal
> ports, local conflict resolution, config paths, and deployment summary.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-serve/`, `crates/roko-agent-server/`,
`crates/roko-acp/`, `crates/roko-mcp-code/`

---

## 1. Service Port Table

All Roko services and their assigned ports:

| Port | Service | Protocol | Binding | Notes |
|------|---------|----------|---------|-------|
| 3000 | Roko Console (web terminal) | HTTPS | Public | Caddy proxy to ttyd |
| 6677 | roko-serve HTTP API | HTTPS | Public | REST + SSE + WebSocket |
| 6677 | roko serve (CLI mode) | HTTPS | Public | Same API as roko-serve |
| 7681 | ttyd (per-service) | WSS | Internal | WebSocket-to-PTY bridge |
| 8545 | EVM JSON-RPC | HTTPS | Public | Anvil-compatible |
| 9090 | Webhook ingress | HTTPS | Public | GitHub/GitLab webhooks |

### Per-Agent Sidecar

The per-agent sidecar (`roko agent serve`) listens on a dynamically
assigned port and exposes 14 routes:

| Route | Purpose |
|-------|---------|
| `/message` | Real LLM dispatch |
| `/stream` | WebSocket streaming |
| `/predictions` | Agent predictions |
| `/research` | Research queries |
| `/tasks` | Task management |
| `/health` | Sidecar health |

The port is assigned dynamically to avoid conflicts when multiple agents
run on the same host. The sidecar registers its port with the control
plane on startup.

### ACP Server

The ACP server (`roko acp`) listens on a configurable port for
Cursor/external agent integration. 180 ACP tests validate the protocol.
The default port is chosen to avoid conflicts with common development
tools.

### MCP Server

The code-intelligence MCP server (`roko-mcp-code`) communicates over
stdio, not TCP. No port allocation required. The MCP server is spawned
by editors and communicates through stdin/stdout pipes.

---

## 2. The Default Port: 6677

Port 6677 is the canonical port for roko-serve. It was chosen to avoid
conflicts with common development services:

| Port | Commonly Used By |
|------|-----------------|
| 3000 | Node.js dev servers, Next.js, Express |
| 4200 | Angular dev server |
| 5173 | Vite dev server |
| 5432 | PostgreSQL |
| 8080 | Generic HTTP servers, Tomcat, Jenkins |
| 8443 | HTTPS alternative |
| 8545 | Ethereum JSON-RPC (Anvil, Hardhat, Ganache) |

6677 is unused by common development tools and memorable. The `roko serve`
command accepts `--port` to override:

```bash
roko serve --port 6677    # Default
roko serve --port 8080    # Alternative
```

Environment variable override:

```bash
ROKO_SERVE_PORT=6678 roko serve
```

---

## 3. Fly.io Internal Ports

On Fly.io, services communicate over the private 6PN network using
`.internal` DNS:

| Internal Address | Port | Service |
|-----------------|------|---------|
| `roko-serve.internal` | 6677 | HTTP API |
| `roko-cli.internal` | 6677 | CLI orchestrator |
| `roko-console.internal` | 3000 | Web terminal |

Internal traffic stays on Fly's private IPv6 network. Public traffic
arrives through Fly's edge proxy with automatic TLS termination.

### Railway Internal Networking

On Railway, services within the same project communicate over the
private network. Railway assigns internal hostnames automatically.
Port 6677 is used consistently.

---

## 4. Local Port Conflict Resolution

When running multiple services locally, map to non-conflicting host
ports:

```yaml
# docker-compose port mapping
services:
  roko-serve:
    ports: ["6677:6677"]
  roko-cli:
    ports: ["6678:6677"]     # Remapped to avoid conflict
  console:
    ports: ["3000:3000"]
```

### Running Multiple Instances

For development with multiple workspaces, each instance needs a distinct
port:

```bash
# Workspace A
roko serve --port 6677

# Workspace B (separate terminal)
roko serve --port 6678
```

The TUI displays the active port in the status bar.

---

## 5. Configuration File Locations

All configuration paths used by deployment:

| File | Location | Purpose |
|------|----------|---------|
| `roko.toml` | Project root | Per-project config |
| `.roko/config.toml` | Project `.roko/` | Per-project overrides |
| `~/.config/roko/config.toml` | XDG config | Global config |
| `~/.config/roko/daemon.env` | XDG config | Daemon env vars (Linux) |
| `~/.local/state/roko/` | XDG state | Daemon logs, subscription state |
| `~/.local/share/roko/` | XDG data | Portable learned patterns |
| `.env` | Project root | Per-project secrets (gitignored) |
| `release-plz.toml` | Workspace root | Release pipeline config |
| `cliff.toml` | Workspace root | Changelog generation |
| `dist-workspace.toml` | Workspace root | cargo-dist config |
| `deploy/fly/*/fly.toml` | Deploy dir | Per-service Fly.io config |
| `docker/*.Dockerfile` | Docker dir | Per-service Docker builds |

---

## 6. Deployment Target Summary

| Target | Binary | Features | Size | Status |
|--------|--------|----------|------|--------|
| Native x86_64 macOS | roko-cli | All | ~30MB | **Working** |
| Native aarch64 macOS | roko-cli | All | ~30MB | **Working** |
| Native x86_64 Linux (glibc) | roko-cli | All | ~30MB | **Working** |
| Native x86_64 Linux (musl) | roko-cli | All | ~35MB | **Working** |
| Native aarch64 Linux (musl) | roko-cli | All | ~35MB | **Working** |
| Docker slim | roko-cli | All | ~20MB img | **Designed** |
| Docker full (tmux+ttyd) | roko-cli | All + web terminal | ~80MB img | **Designed** |
| WASM (wasm32-wasi) | roko-core | Core traits, HDC | ~500KB | **Flags exist** |
| Edge (aarch64 musl, opt-z) | roko-core | Core traits, HDC | ~500KB | **Designed** |
| Daemon (macOS launchd) | roko-cli | All + IPC | ~30MB | **Scaffolded** |
| Daemon (Linux systemd) | roko-cli | All + IPC | ~30MB | **Designed** |
| Cloud (Fly.io) | roko-serve | All | Docker img | **Designed** |
| Cloud (Railway) | roko-serve | All | Docker img | **Scaffolded** |

---

## 7. Test Matrix

| Test | What It Validates |
|------|------------------|
| `cargo build --workspace --release` | Native build succeeds |
| `cargo test --workspace` | 10,300+ tests pass |
| `cargo build -p roko-core --no-default-features` | Minimal build works |
| `docker build -f docker/roko-cli.Dockerfile .` | Docker slim builds |
| `docker compose up --wait` | Local stack healthy |
| `roko doctor` | Installation health |
| `roko daemon install && roko daemon status` | Daemon runs |
| `curl https://.../readyz` | Remote health |

---

## 8. Implementation Status

> **Implementation status:** Native builds work on all target triples.
> roko-serve runs on port 6677 (route counts in
> `tools/http_route_inventory.snapshot.json`). The per-agent sidecar exposes 14 routes on dynamic ports.
> ACP is wired with 180 tests. Docker images are designed but not built.
> Daemon mode is scaffolded on macOS, designed on Linux. Cloud deployment
> is scaffolded for Railway, designed for Fly.io. The release pipeline
> (release-plz, cargo-dist) is designed but not configured. WASM and edge
> targets have feature flags but no end-to-end validation.
