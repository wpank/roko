# 20 — Deployment

> Local development, cloud deployment, scaling tiers, and environment configuration.

**Source**: `tmp/architecture/17-deployment.md` (terminology update to unified vocabulary).

---

## 1. Overview

Roko runs at three scales:

| Tier | Users | Deployment | Agents |
|---|---|---|---|
| **Solo developer** | 1 | `roko serve` on localhost | 1-10 in-process |
| **Small team** | 2-10 | Railway or Fly.io single instance | 10-50 in-process |
| **Production** | 10+ | Railway/Fly multi-instance + relay | 50+ in-process + isolated |

All tiers use the same binary. The difference is configuration: environment variables, execution mode, and whether a relay is involved.

---

## 2. Local Development

### 2.1 Getting Started

```bash
# Install
cargo install roko-cli

# Initialize workspace
roko init

# Set API key
echo "sk-ant-..." | roko config secrets set llm.anthropic

# Start control plane (insecure mode for local dev -- no auth required)
roko serve --insecure

# In another terminal: interactive TUI
roko dashboard
```

The control plane starts on `localhost:6677` with ~85 HTTP routes, SSE, and WebSocket. The TUI connects to the same port and displays real-time Agent status, plan progress, and learning metrics.

### 2.2 Local Agent Workflow

```bash
# Create an Agent
roko agent create --profile coding --prompt "Fix the auth bug"

# Start it
roko agent start --name fix-auth-bug

# Watch progress
roko dashboard

# Or use the self-hosting loop
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"
roko prd draft new "system-prompt-wiring"
roko prd plan system-prompt-wiring
roko plan run plans/
```

### 2.3 Agent Creation UX

**CLI quick create** (auto-fills from prompt):

```bash
roko agent create --prompt "Review PRs for security issues"
```

**Explicit configuration**:

```bash
roko agent create \
  --name pr-reviewer \
  --profile coding \
  --mode reactive \
  --trigger "webhook:/hooks/github-pr" \
  --trigger "schedule:0 9 * * MON" \
  --budget 10.00
```

**From a template**:

```bash
roko agent create --template code-reviewer --repo https://github.com/org/repo
```

**API**:

```
POST /api/agents
{
  "name": "pr-reviewer",
  "prompt": "Review pull requests for security issues",
  "profile": "coding",
  "mode": "reactive",
  "triggers": [
    { "type": "webhook", "path": "/hooks/github-pr" },
    { "type": "schedule", "cron": "0 9 * * MON" }
  ],
  "execution": "in-process",
  "budget": { "daily_limit_usd": 10.0 },
  "model_routing": {
    "gamma_model": "claude-haiku-4-5",
    "theta_model": "claude-sonnet-4-6",
    "delta_model": "claude-opus-4-6"
  }
}
```

### 2.4 Local Chain Development (Mirage)

For on-chain features, start a local Mirage devnet:

```bash
# Start Mirage (anvil + contracts)
cd contracts/
npx hardhat node  # localhost:8545

# Deploy contracts
npx hardhat deploy --network mirage

# Configure roko to use Mirage
# (roko.toml defaults to chain.network = "mirage")
```

---

## 3. Railway Deployment

### 3.1 One-Click Deploy

```
1. Click "Deploy on Railway"               (~30 seconds)
2. Railway asks for env vars               (paste Anthropic key)
3. roko builds and starts                  (~2 minutes)
4. Visit the URL -> setup wizard           (~30 seconds)
5. Create account
6. Onboarding: create first Agent          (~1 minute)
7. Agent is running, visible in dashboard

Total: ~4 minutes from zero to running Agent.
```

### 3.2 Railway Template

```toml
# railway.toml
[build]
builder = "DOCKERFILE"
dockerfilePath = "docker/roko.Dockerfile"

[deploy]
healthcheckPath = "/api/health"
healthcheckTimeout = 30
restartPolicyType = "ON_FAILURE"

[[services]]
name = "roko"
internalPort = 6677
```

### 3.3 Environment Variables

| Variable | Default | Required? | Notes |
|---|---|---|---|
| `ANTHROPIC_API_KEY` | -- | Yes | Primary LLM provider |
| `PERPLEXITY_API_KEY` | -- | No | Research agent |
| `GEMINI_API_KEY` | -- | No | Gemini backend |
| `OPENROUTER_API_KEY` | -- | No | OpenRouter backend |
| `GITHUB_TOKEN` | -- | No | GitHub MCP integration |
| `FLY_API_TOKEN` | -- | No | Enables isolated Agent execution |
| `RELAY_URL` | `wss://relay.nunchi.dev` | No | Relay for multi-instance |
| `PORT` | `6677` | No | HTTP port |
| `RUST_LOG` | `info` | No | Log level |

### 3.4 Health Checks

```
GET /api/health

Response:
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_secs": 3600,
  "agents_running": 3,
  "plans_active": 1
}
```

### 3.5 Scaling on Railway

For higher load, run multiple Railway services behind Railway's internal load balancer. Each instance connects to a shared relay for Agent presence deduplication and message routing.

```
Railway Service 1 (roko serve) ──> Relay (wss://relay.nunchi.dev)
Railway Service 2 (roko serve) ──/
Railway Service 3 (roko serve) ──/
```

---

## 4. Fly.io Deployment

### 4.1 fly.toml

```toml
app = "roko"
primary_region = "iad"

[build]
  dockerfile = "docker/roko.Dockerfile"

[http_service]
  internal_port = 6677
  force_https = true
  auto_start_machines = true
  auto_stop_machines = true
  min_machines_running = 1

[[vm]]
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 2048

[mounts]
  source = "roko_data"
  destination = "/workspace/.roko"
```

### 4.2 Machine Sizing

| Workload | CPUs | Memory | Notes |
|---|---|---|---|
| Solo (1-5 Agents) | 1 shared | 512 MB | Minimum viable |
| Small team (5-20 Agents) | 2 shared | 2 GB | Default |
| Production (20+ Agents) | 4 dedicated | 4 GB | For heavy inference loads |

### 4.3 Regions

Fly supports multi-region deployment. For lowest latency to LLM providers, deploy in:

- `iad` (Ashburn, Virginia) -- closest to Anthropic, OpenAI
- `sjc` (San Jose) -- West Coast alternative
- `lhr` (London) -- EU presence

### 4.4 Isolated Agent Execution

Fly Machines enable true isolation for untrusted workloads. The control plane creates a Fly Machine per Agent:

```
roko process (control plane)
    |
    +-- POST https://api.machines.dev/v1/machines
    |   -> Create Fly Machine with:
    |     - roko agent run --relay ... --inference-proxy ...
    |     - Volume for persistent state
    |     - Network: outbound only (connects to relay)
    |
    +-- Lifecycle managed by control plane:
        - Create on agent.create
        - Suspend on agent.sleep (reactive mode)
        - Destroy on agent.delete
```

Fly Machines bill per-second. Reactive Agents cost $0 while sleeping.

---

## 5. Docker Deployment

### 5.1 Multi-Stage Dockerfile

```dockerfile
# Build stage
FROM rust:1.91 AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p roko-cli

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/roko /usr/local/bin/roko
EXPOSE 6677
VOLUME ["/workspace/.roko"]
HEALTHCHECK CMD curl -f http://localhost:6677/api/health || exit 1
ENTRYPOINT ["roko", "serve"]
```

### 5.2 Docker Compose

```yaml
version: "3.8"
services:
  roko:
    build:
      context: .
      dockerfile: docker/roko.Dockerfile
    ports:
      - "6677:6677"
    volumes:
      - roko_data:/workspace/.roko
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:6677/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  roko_data:
```

---

## 6. Agent Execution Tiers

```
Tier          Where              When to Use
----          -----              -----------
In-process    tokio task         Default. Fast. Shares memory and Route protocol.
              inside roko        Best for trusted code, small teams.

Isolated      Fly Machine or     Untrusted code, heavy compute,
              Railway service    multi-tenant, customer-facing Agents.
```

### 6.1 In-Process Scaling

A single roko process can run 50-100 in-process Agents concurrently. Each Agent is a tokio task consuming ~1MB of stack + working memory. The bottleneck is inference throughput, not Agent count.

For higher Agent counts, run multiple roko processes behind a load balancer, each connected to the same relay.

### 6.2 Agent Clusters

Groups of Agents with shared context and coordinated Graphs:

```
POST /api/clusters
{
  "name": "feature-build",
  "agents": [
    { "profile": "research", "name": "researcher", "mode": "ephemeral" },
    { "profile": "coding", "name": "impl-1", "mode": "ephemeral", "execution": "isolated" },
    { "profile": "coding", "name": "impl-2", "mode": "ephemeral", "execution": "isolated" },
    { "profile": "coding", "name": "reviewer", "mode": "ephemeral" }
  ],
  "pipeline": [
    { "stage": "research", "agents": ["researcher"] },
    { "stage": "implement", "agents": ["impl-1", "impl-2"], "depends_on": ["research"] },
    { "stage": "review", "agents": ["reviewer"], "depends_on": ["implement"] }
  ]
}
```

Pipeline visualization (TUI and dashboard):

```
researcher --> impl-1 --> reviewer
               impl-2 --/
```

Each node shows: Agent name, status (waiting/working/done), current tier, cost so far.

---

## 7. Monitoring and Health

### 7.1 Health Endpoints

| Endpoint | What |
|---|---|
| `GET /api/health` | Basic health check (status, version, uptime) |
| `GET /api/status` | Detailed status (Agents, plans, learning state) |
| `GET /api/metrics` | Prometheus-format metrics |

### 7.2 Lens-Based Monitoring

The Observe protocol (Lens system, see [09-TELEMETRY.md](09-TELEMETRY.md)) provides built-in observability:

- **AgentLens**: Per-Agent metrics (turns, tokens, cost, latency)
- **PlanLens**: Plan execution progress (tasks completed, failed, pending)
- **GateLens**: Verify-protocol pass rates, threshold drift
- **RouterLens**: Model selection distribution, cost per model
- **MemoryLens**: Knowledge Signal counts, tier distribution, decay rates

Lenses emit observation Signals that can be consumed by the dashboard, TUI, or external monitoring systems.

### 7.3 Alerts

Alerts are configured in `roko.toml`:

```toml
[monitoring.alerts]
# Alert when any Agent exceeds daily budget
budget_exceeded = { threshold = 1.0, action = "pause_agent" }

# Alert when gate pass rate drops below threshold
gate_pass_rate = { threshold = 0.5, window = "1h", action = "notify" }

# Alert when inference latency exceeds threshold
inference_latency = { threshold_ms = 30000, action = "notify" }
```

---

## 8. Secrets Management

### 8.1 Local Secrets

```bash
# Set a secret
roko config secrets set llm.anthropic

# List secrets (names only, not values)
roko config secrets list

# Rotate a secret
roko config secrets rotate llm.anthropic

# Check which secrets are configured
roko config check-secrets
```

Secrets are stored in `~/.roko/secrets/` encrypted at rest. They are never passed in environment variables to child Agents when using isolated execution -- Agents use the inference proxy instead.

### 8.2 Environment Variables

For cloud deployment, secrets are set as environment variables in the deployment platform (Railway, Fly.io, etc.). The `roko serve` command reads them on startup.

### 8.3 Provider API Keys

| Provider | Env Variable | Used By |
|---|---|---|
| Anthropic | `ANTHROPIC_API_KEY` | Primary LLM backend |
| Perplexity | `PERPLEXITY_API_KEY` | Research Agent |
| Gemini | `GEMINI_API_KEY` | Gemini backend |
| OpenRouter | `OPENROUTER_API_KEY` | OpenRouter multi-model |
| GitHub | `GITHUB_TOKEN` | GitHub MCP integration |
| Fly.io | `FLY_API_TOKEN` | Isolated Agent execution |

---

## 9. Backbone: Relay + Mirage

The backbone is always-on infrastructure shared across all users. It is deployed as a single container:

| Service | Image | What |
|---|---|---|
| Mirage | `ghcr.io/nunchi/mirage:latest` | Devnet chain (anvil) + relay WebSocket |
| Relay | Built into Mirage | Agent presence, message routing, Signal stream registry |

The relay is embedded in the Mirage container. One deployment covers both chain and relay. The roko workspace is optional -- the relay and chain operate independently.

---

## 10. Unified Vocabulary Notes

| Old Term (arch-17) | Unified Term | Notes |
|---|---|---|
| Feed registry | Signal stream registry | Agents advertise Signal streams |
| Cluster | Agent cluster with pipeline Graph | Pipeline is a Graph of stages |
| Monitor/watcher | Lens (Observe protocol) | Read-only observation Blocks |
| Inference gateway | Route protocol centralization | CascadeRouter handles model selection |
