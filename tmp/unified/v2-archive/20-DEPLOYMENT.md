# 20 — Deployment

> Local development, cloud deployment, scaling tiers, WASM compilation, brain export/import with Merkle-CRDT sync, and agent execution tiers. The same binary runs everywhere; configuration selects the scale.

**Subsumes**: Deployment architecture, scaling tiers, WASM target, brain export/import, agent execution isolation, monitoring.

**Source**: `tmp/architecture/17-deployment.md` (rewritten for the unified model). Major additions: WASM compilation target with progressive enhancement, brain export/import via Merkle-CRDT, Mirage backbone.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (Agent lifecycle, vitality), [09-TELEMETRY](09-TELEMETRY.md) (Lens system, StateHub projections), [14-CONFIG-AND-AUTHORING](14-CONFIG-AND-AUTHORING.md) (5-tier SPI, Tier 4 WASM), [17-SECURITY-MODEL](17-SECURITY-MODEL.md) (sandboxing by tier)

---

## 1. Three Scaling Tiers

All tiers use the same binary. The difference is configuration: environment variables, execution mode, and whether a relay is involved.

| Tier | Users | Deployment | Agents | Relay |
|---|---|---|---|---|
| **Solo developer** | 1 | `roko serve` on localhost | 1-10 in-process | None |
| **Small team** | 2-10 | Railway or Fly.io single instance | 10-50 in-process | Optional |
| **Production** | 10+ | Railway/Fly multi-instance | 50+ in-process + isolated | Required |

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

For on-chain features (see [doc-18](18-ON-CHAIN-REGISTRIES.md)), start a local Mirage devnet:

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

## 3. WASM Compilation

The same Roko core compiles to **native** and **WASM** targets. This enables progressive enhancement: start with the full native binary, deploy lightweight WASM components where sandboxing or portability matters.

### 3.1 What Compiles to WASM

| Component | WASM Target | Use Case |
|---|---|---|
| Cell implementations | `wasm32-wasi` | Marketplace distribution (see [doc-14](14-CONFIG-AND-AUTHORING.md) Tier 4) |
| Scoring functions | `wasm32-wasi` | Portable eval scoring for arenas (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md)) |
| Gate implementations | `wasm32-wasi` | Custom verification logic |
| Extension hooks | `wasm32-wasi` | Third-party Extension distribution (see [doc-08](08-EXTENSION-SYSTEM.md)) |
| Signal processing pipelines | `wasm32-wasi` | Edge deployment, browser-side processing |
| HDC vector operations | `wasm32-unknown-unknown` | Client-side similarity search |

### 3.2 Progressive Enhancement at 3 Capability Levels

```
Full native (default)
  - All crates compiled natively
  - Full filesystem, network, LLM access
  - Maximum performance
  - Deployment: server, desktop

WASM runtime (embedded)
  - Native host + WASM guest Cells
  - Host mediates capabilities (fuel-metered)
  - Sandboxed third-party code
  - Deployment: server with untrusted plugins

WASM standalone (portable)
  - Core engine as WASM module
  - Runs in any WASI-compatible runtime
  - Limited capabilities (no direct fs/net)
  - Deployment: edge, browser, serverless
```

### 3.3 Build Targets

```bash
# Native (default)
cargo build --release -p roko-cli

# WASM Cell (for marketplace publication)
cargo build --release -p my-block --target wasm32-wasi

# WASM core (for portable deployment)
cargo build --release -p roko-core --target wasm32-wasi
```

### 3.4 Fuel Metering

WASM Cells run with fuel limits to prevent runaway computation (see [doc-17](17-SECURITY-MODEL.md) section 10.2):

```toml
# Cell manifest
[block.impl]
tier      = "wasm"
path      = "my-block.wasm"
fuel      = 100_000_000    # execution fuel cap
memory_mb = 64             # memory limit
```

The host runtime (wasmtime) tracks fuel consumption and terminates the WASM instance when the limit is reached. 100M fuel is approximately 1 second of computation on modern hardware.

### 3.5 ABI Contract

WASM Cells communicate with the host via `wit-bindgen` interfaces:

```wit
// roko-block.wit
interface block {
    record signal {
        id: string,
        kind: string,
        payload: string,
        score: tuple<f64, f64, f64, f64, f64>,
    }

    record block-input {
        signals: list<signal>,
        macros: list<tuple<string, string>>,
    }

    record block-output {
        signals: list<signal>,
        persist: list<signal>,
    }

    run: func(input: block-input) -> result<block-output, string>
}
```

The host grants capabilities to the WASM guest based on the Cell's declared capabilities intersected with the Space grants (three-layer capability intersection, see [doc-17](17-SECURITY-MODEL.md) section 3). CaMeL tags are applied at the host function boundary -- the WASM guest cannot strip or modify tags.

---

## 4. Brain Export and Import

An Agent's learned state -- its routing preferences, heuristics, calibration data, knowledge graph, and adaptive thresholds -- can be exported as a portable **brain** and imported into a new instance. This enables knowledge transfer between deployments, backup/restore, and Agent cloning.

### 4.1 What a Brain Contains

```
brain-export-2026-04-26.roko-brain
+-- manifest.toml              # metadata, version, source agent, export time
+-- knowledge/
|   +-- signals.jsonl          # Knowledge Signals (Heuristic, Insight, etc.)
|   +-- hdc-index.bin          # HDC fingerprint index (binary, compact)
+-- learning/
|   +-- cascade-router.json    # CascadeRouter state (EFE posteriors)
|   +-- gate-thresholds.json   # Adaptive gate thresholds (EMA per rung)
|   +-- experiments.json       # Prompt experiment state
|   +-- efficiency.jsonl       # Efficiency event history
|   +-- calibration.json       # Per-operator calibration state
+-- episodes/
|   +-- episodes.jsonl         # Episode history (summarized, not full turns)
+-- profile/
    +-- profile.toml           # Domain profile snapshot
    +-- extensions.toml        # Extension configuration
```

### 4.2 Export Size

A brain export is compact -- typically **100KB to 1MB**:

| Component | Typical Size | Notes |
|---|---|---|
| Knowledge Signals | 50-500 KB | Only Consolidated+ tier Signals exported by default |
| HDC index | 10-100 KB | Binary, compact |
| Learning state | 5-50 KB | JSON, small |
| Episode summaries | 20-200 KB | Summarized, not full turns |
| Profile + config | 2-10 KB | TOML |

Full episode history (with complete turns) is excluded by default. Include it with `--include-episodes=full`, which increases size to 1-10 MB.

### 4.3 Export CLI

```bash
# Export current Agent's brain
roko knowledge backup --agent coder-1 --output coder-brain.roko-brain

# Export with filters
roko knowledge backup --agent coder-1 \
  --min-tier consolidated \     # only high-confidence knowledge
  --since 2026-04-01 \          # recent learning only
  --include-episodes=summary \  # episode summaries, not full turns
  --output coder-brain.roko-brain
```

### 4.4 Import CLI

```bash
# Import into a new Agent
roko knowledge restore --agent coder-2 --input coder-brain.roko-brain

# Import with decay (older knowledge starts at lower balance)
roko knowledge restore --agent coder-2 \
  --input coder-brain.roko-brain \
  --decay-factor 0.8            # imported Signals start at 80% balance
```

### 4.5 Merkle-CRDT Sync Protocol

When two Agent instances share a brain lineage (e.g., one was cloned from the other), they can sync learning state via **Merkle-CRDT merge**. This produces convergent state without central coordination.

```
Agent A (original)          Agent B (clone)
    |                           |
    v                           v
Learn from task X           Learn from task Y
    |                           |
    v                           v
Brain state A'              Brain state B'
    |                           |
    +--- Merkle-CRDT sync ---+
              |
              v
    Merged state (A' + B')
    Both agents converge
```

#### CRDT operations

Each learning update maps to a conflict-free replicated data type:

| Component | CRDT Type | Merge Behavior |
|---|---|---|
| CascadeRouter model counts | **GCounter** (grow-only counter) | Sum of per-node increments. Monotonic. |
| Gate thresholds | **LWW-Register** (last-writer-wins) | Most recent Lamport timestamp wins. |
| Knowledge Signals | **Add-only set** with demurrage | Union of both sets. Duplicate Signals deduplicated by content hash. Balance is a GCounter. |
| Experiment state | **LWW-Register** | Most recent wins. |
| Episode summaries | **Add-only set** | Union. |

```rust
pub enum CrdtOp {
    GCounterIncrement { key: String, delta: u64, node_id: NodeId },
    LwwRegisterSet { key: String, value: Value, timestamp: LamportClock },
    SetAdd { key: String, element: ContentHash },
}
```

#### Merkle tree indexing

Each Agent maintains a Merkle tree over its brain state. The root hash summarizes the entire learning state in 32 bytes.

```rust
pub struct BrainMerkleTree {
    pub root: H256,
    pub nodes: HashMap<H256, MerkleNode>,
}

pub enum MerkleNode {
    Leaf { key: String, value_hash: H256 },
    Branch { left: H256, right: H256 },
}
```

#### Incremental sync

Two Agents exchange Merkle roots. If roots differ, they walk the tree to find divergent subtrees and exchange only the differing CRDT operations. Typical sync payload: **1-10 KB** for incremental updates.

```bash
# One-shot sync
roko knowledge sync --peer wss://other-instance.example.com/sync

# Continuous sync (background)
roko knowledge sync --peer wss://other-instance.example.com/sync --continuous
```

#### Conflict-free convergence

CRDTs are conflict-free by construction. Two Agents that learned different things from different tasks converge to a state that contains both learnings. No manual conflict resolution. GCounters merge via component-wise max. LWW-Registers merge via timestamp comparison. Add-only sets merge via union.

### 4.6 Use Cases

| Scenario | How Brain Export Helps |
|---|---|
| **Backup/restore** | Export brain before risky changes, restore if things go wrong |
| **Agent cloning** | Clone a well-trained Agent for a new workspace |
| **Knowledge transfer** | Import coding heuristics from a senior Agent into a junior one |
| **Multi-instance sync** | Two instances developing the same codebase share learning |
| **Deployment migration** | Move an Agent from local to cloud without losing learning state |
| **Arena bootstrapping** | Import brain from meta-arena into a new coding arena (see [doc-19](19-ARENAS-EVALS-BOUNTIES.md)) |

---

## 5. Railway Deployment

### 5.1 One-Click Deploy

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

### 5.2 Railway Template

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

### 5.3 Environment Variables

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

### 5.4 Health Checks

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

### 5.5 Scaling on Railway

For higher load, run multiple Railway services behind Railway's internal load balancer. Each instance connects to a shared relay for Agent presence deduplication and message routing.

```
Railway Service 1 (roko serve) --> Relay (wss://relay.nunchi.dev)
Railway Service 2 (roko serve) --/
Railway Service 3 (roko serve) --/
```

Brain sync (section 4.5) keeps learning state convergent across instances.

---

## 6. Fly.io Deployment

### 6.1 fly.toml

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

### 6.2 Machine Sizing

| Workload | CPUs | Memory | Notes |
|---|---|---|---|
| Solo (1-5 Agents) | 1 shared | 512 MB | Minimum viable |
| Small team (5-20 Agents) | 2 shared | 2 GB | Default |
| Production (20+ Agents) | 4 dedicated | 4 GB | For heavy inference loads |

### 6.3 Regions

Fly supports multi-region deployment. For lowest latency to LLM providers:

- `iad` (Ashburn, Virginia) -- closest to Anthropic, OpenAI
- `sjc` (San Jose) -- West Coast alternative
- `lhr` (London) -- EU presence

### 6.4 Isolated Agent Execution via Fly Machines

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

## 7. Docker Deployment

### 7.1 Multi-Stage Dockerfile

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

### 7.2 Docker Compose

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

## 8. Agent Execution Tiers

```
Tier          Where              When to Use
----          -----              -----------
In-process    tokio task         Default. Fast. Shares memory and Route protocol.
              inside roko        Best for trusted code, small teams.

Isolated      Fly Machine or     Untrusted code, heavy compute,
              Railway service    multi-tenant, customer-facing Agents.
```

### 8.1 In-Process Scaling

A single roko process can run 50-100 in-process Agents concurrently. Each Agent is a tokio task consuming ~1MB of stack + working memory. The bottleneck is inference throughput, not Agent count.

For higher Agent counts, run multiple roko processes behind a load balancer, each connected to the same relay. Brain sync (section 4.5) keeps learning state convergent.

### 8.2 Agent Clusters with Pipeline Graphs

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

Each node shows: Agent name, status (waiting/working/done), current tier, cost so far. The pipeline is a Graph (see [doc-03](03-GRAPH.md)) -- the same execution engine runs both single-Agent tasks and multi-Agent clusters.

---

## 9. Monitoring and Health

### 9.1 Health Endpoints

| Endpoint | What |
|---|---|
| `GET /api/health` | Basic health check (status, version, uptime) |
| `GET /api/status` | Detailed status (Agents, plans, learning state) |
| `GET /api/metrics` | Prometheus-format metrics |

### 9.2 Lens-Based Monitoring

The Observe protocol (Lens system, see [doc-09](09-TELEMETRY.md)) provides built-in observability:

- **AgentLens**: Per-Agent metrics (turns, tokens, cost, latency)
- **PlanLens**: Plan execution progress (tasks completed, failed, pending)
- **GateLens**: Verify-protocol pass rates, threshold drift
- **RouterLens**: Model selection distribution, cost per model
- **MemoryLens**: Knowledge Signal counts, tier distribution, decay rates
- **CostLens**: Real-time cost telemetry per Cell, per Graph, per Agent

Lenses emit observation Signals consumed by the dashboard, TUI, or external monitoring systems. StateHub projections (see [doc-09](09-TELEMETRY.md)) provide typed, universal views consumed by all surfaces.

### 9.3 Alerts

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

## 10. Secrets Management

### 10.1 Local Secrets

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

Secrets are stored in `~/.roko/secrets/` encrypted at rest (AES-256-GCM). They are never passed in environment variables to child Agents when using isolated execution -- Agents use the inference proxy instead. This ensures that a compromised isolated Agent cannot exfiltrate API keys.

### 10.2 Cloud Deployment Secrets

For cloud deployment, secrets are set as environment variables in the deployment platform (Railway, Fly.io, etc.). The `roko serve` command reads them on startup.

### 10.3 Provider API Keys

| Provider | Env Variable | Used By |
|---|---|---|
| Anthropic | `ANTHROPIC_API_KEY` | Primary LLM backend |
| Perplexity | `PERPLEXITY_API_KEY` | Research Agent |
| Gemini | `GEMINI_API_KEY` | Gemini backend |
| OpenRouter | `OPENROUTER_API_KEY` | OpenRouter multi-model |
| GitHub | `GITHUB_TOKEN` | GitHub MCP integration |
| Fly.io | `FLY_API_TOKEN` | Isolated Agent execution |

---

## 11. Backbone: Relay + Mirage

The backbone is always-on infrastructure shared across all users. It is deployed as a single container:

| Service | Image | What |
|---|---|---|
| Mirage | `ghcr.io/nunchi/mirage:latest` | Devnet chain (anvil) + relay WebSocket |
| Relay | Built into Mirage | Agent presence, message routing, Signal stream registry |

The relay is embedded in the Mirage container. One deployment covers both chain and relay. The roko workspace is optional -- the relay and chain operate independently.

---

## 12. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko serve` starts on :6677 with health check responding | Integration test: start, hit /api/health |
| Railway deploy: Dockerfile builds, health check passes | CI: build Docker image, run health check |
| Fly.io deploy: fly.toml valid, machines start and respond | CI: validate config |
| Docker Compose: services start, volumes mount correctly | Integration test |
| In-process: 50 concurrent Agents start without OOM | Load test: start 50 Agents, measure memory |
| Isolated: Fly Machine created and connected to relay | Integration test with mock Fly API |
| WASM Cell loads, runs with fuel limit, terminates on exhaustion | Integration test: WASM Cell exceeds fuel -> terminated |
| WASM ABI: Cell input/output round-trips through wit-bindgen | Unit test |
| WASM sandbox prevents unauthorized fs/net access | Security test: WASM Cell attempts unauthorized syscall -> trapped |
| Progressive enhancement: native -> WASM runtime -> WASM standalone | Build all three targets, verify each runs |
| Brain export: Agent state serialized to ~100KB-1MB file | Unit test: export, verify size and contents |
| Brain export manifest includes version and source agent | Unit test: verify manifest fields |
| Brain import: Imported state restores routing, thresholds, knowledge | Integration test: export A, import into B, verify B has A's learning |
| Brain import with decay: Older knowledge starts at reduced balance | Unit test: import with decay-factor 0.8, verify balances at 80% |
| Merkle-CRDT sync: Two instances converge after divergent learning | Integration test: A learns X, B learns Y, sync, both have X+Y |
| Merkle-CRDT incremental sync: Only divergent subtrees exchanged | Unit test: measure sync payload size after small update (~1-10KB) |
| CRDT GCounter: merge produces component-wise max | Unit test: two counters with different increments |
| CRDT LWW-Register: merge selects most recent timestamp | Unit test: two registers with different timestamps |
| CRDT Add-only set: merge produces union | Unit test: two sets with different elements |
| Secrets never in env vars for isolated Agents | Integration test: verify inference proxy used, no ANTHROPIC_API_KEY in child env |
| Multi-instance: Two roko processes share relay, no Agent duplication | Integration test with relay mock |
| Agent cluster pipeline: stages execute in dependency order | Integration test: 3-stage pipeline completes correctly |
| Health endpoints return correct data | Integration test: /api/health, /api/status, /api/metrics |
| Alerts fire when thresholds exceeded | Unit test: simulate budget overrun -> pause_agent action triggered |
