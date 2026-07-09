# Deployment

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.
> Merges the "Agent creation UX", "Scaling: hybrid local + cloud", "Clusters", and "Deployment" sections.

---

## Agent creation UX

### Dashboard wizard

```
Step 1: What does this agent do?
┌─────────────────────────────────────────────────────────┐
│ Describe your agent's purpose:                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Review pull requests on the main repo, check for    │ │
│ │ security issues, and post comments.                 │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                         │
│ Or choose a template:                                   │
│ [Code reviewer]  [Chain monitor]  [Research assistant]  │
│ [PR automator]   [Security audit] [Data pipeline]       │
└─────────────────────────────────────────────────────────┘

Step 2: Configuration (auto-filled from description)
┌─────────────────────────────────────────────────────────┐
│ Name:     [pr-reviewer        ]                         │
│ Profile:  [Coding           v ]                         │
│ Mode:     [Reactive          v]                         │
│                                                         │
│ Triggers:                                               │
│  [x] GitHub webhook: push to main                       │
│  [ ] Schedule: ______                                   │
│  [ ] Chain event: ______                                │
│                                                         │
│ Execution:                                              │
│  (o) In-process (recommended for most agents)           │
│  ( ) Isolated (Fly Machine -- separate compute)         │
│                                                         │
│ Model:                                                  │
│  (o) Auto (CascadeRouter selects per-task)              │
│  ( ) Force: [______________]                            │
│                                                         │
│ Budget: [$10.00/day   ] (inference cost limit)          │
└─────────────────────────────────────────────────────────┘

Step 3: Review and create
┌─────────────────────────────────────────────────────────┐
│ Agent: pr-reviewer                                      │
│ Profile: Coding                                         │
│ Mode: Reactive (wakes on GitHub push)                   │
│ Execution: In-process                                   │
│ Model: Auto                                             │
│ Budget: $10/day                                         │
│ Extensions: git, compiler, test-runner, lsp             │
│                                                         │
│ [Create agent]                                          │
└─────────────────────────────────────────────────────────┘
```

### CLI: roko agent create

```bash
# Quick create (auto-fills from prompt)
roko agent create --prompt "Review PRs for security issues"

# Explicit configuration
roko agent create \
  --name pr-reviewer \
  --profile coding \
  --mode reactive \
  --trigger "webhook:/hooks/github-pr" \
  --trigger "schedule:0 9 * * MON" \
  --budget 10.00

# From a template
roko agent create --template code-reviewer --repo https://github.com/org/repo
```

### Agent creation API

```
POST /api/agents
Content-Type: application/json

{
  "name": "pr-reviewer",
  "prompt": "Review pull requests for security issues and post comments",
  "profile": "coding",
  "mode": "reactive",
  "triggers": [
    { "type": "webhook", "path": "/hooks/github-pr" },
    { "type": "schedule", "cron": "0 9 * * MON" }
  ],
  "execution": "in-process",
  "budget": { "daily_limit_usd": 10.0 },
  "extensions": ["git", "compiler", "test-runner"],
  "model_routing": {
    "gamma_model": "claude-haiku-4-5",
    "theta_model": "claude-sonnet-4-6",
    "delta_model": "claude-opus-4-6"
  }
}
```

Response:

```json
{
  "agent_id": "agt_a1b2c3d4",
  "name": "pr-reviewer",
  "status": "created",
  "mode": "reactive",
  "profile": "coding",
  "created_at": "2026-04-24T12:00:00Z"
}
```

---

## Scaling: hybrid local + cloud

### Agent execution tiers

```
Tier          Where              When to use
────          ─────              ───────────
In-process    tokio task         Default. Fast. Shares memory, gateway.
              inside roko        Best for trusted code, small teams.

Isolated      Fly Machine or     Untrusted code, heavy compute,
              Railway service    multi-tenant, customer-facing agents.
```

### In-process scaling

A single roko process can run 50-100 in-process agents concurrently. Each agent is a tokio task consuming ~1MB of stack + working memory. The bottleneck is inference throughput, not agent count.

For higher agent counts, run multiple roko processes behind a load balancer, each connected to the same relay. The relay handles presence deduplication and message routing.

### Isolated execution (Fly Machines)

For workloads that need true isolation (untrusted code execution, customer data separation):

```
roko process (control plane)
    │
    ├── POST https://api.machines.dev/v1/machines
    │   → Create Fly Machine with:
    │     - roko agent run --relay ... --inference-proxy ...
    │     - Volume for persistent state
    │     - Network: outbound only (connects to relay)
    │
    │ Agent connects outbound to relay
    │ Agent sends inference through proxy
    │
    └── Lifecycle managed by control plane:
        - Create on agent.create
        - Suspend on agent.sleep (reactive mode)
        - Destroy on agent.delete
```

Fly Machines bill per-second. Reactive agents cost $0 while sleeping.

```rust
pub struct FlyMachineManager {
    api_token: String,
    app_name: String,
    http: reqwest::Client,
}

impl FlyMachineManager {
    async fn create_agent(&self, spec: &AgentSpec) -> Result<MachineId> {
        let body = json!({
            "config": {
                "image": "ghcr.io/nunchi/roko-agent:latest",
                "env": {
                    "ROKO_AGENT_NAME": spec.name,
                    "ROKO_RELAY_URL": spec.relay_url,
                    "ROKO_INFERENCE_PROXY": spec.inference_proxy_url,
                    "ROKO_AGENT_TOKEN": spec.token,
                },
                "guest": {
                    "cpu_kind": "shared",
                    "cpus": 1,
                    "memory_mb": 512,
                },
                "auto_destroy": true,
            }
        });

        let resp = self.http
            .post(format!(
                "https://api.machines.dev/v1/apps/{}/machines",
                self.app_name
            ))
            .bearer_auth(&self.api_token)
            .json(&body)
            .send()
            .await?;

        let machine: FlyMachine = resp.json().await?;
        Ok(machine.id)
    }
}
```

---

## Clusters

Groups of agents with shared context and coordinated pipelines.

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
  ],
  "shared_context": {
    "prd": "prds/feature-xyz.md",
    "repo": "https://github.com/org/repo"
  }
}
```

Dashboard shows cluster pipeline as a visual graph:

```
researcher ──> impl-1 ──> reviewer
               impl-2 ──/
```

Each node shows: agent name, status (waiting/working/done), current tier, cost so far.

Cluster events are published to the `cluster:{id}` room. The dashboard subscribes when viewing a cluster and unsubscribes when navigating away.

---

## Deployment

### The backbone: relay + mirage

Always on. Shared across all users. Deployed as two containers:

| Service | Image | What |
|---------|-------|------|
| Mirage | `ghcr.io/nunchi/mirage:latest` | Devnet chain (anvil) + relay WebSocket |
| Relay | Built into Mirage | Agent presence, message routing, feed registry |

The relay is embedded in the Mirage container. One deployment covers both chain and relay.

### The workspace: roko

Optional per-user deployment. Adds orchestration, plans, PRDs, learning, inference gateway.

| Variable | Default | Required? |
|----------|---------|-----------|
| `ANTHROPIC_API_KEY` | -- | Yes |
| `PERPLEXITY_API_KEY` | -- | No |
| `GEMINI_API_KEY` | -- | No |
| `MOONSHOT_API_KEY` | -- | No |
| `ZAI_API_KEY` | -- | No |
| `OPENROUTER_API_KEY` | -- | No |
| `GITHUB_TOKEN` | -- | No |
| `FLY_API_TOKEN` | -- | No (enables isolated agents) |
| `PRIVY_APP_ID` | -- | No (enables Privy auth) |
| `PRIVY_APP_SECRET` | -- | No (server-side JWT validation) |
| `RELAY_URL` | `wss://relay.nunchi.dev` | No |
| `PORT` | 6677 | No |
| `RUST_LOG` | info | No |

Healthcheck: `GET /api/health`
Volume: `/workspace/.roko`

### What "deploy" means for a new user

```
1. Click "Deploy on Railway"               (~30 seconds)
2. Railway asks for env vars               (paste Anthropic key)
3. roko builds and starts                  (~2 minutes)
4. Visit the URL -> setup wizard           (~30 seconds)
5. Create account (Privy or email)
6. Onboarding: create first agent          (~1 minute)
7. Agent is running, visible in dashboard

Total: ~4 minutes from zero to running agent.
```

### Local development

```bash
# Install
cargo install roko-cli

# Init
roko init

# Set API key
echo "sk-ant-..." | roko config secrets set llm.anthropic

# Start server (insecure mode for local dev -- no auth required)
roko serve --insecure

# Create an agent (from another terminal or the dashboard)
roko agent create --profile coding --prompt "Fix the auth bug"
```

### Railway template

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
