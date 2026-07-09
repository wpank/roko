# Roko Architecture Redesign

## The Problem

Three separate deployments that users manually wire together. Agents are either ephemeral CLI processes or stateless HTTP workers. No dynamic lifecycle, no heartbeat, no coordination, no auth.

---

## Single Deployment Unit

Deploy one thing: `roko`. It is the control plane, agent runtime, inference gateway, knowledge store, and secret manager in a single process.

```
┌──────────────────────────────────────────────────────────────┐
│                        roko (:6677)                           │
│                                                               │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────────────┐│
│  │ API      │  │ Agent        │  │ Inference Gateway       ││
│  │ Server   │  │ Supervisor   │  │ (route, cache, cost)    ││
│  └──────────┘  └──────┬───────┘  └─────────────────────────┘│
│                       │                                       │
│         ┌─────────────┼─────────────┐                        │
│         ▼             ▼             ▼                         │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐                  │
│  │ Agent     │ │ Agent     │ │ Agent     │  ...N agents     │
│  │ "coder-1" │ │ "research"│ │ "chain-1" │                  │
│  │ ♥ 10s     │ │ ♥ 30s    │ │ ♥ 5s     │                   │
│  └───────────┘ └───────────┘ └───────────┘                  │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐   │
│  │ Shared: NeuroStore, Episodes, Secrets, Learning        │   │
│  └───────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

Mirage stays separate (chain simulator, not agent runtime).

---

## Authentication

### Two auth surfaces, one identity

| Surface | Auth method | Token type |
|---|---|---|
| **Web dashboard** | Privy (email, Google, Apple, wallet) | Privy JWT → roko session |
| **CLI / TUI** | `roko login` (device flow or PKCE) | roko access token |
| **API / CI** | Bearer token | API key or JWT |
| **Local dev** | `roko serve --insecure` | None (localhost only) |

### Dashboard auth: Privy

The dashboard already uses Privy. The flow stays the same but roko-serve needs to validate Privy tokens:

**Login flow:**
1. User visits dashboard → Privy login modal (email / Google / Apple)
2. Privy creates embedded wallet automatically (`createOnLogin: 'all-users'`)
3. Privy issues JWT with user ID, email, wallet address
4. Dashboard stores JWT in `localStorage` as `nunchi_auth_token`
5. All API calls include `Authorization: Bearer <privy_jwt>`
6. roko-serve validates Privy JWT signature (via Privy JWKS endpoint)
7. On first valid token → roko creates a user record (auto-registration)

**What roko-serve validates:**
```
GET https://auth.privy.io/.well-known/jwks.json
→ Cache JWKS, verify JWT signature + expiry
→ Extract: sub (privy user ID), email, wallet address
→ Lookup or create user in .roko/users/
```

**Wallet use:**
- Privy embedded wallet is available for Korai/Mirage chain interactions
- Signing transactions, delegating to agents, posting pheromones
- Optional — users who don't need chain features never see wallet UI
- External wallets (MetaMask) also work via Privy's multi-wallet support

### CLI / TUI auth: `roko login`

```bash
roko login https://my-roko.up.railway.app

# On a machine with a browser:
#   → Opens browser to https://my-roko.up.railway.app/auth/cli
#   → User authenticates (Privy or email/password)
#   → Localhost callback receives token
#   → Token stored in OS keychain (keyring crate)

# On SSH / headless / TUI:
#   → Shows device code: "Visit https://my-roko.up.railway.app/auth/device"
#   →                    "Enter code: ABCD-EFGH"
#   → Polls until user approves in browser
#   → Token stored in OS keychain
```

The CLI login page on roko-serve:
1. Renders a simple auth page (can use Privy's React SDK or email/password)
2. After auth, generates a long-lived roko access token (30 days)
3. Returns token to CLI via localhost callback or device flow polling
4. The roko access token is separate from Privy JWT — it's issued by roko-serve and validated by roko-serve

### API keys for programmatic access

Created from dashboard Settings or CLI:

```bash
# From CLI
roko config api-keys create --name "github-actions" --scope "agent:write"
# → roko_ak_7f3b2c... (displayed once, store securely)

# From dashboard
# Settings > API Keys > Create Key > name, scope, expiry
```

Scopes:
| Scope | What it allows |
|---|---|
| `read` | List agents, read status, view logs |
| `agent:write` | Create, stop, start agents |
| `cluster:write` | Create, stop clusters |
| `secrets:write` | Manage provider API keys |
| `admin` | Everything including user management |

### Auth middleware (roko-serve)

Every `/api/*` request:
1. Read `Authorization: Bearer <token>` header
2. Try Privy JWT validation (check JWKS signature)
3. Try roko access token validation (lookup in DB)
4. Try API key validation (lookup in `.roko/api-keys/`)
5. If none valid → 401
6. Inject user context into request

### First-run bootstrap

1. Deploy roko → no users exist yet
2. First request to any protected route → redirect to `/setup`
3. Setup page: "Create your admin account"
   - Option A: Sign in with Privy (email/Google/Apple)
   - Option B: Create email/password account
4. First user becomes admin
5. Setup mode closes — new users require invitation or Privy auth
6. For local dev: `roko serve --insecure` skips all auth

### Multi-tenant evolution

**Phase 1 (single user):**
- One user, one deployment
- Tables: `user`, `session`, `api_key`
- All resources belong to the single user

**Phase 2 (teams):**
- Add `organization`, `org_member`, `invitation`
- Add `organization_id` FK to all resource tables
- Privy user → auto-joins org if invited
- Roles: owner > admin > member
- API keys scoped to org

**Phase 3 (multi-tenant SaaS):**
- Org isolation at query level
- Per-org billing
- Per-org secret stores
- Org-scoped agent limits

---

## Secret & API Key Management

### Where secrets live

Roko already has a secrets system with namespaces and a resolution chain.

**Resolution priority (highest wins):**
1. CLI flags (`roko run --api-key sk-...`)
2. Environment variables (`ANTHROPIC_API_KEY=sk-...`)
3. `.roko/secrets.toml` (server-side, `0600` perms)
4. OS keychain (designed, not yet wired)
5. External secret store (Vault / AWS SM / 1Password — designed, not yet wired)

**Server-side storage (`.roko/secrets.toml`):**
```toml
[llm]
anthropic = "sk-ant-..."
perplexity = "pplx-..."
gemini = "AIza..."
moonshot = "sk-..."
zai = "..."
openrouter = "sk-or-..."

[rpc]
alchemy = "..."

[integration]
github = "ghp_..."
slack = "xoxb-..."

[infrastructure]
fly = "fo1_..."
railway = "..."
```

### User experience: managing secrets

**From dashboard (Settings > Provider Keys):**

```
┌─────────────────────────────────────────────────────────────┐
│ Provider Keys                                                │
│                                                              │
│ These keys are stored encrypted on your roko instance.       │
│ Agents use them for LLM inference and integrations.          │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ ☁ Anthropic (Claude)                         REQUIRED  │  │
│ │ sk-ant-•••••••••••••••4f2a            ● Connected      │  │
│ │ [Test] [Update] [Remove]                               │  │
│ │ Models: claude-opus-4-6, claude-sonnet-4-5, haiku-3-5  │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ 🔍 Perplexity (Sonar)                        OPTIONAL  │  │
│ │ Not configured                       ○ Not connected   │  │
│ │ [Add Key]                                              │  │
│ │ Enables: web search, deep research, citation retrieval │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ 🌙 Moonshot (Kimi-K2.5)                      OPTIONAL  │  │
│ │ Not configured                       ○ Not connected   │  │
│ │ [Add Key]                                              │  │
│ │ Enables: Kimi-K2.5 with thinking mode                  │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ 🏠 Ollama (Local)                             OPTIONAL  │  │
│ │ Base URL: http://localhost:11434               ● Found  │  │
│ │ [Configure]                                            │  │
│ │ Enables: local model inference, zero API cost          │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Similarly for: Google Gemini, ZAI (GLM), OpenRouter,         │
│ GitHub, Slack                                                │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Storage preference                                     │  │
│ │ ○ Server-side (default) — stored in .roko/secrets.toml │  │
│ │   Keys persist across sessions and devices.            │  │
│ │   Agents read keys directly.                           │  │
│ │                                                        │  │
│ │ ○ Client-only — stored in browser localStorage         │  │
│ │   Keys never leave your browser.                       │  │
│ │   Passed to roko on each API call.                     │  │
│ │   You must re-enter on new devices.                    │  │
│ └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**"Test" button behavior:**
- Calls `POST /api/secrets/test` with `{ namespace: "llm", key: "anthropic" }`
- roko-serve makes a minimal API call to the provider (e.g., list models)
- Returns: connected / invalid key / rate limited / network error

**Client-only mode:**
- When a user chooses client-only, keys are stored in `localStorage`
- Dashboard includes keys in API requests via custom header: `X-Provider-Keys: <encrypted_json>`
- roko-serve uses them for the request but never persists them
- Keys are encrypted in transit (HTTPS) but not at rest (localStorage)

**From CLI:**
```bash
# Set a secret (reads from stdin, never appears in shell history)
echo "sk-ant-xyz" | roko config secrets set llm.anthropic

# Or interactive prompt
roko config secrets set llm.anthropic
# Enter secret: ••••••••••

# List configured secrets (shows keys, never values)
roko config secrets list
# NAMESPACE    KEY          SOURCE        STATUS
# llm          anthropic    secrets.toml  ● valid
# llm          perplexity   env var       ● valid
# integration  github       secrets.toml  ● valid
# llm          gemini       —             ○ not set

# Validate all secrets
roko config check-secrets
# ✓ llm.anthropic: valid (claude-opus-4-6 accessible)
# ✓ llm.perplexity: valid (sonar-pro accessible)
# ✗ llm.gemini: not configured
# ✓ integration.github: valid (repo access confirmed)

# Rotate a secret
echo "sk-ant-new" | roko config secrets rotate llm.anthropic
```

### How agents access secrets

Agents never hold raw API keys. The inference gateway (inside the roko process) holds them:

```
Agent wants to call Claude
  → sends inference request to gateway (internal, no network)
  → gateway resolves ANTHROPIC_API_KEY from secret store
  → gateway calls Anthropic API
  → returns response to agent

Agent never sees the API key.
```

For isolated agents (Fly Machines), the gateway proxies inference:

```
Isolated agent on Fly Machine
  → sends inference request to parent roko instance
  → parent's gateway resolves key + calls provider
  → returns response over HTTPS
  → agent never has direct provider access
```

This means API keys are centralized, never duplicated across agents, and never sent to cloud VMs.

---

## Agent Creation: Onboarding UX

### Dashboard: "Create Agent" wizard

Accessed from: main dashboard → [+ Create Agent] button, or Agents page → [+ New Agent]

**Step 1: Choose a profile**

```
┌─────────────────────────────────────────────────────────────┐
│ What kind of agent?                                          │
│                                                              │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│ │  </> Coding  │ │  🔬 Research │ │  ⛓ Blockchain │         │
│ │              │ │              │ │              │          │
│ │ Writes code, │ │ Deep web     │ │ Monitors     │          │
│ │ runs tests,  │ │ research,    │ │ positions,   │          │
│ │ opens PRs    │ │ enhances     │ │ rebalances,  │          │
│ │              │ │ PRDs         │ │ reacts to    │          │
│ │              │ │              │ │ chain events │          │
│ └──────────────┘ └──────────────┘ └──────────────┘         │
│                                                              │
│ ┌──────────────┐ ┌──────────────┐                           │
│ │  🛡 Security │ │  ⚙ Custom    │                           │
│ │              │ │              │                           │
│ │ Audits code, │ │ Build your   │                           │
│ │ scans for    │ │ own agent    │                           │
│ │ vulns        │ │ from scratch │                           │
│ └──────────────┘ └──────────────┘                           │
└─────────────────────────────────────────────────────────────┘
```

**Step 2: Configure (varies by profile)**

For **coding** agent:
```
┌─────────────────────────────────────────────────────────────┐
│ Configure coding agent                                       │
│                                                              │
│ Name: [coder-1                    ]  (auto-generated)       │
│                                                              │
│ Repository: [https://github.com/org/repo    ]  [Browse]     │
│   or: [/path/to/local/repo                  ]               │
│                                                              │
│ Mode:                                                        │
│   ● Ephemeral — runs one task, then stops                   │
│   ○ Persistent — runs continuously, watches for changes     │
│   ○ Reactive — sleeps, wakes on PR/push events              │
│                                                              │
│ Task (ephemeral only):                                       │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Fix the authentication bug in src/auth.rs             │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                              │
│ Execution:                                                   │
│   ○ In-process (lightweight, shared resources)              │
│   ● Isolated (own VM, filesystem isolation — recommended    │
│     for repos with build steps)                              │
│   Resources: [2 CPU ▼] [4 GB RAM ▼]                        │
│                                                              │
│ Advanced ▾                                                   │
│   Model preference: [auto (CascadeRouter) ▼]               │
│   Heartbeat interval: [10 seconds         ▼]               │
│   Budget cap: [$5.00 per session          ▼]               │
│   Extensions: [✓ file_watcher] [✓ gate] [✓ neuro]         │
│                                                              │
│ [Cancel]                                [Create Agent →]    │
└─────────────────────────────────────────────────────────────┘
```

For **blockchain** agent:
```
┌─────────────────────────────────────────────────────────────┐
│ Configure blockchain agent                                   │
│                                                              │
│ Name: [chain-monitor-1            ]                         │
│                                                              │
│ Chain: [Mirage Devnet ▼]                                    │
│   RPC URL: [https://mirage-devnet.up.railway.app]           │
│   Chain ID: [88888]                                          │
│                                                              │
│ Wallet:                                                      │
│   ● Use Privy embedded wallet (recommended)                 │
│     Address: 0x7f3b...2c4a                                  │
│   ○ Import private key (stored encrypted in agent volume)   │
│   ○ No wallet (read-only monitoring)                        │
│                                                              │
│ Mode:                                                        │
│   ○ Ephemeral                                               │
│   ● Persistent — monitors chain continuously                │
│   ○ Reactive — wakes on new blocks / price threshold        │
│                                                              │
│ Execution:                                                   │
│   ● Isolated (required for blockchain agents with wallets)  │
│                                                              │
│ Strategy:                                                    │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Monitor ETH lending rates across Aave and Compound.   │   │
│ │ Alert if rate diverges > 50bps from ISFR.             │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                              │
│ Delegation caveats:                                          │
│   Max position: [$1,000        ]                            │
│   Approved protocols: [✓ Aave] [✓ Compound] [✗ Uniswap]   │
│   Stop-loss: [$100            ]                             │
│                                                              │
│ [Cancel]                                [Create Agent →]    │
└─────────────────────────────────────────────────────────────┘
```

**Step 3: Review & launch**
```
┌─────────────────────────────────────────────────────────────┐
│ Review agent                                                 │
│                                                              │
│ Name:       coder-1                                         │
│ Profile:    Coding                                          │
│ Mode:       Ephemeral                                       │
│ Execution:  Isolated (2 CPU, 4 GB RAM on Fly)              │
│ Repository: github.com/org/repo                             │
│ Task:       Fix the authentication bug in src/auth.rs       │
│ Budget:     $5.00 max                                       │
│ Model:      Auto (CascadeRouter)                            │
│                                                              │
│ Estimated cost: ~$0.15 - $2.00                              │
│ (based on similar past tasks)                               │
│                                                              │
│ [← Back]                               [Launch Agent →]     │
└─────────────────────────────────────────────────────────────┘
```

### CLI: `roko agent create`

```bash
# Quick — minimal flags, sensible defaults
roko agent create --profile coding --task "Fix the auth bug in src/auth.rs"
# → Agent coder-f7a2 created (ephemeral, in-process)
# → Heartbeat started, watching...

# With repo
roko agent create --profile coding \
  --repo https://github.com/org/repo \
  --task "Add pagination to the users API" \
  --isolated

# Persistent chain monitor
roko agent create --profile blockchain \
  --mode persistent \
  --chain-url https://mirage-devnet.up.railway.app \
  --name chain-monitor-1

# Research agent
roko agent create --profile research \
  --task "Research best practices for Rust async error handling" \
  --mode ephemeral

# Interactive mode (asks questions)
roko agent create
# Profile? [coding/research/blockchain/security/custom]: coding
# Mode? [ephemeral/persistent/reactive]: ephemeral
# Task: Fix the auth bug
# Isolated? [y/N]: y
# → Agent coder-a3b1 created
```

### Dashboard: onboarding for new users

First visit after setup (no agents exist yet):

```
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│               Welcome to Nunchi                              │
│                                                              │
│   Your agent platform is ready. Let's create your first      │
│   agent and see what it can do.                              │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │ 1. Provider keys                           ✓ Done    │  │
│   │    Anthropic key configured and working              │  │
│   │                                                      │  │
│   │ 2. Create your first agent                 → Next    │  │
│   │    Try a coding agent on a sample task               │  │
│   │                                                      │  │
│   │ 3. Watch it work                           ○ Pending │  │
│   │    See the heartbeat, logs, and results              │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│   [Skip onboarding — go to dashboard]                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

Step 2 opens the agent creation wizard with a suggested sample task. Step 3 shows the agent running with annotated UI elements ("This is the heartbeat — it ticks every 10 seconds", "T0 means no LLM call was needed", etc.).

---

## Scaling: Hybrid Local + Cloud

### Two execution tiers

| Tier | Where | When | Startup | Cost |
|---|---|---|---|---|
| **In-process** | Tokio tasks in the roko process | Light work: monitoring, research, T0/T1 ticks | Instant | $0 marginal |
| **Isolated** | Fly Machine (microVM) | Heavy work: cargo build, git operations, DeFi trading | ~2 seconds | ~$0.09/hr for 2CPU/4GB |

The supervisor decides automatically based on profile + user preference. If `FLY_API_TOKEN` is not set, everything runs in-process.

### Isolated agent lifecycle

```
User: POST /api/agents { profile: "coding", isolated: true }
  │
  ├─ Supervisor creates Fly Machine via REST API (~300ms)
  │   Image: roko:latest
  │   Config: 2 CPU, 4GB RAM, 10GB volume
  │   Command: roko agent run --managed --parent-url https://my-roko.up.railway.app
  │
  ├─ Machine boots (~2s)
  │   → Agent connects back to parent via HTTPS
  │   → Registers with supervisor
  │   → Heartbeat starts
  │
  ├─ Agent runs task
  │   → Inference requests → proxied through parent's gateway
  │   → Knowledge queries → proxied through parent's NeuroStore
  │   → File operations → local to the Fly Machine's volume
  │
  ├─ Task completes (ephemeral) or idles (persistent)
  │   → Ephemeral: supervisor stops machine (billing stops)
  │   → Persistent: continues heartbeat (T0 ticks are cheap)
  │
  └─ User: DELETE /api/agents/:id
      → Supervisor destroys Fly Machine + volume
```

### What's shared vs isolated

| Resource | Shared (all agents) | Isolated (per agent) |
|---|---|---|
| NeuroStore (knowledge) | ✓ | — |
| Episodes, learning state | ✓ | — |
| CascadeRouter (model routing) | ✓ | — |
| Pheromones (stigmergy) | ✓ | — |
| Inference gateway + API keys | ✓ | — |
| Filesystem | — | ✓ (own volume) |
| Working directory / git checkout | — | ✓ |
| Private keys (DeFi wallets) | — | ✓ (encrypted volume) |
| Memory / CPU | — | ✓ (own VM for isolated agents) |

API keys are never sent to isolated agents. All inference routes through the parent's gateway.

---

## Agent Modes

### Persistent

Runs until stopped. Heartbeat loop ticks continuously.

```
POST /api/agents { mode: "persistent", profile: "blockchain" }
→ Agent starts, heartbeats forever
→ T0 ticks: check chain state, update counters ($0)
→ T1 ticks: analyze moderate changes ($0.001)
→ T2 ticks: full reasoning for novel events ($0.01-0.10)
→ Dreams during idle (delta cycle, hourly)
→ User stops when done: POST /api/agents/:id/stop
```

Use cases: chain monitoring, fleet supervision, continuous research.

### Ephemeral

Spins up, does one job, shuts down.

```
POST /api/agents { mode: "ephemeral", task: "Fix auth bug" }
→ Agent starts, heartbeats through the task
→ Task completes → agent reports results
→ Auto-stops, resources released
→ Learning persists (episodes, knowledge)
```

Use cases: coding tasks, one-off research, PR review.

### Reactive

Sleeps until triggered, wakes up, works, sleeps again.

```
POST /api/agents {
  mode: "reactive",
  triggers: [
    { type: "webhook", path: "/hooks/github-pr" },
    { type: "schedule", cron: "0 */6 * * *" }
  ]
}
→ Agent registered, sleeping (no compute cost)
→ GitHub PR webhook fires → agent wakes
→ Reviews PR, posts comments
→ No more work → agent sleeps again
```

Use cases: PR review on push, scheduled audits, alert monitoring.

---

## Clusters

Groups of agents with shared context and coordination.

```
POST /api/clusters {
  "name": "feature-build",
  "agents": [
    { "profile": "research", "name": "researcher", "mode": "ephemeral" },
    { "profile": "coding", "name": "impl-1", "mode": "ephemeral", "isolated": true },
    { "profile": "coding", "name": "impl-2", "mode": "ephemeral", "isolated": true },
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
researcher ──→ impl-1 ──→ reviewer
               impl-2 ──↗
```

Each node shows: agent name, status (waiting/working/done), current tier, cost so far.

---

## Deployment

### One Railway template: Roko

| Variable | Default | Required? |
|---|---|---|
| `ANTHROPIC_API_KEY` | — | Yes |
| `PERPLEXITY_API_KEY` | — | No |
| `GEMINI_API_KEY` | — | No |
| `MOONSHOT_API_KEY` | — | No |
| `ZAI_API_KEY` | — | No |
| `OPENROUTER_API_KEY` | — | No |
| `GITHUB_TOKEN` | — | No |
| `FLY_API_TOKEN` | — | No (enables isolated agents) |
| `PRIVY_APP_ID` | — | No (enables Privy auth in dashboard) |
| `PRIVY_APP_SECRET` | — | No (server-side Privy JWT validation) |
| `PORT` | 6677 | No |
| `RUST_LOG` | info | No |

Healthcheck: `/api/health`
Volume: `/workspace/.roko`

**Mirage** — separate template, same as today.

That's it. Two templates. Most users only need Roko.

### What "deploy" means for the user

```
1. Click "Deploy on Railway" button           (~30 seconds)
2. Railway asks for env vars                  (paste Anthropic key)
3. roko deploys and starts                    (~2 minutes)
4. Visit the URL → setup wizard               (~30 seconds)
5. Create account (Privy or email)
6. Onboarding: create first agent             (~1 minute)
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

# Start server + dashboard (insecure mode for local dev)
roko serve --insecure

# Create an agent (from another terminal or the dashboard)
roko agent create --profile coding --task "Fix the auth bug"
```

---

## Dashboard Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ Nunchi                [+ Agent]  [+ Cluster]  [user@email ▼]    │
├───────┬──────────────────────────────────────────────────────────┤
│       │                                                          │
│  Nav  │  Overview                                                │
│       │  ┌─────────────────────────────────────────────────────┐│
│ 🏠    │  │ ● 5 agents   2 clusters   $4.23 today   ↑ 3d 2h   ││
│ Home  │  └─────────────────────────────────────────────────────┘│
│       │                                                          │
│ 🤖    │  Agents                                                  │
│ Agents│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────┐│
│       │  │ coder-1   │ │ research  │ │ chain-1   │ │ coder-2 ││
│ 🔗    │  │ ● T0      │ │ ● T1     │ │ ● T0      │ │ ◐ T2    ││
│Clusters│ │ coding    │ │ research  │ │ blockchain│ │ coding  ││
│       │  │ idle      │ │ querying  │ │ monitoring│ │ building││
│ 🧠    │  │ $0.02/hr  │ │ $0.08/hr │ │ $0/hr     │ │ $0.15/hr││
│Knowledge│└───────────┘ └───────────┘ └───────────┘ └─────────┘│
│       │                                                          │
│ 💰    │  Cluster: feature-xyz                                    │
│ Costs │  ┌─────────────────────────────────────────────────────┐│
│       │  │ researcher ──→ impl-1 ──→ reviewer                  ││
│ 📋    │  │ ✓ done        ◐ working   ○ waiting                 ││
│ Logs  │  │               impl-2 ──→                            ││
│       │  │               ◐ working                              ││
│ ⚙️    │  └─────────────────────────────────────────────────────┘│
│Settings│                                                         │
│       │  Agent: coder-2 (expanded)                               │
│       │  ┌─────────────────────────────────────────────────────┐│
│       │  │ Status: T2 reasoning  │  Uptime: 12m  │  Cost: $1.8││
│       │  │ Task: Implement pagination in users API              ││
│       │  │                                                      ││
│       │  │ Heartbeat ──────────────────────────────────         ││
│       │  │ T0 T0 T0 T0 T1 T0 T0 T2 T0 T0 T0 T1 [T2]         ││
│       │  │                                                      ││
│       │  │ Logs (live)                                          ││
│       │  │ 14:32:21 [T2] PE=0.73 → full reasoning              ││
│       │  │ 14:32:25 [T2] action: edit src/users.rs:142         ││
│       │  │ 14:32:30 [T0] verify: cargo test → 47 passed        ││
│       │  │                                                      ││
│       │  │ [Stop] [Restart] [View Full Trace] [Open in CLI]    ││
│       │  └─────────────────────────────────────────────────────┘│
└───────┴──────────────────────────────────────────────────────────┘
```

### Settings page

```
Settings
├── Account
│   ├── Profile (name, email, avatar from Privy)
│   └── Wallet (Privy embedded wallet address, delegation status)
│
├── Provider Keys
│   ├── Anthropic (Claude) ──── ● connected
│   ├── Perplexity (Sonar) ──── ○ not set
│   ├── Google (Gemini) ──────── ○ not set
│   ├── Moonshot (Kimi) ──────── ○ not set
│   ├── ZAI (GLM) ────────────── ○ not set
│   ├── OpenRouter ──────────── ○ not set
│   ├── Ollama ──────────────── ● found (localhost:11434)
│   └── Storage: [server-side ▼] / [client-only]
│
├── Integrations
│   ├── GitHub ── ○ not set (enables PR creation)
│   ├── Slack ─── ○ not set (enables notifications)
│   └── Railway ── ● connected (OAuth)
│
├── Infrastructure
│   ├── Fly.io ── ○ not set (enables isolated agents)
│   ├── Control plane: https://my-roko.up.railway.app ── ● healthy
│   └── Mirage: https://mirage-devnet.up.railway.app ── ● healthy
│
├── API Keys
│   ├── github-actions (agent:write) ── created 2d ago
│   ├── [+ Create Key]
│   └── [Manage keys]
│
└── Team (phase 2)
    ├── Members
    ├── Invitations
    └── Roles
```

---

## API Surface

### Agent lifecycle

```
POST   /api/agents                    Create agent
GET    /api/agents                    List agents (status, health, cost)
GET    /api/agents/:id                Agent detail (full status, heartbeat history)
POST   /api/agents/:id/start         Start a stopped agent
POST   /api/agents/:id/stop          Graceful stop
DELETE /api/agents/:id                Destroy agent + clean up resources
GET    /api/agents/:id/logs           Agent logs (paginated, filterable)
GET    /api/agents/:id/trace/:tick    Full decision trace for a specific tick
POST   /api/agents/:id/message       Send a message/task to an agent
```

### Clusters

```
POST   /api/clusters                  Create cluster with pipeline
GET    /api/clusters                  List clusters
GET    /api/clusters/:id              Cluster status (pipeline progress)
POST   /api/clusters/:id/stop         Stop all agents in cluster
DELETE /api/clusters/:id              Destroy cluster + all agents
```

### Secrets

```
GET    /api/secrets                    List secret namespaces + keys (not values)
POST   /api/secrets/:ns/:key          Set a secret
DELETE /api/secrets/:ns/:key          Remove a secret
POST   /api/secrets/:ns/:key/test     Test if a secret is valid
```

### Auth

```
POST   /auth/login                     Email/password login
POST   /auth/privy/verify              Verify Privy JWT, create roko session
POST   /auth/device/authorize          Start device flow
POST   /auth/device/token              Poll for device flow token
GET    /auth/callback                  PKCE OAuth callback (for CLI)
POST   /auth/refresh                   Refresh access token
GET    /auth/session                   Current user info
POST   /api/api-keys                   Create API key
DELETE /api/api-keys/:id               Revoke API key
GET    /api/api-keys                   List API keys
```

### Infrastructure

```
GET    /api/health                     Service health
GET    /api/providers                  Configured LLM providers + health
GET    /api/costs                      Cost summary (per agent, per day, per model)
GET    /api/costs/:agent_id            Cost breakdown for one agent
```

### Existing routes

All ~85 existing routes in roko-serve (plans, PRDs, gates, episodes, signals, knowledge, learning, etc.) remain unchanged. They get auth middleware added.

---

## Bardo Source References

Everything in this redesign has prior art in the bardo codebase (`/Users/will/dev/uniswap/bardo/`). This section maps each proposed component to its bardo implementation and notes what can be ported vs rewritten.

### Inference Gateway → bardo-gateway (22.8K LOC)

**The gateway already exists.** `apps/bardo-gateway/` is a production LLM inference proxy with:

- **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix)
- **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr
- **Tool pruning**: strips unused tool definitions to reduce token count
- **Batch API**: Anthropic batch endpoint for async, cheaper inference
- **Cost tracking**: per-request, per-model, per-session with SQLite persistence (`.mori/costs.db`)
- **USDC micropayments**: Machine Payment Protocol (`crates/mpp/`) for ERC-3009 session payments
- **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard
- **REST analytics**: `/v1/stats`, `/v1/analytics/cost-history`, `/v1/cache-economics`, `/v1/analytics/subsystems`
- **29 gateway action counters**: hash/semantic/prefix cache hits, tools pruned, PII blocks, loop detection, convergence injection, output budgets, repetition detection

**Key insight**: The redesign's "inference gateway" should port bardo-gateway rather than build from scratch. It already handles key isolation (agents call gateway, never hold API keys directly), cost tracking, and multi-provider routing.

**Source files**:
| File | LOC | What |
|---|---|---|
| `apps/bardo-gateway/src/` | 22,856 | Full gateway server |
| `crates/bardo-inference/src/` | 413 | Protocol types (InferenceRequest/Response) |
| `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client (`InferenceClient` trait) |
| `crates/mpp/src/` | 988 | USDC micropayment protocol |

### Dashboard → apps/dashboard (Next.js, ~27K LOC)

**A full monitoring dashboard already exists.** `apps/dashboard/` is a Next.js 15 + React 19 app with:

- **20 components**: KpiStrip, CostCenter, CostHistory, LiveFeed, SessionsTable, ModelCosts, Providers, CacheAnalytics, RequestTypes, ThinkingTokens, LatencyTokens, TopRequests, SubsystemPanels, GatewayActions, AreaChart, DonutChart, AnimatedNumber, etc.
- **Real-time WebSocket**: Connects to `/v1/ws/stats`, ring buffers (36K slots @ 100ms = 1hr history)
- **Canvas-based charts**: Cost projection, latency, token flow, cache hit rate
- **Cost bucketing**: Rolling 1m/15m/1h/1d windows with burn rate projection
- **13 subsystem panels**: Provider health, latency breakdown, tool metrics, budget states, loop/convergence detection
- **Design system**: `@bardo/ui` package with Panel, Kpi, Badge, ProgressBar, DataTable components
- **Two themes**: bardo (rose/bone) and gringotts (gold/bronze)
- **No auth**: Currently public — needs Privy integration

**What's missing from the dashboard**: Agent management, settings/config UI, knowledge browser, auth/login pages, agent creation wizard. These are proposed in the redesign but don't exist in bardo either.

**Source files**:
| File | What |
|---|---|
| `apps/dashboard/src/components/` | 20 React components (2,321 LOC) |
| `apps/dashboard/src/hooks/useGateway.ts` | WebSocket + state management (23.6K) |
| `apps/dashboard/src/lib/types.ts` | GatewayStats, StatsEvent, GatewayActions types |
| `apps/dashboard/src/app/globals.css` | Full design system (1,324 LOC) |
| `packages/ui/src/` | Shared component library + design tokens |

### Agent Runtime → mori (108K LOC)

**Mori is the production orchestrator** that roko-cli/orchestrate.rs replaces. Key patterns to port:

- **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM→SIGKILL with 200ms grace (`apps/mori/src/agent/connection.rs:2444-2620`)
- **MultiAgentPool + warm spawning**: Pre-spawn warm agents during gate overlap, `promote_warm()` saves 5-15s per phase (`apps/mori/src/agent/mod.rs`)
- **26 agent roles**: Conductor, Strategist, Implementer, Architect, Auditor, Scribe, Critic, Refactorer, etc. with priority scheduling (`apps/mori/src/agent/roles.rs`)
- **3 LLM backends**: Claude CLI, Codex (GPT-5.4), Cursor ACP — routed by model slug
- **Rate limiter**: 8-agent default concurrency, priority-sorted queue
- **Orphan reaper**: `~/.mori/runtime/agent-pids.json` tracks PIDs, cleanup on restart + periodic reaping
- **Conductor (meta-orchestrator)**: 10 watchers (GhostTurn, ReviewLoop, IterationLoop, TestFailureBudget, SilenceTimeout, CompileFailThreshold, TaskStall, ContextPressure, PhaseTimeout, CooldownFilter) with 3-tier interventions (Nudge/Restart/Abort)

**Source files**:
| File | LOC | What |
|---|---|---|
| `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle |
| `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning |
| `apps/mori/src/agent/roles.rs` | — | 26 roles, backend routing, priority |
| `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |
| `apps/mori/src/app/parallel.rs` | — | Main event loop (100ms tick) |
| `apps/mori/src/orchestrator/executor.rs` | 4,629 | ParallelExecutor state machine |
| `apps/mori/src/orchestrator/memory.rs` | 2,944 | Episode logging, playbook queries |

### Heartbeat → golem-heartbeat (10.2K LOC)

**Full 9-step CoALA pipeline exists but was never integrated into mori's runtime**:

- **9-step tick**: Observe → Retrieve → Analyze → Gate → Simulate → Validate → Execute → Verify → Reflect
- **AdaptiveClock**: Gamma (fast perception), Theta (reflective planning), Delta (offline consolidation)
- **T0/T1/T2 gating**: DailyCostAccumulator (hard budget), GasGate (base fee > 2× EMA), AdaptiveGate (prediction error threshold)
- **Sleepwalker mode**: 3-step reduced pipeline (Observe → Reflect → Publish) for low-cost fallback
- **VCG attention auction**: 6 cognitive bidder kinds (Task, Memory, Inference, Safety, Research, Neuro)
- **Prediction error**: Rolling window EMA, `is_severe()` at >25% error rate

**Source files**:
| File | LOC | What |
|---|---|---|
| `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine orchestration |
| `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline |
| `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate |
| `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction |
| `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |

### DeFi Tools → golem-tools (7.2K LOC)

**29+ tool categories with capability-gated execution**:

- **17 categories**: Data, Trading, Lending, Staking, Restaking, Derivatives, Yield, LP, Vault, Safety, Intelligence, Memory, Identity, Wallet, Streaming, Testnet, Bootstrap
- **ToolExecutor pipeline**: Registry lookup → PolicyCage → Capability validation → Rate limiter → Circuit breaker → Mirage simulation gate → Dispatch → Audit
- **14 tool profiles**: Active, Observatory (72-tool read-only), Conservative, Trader, LP, Vault, etc.
- **Vault tools (25+)**: ERC-4626 lifecycle (create/deposit/withdraw/rebalance), participants, proxies, executors, strategy auctions
- **Identity tools (5)**: ERC-8004 agent registry (register, update, feedback, validate, discover)
- **Safety config**: max_value_per_tx ($10K), per_tick ($50K), per_day ($100K), max_slippage (1%), max_price_impact (3%), min_health_factor (1.2)

**Source files**:
| File | What |
|---|---|
| `crates/golem-tools/src/types.rs` | ToolDef, ToolContext, ToolResult, ToolCategory, ToolProfile |
| `crates/golem-tools/src/executor.rs` | ToolExecutor with full validation pipeline |
| `crates/golem-tools/src/tools/vault/` | 25+ vault lifecycle tools |
| `crates/golem-tools/src/tools/identity/` | 5 ERC-8004 tools |
| `crates/golem-tools/src/safety.rs` | CapabilitySpendTracker, safety config |

### Chain Runtime → golem-chain (5.3K LOC)

- **12 networks**: Ethereum, Polygon, Arbitrum, Optimism, Base, Avalanche, Scroll, Blast, etc.
- **ProviderPool**: Alloy HTTP providers with moka LRU cache (100 entries, 5min TTL)
- **SubgraphClient**: The Graph queries with dual cache (pool 15s, metadata 5m) + auto-pagination
- **RevmSimulator**: Local EVM simulation (30M gas limit, 128KB output limit)
- **Warden**: Time-delay safety (PoolParameterUpdate: 1h, VaultRebalance: 30m, LargeSwap: 10m, CrossChainBridge: 2h)
- **Permit2**: EIP-712 domain separation + permit types
- **ERC-8004 registry**: On-chain agent identity (capabilities, service endpoints, metadata CID)

**Source**: `crates/golem-chain/src/` — provider.rs, revm_sim.rs, warden.rs, permit2.rs, identity.rs

### Affect & Knowledge → golem-daimon (8.9K), golem-grimoire (13.7K), golem-dreams (2.8K)

These are already largely ported to roko as roko-daimon, roko-neuro, and roko-dreams respectively. Key bardo additions not yet in roko:

- **Daimon**: ALMA 3-layer EMA (emotion → mood → personality), 8D k-d tree somatic landscape, 8 behavioral biases, clade contagion
- **Grimoire**: LanceDB episodic vectors, SQLite semantic store, Ebbinghaus demurrage, A-MAC 5-factor admission gate, clade Babel translation
- **Dreams**: Styx WebSocket submission, NREM/REM scheduling with emotional load thresholds, EvolutionEngine memetic curator

### Additional Bardo Components

| Crate | LOC | Status in Roko | Action |
|---|---|---|---|
| `golem-triage` | 7,969 | Not ported | Port request prioritization + anomaly detection |
| `golem-context` | 3,995 | Partial (VCG in roko-compose) | Port context governor + curiosity scoring |
| `golem-mortality` | 9,876 | Reframed as vitality cycling | Port vitality tracking, skip death mechanics |
| `golem-identity` | 7,146 | Not ported | Port wallet identity + replication records |
| `golem-economy` | 323 | Not ported | Port revenue tracking + metabolic loop |
| `golem-sonification` | 5,520 | Not needed | Skip (Phase 3+ novelty) |
| `golem-creature` | 11 | Stub | Skip |
| `golem-engagement` | 11 | Stub | Skip |
| `golem-coordination` | 11 | Stub (pheromones in roko-orchestrator) | Skip (roko has better impl) |
| `golem-surfaces` | 2,869 | Partial (TUI views) | Port output formatting |
| `bardo-terminal` | 34,724 | roko TUI exists (simpler) | Cherry-pick useful views |
| `mori-index` | 5,332 | roko-index exists | Merge incremental indexing features |
| `mori-mcp` | 3,331 | roko-mcp-code exists | Merge MCP features |
| `mori-context` | 702 | In roko-compose | Already covered |
| `mpp` | 988 | Not ported | Port for Phase 2 payment integration |

---

## What Already Shipped in Roko (Phase 1)

Commit `5af205d3` shipped most of Phase 1:

| Feature | Status | Where |
|---|---|---|
| Secrets HTTP API (GET/POST/DELETE/test) | **Shipped** | `roko-serve/src/routes/secrets.rs` |
| Multi-key auth middleware (X-Api-Key + Bearer) | **Shipped** | `roko-serve/src/routes/middleware.rs` |
| API key scopes + SHA-256 + expiry | **Shipped** | `roko-serve/src/routes/auth.rs` |
| `roko login` CLI with credential store | **Shipped** | `roko-cli/src/auth.rs`, `credentials.rs` |
| Agent CRUD (create/start/stop/restart/list) | **Shipped** | `roko-serve/src/routes/agents.rs` |
| ProcessSupervisor integration | **Shipped** | `roko-runtime/src/process.rs` |
| Agent sidecar (13 routes) | **Shipped** | `roko-agent-server/src/` |
| Secret testing (Anthropic, OpenAI, Gemini, Perplexity) | **Shipped** | `roko-serve/src/routes/secrets.rs` |

**Not yet shipped**: Privy JWT validation (stub only), device flow/PKCE, scope enforcement at route level, agent modes (persistent/ephemeral/reactive), profiles, inference gateway, clusters, dashboard UI.

---

## Implementation Path (Revised)

### Phase 1: Auth + Secrets ~~API~~ ✅ DONE
Already shipped. Remaining:
- Privy JWT validation (real JWKS, not structural stub)
- Scope enforcement at route level
- Device flow for headless CLI login

### Phase 2: Inference Gateway (port bardo-gateway)
- Port bardo-gateway's 3-layer cache into roko-serve or new roko-gateway crate
- Port `InferenceClient` trait from golem-inference
- Wire CascadeRouter as the model selection layer
- Port cost tracking (per-request, per-agent, per-session)
- Port WebSocket stats endpoint for dashboard
- Port tool pruning and output budgeting
- Dashboard: gateway monitoring (port bardo dashboard components)

### Phase 3: Dashboard Integration
- Port Next.js dashboard into `apps/dashboard/`
- Add Privy login page
- Add agent creation wizard (new, not in bardo)
- Add settings page with provider keys UI (redesign spec)
- Port gateway monitoring components from bardo dashboard
- Wire to roko-serve API routes

### Phase 4: Agent Lifecycle + Modes
- Add `AgentMode` enum (Persistent/Ephemeral/Reactive)
- Add `AgentProfile` templates (Coding/Research/Blockchain/Security/Custom)
- Wire existing webhook subscriptions for reactive mode
- Port warm spawning from mori's MultiAgentPool
- Port conductor watchers into roko-conductor
- Wire heartbeat tier enforcement (T0/T1/T2) at dispatch time

### Phase 5: Heartbeat Pipeline
- Port golem-heartbeat's 9-step TickPipeline into roko
- Wire AdaptiveClock (gamma/theta/delta) into agent tick loop
- Wire PredictionError + AdaptiveGate
- Port sleepwalker mode for low-cost fallback
- Dashboard: heartbeat visualization, tier distribution

### Phase 6: DeFi Tools + Chain Runtime
- Port golem-tools ToolExecutor framework into roko-std
- Port vault tools (25+) and identity tools (5)
- Port golem-chain's ProviderPool + SubgraphClient
- Port Warden time-delay safety
- Port RevmSimulator for pre-trade simulation
- Wire Mirage integration for testing

### Phase 7: Isolated Execution
- Fly Machines REST API integration
- `roko agent run --managed` child mode
- Inference proxying through parent gateway (uses Phase 2 gateway)
- Volume management for persistent state

### Phase 8: Clusters + Coordination
- Wire FleetConductor (L4) with evaluation logic
- Connect pheromone system to agent dispatch
- Cluster API routes (POST/GET/DELETE)
- Pipeline stage → agent mapping
- Dashboard: cluster visualization

### Phase 9: Multi-tenant
- Organization model
- Invitation-based onboarding
- Per-org resource isolation
- Per-org billing
